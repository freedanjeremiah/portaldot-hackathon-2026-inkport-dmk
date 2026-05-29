"""Strip a rustc wasm to what rent-era pallet-contracts accepts:
keep only `call`/`deploy` exports, drop custom sections. Memory stays imported."""
def _lebu(b, p):
    r = s = 0
    while True:
        x = b[p]; p += 1; r |= (x & 0x7f) << s
        if not x & 0x80: break
        s += 7
    return r, p

def _leb(n):
    o = bytearray()
    while True:
        b = n & 0x7f; n >>= 7
        o.append(b | 0x80 if n else b)
        if not n: break
    return bytes(o)

def strip(src_path, out_path):
    d = open(src_path, "rb").read()
    assert d[:4] == b"\x00asm", "not wasm"
    i = 8; sections = []
    while i < len(d):
        sid = d[i]; i += 1; sz, i = _lebu(d, i)
        sections.append((sid, d[i:i+sz])); i += sz
    out = []
    for sid, body in sections:
        if sid == 0:           # drop custom sections (names/etc.)
            continue
        if sid == 7:           # rewrite export section: keep call/deploy
            p = 0; n, p = _lebu(body, p); keep = []
            for _ in range(n):
                nl, p = _lebu(body, p); nm = body[p:p+nl].decode(); p += nl
                kind = body[p]; p += 1; idx, p = _lebu(body, p)
                if nm in ("call", "deploy"): keep.append((nm, kind, idx))
            nb = bytearray(_leb(len(keep)))
            for nm, kind, idx in keep:
                nb += _leb(len(nm)) + nm.encode() + bytes([kind]) + _leb(idx)
            body = bytes(nb)
        out.append((sid, body))
    res = bytearray(d[:8])
    for sid, body in out:
        res.append(sid); res += _leb(len(body)); res += body
    open(out_path, "wb").write(res)
    return len(res)

if __name__ == "__main__":
    import sys
    print("stripped", strip(sys.argv[1], sys.argv[2]), "bytes")
