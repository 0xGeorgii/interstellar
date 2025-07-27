import enum
import asyncio, logging
import requests
from xmlrpc.client import DateTime
from typing import Any, Dict, List

import stellar_sdk
from stellar_sdk.soroban_server_async import SorobanServerAsync


class StellarWatcherStatus(enum.Enum):
    RUNNING = "running"
    STOPPED = "stopped"


class StellarWatcher:
    def __init__(self, rpc_url: str, contract_id: str):
        self.rpc_url = rpc_url
        self.server = SorobanServerAsync(rpc_url)
        self.contract_id = contract_id
        self.cursor = None
        self.page_size = 100
        self.log = logging.getLogger("StellarWatcher")
        self.status: StellarWatcherStatus = StellarWatcherStatus.RUNNING

    async def watch_events(self):
        last_ledger = await self.server.get_latest_ledger()
        backfill_ledger = max(0, last_ledger.sequence - 2000)
        self.log.info(
            f"Starting event watch on contract {self.contract_id} (backfill from ledger {backfill_ledger})..."
        )
        first = True
        while True:
            try:
                if first:
                    resp = await asyncio.to_thread(
                        requests.post,
                        self.rpc_url,
                        json=self.make_request(start_ledger=backfill_ledger),
                    )
                    backfill_ledger = resp.json()["result"]["latestLedger"]
                    first = False
                else:
                    resp = await asyncio.to_thread(
                        requests.post,
                        self.rpc_url,
                        json=self.make_request(
                            start_ledger=backfill_ledger, cursor=self.cursor
                        ),
                    )
                self.log.debug(f"RPC response (cursor={self.cursor}): {resp}")
                resp_data = resp.json()
                self.cursor = resp_data["result"]["cursor"]

                if not resp_data["result"]["events"]:
                    self.log.info("No new events found, sleeping...")
                    await asyncio.sleep(3)
                    continue
                for ev in resp_data["result"]["events"]:
                    event = StellarEvent.from_rpc_response(ev)
                    self.log.info(f"Stellar event: {event}")
                    try:
                        yield {"chain": "xlm", "event": event.type, "data": []}
                    except Exception as e:
                        self.log.error(f"Error decoding event data: {e}")
                        continue

            except Exception as e:
                self.log.error(f"Error fetching events: {e}")
                await asyncio.sleep(3)
                continue

            await asyncio.sleep(3)

    def make_request(self, start_ledger=None, cursor=None) -> Dict:
        base = {
            "jsonrpc": "2.0",
            "id": 8675309,
            "method": "getEvents",
            "params": {
                "filters": [
                    {
                        "type": "contract",
                        "contractIds": [self.contract_id],
                        "topics": [
                            [stellar_sdk.scval.to_symbol("1inch_order_created").to_xdr()]
                        ],
                    }
                ],
                "pagination": {
                    "limit": self.page_size,
                },
            },
        }

        if start_ledger is not None:
            base["params"]["startLedger"] = start_ledger
        if cursor is not None:
            base["params"]["cursor"] = cursor

        return base


class StellarEventType(enum.Enum):
    CONTRACT = "contract"


class StellarEvent:

    def __init__(self) -> None:
        self.id: str = ""
        self.type: StellarEventType = StellarEventType.CONTRACT
        self.ledger: int = 0
        self.ledger_closed_at: DateTime = DateTime(
            "1970-01-01T00:00:00"
        )
        self.contract_id: str = ""
        self.operation_index: int = 0
        self.transaction_index: int = 0
        self.tx_hash: str = ""
        self.is_successful_contract_call: bool = False
        self.topics: List[str] = []
        self.value: Any = ""

    def __str__(self) -> str:
        return (
            f"StellarEvent(id={self.id}, type={self.type}, ledger={self.ledger}, "
            f"ledger_closed_at={self.ledger_closed_at}, contract_id={self.contract_id}, "
            f"operation_index={self.operation_index}, transaction_index={self.transaction_index}, "
            f"tx_hash={self.tx_hash}, is_successful_contract_call={self.is_successful_contract_call}, "
            f"topics={self.topics}, value={self.value})"
        )

    @staticmethod
    def from_rpc_response(data: Dict) -> "StellarEvent":
        """
        Create a StellarEvent instance from raw RPC response data.
        """
        event = StellarEvent()
        event.id = data.get("id", "")
        event.type = StellarEventType(data.get("type", "contract"))
        event.ledger = data.get("ledger", 0)
        event.ledger_closed_at = DateTime(
            data.get("ledgerClosedAt", "1970-01-01T00:00:00")
        )
        event.contract_id = data.get("contractId", "")
        event.operation_index = data.get("operationIndex", 0)
        event.transaction_index = data.get("transactionIndex", 0)
        event.tx_hash = data.get("txHash", "")
        event.is_successful_contract_call = data.get("isSuccessfulContractCall", False)
        for topic in data.get("topic", []):
            if topic is None:
                continue
            sc_val = stellar_sdk.stellar_xdr.SCVal.from_xdr(topic)
            if sc_val.sym is not None:
                event.topics.append(stellar_sdk.scval.from_symbol(sc_val))
            if sc_val.address is not None:
                event.topics.append(stellar_sdk.scval.from_address(sc_val).address)
        if (event_value := data.get("value", "None")) is not None:
            event.value = StellarEvent.value_from_scval(event_value)
        return event

    @staticmethod
    def value_from_scval(event_value: str) -> Any:
        value_sc_val = stellar_sdk.stellar_xdr.SCVal.from_xdr(event_value)
        if value_sc_val.vec is not None:
            res = ""
            for item in value_sc_val.vec.sc_vec:
                res += str(StellarEvent.value_from_scval(item.to_xdr()))
            return res
        if value_sc_val.u32 is not None:
            return value_sc_val.u32.uint32
        if value_sc_val.i128 is not None:
            return stellar_sdk.scval.from_int128(value_sc_val)
        return None
