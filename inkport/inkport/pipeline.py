"""Glue between the inkport CLI and the proven inkport_chain pipeline.

Everything here is metadata-driven. We reuse, verbatim, the on-chain-validated
encode/decode + harness code in inkport_chain/ (portaldot.py, strip_wasm.py,
test_contract.py) rather than reimplementing it.
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any, Optional

from .config import Config, load_config


def _add_chain_to_path(root: Path):
    """Make `inkport_chain` importable (it lives at the project root)."""
    if str(root) not in sys.path:
        sys.path.insert(0, str(root))


# --------------------------------------------------------------------------
# Paths / naming
# --------------------------------------------------------------------------
def crate_name_for(root: Path, name: str) -> str:
    """The snake_case crate name the translator emits, read from the generated
    Cargo.toml when available; else derived with the same rule."""
    cargo = build_dir(root, name) / "Cargo.toml"
    if cargo.exists():
        for line in cargo.read_text().splitlines():
            line = line.strip()
            if line.startswith("name") and "=" in line:
                return line.split("=", 1)[1].strip().strip('"')
    return _snake(name)


def _snake(name: str) -> str:
    chars = list(name)
    out = []
    for i, ch in enumerate(chars):
        if ch.isupper():
            prev_lower = i > 0 and (chars[i - 1].islower() or chars[i - 1].isdigit())
            next_lower = i + 1 < len(chars) and chars[i + 1].islower()
            prev_upper = i > 0 and chars[i - 1].isupper()
            if i != 0 and (prev_lower or (prev_upper and next_lower)):
                out.append("_")
            out.append(ch.lower())
        else:
            out.append(ch)
    return "".join(out)


def build_dir(root: Path, name: str) -> Path:
    return root / "build" / name


def wasm_path(root: Path, name: str) -> Path:
    return build_dir(root, name) / f"{name}.wasm"


def metadata_path(root: Path, name: str) -> Path:
    return build_dir(root, name) / "metadata.json"


def deployments_path(root: Path, network: str) -> Path:
    return root / "deployments" / f"{network}.json"


def translator_bin(root: Path) -> Path:
    rel = root / "translator" / "target" / "release" / "inkport-translate"
    if rel.exists():
        return rel
    return root / "translator" / "target" / "debug" / "inkport-translate"


def contract_sol(root: Path, name: str) -> Path:
    return root / "contracts" / f"{name}.sol"


# --------------------------------------------------------------------------
# Metadata
# --------------------------------------------------------------------------
def load_metadata(root: Path, name: str) -> dict:
    p = metadata_path(root, name)
    if not p.exists():
        raise FileNotFoundError(
            f"metadata not found for {name}: {p} (run `inkport translate {name}` first)"
        )
    return json.loads(p.read_text())


def message_by_name(meta: dict, msg: str) -> dict:
    for m in meta["messages"]:
        if m["name"] == msg:
            return m
    raise KeyError(f"message '{msg}' not in metadata for {meta.get('name')}")


# --------------------------------------------------------------------------
# Arg coercion (metadata-typed). Reuses inkport_chain encoders downstream;
# here we only turn CLI strings / test-spec values into the right Python types.
# --------------------------------------------------------------------------
def pubkey(suri_or_hex: str) -> str:
    """Resolve an address argument to 0x + 32-byte hex.

    Accepts a dev SURI (//Alice), an 0x32-byte hex pubkey, or an ss58 address."""
    s = str(suri_or_hex)
    if s.startswith("0x") and len(s) == 66:
        return s
    from substrateinterface import Keypair

    if s.startswith("//"):
        kp = Keypair.create_from_uri(s, ss58_format=42)
    else:
        # treat as ss58 address
        kp = Keypair(ss58_address=s, ss58_format=42)
    return "0x" + kp.public_key.hex()


def coerce_arg(ty: str, value: Any) -> Any:
    if ty == "address":
        return pubkey(value)
    if ty == "u128":
        return int(value)
    if ty == "bool":
        if isinstance(value, bool):
            return value
        return str(value).lower() in ("1", "true", "yes", "on")
    raise ValueError(f"unknown metadata arg type: {ty}")


def coerce_args(arg_types: list[str], raw: list[Any]) -> list[Any]:
    if len(raw) != len(arg_types):
        raise ValueError(
            f"expected {len(arg_types)} arg(s) {arg_types}, got {len(raw)}: {raw}"
        )
    return [coerce_arg(t, v) for t, v in zip(arg_types, raw)]


def coerce_expected(ret_ty: Optional[str], value: Any) -> Any:
    if ret_ty is None:
        return None
    return coerce_arg(ret_ty, value)


# --------------------------------------------------------------------------
# Subprocess helpers
# --------------------------------------------------------------------------
def run(cmd: list[str], cwd: Optional[Path] = None, env: Optional[dict] = None) -> subprocess.CompletedProcess:
    """Run a command, streaming nothing; capture output. Caller surfaces errors."""
    return subprocess.run(
        [str(c) for c in cmd], cwd=str(cwd) if cwd else None,
        env=env, capture_output=True, text=True,
    )


def cargo_env() -> dict:
    """Environment with ~/.cargo/bin on PATH so `cargo`/`rustup` resolve."""
    env = dict(os.environ)
    cargo_bin = str(Path.home() / ".cargo" / "bin")
    if cargo_bin not in env.get("PATH", ""):
        env["PATH"] = cargo_bin + os.pathsep + env.get("PATH", "")
    return env
