from contextlib import asynccontextmanager
import asyncio, logging, os
import aiohttp
import dotenv
from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from rich.logging import RichHandler
import uvicorn
from pydantic import BaseModel
import requests

from .db import InterstellarDb, Order

from .ethereum_watcher import EthereumWatcher
from .stellar_watcher import StellarWatcher
from .state_machine import SwapStateMachine


@asynccontextmanager
async def lifespan(app: FastAPI):
    log = logging.getLogger("relayer")
    try:
        logging.basicConfig(
            level=logging.INFO,
            format="%(asctime)s %(name)-12s %(levelname)-8s %(message)s",
            handlers=[RichHandler(rich_tracebacks=True)],
        )
        dotenv.load_dotenv()
        InterstellarDb()
        resolvers_env = os.getenv("RESOLVERS", "")
        healthy = []
        for addr in resolvers_env.split(","):
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

        # Run watchers in the background
        watchers_task = asyncio.create_task(initialize_watchers(log))

        try:
            yield
        finally:
            # Ensure background tasks are cancelled on shutdown
            watchers_task.cancel()
            await watchers_task
            print("Shutting down the application")
    except Exception as e:
        log.error(f"Lifespan error: {e}")


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

@app.post("/order")
async def create_order(order: Order):
    interstellar_order = InterstellarDb().create_order(order)
    for resolver in app.state.resolvers:
        try:
            async with aiohttp.ClientSession() as session:
                async with session.post(f"{resolver}/order", json=interstellar_order) as response:
                    if response.status != 200:
                        logging.error(f"Failed to notify resolver {resolver}: {await response.text()}")
        except Exception as e:
            logging.error(f"Error notifying resolver {resolver}: {e}")
    logging.info(f"Order created with ID: {interstellar_order.order_id}")
    return {"status": "success", "order_id": interstellar_order.order_id}

@app.get("/order_status")
async def get_order_status(order_id: str):
    status = "pending" if order_id else "escrow_id"
    return {"status": status}

class Secret(BaseModel):
    maker_address: str
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
