"""Portaldot rent-era pallet-contracts client. Proven against wss://portaldot.philotheephilix.in."""
from substrateinterface import SubstrateInterface, Keypair

DEFAULT_URL = "wss://portaldot.philotheephilix.in"
GAS = 500_000_000_000
POT = 10**14

def _astr(a):
    if isinstance(a, dict): return a.get("value") or a.get("contract") or a.get("contract_id")
    if isinstance(a, (list, tuple)): return _astr(a[-1])
    return a

class Portaldot:
    def __init__(self, url=DEFAULT_URL, suri="//Alice"):
        self.s = SubstrateInterface(url=url, ss58_format=42, type_registry_preset="substrate-node-template")
        self.kp = Keypair.create_from_uri(suri, ss58_format=42)

    def block(self):
        return self.s.get_block_number(self.s.get_chain_head())

    def deploy(self, wasm_path, ctor_data="0x", endowment_pot=10, salt=None):
        code = open(wasm_path, "rb").read()
        if salt is None:
            salt = "0x" + self.block().to_bytes(4, "little").hex()
        call = self.s.compose_call("Contracts", "instantiate_with_code", {
            "endowment": endowment_pot * POT, "gas_limit": GAS,
            "code": "0x" + code.hex(), "data": ctor_data, "salt": salt})
        rcpt = self.s.submit_extrinsic(self.s.create_signed_extrinsic(call=call, keypair=self.kp), wait_for_inclusion=True)
        if not rcpt.is_success:
            raise RuntimeError(f"instantiate failed: {rcpt.error_message}")
        for ev in rcpt.triggered_events:
            e = ev.value["event"]
            if e["module_id"] == "Contracts" and e["event_id"] in ("Instantiated", "ContractInstantiated"):
                return _astr(e["attributes"]), rcpt
        raise RuntimeError("no contract address in events")

    def call(self, addr, data):
        c = self.s.compose_call("Contracts", "call", {
            "dest": {"Id": addr}, "value": 0, "gas_limit": GAS, "data": data})
        rcpt = self.s.submit_extrinsic(self.s.create_signed_extrinsic(call=c, keypair=self.kp), wait_for_inclusion=True)
        if not rcpt.is_success:
            raise RuntimeError(f"call failed: {rcpt.error_message}")
        return rcpt

    def read(self, addr, data):
        """Dry-run a message; return raw output bytes."""
        r = self.s.rpc_request("contracts_call", [{
            "origin": self.kp.ss58_address, "dest": addr, "value": 0,
            "gasLimit": GAS, "inputData": data}])["result"]
        ok = r["result"]["Ok"]
        return bytes.fromhex(ok["data"][2:])

    def close(self):
        self.s.close()
