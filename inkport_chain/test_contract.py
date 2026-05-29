"""Generic on-chain test harness for InkPort seal0 contracts.

Given a stripped wasm + its metadata.json + a list of (action, args, expected)
steps, deploy to the live Portaldot node and assert real behaviour. Zero mock:
all reads are dry-run `contracts_call` RPCs, all mutations are real
`Contracts.call` extrinsics.

Encoding follows docs/seal-backend-spec.md:
  - call input  = 4-byte selector ++ SCALE(args in order)
  - ctor input  = SCALE(ctor args)  (no selector)
  - scalar SCALE: uintN/u128 -> 16 bytes LE; bool -> 1 byte (00/01)
  - return: SCALE(value); empty when no return.

Steps are tuples:
  ("read",   name, args, expected)  -> dry-run, decode per metadata.ret, assert ==
  ("call",   name, args, None)      -> real extrinsic, assert success
  ("revert", name, args, None)      -> dry-run, assert the contract reverted
"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from inkport_chain.portaldot import Portaldot, GAS  # noqa: E402


# --------------------------------------------------------------------------
# SCALE encode/decode for the scalar tier.
# --------------------------------------------------------------------------
def encode_arg(ty, value):
    if ty == "bool":
        return bytes([1 if value else 0])
    if ty == "u128":
        return int(value).to_bytes(16, "little")
    if ty == "address":
        # value is 32 raw bytes or hex
        if isinstance(value, str):
            value = bytes.fromhex(value[2:] if value.startswith("0x") else value)
        assert len(value) == 32, "address must be 32 bytes"
        return value
    raise ValueError(f"unknown arg type {ty}")


def decode_ret(ty, raw):
    if ty is None:
        return None
    if ty == "bool":
        return raw[0] != 0 if raw else False
    if ty == "u128":
        return int.from_bytes(raw[:16], "little") if raw else 0
    raise ValueError(f"unknown ret type {ty}")


def encode_call(selector_hex, arg_types, args):
    sel = bytes.fromhex(selector_hex[2:])  # 4-byte big-endian selector
    body = b"".join(encode_arg(t, a) for t, a in zip(arg_types, args))
    return "0x" + (sel + body).hex()


def encode_ctor(arg_types, args):
    body = b"".join(encode_arg(t, a) for t, a in zip(arg_types, args))
    return "0x" + body.hex()


# --------------------------------------------------------------------------
# Harness
# --------------------------------------------------------------------------
class ContractTester:
    def __init__(self, wasm_path, metadata_path, ctor_args, url=None):
        self.meta = json.loads(Path(metadata_path).read_text())
        self.wasm = wasm_path
        self.by_name = {m["name"]: m for m in self.meta["messages"]}
        self.p = Portaldot(url) if url else Portaldot()
        self.ctor_args = ctor_args
        self.addr = None

    def deploy(self):
        ctor_types = self.meta["constructor"]["args"]
        data = encode_ctor(ctor_types, self.ctor_args)
        self.addr, rcpt = self.p.deploy(self.wasm, ctor_data=data)
        print(f"  DEPLOY ok -> {self.addr}  (ctor args={self.ctor_args})")
        return self.addr

    def _dry_run(self, name, args):
        m = self.by_name[name]
        data = encode_call(m["selector"], m["args"], args)
        r = self.p.s.rpc_request("contracts_call", [{
            "origin": self.p.kp.ss58_address, "dest": self.addr, "value": 0,
            "gasLimit": GAS, "inputData": data}])["result"]
        return r["result"], m

    def read(self, name, args, expected):
        res, m = self._dry_run(name, args)
        assert "Ok" in res, f"{name}() reverted unexpectedly: {res}"
        raw = bytes.fromhex(res["Ok"]["data"][2:])
        got = decode_ret(m.get("ret"), raw)
        status = "PASS" if got == expected else "FAIL"
        print(f"  READ  {name}({_fmt(args)}) -> {got}  expected {expected}  [{status}]")
        assert got == expected, f"{name}: expected {expected}, got {got}"

    def call(self, name, args, _expected=None):
        m = self.by_name[name]
        data = encode_call(m["selector"], m["args"], args)
        self.p.call(self.addr, data)
        print(f"  CALL  {name}({_fmt(args)}) -> extrinsic ok")

    def revert(self, name, args, _expected=None):
        res, _ = self._dry_run(name, args)
        # A revert surfaces either as an Err result or an Ok with flags bit0 set
        # (revert flag). Accept either.
        reverted = "Err" in res
        if not reverted and "Ok" in res:
            flags = res["Ok"].get("flags", 0)
            reverted = (int(flags) & 1) == 1
        status = "PASS" if reverted else "FAIL"
        print(f"  REVERT {name}({_fmt(args)}) -> reverted={reverted}  [{status}]")
        assert reverted, f"{name}({args}) was expected to revert but did not: {res}"

    def run(self, steps):
        for action, name, args, expected in steps:
            getattr(self, action)(name, args, expected)

    def close(self):
        self.p.close()


def _fmt(args):
    return ", ".join(str(a) for a in args)


def run_contract_test(name, wasm_path, metadata_path, ctor_args, steps, url=None):
    """Deploy `wasm_path` and run `steps`, printing a PASS/FAIL log."""
    print(f"\n=== {name}: on-chain test ===")
    t = ContractTester(wasm_path, metadata_path, ctor_args, url=url)
    try:
        t.deploy()
        t.run(steps)
        print(f"=== {name}: ALL ASSERTIONS PASSED ===")
        return True
    finally:
        t.close()
