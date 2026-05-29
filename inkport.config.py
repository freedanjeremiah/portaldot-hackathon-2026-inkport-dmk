# inkport project config.
#
# Loaded by `inkport.config.load_config()`. `--network NAME` selects an entry
# from NETWORKS. Each network gives a websocket URL plus the chain's SCALE/SS58
# parameters used to encode addresses and scale POT (token) values.
#
# decimals = 14  -> 1 POT = 10**14 plancks (the Portaldot rent-era unit).
# ss58     = 42  -> generic substrate address format used by the dev accounts.

DEFAULT_NETWORK = "portaldot"
DEFAULT_SIGNER = "//Alice"

NETWORKS = {
    "portaldot": {
        "url": "wss://portaldot.philotheephilix.in",
        "decimals": 14,
        "ss58": 42,
    },
    # A local node started with the same rent-era pallet-contracts runtime.
    "local": {
        "url": "ws://127.0.0.1:9944",
        "decimals": 14,
        "ss58": 42,
    },
}
