"""Loader for the project-level `inkport.config.py`.

The config is a plain Python module (executed, not imported as a package) so the
project root is wherever `inkport.config.py` lives. We search upward from the
current working directory and fall back to built-in defaults if none is found.
"""
from __future__ import annotations

import runpy
from pathlib import Path
from typing import Optional

CONFIG_FILENAME = "inkport.config.py"

# Built-in fallback, mirrors the shipped inkport.config.py.
_DEFAULTS = {
    "DEFAULT_NETWORK": "portaldot",
    "DEFAULT_SIGNER": "//Alice",
    "NETWORKS": {
        "portaldot": {
            "url": "wss://portaldot.philotheephilix.in",
            "decimals": 14,
            "ss58": 42,
        },
    },
}


def find_project_root(start: Optional[Path] = None) -> Path:
    """Walk upward from `start` (cwd) looking for inkport.config.py.

    Returns the directory containing it, else `start` itself."""
    start = (start or Path.cwd()).resolve()
    for d in [start, *start.parents]:
        if (d / CONFIG_FILENAME).exists():
            return d
    return start


class Config:
    def __init__(self, data: dict, root: Path):
        self.root = root
        self.networks = data.get("NETWORKS", _DEFAULTS["NETWORKS"])
        self.default_network = data.get("DEFAULT_NETWORK", _DEFAULTS["DEFAULT_NETWORK"])
        self.default_signer = data.get("DEFAULT_SIGNER", _DEFAULTS["DEFAULT_SIGNER"])

    def network(self, name: Optional[str] = None) -> dict:
        name = name or self.default_network
        if name not in self.networks:
            raise KeyError(
                f"unknown network '{name}'. Known: {', '.join(self.networks)}"
            )
        net = dict(self.networks[name])
        net.setdefault("decimals", 14)
        net.setdefault("ss58", 42)
        net["name"] = name
        return net


def load_config(start: Optional[Path] = None) -> Config:
    root = find_project_root(start)
    cfg_path = root / CONFIG_FILENAME
    if cfg_path.exists():
        ns = runpy.run_path(str(cfg_path))
        data = {k: v for k, v in ns.items() if not k.startswith("_")}
    else:
        data = _DEFAULTS
    return Config(data, root)
