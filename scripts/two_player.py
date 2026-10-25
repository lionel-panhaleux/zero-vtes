#!/usr/bin/env python3
"""VEKN's two-player variant as data (rules-digest §18.5): the "List of
allowed cards in 2-player VTES" and the suggested 2-player decklists,
resolved to KRCG ids against the engine's pool (`crypt.rs`, `library.rs`).
The host refuses a 2-seat deck outside the list and serves the decklists
(`GET /catalog?seats=2`).

The list's names are VEKN's own: matched to the pool's ignoring case and
apostrophes, a crypt name's " gN" naming its group; the rest by `ALIASES`.
A crypt name with several versions in the pool allows its V5 printing only
(the list is of V5 cards; rules-qa 91.8): the newest group among those not
the owner's legacy additions (`ADDED_CARDS`). A decklist's group that names no version
falls back to the name. The decklists are every "2-player decklist" the
variant's index links, each checked against its page's card counts; one
outside the variant's own rules (a card off the list or out of the pool, a
library outside 40 to 60) is excluded, its reason kept.

Usage: python3 scripts/two_player.py [pages-dir] > data/two-player.json
       python3 scripts/two_player.py [pages-dir] --check data/two-player.json
Downloads the pages from vekn.net unless a directory holds them (`index.html`
the variant's index, then each page by its number: `657.html` the list).
"""
import html
import json
import os
import re
import sys
import urllib.error
import urllib.request

SITE = "https://www.vekn.net"
INDEX = "/two-player-format"
LIST = "/two-player-format/657-list-of-allowed-cards-in-2-player-vtes"
# VEKN's spellings the pool spells otherwise.
ALIASES = {
    "bone shamblers": "Bone Shambler",
    "fourth tradition: accounting": "Fourth Tradition: The Accounting",
    "gebeyehy abdu": "Gebeyehu Abdu",
    "golconda": "Golconda: Inner Peace",
    "nesserian": "Neserian",
    "spirit's thouch, the": "Spirit's Touch",
}

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
SRC = os.path.join(ROOT, "crates", "engine", "src")
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from window_matrix import ADDED_CARDS  # noqa: E402


def table(path, fields):
    """The generated table's entries: one dict per card, `fields` read."""
    text = open(path, encoding="utf-8").read()
    out = []
    for block in re.split(r"\n    (?:CryptCard|LibraryCard) \{", text)[1:]:
        e = {}
        for f in fields:
            m = re.search(rf"\n\s*{f}: (\"(?:[^\"\\]|\\.)*\"|\d+)", block)
            if m:
                v = m.group(1)
                e[f] = json.loads(v) if v.startswith('"') else int(v)
        out.append(e)
    return out


def key(name):
    return name.lower().replace("’", "'").strip()


def page(src, path):
    if src:
        name = "index" if path == INDEX else path.rsplit("/", 1)[1].split("-")[0]
        return open(os.path.join(src, name + ".html"), encoding="utf-8").read()
    try:
        with urllib.request.urlopen(SITE + path) as r:
            return r.read().decode("utf-8")
    except urllib.error.URLError as e:
        sys.exit(f"{SITE + path}: {e.reason} (a pages directory works offline)")


def lines(raw):
    """The article's text, one non-empty line each, spaces normalised."""
    m = re.search(r'itemprop="articleBody"(.*?)</div>', raw, re.S)
    body = m.group(1) if m else raw
    body = re.sub(r"<(br|/p|/li|/tr|/h\d)[^>]*>", "\n", body)
    body = html.unescape(re.sub(r"<[^>]+>", "", body)).replace("\xa0", " ")
    out = (re.sub(r"\s+", " ", s).strip() for s in body.split("\n"))
    return [s for s in out if s and s != ">"]


def updated(text):
    m = re.search(r"[Ll]ast updated on ([A-Z][a-z]+ \d+, \d{4})", "\n".join(text))
    return m.group(1) if m else None


def main():
    args = sys.argv[1:]
    check = None
    if "--check" in args:
        i = args.index("--check")
        check = args[i + 1]
        del args[i : i + 2]
    src = args[0] if args else None
    crypt = table(os.path.join(SRC, "crypt.rs"), ["id", "name", "group"])
    library = table(os.path.join(SRC, "library.rs"), ["id", "name"])
    by_crypt, by_library = {}, {}
    for c in crypt:
        by_crypt.setdefault(key(c["name"]), []).append(c)
    for c in library:
        by_library.setdefault(key(c["name"]), []).append(c)
    unmatched = []

    def crypt_ids(name, group=None):
        m = re.fullmatch(r"(.*) g(\d+)", name)
        if m:
            name, group = m.group(1), int(m.group(2))
        versions = by_crypt.get(key(ALIASES.get(key(name), name)), [])
        found = [c for c in versions if group is None or c["group"] == group] or versions
        if not found:
            unmatched.append(name)
            return []
        v5 = [c for c in found if c["id"] not in ADDED_CARDS] or found
        newest = max(c["group"] for c in v5)
        return [c["id"] for c in v5 if c["group"] == newest]

    def library_ids(name):
        found = by_library.get(key(ALIASES.get(key(name), name)), [])
        if not found:
            unmatched.append(name)
        return [c["id"] for c in found]

    text = lines(page(src, LIST))
    list_updated = updated(text)
    rows = text[text.index("Type") + 1 :]
    allowed = []
    for name, kind in zip(rows[::2], rows[1::2]):
        ids = crypt_ids(name) if kind == "Crypt" else library_ids(name)
        allowed.extend({"id": i, "name": name} for i in ids)
    if unmatched:
        sys.exit(f"names on the list matching no card: {unmatched}")
    ok = {c["id"] for c in allowed}
    pages = sorted(
        set(re.findall(r'href="(/two-player-format/\d+-2-player-decklist[^"]*)"', page(src, INDEX))),
        key=lambda p: int(p.rsplit("/", 1)[1].split("-")[0]),
    )
    decks, excluded = [], []
    for path in pages:
        unmatched.clear()
        text = lines(page(src, path))
        title = next(s for s in text if s.upper().startswith("2-PLAYER DECKLIST"))
        cards, counted, printed, section, n_library = {}, {}, {}, None, 0
        for s in text:
            m = re.fullmatch(r"(Crypt|Library) ?\((\d+) cards[^)]*\)", s)
            if m:
                section = m.group(1).lower()
                printed[section] = int(m.group(2))
                counted[section] = 0
                continue
            m = re.fullmatch(r"(\d+)x (.+)", s)
            if not m or section is None:
                continue
            n, rest = int(m.group(1)), m.group(2)
            if section == "crypt":
                c = re.fullmatch(r"(.+?) (\d+) .*?(?::\s*(\d+))?", rest)
                ids = crypt_ids(c.group(1), int(c.group(3)) if c.group(3) else None)
            else:
                ids = library_ids(rest)
                n_library += n
            counted[section] += n
            for i in ids[:1]:
                cards[str(i)] = cards.get(str(i), 0) + n
        if set(printed) != {"crypt", "library"} or counted != printed:
            sys.exit(f"{path}: read {counted}, the page says {printed}")
        words = title.split(":", 1)[1].strip().lower().split()
        name = "2-player: " + " ".join(
            w if i and w in ("of", "and", "the") else w.capitalize() for i, w in enumerate(words)
        )
        reasons = [f"not in the pool: {n}" for n in unmatched]
        reasons += [f"not on the list: {i}" for i in cards if int(i) not in ok]
        if not 40 <= n_library <= 60:
            reasons.append(f"library of {n_library}")
        deck = {"name": name, "source": SITE + path, "updated": updated(text)}
        if reasons:
            excluded.append({**deck, "reasons": reasons})
        else:
            decks.append({**deck, "cards": cards})
    out = {
        "source": SITE + LIST,
        "updated": list_updated,
        "allowed": sorted(allowed, key=lambda c: c["id"]),
        "decks": decks,
        "excluded": excluded,
    }
    text = json.dumps(out, indent=1, ensure_ascii=False) + "\n"
    if check:
        if open(check, encoding="utf-8").read() != text:
            sys.exit(f"{check} is stale: regenerate it")
        print(f"# {len(out['allowed'])} cards allowed, {len(decks)} decks, as {check}")
    else:
        sys.stdout.write(text)


if __name__ == "__main__":
    main()
