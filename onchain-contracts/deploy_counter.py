import sys
from substrateinterface import SubstrateInterface, Keypair

URL = "wss://portaldot.philotheephilix.in"
WASM = "onchain-contracts/counter/target/wasm32-unknown-unknown/release/counter.wasm"
GAS = 500_000_000_000
ED  = 10**14  # 1 POT

def main():
    code = open(WASM, "rb").read()
    print(f"wasm bytes: {len(code)}")
    s = SubstrateInterface(url=URL, ss58_format=42, type_registry_preset="substrate-node-template")
    alice = Keypair.create_from_uri("//Alice", ss58_format=42)
    print("signer:", alice.ss58_address)

    # --- instantiate_with_code (old 5-arg pallet-contracts) ---
    call = s.compose_call("Contracts", "instantiate_with_code", {
        "endowment": ED,
        "gas_limit": GAS,
        "code": "0x" + code.hex(),
        "data": "0x",            # deploy() ignores input
        "salt": "0x01",
    })
    ext = s.create_signed_extrinsic(call=call, keypair=alice)
    rcpt = s.submit_extrinsic(ext, wait_for_inclusion=True)
    print("instantiate success:", rcpt.is_success)
    if not rcpt.is_success:
        print("ERROR:", rcpt.error_message); s.close(); sys.exit(1)
    addr = None
    for ev in rcpt.triggered_events:
        d = ev.value["event"]
        if d["module_id"] == "Contracts" and d["event_id"] in ("Instantiated", "ContractInstantiated"):
            attrs = d["attributes"]
            # attributes shape varies: dict or list
            if isinstance(attrs, dict):
                addr = attrs.get("contract") or attrs.get("contract_id")
            elif isinstance(attrs, (list, tuple)):
                addr = attrs[-1]
            print("Instantiated event:", attrs)
    print("contract address:", addr)
    if not addr:
        print("no address parsed; dumping events"); 
        for ev in rcpt.triggered_events: print(" ", ev.value["event"]["module_id"], ev.value["event"]["event_id"])
        s.close(); sys.exit(1)

    def read_counter():
        res = s.rpc_request("contracts_call", [{
            "origin": alice.ss58_address, "dest": addr, "value": 0,
            "gasLimit": GAS, "inputData": "0x00000002",
        }])
        return res

    def inc():
        c = s.compose_call("Contracts", "call", {
            "dest": {"Id": addr}, "value": 0, "gas_limit": GAS,
            "data": "0x00000001",
        })
        e = s.create_signed_extrinsic(call=c, keypair=alice)
        r = s.submit_extrinsic(e, wait_for_inclusion=True)
        return r

    print("\n--- dry-run get (#0) ---")
    print(read_counter())
    print("\n--- inc #1 ---")
    r1 = inc(); print("inc1 success:", r1.is_success)
    print("get:", read_counter())
    print("\n--- inc #2 ---")
    r2 = inc(); print("inc2 success:", r2.is_success)
    print("get:", read_counter())
    s.close()

if __name__ == "__main__":
    main()
