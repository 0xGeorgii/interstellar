import asyncio, logging, os
import dotenv
from rich.logging import RichHandler

from .ethereum_watcher import EthereumWatcher
from .stellar_watcher import StellarWatcher
from .state_machine import SwapStateMachine

dotenv.load_dotenv()

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(name)-12s %(levelname)-8s %(message)s",
    handlers=[RichHandler(rich_tracebacks=True)],
)
log = logging.getLogger("relayer")


async def produce(watcher, queue):
    async for ev in watcher.watch_events():
        await queue.put(ev)


async def main():

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


if __name__ == "__main__":
    asyncio.run(main())
