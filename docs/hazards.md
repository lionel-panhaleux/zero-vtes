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
- Fixture finders (`plain`, `textless`, `with_cap`, `vampire_with`) go
  through `fixture_vampire` (`tests/common`): the vampires with no
  definition first, then those printing only plain bonuses
  (`fixture_rank`): each round's definitions move or exhaust them. One
  moved silently can break a fixture (blood above its capacity drains).
  Fix by the trait: a Discipline is a plain vampire's `gained_levels`,
  infernal its `made_infernal`, an age a capacity the pool still has; a
  clan or a capacity no plain vampire has names its vampire
  (`find_card`), its text read for what it bars there (Hillanvale's -1
  intercept makes her block of a Camarilla bleed futile); a count's
  fodder need not be textless.
- A minion's lock asserted after `until(its seat, MinionPhase)` is gone:
  that seat's unlock phase ran first. Assert it at the acting seat's next
  minion phase, or read what the lock caused (blood, pool).
- A fixture's hand cards (`hand`) carry `CopyId::NONE`: a copy id is no
  sentinel for "taken from hand" (Charming Lobby's named card: an explicit
  `ActionCtx::named_played`).
- A card put in play by a fixture (`CardInPlay::new`) carries none of its
  mode's layers: a retainer's damage (Ghoul Retainer's bite) or a bearer
  modifier is missing until the mode's `Effect::Layer`s are pushed too (an
  `equipped` helper).
- A non-strike ability used in the strike step (Ghoul Retainer's weapon
  use) leaves the strike to choose; a single strike left (the hand strike)
  elides, its request never seen.
- A committed strike (a weapon's maneuver, Joumlon's Axe's cancel) left
  with one option elides, its pair-0 request never seen: assert the damage
  it dealt, or read the next pair's optional request (a `Pass` first).
- A blood total that another strike's damage also explains proves no cost:
  Target Vitals' canceled cost (PIB 20121031) was never paid while its test
  passed on the opposing hand strike's 1. Pin a cost apart (the strikes
  out of the way, or both counted).
- A referendum no one can vote in (no titled vampire, no Edge, no vote
  card) resolves with no decision: `to_referendum` never sees it and the
  game plays on. Give a seat the Edge to stop at a blood hunt.
- A loop stopping once `Engine::referendum()` is none must start inside the
  referendum (`to_referendum`): before the political action's blocks it is
  none too, and the loop passes the whole polling away (Elder Kindred
  Network's control run did).
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
- A round's regenerated tables move fixtures it never touched: fuzz's
  `per_seat` picks from all of `CRYPT`, not bounded by `FIXTURE_CRYPT`, so
  an appended vampire matching its filter joins a seat and reshuffles the
  whole fixture (14a, 15a in round 32); a card-naming term (Precision,
  Sibyl's Tongue) offers every `LIBRARY` entry, so library growth names
  another card at the same index (30b, 31b). Expected each round: attribute
  each move before blessing.
- The host events sweep (`crates/host/tests/events.rs`) deals its decks by
  seed modulo the catalog's size: a catalog size change (a TWDA refresh, a
  round changing the kept decks) reshuffles every `KEPT` seed's game while
  the test may stay green. Re-probe each kept seed's cause on its own
  (`HOST_SEED_FROM=n HOST_SEEDS=n HOST_REDUCE=1`, the sweep stopping at its
  first failing seed; run alone, a seed fails the cross-seed totals, to
  ignore: only the checks inside its game count) and replace the ones that
  lost it.
- Refactors: the fuzz golden replays and perft counts (`tests/golden/`,
  DESIGN §7.3/§7.5) must stay green; a failure names the seed and
  decision range (`FUZZ_TRACE=1 SEED_FROM=n` traces it, on one thread).
  A fixture's seeds run across threads: stderr may show several seeds'
  panics, the test's failure names the lowest. Re-bless an
  intended change with `GOLDEN_BLESS=1 cargo test --profile ci --test
  fuzz --test perft`. Games never reach every definition: also diff a
  `{:#?}` dump of `DEFS`/`VAMPIRES`.
- The engine has legal unbounded repeats (block re-attempts, DESIGN
  §11.3): cap decisions in any exploratory driver or scratch run.
- A fixture change made after a request was built (a minion unlocked,
  counters set) is not in that request: the window was computed before, and
  "not offered" then proves nothing. Change the state before the window
  opens, or let another minion act so the next window is built anew, and
  assert something the change makes offered too.
- The `noted_*` helpers read `Engine::notes`, off in a bare fixture: they
  panic "noting" unless `e.notes = Some(Vec::new())` was set before the
  play they read.
- `Engine::at_master_phase` leaves a bare fixture's flow in the minion
  phase: "during your minion phase" abilities are still offered there.
  Prove a master-phase non-offer at a real master phase (`until` it).
- A vampire that must hunt is offered only its hunt: that minion phase
  window elides and the hunt is taken on the way (`until` passes it).
- `Minion::gained_levels` counts one level per entry: a superior level is
  two entries (`vec!["aus", "aus"]`); an upper-case name is still one.
- An influence phase whose only option is a due (a Choir's burn: no Pass
  while one waits) elides, resolved on the way: give the seat an
  uncontrolled vampire to transfer to, so the window is a decision.
- A minion phase offering only a forced action (Spirit Marionette's
  taken minion with a single bleed) elides: the action is taken on the
  way. Give it a second way to bleed (a bleed card) to stop there.
- A block attempt short of the stealth is pending until its seat's
  change-of-blocker window closes, and passing that window declines the
  seat: `failed_attempts` is empty after the `DeclareBlock` pick, and the
  same minion's second attempt (rules-qa 48.6) is not reachable by passing.

## Cards

- "Put this card on … / in play" (a recurring miss, owner): the card
  itself enters play, never the ash heap. As the card resolves,
  `Engine::place_now` puts it in play (Wall of Filth, Morbidity); the
  `Held` / `place_held` stand-in is only for a placement after the card
  reached the ash heap (after combat, on a hit). Its test asserts the
  card in play is not `effect_only` and the ash heap lacks it —
  "something is on the vampire" passes for a stand-in too.
- `scripts/window_matrix.py --check` compares each card's windows as a
  union: a generator change can move another card's per-mode windows
  (library.rs) with no divergence. Diff `scripts/gen_library.py`'s output
  too (Slake the Thirst, Shape Mastery: an "as above" across card types).
- A `Rider`'s `requires` reads no `AnyOf` (not a rider slot): one rider
  per alternative (Scorpion's Touch's `CardMode(2)`, `CardMode(3)`).
- A granted `ActionKind::BurnCard` resolves only the effects its arm in
  `succeed` names (the families walk lists them, `K::BurnCard`): "burn this
  card and unlock" (Burden the Mind) needed both; any other effect there is
  silently dropped.
- `data/rulings-v5.csv` is CRLF: edit it byte-wise. Tests in
  `tests/rulings.rs` must back a ledger row; put regressions elsewhere.
- A definition is a plain `pub static NAME: CardDef = CardDef { name: …`:
  `scripts/rulings_v5.py` reads the printed name there, and a static a
  macro generates is unreadable to it (the round-32 Kholos).
- A Discipline new to the crypt data (Thanatosis in round 26, Melpominee
  and Valeren in round 28) needs its seat-context feature appended in
  `crates/env` (`context::LATE_DISCIPLINES`; `tests/context.rs` fails
  otherwise): an observation change, told to the training line.
- A library mode's requirement tag other than a three-letter Discipline
  (`FLIGHT`) was once read as no requirement: `cards::has_disciplines`
  filters the tags it checks, so a new kind of tag needs its arm there.
- A new interrupt or `Terms` offering `FindCard` needs its zone in
  `crates/host/src/describe.rs` `found_in` (whose seat's library, hand or
  ash heap): its catch-alls give none, or for play terms the player's own
  heap, silently; the keys sweep (`a_found_card_names_the_zone_it_is_in`)
  meets it only on the seeds that reach it, and a wrong heap only where the
  card is absent from it (Undele and round 29's look-to-hand found by the
  weekly sweep; Victim of Habit's prey heap and Marthe Dizier by review).
- A new place a card can sit (Hukros's `Minion::stored`) needs its
  `copies::Place` in `census` (fuzz's copy conservation, its two owner
  maps), the host's `zone_ref` and narrator `spot` (who sees it), the
  events reducer (`tests/reduce`), `legal_view`'s hiding, and a
  `virtual_source` arm if cards are played from it: an index into
  `cards` from a `PlayCtx::from` that is no card in play panics (a
  Charming Lobby return did, guarded now).
- A layer's `source` is `CardId::NONE` for a layer of no card in play (an
  action's): match the modifier before indexing `cards[l.source]`.
- While a polling commitment applies, the referendum context is out of
  `Engine::referendum` (`referendum_apply_now` takes it): code reached
  from it (a vote card burned and replaced) cannot tell a referendum is
  under way by that field.
- While a choice applies, `Engine::flow` is out (`Flow::GameOver`):
  `Engine::phase()` reads `Over` in an announce or a cost-terms apply. A
  phase a printed predicate needs there is read from state
  (`GameState::in_minion_phase`, `in_master_phase`). A site that suspends
  its own continuation for a wait (a diablerie asking `ask_burn`) sets a
  flow before it returns, or the game ends there.
- An action card's effects resolve in `succeed` with the action taken out
  of `Engine::action`: code reached from them reads the acting minion from
  `GameState::acting` (Abandoning the Flesh against Sacrificial Lamb).
- A title card on a vampire is its title (`claimed_title` reads
  `title_card_on`): "cannot gain a title" (Persona Non Grata,
  `GameState::may_gain_title`) is a mask where the card is played or its
  vampire chosen, and a guard at each site moving a title card (Regent,
  `oldest_for`); `gain_title` only no-ops a title gained without a card.
- An action card canceled as played declares no action (rules-qa 73.2):
  what ends "with its next action" (Spirit Marionette's control) also ends
  in `settle_canceled`, not only in `end_action`.
- An action card is out of play for its whole action and reaches the ash
  heap only as the action ends (`place_action_card`): a card reading "the
  action card … in your ash heap" during the action reads where it is
  bound (`Engine::action_card_to_ash`), not the ash heap.
- A strike card's mode holds `Effect::Strike(k)` or, committing at the
  maneuver, `Effect::CommittedStrike(k)` (Thrown Gate, Slam): code asking
  whether a mode strikes matches both (`is_strike_mode`). Matching only
  `Strike` once charged Lucian's toll twice on a Thrown Gate played as
  the strike.
- A granted action's `terms` are not asked like an ability's: `use_ability`
  declares the action and returns before `Flow::AbilityTerms`, so a term
  with no arm in its `DeclareAction` branch is silently skipped (Talbot's
  master discard was; it is now paid as the action resolves, `succeed`).
- An ability whose terms can have no option (cards to discard, an
  equipment or retainer from hand, an ash heap card) needs its check among
  `ability_gate`'s terms checks, or it opens an empty request (Velya's
  discard, served v2.31.1).
- An action window opened where none opened before (a hunt's
  would-be-successful window, for Foul Blood) is offered to every card and
  ability already timed there: fuzz trajectories move and recorded games
  (`data/records`) stop, though the new card never meets them. Prefer the
  existing window of the same moment (Foul Blood: blocks declined).
- A contested vampire is not `present()`: an entry interrupt that reads
  its vampire (Seterpenre's masters put on him) is dropped by its settle arm
  if the contest is staged with it; such an entry stages its contests once
  it ends (`Interrupt::DisciplineSearch::entering`, Victor Pelletier's
  `BuyTitle`), "before contest" (LSJ 20080526).
- An immunity is not a reduction: `Stat::DamageImmuneFromOpponent`
  (`GameState::damage_immune`), not `NoDamageFromOpponent`, or Haymaker,
  the Walking Stick and the Flamethrower miss it (rules-qa 116.138).
- The would-be-successful window opens for a bleed of 1 or more and a
  political action only (`ActionState::WouldSucceed`); for any other action
  the acting Methuselah's moment after blocks are declined is
  `Window::ActionDuring` with `Requirement::BlocksDeclined` (Forger's
  Hammer). A card timed there never offered is this.
- `Effect::FirstStrike` before its `Effect::Strike` in a mode is lost: a
  strike chosen resets the side's strike state (`Side::set_strike`). Meat
  Hook's [POT] tear lost its first strike so; list it after the strike.
- An ally's own card locks through its body (`Engine::lock_card`,
  `Requirement::CardUnlocked`): a reader of `CardInPlay::locked` misses it
  (`location_intercept_reach` did, for Political Antagonist).
- A strike's riders that need it resolved sit in `resolve_strikes`' landing
  loop, not in the "strike: combat ends" block beside Oubliette's (Escaped
  Mental Patient's burn was never set there).
- A Shell Break become a retainer is no `CardType::Retainer` card: a new
  reader of a card in play asks `GameState::is_retainer` or
  `card_in_play_matches`, never the printed type (hand, library and ash
  heap only).
- `Engine::note_play` runs before a play's effects: a "once" spent there is
  already spent for a card the play brings in (Pack Alpha's Raven Spy for
  Ananasi Vampirephile's once-each-turn grant): spend after the effects
  loop (`Engine::spend_turn_grant`).
- A political action card's effects other than its referendum never run
  (`succeed`'s political branch): an `Effect::ActionLayer` there was
  silently dropped until pushed as the referendum starts.
- `Engine::announce_paid` rebuilds the play's `PlayCtx` from its card,
  mode, actor and target: a field set before it (a grant payment's
  `grant_surcharge`, Phagian) is lost unless carried there by name.
- `Effect::family`'s arms are long runs of variants ending in one family:
  a variant listed in the run ending `=> Family::Play` but implemented in
  `apply_shared_effect` (or the reverse) compiles and passes the families
  test, then panics as it resolves (`a play effect without its arm`:
  Patterns in the Chaos' `PeekBledHand`). Read the run's last line.
- A card a search pays for outside a play (Antonio d'Erlette's mage ally,
  Magic of the Smith's equipment) takes its cost reductions only through
  the cost terms (`CostThen::AllyFound`, `EquipFound`): the ally search
  paid its cost directly, The Shard and Powerbase: Tshwane never offered.
- The burn option also reads the card's target filter
  (`trait_target_requirements`): a plays-as vampire barring it (Lisandro,
  LSJ 20050225) hid behind a targeted master's clan filter (Regenerative
  Blood's "a Samedi you control"); test such a fix with an untargeted card
  (Blessings of the Loa, Dirty Contract).
- While a Winged Second chooses its strike, `CombatCtx::sides[i]` is the
  second's own `Side` and `range` reads close (`second_swap`): a reader of
  the combatant or the range outside that request (the host's view and
  narration) goes through `combatant_side` / `combat_range`, or it shows
  the second as the combatant.

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
  commits first, or the change moves to a temporary worktree (CLAUDE.md
  "Harness", owner 2026-11-10). Imperfect isolation is accepted where
  files overlap; silent inclusion is not.
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
