# zero-vtes

A rules-enforcing [VTES](https://www.vekn.net) (Vampire: The Eternal Struggle)
engine in Rust, a server that hosts games against its bots, and a self-play
deep-RL agent for the V5 card pool.

- **Play**: [engine.krcg.org](https://engine.krcg.org) serves the engine and
  its bots to any client that speaks the protocol; the Succubus Club fork is
  the first such client.
- **Build a client**: [`docs/protocol.md`](docs/protocol.md) is the contract
  (JSON over HTTP and WebSocket, semver), with its JSON Schema
  (`crates/api/schema.json`), OpenAPI (`crates/api/openapi.json`) and
  TypeScript types (`crates/api/protocol.ts`).
- **The engine**: every V5 card and a growing set of legacy cards, every
  ruling tracked in a ledger, most as tests
  ([`data/rulings-v5.csv`](data/rulings-v5.csv)); the owner's
  rulings, a VEKN judge's, in [`docs/rules-qa.md`](docs/rules-qa.md).

## Layout

| Path | What |
|---|---|
| `crates/engine` | the rules engine: a decision-request state machine, card definitions as data |
| `crates/api` | the protocol's types (serde only, no engine) |
| `crates/host` | the engine behind the protocol: views per viewer, events, records |
| `crates/text` | the optional English renderer, over the protocol only |
| `crates/server` | the HTTP and WebSocket server (Postgres or files) |
| `crates/policy`, `crates/py`, `train/` | the bots: inference in Rust, training in PyTorch |
| `deploy/` | the server's deployment (pyinfra) |

[`docs/index.md`](docs/index.md) maps the documentation; [`DESIGN.md`](DESIGN.md)
holds the architecture and the decisions that hold.

## Quick start

With Rust stable:

```sh
cargo test --profile ci -q                # the default members: engine, api, host, text, policy
cargo run --profile ci --example replay -p zvtes-host -- data/records/<file>.json
```

A local server, its records as JSON lines in `records/` (Postgres with
`ZVTES_DATABASE_URL`; the other settings in `crates/server/src/main.rs`):

```sh
ZVTES_LISTEN=127.0.0.1:8040 cargo run --release -p zvtes-server
node crates/server/tests/client.mjs http://127.0.0.1:8040   # plays a game (Node 22+)
```

The `network` bot profile needs weights (`ZVTES_POLICY`, a `.zvp` file);
without them the server offers its scripted bots only. Training: `uv sync`, then
`uv run python -m train.ppo --out runs/<name>` (the training skill,
`.claude/skills/training/SKILL.md`).

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md).

## License

Licensed under the [MIT License](LICENSE).

Portions of the materials are the copyrights and trademarks of Paradox Interactive AB,
and are used with permission. All rights reserved.
For more information please visit [white-wolf.com](http://www.white-wolf.com).

![Dark Pack](dark-pack.png)
