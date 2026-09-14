# zero-vtes — Design Document

A rules-enforcing VTES engine and a self-play deep-RL agent for the V5 card pool.

This document records the design decisions discussed and settled before implementation.
Every structural claim traces to a source: the [VEKN rulebook](https://www.vekn.net/rulebook),
[vtes-advanced-rules](https://github.com/vtes-biased/vtes-advanced-rules) (judge-level semantics),
the [play procedures draft](https://gist.github.com/lionel-panhaleux/766eedd27ff26ae30169508ddc6bf95c)
(shortcuts, declarations, free-for-all mechanics), the
[jol-rebuild](https://github.com/VTES-JOL/jol-rebuild) timing-window taxonomy (studied, not reused),
and card data from [KRCG](https://static.krcg.org/data/vtes.json).

---

## 1. Vision

Build, in order:

1. **A fast, rules-enforcing V5 game engine** (Rust) — the first of its kind. A community
   artifact in its own right: deterministic, replayable, exposing a serializable
   decision protocol any client can drive.
2. **A self-play trained agent** (PyTorch, AlphaStar-style scaled down) that plays
   legal, competent, eventually strong VTES on 4–5 player tables with the V5
   preconstructed decks.
3. **A pluggable bot**: the trained policy behind the engine-as-rules-oracle, usable
   by the community as a training partner in online play.

Explicit non-goals (v1): open deck building, negotiation/table talk (the agent plays
"silent VTES" — signals only through actions and votes), legacy card pool (the
architecture must not preclude it; the implementation does not attempt it).

## 2. Prior art — what we take and what we reject

| Source | Take | Reject |
|---|---|---|
| **AlphaZero** | Self-play + policy/value net spirit | MCTS on true state (hidden info), two-player zero-sum assumptions |
| **AlphaStar** | Entity-transformer encoding, auto-regressive factored actions with masking, asymmetric critic (omniscient value net, legal-view policy), league training (main + exploiters + snapshots, PFSP opponent sampling) | Imitation bootstrap (no VTES replay corpus exists), TPU-scale compute |
| **Pluribus** | License to proceed without multiplayer equilibrium guarantees; fixed non-adapting strategy stance; cheap-compute mindset | CFR/blueprint method (needs short horizons and information abstraction VTES doesn't have) |
| **jol-rebuild** | Window taxonomy + anchor-card method, mechanics coverage checklist, convergent validation of impulse/sequencing primitives | Client-first architecture, legality-as-guards (vs enumeration), permissive/enforced dual mode, card-text parsing for behavior |

Key structural advantages over the games above: turn-based, engine-enumerable legal
actions, low per-decision branching. Key structural burden: rules complexity — the
engine is ~70% of the project.

## 3. Scope: the V5 pool

Pool = union of official V5-era products (per KRCG set data):
Fifth Edition, Fifth Edition (Anarch), Fifth Edition (Companion), New Blood,
New Blood II, New Blood III, Fall of London, Shadows of Berlin, Echoes of Gehenna.

Measured from `vtes.json` (2026-08):

- **621 unique cards**: 200 crypt (all vampires), 421 library.
- Library by type: Master 124, Combat 63, Action 59, Action Modifier 53, Reaction 40,
  Equipment 23, Event 20, Ally 19, Political Action 19, Retainer 12 (multi-type cards
  counted once per type).
- Decklists: the official preconstructed decks from these products (fixed, public).

**Engine pool vs training roster.** The engine implements the **full 621-card pool
in v1** — including all 20 Events and their machinery (conditional delayed
replacement "do not replace until X", token entities, global capacity-cap and
sect-override layers, randomness beyond shuffles, mandatory per-minion unlock
decisions). Events are less special than they look: playing one is an optional
discard-phase action (discard a card, *or* play an event; each event once per game
per player), so nothing about them distorts the decision protocol — they simply
stress the effects framework, which must express them before it freezes.

The **v1 training roster** is the 14 full-size preconstructed decks
(Fifth Edition 7 · Anarch 4 · Companion 3), reconstructible from KRCG `_set`
bundle codes: a union of **379 unique cards** (138 crypt, 241 library), 17–41
uniques per deck. The 14 New Blood mini-starters and the 67 singles-only cards
(Fall of London, Echoes of Gehenna, 7 of Shadows of Berlin — all events among
them) are implemented but appear in no v1 decklist; they enter play via community
constructed decks and later training phases.

Consequences probed against actual card text (see §6 for the window census):

- **In scope**: referendums with interactive terms, bleed redirection, wakes,
  action-cancel-as-announced, card-cancel-as-played, out-of-turn masters
  (Archon Investigation interrupts mid-action on a bleed-amount threshold),
  action continuation (Form of Mist), combat-end replacement (Telepathic Tracking),
  additional strikes, presses, prevention, Events (global ongoing effects),
  diablerie + blood hunt (basic rulebook action on torpor vampires — card-independent),
  contested cards and titles, Anarch mechanics.
- **Deferred — zero V5 tenants** (hooks designed, windows never opened in v1):
  torpor-replacement (`WOULD_GO_TO_TORPOR`, Amaranth-style combat diablerie),
  burn-replacement (`WOULD_BE_BURNED`), bidding free-for-alls.

The set list is configuration, not code: extending toward legacy means adding cards
and enabling deferred windows, not changing the framework.

## 4. Engine architecture

### 4.1 The core inversion: decision requests

The engine is a resumable state machine. It advances deterministically until some
player must choose, then emits a **decision request**:

```
DecisionRequest {
  player,             // who must answer
  kind,               // e.g. ChooseAction, BlockOrPass, CastVotes, ChooseStrike…
  options: [Option],  // enumerated legal choices, each with feature data
  observation,        // requester's legal view of the game state
}
```

The answer is an index into `options`. This is the entire agent interface for every
choice in the game — for the neural net, scripted bots, the fuzzer, and human clients.

**Factoring rule**: combinatorial choices are decomposed into unit choices
(allocate 1 pool at a time, pick one target at a time, confirm terms stepwise).
No decision ever enumerates a product space. This matches the auto-regressive policy
head *and* the tournament declaration protocol (play procedures §X.1: announce
discipline level, X cost, targets, amounts — our decision sequence is that protocol
at machine speed).

**Determinism & replay**: a game is fully determined by `(seed, [answers])`. The
decision log is the replay format: compact, resimulable, bisectable, and a future
imitation-learning corpus. Rewind = truncate log + resimulate (this is also the
mechanism behind human-facing shortcut interrupts, §8).

### 4.2 Two window primitives

Everything with timing in VTES instantiates one of exactly two primitives
(plus deterministic auto-steps):

**Ordered window (impulse/sequencing)** — rulebook Sequencing: offers rotate in a
defined order (acting Methuselah first; directed → defender first; undirected →
prey, predator; then clockwise). Any play returns the impulse to the acting
Methuselah. Closes when all pass consecutively. Parameterized by:
- ordering (who is offered, in what sequence),
- legality filter (which effects are playable: reactions+modifiers during actions,
  combat cards at a given step, wakes+cancels only during as-played windows, …).
  The as-played window (rules-qa 2.4) admits *exactly* wakes and cancel-as-played
  effects: no unlock reactions, no in-play card abilities (locking The Barrens
  to discard is not legal there). During an action it opens after every card
  play — a wake played there resolves before the played card's effect (e.g.
  before Consign to Oblivion raises the wake's cost) — so under elision it is
  non-empty whenever any Methuselah holds a wake for a locked minion. Wakes are
  action-scoped; master-phase as-played windows admit cancels only, pending
  rules-qa 1.5.

**Unordered commitment window (polling; bidding when it returns)** — no priority
holder, no reset. Repeated rotations; on each offer a player may commit an
**irrevocable** contribution or pass **without forfeiting** — they may still commit
on a later rotation. The atomic unit is the **source** (rules-qa 1.1): a source is
a vampire or a Methuselah (Edge and card-discard ballots are Methuselah-sourced;
card effects vary — most add to a vampire's votes, some grant Methuselah votes); a
source casts all its votes at once, all agreeing, and is then spent for the
referendum. Sources are independent: they may be committed on different rotations,
and a Methuselah's own ballots (Edge, each discard, each Methuselah-vote effect)
need not agree with each other. Closes when a full rotation passes consecutively. Rotations start at the
acting Methuselah and proceed clockwise: this matches table practice (the caller
typically opens — "N in favour, anyone opposed?") and the play-procedures rule that
the acting player controls the pace. Because closure requires a *full* rotation of
consecutive passes, every commitment reopens a response opportunity for every other
player before tally — nobody can be locked out of reacting to the latest vote.

They **interleave**: during the polling step, casting votes uses the unordered
window, but *playing cards and effects* stays ordered (normal sequencing).
Witness in pool: **Scalpel Tongue** — an ordered card play during polling whose
legal targets are "vampires who *have cast* votes in this referendum". Therefore:
- per-vampire vote-cast state is first-class, queryable by card predicates;
- "referendum passing automatically" is engine-visible state (Scalpel Tongue ruling:
  unusable then);
- committing early vs late is strategically live, so serialization is a modeling
  decision we make explicitly: the rotation scheme above preserves the option to
  wait, at the cost of a positional artifact within each rotation. In tabletop play
  vote order has near-zero impact in the common case; we accept the artifact and
  document it (§10).

### 4.3 Window elision — the formal analog of human shortcuts

Human play survives VTES's window density via binding shortcuts (play procedures
§X.2). The simulator is omniscient, so it can do the same *provably*:

> **Elision rule**: a decision request is emitted only if the player has at least
> one legal option besides pass. Otherwise the engine auto-passes. Forced moves
> (single mandatory option) are auto-played.

Soundness condition: an auto-pass must be **observationally identical** to a chosen
pass from every other seat's perspective (opponents see "passed" either way; no
information about hidden hands leaks into the observation stream). This must hold as
an invariant, and be tested.

This is not an optimization garnish — it is what makes self-play viable. Without it,
a 4-player game is tens of thousands of decision points, each a network inference;
with it, only the live choices remain (est. >90% reduction), and RL credit
assignment shortens accordingly.

The play-procedures document also defines the *human-facing* shortcut protocol
(binding shortcuts, interrupt-and-rewind, preemptive impulse requests = MTGO-style
"stops"). That protocol is a client-layer concern implemented on top of the decision
log (§8); the training path never sees it.

### 4.4 Workflow state machines

The rulebook's own structures are taken literally as nested state machines. Each
state either performs deterministic bookkeeping or hosts a window instance:

- **Turn**: Unlock → Master → Minion → Influence → Discard, with phase-scoped
  budgets (MPA, transfers ramp 1/2/3/4, discard actions) and unlock-phase
  sub-steps (contest/title upkeep and yields, Edge pool option, burn-option
  discards, "during unlock" effects). The non-minion play windows are first-class
  hosts of the same primitives: the Master phase is an offer window (master card
  plays, master-phase-action effects, cancel-as-played sub-windows, out-of-turn
  interactions) and the Discard phase likewise (discard-or-event as the discard
  phase action, once-per-game event tracking per player).
  **Contest upkeep factoring**: each Methuselah resolves *their side* of each
  contest during *their own* unlock phase — self-contest is impossible, so no
  player ever orders both sides of a contest, and no cross-player simultaneity
  exists. Iterate the unlocking player's contested objects in canonical arena
  order; each is a binary unit decision — pay upkeep (1 pool for a contested card,
  1 blood for a contesting titled vampire) or yield. Forced yields (no resource;
  vampire in torpor) are auto-played under the elision rule; "gains another title
  while contesting → immediately yields the contested one" is deterministic
  bookkeeping. Two broke Princes resolve naturally: whoever's unlock phase comes
  first is force-yielded (elided), leaving the survivor sole claimant, titled
  again once uncontested. Cards yielded burn with everything on them; a card whose
  rivals all yielded unlocks face-up on its controller's next unlock phase.
- **Action** (rulebook "Detailed action flow"; block-loop semantics per rules-qa
  1.3): declare (lock acting minion, announce card+choices) → as-announced window
  → block loop → resolution (success: pay cost, apply effects) or block resolution
  (blocker locks + combat; the after-combat window — Cats' Guidance, Forced
  Vigilance [for] — opens once, after *all* combats queued by the action, per
  rules-qa 2.1) → after-resolution window (Freak Drive) → possible
  **action continuation** (Form of Mist: re-enter the block loop with preserved
  stealth/intercept context).
  **Block loop**: one ordered window, not a success/failure checkpoint. A declared
  attempt has no stealth ≥ intercept test of its own — it stays pending until the
  window closes on all-pass-consecutively, with any play voiding prior passes
  (impulse-reset), so third parties can still feed intercept (KRCG News Radio) or
  strip stealth (Draba, Night Terrors) after the attempting Methuselah passes. At
  closure: intercept ≥ stealth → block resolves (blocker locks, combat); otherwise
  the attempt fails and the loop returns to "no attempt" (rulebook B.4), where
  the same Methuselah may attempt again, with the same or another minion. A
  Methuselah's pass with **no ongoing attempt is a final decline** — nothing reopens it except a change of the
  action's target (bounces: Deflection, Telepathic Misdirection; *not* the card
  "Change of Target", which terminates the action). Consequence for elision and
  ordering: a would-be third-party helper needs the defender to *attempt first* —
  impulse only reaches them after the target passes, and a target passing without
  an attempt has declined for good.
  **NRA (no-repeat-action), per rules-qa 1.4**: enforced in legal-action
  generation. Lock triggers on *resolution* (blocked counts; canceled-as-announced
  does not). Identity: card **name** per minion for played cards; card **copy**
  for cards in play (two Anarch Revolts = two distinct actions). Special caps
  independent of cards: one bleed and one political action per minion per turn;
  other rulebook actions (hunt, leave/rescue from torpor, equip…) unrestricted.
  Card texts like "cannot take the same action" extend to the exempt rulebook
  actions too.
- **Combat**: rounds of {before-range → determine range (alternating maneuvers) →
  before-strikes → strike declaration (acting first; maneuver-from-strike-card
  commits the strike) → strike resolution (simultaneous pairs; first strike;
  prevention windows inside damage resolution) → additional strikes → press step →
  end-of-round} with would-end replacement (Telepathic Tracking) and about-to-end
  queueing hooks; combat ends immediately when a combatant is not ready.
  Per rules-qa 1.2: within a step, impulse-reset applies (any play returns first
  opportunity to the acting side; step closes on consecutive declines). "Acting
  minion" = the minion performing the action, blocked or rushed alike; the other
  combatant is a "blocker" only if it actually blocked. Combats with **no acting
  minion** exist (Taunt the Caged Beast, Siren's Lure, the Slave rule): first
  opportunity then goes to the acting Methuselah's minion, else clockwise from
  the acting Methuselah.
- **Referendum**: terms confirmation (stepwise, factored) → polling (unordered
  votes + card plays as commitments of the same rotation; "before votes and
  ballots are cast" is a requirement inside polling, not a window) → tally →
  the action's after-resolution window.
- **Diablerie / blood hunt**: diablerie sequence owned by one workflow regardless of
  entry path (v1 entry: the basic action on torpor vampires); blood hunt as a
  restricted referendum (vote sources only, no modifier/reaction windows).

Window inventory: start from the census in §6; names are specific
(`ACTION_AS_ANNOUNCED`, not "interrupt") — one name per legality filter.

### 4.5 Cards and effects: one representation, two derivations

The framework rule that everything else depends on:

> **Playability and execution derive from the same card representation.**
> There is no hand-written `can_play` separate from `execute`. The legal-action
> mask is *generated*, and therefore cannot drift from behavior.

- **Static data** (name, types, cost, disciplines, clan, capacity, group, title,
  requirements, burn option) is generated from KRCG data — never hand-typed.
- **Behavior** is a composition of effect primitives from a Rust library
  (pay-cost, require-discipline-level, add-stealth, bleed-modifier, inflict-damage,
  prevent, add-votes, put-counter, search, …) plus declarative playability:
  window(s), actor constraints, target predicates (e.g. "vampire who has cast votes
  this referendum"), once-per tracking (per action / combat / round / turn /
  referendum — play procedures and rulebook wording templates §1.2 of
  vtes-advanced-rules map nearly 1:1 onto these primitives).
- **Mask policy** (owner, rules-qa 4.3): the mask removes only what the rules
  or the card forbid — alternation, per-scope limits, "only when needed",
  "cannot be played with no damage to prevent". A legal play with no effect
  stays offered: playing a card just to cycle it is common and deliberate.
- Genuinely weird cards get custom code behind the same interface. The framework is
  sized against the stress set (§6), not the median card.
- **Modifier scopes**: effects live in explicit scopes — strike, round, combat,
  action, phase, turn, permanent-while-in-play — on a scope stack popped wholesale
  when the scope ends (the rulebook's own model: stealth/intercept "persist for
  action duration then reset"). Events and in-play cards contribute persistent
  layers. Interrupt classes get explicit hook points: prevention (order-sensitive),
  cancel-as-played, replacement draws, cost reducers (usable at play or resolution).

- **Settled shape (owner, 2026-09-03; worked examples in
  [`docs/effects-framework.md`](docs/effects-framework.md))**: a card is a set of
  *modes* (its printed `[dis]`/`[TYPE]` lines); a mode is `{tags, windows,
  actor, requires, cost, targets, limit, effects, riders}`. The **legality
  half** (`windows`, `actor`, `requires`, `cost`, `targets`, `limit`) is a
  **closed data vocabulary** — named predicates evaluated by the engine; no
  card decides its own legality in custom code — so the mask is enumerated
  from data. The **execution half** (`effects`, `riders`) is interpreted
  primitives with a `Custom` escape hatch. In-play abilities are the same
  shape with an in-play source; continuous effects are scope layers; riders
  and triggers bind to named hooks. Behavior is authored as **Rust data
  tables** per card, checked by the compiler against the generated static half
  (`crates/engine/src/library.rs`, `crypt.rs`). Numeric terms are stepwise
  sub-decisions taken at play (Villein) or at the referendum's terms step
  (rules-qa 3.5). Stealth/intercept-adding plays carry `StealthNeeded` /
  `InterceptNeeded` unless the card text waives it (rules-qa 3.2).

Semantics authority for the tricky parts (requirements vs costs, cancellation vs
replacement, cumulative/stacking, simultaneity ordering, NRA, changing acting
minion): **vtes-advanced-rules**, which doubles as the spec for this layer.

### 4.6 State representation

Flat, arena-style, plainly cloneable (search later, fuzzing now), no interior
mutability. Zones per rulebook: ready / controlled / uncontrolled / torpor /
ash heap / hand / library / crypt (+ set-aside, removed). Counters, Edge, pool,
labels (contested, merged, Anarch, titles) as plain data. Cheap hashing for
transposition/debugging. No allocation in the hot loop; card behaviors resolved to
static function tables at load.

## 5. RL-facing surface

- **Dual observations, natively**: every decision request carries the decider's
  legal view; the training pipeline can additionally request the omniscient view
  (all hands, libraries) for the asymmetric critic. Nothing omniscient ever reaches
  the policy.
- **Options as feature vectors**: the policy scores the options presented
  (pointer-network style), not indices into a fixed action vocabulary. Card pool
  growth changes data, not network output shape — this is the V5→legacy path on the
  ML side.
- **Batched stepping**: `GamePool::step_until_decisions()` advances N games and
  returns a batch of `(game, player, obs, mask)` for a single GPU inference call.
  Exposed to Python via PyO3 with zero-copy tensors; the serialized protocol is for
  clients, not the training hot path.
- **Encoding sketch** (training phase, not engine concern): entity tokens
  (players with relative-seat/predator-prey encoding, minions, cards in hand,
  in-play cards), transformer trunk, auto-regressive head over decision options;
  reward = tournament scoring (VPs per oust, GW bonus), small survival shaping
  early; league = current agent + frozen snapshots + exploiters + scripted bots,
  PFSP sampling; decks × seats sampled per episode; policy conditioned on own
  decklist identity.

## 6. Window census (V5 pool, measured)

Text-pattern probe of the 421 library cards — the *pre-review* regex census,
kept for scale only; per-card truth (which lost e.g. Hidden Strength and Rat's
Warning from the press window) is the reviewed matrix below:

| Window / mechanism | V5 tenants | Examples |
|---|---|---|
| Card cancel as played | 9 | Sudden Reversal, Anticipation, The Line |
| Action cancel / as announced | 6 | Consign to Oblivion, Seduction, Form of the Cobra |
| Out-of-turn master | 2 | **Archon Investigation** (mid-action, bleed-amount trigger), Sudden Reversal |
| Wake (react as if unlocked) | 5 | Eyes of Argus, Forced Awakening, On the Qui Vive |
| Bleed redirect (blocks declined) | 5 | Deflection, My Enemy's Enemy, Telepathic Misdirection |
| Stealth/intercept exchange | many | Resist Earth's Grasp, Eagle's Sight |
| Press step | 18 | Flash, Hidden Strength, Rat's Warning |
| Additional strike | 11 | Blur, Pursuit, AK-47 |
| Strike: combat ends | 10 | Majesty, Earth Meld, Form of Mist |
| Combat would-end replacement / continue | 5 | Telepathic Tracking, Form of Mist (continue as unblocked), Immortal Grapple |
| Damage prevention | 30 | Indomitability, Hidden Strength, Flesh of Marble |
| Referendum/polling cards | 47 | Kine Resources Contested, Bewitching Oration, **Scalpel Tongue**, Amici Noctis, Banishment |
| Events (global ongoing) | 20 | Anarch Revolt, Fall of the Camarilla, Thirst |
| Diablerie-referencing cards | 4 | The Slow Withering, Carver's Meat Packing |
| Torpor/burn replacement | **0** | *deferred* |
| Bidding | **0** | *deferred* |

A generated per-card draft of this census lives in
[`data/window-matrix-draft.csv`](data/window-matrix-draft.csv)
(regenerate with [`scripts/window_matrix.py`](scripts/window_matrix.py)):
421 library cards classified per *mode* (discipline/type line) by play window
plus flags (delayed replacement, out-of-turn, in-play granted actions, bounce,
…). The hand-reviewed, owner-arbitrated matrix is
[`data/window-matrix-reviewed.csv`](data/window-matrix-reviewed.csv) (rules-qa
Batch 2); the generator encodes the review's conventions and reproduces it
exactly (`scripts/window_matrix.py --check data/window-matrix-reviewed.csv`),
so the reviewed matrix is the authoritative implementation input and the
generator is its regression check against card-data updates.

**Stress set** (the framework must express these before it is frozen):
Archon Investigation · Scalpel Tongue · Form of Mist · Telepathic Tracking ·
Deflection · Eyes of Argus · Sudden Reversal · Consign to Oblivion ·
Immortal Grapple · Blur · Villein (trifle) · Homunculus (unlock-phase effect) ·
Anarch Revolt (event, ongoing global) · Kine Resources Contested (interactive terms)
· one contested unique + one contested title scenario.

## 7. Testing strategy

The engine's value is trust; trust is measured, not asserted.

1. **Rulings as tests**: vtes-rulings YAML (card-keyed prose with provenance,
   so hand-translated, not generated) filtered to the V5 pool
   (`data/rulings-v5.csv`); each applicable ruling becomes a scenario test. Scenario format =
   decision scripts with KRCG card names, readable as annotated game transcripts.
   The CSV is the ledger: every row on an implemented card (`DEFS`) carries a
   `status` — `test:<fn>` (a `#[test]` under `crates/engine/tests/` whose
   comment cites the row's references; new ones live in `tests/rulings.rs`),
   `blocked:<gap>` (a card or mechanism the engine lacks, revisited when it
   lands) or `n/a:<reason>` (not engine-observable, or the V5 card is only the
   ruling's example). `scripts/rulings_v5.py --check data/rulings-v5.csv`
   verifies the ledger against the tests offline; regeneration keeps the
   statuses of unchanged rows and drops reworded ones as stale. The
   invariant is the ruling's assertion: a script is corrected as a script
   (fixture, window order), but engine behavior is never bent to a script —
   a real divergence is fixed in the engine or the row is marked `blocked:`.
2. **Property fuzzing**: millions of random-legal games; invariants — counter/pool
   conservation modulo explicit sources, no negative resources, capacity ceilings,
   every window terminates (paranoid step budget), elision observational-equivalence
   (§4.3), contested-state consistency, decision-log resimulation reproduces
   identical state hashes.
3. **Golden replays** pinned across refactors.
4. **Coverage checklist**: every row of the mechanics inventory (superset of
   jol-rebuild's gap analysis: named counters, limited effects, prevention ordering,
   simultaneous ousts, NRA on resolution incl. blocked (not on cancellation),
   contest upkeep, master
   accounting, withdrawal/timeout) answers "where is this handled, where is it
   tested". For the effects of cards in play, the hook vocabulary is
   closed by enumeration from the ability census over the in-precon pool
   (`data/ability-census-draft.csv`, 359 effect sentences on 385 cards;
   counts are regex matches, indicative) — the *hooks* are fixed now, the
   *tenants* land card by card:

   | Hook / mechanism | Sentences | First tenant | Status |
   |---|---|---|---|
   | Own unlock phase (optional ability / mandatory rider) | 39 | Vessel, hunting grounds / Creeping Sabotage | done (`UnlockPhaseOwn`, `Hook::UnlockPhase` + scope) |
   | Every / other Methuselah's unlock phase | 5 | Anarch Revolt / Smiling Jack | done (`RiderScope`, `Flow::RiderTerms` for a rider's choices) |
   | Master phase (optional / mandatory) | 10 | Blood Doll / Brujah Debate | optional done; mandatory rider pending |
   | Discard phase | 3 | The Coven | done (`Hook::DiscardPhase`, fired as the impulse leaves the controller) |
   | Influence phase | 6 | Powerbase: Montreal | pending |
   | End of turn | 2 | Dreams of the Sphinx | done (`Seat.hand_bonus` cleared at `end_turn`) |
   | Minion to torpor | 8 | Guardian Angel / Fame | done (`Hook::HostToTorpor`, mid-combat oust teardown) |
   | Minion burned / leaves the ready region | 7 | Tension in the Ranks | done (`Hook::ReadyMinionBurnedOrTorpor`, `burn_minion`) |
   | Enters play | 12 | The Rack, Ravnos Carnival | pending |
   | After a successful action / bleed | 2 | Alamut, Cappadocian Crypt | pending |
   | Blocked / block resolution | 4 | Crypt's Sons, Sniper Rifle, Unlicensed Taxicab | host rider done (`Hook::HostBlocked`: Unlicensed Taxicab); Sniper Rifle done as the blocker's before-range ability (`Requirement::ActorHasBlocked`, not a hook); Crypt's Sons pending |
   | Combat steps (weapon strikes, maneuvers) | 37 | .44 Magnum | done (host strike abilities, `Side.strike_source`; a gun's maneuver or Sniper Rifle's range set commits the initial strike, `Side.committed`); armor's per-combat allowance, `Damage.gun`, and a weapon's press to continue, `Side.continue_presses` (Kevlar Vest, Righteous Blade) |
   | Polling step | 11 | Alamut | pending |
   | Granted actions (Ⓓ burn / steal / enter combat) | 26 | Anarch Revolt, Creeping Sabotage | done for burn and for a host's own action (Codex of the Edenic Groundskeepers' bleed); steal and enter-combat pending |
   | Continuous layers | 179 | Villein, Guardian Angel | done (`Layer`; a bearer's stats `Modifier::Bearer`) |
   | Lock / burn abilities in action or combat windows | 17 | Homunculus, Guardian Angel | done (abilities in windows); untimed "you can lock this card" done (`Window::AnyImpulse`: The Barrens, Dreams of the Sphinx) |
   | Counters on a card | 46 | Smiling Jack, Dreams of the Sphinx | done (`CardInPlay.counters`; `PoolToCounter`, `AddCounter`, `BurnSelfAtCounters`, `CountersAtLeast`) |

   | Would replace a card | — | Visit from the Capuchin | done (`Hook::WouldReplace` in `Engine::replace`; delayed replacements by `Until`) |

   Library search sits outside the hook shape and is not started: every
   V5 search card needs another mechanism first — equipment (Magic of the
   Smith, Vast Wealth, Michael Harris; Black Market Cache fetches
   equipment) or an action creating a vampire (Childe of the Revolution,
   Waters of Duat) — so it lands with the equipment slice (8c), as do the
   draw alternatives ("if you would draw a card from your library, you
   can draw one of those cards instead": Black Market Cache, Shilmulo
   Tarot). A search chooses its card at resolution, not at announcement
   (spec §1.14.1, §3.1.2), and shuffles with the engine's `Rng`. Control
   change of a card in play is done for the hostless
   case (`CardInPlay.controller`, The Coven); "steal this location"
   actions and the re-placement of a stolen minion card (spec §6.3.2)
   land with locations (Dominate Kine [DOM], The Rack, the Powerbases,
   Living Manse on a minion) — no precon card steals equipment.
5. **Perft-style counts** on scripted positions (number of legal decision sequences
   to depth k) to catch legality regressions numerically.

## 8. Community surface

- **Wire protocol**: the same decision requests/answers, serialized (JSON/msgpack),
  over socket/WebSocket. Any client — including a future rules-enforcing online
  client, or a bot adapter for existing platforms — drives the engine through it.
  Non-enforcing platforms (JOL, Succubus Club) can host the bot only with the engine
  embedded as its rules oracle.
- **Human ergonomics layer** (not in the engine core): binding shortcuts with
  interrupt-and-rewind (log truncation + resimulation), preemptive impulse
  requests ("stops"), pace defaults — the play-procedures document is the spec.
- Bot inference service: engine + ONNX-exported policy, CPU-friendly (Pluribus
  precedent: play-time compute is cheap).

## 9. Performance targets

- Engine step: µs-scale; a full 4p random-legal game in low single-digit ms.
- Self-play throughput: thousands of concurrent games per box, NN inference the
  bottleneck (as it should be), batch sizes in the thousands.
- Full determinism under fixed `(seed, answers)` across platforms.

## 10. Explicit modeling decisions

Recorded so nobody mistakes them for oversights:

1. **Polling serialization**: free-for-all vote casting is serialized as repeated
   no-forfeit rotations starting at the acting Methuselah, clockwise (§4.2).
   Order-dependent edge cases (Scalpel Tongue timing) are preserved; the pure
   simultaneity of a live table is not.
2. **Silent VTES**: no negotiation channel; deals are social, never mechanical.
3. **Elision**: provably-empty windows are skipped by the omniscient engine;
   observational equivalence is an invariant, and live-play pacing tells are a
   client-layer concern.
4. **Deferred windows**: torpor/burn replacement and bidding — hooks exist, nothing
   opens them in the V5 pool.
5. **Fixed decklists**: the official precons; deck building is out of scope for v1.
6. **Tournament scoring as reward**: VPs + GW, timeout/withdrawal rules included so
   the agent learns real endgame incentives. Time limit: 100 player turns (owner,
   2026-09-02 — a tournament game rarely exceeds 20 full rounds), 0.5 VP per
   survivor; VPs are stored in half-points. Withdrawal (0.5 VP, an unlock-phase
   decision) is P1.
7. **Unlock-phase ordering**: the unlock phase is one impulse sequence
   (rules-qa 3.7, 7.8): cards unlock first, unordered; then one ordered
   window in which the active Methuselah settles their mandatory effects
   (contest upkeep — pay or yield — and the pending riders) in the order
   they choose among their optional ones (the Edge's pool, abilities), and
   cannot pass the impulse before the mandatory ones are done; the other
   players' unlock-phase abilities (Homunculus) follow in rotation, the
   impulse returning to the active player after each — the Edge's pool
   still on offer then, lost only once everyone passes.
8. **Mid-combat ousts**: a pool loss fired inside a combat (Fame's torpor
   rider) ousts at once. If the active Methuselah is ousted the turn passes
   with the action and combat torn down; if the opponent's Methuselah is
   ousted their minion leaves the game with them, so the combat ends at
   once — no end-of-round window for the survivor (nothing in the pool
   continues a combat with no opponent) — and the blocked action resumes
   at its after-combat window without the ousted seat; a directed action
   whose target left the game is not continued (Form of Mist's
   continuation masked) — unreachable in the current pool, guarded anyway.

## 11. Phasing

- **P0 — skeleton** (done 2026-09-03): state arena, decision loop, two window
  primitives, turn/phase machine, **real V5 crypt cards, no library cards**
  (owner, 2026-09-02: the first pass is not an inexact game — titles and
  contests are on the cards, so they are modeled). Generated crypt data
  (`scripts/gen_crypt.py` → `crates/engine/src/crypt.rs`, `--check` guards
  drift): the 200 V5 vampires with printed sect and title, and the 14
  full-size precon crypts, dealt at setup. The card-free rulebook actions
  (bleed, hunt, leave torpor, rescue, diablerie with the blood hunt as the
  first polling tenant, title votes as sources); exact influence decisions
  (transfers, withdrawals, influence-out, the 4-transfer + 1-pool crypt draw,
  no voluntary self-contest); card contests (face down, out of play, 1 pool
  per unlock phase or yield-and-burn) and title contests (prince/baron of one
  city, justicar per clan; 1 blood per unlock phase or the title is lost;
  torpor and 0 blood force the yield) settled by each Methuselah on their own
  unlock phase in arena order; random-legal agents; fuzzer + invariants +
  replay in CI from two starting positions (opening, seeded mid-game).
  Carried into P1: block attempts that cannot succeed are pruned by elision
  (P1: intercept reachability); the 100-player-turn time limit fires at the
  turn boundary; a blocked leave-torpor (no combat, blocker may diablerise)
  is unreachable without intercept. Two rulebook facts shape the card-less
  pool: every undirected action is +1 stealth, so only bleeds and
  cross-controller rescue/diablerie are blockable, and a vampire enters torpor
  at 0 blood, so leave-torpor is unpayable and rescue is the only way out.
- **P1 — stress set** (in progress): effects framework sized against §6's stress
  set; the A/B/C block loop, combat, referendum machines complete;
  rulings-as-tests harness. Slice 1 done (2026-09-03): library zones (precon
  libraries, hand of 7, ash heap), the master phase as an offer window with
  master-phase-action and trifle accounting, out-of-turn accounting (rules-qa
  3.3), numeric terms at announcement (3.5), the as-played cancel window with
  a nested cancel stack (spec §1.8), cost folded through layers, the discard
  phase; first definitions Villein and Sudden Reversal (`cards::defs`). Slice
  2 done (2026-09-03): the action as a state machine on the engine (as-announced
  window, the block loop as rulebook states A/B/C with a failed attempt
  returning to A, blocks-declined window), modifiers and reactions enumerated
  from definitions with actor, discipline tags, "only when needed" (rules-qa
  3.2) and intercept reachability replacing the P0 prune, wakes in the
  as-played window (2.4), bounces reopening blocks (1.3), action-scoped cost
  layers, deferred reaction replacement, the employ action with NRA by card
  name, an unlock-phase in-play ability window; definitions Bonding,
  Conditioning, Consign to Oblivion, Form of the Cobra [PRO], Eyes of Argus,
  Deflection, Archon Investigation, Homunculus. Slice 3 done (2026-09-04):
  the combat machine (`combat.rs`) — each step an ordered window over the two
  combatant seats, acting side first with impulse-reset (rules-qa 1.2);
  determine range and press as alternating toggles; strike choice acting
  first; a before-resolution window (spec §4.3.2, home of `COMBAT_UNTIMED`);
  resolution with combat-ends first, dodge, simultaneous steal-before-mend;
  prevention per damaged side, acting first, then damage applied at once with
  the rulebook's normal-then-aggravated resolution (its three examples are
  unit tests); additional strikes with the one-limited-source cap; presses; a
  shared end-of-round / would-end window with replacement (Telepathic
  Tracking) and after-combat riders lost on continuation; the after-combat
  window with the Form of Mist continuation re-entering the block loop with
  declines kept (rules-qa 3.4); the blocked leave-torpor (blocker's diablerie
  → blood hunt). Vocabulary added: `Actor::Combatant`, combat requirements,
  `StrikeKind`, `Limit` (per round/combat/action by card name), `Terms::XBlood`
  (Hidden Strength). Definitions: Flash, Torn Signpost, Roundhouse, Body
  Flare, Theft of Vitae, Majesty, Side Strike, Blur, Immortal Grapple, Hidden
  Strength, Soak, Indomitability, Claws of the Dead, Taste of Vitae,
  Telepathic Tracking, Cats' Guidance, Form of Mist, Form of the Cobra [pro]
  (28 defs). Known gaps: printed crypt abilities (strength, prevention,
  maneuvers on vampire cards) are not generated yet — strength is the rulebook
  default of 1; maneuver-from-strike-card riders (Aid from Bats, Slam, Diversion
  [tha]); environmental / per-round damage (Carrion Crows, Weather Control);
  first strike, weapons and equipment; cards played by minions not in the
  combat (Martyr's Resilience, Touch of Valeren [FOR]); rush / card-created
  combats. Batch 4 of rules-qa lists the modeling calls to confirm.
  Slice 4 done (2026-09-04): the referendum machine (`referendum.rs`) —
  the blood hunt generalized: terms chosen stepwise by the caller
  (allocations with feasibility masks, seat / minion / clan choices), the
  polling step as the unordered primitive with card plays as commitments
  of the same rotation, per-source vote records (abstentions cancel),
  Methuselah ballots (Edge, the calling card's vote, a political action
  card burned from hand), the tally with Scorn's penalty first, a
  shortfall re-allocation, simultaneous ousts as a batch (VP to every
  predator, 6 pool to survivors only), cancellation with the card
  returned and an immediate discard-down; the political action (+1
  stealth, one per minion per turn, cost at resolution, no referendum if
  the acting minion is no longer ready) and the after-resolution window;
  Expulsion's per-turn bars, delayed replacement (Old Friends), bleed
  reduction, an intercept layer (Perfect Paragon [PRE]), clan
  requirements. 25 politics defs (53 total). Known gaps: Anarch Revolt
  and the other events (P2 machinery); Archon, Raising the Portcullis
  (in-play granted actions); Disputed Territory and Revolutionary Council
  (locations / equipment); the Justicar title cards (card-granted titles
  and their contests). Batch 5 of rules-qa (answered 2026-09-04) records the
  modeling calls: a banished vampire returns as the arena entry it left,
  cards and history included; the blood hunt sits inside the diablerie
  action, the after-resolution window after it.
  Slice 5 done (2026-09-05): effects carried by cards in play — mandatory
  ones fired at a hook and resolved by whichever Methuselah the text
  names (Anarch Revolt is controlled by its player, yet every Methuselah
  resolves its burn at their own unlock phase), and optional / granted
  ones offered as abilities. Anarch Revolt is a Master card in V5 ("Put
  this card in play"); the `Event` card type (a controlled card type, 20 in
  the pool, none in a precon) stays P2 and reuses this shape.
  Vocabulary: `Effect::PutInPlay` (no host); `CardDef.riders` — a `Rider`
  is `(Hook, effects)` (effects-framework §4 `phase_trigger`);
  `Hook::UnlockPhase` riders are pending mandatory effects the active
  Methuselah orders among their own unlock-phase abilities
  (`Choice::ResolveRider`, `Pass` masked until none is pending) and
  resolves before the impulse passes (rules-qa 6.1), one burn per copy
  (§1.15), `oust_and_pass` at 0; granted actions as `abilities` with
  `Actor::ActingCandidate` on *any* seat's card in play, enumerated in the
  minion phase like card actions (`Choice::UseAbility` carries the actor;
  `ActionCtx.source`), their stealth from the card's text only, NRA per
  copy (`Minion.acted: Vec<ActionId>` — name identity for hand cards, copy
  identity for cards in play — recorded at resolution) plus the type cap
  for political actions, Delaying Tactics banning by the card's name, no
  calling-card vote; `RefEffect::BurnSource` (`burn_card_in_play`: owner's
  ash, layers dropped). Definition: Anarch Revolt (54 defs). Batch 6 of
  rules-qa (answered 2026-09-06) records the calls.
  Slice 5b-i (2026-09-06), the non-unique in-play effects of the widening
  batch: `Rider` gains a `RiderScope` (every / controller / others);
  `Hook::HostToTorpor` fired where the torpor happens (rules-qa 6.4);
  host abilities keyed on the host's *controller*, offered in combat
  windows too (no as-played window, per-copy `Limit::PerCombat` in
  `CombatCtx.abilities_used`); `Modifier::HostInterceptOnBleedAtYou`
  (counted by reachability); `ActionKind::Card` — a generic action card
  resolving its mode's effects on success through `apply_card_effect`
  (shared with plays), its printed stealth read at declaration, X cost
  from `Terms::CopiesControlled` read at announcement for the mask and
  again when paid; `ActionKind::BurnCard` ("burn this card as a Ⓓ action":
  directed at the card's controller, undirected +1 for their own minions);
  a rider's pool loss on another Methuselah goes through `burn_pool_batch`
  (ousted at once, the phase goes on); a granted action has no default
  stealth, only its printed one (rules-qa 6.2). Host abilities in phase
  windows follow the "during X, do Y" template — once per that phase per
  copy (`GameState.phase_uses`, rules-qa 6.6) — offered in the master
  phase (no master phase action spent) and, `Window::UnlockPhaseOwn`, in
  the controller's own unlock phase; a play's "you can burn a
  card named X (if any)" is a term announced with the play (any copy in
  play; `Terms::BurnNamedInPlay`, rules-qa 6.7) and burned at resolution. Definitions:
  Guardian Angel, Creeping Sabotage, Blood Doll, Vessel (58 defs). The
  fuzz midgame fixture now seeds each library with the slice, so
  replacements keep offering defined cards. Every remaining Methuselah
  ousted in one batch is a legal end (rules-qa 6.9): prey VPs stand, no
  last-standing bonus; `GameState::game_win()` is the tournament game win
  (2+ VP and a strict lead, ousted or not; a tie is a draw). Next: library-card contests (a unique card
  played while another copy is in play) — every remaining card of the
  batch is Unique — with Fame, Tension in the Ranks, Smiling Jack,
  Constant Revolution, The Barrens, Dreams of the Sphinx, The Coven and the
  (2+ VP and a strict lead, ousted or not; a tie is a draw). The stress
  set's contested scenarios: crypt-card and title contests are scripted
  unit tests (`game.rs`: `card_contest_pay_yield_and_return`,
  `title_contest_prince_and_baron_of_one_city`); slice 5c adds the
  library-card half. Still open from the P1 definition: the
  rulings-as-tests harness (`data/rulings-v5.csv` is not consumed yet).
  Slice 5c (2026-09-06): library-card contests (rulebook "Contested
  Cards", spec §1.13) — `CardInPlay.contested` / `present()`: a contested
  card is out of play, so abilities, riders, copy counts and "a card named
  X" skip it and its layers are suspended (`GameState::layer_active`);
  entry after the card's own effects resolve (§1.13.2) through the single
  `enter_play` helper (plays, `Card` actions, employ); self-contest masked
  (§1.13.1, rules-qa 7.1); upkeep as `Contest::Library` in the unlock
  machinery (1 pool, yield burns, return face up with no rival). Tenants:
  the 13 precon hunting grounds, Papillon, Carfax Abbey — a hostless
  unique location with an optional own-unlock pick (`Actor::Controller`,
  `Choice::UseAbility.target`, `Effect::HuntingGroundBlood`,
  `Minion.hunting_ground_fed` for rules-qa 6.5; `Limit::PerPhaseEach` for
  Carfax Abbey's two picks within one "during" clause). 73 defs.
  Slice 5d (2026-09-06): Fame — `Rider.requires` (host predicates read at
  collection and at resolution, so an unmet mandatory rider is not pending
  and its window elides; `Requirement::HostInTorpor` is false for a host
  that left the game), `Effect::BurnPool` ("that Methuselah burns 1 pool",
  every Methuselah's own unlock phase; LSJ 20010121 = rules-qa 6.1) and
  `Effect::HostControllerBurnsPool` fired where the torpor happens (rules-qa
  6.4) through `burn_pool_batch`, with the mid-combat oust teardown (§10.8:
  `apply_damage` reports the flow moved on; the acting seat's turn passes,
  an ousted opponent ends the combat at once, two seats left ends the
  game). 74 defs.
  Slice 5e (2026-09-06): Tension in the Ranks — minions leave play through
  one path each: `burn_minion` (the cards on it burned to their owners'
  ash heaps, spec §1.12.1; "after a ready minion is burned" riders only if
  it was ready — not a diablerie victim, not a yielded contested vampire)
  and `minions_to_torpor` (cards stay; host riders and the "sent to torpor"
  riders fire together), both through `leave_riders` batching the pool
  losses (`Effect::TheirControllerBurnsPool`, the subject minion's
  controller; Fame's host rider renamed onto it). The burn sites: combat
  damage, the blood hunt, Archon Investigation (`BurnActingMinion`, whose
  rider may oust the active Methuselah inside the play — the resolution
  passes the turn), the diablerie victim, a yielded crypt contest. An oust
  removes what the Methuselah *controlled* (spec §6.1.2,
  `GameState::card_controller`: a card follows its host's controller): the
  cards on their minions whoever played them go, their cards on other
  Methuselahs' minions stay — removed, not burned, no rider (§1.10). The
  burn ability: `Actor::Methuselah` abilities (any Methuselah in their own
  window), `Requirement::MasterPhaseActionLeft` / `HandMasters(n)`,
  `Terms::DiscardMasters(n)` chosen one card at a time in a
  `Flow::AbilityTerms` window (label `PlayTerms`) before the ability
  resolves, `Effect::SpendMasterPhaseAction`, `BurnSelf` as an ability
  effect; `use_ability` takes the flow to resume. 75 defs. Control of a
  card in play (rules-qa 7.5): a master card stays its player's wherever
  it sits (Fame on another Methuselah's vampire), a minion card on a minion
  is its host's controller's — `GameState::card_controller` keys the
  contest paths (self-contest mask, `library_rivals`) and the oust: an
  ousted Methuselah's controlled cards are removed (§1.10, nothing
  triggers), other players' cards on their removed minions are burned to
  their owners' ash heaps. `RiderScope` and `Actor::Controller` still key
  on the player, right for every card in the pool so far. An out-of-turn
  play paid down to 0 pool (Archon Investigation at 3) ousts its player
  without ending the active Methuselah's turn: the player drops out of the
  open windows (`OrderedWindow::remove`).
  Slice 5f (2026-09-06): Smiling Jack, The Anarch — a mandatory rider
  with choices: `Effect::PoolToCounter` (the controller's rider, onto
  `CardInPlay.counters`; the last pool moved ousts them) and
  `Effect::PoolOrBloodPerCounter` (the others' rider: each unit paid as
  chosen in a `Flow::RiderTerms` window, `Choice::BurnPool` /
  `BurnBloodFrom`, any mix per ANK 20181006, empty vampires no source per
  LSJ 19971212; the window elides when the pool is the only source; a
  pool paid to 0 ousts on the spot). `resolve_unlock_rider` now sets the
  flow itself (the resume, the choice window, or the next turn).
  `Requirement::ActorVampire` for "Vampires can burn this card" (vacuous
  until allies land). 76 defs.
  Slice 5g (2026-09-06): Dreams of the Sphinx, The Barrens — untimed
  in-play abilities: `Window::AnyImpulse` is asked for at every decision
  point where a seat holds the impulse (master, minion, influence and
  discard phases, the unlock rotation, every action and combat window,
  the polling step) and never in the as-played or terms windows (ANK
  20200616; LSJ 20061207 / rules-qa 2.4); using one is an effect (the
  ordered window's impulse resets, the polling window commits). Card
  locks: `Requirement::CardUnlocked` / `Effect::LockSelf`, cards a
  Methuselah controls unlock in their unlock phase (`start_turn`);
  counters `AddCounter` / `BurnSelfAtCounters` (the third lock's effect
  lands, then the card burns); `Seat.hand_bonus` for "+2 hand size until
  the end of the turn" (drawn up at once, cleared at the end of the
  discard phase per LSJ 20100812, the cards over size then discarded down
  — corrected in slice 7, rules-qa 8.2); `GainPool`, `HasEdge`;
  `TargetKind::Uncontrolled` / `Target::Uncontrolled(slot)` /
  `BloodToUncontrolledTarget` (capped at capacity); `Terms::Discard { n,
  masters }` generalizes the discard terms (The Barrens: any card,
  `HandNotEmpty`). A starving vampire's forced hunt is offered alongside
  the untimed abilities, not instead of them (owner, 2026-09-06: The
  Coven's blood spares the hunt). Rules-qa 7.8: contest upkeep is
  freely ordered among the unlock effects like the riders (a mandatory
  decision), the Edge's pool is an optional one — the opening Edge
  window and the contest sub-flow are gone, `Pending::{Rider, Contest}`
  in the one unlock effects window (`Choice::PayContest(Contest)` /
  `YieldContest(Contest)`), forced settlements at collection. 78 defs.
  Slice 5h (2026-09-06): The Coven — control of a card in play is state
  (`CardInPlay.controller`, its player until a change; `card_controller`
  still routes a minion card to its host's controller), read by
  everything control-keyed (abilities, rider scopes, contests, ousts,
  unlocking); `Hook::DiscardPhase` riders fire as the active Methuselah
  passes out of their discard phase (LSJ 20100723) —
  `Effect::PredatorTakesControl` to the controller's predator at that
  time — onto a predator already controlling a copy, the incoming one is
  burned (a forced contest, spec §1.13.1); `Effect::BloodToTarget`
  (capped). Fuzz: a game ends only with ≤1 Methuselah or at the time
  limit (a flow left on the placeholder ended a 5g game silently — the
  Coven's minion-phase use found it), every card in play has a living
  controller, and `Seat.removed` counts cards removed while another
  controlled them, closing the conservation sink. Rules-qa 7.9: each of
  the active Methuselah's phases ends on an impulse rotation over the
  other seats (`Flow::PhaseRotation`, label `PhaseImpulse`; mandatory
  effects first — The Coven moves at the predator's impulse — then
  untimed abilities; an effect returns the impulse to the active player
  in their phase; single-option rotations elide as any window). 79
  defs.
  Slice 5i (2026-09-06): Constant Revolution, the last card of the
  batch — an action card entering play with counters
  (`Effect::EnterWithCounters`), the controller's `AddCounter` unlock
  rider, the others' `PoolOrRandomCardsPerCounter` rider announced as a
  card count in the rider terms window (`SetX`; random burns from the
  engine's own `Rng`, so replay stays seed + log; rules-qa 7.10 on the
  replacement), a granted burn "that costs 1 pool" (`Effect::PayPool`
  paid at resolution, `Requirement::PoolAtLeast` at declaration; paid
  to 0 the acting Methuselah is ousted); a granted burn is directed at
  the card's *controller*. 80 defs — the in-play effects batch (§6
  stress set) is closed. Two pool-paying ability paths still assume the
  user is the active seat (`PoolToHostBlood` ousts with `oust_and_pass`):
  true for Blood Doll's own-phase windows, to route through `oust` when
  the first pool-paying untimed ability reaches a non-active seat.
  Slice 6 (2026-09-06): the rulings-as-tests harness (§7.1) —
  `data/rulings-v5.csv` is the ledger, `scripts/rulings_v5.py --check`
  the gate; the 248 rows on the 80 defs are triaged (122 `test:` over 72
  tests, 42 of them new in `tests/rulings.rs`; 57 `blocked:` naming the
  missing card or mechanism; 69 `n/a:`). One engine finding, fixed: the
  Delaying Tactics repeat ban is keyed by `ActionId` — a card by name, a
  granted action by copy (LSJ 20020201). One extractor bug, fixed: quoted
  YAML keys (names with a colon or apostrophe) were read as continuation
  lines, misattributing their rulings to the previous card (1121 rows
  now). Rules-qa 8.1 answered (2026-09-07): Bonding's [DOM] mode needs the
  stealth to be needed (`StealthNeeded`), like a plain stealth card. Next:
  draw replacement and hand size (the `blocked:` rows name the rest of
  the gaps).
  Slice 7 (2026-09-13): replacement and hand size. Hand size is derived
  (`GameState::hand_size`: 7, the until-end-of-turn bonus, and the
  `Modifier::HandSize` / `HandSizePerCounter` layers of the uncontested
  cards the seat controls), and the cards not replaced yet count against
  it (`Seat.held`, by the moment they come due: `Until::NextUnlockPhase`
  before cards unlock, `Until::NextDiscardPhase` as the discard phase
  begins, the end of the action for Consign to Oblivion [OBL]; spec
  §1.9.2, the longest delay wins). A hand over that size is discarded down
  before any other decision (`WindowLabel::DiscardDown` for any seat;
  rulebook "Drawing Cards": "immediately discard down") — which corrects
  slice 5g: the cards over size after Dreams of the Sphinx's bonus no
  longer stay (rules-qa 8.2). Every card leaving the hand that the rules
  replace goes through `Engine::replace` — a card played or canceled, the
  action card at declaration, the discard phase action, ability discards
  (The Barrens, Tension in the Ranks), a political action card burned for
  a ballot, Constant Revolution's random burns, a delayed replacement
  coming due — which offers each replacement to the seat's
  `Hook::WouldReplace` riders first (spec §6.8.3: "whatever took the card
  out of hand"; rules-qa 8.3: an empty library replaces nothing, so
  nothing burns instead); a hand-size increase only draws up
  (`draw_up`), a library-card contest ending included. Mirror Walk [THA]:
  `Effect::EndActionIfBlocked` ends the action as blocked where the block
  resolves — the blocker locked, no combat, no diablerie on a blocked
  leave-torpor (rules-qa 8.4). Definitions: Elder Library, Visit from the
  Capuchin, Wake with Evening's Freshness, Mirror Walk (84 defs). Ledger:
  266 rows on 84 defs, 79 tests referenced; Consign to Oblivion's "replace
  the action card before playing it" row is now a test (the action card
  is replaced at declaration, before the as-announced window). Library
  search is not in this slice (§7.4): it lands with equipment, with the
  draw alternatives (Black Market Cache, Shilmulo Tarot) — the tenants of
  the Capuchin's "choose which replacement treatment applies" (ANK
  20200905, `blocked:`).
  Slice 8a (2026-09-13): the equip action and equipment on a minion, the
  first of three equipment sub-slices (8b weapons in combat, 8c search and
  draw alternatives). Equipping from hand is `ActionKind::Equip` (rulebook
  "Equip": undirected, +1 stealth plus what the card prints for the action
  to equip with it; pool and blood costs paid at resolution — unpayable, or
  no longer holdable, by then it fizzles, rules-qa 3.6; no-repeat by card
  name like employ, rules-qa 1.4). From another minion the Methuselah
  controls, ready or in torpor, `ActionKind::EquipFrom` announces the
  equipment one card at a time (`Flow::ActionTerms`,
  `Choice::TakeEquipment`), is free, checks no requirement (ANK
  20180719-3), adds each taken card's own equip stealth (LSJ 20060222) and
  moves what is still there at resolution. "A minion can have only one
  vehicle" is `Requirement::ActorWithout(Holding)`, read on the offers and
  again at resolution (`GameState::can_hold`). What equipment gives its
  bearer is read live on the current bearer: `Modifier::Bearer { stat, n,
  during, who }` for stealth, bleed and intercept (`ActionCtx::stealth` and
  `bleed_amount` take the state), the named trait in `who` (a non-Ravnos
  holding Treasured Samadji gets nothing, rules-qa 9.1), and
  `YoungerInterceptAgainstBearer`; host abilities are offered at the strike
  choice under `CombatCtx::allows` (no dodge under a grapple); a host's own
  granted action (`Actor::Host` declaring one: the Codex's bleed, 1 blood
  paid at resolution and fizzling if the Codex left the acting vampire, RTR
  19960221; its +3 against a Methuselah with no ready unlocked minion is
  part of the action's bleed amount, read live like any bleed modifier);
  `Hook::HostBlocked` riders applied where the block resolves, before
  anything else (Unlicensed Taxicab, ANK 20220116; `TitleClass::Archbishop`
  kept from the text, matching no V5 vampire yet); a granted burn's
  `LockHostSkipNextUnlock` (`Minion.skip_unlock`). Definitions: Sport Bike,
  Unlicensed Taxicab, Treasured Samadji, Codex of the Edenic Groundskeepers,
  Stolen Police Cruiser, Bowl of Convergence (90 defs), scripted in
  `tests/equipment.rs`. The Bowl's [AUS] burn is an ability timed in the
  block loop's windows: once per action (`Limit::PerAction`,
  `ActionCtx.abilities_used`), only on the bearer's own needed block
  attempt, and counted by intercept reachability. Held back whole: Living
  Manse (a location on a minion, with location stealing), Flaming Candle
  (once a game) and Sword of the Archangel (a cancel of grapple and aim
  cards). Rules-qa Batch 9 answered (2026-09-13): the bearer-trait and
  equip-from readings stand; for 8c, Magic of the Smith offers no
  equipment with an unpayable blood cost, Vast Wealth burns a first find
  it cannot equip and pays an ousting pool cost, draw alternatives are a
  choice per drawn card (one treatment per replacement beside the
  Capuchin, nothing from an empty library), Heart of Nizchetus' bottom
  order is the player's.
  Slice 8b (2026-09-13): weapons in combat. A weapon's strike is an
  ability of the card in play (`Actor::Host`, `CombatStrikeDeclaration`)
  and the chosen strike remembers it (`Side.strike_source`): no longer on
  the striker when strikes resolve, it has no effect (spec §4.10.4). A
  melee weapon's "strength+1" is `StrikeKind::Melee`, close range and not
  a hand strike (Immortal Grapple bars it, Claws of the Dead does not
  reach it). "With 1 optional maneuver each combat" is a determine-range
  ability limited per copy (`Limit::PerCombat`, tracking the weapon, LSJ
  19980302-2) whose use chooses the initial strike (`Effect::CommitStrike`,
  `Side.committed`; rulebook "Determine Range" and "Choose Strike"): only
  that card's strike is offered for the round's initial strike, none at
  all if an effect now bars it (spec §4.3.1), and no second commitment
  that round; additional strikes are free. AK-47's rider
  (`Effect::AdditionalStrikeWithSource`) is read once its strike has
  resolved — dodged or not, not when a combat ends strike ended the round
  first — and grants the round's one limited additional strike, only usable
  with the gun or declined (`Side.additional_with`, `Choice::Pass` at the
  strike choice). Sniper Rifle's "if the bearer blocks … before range is
  determined" is a before-range ability of the blocker in the first round
  (`ActorHasBlocked`, `FirstRound`): `Effect::SetRangeLong` skips the
  determine range step, usable only while no other effect set the range
  (RTR 19970630, ANK 20180720). Definitions: .44 Magnum, AK-47, Assault
  Rifle, Flamethrower, Ivory Bow, Femur of Toomler, Kali's Fang, Sniper
  Rifle (98 defs), scripted in `tests/weapons.rs`. Per-combat and
  per-action ability uses are keyed by card copy alone — two limited
  abilities on one card would share a use (none yet) — and a strike
  commitment names a card in play: a strike card played in determine range
  for its maneuver will need the same commitment on its play (none
  implemented yet). Rules-qa Batch 10 answered (2026-09-14): the readings
  stand, and Sniper Rifle's range set needs the rifle on the blocker when
  it blocked (`CombatCtx.held_at_block`, `SourceHeldWhenBlocking`). Kevlar
  Vest: pending damage counts its gun-strike points (`Damage.gun`; other
  preventions take the rest first); gun or other is chosen once each
  combat, and the gun branch prevents one point a use while points are
  left, over the combat's strikes, only against gun-strike damage
  (`CombatCtx.gun_prevention_left`; LSJ 20081210). Righteous Blade: its
  strike banks a press only usable to continue
  (`Effect::ContinuePressEachCombat`, `Side.continue_presses`) as it is
  chosen, kept though dodged or unresolved, once per Blade per combat.
  100 defs.
- **P2 — full engine pool**: all 621 cards including Events and their machinery;
  the 14 training decklists plus the New Blood minis as data; perft baselines;
  protocol server; scripted reference bots (aggro-bleed, wall, vote).
- **P3 — training loop**: PyO3 batched stepping, encoder, PPO self-play at 4p;
  evaluation ladder vs scripted bots and frozen snapshots.
- **P4 — league & 5p**: exploiters, PFSP, deck/seat sampling, evaluation vs humans.
- **P5 — community**: bot service, client adapter, human-ergonomics layer.

## Outlook: deck building (post-v1)

Once engine + pilot agent work, deck building is the natural extension — and the
project's architecture is deliberately ready for it: fast simulator + competent
pilot = the deck-evaluation oracle every prior deckbuilding-AI attempt lacked;
pointer-style policy means new decklists are data, not network changes; engine
pool ⊃ training roster means the search space is already implemented.

- **Formulation**: not "find the best deck" but co-evolve a metagame — deck
  fitness is relative to the field, archetypes cycle, so use population /
  quality-diversity search (archetype descriptors as diversity axes) with
  fitness = expected tournament score vs the current population.
- **Known hard problem — pilot–deck chicken-and-egg**: candidate decks score
  through the pilot; unfamiliar archetypes yield false negatives that steer
  search away from novelty. Mitigations: decklist-conditioned policy (planned
  from v1), per-candidate fine-tuning before scoring, full co-evolution (pilot's
  training distribution = builder's population).
- **Unique assets**: the TWDA (~4k tournament-winning decklists, machine-readable
  via KRCG) as a generative prior / mutation bias; deck legality validation
  already solved in krcg.
- **Bridge study — precon tuning**: frozen pilot, ≤10 card swaps from a precon,
  hill-climb. Cheap, exercises the whole loop, measures pilot bias, and a
  measurably improved precon is a community artifact by itself. Can start as
  soon as the P3/P4 bot is decent.

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
