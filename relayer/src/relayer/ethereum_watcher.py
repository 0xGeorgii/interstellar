from web3 import Web3
from web3.middleware import ExtraDataToPOAMiddleware
import asyncio, json, logging

class EthereumWatcher:
    def __init__(self, rpc_url: str, abi_path: str, escrow_address: str):
        self.w3  = Web3(Web3.HTTPProvider(rpc_url, request_kwargs={"timeout": 60}))
        self.w3.middleware_onion.inject(ExtraDataToPOAMiddleware, layer=0)
        self.htlc = self.w3.eth.contract(abi=json.load(open(abi_path)),
                                         address=self.w3.to_checksum_address(escrow_address))
        self.log  = logging.getLogger("EthereumWatcher")

    async def watch_events(self):
        event_filter = self.htlc.events.Deposited.create_filter(fromBlock="latest")  # web3.py filters :contentReference[oaicite:1]{index=1}
        while True:
            for entry in event_filter.get_new_entries():
                self.log.info(f"Deposit on Ethereum: {entry['args']}")
                yield {"chain": "eth", "event": entry["event"], "data": dict(entry["args"])}
            await asyncio.sleep(2)
