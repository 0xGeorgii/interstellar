from contextlib import asynccontextmanager
import asyncio, logging, os
import dotenv
from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel
from rich.logging import RichHandler
import uvicorn
from typing import List, Optional
from .soroban import *
from decimal import *
import stellar_sdk
from stellar_sdk import Keypair, Network, SorobanServer, TransactionBuilder
from stellar_sdk.xdr import (
    SCVal, SCValType, SCAddress, SCAddressType,
    SorobanAuthorizationEntry, SorobanCredentials, SorobanCredentialsType,
    SorobanAddressCredentials, SorobanAuthorizedInvocation,
    SorobanAuthorizedFunction, SorobanAuthorizedFunctionType,
    InvokeContractArgs, SCString, Uint64
)

dotenv.load_dotenv()
stellar_network_passphrase = os.getenv("STELLAR_NETWORK_PASSPHRASE")
stellar_escrow_factory_address = os.getenv("STELLAR_ESCROW_FACTORY_ADDRESS")
stellar_resolver_address = os.getenv("STELLAR_RESOLVER_ADDRESS")
stellar_resolver_secret = os.getenv("STELLAR_RESOLVER_SECRET")
stellar_swapper_address = os.getenv("STELLAR_SWAPPER_ADDRESS")
stellar_swapper_secret = os.getenv("STELLAR_SWAPPER_SECRET")

horizon_server = Server("https://horizon-testnet.stellar.org")
soroban_server = SorobanServer("https://soroban-testnet.stellar.org")

@asynccontextmanager
async def lifespan(app: FastAPI):
    try:
        logging.basicConfig(
            level=logging.INFO,
            format="%(asctime)s %(name)-12s %(levelname)-8s %(message)s",
            handlers=[RichHandler(rich_tracebacks=True)],
        )
        log = logging.getLogger("resolver")

        try:
            pass
        except Exception as _:
            pass
        yield
        print("Shutting down the application")
    except Exception as _:
        pass
    finally:
        pass

app = FastAPI(lifespan=lifespan)
origins = [
    "*",
]

class SorobanTransactionRequest(BaseModel):
    prepared_transaction_xdr: str
    user_public_key: str

class SorobanContractCallRequest(BaseModel):
    user_public_key: str
    contract_id: str
    method: str
    args: List
    auth_signatures: Optional[List] = None

@app.post("/execute-authorized-soroban-transaction")
async def execute_authorized_soroban_transaction(request: SorobanTransactionRequest):
    """
    Execute a pre-authorized transaction prepared on the client side
    """
    try:
        result = await soroban_executor.execute_with_authorization(
            request.prepared_transaction_xdr,
            request.user_public_key
        )
        
        if result["status"] == "error":
            raise HTTPException(status_code=400, detail=result["error"])
        
        return result
        
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))

@app.post("/execute-soroban-contract-call")
async def execute_soroban_contract_call(request: SorobanContractCallRequest):
    """
    Execute contract call with pre-signed authorization
    """
    try:
        result = await soroban_executor.execute_contract_call_with_auth(
            request.user_public_key,
            request.contract_id,
            request.method,
            request.args,
            request.auth_signatures or []
        )
        
        if result["status"] == "error":
            raise HTTPException(status_code=400, detail=result["error"])
        
        return result
        
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))

@app.get("/health")
async def health_check():
    return {"status": "healthy"}

class Wallet(BaseModel):
    network: str
    address: str
    token: str

class DutchAuction(BaseModel):
    start_time: int
    start_multiplier: Decimal
    stop_time: int
    stop_multiplier: Decimal

class Order(BaseModel):
    hashlock: str
    src_wallet: Wallet
    src_amount: Decimal
    dst_wallet: Wallet
    dst_amount: Decimal
    auction: Optional[DutchAuction]

class MakingAuth(BaseModel):
    pass

@app.post("/order")
async def process_order(order: Order):
    # Handle order processing
    match (order.src_wallet.network, order.dst_wallet.network):
        case ("Ethereum", "Stellar"):
            # Solidity maker-to-taker

            # Soroban taker-to-maker
            contract_address_xdr = address_to_scval(stellar_escrow_factory_address)
            auth_address_xdr = address_to_scval(stellar_resolver_address)

            amount_val = \
                encode_amount_calc_flat(int(order.dst_amount)) \
                    if order.auction is None \
                    or order.auction.start_multiplier <= order.auction.stop_multiplier \
                    else \
                encode_amount_calc_linear(
                    order.auction.start_time,
                    order.auction.stop_time,
                    int(order.dst_amount * order.auction.start_multiplier),
                    int(order.dst_amount * order.auction.stop_multiplier)
                )

            timelocks_data = {
                "withdrawal": int(time.time()) + 86400,
                "public_withdrawal": int(time.time()) + 172800,
                "cancellation": int(time.time()) + 259200,
                "public_cancellation": int(time.time()) + 345600
            }

            immutables_xdr = encode_escrow_immutables(
                hashlib.sha256(b'secret').digest(),
                "Taker2Maker",
                order.dst_wallet.address,
                None,
                amount_val,
                5,
                timelocks_data
            )

            # Create function arguments
            args = [
                immutables_xdr,
                address_to_scval(stellar_resolver_address)
            ]

            # Create authorization entry
            auth_entry = SorobanAuthorizationEntry(
                credentials=SorobanCredentials(
                    type=SorobanCredentialsType.SOROBAN_CREDENTIALS_ADDRESS,
                    address=SorobanAddressCredentials(
                        address=auth_address_xdr,
                        nonce=stellar_sdk.xdr.Int64(123),
                        signature_expiration_ledger=stellar_sdk.xdr.Uint32(100),
                        signature=SCVal(type=SCValType.SCV_VOID)
                    )
                ),
                root_invocation=SorobanAuthorizedInvocation(
                    function=SorobanAuthorizedFunction(
                        type=SorobanAuthorizedFunctionType.SOROBAN_AUTHORIZED_FUNCTION_TYPE_CONTRACT_FN,
                        contract_fn=InvokeContractArgs(
                            contract_address=contract_address_xdr,
                            function_name=SCString(b"create_escrow"),
                            args=args
                        )
                    ),
                    sub_invocations=[]
                )
            )

            # Build transaction
            source_account = soroban_server.load_account(stellar_resolver_address)
            
            transaction = (
                TransactionBuilder(
                    source_account=source_account,
                    network_passphrase=stellar_network_passphrase,
                    base_fee=100
                )
                .append_invoke_contract_function_op(
                    contract_id=stellar_escrow_factory_address,
                    function_name="create_escrow",
                    parameters=args,
                    auth=[auth_entry]
                )
                .set_timeout(30)
                .build()
            )

            # Prepare and simulate transaction
            try:
                prepared_transaction = soroban_server.prepare_transaction(transaction)
            except stellar_sdk.exceptions.PrepareTransactionException as pte:
                print(pte)
                raise
            
            # Sign and submit
            prepared_transaction.sign(source_keypair)
            response = soroban_server.send_transaction(prepared_transaction)
            
            # Parse result (equivalent to assert_eq!(r, 2))
            if response.status == "SUCCESS":
                # The return value would be in the transaction result
                # You'd need to parse the XDR result to get the actual return value
                print("Transaction successful")
                return True
            else:
                print(f"Transaction failed: {response}")
                return False

            pass
        case ("Stellar", "Ethereum"):
            # Solidity taker-to-maker
            # Soroban maker-to-taker
            pass
        case _:
            return {"status": "failure", "cause": "Unsupported exchange direction"}
    return {"status": "success", "order": order}

class Secret(BaseModel):
    value: str

@app.post("/secret")
async def create_secret(secret: Secret):
    # Handle secret creation
    return {"status": "success", "secret": secret}

app.add_middleware(
    CORSMiddleware,
    allow_origins=origins,
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

if __name__ == "__main__":
    port = int(os.getenv("PORT", "4000"))
    uvicorn.run("src.resolver.main:app", host="0.0.0.0", port=port, reload=True)
