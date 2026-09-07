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
- v1 scope: full 621-card V5 engine pool; training on the 14 full-size precon
  decklists (379 uniques). Stack: Rust engine + PyO3 + PyTorch training.
- `docs/rules-qa.md`: running rules Q&A with the owner (a VEKN judge and author
  of the KRCG library/API). Batches 1–2 are answered; later batches emerge
  from implementation — check before assuming sequencing semantics.
- `data/window-matrix-reviewed.csv`: authoritative per-card window/flag matrix
  (hand-reviewed, owner-arbitrated; queue in `docs/window-review-questions.md`).
  `scripts/window_matrix.py` regenerates the draft and, with `--check`, verifies
  it still reproduces the reviewed matrix — keep both in sync.
- `data/rulings-v5.csv`: the rulings-as-tests ledger (DESIGN §7.1) — every row
  on an implemented card carries `test:<fn>` / `blocked:<gap>` / `n/a:<reason>`;
  `scripts/rulings_v5.py --check data/rulings-v5.csv` verifies it against the
  tests offline — keep both in sync (regeneration keeps the statuses).
- Card data source: https://static.krcg.org/data/vtes.json (the `v5/` dir there
  is a *schema* version, not the V5 format). Engine crypt data is generated
  from it: `scripts/gen_crypt.py` → `crates/engine/src/crypt.rs` (never edit by
  hand; `--check` verifies).
- Reference material: the `vtes` skill (rulebook, tournament rules, judges'
  guides); `../vtes-advanced-rules` (judge-level semantics + machine-readable
  rulings YAML — the spec for effect semantics and a scenario-test generator);
  play procedures gist (shortcuts, declarations):
  https://gist.github.com/lionel-panhaleux/766eedd27ff26ae30169508ddc6bf95c
