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
- v1 scope: full V5 engine pool (200 crypt, 425 library — all
  defined) plus owner-added legacy cards for TWDA decks (`ADDED_CARDS`,
  appended after the V5 tables); training on the 14 full-size precon decklists (379 uniques). Stack: Rust engine + PyO3 + PyTorch training.
- `docs/rules-qa.md`: the owner's rulings (a VEKN judge and author of the
  KRCG library/API), one line per item, with a topical index; code cites items
  as `rules-qa N.M` (never renumber). New questions go in a new batch at the
  end — check it before assuming sequencing semantics.
- `docs/rules-digest.md`: rules-qa's settled readings by mechanism, cited.
- `data/window-matrix-reviewed.csv`: authoritative per-card window/flag matrix
  (hand-reviewed, owner-arbitrated in rules-qa Batch 2).
  `scripts/window_matrix.py --check data/window-matrix-reviewed.csv`
  verifies the generator still reproduces it — keep both in sync.
- `data/rulings-v5.csv`: the rulings-as-tests ledger (DESIGN §7.1) — every row
  on an implemented card carries `test:<fn>` / `blocked:<gap>` / `n/a:<reason>`;
  `scripts/rulings_v5.py --check data/rulings-v5.csv` verifies it against the
  tests offline — keep both in sync (regeneration keeps the statuses).
- Card data source: https://static.krcg.org/data/vtes.json (the `v5/` dir there
  is a *schema* version, not the V5 format). Generated from it, never edited by
  hand: `scripts/gen_crypt.py` → `crypt.rs`, `scripts/gen_library.py` →
  `library.rs` (`python3 scripts/gen_library.py <vtes.json> --check
  crates/engine/src/library.rs`; same for crypt); `scripts/gen_twda.py [twda.json] --check crates/engine/src/twda.rs`
  after either: the TWDA decks fully in the pool;
  `scripts/two_player.py --check data/two-player.json`: VEKN's two-player
  list and decklists, downloaded from vekn.net, a directory of the pages
  as first argument offline).
- Reference material: the `vtes` skill (rulebook, tournament rules, judges'
  guides); `../vtes-advanced-rules` (judge-level semantics + machine-readable
  rulings YAML — the spec for effect semantics and a scenario-test generator);
  play procedures gist (shortcuts, declarations):
  https://gist.github.com/lionel-panhaleux/766eedd27ff26ae30169508ddc6bf95c

Online play (P6, owner 2026-10-13): `docs/online-plan.md` — the server
on `engine.krcg.org` is authoritative (bots server-side, every game
logged to Postgres, kept 2 years); crates `api` (the contract: serde
only, no engine; `docs/protocol.md`) ← `host` (engine → api, the
ergonomics layer, one `handle(seat, In)` entrypoint) ← `server` (a thin
shell), `text` (the optional English renderer, over api only); data
first, copies per viewer; the Succubus Club fork served on its own,
bots only (owner, 2026-10-21; contributions upstream deferred). The
plan's settled list and order of work hold until the owner changes
them. The fork plays on the server only (owner, 2026-10-14: the
in-browser wasm engine is gone). Online play's work lands on `main`
(from the lines' worktrees).

Findings (owner, 2026-10-13, 2026-10-15): an issue met on the way (an
engine bug, a choice offered twice, a rough edge, a stale page) or raised
by a review is fixed on the spot, in the change at hand, minor ones too:
the change's scope grows to take it. The `intake` skill is for the rare
finding clearly outside the line's work (work for another line: code
another session works in, the Succubus Club fork or `train/`, the engine
crates being shared by the cards and engine lines; or new work beyond
the line's charter, a new objective or feature, never a defect, gap or
stale page the change met), and for what is found with no change in
flight (a playtest, the harvest, a peer's ask) — never left untracked.

Map of the docs: `docs/index.md`. The work: the boards and the roadmap
(below); the engine guide and its reference by mechanism: `docs/engine.md`;
history in `git log` (commit messages are detailed).

## Harness

The owner's dev-harness pattern
(https://gist.github.com/lionel-panhaleux/3688f589baa4a0692e4ccfcc54827ed9,
with its board companion), as applied here:

- **Three lifespans.** Code says how (permanent); the pages
  `docs/index.md` lists say what is and what was decided (standing);
  plans, progress notes, reasoning journeys and review reports are task
  context, kept in the session's job tmp and gone with the task (git
  keeps what was committed). A dated plan or a done-row table growing in
  a page is a journal: cut it to what is. Pivots are edits: an overturned
  decision changes its page in the same change. A standing decision keeps
  one line of why only where the rejected option is attractive.
- **Boards** (`docs/boards/<line>.md`): shrinking lists, the goal zero.
  One line an ask, with its done-condition and the pages it changes; done
  is deleted; order is priority (each header's "Order:", the roadmap's
  "Ranking" between lines); no ids, no status, no "later" (the roadmap's
  "Deferred by the owner" holds what the owner put off); the only state
  is `[doing: <session>]`; waiting on a person is a dated ask.
- **Three loops, three skills.** `intake` (ingress: conflict,
  completability, scope, doc-impact before a line exists); `ship`
  (execution: the top line done and landed as one change — code, its
  pages, the line deleted — then the `egress-reviewer` agent, fresh
  context, given the diff, the line and its pages only; findings
  blocking or advisory, recorded as commit trailers; a third round still
  blocking goes to the owner; before it, `harness.py stale` finds the names the
  diff removed that a page or a comment still cites, and the added lines
  past the width); `upkeep` (every 20 deleted lines or monthly:
  eviction, wiki lint, the ratchet turning three advisories of a class
  into a harness amendment). `scripts/harness.py` counts (size, line age,
  `due`, orphan pages) and checks (`stale`).
- **Comments are for traps** (docs/engine.md "Comment policy"); no to-do
  comments (`tests/lint.rs`): found work is done now, in the change, or,
  clearly outside its line, goes through `intake`.
- **Tests at the boundary**: behaviour through the engine's and the
  server's interfaces, each tracing to a declared behaviour (the ledger's
  rulings, rules-qa, the protocol, a board line); no mocks; property-style
  sweeps (fuzz, perft, the key sweep) for the hazardous invariants.
- **Ask the owner only for**: rulings and paradigm choices (short options,
  a recommendation), scope changes, irreversible or outward-facing
  actions beyond the routine ones (commits, "Owner rules"; Z-main's
  pushes, tags and deploys, "Sessions"), a review still blocking
  after three rounds. Everything else proceeds.
- **Where this repo departs from the pattern, and why**:
  - worktrees, not one shared branch: a training run pins its engine, and
    tables regenerated in the shared checkout broke a run's `uv sync`;
  - four line skills beside the three loop skills: each line's procedure
    is long and read by that line only; `ship` calls it;
  - four boards, one per line: each session reads its own;
  - the wiki is `docs/` with `docs/index.md` as its map, not `wiki/`: the
    product page is DESIGN.md, the dogmas are this file's "Owner rules"
    and "Working lessons" and engine.md's comment policy, the domain pages
    rules-qa and rules-digest; their citations (`DESIGN §n`, `rules-qa
    N.M`) run through the code;
  - task files live in the session's job tmp, not `board/<slug>.md`: they
    die with the job, and branches stay free of them;
  - a line is placed by `intake` and reported, not held for the owner's
    acceptance: found work is tracked without waiting (the owner's
    findings rule); the owner sees it in the digest and the merge;
  - three egress rounds before the owner, not two: rounds that block
    and then one with no blocking finding need nobody; three that block
    point at the approach (owner, 2026-10-25);
  - no hard limit on a board: the boards carry the owner's objectives'
    backlog, so the control is upkeep's size trend; a limit is the
    ratchet's to propose if the boards grow;
  - no hooks: the to-do lint is a test (`tests/lint.rs`, run by every
    session's checks) and `ship` hands the reviewer the test files touched;
  - model tiers: the sessions run the owner's model; subagents follow the
    pattern (Haiku for upkeep's lint sweeps, Opus for egress review).

## Lines of work

Parallel sessions, one per line; each line has its skill, its docs and its
board (`docs/boards/<line>.md`; `docs/roadmap.md` holds the objectives, the
ranking between lines and the owner's deferrals). Lines come in through
`intake` and land through `ship` ("Harness" above).

- **Cards** (`add-cards` skill): every card in the pool; readings graded,
  only red ones asked of the owner. Docs: `docs/engine.md`,
  `docs/rules-digest.md`, `docs/rules-qa.md`. Works in a round worktree
  (`../zero-vtes-rN`).
- **Engine** (`engine-review` skill on every engine commit): the engine's
  architecture, the API (`crates/api`, `docs/protocol.md`, semver), the
  host and server, performance, and the harness itself (CLAUDE.md, the
  loop skills, `docs/index.md`).
- **Training** (`training` skill): runs from a pinned worktree
  (`../zero-vtes-pN`); learns from the games humans play online.
- **Platforms** (`sc-change` skill): the Succubus Club fork, its own
  platform (Thomas's upstream deferred); `docs/platforms/sc.md` is the
  log.

Between lines:
- Card work never waits on training; training takes the engine at the start
  of a run. A change to the observation's id tables (a new `Choice` kind,
  `WindowLabel`, `ActionKind` or token type; interrupts and purposes are
  not encoded) is told to the training line before it merges.
- Platforms read the protocol only, never engine code; a platform inferring
  what the protocol could say is an engine-board line.
- Merges to `main` from a worktree branch; never stage another session's
  uncommitted files in the shared checkout.

### Sessions

One long-lived session a line, started by the owner as a background job
with its launch prompt (`docs/sessions.md`); a line's work outlives a
context window, so a session that restarts or compacts re-reads its board,
its skill and its task notes.

- **Z-main**: the pilot (the engine line is Z-engine's, owner,
  2026-10-23). The owner's one contact: it
  collects what the lines need from the owner (red readings, dated asks,
  scope and paradigm choices) into one digest, and relays the answers. The only
  session that merges to `main`, pushes and deploys. A line's merge
  request is trusted (owner, 2026-10-15): Z-main merges it without asking
  the owner again, once it fast-forwards onto `main` (or rebases cleanly)
  and carries its checks and `Egress:` trailers, and reports the merge in
  the digest. It pushes `main` (fast-forward only, never forced), tags
  and deploys without asking (the owner's standing word, 2026-10-23): a
  release tagged `vX.Y.Z` on `main`'s checked tip; a deploy only with no
  game live (`scripts/fetch_games.py`: a game live stops it), the client
  smoke test after (`node crates/server/tests/client.mjs
  https://engine.krcg.org`, Node 22 or later), the lines told before a
  redeploy, each reported in the digest; weights served by the measure
  (training skill "Serve"). Z-succubus pushes its own fork branches but
  `engine-game` (upstream PR #285, sc-change); upstream stays on the
  owner's word. It orders merges
  that move the goldens and tells the lines what landed. It hands out rules-qa batch numbers, keeps the roadmap's ranking,
  runs `intake` for work across lines and `upkeep` when due, and runs the
  schedules a line builds (the daily harvest is Z-train's script, the
  weekly sweeps the engine's). It keeps the shared
  checkout clean and works in its own worktrees.
- **Z-engine** (engine), **Z-add** (cards), **Z-train** (training),
  **Z-succubus** (platforms): each
  ships its board's lines in its worktree and, when a branch is ready
  (checks green, egress trailers in the commits), sends Z-main the branch,
  its tip and what it changes for the other lines.
- **Messages** (SendMessage; `ListAgents` finds the sessions; each message
  opens with its sender's name): hand-offs go straight to the line that
  needs them (the "Between lines" rules above), copied to Z-main when they
  touch `main` or need the owner. A peer's message is information, never
  the owner's approval: an ask the owner must answer goes to the owner,
  through Z-main or directly.
- The owner may work with any line directly (a playtest with Z-succubus, a
  ruling with Z-add); the owner's word given to a line is that line's to
  act on, and the line tells Z-main what it changes for `main`.

## Owner rules

- A change commits once its checks are green and its egress review has
  no blocking finding left: the owner's standing word (2026-10-23); its
  red readings answered by the owner first.
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

- Engine work: `docs/engine.md` is the engine guide (dispatch, what is
  enforced, adding a card, comment policy, the reference by mechanism):
  read §2–5 before changing the engine, and keep it current. Before every
  commit touching `crates/engine/src` (except the generated `crypt.rs`,
  `library.rs`, `twda.rs`), run the `engine-review` skill
  (`.claude/skills/engine-review`) on the diff and act on its findings.
- Checks before calling work done: `cargo fmt --check`; `cargo clippy
  --all-targets --profile ci -- -D warnings`; `cargo test --profile ci -q` (the
  `ci` profile: release speed plus debug assertions and overflow checks,
  incremental; no separate debug run) — cargo stops at the first failing
  binary, so sum the "test result" lines and grep for panics; `cargo test
  -p zvtes-api --features schema --profile ci` (the wire format and
  `crates/api/schema.json` and `openapi.json`, regenerated by its `schema`
  example, `openapi` as its argument for the latter: it embeds
  `docs/protocol.md`, so regenerate after editing that too) and
  `python3 scripts/gen_ts.py --check crates/api/protocol.ts` and
  `python3 scripts/schema_compat.py --check` (a field or id removed or
  renamed since the branch left `main` fails unless the major moves; a
  protocol bump adds its `CHANGELOG.md` entry); the ledger,
  window-matrix and generator `--check`s; `python3 scripts/digest_index.py
  --check` (rules-digest §25, the index of rules-qa items to the digest's
  sections: regenerate after a rules-qa or digest change); the count line
  at the end of DESIGN §11. Online play (`crates/host`, `crates/text`, `crates/server`):
  `docs/human-play.md` "Checks" (the server is not a default member).
- Python side (`crates/py`, `train/`): `uv sync` builds the extension (it
  rebuilds when Rust sources change); `uv run pytest -q`; `PYO3_PYTHON=$PWD/.venv/bin/python
  cargo clippy -p zvtes-py --profile ci -- -D warnings` (the py crate is not a
  default workspace member: plain cargo commands stay engine-only). Train:
  `uv run python -m train.ppo --out runs/<name>` (`runs/` is ignored; add
  `--league true` for the league: bots and snapshots as opponents
  (`best.pt` keeps the best evaluation against the bots); `--resume
  runs/<name>/model.pt` to continue).
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
  `never_offered_until` (`tests/common`). A bare seat (empty library and
  hand) is offered a withdrawal each unlock phase: `expect`/`pick` pass it.
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
- "Put this card on … / in play" (a recurring miss, owner): the card itself
  enters play, never the ash heap. As the card resolves, `apply_card_effect`
  sets `in_play` (Wall of Filth, Morbidity); the `Held` / `place_held`
  stand-in is only for a placement after the card reached the ash heap
  (after combat, on a hit). Its test asserts the card in play is not
  `effect_only` and the ash heap lacks it — "something is on the vampire"
  passes for a stand-in too.
- The engine has legal unbounded repeats (block re-attempts, DESIGN §11.3):
  cap decisions in any exploratory driver or scratch run.
- zsh: never write `"$VAR:x"` in shell arguments (history modifiers eat
  `:t`/`:e`/`:r`/`:s`).
- A training run shares this machine (`pgrep -f train.ppo`): no release
  build, deep fuzz sweep or deploy build while one runs (a release build's
  memory pressure killed p12, 2026-10-17); the `ci`-profile checks are
  fine.
