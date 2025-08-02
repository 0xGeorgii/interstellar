from contextlib import asynccontextmanager
import asyncio, logging, os
import dotenv
from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel
from rich.logging import RichHandler
import uvicorn
from typing import List, Optional
from .soroban import SorobanAuthExecutor
from decimal import *


@asynccontextmanager
async def lifespan(app: FastAPI):
    try:
        logging.basicConfig(
            level=logging.INFO,
            format="%(asctime)s %(name)-12s %(levelname)-8s %(message)s",
            handlers=[RichHandler(rich_tracebacks=True)],
        )
        dotenv.load_dotenv()
        log = logging.getLogger("resolver")

        # Initialize the executor
        soroban_executor = SorobanAuthExecutor(
            server_secret_key=os.getenv("STELLAR_SECRET"),
            network=os.getenv("STELLAR_NETWORK", "TESTNET")
        )

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
async def available_order(order: Order):
    # Handle order creation
    match (order.src_wallet.network, order.dst_wallet.network):
        case ("Ethereum", "Stellar"):
            # Solidity maker-to-taker
            # Soroban taker-to-maker
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
