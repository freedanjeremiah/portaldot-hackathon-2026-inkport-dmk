"""inkport — Solidity -> seal0 -> Portaldot pipeline CLI.

One tool that ties the proven InkPort seal0 pipeline together:

  translate  .sol -> seal0 Rust + metadata.json (via inkport-translate)
  build      cargo build the crate to wasm + strip -> build/<Name>/<Name>.wasm
  deploy     deploy to a network, encoding ctor args from metadata
  call       send (mutating) or dry-run (view) a message, decode the result
  test       run tests/<Name>.json step specs against the live node
  all        translate+build+deploy+test every contract that has a test spec

Everything is metadata-driven (build/<Name>/metadata.json): no per-contract
logic lives in this file. Chain + encoding logic is reused from inkport_chain/.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import List, Optional

import typer

from .config import load_config
from . import pipeline as P

app = typer.Typer(
    help="InkPort — Solidity → seal0 → Portaldot pipeline",
    add_completion=False,
    no_args_is_help=True,
)


# --------------------------------------------------------------------------
# Lazily import the proven chain harness (needs project root on sys.path).
# --------------------------------------------------------------------------
def _chain(root: Path):
    P._add_chain_to_path(root)
    from inkport_chain import portaldot, strip_wasm, test_contract  # noqa: E402

    return portaldot, strip_wasm, test_contract


def _err(msg: str):
    typer.secho(msg, fg=typer.colors.RED, err=True)


def _ok(msg: str):
    typer.secho(msg, fg=typer.colors.GREEN)


def _resolve_network(cfg, network):
    """Resolve a network from config, exiting cleanly on an unknown name."""
    try:
        return cfg.network(network)
    except KeyError as e:
        _err(str(e).strip('"'))
        raise typer.Exit(2)


# ==========================================================================
# translate
# ==========================================================================
@app.command()
def translate(
    file: str = typer.Argument(..., help="Path to the .sol contract"),
    out: Optional[str] = typer.Option(None, "--out", help="Output dir (default build/<Name>)"),
):
    """Translate a Solidity file to a seal0 Rust crate + metadata.json."""
    cfg = load_config()
    root = cfg.root
    sol = Path(file)
    if not sol.is_absolute():
        sol = (root / sol) if not sol.exists() else sol.resolve()
    if not sol.exists():
        _err(f"no such file: {file}")
        raise typer.Exit(2)
    name = sol.stem
    out_dir = Path(out) if out else P.build_dir(root, name)
    out_dir.mkdir(parents=True, exist_ok=True)

    binp = P.translator_bin(root)
    if not binp.exists():
        _err(f"translator binary not found: {binp}\n"
             f"Build it: source ~/.cargo/env && cd translator && cargo build --release")
        raise typer.Exit(1)

    r = P.run([binp, sol, "--target", "seal", "--out", out_dir], env=P.cargo_env())
    if r.returncode != 0:
        _err(r.stdout + r.stderr)
        raise typer.Exit(1)
    _ok(f"translated {name} -> {out_dir}")
    typer.echo(f"  metadata: {P.metadata_path(root, name)}")
    return name


# ==========================================================================
# build
# ==========================================================================
@app.command()
def build(name: str = typer.Argument(..., help="Contract name (e.g. ERC20)")):
    """cargo build the generated crate to wasm, strip it -> build/<Name>/<Name>.wasm."""
    cfg = load_config()
    root = cfg.root
    crate = P.build_dir(root, name)
    if not (crate / "Cargo.toml").exists():
        _err(f"crate not found: {crate} (run `inkport translate contracts/{name}.sol` first)")
        raise typer.Exit(1)

    r = P.run(
        ["cargo", "+stable", "build", "--release",
         "--target", "wasm32-unknown-unknown"],
        cwd=crate, env=P.cargo_env(),
    )
    if r.returncode != 0:
        _err("cargo build failed:")
        _err(r.stdout + r.stderr)
        raise typer.Exit(1)

    crate_name = P.crate_name_for(root, name)
    raw = crate / "target" / "wasm32-unknown-unknown" / "release" / f"{crate_name}.wasm"
    if not raw.exists():
        _err(f"expected wasm not produced: {raw}")
        _err(r.stdout + r.stderr)
        raise typer.Exit(1)

    _, strip_wasm, _ = _chain(root)
    out = P.wasm_path(root, name)
    n = strip_wasm.strip(str(raw), str(out))
    _ok(f"built {name} -> {out} ({n} bytes, stripped)")
    return str(out)


# ==========================================================================
# deploy
# ==========================================================================
@app.command()
def deploy(
    name: str = typer.Argument(...),
    arg: List[str] = typer.Option([], "--arg", help="Constructor arg (repeatable)"),
    value: int = typer.Option(0, "--value", help="Endowment in POT"),
    network: Optional[str] = typer.Option(None, "--network"),
    signer: Optional[str] = typer.Option(None, "--signer"),
):
    """Deploy a built contract; encode ctor args from metadata; save the address."""
    cfg = load_config()
    root = cfg.root
    net = _resolve_network(cfg, network)
    signer = signer or cfg.default_signer

    wasm = P.wasm_path(root, name)
    if not wasm.exists():
        _err(f"wasm not found: {wasm} (run `inkport build {name}` first)")
        raise typer.Exit(1)
    meta = P.load_metadata(root, name)
    portaldot, _, test_contract = _chain(root)

    ctor_types = meta["constructor"]["args"]
    ctor_args = P.coerce_args(ctor_types, list(arg))
    data = test_contract.encode_ctor(ctor_types, ctor_args)

    p = portaldot.Portaldot(url=net["url"], suri=signer)
    try:
        addr, _ = p.deploy(str(wasm), ctor_data=data, endowment_pot=value or 10)
    finally:
        p.close()

    _save_deployment(root, net["name"], name, addr)
    _ok(f"deployed {name} -> {addr}  (network={net['name']}, signer={signer})")
    return addr


def _save_deployment(root: Path, network: str, name: str, addr: str):
    path = P.deployments_path(root, network)
    path.parent.mkdir(parents=True, exist_ok=True)
    data = {}
    if path.exists():
        data = json.loads(path.read_text())
    data[name] = addr
    path.write_text(json.dumps(data, indent=2) + "\n")


def _load_deployment(root: Path, network: str, name: str) -> str:
    path = P.deployments_path(root, network)
    if not path.exists():
        raise FileNotFoundError(
            f"no deployments file {path}; deploy {name} first")
    data = json.loads(path.read_text())
    if name not in data:
        raise KeyError(f"{name} not deployed on {network}; run `inkport deploy {name}`")
    return data[name]


# ==========================================================================
# call
# ==========================================================================
@app.command()
def call(
    name: str = typer.Argument(...),
    message: str = typer.Argument(...),
    arg: List[str] = typer.Option([], "--arg", help="Message arg (repeatable)"),
    signer: Optional[str] = typer.Option(None, "--signer"),
    value: int = typer.Option(0, "--value", help="Value in POT (mutating calls)"),
    network: Optional[str] = typer.Option(None, "--network"),
):
    """Call a message. Mutating -> send extrinsic; view -> dry-run + decode result."""
    cfg = load_config()
    root = cfg.root
    net = _resolve_network(cfg, network)
    signer = signer or cfg.default_signer

    meta = P.load_metadata(root, name)
    m = P.message_by_name(meta, message, len(arg))
    portaldot, _, test_contract = _chain(root)

    args = P.coerce_args(m["args"], list(arg))
    data = test_contract.encode_call(m["selector"], m["args"], args)
    addr = _load_deployment(root, net["name"], name)
    pot = net.get("decimals", 14)
    value_planck = value * (10 ** pot)

    p = portaldot.Portaldot(url=net["url"], suri=signer)
    try:
        if m.get("mutates"):
            # Contracts.call's extrinsic success reflects dispatch, not a
            # contract revert. Dry-run first and honor the revert bit
            # (flags & 1) — mirror the read path — so a reverting mutating
            # message reports a clear error with a non-zero exit instead of a
            # bogus `-> ok`.
            origin = p.signer(signer).ss58_address
            dr = p.s.rpc_request("contracts_call", [{
                "origin": origin, "dest": addr, "value": value_planck,
                "gasLimit": portaldot.GAS, "inputData": data}])["result"]
            dres = dr["result"]
            reverted = "Ok" not in dres
            if not reverted and "Ok" in dres:
                reverted = (int(dres["Ok"].get("flags", 0)) & 1) == 1
            if reverted:
                _err(f"{message} reverted: {dres}")
                raise typer.Exit(1)
            kp = p.signer(signer)
            rcpt = p.call(addr, data, value=value_planck, keypair=kp)
            evs = p.events(rcpt)
            _ok(f"call {name}.{message}({', '.join(map(str, args))}) as {signer} -> ok"
                + (f"  ({len(evs)} event(s))" if evs else ""))
            # Decode emitted events against the metadata schema when possible.
            for raw in evs:
                ev = _match_event(meta, raw, test_contract)
                if ev:
                    typer.echo(f"  event {ev[0]} {ev[1]}")
        else:
            origin = p.signer(signer).ss58_address
            r = p.s.rpc_request("contracts_call", [{
                "origin": origin, "dest": addr, "value": value_planck,
                "gasLimit": portaldot.GAS, "inputData": data}])["result"]
            res = r["result"]
            # A read can fail two ways: the node returns `Err`, or it returns
            # `Ok` with the contract-revert bit set in `flags` (flags & 1). The
            # latter still carries decodable bytes, so checking only for "Ok"
            # would print a bogus value for a reverting view. Mirror `test`'s
            # revert detection and surface a clear error instead.
            reverted = "Ok" not in res
            if not reverted and "Ok" in res:
                reverted = (int(res["Ok"].get("flags", 0)) & 1) == 1
            if reverted:
                _err(f"{message} reverted: {res}")
                raise typer.Exit(1)
            raw = bytes.fromhex(res["Ok"]["data"][2:])
            got = test_contract.decode_ret(m.get("ret"), raw)
            _ok(f"call {name}.{message}({', '.join(map(str, args))}) -> {got}")
            typer.echo(json.dumps({"result": _jsonable(got)}))
            return got
    finally:
        p.close()


def _match_event(meta: dict, raw: bytes, test_contract):
    for em in meta.get("events", []):
        exp = sum(32 if f["type"] == "address" else (1 if f["type"] == "bool" else 16)
                  for f in em["fields"])
        if len(raw) == exp:
            return em["name"], test_contract.decode_event(em["fields"], raw)
    return None


def _jsonable(v):
    if isinstance(v, bytes):
        return "0x" + v.hex()
    return v


# ==========================================================================
# test
# ==========================================================================
@app.command()
def test(
    name: str = typer.Argument(...),
    network: Optional[str] = typer.Option(None, "--network"),
):
    """Run tests/<Name>.json step specs against the live node; PASS/FAIL per step."""
    cfg = load_config()
    root = cfg.root
    net = _resolve_network(cfg, network)
    ok = _run_test_spec(root, net, name)
    raise typer.Exit(0 if ok else 1)


def _run_test_spec(root: Path, net: dict, name: str) -> bool:
    spec_path = root / "tests" / f"{name}.json"
    if not spec_path.exists():
        _err(f"no test spec: {spec_path}")
        return False
    spec = json.loads(spec_path.read_text())
    meta = P.load_metadata(root, name)
    portaldot, _, test_contract = _chain(root)
    POT = 10 ** net.get("decimals", 14)

    typer.echo(f"\n=== {name}: on-chain test ({net['name']}) ===")
    deployer = spec.get("deployer", "//Alice")
    p = portaldot.Portaldot(url=net["url"], suri=deployer)
    addr = None
    last_events: list[bytes] = []
    passed = True
    # Dependency contract addresses (for cross-contract tests), keyed by label.
    deps: dict[str, str] = {}

    def _val(step) -> int:
        return int(step.get("value", 0)) * POT

    def _resolve(raw_args):
        """Substitute `@label` tokens with the dependency contract's address
        (as a 0x-prefixed 32-byte pubkey hex), so cross-contract callers can
        pass a deployed callee's address."""
        out = []
        for a in raw_args:
            if isinstance(a, str) and a.startswith("@"):
                label = a[1:]
                if label not in deps:
                    raise KeyError(f"dependency '{label}' not deployed (use deploy_dep)")
                # AccountId32 ss58 -> 0x pubkey hex via the harness keypair.
                from substrateinterface import Keypair
                kp = Keypair(ss58_address=deps[label], ss58_format=net.get("ss58", 42))
                out.append("0x" + kp.public_key.hex())
            else:
                out.append(a)
        return out

    try:
        for i, step in enumerate(spec["steps"]):
            action = step["action"]
            try:
                if action == "deploy":
                    ctor_types = meta["constructor"]["args"]
                    ctor_args = P.coerce_args(ctor_types, step.get("args", []))
                    data = test_contract.encode_ctor(ctor_types, ctor_args)
                    addr, _ = p.deploy(str(P.wasm_path(root, name)), ctor_data=data,
                                       endowment_pot=int(step.get("value", 10)))
                    _save_deployment(root, net["name"], name, addr)
                    typer.echo(f"  [PASS] deploy({ctor_args}) -> {addr}")
                    continue

                if action == "deploy_dep":
                    dep_name = step["name"]
                    dep_meta = P.load_metadata(root, dep_name)
                    dep_ctor_types = dep_meta["constructor"]["args"]
                    dep_args = P.coerce_args(dep_ctor_types, _resolve(step.get("args", [])))
                    dep_data = test_contract.encode_ctor(dep_ctor_types, dep_args)
                    dep_addr, _ = p.deploy(str(P.wasm_path(root, dep_name)),
                                           ctor_data=dep_data,
                                           endowment_pot=int(step.get("value", 10)))
                    _save_deployment(root, net["name"], dep_name, dep_addr)
                    deps[step.get("as", dep_name)] = dep_addr
                    typer.echo(f"  [PASS] deploy_dep {dep_name}({dep_args}) -> {dep_addr}")
                    continue

                if action == "event":
                    em = next(e for e in meta["events"] if e["name"] == step["name"])
                    exp_len = sum(32 if f["type"] == "address" else (1 if f["type"] == "bool" else 16)
                                  for f in em["fields"])
                    expected = {k: P.coerce_arg(_field_ty(em, k), v)
                                for k, v in step["expected"].items()}
                    match = None
                    for raw in last_events:
                        if len(raw) != exp_len:
                            continue
                        fields = test_contract.decode_event(em["fields"], raw)
                        if all(fields.get(k) == v for k, v in expected.items()):
                            match = fields
                            break
                    assert match, f"event {step['name']}{expected} not found"
                    typer.echo(f"  [PASS] event {step['name']} {step['expected']}")
                    continue

                msg = step["message"]
                m = P.message_by_name(meta, msg, len(step.get("args", [])))
                args = P.coerce_args(m["args"], _resolve(step.get("args", [])))
                data = test_contract.encode_call(m["selector"], m["args"], args)
                signer = step.get("signer", deployer)

                if action == "call":
                    # The Contracts.call extrinsic's `is_success` reflects only
                    # the *dispatch* result, not a contract-level revert — a
                    # reverting message still lands on-chain as a "successful"
                    # extrinsic. Dry-run first (contracts_call) and honor the
                    # revert bit (flags & 1) so a silent revert FAILS the step
                    # instead of being reported as a pass.
                    origin = p.signer(signer).ss58_address
                    dr = p.s.rpc_request("contracts_call", [{
                        "origin": origin, "dest": addr, "value": _val(step),
                        "gasLimit": portaldot.GAS, "inputData": data}])["result"]
                    dres = dr["result"]
                    reverted = "Ok" not in dres
                    if not reverted and "Ok" in dres:
                        reverted = (int(dres["Ok"].get("flags", 0)) & 1) == 1
                    assert not reverted, f"{msg}({_fmt(args)}) reverted: {dres}"
                    kp = p.signer(signer)
                    rcpt = p.call(addr, data, value=_val(step), keypair=kp)
                    last_events = list(p.events(rcpt))
                    typer.echo(f"  [PASS] call {msg}({_fmt(args)}) as {signer}"
                               + (f"  ({len(last_events)} event(s))" if last_events else ""))

                elif action == "read":
                    origin = p.signer(signer).ss58_address
                    r = p.s.rpc_request("contracts_call", [{
                        "origin": origin, "dest": addr, "value": _val(step),
                        "gasLimit": portaldot.GAS, "inputData": data}])["result"]
                    res = r["result"]
                    assert "Ok" in res, f"{msg} reverted unexpectedly: {res}"
                    raw = bytes.fromhex(res["Ok"]["data"][2:])
                    got = test_contract.decode_ret(m.get("ret"), raw)
                    expected = P.coerce_expected(m.get("ret"), step["expected"])
                    assert got == expected, f"{msg}: expected {expected}, got {got}"
                    typer.echo(f"  [PASS] read {msg}({_fmt(args)}) -> {_show(got)}")

                elif action == "revert":
                    origin = p.signer(signer).ss58_address
                    r = p.s.rpc_request("contracts_call", [{
                        "origin": origin, "dest": addr, "value": _val(step),
                        "gasLimit": portaldot.GAS, "inputData": data}])["result"]
                    res = r["result"]
                    reverted = "Err" in res
                    if not reverted and "Ok" in res:
                        reverted = (int(res["Ok"].get("flags", 0)) & 1) == 1
                    assert reverted, f"{msg}({args}) was expected to revert"
                    typer.echo(f"  [PASS] revert {msg}({_fmt(args)}) as {signer}")

                else:
                    raise ValueError(f"unknown action '{action}'")

            except Exception as e:  # noqa: BLE001
                passed = False
                label = step.get("message") or step.get("name") or action
                typer.secho(f"  [FAIL] step {i} {action} {label}: {e}",
                            fg=typer.colors.RED)
    finally:
        p.close()

    (_ok if passed else _err)(
        f"=== {name}: {'ALL STEPS PASSED' if passed else 'FAILED'} ===")
    return passed


def _field_ty(em: dict, field: str) -> str:
    for f in em["fields"]:
        if f["name"] == field:
            return f["type"]
    raise KeyError(field)


def _fmt(args) -> str:
    return ", ".join(_show(a) for a in args)


def _show(a):
    if isinstance(a, str) and a.startswith("0x") and len(a) == 66:
        return a[:10] + "…"
    return str(a)


# ==========================================================================
# all
# ==========================================================================
@app.command("all")
def run_all(network: Optional[str] = typer.Option(None, "--network")):
    """translate+build+deploy+test every contract in contracts/ with a test spec."""
    cfg = load_config()
    root = cfg.root
    net = _resolve_network(cfg, network)

    contracts_dir = root / "contracts"
    tests_dir = root / "tests"
    names = sorted(
        sol.stem for sol in contracts_dir.glob("*.sol")
        if (tests_dir / f"{sol.stem}.json").exists()
    )
    if not names:
        _err("no contracts with a matching tests/<Name>.json spec")
        raise typer.Exit(1)

    # Ensure the translator binary exists once up front.
    binp = P.translator_bin(root)
    if not binp.exists():
        typer.echo("building translator…")
        r = P.run(["cargo", "build", "--release"], cwd=root / "translator", env=P.cargo_env())
        if r.returncode != 0:
            _err(r.stdout + r.stderr)
            raise typer.Exit(1)

    results = {}
    for name in names:
        typer.secho(f"\n########## {name} ##########", fg=typer.colors.CYAN, bold=True)
        stage = "translate"
        try:
            # translate
            out_dir = P.build_dir(root, name)
            out_dir.mkdir(parents=True, exist_ok=True)
            r = P.run([P.translator_bin(root), P.contract_sol(root, name),
                       "--target", "seal", "--out", out_dir], env=P.cargo_env())
            assert r.returncode == 0, f"translate failed:\n{r.stdout}{r.stderr}"
            typer.echo(f"  translate: ok")

            # build
            stage = "build"
            crate = P.build_dir(root, name)
            r = P.run(["cargo", "+stable", "build", "--release",
                       "--target", "wasm32-unknown-unknown"], cwd=crate, env=P.cargo_env())
            assert r.returncode == 0, f"cargo build failed:\n{r.stdout}{r.stderr}"
            crate_name = P.crate_name_for(root, name)
            raw = crate / "target" / "wasm32-unknown-unknown" / "release" / f"{crate_name}.wasm"
            assert raw.exists(), f"wasm not produced: {raw}"
            _, strip_wasm, _ = _chain(root)
            n = strip_wasm.strip(str(raw), str(P.wasm_path(root, name)))
            typer.echo(f"  build: ok ({n} bytes)")

            # deploy + test are driven by the spec's deploy step inside _run_test_spec.
            stage = "test"
            ok = _run_test_spec(root, net, name)
            results[name] = "PASS" if ok else "FAIL"
        except Exception as e:  # noqa: BLE001
            typer.secho(f"  [{stage} ERROR] {e}", fg=typer.colors.RED)
            results[name] = "FAIL"

    typer.echo("\n========== inkport all: SUMMARY ==========")
    width = max(len(n) for n in results)
    all_ok = True
    for name, status in results.items():
        colour = typer.colors.GREEN if status == "PASS" else typer.colors.RED
        typer.secho(f"  {name.ljust(width)}  {status}", fg=colour)
        all_ok = all_ok and status == "PASS"
    typer.echo("==========================================")
    typer.secho(f"  {'ALL PASS' if all_ok else 'SOME FAILED'}",
                fg=typer.colors.GREEN if all_ok else typer.colors.RED, bold=True)
    raise typer.Exit(0 if all_ok else 1)


if __name__ == "__main__":
    app()
