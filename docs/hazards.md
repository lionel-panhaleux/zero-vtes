# Hazards: the traps already met

The non-local traps: behaviour not evident where it lives, mistakes made
once already. Read the sections naming what a change touches before
starting it; the egress reviewer checks the diff against them. A hazard
joins this page only from a real incident (upkeep's retrospective, a
review finding); one the code or a page now makes impossible leaves it.

## Engine tests

- Window elision breaks naive tests: single-option windows auto-apply, a
  lone-Pass minion phase elides into the next turn (add a second minion),
  state from elided follow-ups lands on the next `run()`. Prove a
  non-offer with `never_offered_until` (`tests/common`). A bare seat
  (empty library and hand) is offered a withdrawal each unlock phase:
  `expect`/`pick` pass it.
- Test fixtures: blood above capacity drains at the first `run()`;
  library cards carry clan requirements (check `LIBRARY` clans before
  picking test vampires).
- Adding a card to a fuzz fixture reshuffles its random games: rare
  coverage keys may starve and latent bugs surface. A fixture's `seeds`
  is about twice the seed where its last key first appears (at most
  300); `FUZZ_SEEDS=n` sweeps deeper (checked against the goldens'
  seeds). Every 4th seed is replayed.
- The host events sweep (`crates/host/tests/events.rs`) deals its decks by
  seed modulo the catalog's size: a catalog size change (a TWDA refresh, a
  round changing the kept decks) reshuffles every `KEPT` seed's game while
  the test may stay green. Re-probe each kept seed's cause at
  `HOST_SEEDS=400` and replace the ones that lost it.
- Refactors: the fuzz golden replays and perft counts (`tests/golden/`,
  DESIGN §7.3/§7.5) must stay green; a failure names the seed and
  decision range (`FUZZ_TRACE=1 SEED_FROM=n` traces it). Re-bless an
  intended change with `GOLDEN_BLESS=1 cargo test --profile ci --test
  fuzz --test perft`. Games never reach every definition: also diff a
  `{:#?}` dump of `DEFS`/`VAMPIRES`.
- The engine has legal unbounded repeats (block re-attempts, DESIGN
  §11.3): cap decisions in any exploratory driver or scratch run.

## Cards

- "Put this card on … / in play" (a recurring miss, owner): the card
  itself enters play, never the ash heap. As the card resolves,
  `Engine::place_now` puts it in play (Wall of Filth, Morbidity); the
  `Held` / `place_held` stand-in is only for a placement after the card
  reached the ash heap (after combat, on a hit). Its test asserts the
  card in play is not `effect_only` and the ash heap lacks it —
  "something is on the vampire" passes for a stand-in too.
- `data/rulings-v5.csv` is CRLF: edit it byte-wise. Tests in
  `tests/rulings.rs` must back a ledger row; put regressions elsewhere.

## The shared tree

Every line works in the one checkout on `main` (CLAUDE.md "Sessions").

- A build, a check or a test run compiles every session's uncommitted
  edits, not only yours: a failure in a file you did not touch is
  another session's work in progress; tell it (SendMessage), never fix or
  revert its file.
- A golden re-bless (`GOLDEN_BLESS=1`) or a regenerated table records
  the whole tree: only with no other session's uncommitted edit under
  `crates/engine/src` (`git status -- crates/engine/src`, each file
  yours), else asked of the sessions first.
- `git commit -a` or a bare `git commit` after `git add .` takes the
  others' files: commit by path (`git commit -- <paths>`). The index is
  shared too: never partial staging (`git add -p`, `git apply --cached`)
  then a bare `git commit`, which takes whatever another session staged.
- `git commit -- <file>` takes the whole working file, another session's
  hunks in it too (the cards and engine lines share `crates/engine/src`,
  docs/engine.md, rules-qa, the ledger). Before writing a shared file,
  `git diff -- <file>` shows others' hunks; a file you commit holding
  another session's hunks waits until that session agrees they go in, or
  commits first. Imperfect isolation is accepted where files overlap;
  silent inclusion is not.
- Green checks in the shared tree prove the tree, not your commit: a
  change reading another session's uncommitted file (a helper, a table,
  a schema) lands a broken `main`. Your change uses only committed code
  and your own files.
- The harvest and the sweeps need `main`'s tip with no edit: they run on
  a detached checkout (docs/watch.md), never in the shared tree.

## The shared machine

- A training run shares this machine (`pgrep -f train.ppo`): no release
  build, deep fuzz sweep or deploy build while one runs (a release
  build's memory pressure killed p12, 2026-10-17); the `ci`-profile
  checks are fine.
- The sessions share one machine and one checkout: never `git stash` (it
  takes every session's uncommitted edits; copy files under the job tmp
  instead), and a process killed by its PID, never by a pattern (`pkill -f
  'cargo test'` or `pkill -f <checkout>/target` stops every session's
  runs).
- zsh: never write `"$VAR:x"` in shell arguments (history modifiers eat
  `:t`/`:e`/`:r`/`:s`).
