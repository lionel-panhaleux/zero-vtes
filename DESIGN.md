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
ledger): Camera Phone, Laptop Computer, Direct Intervention, Wash (2026-09-25);
and legacy cards making tournament-winning decks (TWDA) playable, appended
after the V5 tables so existing indices stay put (`ADDED_CARDS`, 2026-09-28):
Ashur Tablets, Carlton Van Wyk, Donnybrook, Ennoia's Theater, Gilbert Duane
(G6), Hell-for-Leather, Jake Washington, Life Boon, Murmur of the False Will,
Powerbase: Savannah, Rötschreck, Saatet-ta.

- **637 unique cards**: 201 crypt (200 V5 + 1 added), 436 library (421 V5 + 4
  extra + 11 added).
  V5 library by type: Master 124, Combat 63, Action 59, Action Modifier 53,
  Reaction 40, Equipment 23, Event 20, Ally 19, Political Action 19, Retainer 12
  (multi-type cards counted per type).
- **Engine pool = the whole pool**, Events included (conditional delayed
  replacement, tokens, global capacity/sect layers, randomness, mandatory unlock
  decisions). Playing an event is an optional discard-phase action, so events
  stress the effects framework, not the decision protocol.
- **v1 training roster**: the 14 full-size precons (Fifth Edition 7 · Anarch 4 ·
  Companion 3), from KRCG `_set` bundle codes: 379 unique cards (138 crypt, 241
  library), 17–41 uniques per deck; since 2026-09-28 also the TWDA decks
  fully in the pool (`--decks precons,twda`; `Deck::twda`): 81 with the
  added legacy cards (28 without), 62 once near-copies are merged — 23
  Gangrel, 9 Malkavian, 8 Ministry, the rest 1–5 a clan, none Nosferatu,
  Toreador or Ventrue; a seat draws a clan uniformly, then a deck of it,
  and a deck sharing more than 20% of its crypt with one already at the
  table is redrawn (owner: crippling contests — as a dataset filter it
  would keep 11 decks, 1 with the precons, the TWDA reusing the precons'
  vampires). The New Blood mini-starters are decklist data
  (`Deck::mini`) outside the roster — half decks, not legal (owner): never
  trained on; the singles-only cards (Fall of
  London, Echoes of Gehenna, part of Shadows of Berlin — all the events) are
  implemented but in no decklist.
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
pass from every other seat (no hidden-hand information leaks) — tested by
`tests/elision.rs` (a hand-swap counterfactual, §7.2). A forced move (a lone
option that is not a pass) has no counterfactual: its elision is the shared
`apply` itself. Elision is what makes self-play viable (only live choices reach the
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

Built: `Engine::run`/`choose`, the legal view, observations (`obs.rs`), the
batched pool (`crates/py`), the first model and PPO loop (`train/`); §11.2
"Observations" and "Training" describe them. Planned: the omniscient critic,
decklist conditioning, the league.

- **Legal view** (`GameState::legal_view(viewer)`, a redacted `GameState`:
  `Hash + Eq`): what `viewer` may know — the state, every hidden card identity
  replaced by `HIDDEN` in place (zone sizes, counters, blood, pool and flags
  kept). Hidden: every seat's library and crypt, the viewer's own included
  (order unknown; a search shuffles; the cards Heart of Nizchetus puts at the
  bottom are hidden too — the policy remembers its own decisions); another
  seat's hand unless `hand_visible_to` (Revelations [AUS], Owl Companion);
  another seat's face-down uncontrolled vampires (rulebook: a Methuselah looks
  at their own), a banished vampire's slot excepted (it left play in view of
  all); cards stored face down (Shilmulo Tarot, `Effect::StoreTopCards`) except
  to the card's controller. Public: ash heaps, removed cards, minions and cards
  in play (contested cards keep their identity), counters, `held` counts. The
  rng is on `Engine`, not in the state. The action, combat and referendum
  contexts and the as-played stack are public (announced plays, targets,
  windows) except a card named from hand as a play or action is announced
  (`PlayCtx::named`, `ActionCtx::exchange`'s hand card), which need not be
  shown (advanced rules §1.14.2): the observation must redact it when it adds
  the contexts. Owner review points: the banished slot and contested cards kept
  public; a vampire revealed on its way to the uncontrolled region
  (`RevealCryptTop`) hidden like any other.
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

The engine's value is trust; trust is measured, not asserted. One test profile,
`ci` (release optimizations with debug assertions and overflow checks,
incremental): `cargo test --profile ci`, about 25 s for the full check after an
engine edit on an Apple M3.

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
identical state hashes (every 4th seed; the goldens pin the rest). A fixture
plays 100–300 seeds: about twice the seed where its last coverage key first
appears (the midgame's at 163; the opening plays 300); `FUZZ_SEEDS=n` sweeps
deeper. The cost is the engine itself (half of it `GameState::printed_mod` on
the late tables), not the harness.

`tests/elision.rs`: elision's observational equivalence (§4.3), from the opening
and three fuzz tables (`tests/common/tables.rs`), 100 seeds each. At every real
request to X whose options are one hand-independent option (a pass, the hand
strike) plus plays from hand, a clone gets X's hand swapped for as many copies
of a card X cannot play (a clan none of X's minions has) and must elide X's
window; the actual game takes that option, the clone follows its choices, and
until X's next real request the requests to others and `legal_view(Y)` for
every Y who cannot see X's hand (with the action, combat and referendum under
way) must be equal. A card of X's made public (Revelations looking at one
distinct card) ends the comparison. Coverage keys per window and role: blocks
and reactions, modifiers, strike, maneuver, press, polling, Sudden Reversal and
Direct Intervention out of turn, as-played cancels (~7 400 samples, ~26 000
compared steps).

### 7.3 Golden replays

`tests/golden/<fuzz test>.txt` pins every §7.2 game, checked in the same sweep
(no extra runs): per seed the real decisions, final turn, VPs and state hash,
and a checkpoint every 64 decisions — the low 32 bits of an FNV fold of each
decision (window name, seat, option count, choice) and of the state hash at the
checkpoint (~300 KB for 22 tests, 3 700 games). A mismatch names the test, the
first diverging seed (and how many differ) and the 64-decision range of its
first divergence, with the command tracing it (`FUZZ_TRACE=1 SEED_FROM=n`,
decisions numbered). An intended change re-blesses: `GOLDEN_BLESS=1 cargo test
--profile ci --test fuzz`; a missing file fails, a partial sweep (`SEED_FROM`)
skips the check. Identical in every profile; the state hash is stable on
64-bit little-endian targets (lengths hash as `usize`).

### 7.4 Coverage checklist

Every mechanic answers "where is it handled, where is it tested" — §11.2 is that
map. The in-play hook vocabulary was closed by enumerating the pool's in-play
abilities (a since-retired regex census); every hook has tenants (`Hook` index
in §11.2).

### 7.5 Perft-style counts

`tests/perft.rs`: the real-decision sequences to depth k (elided windows do not
branch; a game over is a leaf) from 7 positions — the opening, two random-play
midgames (canaries: any earlier change moves them), and scripted ones: an action
against modifiers and reactions, a blocked bleed's combat, a political action's
terms and polling, a master phase. Pinned with a per-window breakdown (decision
nodes, options offered) in `tests/golden/perft.txt`, so a legality regression
names its window; re-bless with `GOLDEN_BLESS=1 cargo test --profile ci --test
perft`. 0.2 s. Speed baseline (`perft_speed`, ignored,
depth +1, one `Engine` clone per step, one thread): ~0.6 M steps/s on an Apple
M3 (0.17 M on the later midgame/5).

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
   survivor; training and evaluation tables draw 60–90 per game (owner,
   2026-09-28; `Engine::time_limit`); VPs stored in half-points (`Seat.half_vps`); `GameState::game_win`
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
| P2 · 12b | 09-27 | The 5 partial vampires finished (Daria, Djeneba, Marchesa Liliana, Mithras, Ragnar Nordstrom): every vampire has all its text |
| P2 · 12c | 09-27 | Regression tooling: golden replays of every fuzz game (§7.3), perft counts (§7.5), `Engine: Clone` |
| P2 · 12d | 09-27 | Elision's observational equivalence (`tests/elision.rs`, §7.2), `GameState::legal_view` (§5); block pruning no longer reads third parties' hands |
| P2 · 13 | 09-27 | Decklists: the New Blood mini-starters as data, `Engine::with_decks` (`decks.rs`); scripted reference bots aggro-bleed and wall (`bots.rs`, `agent::Agent`) |
| P3 · 14a–14c | 09-27 | Observations (`obs.rs`: entity tokens, option rows with pointers; honest by test); `crates/py` `GamePool` (batched parallel stepping, bots inside, replayable); `train/`: pointer-scoring transformer, PPO self-play, evaluation vs random and bots |
| P4 · 16a | 09-28 | Pool extension for TWDA decks: 12 legacy cards appended (`ADDED_CARDS`), first strike, a master card becoming an ally, the before-resolution combat window, out-of-turn Rötschreck, shuffle-on-burn, Ashur Tablets' picks, Life Boon blocked |
| P4 · 15a–15c | 09-28 | Several networks at one table (`net:k` seats, `agent` column), matchmaking queued from Python, 4- and 5-seat tables only; the league: one learner seat, an exploiter, snapshots, bots, PFSP (`train/league.py`) |

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
`decks.rs` (decklists), `agent.rs` and `bots.rs` (policies),
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
  toll lands pays or resigns (`Interrupt::BlockToll`). A forced attempt
  (`can_attempt_block[_anyone]`, `AttemptBlock`, `TargetAttemptsBlock`) needs
  the toll payable (rules-qa 42.8).
- **Reachability pruning** (`intercept_reachable`): an attempt that cannot reach
  the stealth is not offered. It counts the blocker's own and printed intercept,
  intercept abilities of cards on it, other cards in play
  (`location_intercept_reach`: Market Square, KRCG News Radio, The Rumor Mill),
  and stops pruning while a non-acting Methuselah could strip the stealth
  (`stealth_strippable`): the blocker's own Methuselah by a playable card in
  hand, another by holding any card while one of their minions could play a
  stealth strip of the pool (Night Terrors) — their hand is hidden from the
  blocker (§4.3; reading it leaked, found by `tests/elision.rs`). Any new
  third-party intercept/stealth-strip card must be counted here, or pruning
  hides a live attempt.

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
  (rules-qa 13.1; none left); fixtures wanting a plain vampire pin one by name.
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
- `Stat::DirectedActionCost` (Daria, `Engine::directed_action_toll`): masked
  at announce with the action's own blood (`announced_blood`), paid first in
  `succeed` (unpayable: fizzle). `Stat::StrikeCardToll` (Djeneba,
  `strike_card_toll` inside `combat_card_surcharge`): on the opposing
  combatant's strike-mode cards (`plays::is_strike_mode`), an ally paying
  life. Liliana's seven cards: `Effect::RemoveOwnAshCards` →
  `Interrupt::RemoveFromAsh` on her own heap (one pick a step; burned
  vampires, `Seat::ash_crypt`, count and are offered, as for Shroud of Decay).
- Ability limits: "each combat / round" count per copy
  (`AbilitySource::use_key`, Kevlar Vest's branches share one use); "during
  an action" per copy and ability (`AbilitySource::action_key`, Bone
  Shambler's two, rules-qa 42.9).

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
  `discipline_surcharge` (Libertas), `combat_card_surcharge` (Terror Frenzy,
  Djeneba), `torpid_surcharge`, Haven's +1 on masters targeting its minion;
  Daria's on the action (`directed_action_toll`). An ally combatant burned by
  paying its last life for a card ends the combat (`pay_play_blood`).
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
- Pool order: the V5 cards (and `EXTRA_CARDS`) sorted by name, then the
  `ADDED_CARDS` by KRCG id (`window_matrix.pool_order`): an addition never
  moves an existing index. A legacy master's "Master: " type prefix is skipped
  for labels; "Out-of-turn. Frenzy." labels `Frenzy`.
- Decklists: `PRECONS`/`PRECON_LIBRARIES` (the 14 full-size precons) and
  `MINI_PRECONS`/`MINI_LIBRARIES` (the 14 New Blood mini-starters: 6 crypt
  cards, 48–49 library cards), each sorted by set then deck, joined by
  `decks::Deck` (`precon`, `mini`, `twda`, `all`). `TWDA` (`twda.rs`,
  `scripts/gen_twda.py` from KRCG twda.json and the generated tables'
  `id:` order): every tournament-winning deck whose cards are all in the
  pool, tournament-legal sizes (crypt 12+, library 60–90), near-copies
  (over 90% of the list shared) merged into the winner of the largest event,
  TWDA order; `Deck::crypt_share`, `Deck::clan` (most crypt copies);
  regenerated after a pool change, it picks up the decks the additions
  complete. `Engine::with_decks(seed, decks)`
  seats them in order; `Engine::new` draws random precons with the same random
  sequence as before (goldens unchanged).

**Agents and scripted bots** (`agent.rs`: `Agent`, `RandomLegal`, `play_out`;
`bots.rs`: `Scripted`, `Style`)
- `Agent::decide(&Engine, &DecisionRequest)`: a policy may read the engine, but
  only what its seat's legal view shows. `tests/bots.rs` answers every 3rd
  decision of bot tables again from `legal_view` (a bot reads the action's
  `seat`, `acting`, `kind`, `bleed_target`, `directed_seats`, the referendum's
  `caller`; never `ActionCtx::card`/`exchange`, which name a card from hand and
  are not redacted).
- The bots score the options of the windows that define a style; everything
  else scores 0 and ties are broken by a seeded random draw ("random with a
  spine": the difference from `RandomLegal` is exactly these rules).
  Deck-agnostic. Common to all styles: master cards played rather than passing;
  contests paid; influence brings out every funded vampire, transfers onto the
  slot nearest funding while under a vampire target (3; wall 4) and above a pool
  floor (6; wall 8), draws from the crypt with at most one uncontrolled vampire
  left, never withdraws; hunt when empty or under a third of capacity (wall:
  half); combat cards that strike or prevent over other combat cards over the
  hand strike; the discard phase discards a card no controlled minion could
  play; own referendums voted for (the Edge and political cards burned for
  them, vote cards played on the side they add to), everyone else's against;
  political actions taken (over a bleed of 1 for aggro-bleed). Referendum
  terms by what the referendum does: damage (pool burned by allocation or
  choice, a victim of Parity Shift) at the prey when it may be named, the
  predator otherwise; gains (Parity Shift's points, Camarilla's Iron Fist's
  chosen Methuselah, Disputed Territory's taker) to the caller; minions to
  banish, expel or lock among the prey's and the predator's at random; title
  cards on the caller's own; the clan with the most of the caller's vampires
  (owner, 2026-09-27). A failed block
  attempt is not repeated with the same minion in the same action (the rulebook
  allows it as often as wished: an always-block policy loops, so the bot
  remembers its attempts).
- Aggro-bleed: bleeds with every minion (the biggest bleed first), plays bleed
  modifiers, blocks only actions directed at it. Wall: keeps one ready unlocked
  minion home (other actions only with two or more), blocks everything, plays
  intercept. No vote style: no style is assigned to a precon, and political
  actions are part of both (owner, 2026-09-27).
- Measured 2026-09-27 (precons rotated). Random play alone (200 games): 94.7
  turns a game, 1.58 ousts, 75% of games at the time limit. Against three
  random seats (200 games per style): aggro-bleed 94% game wins and 3.75 VPs a
  game, wall 83% and 3.29, the random seats 0.08–0.21. Two of each style at
  one table (480 games): aggro-bleed 1.51 VPs a seat, wall 0.49; 45 turns a
  game, none at the time limit; 2–5 ms a game.

**Legacy additions** (slice 16a; `defs.rs` section "Legacy cards for
tournament decks", `tests/legacy.rs`; rules-qa Batch 44)
- First strike (`Effect::FirstStrike`, `Side::first_strike`): resolved in
  `resolve_strikes` after strikes: combat ends and before the others; the
  pool's only one burns an equipment or retainer (Donnybrook [obf]), so a
  weapon burned so loses its strike; a dodge still protects.
- Before-resolution combat window (`Window::CombatBeforeResolution`): cards
  triggered by the chosen opposing strike (Rötschreck, with
  `CombatCtx::aggravated_strike_on`). Rötschreck ends combat at once
  (`end_combat_now`), the lock and torpor landing as the combat ends
  (`CombatCtx::rotschreck`), dropped if the combat continues.
- `Effect::BecomesAlly(body)`: a master card in play as its ally (Jake
  Washington), not recruited; `Effect::BloodToTargetX` with `Terms::Count`.
- `Hook::SelfBurned` with `ShuffleSourceIntoLibrary`: a burned ally's card
  shuffled into its owner's library instead of the ash heap (Saatet-ta).
- `Effect::AshurTablets`: three copies controlled as one enters play remove
  every copy (`remove_card_in_play`), 3 pool, then `Interrupt::AshurPicks`
  (hand first, then up to 12 more to the library, shuffled);
  `Requirement::NotPlayedThisTurn` (`GameState::turn_played`).
- `Effect::GainTransfers` (Ennoia's Theater), `Effect::RemoveCryptTopAnarch`
  with `Interrupt::BecomesAnarch` (Gilbert Duane), `Modifier::
  HostInterceptVsVampires` and `Minion::diablerie_turn` against
  `Seat::previous_turn` (Carlton Van Wyk), `Effect::OpposingControllerBurnsPool`
  at the end of the round (Hell-for-Leather [obt]; ousts via
  `combat_ousts`), `Requirement::ActingCombatantYourVampire` with
  `Effect::LockTargetCard` (Powerbase: Savannah).
- `Window::Ousting` (Life Boon) never opens: ousts apply where the pool runs
  out (§11.3).

**Observations** (`obs.rs`: `encode`, `Obs`, `schema_json`)
- One request as its seat sees it: tokens — seats in relative order (pool,
  VPs, Edge, zone sizes, prey/predator, phase budgets), minions in play
  (contested ones too), the viewer's virtual hand (`Choice::Play` indexing),
  cards in play (host as a pointer), uncontrolled slots (the viewer's face up,
  others' `HIDDEN` but a banished one), a last context token (window; action
  kind, acting minion, bleed target, stealth, bleed amount; combat minions,
  range, round; referendum caller, calling minion or diablerist). Each token:
  ids `[type, vocabulary, pointer, pointer]` and `TOK_F` floats. Options:
  ids `[choice kind, vocabulary, pointer ×3]` (hand card, actor, target; the
  source of an ability; the minion, seat, card or slot named) and `OPT_F`
  floats (amount, favor, mode, zone, title contest, action kind). A card or
  minion an option names carries its identity too (a rider's card may have
  left play). Vocabulary: crypt, library, hidden, windows, clans.
- `tests/obs.rs`: the encoding of the full state equals the encoding from
  the seat's legal view (every 2nd decision of bot, random and fuzz-table
  games; a planted leak of face-down vampires fails it), and every pointer
  names a token. At most 63 tokens and 104 options met.
- Review points: ash heaps, removed cards and cards stored on cards are sizes
  or absent, not tokens; no decklist identity yet; `ActionCtx::card` (a card
  named from hand) is never read, by construction only.

**Training** (`crates/py`: `zvtes_py.GamePool`, `replay`, `schema`,
`deck_names`, `deck_families`, `vocab_keys`; `train/`: `model.py`,
`ppo.py`, `league.py`)
- `GamePool(n, seats, seed, rotate, decks, budget, keep_logs, tables,
  time_limits)`: N games,
  each waiting on one network's decision; seats are "net:k" (a network
  answered from Python, 0 the learner; "policy" = "net:0"), "random",
  "aggro" or "wall" (answered inside); `rotate` shifts seating per game;
  decks drawn per seat from `decks` (default the 14 precons). `queue(specs)`
  replaces the tables new games take (seats, optional decks; 4 or 5 seats: tournament tables only),
  the default table once it runs out. `observe()` pads the batch (with an
  `agent` column: the network to ask); `act(actions)` advances every game in parallel (rayon) to
  its next policy decision and returns the games finished (slot, game
  number in start order, seed, time limit, decks, seats, half-VPs, turns,
  decisions, truncated, log). Each game draws its time limit in
  `time_limits` (60–90 player turns inclusive, owner 2026-09-28: player
  turns, so a table losing a Methuselah early plays more rounds;
  `Engine::time_limit`, default `TURN_LIMIT` for the engine's own tests):
  0.5 VP per survivor, counting toward the game win (2 VP + 0.5 wins with a
  strict lead). `replay(seed, decks, log, time_limit)`. A game over
  `budget` decisions is cut (§11.3).
- Vocabulary keys (`obs::vocab_keys`: `crypt:<KRCG id>`,
  `library:<KRCG id>`, `window:<k>`, `clan:<name>`): checkpoints and
  snapshot directories (`vocab.json`) record them; loading under another
  vocabulary moves each embedding row to its key's new id, new keys
  initialized fresh (`model.remap_vocab`; the optimizer's moments are
  dropped then) — a pool extension keeps trained networks. `train/tests`: batches well formed,
  games replay from their log, only 4- and 5-seat tables, queued tables route rows to their networks, one PPO iteration and two league iterations run (the exploiter trains; resume keeps the beat rates), beat rates move once per iteration and retire the random seat and the exploiter; vocabulary keys match the schema and embeddings follow their keys; deck families (minis refused for training); TWDA tables train; evaluation seatings cover every opponent order; games replay with their drawn time limit.
- ~170k decisions/s (300 random games/s) through Python on 8 cores; the
  model is the bottleneck: a d=96, 2-layer trunk takes ~0.3 s per 1,024
  decisions for a training step on the M3's GPU (MPS; half precision
  measured slower, CPU slower): an iteration of 16k decisions, 3 epochs, is
  ~2 s of rollout and ~12–15 s of update.
- Model: token embedding (vocabulary + type + features + its pointers'
  bases), transformer trunk; each option scored from its kind, identity,
  features and the tokens it points at, with the context token; value from
  the context and mean tokens. PPO: without the league one network plays
  every seat of 4- and 5-seat tables; a trained seat's decisions form its
  chain, rewarded at the end by (VPs + `gw_weight` × game win) / 5
  (`gw_weight` 2 since the second league run: tournament ranking puts game
  wins first; 1 before); entropy bonus 0.02 (0.01 before; 0.03 made the entropy climb, 0.62 → 0.89 in 30 iterations, with every evaluation flat or down); truncated
  rollouts bootstrap from a chain's last decision, carried to the next
  rollout. Evaluation samples the learner at every seat and, for each,
  every distinct order of the opponents around it (`eval_tables`; before
  2026-09-28 the order was fixed and only rotated, so at 4 seats the wall
  always sat across the table: the starting checkpoint measured 0.60 VP /
  12.5% game wins against aggro/wall/aggro that way, 1.09 / 24% with the
  orders mixed — the earlier "plateau" was largely the seating), and counts
  exactly the queued games (not the first to finish, which favored short
  games): `vs_random` (four random seats; three before the second league
  run), `vs_bots4` (aggro/wall/aggro, the earlier `vs_bots`), `vs_bots5`
  (aggro/wall/aggro/wall), `vs_snapshot` (four copies of the latest
  snapshot), 256 games each (96 before: one snapshot measured 0.44–0.76 VP
  against aggro/wall/aggro over six evaluation seeds). A pool game that
  reaches an empty request panics with its seed, decks and log (the
  engine's own check is a debug assertion) — how training found rules-qa
  43.1. Game-win benchmarks at a 5-seat table (owner): over 16% is
  already very good, over 30% very strong, close to 50% top level.
- The league (`--league true`, `train/league.py`; revised after the first
  league run, owner 2026-09-28): each new table is 4 or 5 seats, one the
  learner's (`net:0`), with `--exploiter true` one the exploiter's
  (`net:30000`; off by default since the second league run), the others
  drawn by PFSP among the bots and the last `opponents_kept` (4)
  snapshots of the learner (saved every `snapshot_every` (10) iterations)
  and of retired exploiters (`net:30000+<it>`), in
  `runs/<name>/snapshots/`: weight (1 − p)² + 0.02, p the learner's
  pairwise beat rate against it (more VPs than that seat, a tie ½; from ½,
  moved once per iteration by `pfsp_rate` (0.1) toward that iteration's
  mean; truncated games not counted). The random seat retires once p
  passes `retire_random` (0.85). The exploiter is a second trained network
  (own optimizer and chains) that plays only the learner's tables; it is
  frozen into the pool and restarted from the learner's weights once the
  learner's beat rate against it falls under `exploiter_beaten` (0.3), or
  after `exploiter_reset` (50) iterations. One forward pass per network
  present. The pool refuses tables other than 4 or 5 seats. 512 games in
  parallel (256 before the third league run: one learner seat a table
  halved the samples). The learning rate tapers linearly from the run's
  first iteration to `lr_final` (0.1) × `lr` at the last; the evaluation
  with the most VPs against aggro/wall/aggro is kept as `best.pt` (a
  checkpoint like `model.pt`, resumable). Not built:
  evaluation against humans (needs the protocol server, §8).
- First run (2026-09-27, defaults, 140 iterations ≈ 2.3M decisions, ~1 h on
  the M3): against three random seats 0.71 VP a game at iteration 10 (random
  level: 0.73), 1.86 at 40, 2.7 at 50, then ~2.9–3.1 VPs and 72–77% game wins
  from 60 on; against aggro/wall/aggro 0.07–0.17 VP up to 50, then 0.4–0.7
  (an even share: ~0.75) and 5–12% game wins — a plateau from about 90 on.
  Self-play alone does not reach the bots: the league (bots and snapshots as
  opponents, P4) is the next lever.
- First league run (2026-09-28, defaults, resumed from the first run's
  iteration 90, to 200 ≈ 1.4M more decisions, ~1 h): against aggro/wall/aggro
  0.66 VP at 100, then 0.8–1.03 from 110 to 180 (15–20% game wins, above an
  even share), falling back to 0.77 at 190 and 0.58 at 200 as the entropy
  dropped from ~0.4 to 0.27; against random seats a steady climb to 3.57 VP
  and 90% game wins at 190; against the latest snapshot 0.9–1.04, 0.78–0.83
  at the end. The league lifts the bot result off the self-play plateau, but
  the late decline reads as the policy sharpening against random and
  self-play tables. The PFSP beat rates are noisy: with dozens of games per
  opponent per iteration, the 0.05 EMA spans ~20 games (aggro swung 0.67 →
  0.27 → 0.58 in 20 iterations).
- Second league run (2026-09-28, the revised league — one learner seat,
  the exploiter, per-iteration beat rates, `gw_weight` 2, entropy 0.02 —
  resumed from the same iteration 90, to 200, ~1.5 h; 256-game
  evaluations). The start (iteration 90) measured the same way:
  aggro/wall/aggro 0.60 VP, 12.5% game wins; aggro/wall/aggro/wall 1.90,
  41%; four random seats 3.92, 79%. Against aggro/wall/aggro: 0.53–0.82 up
  to 140, then 0.86, 0.89, 0.85, 0.85 (15–19% game wins) from 150 to 180,
  back to 0.75 at 190 and 0.61 (10.5%) at 200 as the entropy fell from
  ~0.68 to 0.59; aggro/wall/aggro/wall peaked at 2.07 (44.5%) at 150 and
  ended at 1.57 (32%); random seats 3.8–4.1 (79–84%) throughout. Same
  shape as the first league run: a mid-run gain lost over the last 20
  iterations. The exploiter never led the learner (beat rates 0.49–0.53)
  and was retired twice by the 50-iteration clock: with the learner's
  reward and weights it has no reason to find anything the learner does
  not. The learner's snapshots (`net:150`–`net:180`) are the best models
  so far. Each learner seat now yields ~6k samples an iteration (half the
  first league run's).
- Third league run (2026-09-28: 512 games, the learning rate tapering to
  0.1×, `best.pt`, no exploiter; from iteration 90 to 200). On its own
  (fixed-seating) evaluation, aggro/wall/aggro rose from 0.57 to a best of
  0.86 VP at 180 and ended at 0.81 — no late collapse this time. Re-measured
  on the fair evaluation (every opponent order, 60–90-turn time limits,
  precons, 256 games): the start (iteration 90) 1.05 VP / 25% game wins
  against aggro/wall/aggro and 1.28 / 26% against aggro/wall/aggro/wall;
  the second run's 160 1.06 / 23% and 1.47 / 27%; this run's best 1.11 /
  23% and 1.33 / 25%, its last 1.07 / 23% and 1.21 / 20%. The league runs
  did not improve on the self-play checkpoint against the bots: all sit near
  an even share at 4 seats (~25% game wins) and a little above it at 5
  (~20%) — their measured "gains" were the seating's noise.

### 11.3 Open gaps

Still true today; "(verify)" marks items inferred from older notes.

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
  - A canceled card whose cost "is still paid" (Meditative Grove) takes the
    player's printed reductions but not Regina Blake's announced blood
    (`settle_canceled`).
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
  - Decisions are unbounded within an action: `TURN_LIMIT` bounds turns, but
    the rulebook allows block re-attempts as often as wished, so a policy
    that always declares a block never ends the action. A driver with a
    deterministic policy (the P3 greedy evaluation, a human client) needs a
    decision budget; the scripted bots remember their attempts.
- **Life Boon** is defined but never offered: saving a Methuselah from an oust
  needs a decision point before the oust, and ousts apply at once where the
  pool runs out (ledger rows `blocked:`).
- Legacy additions partly covered by tests only through code paths: a
  Rötschreck range check (a strike ineffective at the range), Powerbase:
  Savannah with Deep Song's acting target, Saatet-ta's stealth and intercept
  locks, Carlton's strike and dodge.
- **P2 deliverable not started**: the protocol server (§8).
- Owner suggestions: hard legacy cards to stress the engine (Rötschreck, Outside
  the Hourglass, Rewind Time) and legacy cards closing test loops (a sterile
  baron); more `EXTRA_CARDS` / `ADDED_CARDS` at the owner's call (§3); the next
  TWDA unlocks by deck count (after the 12 of slice 16a).

### 11.4 What remains

- **P2 — full engine pool**: the deliverable above.
- **P3 — training loop**: built (slice 14); remaining: training long enough
  to beat the bots, the omniscient critic, decklist conditioning, frozen
  snapshots on the ladder, the early survival shaping of §5 (the reward is
  tournament scoring only).
- **P4 — league & 5p**: the league, PFSP, 4- and 5-seat tables and seat
  sampling, the exploiter built (slice 15); remaining: evaluation vs humans
  (with the protocol server).
- **P5 — community**: bot service, client adapter, human-ergonomics layer.

Definitions: 436 library, 102 vampires; 918 Rust tests, 12 Python tests

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
