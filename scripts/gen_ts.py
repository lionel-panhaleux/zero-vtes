#!/usr/bin/env python3
"""Generate the protocol's TypeScript types from its JSON Schema.

The contract's source is `crates/api` (Rust); `crates/api/schema.json` is
generated from it; this turns the schema into `crates/api/protocol.ts`
for TypeScript clients (docs/protocol.md). It reads the shapes schemars
emits for the api's types: objects, string-id sets, internally tagged
enums, flattened fields (properties beside a `oneOf`), optional fields.

Usage: python3 scripts/gen_ts.py [crates/api/schema.json] > crates/api/protocol.ts
       python3 scripts/gen_ts.py [crates/api/schema.json] --check crates/api/protocol.ts
"""
import json
import os
import sys

ROOTS = ["In", "Out", "GameSpec", "SpecError", "GameRecord"]


def doc(s, indent=""):
    d = s.get("description")
    if not d:
        return ""
    lines = d.strip().split("\n")
    if len(lines) == 1:
        return f"{indent}/** {lines[0]} */\n"
    body = "".join(f"{indent} * {l}".rstrip() + "\n" for l in lines)
    return f"{indent}/**\n{body}{indent} */\n"


def is_null(s):
    return s.get("type") == "null"


def ty(s, indent):
    """The TypeScript type of schema `s`."""
    if "$ref" in s and len([k for k in s if k not in ("description",)]) == 1:
        return s["$ref"].rsplit("/", 1)[1]
    if "const" in s:
        return json.dumps(s["const"]).replace('"', "'")
    if "enum" in s:
        return " | ".join(json.dumps(v).replace('"', "'") for v in s["enum"])
    for k in ("anyOf", "oneOf"):
        if k in s and "properties" not in s and "$ref" not in s:
            parts = [x for x in s[k] if not is_null(x)]
            t = union([ty(x, indent) for x in parts])
            return f"{t} | null" if len(parts) < len(s[k]) else t
    if "allOf" in s:
        return " & ".join(ty(x, indent) for x in s["allOf"])
    t = s.get("type")
    if isinstance(t, list):
        base = [x for x in t if x != "null"]
        inner = ty({**s, "type": base[0]}, indent)
        return f"{inner} | null" if "null" in t else inner
    if t in ("integer", "number"):
        return "number"
    if t == "string":
        return "string"
    if t == "boolean":
        return "boolean"
    if t == "null":
        return "null"
    if t == "array":
        if "prefixItems" in s:
            return "[" + ", ".join(ty(x, indent) for x in s["prefixItems"]) + "]"
        inner = ty(s["items"], indent)
        return f"({inner})[]" if "|" in inner or "&" in inner else f"{inner}[]"
    if t == "object" or "properties" in s or "$ref" in s:
        return obj(s, indent)
    raise SystemExit(f"gen_ts: unhandled schema {json.dumps(s)[:200]}")


def union(ts):
    return " | ".join(dict.fromkeys(ts))


def obj(s, indent):
    """An object: its properties, then what it is flattened with."""
    parts = []
    props = s.get("properties", {})
    if props:
        req = set(s.get("required", []))
        inner = indent + "    "
        body = ""
        for name, p in props.items():
            opt = "" if name in req else "?"
            body += doc(p, inner) + f"{inner}{name}{opt}: {ty(p, inner)}\n"
        parts.append("{\n" + body + indent + "}")
    elif s.get("additionalProperties") not in (None, False):
        parts.append(f"Record<string, {ty(s['additionalProperties'], indent)}>")
    if "$ref" in s:
        parts.append(s["$ref"].rsplit("/", 1)[1])
    for k in ("oneOf", "anyOf"):
        if k in s and ("properties" in s or "$ref" in s):
            alts = [ty(x, indent + "    ") for x in s[k]]
            parts.append("(\n" + "".join(f"{indent}    | {a}\n" for a in alts) + indent + ")")
    if not parts:
        return "Record<string, never>"
    return " & ".join(parts)


def generate(schema):
    defs = schema["$defs"]
    out = [
        f"/* zvtes protocol {schema['version']} (docs/protocol.md): generated from\n"
        " * crates/api/schema.json by scripts/gen_ts.py, do not edit. */\n",
        f"export const PROTOCOL_VERSION = '{schema['version']}'\n",
    ]
    for name in sorted(defs, key=lambda n: (n not in ROOTS, ROOTS.index(n) if n in ROOTS else 0, n)):
        s = defs[name]
        t = ty(s, "")
        if len(t) > 90 and "\n" not in t and " | " in t:
            t = "\n" + "".join(f"    | {x}\n" for x in t.split(" | ")).rstrip("\n")
        out.append("\n" + doc(s) + f"export type {name} = {t}\n")
    return "".join(out)


def main():
    args = sys.argv[1:]
    check = None
    if "--check" in args:
        i = args.index("--check")
        check = args[i + 1]
        del args[i : i + 2]
    path = args[0] if args else os.path.join(os.path.dirname(__file__), "..", "crates/api/schema.json")
    ts = generate(json.load(open(path)))
    if check:
        if open(check).read() != ts:
            sys.exit(f"{check} is stale: python3 scripts/gen_ts.py > {check}")
        return
    sys.stdout.write(ts)


if __name__ == "__main__":
    main()
