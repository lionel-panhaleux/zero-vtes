# Contributing

zero-vtes is built mostly by Claude Code sessions working to the rules in
[`CLAUDE.md`](CLAUDE.md) and the skills under `.claude/skills/`. Those pages
are the contributor's guide for everyone, people included; this page is the
way in.

## Where the work is

- **The boards**, `docs/boards/<line>.md` (cards, engine, training,
  platforms): one line an ask, with its done-condition and the pages it
  changes, in priority order. [`docs/roadmap.md`](docs/roadmap.md) holds the
  objectives and the ranking between the boards.
- **Found something?** A bug, a wrong ruling, a stale page: open an issue
  saying what you saw (a game record helps: `GET /games/{id}/record` with
  your seat's token, once the game is over). It becomes a board line or is
  fixed in the change at hand (CLAUDE.md "Findings").

## Before you change anything

- [`DESIGN.md`](DESIGN.md): the settled architecture; its resolved
  questions hold until the owner reopens them.
- Engine work: [`docs/engine.md`](docs/engine.md) §2–5 (dispatch, what is
  enforced, adding a card, the comment policy).
- Cards and rules: [`docs/rules-qa.md`](docs/rules-qa.md) (the owner's
  rulings; cite them as `rules-qa N.M`, never renumber), its "Default
  readings", and [`docs/rules-digest.md`](docs/rules-digest.md); the
  `add-cards` skill is the procedure. A reading no ruling settles is
  implemented and recorded as a question in a new rules-qa batch.
- Clients and the server: [`docs/protocol.md`](docs/protocol.md); its §14
  says what a minor and a major version are.

## A change

One change does one board line whole: the code and its tests, every page
the line names updated, the line deleted (the `ship` skill). In short:

- Card-printed data lives in `crates/engine/src/cards/defs.rs` and
  `vampires.rs`, never in engine code; conditions are read live; legal
  no-ops stay offered (CLAUDE.md "Owner rules").
- Generated files are generated, never edited: `crypt.rs`, `library.rs`,
  `twda.rs` (`scripts/gen_*.py`); `crates/api/schema.json` and `openapi.json`
  (`cargo run -p zvtes-api --features schema --example schema [openapi] >
  <file>`; `openapi.json` embeds `docs/protocol.md`, so regenerate it after
  editing that page); `protocol.ts` (`scripts/gen_ts.py`).
- Tests go through the engine's and the server's interfaces, each tracing
  to a ruling, a rules-qa item, the protocol or a board line; no mocks.
- Comments are for traps, and there are no to-do comments
  (`crates/engine/tests/lint.rs`).

## The checks

All of CLAUDE.md's "Checks before calling work done" pass before review, the
main ones:

```sh
cargo fmt --check
cargo clippy --all-targets --profile ci -- -D warnings
cargo test --profile ci -q
cargo test -p zvtes-api --features schema --profile ci
python3 scripts/gen_ts.py --check crates/api/protocol.ts
python3 scripts/rulings_v5.py --check data/rulings-v5.csv
python3 scripts/window_matrix.py --check data/window-matrix-reviewed.csv
```

The fuzz goldens (`crates/engine/tests/golden/`) replay random games: a
change that moves them says why, seed by seed, and re-blesses them with
`GOLDEN_BLESS=1 cargo test --profile ci --test fuzz --test perft`. The
server and the Python side have their own checks (`docs/human-play.md`
"Checks", CLAUDE.md "Working lessons").

## Pull requests

Against `main`, one board line (or one issue) each, its checks green. A
fresh reviewer reads the diff against the line and its pages (the
`egress-reviewer` agent); its findings are fixed (one clearly outside the
change is tracked instead), each recorded as a commit trailer (`Egress:`,
`Advisory:`). The owner merges.

Contributions are under the [MIT License](LICENSE).
