import typer

app = typer.Typer(help="InkPort — Solidity → ink! framework for Portaldot")

@app.command()
def init(directory: str = "."):
    """Scaffold a new InkPort project."""
    raise NotImplementedError

@app.command()
def compile(contract: str = typer.Option(None, "--contract")):
    """Translate Solidity to ink! and build WASM + metadata."""
    raise NotImplementedError

@app.command()
def deploy(network: str = typer.Option(..., "--network"),
           contract: str = typer.Option(None, "--contract"),
           value: float = typer.Option(0.0, "--value"),
           account: str = typer.Option("deployer", "--account")):
    """Deploy a compiled contract to a network."""
    raise NotImplementedError

@app.command()
def run(script: str, network: str = typer.Option(..., "--network")):
    """Run a Python script with an injected InkPort context."""
    raise NotImplementedError

@app.command()
def test(network: str = typer.Option("local", "--network")):
    """Run behavioral / golden tests."""
    raise NotImplementedError

@app.command()
def report(contract: str = typer.Option(None, "--contract")):
    """Print the translation report."""
    raise NotImplementedError
