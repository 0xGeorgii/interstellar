from __future__ import annotations
from typing import Any, Tuple
import os, json, random, sys
from hexbytes import HexBytes
from dotenv import load_dotenv
from eth_account import Account
from web3 import Web3
import time
from eth_account.signers.local import LocalAccount

from .evm_utils import _send_tx

load_dotenv()

# ───────────────────────────────────────────────────────────────────── utils
HERE = os.path.dirname(os.path.abspath(__file__))

def _abi(name: str):  # very small helper for *.json ABI
    with open(os.path.join(HERE, f"{name}.json")) as f:
        return json.load(f)["abi"]

def _pack_timelocks(t0: int, t1: int, t2: int, t3: int) -> int:
    return t0 | (t1 << 64) | (t2 << 128) | (t3 << 192)
# ─────────────────────────────────────────────────────────────────────
# ───────────────────────────────────────────────────────────────────── web3 setup
RPC_URL             = os.getenv("EVM_RPC", "")
w3                  = Web3(Web3.HTTPProvider(RPC_URL))
RESOLVER            = Web3.to_checksum_address(os.getenv("EVM_RESOLVER_ADDRESS", ""))
RESOLVER_ABI        = _abi("Resolver")
resolver            = w3.eth.contract(address=RESOLVER, abi=RESOLVER_ABI)
FACTORY_ADDR        = Web3.to_checksum_address(os.getenv("EVM_ESCROW_FACTORY_ADDRESS", ""))
FACTORY_ABI         = _abi("EscrowFactory")
factory             = w3.eth.contract(address=FACTORY_ADDR, abi=FACTORY_ABI)
LOP_ADDR            = Web3.to_checksum_address(os.getenv("EVM_LOP_ADDRESS", ""))
LOP_ABI             = _abi("LimitOrderProtocol")
lop                 = w3.eth.contract(address=LOP_ADDR, abi=LOP_ABI)
# ────────────────────────────────────────────────────────── on-cain hash
def _order_hash(order_tuple: Tuple[int, ...]) -> HexBytes:
    """
    Try the on‑chain helper first (cheapest & always 100 % exact).
    If it fails (old deployments without `hashOrder`) fall back to local hash.
    """
    try:
        return HexBytes(lop.functions.hashOrder(order_tuple).call())
    except Exception:
        raise Exception("Failed to call hashOrder on LOP")
    
def build_extension_bytes(factory_address: str, maker_traits: int) -> bytes:
    """
    Correctly builds the extension data with an offset header and concatenated data,
    as required by ExtensionLib.sol.
    """
    # The fields are ordered according to the contract's enum
    # We only need to provide data for the fields we're using.
    fields = {
        "post_interaction_data": bytes.fromhex(factory_address.replace("0x", "")),
        "custom_data": maker_traits.to_bytes(32, 'big')
    }
    
    FIELD_ORDER = [
        "maker_asset_suffix", "taker_asset_suffix", "making_amount_data",
        "taking_amount_data", "predicate", "maker_permit",
        "pre_interaction_data", "post_interaction_data" # Note: custom_data is handled separately
    ]

    offsets = 0
    concatenated_data = b""
    current_offset = 0

    # Build the offset header and concatenated data for the first 8 fields
    for i, field_name in enumerate(FIELD_ORDER):
        field_data = fields.get(field_name, b'')
        current_offset += len(field_data)
        concatenated_data += field_data
        # Pack the current end-offset into the 256-bit integer
        offsets |= (current_offset << (i * 32))

    header = offsets.to_bytes(32, 'big')
    custom_data_bytes = fields.get("custom_data", b'')

    # The final extension is: 32-byte offsets + concatenated data + custom data
    return header + concatenated_data + custom_data_bytes

def build_taker_traits(extension_length: int) -> int:
    """
    Builds the taker_traits integer, specifying the length of the extension.
    """
    # Bit 224-247: ARGS_EXTENSION_LENGTH
    return extension_length << 224

# ─────────────────────────────────────────────────────────────── magic logic
def test() -> None:
    
    # ────────────────────────────────────────────────────────── participants
    RPC_URL                         = os.getenv("EVM_RPC", "")
    w3                              = Web3(Web3.HTTPProvider(RPC_URL))

    maker_account: LocalAccount     = Account.from_key(os.getenv("EVM_MAKER_SC"))
    resolver_account: LocalAccount  = Account.from_key(os.getenv("EVM_RESOLVER_SC"))
    # ──────────────────────────────────────────────────────────

    ONE_ETH                         = 10**18
    MAKING_AMT                      = int(0.000009 * ONE_ETH)
    TAKING_AMT                      = int(0.000001 * ONE_ETH)
    SAFETY_DEP                      = int(0.001 * ONE_ETH)

    salt                            = random.getrandbits(256)
    ZERO                            = 0  # native ETH
    maker                           = int(maker_account.address, 16)
    taker                           = int(resolver_account.address, 16)

    maker_traits = 0
    maker_traits |= (1 << 249)  # HAS_EXTENSION_FLAG 
    maker_traits |= (1 << 251)  # POST_INTERACTION_CALL_FLAG_POSITION
    maker_traits |= (1 << 255)  # _NO_PARTIAL_FILLS_FLAG

    ESCROW_FACTORY_ADDRESS = resolver.address
    extension_bytes = build_extension_bytes(ESCROW_FACTORY_ADDRESS, maker_traits)
    args = extension_bytes 

    extension_hash = w3.keccak(extension_bytes)
    extension_hash_int = int.from_bytes(extension_hash, 'big')
    UINT160_MAX = (1 << 160) - 1
    hash_part = extension_hash_int & UINT160_MAX
    random_part = random.getrandbits(96) << 160
    salt = random_part | hash_part

    order_data: Tuple[int, ...] = (
        salt,           # salt uint256
        maker,          # maker uint256
        taker,          # receiver uint256
        int("0x7b79995e5f793A07Bc00c21412e50Ecae098E7f9", 16),           # makerAsset uint256
        int("0x7b79995e5f793A07Bc00c21412e50Ecae098E7f9", 16),           # takerAsset uint256
        MAKING_AMT,     # makingAmount uint256
        TAKING_AMT,     # takingAmount uint256
        maker_traits,   # makerTraits uint256
    )
    order_hash = _order_hash(order_data)
    print(f"orderHash  : {order_hash.hex()}")

    # ────────────────────────────────────────────────────────── build & verify signature locally
    order_hash_signer = maker_account
    sig = order_hash_signer.unsafe_sign_hash(order_hash) # type: ignore
    r_bytes = int(sig.r).to_bytes(32, "big")
    vs_int = sig.s | ((sig.v - 27) << 255)
    vs_bytes = vs_int.to_bytes(32, "big")

    # ────────────────────────────────────────────────────────── off‑chain sanity check – abort if it doesn’t recover to maker
    recovered = Account._recover_hash(order_hash, vrs=(sig.v, sig.r, sig.s))
    if recovered.lower() != order_hash_signer.address.lower():
        sys.exit(
            f"✗ Signature would recover to {recovered}, "
            f"not {order_hash_signer.address}. Aborting."
        )
    # ────────────────────────────────────────────────────────── prepare immutables & structs
    secret = os.urandom(32)
    print(f"secret     : {secret.hex()}")
    hash_lock = w3.keccak(secret)
    timelocks_src = 0 # _pack_timelocks(10, 120, 121, 122)

    src_immutables: Tuple[Any, ...] = (
        order_hash,     # bytes32 orderHash
        hash_lock,      # bytes32 hashlock aka hash of the secret
        maker,          # Address maker
        taker,          # Address taker
        int("0x7b79995e5f793A07Bc00c21412e50Ecae098E7f9", 16),           # Address token
        MAKING_AMT,     # uint256 amount
        SAFETY_DEP,     # uint256 safetyDeposit
        timelocks_src,  # uint256 timelocks
    )
    
    taker_traits = build_taker_traits(len(extension_bytes))

    fn_src = resolver.functions.deploySrc(
        src_immutables,     # IBaseEscrow.Immutables calldata immutables
        order_data,         # IOrderMixin.Order calldata order
        r_bytes,            # bytes32 r
        vs_bytes,           # bytes32 vs
        MAKING_AMT,         # uint256 amount
        taker_traits,       # TakerTraits takerTraits
        args                # bytes calldata args
    )

    print("\n🚀 deploySrc …")
    receipt_src = _send_tx(w3, resolver_account, fn_src, value=SAFETY_DEP + TAKING_AMT)
    print("\n⏳ Waiting for 5 seconds before processing the event...")
    time.sleep(5)
    ev = factory.events.SrcEscrowCreated().get_logs(from_block=receipt_src['blockNumber'])
    print(f"\n✅ Src escrow deployed: {ev}")
    
    # -------------------   deployDst  -------------------------------------
    current_block_number = w3.eth.block_number
    timelocks_dst = 0 #_pack_timelocks(10, 100, 101, 0)
    dst_immutables: Tuple[Any, ...] = (
        order_hash,
        hash_lock,
        maker,
        taker,
        ZERO,
        TAKING_AMT,
        SAFETY_DEP,
        timelocks_dst
    )
    relock = (src_immutables[7] >> 128) & ((1 << 64) - 1)
    cancel_ts = w3.eth.get_block(current_block_number).get('timestamp') + relock + 100

    fn_dst = resolver.functions.deployDst(
        dst_immutables, # IBaseEscrow.Immutables calldata dstImmutables
        cancel_ts   ,   # uint256 srcCancellationTimestamp
    )
    print(f"\n🛠  deployDst … with value {TAKING_AMT + SAFETY_DEP}")
    _send_tx(w3, resolver_account, fn_dst, value=TAKING_AMT + SAFETY_DEP)

    print("\n✅ All done – both escrows deployed.")
    print("\n⏳ Waiting for 5 seconds before withdrawing...")
    time.sleep(5)

    src_escrow_addr = input("Enter src escrow address: ")
    SRC_ESCROW_ADDR = Web3.to_checksum_address(src_escrow_addr)
    dst_escrow_addr = input("Enter dst escrow address: ")
    DST_ESCROW_ADDR = Web3.to_checksum_address(dst_escrow_addr)

    fn_withdraw = resolver.functions.withdraw(
        SRC_ESCROW_ADDR,
        secret,
        src_immutables
    )
    print(f"\n🛠  withdraw from src escrow {SRC_ESCROW_ADDR}")
    _send_tx(w3, resolver_account, fn_withdraw)
    fn_withdraw = resolver.functions.withdraw(
        DST_ESCROW_ADDR,
        secret,
        dst_immutables
    )
    print(f"\n🛠  withdraw from dst escrow {DST_ESCROW_ADDR}")
    _send_tx(w3, resolver_account, fn_withdraw)
    print("\n✅ Withdrawn from both escrows.")
