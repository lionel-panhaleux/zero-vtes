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
  picking test vampires). Copies of one card in hand are offered once, at
  the first copy's index.
- Fixture finders (`plain`, `textless`, `with_cap`, `vampire_with`) take
  the first vampire with no definition: each round's definitions move or
  exhaust them. One moved silently can break a fixture (blood above its
  capacity drains). Fix by the trait: a Discipline is a plain vampire's
  `gained_levels`, infernal its `made_infernal`, an age a capacity the
  pool still has; a clan or a capacity no plain vampire has names its
  vampire (`find_card`), its text read for what it bars there
  (Hillanvale's -1 intercept makes her block of a Camarilla bleed
  futile); a count's fodder need not be textless.
- A minion's lock asserted after `until(its seat, MinionPhase)` is gone:
  that seat's unlock phase ran first. Assert it at the acting seat's next
  minion phase, or read what the lock caused (blood, pool).
- A committed strike (a weapon's maneuver, Joumlon's Axe's cancel) left
  with one option elides, its pair-0 request never seen: assert the damage
  it dealt, or read the next pair's optional request (a `Pass` first).
- A play's cost is paid as it resolves, after its as-played window: assert
  it after the next `request`, with a choice still pending in that window
  (a second card), or the elided combat runs on past it.
- `pick_when` passes through every window until its choice is offered: a
  choice that never comes (an allocation's last seat, forced and elided;
  a card the play window does not offer) plays the game out to "game
  over". Allocations walk the seats in order, one `AllocateTo` each.
- New state in `GameState` moves every fuzz golden's hash, the games
  unchanged: a field or a `Release` added with a hash written only when
  set (`state::Sparse`, `Held`'s later releases) or kept untracked (a
  copy list) leaves them green; a `Modifier`, `Release` or other hashed
  enum variant goes at the end, and a field of a struct a `Modifier`
  carries (`AsVampire`) is hashed only when set.
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
- A Discipline new to the crypt data (Thanatosis in round 26, Melpominee
  and Valeren in round 28) needs its seat-context feature appended in
  `crates/env` (`context::LATE_DISCIPLINES`; `tests/context.rs` fails
  otherwise): an observation change, told to the training line.
- A library mode's requirement tag other than a three-letter Discipline
  (`FLIGHT`) was once read as no requirement: `cards::has_disciplines`
  filters the tags it checks, so a new kind of tag needs its arm there.
- A layer's `source` is `CardId::NONE` for a layer of no card in play (an
  action's): match the modifier before indexing `cards[l.source]`.
- While a polling commitment applies, the referendum context is out of
  `Engine::referendum` (`referendum_apply_now` takes it): code reached
  from it (a vote card burned and replaced) cannot tell a referendum is
  under way by that field.
- While a choice applies, `Engine::flow` is out (`Flow::GameOver`):
  `Engine::phase()` reads `Over` in an announce or a cost-terms apply. A
  phase a printed predicate needs there is read from state
  (`GameState::in_minion_phase`, `in_master_phase`).
- An action card is out of play for its whole action and reaches the ash
  heap only as the action ends (`place_action_card`): a card reading "the
  action card … in your ash heap" during the action reads where it is
  bound (`Engine::action_card_to_ash`), not the ash heap.

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
