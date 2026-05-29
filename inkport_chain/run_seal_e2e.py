"""End-to-end driver for the seal0 backend.

For each fixture contract:
  1. translate .sol -> seal0 Rust (inkport-translate --target seal)
  2. cargo +stable build --release --target wasm32-unknown-unknown
  3. strip the wasm (inkport_chain/strip_wasm.py)
  4. deploy to wss://portaldot.philotheephilix.in
  5. run real on-chain assertions via inkport_chain/test_contract.py

Run:  python inkport_chain/run_seal_e2e.py
"""
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))
from inkport_chain.strip_wasm import strip  # noqa: E402
from inkport_chain.test_contract import run_contract_test  # noqa: E402

TRANSLATOR = ROOT / "translator"
BIN = TRANSLATOR / "target" / "release" / "inkport-translate"
GEN = ROOT / "build" / "seal"


def sh(cmd, cwd=None):
    print(f"  $ {' '.join(str(c) for c in cmd)}")
    subprocess.run(cmd, cwd=cwd, check=True)


def build_translator():
    sh(["cargo", "build", "--release", "--quiet"], cwd=TRANSLATOR)


def prepare(sol_name, crate_name):
    """translate -> build -> strip; return (stripped_wasm, metadata_json)."""
    sol = ROOT / "contracts" / sol_name
    out = GEN / crate_name
    out.mkdir(parents=True, exist_ok=True)
    sh([str(BIN), str(sol), "--target", "seal", "--out", str(out)])
    sh(["cargo", "+stable", "build", "--release", "--target",
        "wasm32-unknown-unknown", "--quiet"], cwd=out)
    raw = out / "target" / "wasm32-unknown-unknown" / "release" / f"{crate_name}.wasm"
    stripped = out / f"{crate_name}.stripped.wasm"
    n = strip(str(raw), str(stripped))
    print(f"  stripped {crate_name}: {n} bytes")
    return str(stripped), str(out / "metadata.json")


def main():
    build_translator()
    results = {}

    # ----- Flipper -----
    wasm, meta = prepare("Flipper.sol", "flipper")
    results["Flipper"] = run_contract_test(
        "Flipper", wasm, meta, ctor_args=[False],
        steps=[
            ("read", "get", [], False),
            ("call", "flip", [], None),
            ("read", "get", [], True),
            ("call", "flip", [], None),
            ("read", "get", [], False),
        ],
    )

    # ----- Counter -----
    wasm, meta = prepare("Counter.sol", "counter")
    results["Counter"] = run_contract_test(
        "Counter", wasm, meta, ctor_args=[0],
        steps=[
            ("read", "get", [], 0),
            ("call", "inc", [], None),
            ("read", "get", [], 1),
            ("call", "incBy", [5], None),
            ("read", "get", [], 6),
        ],
    )

    # ----- SimpleStorage -----
    wasm, meta = prepare("SimpleStorage.sol", "simple_storage")
    results["SimpleStorage"] = run_contract_test(
        "SimpleStorage", wasm, meta, ctor_args=[0],
        steps=[
            ("read", "get", [], 0),
            ("call", "set", [42], None),
            ("read", "get", [], 42),
            ("revert", "setIfPositive", [0], None),
            ("call", "setIfPositive", [7], None),
            ("read", "get", [], 7),
        ],
    )

    print("\n========== SUMMARY ==========")
    ok = True
    for name, passed in results.items():
        print(f"  {name}: {'PASS' if passed else 'FAIL'}")
        ok = ok and passed
    print("=============================")
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
