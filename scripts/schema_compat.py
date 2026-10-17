#!/usr/bin/env python3
"""What a change of the protocol's schema removes or adds (docs/protocol.md
§14): every property, closed-set value and tagged variant of
crates/api/schema.json as a path (`Decision.due`, `Window=master_phase`,
`Event[revealed].card`).

    schema_compat.py OLD.json NEW.json     the paths removed and added
    schema_compat.py --check [--base REF]  the working schema against REF's
                                           (default: where the branch left
                                           main, HEAD on main): a path
                                           removed (a field renamed is one)
                                           fails unless PROTOCOL_VERSION's
                                           major moved
"""

import json
import re
import subprocess
import sys

SCHEMA = "crates/api/schema.json"
LIB = "crates/api/src/lib.rs"


def tag(node):
    """A tagged variant's tag: the const of its tag property (`type`,
    `kind`, `error`…), so each variant's fields are its own paths."""
    for sub in node.get("properties", {}).values():
        if isinstance(sub, dict) and "const" in sub:
            return sub["const"]
    return None


def walk(node, path, out):
    if not isinstance(node, dict):
        return
    for key, sub in node.get("properties", {}).items():
        out.add(f"{path}.{key}")
        walk(sub, f"{path}.{key}", out)
    for value in node.get("enum", []):
        out.add(f"{path}={value}")
    if "const" in node:
        out.add(f"{path}={node['const']}")
    for key in ("oneOf", "anyOf", "allOf"):
        for sub in node.get(key, []):
            t = tag(sub) if isinstance(sub, dict) else None
            walk(sub, f"{path}[{t}]" if t is not None else path, out)
    if isinstance(node.get("items"), dict):
        walk(node["items"], f"{path}[]", out)
    if isinstance(node.get("additionalProperties"), dict):
        walk(node["additionalProperties"], f"{path}{{}}", out)


def paths(schema):
    out = set()
    for name, node in schema.get("$defs", {}).items():
        out.add(name)
        walk(node, name, out)
    walk(schema, "", out)
    return out


def major(lib_rs):
    version = re.search(r'PROTOCOL_VERSION: &str = "(\d+)\.\d+\.\d+"', lib_rs)
    return int(version.group(1))


def show(ref, path):
    return subprocess.run(
        ["git", "show", f"{ref}:{path}"], capture_output=True, text=True, check=True
    ).stdout


def main(argv):
    if argv[:1] == ["--check"]:
        base = (
            argv[argv.index("--base") + 1]
            if "--base" in argv
            else subprocess.run(
                ["git", "merge-base", "HEAD", "main"],
                capture_output=True,
                text=True,
                check=True,
            ).stdout.strip()
        )
        old = json.loads(show(base, SCHEMA))
        new = json.load(open(SCHEMA))
        moved = major(open(LIB).read()) > major(show(base, LIB))
    elif len(argv) == 2:
        old, new = (json.load(open(p)) for p in argv)
        moved = None
    else:
        print(__doc__, file=sys.stderr)
        return 2
    before, after = paths(old), paths(new)
    removed, added = sorted(before - after), sorted(after - before)
    for p in removed:
        print(f"- {p}")
    for p in added:
        print(f"+ {p}")
    if moved is None:
        return 0
    if removed and not moved:
        print(
            f"{len(removed)} removed without a major version (docs/protocol.md §14)",
            file=sys.stderr,
        )
        return 1
    print(f"# {len(added)} added, {len(removed)} removed", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
