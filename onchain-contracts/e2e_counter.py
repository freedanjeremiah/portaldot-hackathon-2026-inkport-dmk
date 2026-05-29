"""End-to-end: deploy raw counter to live Portaldot, inc twice, verify reads. Proper assertions."""
from substrateinterface import SubstrateInterface, Keypair

URL="wss://portaldot.philotheephilix.in"
WASM="onchain-contracts/counter/counter.stripped.wasm"
GAS=500_000_000_000
SEL_INC="0x00000001"; SEL_GET="0x00000002"

def read_counter(s, alice, addr):
    r=s.rpc_request("contracts_call",[{"origin":alice.ss58_address,"dest":addr,"value":0,
        "gasLimit":GAS,"inputData":SEL_GET}])["result"]
    ok=r["result"]["Ok"]
    data=ok["data"]  # 0x + u32 LE
    raw=bytes.fromhex(data[2:])
    return int.from_bytes(raw[:4],"little") if len(raw)>=4 else None

def main():
    s=SubstrateInterface(url=URL,ss58_format=42,type_registry_preset="substrate-node-template")
    alice=Keypair.create_from_uri("//Alice",ss58_format=42)
    code=open(WASM,"rb").read()
    head=s.get_chain_head(); blk=s.get_block_number(head)
    salt="0x"+blk.to_bytes(4,"little").hex()  # unique-ish per run
    print(f"node block={blk} signer={alice.ss58_address} wasm={len(code)}B salt={salt}")

    call=s.compose_call("Contracts","instantiate_with_code",{
        "endowment":10*10**14,"gas_limit":GAS,"code":"0x"+code.hex(),"data":"0x","salt":salt})
    rcpt=s.submit_extrinsic(s.create_signed_extrinsic(call=call,keypair=alice),wait_for_inclusion=True)
    assert rcpt.is_success, f"instantiate failed: {rcpt.error_message}"
    print("INSTANTIATE ok | block:",rcpt.block_hash,"| fee:",rcpt.total_fee_amount)

    addr=None
    for ev in rcpt.triggered_events:
        e=ev.value["event"]
        if e["module_id"]=="Contracts" and e["event_id"] in("Instantiated","ContractInstantiated"):
            a=e["attributes"]
            addr = a[-1] if isinstance(a,(list,tuple)) else (a.get("contract") or a.get("contract_id"))
    assert addr, "no contract address in events"
    print("CONTRACT:",addr)

    v0=read_counter(s,alice,addr); print("get #0 ->",v0); assert v0==0, f"expected 0 got {v0}"

    for n in (1,2):
        c=s.compose_call("Contracts","call",{"dest":{"Id":addr},"value":0,"gas_limit":GAS,"data":SEL_INC})
        r=s.submit_extrinsic(s.create_signed_extrinsic(call=c,keypair=alice),wait_for_inclusion=True)
        assert r.is_success, f"inc{n} failed: {r.error_message}"
        v=read_counter(s,alice,addr); print(f"inc#{n} ok -> get ->",v); assert v==n, f"expected {n} got {v}"

    print("\n=== E2E PASS: deploy + 2x inc + reads all verified on live Portaldot ===")
    s.close()

if __name__=="__main__":
    main()
