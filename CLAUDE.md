# zero-vtes

A rules-enforcing VTES (Vampire: The Eternal Struggle) engine in Rust plus a
self-play deep-RL agent (AlphaStar-style scaled down), targeting the V5 card pool.

**Read `DESIGN.md` first** — it records all settled architecture and scoping
decisions, with data-backed rationale. Do not re-litigate its "Resolved questions"
without new information from the owner.

Key facts:

- Engine = decision-request state machine (agents answer indexed options); two
  window primitives (ordered impulse/sequencing, unordered polling commitments);
  window elision (auto-pass provably empty windows); replay = (seed, decision log).
- Card legality and execution derive from one representation — masks are
  generated, never hand-written.
- v1 scope: full 625-card V5 engine pool (200 crypt, 425 library — all
  defined); training on the 14 full-size precon decklists (379 uniques). Stack: Rust engine + PyO3 + PyTorch training.
- `docs/rules-qa.md`: the owner's rulings (a VEKN judge and author of the
  KRCG library/API), one line per item, with a topical index; code cites items
  as `rules-qa N.M` (never renumber). New questions go in a new batch at the
  end — check it before assuming sequencing semantics.
- `data/window-matrix-reviewed.csv`: authoritative per-card window/flag matrix
  (hand-reviewed, owner-arbitrated in rules-qa Batch 2).
  `scripts/window_matrix.py --check` verifies the generator still reproduces
  it — keep both in sync.
- `data/rulings-v5.csv`: the rulings-as-tests ledger (DESIGN §7.1) — every row
  on an implemented card carries `test:<fn>` / `blocked:<gap>` / `n/a:<reason>`;
  `scripts/rulings_v5.py --check data/rulings-v5.csv` verifies it against the
  tests offline — keep both in sync (regeneration keeps the statuses).
- Card data source: https://static.krcg.org/data/vtes.json (the `v5/` dir there
  is a *schema* version, not the V5 format). Generated from it, never edited by
  hand: `scripts/gen_crypt.py` → `crypt.rs`, `scripts/gen_library.py` →
  `library.rs` (`python3 scripts/gen_library.py <vtes.json> --check
  crates/engine/src/library.rs`; same for crypt).
- Reference material: the `vtes` skill (rulebook, tournament rules, judges'
  guides); `../vtes-advanced-rules` (judge-level semantics + machine-readable
  rulings YAML — the spec for effect semantics and a scenario-test generator);
  play procedures gist (shortcuts, declarations):
  https://gist.github.com/lionel-panhaleux/766eedd27ff26ae30169508ddc6bf95c

State and next steps: DESIGN §11 (slice table, engine reference by mechanism,
open gaps, what remains); history in `git log` (commit messages are detailed).

## Owner rules

- Commit only when the owner says so (often mid-turn).
- Apply every ruling of the cards you add: read the card's KRCG rulings, its
  ledger rows and `../vtes-advanced-rules` before designing, and make the design
  satisfy each.
- Never invent stand-ins for card-printed data; card-printed numbers and names
  live in `cards/defs.rs`, not engine code. Every vampire ends with ALL its text
  (partial definitions quote the rest in `incomplete`, rules-qa 13.1).
- Conditions are evaluated live, never snapshotted — even when no pool card
  could change them yet.
- Keep legal no-op plays (cycling a card is a real option); only illegal-target
  actions are masked (rules-qa 17.4).
- Before designing a card, apply rules-qa's "Default readings" (choices at
  declaration; a card played by another card is still played, reductions
  apply; costs payable even if the payer burns; caps read narrowly). A reading
  no ruling settles is implemented and recorded as a question in a new
  rules-qa batch. Card text addresses its controller by default (32.7); Ⓓ
  is only indicative (20.1).
- The owner may add legacy cards to close test loops (e.g. a sterile baron);
  hard cards to stress the engine later: Rötschreck, Outside the Hourglass,
  Rewind Time.

## Working lessons

- Checks before calling work done: `cargo fmt --check`; `cargo clippy
  --all-targets --profile ci -- -D warnings`; `cargo test --profile ci -q` (the
  `ci` profile: release speed plus debug assertions and overflow checks,
  incremental; no separate debug run) — cargo stops at the first failing
  binary, so sum the "test result" lines and grep for panics; the ledger,
  window-matrix and generator `--check`s; the count line at the end of DESIGN
  §11.
- Python side (`crates/py`, `train/`): `uv sync` builds the extension (it
  rebuilds when Rust sources change); `uv run pytest -q`; `PYO3_PYTHON=$PWD/.venv/bin/python
  cargo clippy -p zvtes-py --profile ci -- -D warnings` (the py crate is not a
  default workspace member: plain cargo commands stay engine-only). Train:
  `uv run python -m train.ppo --out runs/<name>` (`runs/` is ignored).
- Refactors: the fuzz golden replays and perft counts (`tests/golden/`,
  DESIGN §7.3/§7.5) must stay green; a failure names the seed and decision
  range (`FUZZ_TRACE=1 SEED_FROM=n` traces it). Re-bless an intended change
  with `GOLDEN_BLESS=1 cargo test --profile ci --test fuzz --test perft`. Games
  never reach every definition: also diff a `{:#?}` dump of `DEFS`/`VAMPIRES`.
- `data/rulings-v5.csv` is CRLF: edit it byte-wise. Tests in `tests/rulings.rs`
  must back a ledger row; put regressions elsewhere.
- Window elision breaks naive tests: single-option windows auto-apply, a
  lone-Pass minion phase elides into the next turn (add a second minion), state
  from elided follow-ups lands on the next `run()`. Prove a non-offer with
  `never_offered_until` (`tests/common`).
- Adding a card to a fuzz fixture reshuffles its random games: rare coverage
  keys may starve and latent bugs surface. A fixture's `seeds` is about twice
  the seed where its last key first appears (at most 300); `FUZZ_SEEDS=n` sweeps
  deeper (checked against the goldens' seeds). Every 4th seed is replayed.
- Tests: one binary per file under `crates/engine/tests/` (merging them into
  one binary was measured slower to rebuild); shared helpers in
  `tests/common` (`CardInPlay::new` for a card put in play).
- Test fixtures: blood above capacity drains at the first `run()`; library
  cards carry clan requirements (check `LIBRARY` clans before picking test
  vampires).
- The engine has legal unbounded repeats (block re-attempts, DESIGN §11.3):
  cap decisions in any exploratory driver or scratch run.
- zsh: never write `"$VAR:x"` in shell arguments (history modifiers eat
  `:t`/`:e`/`:r`/`:s`).
