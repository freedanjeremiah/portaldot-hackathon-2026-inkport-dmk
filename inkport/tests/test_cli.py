from typer.testing import CliRunner
from inkport.cli import app

runner = CliRunner()

def test_help_lists_commands():
    result = runner.invoke(app, ["--help"])
    assert result.exit_code == 0
    for cmd in ["init", "compile", "deploy", "test", "report", "run"]:
        assert cmd in result.stdout
