#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Dhiraj Patra
# SPDX-License-Identifier: MIT
"""Independent reference implementation of pwdx spec v1 (documentation + vector generator).

Not shipped. It deliberately shares no code with the Rust core. Site normalization is NOT
implemented here: vectors carry `site_normalized` explicitly (Rust tests cover normalization).
Requires: pip install argon2-cffi
"""
import hashlib, hmac, json, os, struct, sys, unicodedata
from argon2.low_level import hash_secret_raw, Type

HERE = os.path.dirname(os.path.abspath(__file__))
WORDS = open(os.path.join(HERE, "..", "crates", "core", "data", "eff_large_wordlist.txt"),
             encoding="ascii").read().split("\n")
WORDS = [w for w in WORDS if w]
assert len(WORDS) == 7776

UPPER, LOWER, DIGITS, SYMBOLS = ("ABCDEFGHIJKLMNOPQRSTUVWXYZ", "abcdefghijklmnopqrstuvwxyz",
                                 "0123456789", "!@#$%^&*-_=+")
CLASSES = {"alnum": [UPPER, LOWER, DIGITS], "full": [UPPER, LOWER, DIGITS, SYMBOLS]}
RANGES = {"alnum": (8, 128), "full": (8, 128), "passphrase": (4, 16), "pin": (4, 12)}


def lp(b: bytes) -> bytes:
    return struct.pack(">I", len(b)) + b


def hkdf_expand(prk: bytes, info: bytes, length: int) -> bytes:
    okm, t, i = b"", b"", 1
    while len(okm) < length:
        t = hmac.new(prk, t + info + bytes([i]), hashlib.sha512).digest()
        okm += t
        i += 1
    return okm[:length]


class Stream:
    def __init__(self, data: bytes):
        self.d, self.p = data, 0

    def u8(self) -> int:
        b = self.d[self.p]  # IndexError == StreamExhausted
        self.p += 1
        return b

    def pick(self, n: int) -> int:
        assert 1 <= n <= 65536
        if n <= 256:
            limit = 256 - (256 % n)
            while True:
                b = self.u8()
                if b < limit:
                    return b % n
        limit = 65536 - (65536 % n)
        while True:
            v = (self.u8() << 8) | self.u8()
            if v < limit:
                return v % n


def derive(key: bytes, secret: str, site_norm: str, user: str, counter: int,
           profile: str, length: int) -> str:
    assert len(key) == 32 and counter >= 1
    lo, hi = RANGES[profile]
    assert lo <= length <= hi
    s = unicodedata.normalize("NFKC", secret).encode()
    u = unicodedata.normalize("NFKC", user).strip().encode()
    salt = hashlib.sha256(b"pwdx/v1/salt" + key).digest()
    stretch = hash_secret_raw(secret=s, salt=salt, time_cost=3, memory_cost=65536,
                              parallelism=1, hash_len=32, type=Type.ID, version=19)
    master = hmac.new(key, b"pwdx/v1/master" + stretch, hashlib.sha512).digest()
    info = (b"pwdx/v1/site" + lp(site_norm.encode()) + lp(u) + struct.pack(">I", counter)
            + lp(f"{profile}:{length}".encode()))
    st = Stream(hkdf_expand(master, info, 1024))
    if profile in CLASSES:
        classes = CLASSES[profile]
        full = "".join(classes)
        out = [c[st.pick(len(c))] for c in classes]
        while len(out) < length:
            out.append(full[st.pick(len(full))])
        for i in range(length - 1, 0, -1):
            j = st.pick(i + 1)
            out[i], out[j] = out[j], out[i]
        return "".join(out)
    if profile == "passphrase":
        return "-".join(WORDS[st.pick(7776)] for _ in range(length))
    if profile == "pin":
        return "".join(DIGITS[st.pick(10)] for _ in range(length))
    raise ValueError(profile)


K1 = bytes(range(32))
K2 = bytes([0xAB]) * 32
SECRET = "correct horse battery staple"


def v(id_, **kw):
    d = dict(key_hex=K1.hex(), secret=SECRET, site="example.com", user="alice@example.com",
             counter=1, profile="full", length=24)
    d.update(kw)
    d["id"] = id_
    d.setdefault("site_normalized", d["site"])
    return d


CASES = [
    v("full-default-24"),
    v("full-min-8", length=8),
    v("full-max-128", length=128),
    v("alnum-default-24", profile="alnum"),
    v("alnum-min-8", profile="alnum", length=8),
    v("alnum-max-128", profile="alnum", length=128),
    v("passphrase-8", profile="passphrase", length=8),
    v("passphrase-min-4", profile="passphrase", length=4),
    v("passphrase-max-16", profile="passphrase", length=16),
    v("pin-6", profile="pin", length=6),
    v("pin-min-4", profile="pin", length=4),
    v("pin-max-12", profile="pin", length=12),
    v("counter-2", counter=2),
    v("counter-max", counter=4294967295),
    v("other-user", user="bob@example.com"),
    v("other-site", site="example.org"),
    v("user-case-differs", user="Alice@example.com"),
    v("second-key", key_hex=K2.hex()),
    # normalization: these must equal their base vector's output
    v("norm-secret-composed", secret="caf\u00e9 au lait"),
    v("norm-secret-decomposed", secret="cafe\u0301 au lait"),
    v("norm-secret-fullwidth", secret="\uff23\uff2f\uff32\uff32\uff25\uff23\uff34 123"),
    v("norm-secret-ascii-of-fullwidth", secret="CORRECT 123"),
    v("norm-secret-ligature", secret="of\ufb01ce pass"),
    v("norm-secret-ascii-of-ligature", secret="office pass"),
    v("norm-user-padded", user="  alice@example.com\t"),
    v("secret-trailing-space", secret=SECRET + " "),
    v("secret-emoji-cjk", secret="\U0001F511 \u6f22\u5b57 \u0645\u0631\u062d\u0628\u0627"),
    # site input variants (raw input -> normalized form from the spec)
    v("site-url-variant", site="HTTPS://WWW.Example.COM/path?q=1", site_normalized="example.com"),
    v("site-subdomain-port", site="login.example.co.uk:8443", site_normalized="example.co.uk"),
    v("site-idn", site="https://www.B\u00fccher.de/", site_normalized="xn--bcher-kva.de"),
]

ERRORS = [
    dict(id="err-control-char-secret", secret="a\u0007b"),
    dict(id="err-empty-secret", secret=""),
    dict(id="err-empty-site", site=""),
    dict(id="err-empty-user", user="   "),
    dict(id="err-counter-zero", counter=0),
    dict(id="err-full-len-7", length=7),
    dict(id="err-full-len-129", length=129),
    dict(id="err-pin-len-3", profile="pin", length=3),
    dict(id="err-pin-len-13", profile="pin", length=13),
    dict(id="err-passphrase-3", profile="passphrase", length=3),
    dict(id="err-passphrase-17", profile="passphrase", length=17),
]

if __name__ == "__main__":
    out = {"spec": "v1", "vectors": [], "errors": []}
    for c in CASES:
        c = dict(c)
        c["expected"] = derive(bytes.fromhex(c["key_hex"]), c["secret"], c["site_normalized"],
                               c["user"], c["counter"], c["profile"], c["length"])
        out["vectors"].append(c)
    for e in ERRORS:
        d = v(e["id"])
        d.update(e)
        d.pop("site_normalized", None)
        out["errors"].append(d)
    path = os.path.join(HERE, "v1.json")
    with open(path, "w", encoding="utf-8") as f:
        json.dump(out, f, ensure_ascii=True, indent=2)
        f.write("\n")
    print(f"wrote {path}: {len(out['vectors'])} vectors, {len(out['errors'])} error cases")
