"""Portaldot rent-era pallet-contracts client. Proven against wss://portaldot.philotheephilix.in."""
import time
from substrateinterface import SubstrateInterface, Keypair

DEFAULT_URL = "wss://portaldot.philotheephilix.in"
GAS = 500_000_000_000
POT = 10**14

# transient websocket errors worth reconnecting + retrying on
_TRANSIENT = ("connection", "websocket", "broken pipe", "closed", "reset", "timed out", "eof")

def _is_transient(exc):
    return any(t in str(exc).lower() for t in _TRANSIENT)

def _astr(a):
    if isinstance(a, dict): return a.get("value") or a.get("contract") or a.get("contract_id")
    if isinstance(a, (list, tuple)): return _astr(a[-1])
    return a

class Portaldot:
    def __init__(self, url=DEFAULT_URL, suri="//Alice"):
        self.url = url
        self.s = SubstrateInterface(url=url, ss58_format=42, type_registry_preset="substrate-node-template")
        self.kp = Keypair.create_from_uri(suri, ss58_format=42)

    def _reconnect(self):
        try: self.s.close()
        except Exception: pass
        self.s = SubstrateInterface(url=self.url, ss58_format=42, type_registry_preset="substrate-node-template")

    def _retry(self, fn, tries=4):
        """Run a network op, reconnecting + retrying on transient socket errors."""
        last = None
        for i in range(tries):
            try:
                return fn()
            except Exception as e:
                last = e
                if not _is_transient(e):
                    raise
                time.sleep(1.5 * (i + 1))
                self._reconnect()
        raise last

    def block(self):
        return self._retry(lambda: self.s.get_block_number(self.s.get_chain_head()))

    def deploy(self, wasm_path, ctor_data="0x", endowment_pot=10, salt=None):
        code = open(wasm_path, "rb").read()
        import os
        def attempt():
            # fresh random salt per attempt so a retry after a dropped socket can't collide
            s = salt if salt is not None else "0x" + os.urandom(8).hex()
            call = self.s.compose_call("Contracts", "instantiate_with_code", {
                "endowment": endowment_pot * POT, "gas_limit": GAS,
                "code": "0x" + code.hex(), "data": ctor_data, "salt": s})
            rcpt = self.s.submit_extrinsic(self.s.create_signed_extrinsic(call=call, keypair=self.kp), wait_for_inclusion=True)
            if not rcpt.is_success:
                raise RuntimeError(f"instantiate failed: {rcpt.error_message}")
            for ev in rcpt.triggered_events:
                e = ev.value["event"]
                if e["module_id"] == "Contracts" and e["event_id"] in ("Instantiated", "ContractInstantiated"):
                    return _astr(e["attributes"]), rcpt
            raise RuntimeError("no contract address in events")
        return self._retry(attempt)

    def signer(self, suri):
        """Return a fresh dev keypair for `suri` (e.g. '//Bob')."""
        return Keypair.create_from_uri(suri, ss58_format=42)

    def call(self, addr, data, value=0, keypair=None):
        kp = keypair or self.kp
        def attempt():
            c = self.s.compose_call("Contracts", "call", {
                "dest": {"Id": addr}, "value": value, "gas_limit": GAS, "data": data})
            rcpt = self.s.submit_extrinsic(self.s.create_signed_extrinsic(call=c, keypair=kp), wait_for_inclusion=True)
            if not rcpt.is_success:
                raise RuntimeError(f"call failed: {rcpt.error_message}")
            return rcpt
        return self._retry(attempt)

    def events(self, rcpt):
        """Extract raw ContractEmitted event data bytes from a receipt.

        Returns a list of bytes payloads (the seal_deposit_event `data`)."""
        out = []
        for ev in rcpt.triggered_events:
            e = ev.value["event"]
            if e["module_id"] == "Contracts" and e["event_id"] in ("ContractEmitted", "Emitted"):
                attrs = e["attributes"]
                data = None
                if isinstance(attrs, dict):
                    data = attrs.get("data")
                elif isinstance(attrs, (list, tuple)):
                    # list-of-attrs form: [{AccountId}, {Vec<u8> data}].
                    data = attrs[-1]
                # Unwrap a {"type":..,"value":..} attribute wrapper.
                if isinstance(data, dict):
                    data = data.get("value")
                if isinstance(data, str) and data.startswith("0x"):
                    out.append(bytes.fromhex(data[2:]))
                elif isinstance(data, (list, tuple)):
                    out.append(bytes(data))
        return out

    def read(self, addr, data, origin=None, value=0):
        """Dry-run a message; return raw output bytes."""
        def attempt():
            r = self.s.rpc_request("contracts_call", [{
                "origin": origin or self.kp.ss58_address, "dest": addr, "value": value,
                "gasLimit": GAS, "inputData": data}])["result"]
            ok = r["result"]["Ok"]
            return bytes.fromhex(ok["data"][2:])
        return self._retry(attempt)

    def close(self):
        self.s.close()
