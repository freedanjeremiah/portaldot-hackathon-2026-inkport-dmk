from typer.testing import CliRunner
from inkport.cli import app

runner = CliRunner()


def test_help_lists_commands():
    result = runner.invoke(app, ["--help"])
    assert result.exit_code == 0
    for cmd in ["translate", "build", "deploy", "call", "test", "all"]:
        assert cmd in result.stdout


def test_config_loads_portaldot():
    from inkport.config import load_config

    cfg = load_config()
    net = cfg.network()  # default
    assert net["url"].startswith("wss://") or net["url"].startswith("ws://")
    assert net["decimals"] == 14
    assert net["ss58"] == 42
    assert cfg.default_signer == "//Alice"


def test_pubkey_resolves_suri():
    from inkport.pipeline import pubkey

    pk = pubkey("//Alice")
    assert pk.startswith("0x") and len(pk) == 66
    # idempotent on hex
    assert pubkey(pk) == pk
