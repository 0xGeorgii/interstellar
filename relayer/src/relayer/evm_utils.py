from typing import Any, Dict, Tuple
from hexbytes import HexBytes
from web3 import Web3
from web3.types import TxParams, TxReceipt
from web3.contract.contract import ContractFunction
from eth_account.signers.local import LocalAccount
from eth_abi.abi import encode as abi_encode

def _send_tx(w3: Web3, acc: LocalAccount, fn: ContractFunction, value: int = 0, CHAIN_ID: int = 11155111) -> TxReceipt:
    base: TxParams = {}
    base['from'] = acc.address
    base['chainId'] = CHAIN_ID
    base['gas'] = 2_500_000
    base['gasPrice'] = w3.to_wei("0.5", "gwei")
    base['nonce'] = w3.eth.get_transaction_count(acc.address, "pending")
    tx      = fn.build_transaction(base | {"value": value}) # type: ignore
    signed  = acc.sign_transaction(tx) # type: ignore
    tx_hash = w3.eth.send_raw_transaction(signed.raw_transaction)
    receipt = w3.eth.wait_for_transaction_receipt(tx_hash)
    print(f"✓ {tx_hash.hex()} confirmed in block {receipt['blockNumber']}")
    return receipt

def _order_hash_local(w3: Web3, ds: Any, order: Tuple[Any, ...]) -> HexBytes:
    """
    Bytes-perfect mirror of OrderLib.hashOrder() for contracts that pre-date
    `hashOrder()` being public.
    """
    _ORDER_TYPEHASH = w3.keccak(
        text="Order(uint256 salt,Address maker,Address receiver,Address makerAsset,"
        "Address takerAsset,uint256 makingAmount,uint256 takingAmount,"
        "MakerTraits makerTraits)"
    )
    keys = ("salt","maker","receiver","makerAsset","takerAsset",
                "makingAmount","takingAmount","makerTraits")
    order_dict: Dict = dict(zip(keys, order)) # type: ignore
    struct_enc = abi_encode(
        [
            "bytes32",
            "uint256",
            "uint256",
            "uint256",
            "uint256",
            "uint256",
            "uint256",
            "uint256",
            "uint256",
        ],
        [
            _ORDER_TYPEHASH,
            order_dict["salt"],
            order_dict["maker"],
            order_dict["receiver"],
            order_dict["makerAsset"],
            order_dict["takerAsset"],
            order_dict["makingAmount"],
            order_dict["takingAmount"],
            order_dict["makerTraits"],
        ],
    )
    return HexBytes(w3.keccak(b"\x19\x01" + ds + w3.keccak(struct_enc)))