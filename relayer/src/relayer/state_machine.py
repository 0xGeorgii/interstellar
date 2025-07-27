import logging

class SwapStateMachine:
    def __init__(self):
        self.log = logging.getLogger("FSM")
        self.swaps = {}  # hash -> dict of eth/xlm deposit/claim

    def handle(self, ev):
        return
        h = ev["data"].get("hash") or ev["data"]
        swap = self.swaps.setdefault(h, {})
        swap[ev["chain"]] = ev
        if "eth" in swap and "xlm" in swap:
            self.log.info(f"✳ BOTH deposits seen for HTLC {h[:8]}… – ready for claims")
        if swap.get("eth") and ev["event"] == "Claimed":
            self.log.info(f"✔ Ethereum claimed, swap complete")
