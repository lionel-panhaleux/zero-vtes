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
   tested".
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
   (rules-qa 3.7): the active Methuselah settles their own effects — the engine
   takes the Edge decision first, then contest upkeep in arena order; the only
   line dropped is paying contest upkeep down to 0 pool before taking the
   Edge's pool — then passes the impulse so other players' unlock-phase
   abilities (Homunculus) can be used, the impulse returning to the active
   player after each.

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
