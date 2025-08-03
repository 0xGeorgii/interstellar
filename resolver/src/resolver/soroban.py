import hashlib
import time
from stellar_sdk import *
from stellar_sdk.xdr import *

# --- Helpers To Build Encoded Values ---

def make_bytes32(hex_or_bytes):
    return Hash(HashID.from_hex(hex_or_bytes) if isinstance(hex_or_bytes, str) else HashID(hex_or_bytes))

def sc_address_from_str(address: str) -> SCAddress:
    if StrKey.is_valid_ed25519_public_key(address):
        return SCAddress(
            type=SCAddressType.SC_ADDRESS_TYPE_ACCOUNT,
            account_id=Uint256(StrKey.decode_ed25519_public_key(address)),
        )
    elif StrKey.is_valid_contract(address):
        return SCAddress(
            type=SCAddressType.SC_ADDRESS_TYPE_CONTRACT,
            contract_id=Hash(StrKey.decode_contract(address)),
        )
    raise ValueError("Invalid address format")

def address_to_scval(address: str) -> SCVal:
    return SCVal(
        type=SCValType.SCV_ADDRESS,
        address=sc_address_from_str(address)
    )

def i128_to_scval(n: int) -> SCVal:
    return SCVal(
        type=SCValType.SCV_I128,
        i128=Int128Parts(hi=Int64((n >> 64) & 0xFFFFFFFFFFFFFFFF), lo=Uint64(n & 0xFFFFFFFFFFFFFFFF)),
    )

def u64_to_scval(n: int) -> SCVal:
    return SCVal(type=SCValType.SCV_U64, u64=Uint64(n))

def bytes_to_sc_bytes(data: bytes) -> SCVal:
    return SCVal(
        type=SCValType.SCV_BYTES,
        bytes=SCBytes(data),
    )

def symbol_to_scval(sym: str) -> SCVal:
    return SCVal(
        type=SCValType.SCV_SYMBOL,
        sym=SCSymbol(sym.encode("utf-8")),
    )

def map_to_scval(pairs: list[tuple[SCVal, SCVal]]) -> SCVal:
    return SCVal(
        type=SCValType.SCV_MAP,
        map=SCMap(pairs)
    )

def vec_to_scval(items: list[SCVal]) -> SCVal:
    return SCVal(
        type=SCValType.SCV_VEC,
        vec=SCVec(items)
    )

def enum_to_tuple_variant(tag_name: str, payload: SCVal=None) -> SCVal:
    inner_vec = [symbol_to_scval(tag_name)]
    if payload is not None:
        inner_vec.append(payload)

    return vec_to_scval(inner_vec)

def option_to_scval(value) -> SCVal:
    """Convert optional value to SCVal (None or Some(value))"""
    if value is None:
        return SCVal(type=SCValType.SCV_VOID)
    else:
        return value

# --- Custom Type Constructs ---

def encode_amount_calc_flat(amount: int):
    return enum_to_tuple_variant("Flat", i128_to_scval(amount))


def encode_amount_calc_linear(start_time, stop_time, start_amount, stop_amount):
    params = SCMap([
        SCMapEntry(key=symbol_to_scval("start_time"), val=u64_to_scval(start_time)),
        SCMapEntry(key=symbol_to_scval("stop_time"), val=u64_to_scval(stop_time)),
        SCMapEntry(key=symbol_to_scval("start_amount"), val=i128_to_scval(start_amount)),
        SCMapEntry(key=symbol_to_scval("stop_amount"), val=i128_to_scval(stop_amount)),
    ])
    payload = SCVal(type=SCValType.SCV_MAP, map=params)
    return enum_to_tuple_variant("Linear", payload)


def encode_timelocks(withdrawal, public_withdrawal, cancellation, public_cancellation):
    return map_to_scval([
        SCMapEntry(key=symbol_to_scval("withdrawal"), val=u64_to_scval(withdrawal)),
        SCMapEntry(key=symbol_to_scval("public_withdrawal"), val=u64_to_scval(public_withdrawal)),
        SCMapEntry(key=symbol_to_scval("cancellation"), val=u64_to_scval(cancellation)),
        SCMapEntry(key=symbol_to_scval("public_cancellation"), val=u64_to_scval(public_cancellation)),
    ])


def encode_escrow_direction(enum_val: str):
    assert enum_val in ["Maker2Taker", "Taker2Maker"]
    return enum_to_tuple_variant(enum_val)


def encode_escrow_immutables(
    hashlock: bytes,
    direction: str,
    maker: str,
    token: Optional[str],  # Now optional
    amount: SCVal,  # i128 or variant
    safety_deposit: int,  # Renamed and simplified
    timelocks: dict,
):
    entries = [
        SCMapEntry(key=symbol_to_scval("hashlock"), val=bytes_to_sc_bytes(hashlock)),
        SCMapEntry(key=symbol_to_scval("direction"), val=encode_escrow_direction(direction)),
        SCMapEntry(key=symbol_to_scval("maker"), val=address_to_scval(maker)),
        SCMapEntry(key=symbol_to_scval("token"), val=option_to_scval(
            address_to_scval(token) if token is not None else None
        )),
        SCMapEntry(key=symbol_to_scval("amount"), val=amount),
        SCMapEntry(key=symbol_to_scval("safety_deposit"), val=i128_to_scval(safety_deposit)),
        SCMapEntry(key=symbol_to_scval("timelocks"), val=encode_timelocks(**timelocks) if timelocks else encode_timelocks(0, 0, 0, 0)),
    ]
    
    return map_to_scval(entries)