from contextlib import asynccontextmanager
import asyncio, logging, os
import dotenv
from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from rich.logging import RichHandler
import uvicorn
from pydantic import BaseModel
import requests

from .ethereum_watcher import EthereumWatcher
from .stellar_watcher import StellarWatcher
from .state_machine import SwapStateMachine


@asynccontextmanager
async def lifespan(app: FastAPI):
    try:
        logging.basicConfig(
            level=logging.INFO,
            format="%(asctime)s %(name)-12s %(levelname)-8s %(message)s",
            handlers=[RichHandler(rich_tracebacks=True)],
        )
        dotenv.load_dotenv()
        log = logging.getLogger("relayer")
        # Discover and ping resolvers from environment
        resolvers_env = os.getenv("RESOLVERS", "")
        healthy = []
        for addr in resolvers_env.split(","):  # ping each resolver health endpoint
            addr = addr.strip()
            if not addr:
                continue
            try:
                resp = requests.get(f"{addr}/health", timeout=5)
                if resp.status_code == 200:
                    healthy.append(addr)
                    log.info(f"Resolver {addr} is healthy")
                else:
                    log.warning(f"Resolver {addr} unhealthy status {resp.status_code}")
            except Exception as err:
                log.warning(f"Unable to reach resolver {addr}: {err}")
        app.state.resolvers = healthy
        log.info(f"Using {len(healthy)} healthy resolver(s): {healthy}")
        try:
            # Initialize watchers asynchronously
            await initialize_watchers(log)
        except Exception:
            pass
        yield
        print("Shutting down the application")
    except Exception as _:
        pass
    finally:
        pass


async def produce(watcher, queue):
    async for ev in watcher.watch_events():
        await queue.put(ev)


async def initialize_watchers(log: logging.Logger):
    mode = os.getenv("MODE", "D").upper()
    ethereum_watcher = None
    stellar_watcher = None

    if mode in ["E", "D"]:
        log.info(f"{mode} mode, initializing Ethereum watcher...")
        ETHEREUM_RPC = os.getenv("ETHEREUM_RPC") or ""
        if not ETHEREUM_RPC:
            log.error("ETHEREUM_RPC not set, using default")
            exit(1)

        ETHEREUM_ESCROW_ABI = os.getenv("ETHEREUM_ESCROW_ABI") or ""
        if not ETHEREUM_ESCROW_ABI:
            log.error("ETHEREUM_ESCROW_ABI not set, using default")
            exit(1)

        ETHEREUM_ESCROW_ADDRESS = os.getenv("ETHEREUM_ESCROW_ADDRESS") or ""
        if not ETHEREUM_ESCROW_ADDRESS:
            log.error("ETHEREUM_ESCROW_ADDRESS not set, using default")
            exit(1)

        ethereum_watcher = EthereumWatcher(
            ETHEREUM_RPC, ETHEREUM_ESCROW_ABI, ETHEREUM_ESCROW_ADDRESS
        )

    if mode in ["S", "D"]:
        log.info(f"{mode} mode, initializing Stellar watcher...")
        STELLAR_RPC = os.getenv("STELLAR_RPC") or ""
        if not STELLAR_RPC:
            log.error("STELLAR_RPC not set, using default")
            exit(1)

        STELLAR_CONTRACT_ID = os.getenv("STELLAR_CONTRACT_ID") or ""
        if not STELLAR_CONTRACT_ID:
            log.error("STELLAR_CONTRACT_ID not set, using default")
            exit(1)

        stellar_watcher = StellarWatcher(STELLAR_RPC, STELLAR_CONTRACT_ID)

    log.info("Starting relayer...")
    fsm = SwapStateMachine()
    queue = asyncio.Queue()
    producers = []
    if ethereum_watcher:
        producers.append(asyncio.create_task(produce(ethereum_watcher, queue)))
    if stellar_watcher:
        producers.append(asyncio.create_task(produce(stellar_watcher, queue)))
    try:
        while True:
            ev = await queue.get()
            fsm.handle(ev)
    finally:
        for p in producers:
            p.cancel()

app = FastAPI(lifespan=lifespan)
origins = [
    "*",
]

class OrderData(BaseModel):
    maker_pk: str
    salt: str
    src_chain: int
    dst_chain: int
    make_amount: str
    take_amount: str

class Order(BaseModel):
    order_data: OrderData
    signature: str

@app.post("/order")
async def create_order(order: Order):
    # Handle order creation
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
    uvicorn.run("src.relayer.main:app", host="0.0.0.0", port=8000, reload=True)
