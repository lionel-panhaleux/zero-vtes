# zero-vtes — Design Document

A rules-enforcing VTES engine and a self-play deep-RL agent for the V5 card pool.

Settled architecture and modeling decisions, and a current-state reference of the
engine (§11). History lives in git; rules answers live in
[`docs/rules-qa.md`](docs/rules-qa.md) and are not repeated here. Sources: the
[VEKN rulebook](https://www.vekn.net/rulebook),
[vtes-advanced-rules](https://github.com/vtes-biased/vtes-advanced-rules) ("spec §x":
judge-level semantics, the spec for effect semantics), the
[play procedures draft](https://gist.github.com/lionel-panhaleux/766eedd27ff26ae30169508ddc6bf95c)
(shortcuts, declarations), the
[jol-rebuild](https://github.com/VTES-JOL/jol-rebuild) window taxonomy (studied, not
reused), card data from [KRCG](https://static.krcg.org/data/vtes.json).

---

## 1. Vision

1. **A fast, rules-enforcing V5 engine** (Rust): deterministic, replayable, driven
   through a serializable decision protocol any client can use.
2. **A self-play agent** (PyTorch, AlphaStar-style scaled down) playing legal,
   competent, eventually strong VTES on 4–5 player tables with the V5 precons.
3. **A pluggable bot**: the policy behind the engine-as-rules-oracle, a community
   training partner.

Non-goals (v1): open deck building, negotiation ("silent VTES": signals only
through actions and votes), the legacy pool (not precluded, not attempted).

## 2. Prior art — what we take and what we reject

| Source | Take | Reject |
|---|---|---|
| **AlphaZero** | Self-play + policy/value net spirit | MCTS on true state (hidden info), two-player zero-sum assumptions |
| **AlphaStar** | Entity-transformer encoding, auto-regressive factored actions with masking, asymmetric critic (omniscient value net, legal-view policy), league training (main + exploiters + snapshots, PFSP) | Imitation bootstrap (no VTES replay corpus), TPU-scale compute |
| **Pluribus** | License to proceed without multiplayer equilibrium guarantees; fixed non-adapting strategy; cheap play-time compute | CFR/blueprint (needs short horizons and information abstraction VTES lacks) |
| **jol-rebuild** | Window taxonomy + anchor-card method, mechanics coverage checklist, convergent validation of impulse/sequencing | Client-first architecture, legality-as-guards (vs enumeration), permissive/enforced dual mode, card-text parsing for behavior |

VTES is turn-based with engine-enumerable legal actions and low per-decision
branching; the burden is rules complexity — the engine is most of the project.

## 3. Scope: the V5 pool

Pool = the V5-era products (KRCG set data): Fifth Edition, Fifth Edition (Anarch),
Fifth Edition (Companion), New Blood, New Blood II, New Blood III, Fall of London,
Shadows of Berlin, Echoes of Gehenna — plus legacy cards the owner adds one by one
(`EXTRA_CARDS` in `scripts/window_matrix.py`, read by every generator and the
ledger): Camera Phone, Laptop Computer, Direct Intervention, Wash (2026-09-25).

- **625 unique cards**: 200 crypt (all vampires), 425 library (421 V5 + 4 extra).
  V5 library by type: Master 124, Combat 63, Action 59, Action Modifier 53,
  Reaction 40, Equipment 23, Event 20, Ally 19, Political Action 19, Retainer 12
  (multi-type cards counted per type).
- **Engine pool = the whole pool**, Events included (conditional delayed
  replacement, tokens, global capacity/sect layers, randomness, mandatory unlock
  decisions). Playing an event is an optional discard-phase action, so events
  stress the effects framework, not the decision protocol.
- **v1 training roster**: the 14 full-size precons (Fifth Edition 7 · Anarch 4 ·
  Companion 3), from KRCG `_set` bundle codes: 379 unique cards (138 crypt, 241
  library), 17–41 uniques per deck. The New Blood mini-starters and the
  singles-only cards (Fall of London, Echoes of Gehenna, part of Shadows of
  Berlin — all the events) are implemented but in no v1 decklist.
- **Deferred — zero V5 tenants** (hooks designed, never opened): torpor
  replacement (`WOULD_GO_TO_TORPOR`), burn replacement (`WOULD_BE_BURNED`),
  bidding free-for-alls.

The set list is configuration: extending toward legacy means adding cards and
enabling deferred windows, not changing the framework.

## 4. Engine architecture

### 4.1 The core inversion: decision requests

The engine is a resumable state machine. It advances deterministically until some
player must choose, then emits a **decision request**
(`DecisionRequest { seat, window: WindowLabel, options: Vec<Choice> }`; the
observation — the requester's legal view — is P3 work, §5). The answer is an index
into `options`: the entire interface for the network, scripted bots, the fuzzer
and human clients.

**Factoring rule**: combinatorial choices are decomposed into unit choices
(1 pool at a time, one target at a time, terms stepwise). No decision enumerates
a product space. This matches the auto-regressive policy head and the tournament
declaration protocol (play procedures §X.1).

**Determinism & replay**: a game is `(seed, [answers])`; all randomness (shuffles,
random discards, coin flips) comes from the engine's own `Rng`. The decision log
(`Engine.log`, real decisions only) is the replay format and a future imitation
corpus. Rewind = truncate + resimulate (the mechanism behind human shortcut
interrupts, §8).

### 4.2 Two window primitives

Everything with timing instantiates one of two primitives (`window.rs`) plus
deterministic auto-steps:

**Ordered window (impulse/sequencing)** — `OrderedWindow`, rulebook Sequencing:
offers rotate in a defined order (acting Methuselah first; directed → target
first; undirected → prey, predator; then clockwise); any play returns the impulse
to the acting Methuselah; closes when all pass consecutively. Parameterized by
ordering and legality filter (one `Window` per filter). The **as-played window**
(rules-qa 2.4) admits exactly wakes and cancel-as-played effects — no unlock
reactions, no in-play abilities. During an action it opens after every card play
(a wake there resolves before the played card's effect); in the master phase it
admits cancels only — the others may also play out-of-turn masters in the phase,
nothing else (rules-qa 1.5).

**Unordered commitment window (polling)** — `PollingWindow`: no priority holder,
no reset. Repeated rotations from the acting Methuselah, clockwise; each offer is
an **irrevocable** commitment or a pass **without forfeit**; closes when a full
rotation passes consecutively, so every commitment reopens a response opportunity
for everyone before tally. The atomic unit is the **source** (rules-qa 1.1): a
vampire or a Methuselah (Edge, card-discard ballots, Methuselah-vote effects);
a source casts all its votes at once, all agreeing; sources are independent.

They **interleave**: during polling, votes use the polling window while card plays
stay ordered and commit within the same rotation. Witness: **Scalpel Tongue**
(targets "vampires who have cast votes") — so per-vampire vote-cast state and
"referendum passing automatically" are engine-visible, and early-vs-late commitment
is strategically live; the rotation scheme preserves the option to wait at the
cost of a positional artifact within a rotation (§10.1).

### 4.3 Window elision — the formal analog of human shortcuts

> **Elision rule**: a decision request is emitted only if the player has more
> than one option. A request whose only option is pass is auto-passed; a forced
> move (single mandatory option) is auto-played.

Implemented in `Engine::run`: any single-option request is applied without
logging. Soundness: an auto-pass must be **observationally identical** to a chosen
pass from every other seat (no hidden-hand information leaks) — a tested
invariant. Elision is what makes self-play viable (only live choices reach the
network; credit assignment shortens accordingly). Block attempts that cannot
succeed are pruned the same way (§11.2 "Block loop").

The human-facing shortcut protocol of the play procedures (binding shortcuts,
interrupt-and-rewind, "stops") is a client-layer concern on top of the decision
log (§8); training never sees it.

### 4.4 Workflow state machines

The rulebook's structures are taken literally as nested state machines (`Flow` in
`game.rs`, `ActionState`, combat `Step`, `RefStep`); each state is bookkeeping or
hosts a window.

- **Turn**: Unlock → Master → Minion → Influence → Discard, with phase budgets
  (master phase actions, transfers 1/2/3/4, discard phase actions). The master and
  discard phases are first-class offer windows (master plays and abilities with
  cancel sub-windows; discard-or-event as the discard phase action). Each of the
  active Methuselah's phases ends on an impulse rotation over the other seats
  (`Flow::PhaseRotation`, rules-qa 7.9). Unlock-phase ordering: §10.7.
  **Contest upkeep factoring**: each Methuselah settles *their side* of each
  contest during *their own* unlock phase — self-contest is impossible, so nobody
  orders both sides and no cross-player simultaneity exists. Each contested object
  is a binary pay/yield decision (1 pool for a card, 1 blood for a contesting
  titled vampire); forced yields (no resource, torpor) are elided; gaining another
  title while contesting yields the contested one. Two broke princes resolve
  naturally: whoever's unlock phase comes first is force-yielded. Yielded cards
  burn with everything on them; a card whose rivals all yielded returns face up
  on its controller's next unlock phase.
- **Action** (rulebook "Detailed action flow"): declare (lock, announce card and
  choices) → as-announced window → block loop → [blocked: block resolution →
  combats → after-combat window] → resolution → after-resolution window →
  possible **continuation** (Form of Mist re-enters the block loop with declines
  kept, rules-qa 3.4). The after-combat window opens once, after all combats of
  the action (rules-qa 2.1).
  **Block loop** (rules-qa 1.3): one ordered window, not a success/failure
  checkpoint. A declared attempt stays pending until all pass consecutively (any
  play voids prior passes), so third parties can still add intercept or strip
  stealth after the attempting Methuselah passes. At closure intercept ≥ stealth
  → block; otherwise the attempt fails and the loop returns to "no attempt"
  (rulebook B.4), where the same Methuselah may attempt again. A pass **with no
  ongoing attempt is a final decline**, reopened only by a change of target
  (bounces: Deflection, Telepathic Misdirection — not the card Change of Target,
  which ends the action). Hence a third-party helper needs the defender to
  attempt first.
  **NRA** (rules-qa 1.4): enforced in legal-action generation; locked on
  *resolution* (blocked counts, canceled-as-announced does not); identity by card
  **name** for played cards, by **copy** for cards in play; one bleed and one
  political action per minion per turn; other rulebook actions unrestricted
  unless a card says "cannot take the same action".
- **Combat**: rounds of {before range → determine range (alternating maneuvers)
  → before strikes → strike choice (acting first; a maneuver from a strike card
  or weapon commits the strike) → before resolution → resolution (combat ends
  first, else simultaneous) → prevention → damage → additional strikes → press →
  end of round}, with would-end replacement (Telepathic Tracking); combat ends at
  once when a combatant is not ready. Impulse-reset within each step (rules-qa
  1.2). "Acting minion" = the minion performing the action, blocked or rushed;
  the other is a "blocker" only if it blocked. Combats with no acting minion
  exist: first opportunity goes to the acting Methuselah's minion, else clockwise.
- **Referendum**: terms (stepwise, factored) → polling (unordered votes + ordered
  plays; "before votes are cast" is a requirement inside polling, not a window)
  → tally → the action's after-resolution window.
- **Diablerie / blood hunt**: one workflow whatever the entry path; the blood hunt
  is a restricted referendum (vote sources only, no cards, no terms) inside the
  diablerie action, the after-resolution window after it.

Window names are specific (`ActionAsAnnounced`, not "interrupt") — one name per
legality filter.

### 4.5 Cards and effects: one representation, two derivations

> **Playability and execution derive from the same card representation.**
> There is no hand-written `can_play` separate from `execute`. The legal-action
> mask is *generated*, and therefore cannot drift from behavior.

- **Static data** (name, types, cost, disciplines, clan, capacity, group, title,
  requirements, burn option, ally body, labels) is generated from KRCG data —
  never hand-typed (`crypt.rs`, `library.rs`).
- **Behavior** (owner, 2026-09-03): a card is a set of
  *modes* (its printed Discipline/type lines); a mode is
  `ModeDef { windows, actor, requires, target, terms, waives_needed, limit,
  effects }`; a `CardDef` adds in-play `abilities` (same shape), `riders`,
  trifle/out-of-turn/delayed-replacement flags, and takes its printed cost from
  the generated half. The **legality half** is a **closed data
  vocabulary** of named predicates evaluated by the engine — no card decides its
  own legality in custom code — so the mask is enumerated from data. The
  **execution half** is interpreted primitives (`Effect`, `RefEffect`) with
  custom arms where needed. In-play abilities have the same shape with an in-play
  source; continuous effects are layers; riders bind to named `Hook`s. Behavior
  is authored as Rust data tables (`cards::defs`, `cards::vampires`) keyed by
  printed name, checked by the compiler against the generated static half.
  Numeric terms are stepwise sub-decisions at play (Villein) or at the
  referendum's terms step (rules-qa 3.5).
- **Mask policy** (owner, rules-qa 4.3): the mask removes only what the rules or
  the card forbid — alternation, per-scope limits, "only when needed" (stealth/
  intercept plays carry `StealthNeeded`/`InterceptNeeded` unless waived, rules-qa
  3.2), "cannot be played with no damage to prevent". A legal play with no effect
  stays offered: cycling a card is common and deliberate.
- **Scopes**: layers (`Layer { scope, modifier, source }`) are `Scope::InPlay`
  (while the source is in play and present) or `Scope::Action` (dropped when the
  action ends). Round/combat state lives on `CombatCtx`/`Side`, action state on
  `ActionCtx`, per-phase/turn use counts on `GameState.phase_uses`/`turn_uses`.
  Everything conditional is **read live**, never snapshotted (§11.2).
- Weird cards get custom code behind the same interface; the framework is sized
  against the stress set (§6), not the median card.

Semantics authority for requirements vs costs, cancellation vs replacement,
stacking, simultaneity, NRA, changing acting minion: **vtes-advanced-rules**.

### 4.6 State representation

Flat, arena-style, plainly cloneable, no interior mutability (`GameState`:
`seats`, `minions`, `cards`, `layers`, …; entries are never removed — flags mark
what left play). Zones per rulebook: ready / controlled / uncontrolled / torpor /
ash heap / hand / library / crypt (+ removed). Counters, Edge, pool, labels as
plain data. `GameState::hash` for replay checks. Card behavior resolved to static
tables (`cards::lookup` is indexed by library index).

## 5. RL-facing surface

Planned for P3; nothing here exists yet beyond `Engine::run`/`choose`.

- **Dual observations**: every request carries the decider's legal view; training
  can also request the omniscient view for the asymmetric critic. Nothing
  omniscient reaches the policy. (`GameState::hand_visible_to` already answers
  open-hand queries.)
- **Options as feature vectors**: the policy scores the options presented
  (pointer-network style), not indices into a fixed vocabulary — pool growth
  changes data, not network shape.
- **Batched stepping**: a `GamePool::step_until_decisions()`-style call advancing
  N games and returning `(game, player, obs, mask)` batches for one GPU inference,
  exposed via PyO3 with zero-copy tensors; the serialized protocol is for clients.
- **Encoding sketch**: entity tokens (players with relative-seat encoding,
  minions, hand, in-play cards), transformer trunk, auto-regressive head over
  options; reward = tournament scoring (VPs, GW), small survival shaping early;
  league = current agent + snapshots + exploiters + scripted bots, PFSP; decks ×
  seats sampled per episode; policy conditioned on own decklist identity.

## 6. Window census (V5 pool)

The per-card truth is [`data/window-matrix-reviewed.csv`](data/window-matrix-reviewed.csv):
every library card classified per *mode* by play window plus flags (delayed
replacement, out-of-turn, in-play granted actions, bounce, …), hand-reviewed and
owner-arbitrated (rules-qa Batch 2).
[`scripts/window_matrix.py`](scripts/window_matrix.py) generates the draft
and, with `--check data/window-matrix-reviewed.csv`, verifies it still
reproduces the reviewed matrix — the regression check against card-data updates. `gen_library.py` takes
the windows from it. The plays test checks that split modes' windows cover the
matrix exactly.

Scale (pre-review regex census, indicative): prevention ~30 cards, polling ~47,
press 18, additional strike 11, events 20, cancel-as-played 9, wakes 5, bleed
redirect 5; torpor/burn replacement and bidding **0** (deferred).

**Stress set** (the framework had to express these before freezing; all defined):
Archon Investigation · Scalpel Tongue · Form of Mist · Telepathic Tracking ·
Deflection · Eyes of Argus · Sudden Reversal · Consign to Oblivion · Immortal
Grapple · Blur · Villein · Homunculus · Anarch Revolt · Kine Resources Contested ·
a contested unique and a contested title (`tests/contests.rs`, `game.rs` unit tests).

## 7. Testing strategy

The engine's value is trust; trust is measured, not asserted.

### 7.1 Rulings as tests

`data/rulings-v5.csv` is the ledger: the vtes-rulings YAML (card-keyed prose with
provenance — hand-translated, not generated) filtered to the pool. Every row on an
implemented card (`DEFS` or `VAMPIRES`) carries a `status`: `test:<fn>` (a
`#[test]` under `crates/engine/tests/` whose comment cites the row's references;
new ones in `tests/rulings.rs`), `blocked:<gap>` (a card or mechanism the engine
lacks, revisited when it lands) or `n/a:<reason>` (not engine-observable, or the
card is only the ruling's example). Rows on unimplemented vampires have no status
yet. `scripts/rulings_v5.py --check data/rulings-v5.csv` verifies the ledger
against the tests offline; regeneration keeps the statuses of unchanged rows and
drops reworded ones as stale. The invariant is the ruling's assertion: a script is
corrected as a script (fixture, window order), but engine behavior is never bent
to a script — a real divergence is fixed in the engine or the row is `blocked:`.
Scenario tests are decision scripts using KRCG card names.

### 7.2 Property fuzzing

`tests/fuzz.rs`: random-legal games from the opening, a seeded midgame, and one
table per card family (each new family gets its own `random_games_with_*` test and
coverage keys). Invariants: card and counter conservation modulo explicit sources
and sinks (`Seat.removed`, cards stored or face down on banished vampires), no
negative resources, capacity ceilings, every card in play has a living
controller, a game ends only with ≤1 Methuselah or at the time limit, step budget
(every window terminates), no empty request, decision-log resimulation reproduces
identical state hashes. Observational equivalence of elision is still to be
tested once observations exist (P3).

### 7.3 Golden replays

Pinned `(seed, log)` replays across refactors — not built yet.

### 7.4 Coverage checklist

Every mechanic answers "where is it handled, where is it tested" — §11.2 is that
map. The in-play hook vocabulary was closed by enumerating the pool's in-play
abilities (a since-retired regex census); every hook has tenants (`Hook` index
in §11.2).

### 7.5 Perft-style counts

Number of legal decision sequences to depth k on scripted positions, to catch
legality regressions numerically — not built yet.

## 8. Community surface

- **Wire protocol**: the same requests/answers, serialized (JSON/msgpack), over
  socket/WebSocket. Non-enforcing platforms (JOL, Succubus Club) can host the bot
  only with the engine embedded as rules oracle.
- **Human ergonomics layer** (not in the core): binding shortcuts with
  interrupt-and-rewind (log truncation + resimulation), preemptive impulse
  requests ("stops"), pace defaults — the play-procedures document is the spec.
- Bot service: engine + ONNX-exported policy, CPU-friendly.

## 9. Performance targets

- Engine step: µs-scale; a full 4p random-legal game in low single-digit ms.
- Self-play: thousands of concurrent games per box, NN inference the bottleneck.
- Full determinism under fixed `(seed, answers)` across platforms.

## 10. Explicit modeling decisions

Recorded so nobody mistakes them for oversights:

1. **Polling serialization**: free-for-all vote casting is serialized as repeated
   no-forfeit rotations from the acting Methuselah, clockwise (§4.2).
   Order-dependent edge cases (Scalpel Tongue) are preserved; pure simultaneity is
   not.
2. **Silent VTES**: no negotiation channel; deals are social, never mechanical.
3. **Elision**: provably-empty windows are skipped by the omniscient engine;
   observational equivalence is an invariant; live-play pacing tells are a client
   concern.
4. **Deferred windows**: torpor/burn replacement and bidding — nothing in the
   pool opens them.
5. **Fixed decklists**: the official precons; deck building is out of v1.
6. **Tournament scoring as reward**: VPs + GW. Time limit `TURN_LIMIT` = 100
   player turns (owner, 2026-09-02), checked at the turn boundary, 0.5 VP per
   survivor; VPs stored in half-points (`Seat.half_vps`); `GameState::game_win`
   = 2+ VP and a strict lead, ousted or not (a tie is a draw). Tournament
   withdrawal (0.5 VP, an unlock-phase decision) is not implemented yet (§11.3).
7. **Unlock-phase ordering** (rules-qa 3.7, 7.8): cards unlock first, unordered;
   then one ordered window in which the active Methuselah settles their mandatory
   effects (contest upkeep and pending riders: `Pending::{Rider, Contest}`) in the
   order they choose among their optional ones (the Edge's pool, abilities), and
   cannot pass before the mandatory ones are done; the other players'
   unlock-phase abilities follow in rotation, the impulse returning to the active
   player after each — the Edge's pool still on offer then, lost once everyone
   passes.
8. **Mid-combat ousts**: a pool loss inside a combat (Fame's torpor rider) ousts
   at once (`apply_damage` reports the flow moved on). If the active Methuselah is
   ousted the turn passes with the action and combat torn down; if the opponent's
   Methuselah is ousted their minion leaves with them, the combat ends at once (no
   end-of-round window), and the action resumes at its after-combat window
   without the ousted seat; a directed action whose target left is not continued
   (Form of Mist masked). Two seats left ends the game.

## 11. Phasing and engine reference

### 11.1 Phases and slices

Slice IDs are cited by module docs, test section headers and rules-qa batch
titles; `git log` has the detail.

| Phase / slice | Done (2026) | Scope |
|---|---|---|
| P0 skeleton | 09-03 | Decision loop, both primitives, turn machine, generated V5 crypt, card-free rulebook actions, influence, card/title contests, fuzzer + replay |
| P1 · 1–2 | 09-03 | Library zones, master/discard offer windows, as-played cancel stack; the action machine, block loop, modifiers/reactions, wakes, bounces, NRA |
| P1 · 3–4 | 09-04 | Combat machine (`combat.rs`); referendum machine (`referendum.rs`), political actions |
| P1 · 5, 5b–5i | 09-06 | Cards in play: riders, granted actions, library-card contests, leaving play, oust by control, untimed abilities, counters, control change, phase impulse rotations |
| P1 · 6 | 09-07 | Rulings-as-tests harness (§7.1) |
| P1 · 7 | 09-13 | Replacement and derived hand size |
| P1 · 8a–8d | 09-14/15 | Equipment (8a), weapons (8b), library search (8c-i), stored cards and draw alternatives (8c-ii); stress set closed (8d) |
| P2 · 9a–9d | 09-15/25 | Printed vampire text: stat modifiers (9a), combat abilities (9b), abilities outside combat (9c-i/ii), printed actions and card targets (9d, 9d-iv) |
| P2 · 10a–10c | 09-25 | Allies and retainers (`MinionKind::Ally`), their precon cards |
| P2 · 11a–11d | 09-25 | Precon cards on existing vocabulary (a), wakes and forced block attempts (b-i–b-iii), block tolls (c), cancellation (d) |
| P2 · 11e–11h | 09-25 | Titles and votes (e), combat staples (f), archetypes (g), Amici Noctis (h) |
| P2 · 11i–11n | 09-26 | Combat cards (i, j), action modifiers and reactions (k), after-resolution windows and remembered Methuselahs (l), corruption and multi-Discipline (m), Eagle's Sight (n) |
| P2 · 11o–11r | 09-26 | Actions waves 1–2 (o, p), masters and created vampires (q), last precon equipment (r): every precon card defined |
| P2 · 11s–11v | 09-26 | Pool combat cards (s), reactions/modifiers (t), actions (u, v) |
| P2 · 11w–11z | 09-26 | Pool masters (w–y), cost reductions and Fleshforge Chamber (z) |
| P2 · 11aa–11ab | 09-27 | Last allies, equipment, Mobile HQ, Minion Tap (aa); last political actions (ab) |
| P2 · 11ac–11ae | 09-27 | The 20 events: Gehenna and held replacements (ac), unlock-phase events (ad), sect falls, Liaison, token allies (ae) — every library card defined |
| P2 · 12a | 09-27 | The last 19 vampires' printed text: every vampire defined (5 partial) |

### 11.2 Engine reference (by mechanism)

Where each mechanism lives and the modeling calls not obvious from the code or
the rulebook. Rules answers are in rules-qa; cited here only as pointers.

**Crate layout** (`crates/engine/src`): `game.rs` (`Engine`, `Flow`, run loop,
turn structure, ousts), `window.rs` (primitives), `decision.rs` (`Choice`,
`WindowLabel`, `ActionKind`), `plays.rs` (card plays, abilities, costs,
`apply_card_effect`), `actions.rs` (`ActionCtx`, block loop, resolution),
`combat.rs` (`CombatCtx`, `Side`), `referendum.rs` (`ReferendumCtx`),
`contests.rs`, `influence.rs`, `draws.rs` (`Interrupt`, draws, searches),
`state.rs` (`GameState`, `Minion`, `CardInPlay`, `Seat`, live stat reads),
`cards/mod.rs` (the vocabulary: `Window`, `Actor`, `Requirement`, `Terms`,
`Effect`, `Modifier`, `Hook`, `Limit`, `CardDef`, `VampireDef`), `cards/defs.rs`
and `cards/vampires.rs` (definitions), `crypt.rs`/`library.rs` (generated).
Tests: one file per family under `crates/engine/tests/` (`precon.rs` holds the
slice-11 cards by section).

**Run loop and interrupts** (`game.rs`, `draws.rs`)
- `Engine::run`: settle interrupts, drain excess blood (`drain_all_excess`),
  release hostages, then request; single-option requests auto-apply.
- A choice owed by a specific player in the middle of a resolution is an
  `Interrupt` (`Engine.interrupts`), served in order before any other decision:
  discard-down, draw alternatives, replacement treatment, entry effects
  (`EntryBurn`), margin distribution, `MinionOf` picks, block tolls, etc. A player
  with nothing to choose resolves at once.
- An ousted seat drops out of open windows (`OrderedWindow::remove`).
- `Window::AnyImpulse` (untimed "you can lock this card" abilities) is offered
  wherever a seat holds the impulse — phases, unlock rotation, action/combat
  windows, polling — never in as-played or terms windows; using one is an effect
  (impulse reset / polling commitment). A starving vampire's forced hunt is
  offered beside untimed abilities, not instead.

**Turn and phases** (`game.rs`, `contests.rs`, `influence.rs`, `plays.rs`)
- Unlock: §10.7 (`Flow::Abilities`, `Pending`). Cards a Methuselah controls
  unlock in their unlock phase. Substitutions: `Minion.skip_unlock`,
  `Modifier::CounterInsteadOfUnlock`, `HostDoesNotUnlock`. The rulebook burn
  option is a discard in any unlock phase (`Seat::burn_option_used`).
- Master phase: out-of-turn masters are accounted against the next master phase
  (rules-qa 3.3); a canceled master gives its player a master phase action back
  when Wash says so (`Seat.mpa_next`). Abilities paid with a master phase action
  are not once-per-phase.
- Minion phase: `Engine::mandatory_actions` — a mandatory action (printed,
  Puppet Master's take) is the only offer and the phase cannot end before it; a
  minion unable to take it declares nothing (spec §3.9). Several mandatory
  actions are ordered by the acting Methuselah (rules-qa 17.2).
- Influence: transfers, `Choice::Withdraw` (2 transfers: a counter back to pool),
  crypt draw (4 transfers + 1 pool), influence-out, no voluntary self-contest;
  `Window::InfluencePhaseOwn` (once per phase), `Window::Transfers` (paid in
  transfers); `GameState::transfer_bonus`.
- Discard: one discard phase action (`Stat::DiscardPhaseActions` read as the
  phase begins, `Seat::granted_dpa`); `Hook::DiscardPhase` riders fire as the
  impulse leaves the active Methuselah. `Seat.hand_bonus` clears at the end of
  the turn.
- "During X, do Y" abilities are once per that phase per copy (`phase_uses`,
  rules-qa 6.6); an ability spending a resource with no "during" clause
  (`DiscardPhaseAction`, `Transfers`) is limited only by the resource.

**Card plays** (`plays.rs`: `PendingPlay`, `PlayCtx`)
- Offers are enumerated from definitions: windows, actor, requirements,
  targets, limits, affordability counting potential reductions.
- Announcement: terms (`Flow::PlayTerms`), cost reductions (`Flow::CostTerms`),
  then the as-played window (`Flow::AsPlayed`, a nested cancel stack, spec §1.8).
  Costs are folded through layers and paid at resolution for actions (unpayable
  → fizzle: successful, no effect; rules-qa 3.6).
- Cancel as played: the card is replaced; its own "do not replace until" is
  canceled with it; a canceled limited bleed leaves the limit unused; cancelling
  abilities (The Vozhd of Gravesend) mark the play canceled as used
  (`as_played_apply`). Cancels by type/Discipline read the mode played.
- Multi-Discipline cards: the generator gives one mode per non-empty set of their
  Disciplines (`Label::MultiDiscipline`); a mode's tags are all required, as on a
  combo card. A definition may split a printed "or" into modes, mapped back by
  `cards::library_mode`. Surcharges read the Disciplines used
  (`played_disciplines`).
- Cards played from a location (Fleshforge Chamber: `virtual_hand`,
  `PlayCtx::from`) are not replaced; `Engine::playable_card` reads a play's card.
- Allies playing cards "as a vampire" (`Modifier::PlaysAsVampire`): meet one
  basic Discipline, pay in life (last life burns them; the card then has no
  effect, an action resolving first).

**Actions** (`actions.rs`: `ActionCtx`, `ActionState`; `ActionKind`, `ActionId`)
- Kinds: rulebook (bleed, hunt, leave torpor, rescue, diablerie, political),
  `Card`, `Equip`/`EquipFrom`/`EquipFound`, `Recruit`/`Employ`, `BurnCard`,
  `StealCard`, `Ability` (granted by a card in play or printed on a vampire;
  gate `Engine::may_take_granted`). A granted action has only its printed
  stealth (rules-qa 6.2).
- Directedness is `ActionCtx::directed_seat`: bleed target; rescue/diablerie of
  another's vampire; a minion, seat or card target's controller unless the acting
  Methuselah (a card target never makes the action directed at its minion). The
  Ⓓ icon is only a reminder (rules-qa 17.4, 20.1). `GameState::may_direct_at`
  (Shadow Cloak, Secure Haven) filters directed actions of every kind.
- Windows: as-announced (acting Methuselah, then others: `ReactionAsAnnounced`)
  → block loop (`ActionBlockAttempt`, `ReactionUnlock`,
  `ReactionUnlockAttempt`) → `ActionState::Blocked` (Crypt's Sons continue as if
  unblocked) → block resolution → combat → after-combat → for a bleed of 1+,
  `ActionWouldBeSuccessful` → resolution → after-resolution.
- **After resolution always opens** once an action reached resolution —
  successful, fizzled, failed, blocked (after its combat), or ended before block
  resolution (Change of Target, Mirror Walk; no card is played there). Only a
  cancel before resolution or an acting minion leaving skips it (`action_windows`).
  Damage "after resolution" is ordered there by the acting Methuselah.
- Resolution: costs paid then; targets re-read — gone or illegal → fizzle, paid
  (card actions) or successful with no effect (granted actions, spec §3.4.1). An
  action whose acting minion left play fails. A bleed target ousted before
  resolution: fizzled, no pool, no Edge. A **successful bleed** is one of 1+;
  only it takes the Edge (`ActionCtx.bleed_successful`); a bleed at 0 is still a
  successful action. A hunt succeeds only if it gains blood.
- **NRA**: `ActionId` — card name for hand cards, `InPlay(card, ability)` per
  copy and ability (LSJ 20100909), `Printed(vampire, ability)`, `Rulebook(kind,
  target)` (Change of Target's ban); `Effect::SameActionAs` unifies two printed
  actions. Recorded at resolution (`mark_resolved`); a failed bleed or political
  action counts. Delaying Tactics bans by `ActionId`.
- **Live conditions** (owner, 2026-09-25): an effect's conditions are evaluated
  throughout, never snapshotted (`Effect::If` → `ActionCtx.conditional`); a
  limited bleed increase counts against the limit only while its condition holds.
  The ruling-backed exception: Night Terrors' −X stealth is fixed at play (RTR
  20030519).
- Intercept given to a minion lasts the whole action, across failed attempts
  (`ActionCtx.intercept_given`, rules-qa 15.2).
- Enter-combat actions (`Effect::EnterCombatWithTarget`): the combat begins at
  resolution (`start_combat_at_resolution`) and is part of the action; no "held
  when blocking" state; a blocked rush continued as if unblocked enters its
  combat at resolution (rules-qa 17.3).
- Action-scoped riders live on `ActionCtx` (block riders read at announcement,
  `barred_blockers`, `vampires_cannot_block`, `no_unlock`, `no_reactions`,
  `put_if_no_block`, `after_resolution_unlocks`, `cost_reduction`, …); combat
  riders from the action go through `Engine::combat_riders`.

**Block loop** (`actions.rs`)
- Eligibility: prey/predator/target rule (`eligible_blockers`), widened per
  attempt by `can_attempt_block_anyone` (Eagle's Sight: one attempt, not the
  acting Methuselah's own minion nor a decliner nor a barred blocker).
- Forced attempts (`Effect::AttemptBlock`, `R::CanAttemptBlock` →
  `can_attempt_block`). A card-made failure bars re-attempts this action.
- Tolls (`block_toll`): paid as the attempt is made; an attempt under way when a
  toll lands pays or resigns (`Interrupt::BlockToll`).
- **Reachability pruning** (`intercept_reachable`): an attempt that cannot reach
  the stealth is not offered. It counts the blocker's own and printed intercept,
  intercept abilities of cards on it, other cards in play
  (`location_intercept_reach`: Market Square, KRCG News Radio, The Rumor Mill),
  and stops pruning while a non-acting Methuselah holds a playable stealth strip
  (`stealth_strippable`). Any new third-party intercept/stealth-strip card must
  be counted here, or pruning hides a live attempt.

**Combat** (`combat.rs`: `CombatCtx`, `Side`, `Step`, `Damage`, `Held`)
- Only combat cards are played during combat — no modifiers, reactions or wakes
  (spec §4.1.3). A combat is part of its action: `Engine::action` and its layers
  stay in force.
- Strikes: a weapon strike remembers its source (`Side.strike_source`) and does
  nothing if the weapon left; a maneuver from a weapon or strike card commits the
  initial strike (`Side.committed`, `committed_card`); the round's one limited
  additional-strike source; a strike not said to be ranged is close range only
  (rules-qa 21.1). Strike riders fire as the strike lands (in range, not dodged,
  not aimed at a retainer). Ranged strikes at long range may be aimed at a
  retainer (`Side::strike_at`).
- Damage: prevention per side, acting side first, then third parties from outside
  the combat (`Step::OthersPrevent`); the preventer picks which damage a
  prevention covers (`Step::AllocateSource`); applied at once, normal then
  aggravated (the rulebook's examples are unit tests). Environmental damage lands
  in the normal strike resolution. Unpreventable damage (`Side::unpreventable`)
  stays out of `pending`.
- Combat ends: `Effect::EndCombat` goes to the end-of-round window; no new round
  once a combatant left the ready region; mid-combat ousts §10.8.
- Delayed "put this card on …" effects: only action cards wait in limbo; other
  combat cards go to the ash heap and the effect is an effect-only card in play
  (`CardInPlay.effect_only`), lost if the strike misses, combat ends first, or
  combat continues (rules-qa 21.1).
- Per-combat/round/action ability uses key on the card copy: a card's abilities
  share one use (rules-qa 38.8).
- Frenzy: `Effect::Frenzy` marks the frenzy mode; `Engine::frenzy_on` names the
  minion affected (rules-qa 28.11); `frenzy_immune` cancels and bars.
- Weapons: melee `StrikeKind::Melee` (not a hand strike); a weapon's per-combat
  maneuver is `Limit::PerCombat` per copy; `cards::melee_strike`,
  `cards::becomes_weapon` (Weighted Walking Stick is a weapon only in play).

**Referendums** (`referendum.rs`: `ReferendumCtx`, `RefKind`, `RefStep`,
`Source`, `VoteRecord`, `TermsState`; `ReferendumDef`, `Term`, `RefEffect`)
- Terms stepwise by the caller after the action succeeds (feasibility masks);
  a `Term` choice is never a target (rules-qa 38.6). Polling with card plays as
  commitments; per-source vote records (abstentions cancel); Methuselah ballots
  (Edge, the calling card's vote, one political card burned from hand); tally:
  more for than against, ties fail; Scorn first.
- Votes are read live (`votes_of`, `clan_votes`, `calling_votes`, `host_votes`);
  grants (`ReferendumCtx.grants`) add to casts already made. An ousted
  Methuselah casts nothing.
- A canceled referendum returns the card, discarding down at once. Political
  action: +1 stealth, cost at resolution, no referendum if the acting minion is
  no longer ready.
- Simultaneous ousts are one batch (`oust_batch`): a VP to every predator, 6 pool
  to survivors only; every remaining Methuselah ousted at once is a legal end
  (rules-qa 6.9).

**Contests and titles** (`contests.rs`; `state.rs`)
- `Contest::{Card, Title, Library}`. A contested card is out of play:
  `CardInPlay::present()` false, its layers suspended (`GameState::layer_active`),
  skipped by abilities, riders, copy counts and "a card named X". A library card
  enters contest only after its own effects resolve (spec §1.13.2).
- Self-contest is masked (rules-qa 7.1; `controls_unique_copy` also for
  searches); a forced one burns the incoming copy (The Coven).
- Titles are read live (`GameState::title_of`): **claimed** (`claimed_title`: a
  title card on the vampire, else `Minion.granted_title`, else printed unless
  lost) vs **effective** (none while contested, out of play, off-sect, or its
  sect fallen). One title per vampire (`gain_title` loses the printed one and
  burns an older card). A unique title card contests as a card (pool), giving its
  title once the contest ends (rules-qa 19.1); a non-unique one (Toreador
  Justicar) contests as its title (blood). `steal_title` (Valerius) burns the card
  and gains the title itself (rules-qa 19.6). Liaison is
  `Title::Independent { "Liaison", 4 }`, the younger claimant yielding.

**Minions** (`state.rs`: `Minion`, `MinionKind::{Vampire, Ally, Created, Token}`)
- Vampire traits are read live: `capacity_of` (with `capacity_ceiling`),
  `discipline_level`, `has_discipline`, `Minion::clan`, `GameState::sect_of`
  (`SectFallen` over `underlying_sect`, `sect_override`); printed data only via
  `printed_*`. Blood gain caps through `gain_blood`; excess drains as the engine
  settles; out-of-play counters are kept.
- **Allies**: a minion and a card in play at once (`CardInPlay.body`, not
  `host`, so never "a card on a minion"; `CardInPlay::bearer()` = host or body).
  No clan, sect, Disciplines, title, capacity or votes; neither younger nor older
  (`younger_than`, `ally_or_younger`); life uncapped; damage burns life, torpor
  burns it; no hunt/rescue/diablerie/politics, no blood costs; recruited ones act
  next turn (`Minion.recruited`). Its version's type line is kept
  (`MinionKind::Ally.body`). A unique ally contests as a minion (`card_rivals`).
- **Created vampires** (`MinionKind::Created`: Childe of the Revolution, Waters of
  Duat): 1-capacity, sire's clan, printed or sire's sect, no Discipline or title;
  their card is the body. **Tokens** (`MinionKind::Token`: Absimiliard's Army):
  no card in play, name or label; removed without burning when the Army leaves.
- Leaving play has one path each: `burn_minion` (cards on it to owners' ash
  heaps; "ready minion burned" riders only if it was ready) and
  `minions_to_torpor`, both through `leave_riders` batching pool losses. Burned
  vampires reach the ash heap (`Seat.ash_crypt`; its owner's, `Minion::origin`,
  even under stolen control).
- Control: permanent steal (`StealTargetVampire`), temporary
  (`Minion.temp_from`, `Engine::take_control`: returns at the end of the taker's
  turn, removed if its Methuselah was ousted). Banishment (`Engine::banish`)
  returns the vampire as the arena entry it left, cards included
  (`UncontrolledSlot.banished`).
- Per-minion markers: `corruption` (per Methuselah; kept through torpor, control
  change, banishment; cleared when burned/removed), `hostage`, `nightmare`,
  `diablerie_shield`, `must_hunt`, `played_since_unlock` (wake limits),
  `unlock_at_discard`.

**Cards in play** (`state.rs` `CardInPlay`; `cards/mod.rs`)
- Control: `CardInPlay.controller` is state; `GameState::card_controller`
  routes a minion card to its host's controller, a master card stays its
  player's (rules-qa 7.5), `Effect::StillYours` keeps a card its player's on
  another's minion. `Engine::give_card_control` / `give_location` (a location on
  a minion moves onto a ready minion of the new controller that can hold it, else
  burns).
- **Oust**: cards the Methuselah *controlled* are removed (not burned, nothing
  triggers, spec §1.10); other players' cards on their removed minions burn to
  their owners' ash heaps.
- Fields: `mode` (the version it entered with), `by` (the minion that played it),
  `against` (remembered Methuselahs, `SeatSet`), `chosen`, `stored` (library
  cards held on it, burned with it), `counters`, `entered`, `effect_only`.
- Riders (`Rider { hook, scope, requires, effects }`, `RiderScope::{Every,
  Controller, Others, Prey}`): requirements read at collection and at resolution
  (an unmet rider is not pending, its window elides); unlock riders are pending
  mandatory effects ordered by their resolver; one burn per copy;
  `R::OneCopyPerTurn` drops the other copies once one resolves. Vampire riders
  (`VampireDef.riders`) fire at the hook site directly.
- `Hook` index: `UnlockPhase`, `MasterPhaseStart`, `DiscardPhase`,
  `HostToTorpor`, `ReadyMinionBurnedOrTorpor`, `ByLeavesReady`,
  `CombatOpponentLeftReady`, `CombatOpponentOfOwnClanLeft`, `ContractHostLeaves`,
  `WouldReplace`, `WouldDraw`, `HostBlocked`, `HostBlocks`,
  `HostAnnouncesAction`, `ActionAnnouncedAtTarget`,
  `HostAboutToBleedRemembered`, `OwnBleedSuccessful`, `OwnReferendumFailed`,
  `SuccessfulHunt`, `OwnVampireInfluenced`, `PreyOusted`, `GehennaPlayed`.
- Entry: `enter_play` is the single entry point (contest checked by the caller
  after entry effects); `apply_entry_effects` (a retainer's layers on its
  employer, an ally's on itself); `push_mode_layers`.
- A card that becomes another (`cards::became`): its windowless mode only.
  Living Manse is equipment to equip and a location in play
  (`cards::is_equipment_in_play`; out of `equipment_on`).

**Printed vampire text** (`cards/vampires.rs`: `VampireDef`, `Printed`)
- `crypt.rs` stays generated; behavior is `VampireDef` keyed by name. Plain stat
  modifiers are **not layers** (a vampire is not a card in play): read live by
  `GameState::printed_mod`, so nothing is pushed or retracted on torpor, contest,
  burn or oust. Printed text applies in torpor; a contested vampire gives
  nothing. Abilities are `ModeDef`s with actor `Host` = the vampire;
  `AbilitySource::{Card, Vampire}` keys uses and resolution.
- A vampire may land with part of its text, the rest quoted in `incomplete`
  (rules-qa 13.1); fixtures wanting a plain vampire pin one by name.
- Printed predicates read `PrintedCtx`: the action, the opposing minion
  (the blocker for a block toll, the calling vampire for a vote toll) and
  the card played (`PrintedCtx::card`, `R::PlayedCard`). Printed cost
  reductions (`GameState::printed_cost_off`: pool, blood, "blood or pool")
  apply wherever the minion pays a card's cost, a card another puts in play
  included — it is still played (Piper, Contraband, a search; rules-qa
  41.13); Regina Blake's blood is announced with the card like a reduction
  (`PlayCtx::blood_payer`, `Engine::blood_payers`), paid with the named or
  found card's cost in `Engine::pay_named_discounts`.
- "Plays cards requiring a <clan> as if" (`VampireDef::plays_as`) counts at
  card plays only (`GameState::meets_clan`: clan requirement, target, terms,
  `R::ControlsReadyClan`); a card's effects in play read the clan itself
  (`GameState::of_card_clan`).
- Vampire unlock-phase riders are pending mandatory effects
  (`Pending::Printed`, `Choice::ResolvePrinted`); `Stat::NoDirectedActions`
  filters the minion phase's offers (`Engine::directed_actor`); a combat an
  ability begins after resolution returns to that window
  (`CombatCtx::after_resolution`).

**Equipment** (`plays.rs`, `actions.rs`)
- Equip from hand: +1 stealth plus printed, cost at resolution (unpayable or no
  longer holdable: fizzle), NRA by name. From another own minion
  (`EquipFrom`): free, no requirement check, each taken card's equip stealth
  added, moves what is still there at resolution. One vehicle etc. via
  `GameState::can_hold`. `Engine::put_equipment` is shared by equip, searches
  and Vast Wealth; "only one played or equipped in a game" is recorded there.
- Bearer effects are read live on the current bearer (`Modifier::Bearer`), with
  the named trait (a non-Ravnos holding Treasured Samadji gets nothing,
  rules-qa 9.1).

**Replacement, hand size, draws, searches** (`plays.rs`, `draws.rs`)
- Hand size is derived (`GameState::hand_size`); cards not yet replaced count
  against it (`Seat.held`, `Until`, `Held`, `release_held`; the longest delay
  wins, spec §1.9.2). Held-until conditions include events' ("until a Methuselah
  is ousted", "as long as this card is in play": waiting on the condition, not
  the card). An oversize hand discards down before any other decision.
- Every card leaving hand that the rules replace goes through `Engine::replace`,
  offered first to `Hook::WouldReplace` riders; an empty library replaces nothing
  (and nothing burns instead, rules-qa 8.3). A hand-size increase only draws up
  (`draw_up`).
- Draw alternatives (`Hook::WouldDraw`): a choice per card drawn (rules-qa 9.4);
  beside Visit from the Capuchin a replacement is the Capuchin's counter or a
  stored card.
- Searches (`Flow::LibrarySearch`, `Interrupt::DisciplineSearch`, `SearchStore`,
  `SearchToHand`) pick by card, not position, at resolution (spec §1.14.1), and
  shuffle with the engine's `Rng`; an equipment search offers only what could
  be equipped as a play (no unpayable blood cost, rules-qa 9.3).

**Costs and reductions** (`plays.rs`)
- Reductions are announced with the card (`Flow::CostTerms`,
  `PlayCtx::discounts`, `CostSource`; no pass while unaffordable without them),
  locked at declaration (not if canceled as played) or at resolution for
  non-action cards; paid through `reduced_cost_with` / `spend_counters` (counters
  spent only as a cost is paid). Found or named cards take their reductions once
  chosen (`CostThen`, `pay_named_discounts`, `payable_with_potential`).
- Surcharges: `card_surcharge` (Discipline and event surcharges),
  `discipline_surcharge` (Libertas), `torpid_surcharge`, Haven's +1 on masters
  targeting its minion.
- Paying the last pool ousts: out of turn or from an ability, without passing the
  active turn (`pay_pool_for_ability`); a play whose effects already ousted its
  player does not oust again.

**Events** (`plays.rs`; `defs.rs` section "Events")
- A discard phase action, one per phase (`Seat::event_played`), each event once a
  game per player (`GameState::once_played`), controlled by its player.
  `Requirement::GehennaInPlay` is checked as played only (PIB 20121031);
  `Modifier::NoGehennaEvents` bars Gehenna plays; `Hook::GehennaPlayed`.
- Global layers read live: `capacity_ceiling` (Wormwood), `SectFallen`,
  `OnlyMustHunt`, `torpid_surcharge`. Timed effects: The Uncoiling's burn due in
  its controller's discard phase (`Seat::uncoiling_due`), Thirst as the minion
  phase ends, FBI's damage as the combat ends (`CombatCtx::fbi_damage`).

**Generated data** (`scripts/gen_crypt.py` → `crypt.rs`, `scripts/gen_library.py`
→ `library.rs`, both `--check`)
- Generated labels read from card text/type line: `Weapon` (incl. "Melee
  weapon."), `Electronic`, `Location` (type line only), `Discipline`, `Grapple`,
  `Aim`, `Title`, `RequiresAnarch`, `MakesAnarch`, `OperationAntigen`,
  `MultiDiscipline`; "Unique." after the first sentence; ally body (life,
  strength, bleed) and subtypes. `Title::Independent { name, votes }`. Printed
  justicars keep the clan's old name.

### 11.3 Open gaps

Still true today; "(verify)" marks items inferred from older notes.

- **Incomplete vampires (5)**: Daria (+1 blood on directed actions), Djeneba
  (opponent's strike cards +1), Marchesa Liliana (a pick-seven removal terms
  flow), Mithras (block/vote toll: `Stat::BlockToll` and
  `Stat::VoteAgainstToll` now exist), Ragnar Nordstrom (Protean discount:
  `Stat::BloodCostOff` now exists). Every other vampire is defined.
- **Inert clauses, no V5 tenant**: Blood Trade's boons; Fall of the Sabbat's
  Black Hand; The Fourth Cycle's conviction cards; "referendum passes
  automatically" (Amici Noctis, Voter Captivation rows blocked); first strike and
  self-damaging weapons (rows n/a).
- **Mechanisms missing** (ledger `blocked:` rows name the rest): a mage
  subtype (Sri Sansa's "mortal or mage ally": no mage in the pool, none
  generated); immunity to damage from a
  source; tournament withdrawal (§10.6); allies paying blood costs under The
  Shard (ANK 20230721, not modeled).
- **Known edges**:
  - `ImmuneToDirectedActions` is read on card plays only
    (`GameState::immune_to_directed` in `plays.rs`): a granted or printed
    action requiring a Discipline would need it in `ability_targets` too.
  - `Terms::XBlood` stops at the blood left after a fixed cost but not after a
    surcharge (Terror Frenzy on Lightning Reflexes offers one X too many,
    clamped at resolution). (verify)
  - Disarm's torpor is applied inside the card's resolution: a torpor rider
    ousting the active Methuselah there finishes the play's bookkeeping first.
  - True Love's Face's pay-to-cancel is not offered for a last pool (an oust in
    the middle of a play).
  - A canceled card whose cost "is still paid" (Meditative Grove) pays it in
    full: Regina Blake's announced blood is not used (`settle_canceled`).
  - `Effect::PoolToHostBlood` ousts through `oust_and_pass`, assuming the user is
    the active seat — true for Blood Doll; route through `oust` when a
    pool-paying untimed ability reaches a non-active seat.
  - A cancelling ability marks the play canceled as it is used
    (`as_played_apply`); one with terms the user could back out of would need
    the mark after its terms.
  - Self-contest of a unique title: Toreador Justicar's terms may choose a
    Toreador of a Methuselah already holding the title (should be masked for
    the caller's own, the incoming card burned for another's); Valerius stealing
    Prince of a city whose baron his Methuselah controls is the same case. (verify)
  - Directed actions name one Methuselah at most (`ActionCtx::directed_seats`);
    Shadow Cast's remembered-set test is written for several.
  - Rapid Change [PRO] and Wolf Claws [PRO] are covered by fuzz only.
- **P2 deliverables not started**: perft baselines (§7.5), golden replays (§7.3),
  observation-equivalence test of elision, protocol server (§8), scripted
  reference bots (aggro-bleed, wall, vote), the New Blood minis as decklist
  data.
- Owner suggestions: hard legacy cards to stress the engine (Rötschreck, Outside
  the Hourglass, Rewind Time) and legacy cards closing test loops (a sterile
  baron); more `EXTRA_CARDS` at the owner's call (§3).

### 11.4 What remains

- **P2 — full engine pool**: finish the 5 incomplete vampires and the
  deliverables above.
- **P3 — training loop**: observations (§5), PyO3 batched stepping, encoder, PPO
  self-play at 4p; evaluation ladder vs scripted bots and frozen snapshots.
- **P4 — league & 5p**: exploiters, PFSP, deck/seat sampling, evaluation vs
  humans.
- **P5 — community**: bot service, client adapter, human-ergonomics layer.

Definitions: 425, 882 tests

## Outlook: deck building (post-v1)

Once engine + pilot work, deck building is the natural extension: fast simulator
+ competent pilot = the deck-evaluation oracle prior deckbuilding AIs lacked;
pointer-style policy makes new decklists data; engine pool ⊃ training roster.

- **Formulation**: co-evolve a metagame, not "find the best deck" — population /
  quality-diversity search (archetype descriptors as diversity axes), fitness =
  expected tournament score vs the current population.
- **Pilot–deck chicken-and-egg**: unfamiliar archetypes score low through the
  pilot. Mitigations: decklist-conditioned policy (from v1), per-candidate
  fine-tuning, full co-evolution.
- **Assets**: the TWDA (~4k winning decklists via KRCG) as a generative prior;
  deck legality already solved in krcg.
- **Bridge study — precon tuning**: frozen pilot, ≤10 swaps from a precon,
  hill-climb; exercises the loop, measures pilot bias, and an improved precon is
  a community artifact. Can start once the P3/P4 bot is decent.

## Glossary

Terms the owner uses, as recorded in rules Q&A:

- **Combo**: a single line of Discipline icons, "[dom][pre] X": every one
  is needed (Iron Glare; KRCG `combo`).
- **Choice**: separate lines, "[pre] X / [tem] Y": one Discipline is
  chosen, at one level (Line Brawl). A card printing "more than one
  Discipline can be used" lets any set of them combine, each effect
  applying (Break the Bonds).
- **Outferior**: the inferior-only option of a card that a vampire can use
  without the card's bloodline Discipline (Outside the Hourglass; rules-qa
  25.2).

## Resolved questions (2026-08-02)

- **v1 training roster**: the 14 full-size precons (379 unique cards). New Blood
  minis and singles are implemented but not in the v1 deck mix.
- **Polling rotation order**: starts at the acting Methuselah, clockwise; closure
  on a full rotation of consecutive passes guarantees universal response
  opportunity to every commitment (§4.2). Confirmed.
- **Events**: implemented in v1 with the rest of the engine pool. Playing one is
  an optional discard-phase action (discard a card, *or* play an event; each event
  once per game per player) — nothing forces an event play, an event in hand can
  always simply be discarded. The general lesson absorbed into §4.4: the
  non-minion play windows (Master phase, Discard phase) are first-class offer
  windows, not afterthoughts.
- **Contest upkeep factoring**: each Methuselah resolves their side of a contest
  during their own unlock phase (self-contest impossible → no player ever orders
  both sides); per-object binary pay/yield decisions in canonical order; forced
  yields elided (§4.4).
- **Window census vs per-card reality**: draft matrix generated for all 421
  library cards, then hand-reviewed card by card (Claude) and arbitrated by the
  owner (rules-qa Batch 2, 2026-09-02); the review's conventions were fed back
  into the generator, which now converges with the reviewed matrix (§6).

## Process: rules Q&A pass

Design-level rules understanding is de-risked through a running Q&A with the
project owner (VEKN judge): batched questions organized by subsystem, answers
recorded in `docs/rules-qa.md`, each answer eventually backed by a scenario test
so the engine's behavior and the recorded ruling can never drift apart.
