# zero-vtes — Design Document

A rules-enforcing VTES engine and a self-play deep-RL agent for the V5 card pool.

What the engine and the agent are, the decisions that hold and the engine's
phases (§11); the engine reference by mechanism is
[`docs/engine.md`](docs/engine.md). History lives in git; rules answers
live in [`docs/rules-qa.md`](docs/rules-qa.md), not repeated here. Sources:
the [VEKN rulebook](https://www.vekn.net/rulebook),
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
through actions and votes). Every card, legacy included, is the roadmap's
objective B: the cards line adds them in rounds (§3).

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
Shadows of Berlin, Echoes of Gehenna — plus legacy cards the owner added
(`EXTRA_CARDS` in `scripts/window_matrix.py`, read by every generator and the
ledger), and every other card (roadmap objective B), appended after the V5
tables in batches so existing indices stay put (`ADDED_BATCHES`, merged as
`ADDED_CARDS`). The script holds the lists; the cards line adds them in
rounds, set by set (the `add-cards` skill, the cards board).

- **The pool's counts**: DESIGN §11's count line (definitions) and the
  cards board's header (the pool against KRCG's, the TWDA decks in it).
  V5 library by type: Master 124, Combat 63, Action 59, Action Modifier 53,
  Reaction 40, Equipment 23, Event 20, Ally 19, Political Action 19, Retainer 12
  (multi-type cards counted per type).
- **Engine pool = the whole pool**, Events included (conditional delayed
  replacement, tokens, global capacity/sect layers, randomness, mandatory unlock
  decisions). Playing an event is an optional discard-phase action, so events
  stress the effects framework, not the decision protocol.
- **Training decks**: deck families drawn per seat (`--decks`): the 14
  full-size precons (Fifth Edition 7 · Anarch 4 · Companion 3, from KRCG's
  precon bundles, `krcg.sets`: 379 unique cards), the reviewed anniversary
  decks (`Deck::anniversary`), the Sabbat V5 precons (`Deck::sabbat`) and the
  TWDA decks fully in the pool (`Deck::twda`, `scripts/gen_twda.py`),
  capped at 3 versions of a deck (owner: decks sharing more than 25% of
  their crypt are the same deck; a precon is its deck's first version; a
  further version only with a library sharing at most 50% with each kept
  one). A seat draws a clan uniformly, then a deck of it, and a deck
  sharing more than 20% of its crypt with one already at the table is
  redrawn (contests). The New Blood mini-starters are decklist data
  (`Deck::mini`) outside the roster — half decks, not legal (owner): never
  trained on; the singles-only cards (Fall of London, Echoes of Gehenna,
  part of Shadows of Berlin — all the events) are in no precon.
- **No V5 tenant**: burn replacement, bidding free-for-alls, torpor
  replacement outside combat (the combat's going-to-torpor step,
  `Step::GoingToTorpor`, serves a legacy card: Decapitate).

The set list is configuration: extending toward legacy means adding cards and
enabling deferred windows, not changing the framework.

## 4. Engine architecture

### 4.1 The core inversion: decision requests

The engine is a resumable state machine. It advances deterministically until some
player must choose, then emits a **decision request**
(`DecisionRequest { seat, window: WindowLabel, options: Vec<Choice> }`; the
observation, built from the requester's legal view, is §5's). The answer is an
index into `options`: the entire interface for the network, scripted bots, the
fuzzer and human clients.

**Factoring rule**: combinatorial choices are decomposed into unit choices
(1 pool at a time, one target at a time, terms stepwise). No decision enumerates
a product space. This matches the auto-regressive policy head and the tournament
declaration protocol (play procedures §X.1).

**Determinism & replay**: a game is `(seed, [answers])`, the seed 53 bits (the
server's: too many to search from a known opening hand); all randomness (shuffles,
random discards, coin flips) comes from the engine's own `Rng`. The decision log
(`Engine.log`, real decisions only) is the replay format and the corpus of
human games. Rewind = truncate + resimulate (the mechanism behind undo, §8).

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
network; credit assignment shortens accordingly). Every legal block attempt
is offered, one that cannot reach the stealth included (rules-qa 73.8:
every legal play); the host hides those with the opportunities off
(docs/engine.md §6.6 Block loop).

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
  contest during *their own* unlock phase, so no cross-player simultaneity
  exists; two of their own vampires claiming one title (rules-qa 3.1) are
  both settled there, in their order. Each contested object is a binary
  pay/yield decision (1 pool for a card, 1 blood for a contesting titled
  vampire); forced yields (no resource, torpor) are elided;
  gaining another title while contesting yields the contested one. Two broke
  princes resolve naturally: whoever's unlock phase comes first is
  force-yielded. Yielded cards burn with everything on them; a card whose
  rivals all yielded returns face up on its controller's next unlock phase.
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
  → block; short of it, the attempting Methuselah's pass in that round was
  its decline, and the open window goes on from the seat after it, never
  back to the acting one, whose impulse there is only the window's first
  and the one after a play (rules-qa 82.1). Failed otherwise (an effect,
  a lock, a toll unpaid), the loop returns to "no attempt" (rulebook B.4),
  where the same Methuselah may attempt again (82.2). A pass **with no
  ongoing attempt is a final decline**, reopened only by a change of target
  (bounces: Deflection, Telepathic Misdirection — not the card Change of Target,
  which ends the action). Hence a third party's help cannot reopen a final
  decline; a reduction of stealth or intercept needs no attempt (rules-qa
  1.3, LSJ 20020612).
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
  plays; "before votes are cast" is a requirement inside polling, not a window
  — a political referendum opens with a pre-vote rotation for those cards,
  the caller last: rules-qa 64.12) → tally → the action's after-resolution
  window.
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
- **Behavior**: a card is a set of *modes* (its printed Discipline/type lines);
  a mode is `ModeDef { windows, actor, requires, target, terms, waives_needed,
  limit, effects }`; a `CardDef` adds in-play `abilities` (same shape),
  `riders`, trifle/out-of-turn/delayed-replacement flags, and takes its printed
  cost from the generated half. The **legality half** is a **closed data
  vocabulary** of named predicates evaluated by the engine — no card decides its
  own legality in custom code — so the mask is enumerated from data. The
  **execution half** is interpreted primitives (`Effect`, `RefEffect`) with
  custom arms where needed. In-play abilities have the same shape with an
  in-play source; continuous effects are layers; riders bind to named `Hook`s.
  Behavior is authored as Rust data tables (`cards::defs`, `cards::vampires`)
  keyed by printed name, checked by the compiler against the generated static
  half. Numeric terms are stepwise sub-decisions at play (Villein) or at the
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
  Everything conditional is **read live**, never snapshotted (docs/engine.md §6.5).
- Weird cards get custom code behind the same interface; the framework is sized
  against the stress set (§6), not the median card.

Semantics authority for requirements vs costs, cancellation vs replacement,
stacking, simultaneity, NRA, changing acting minion: **vtes-advanced-rules**.

### 4.6 State representation

Flat, arena-style, plainly cloneable, no interior mutability (`GameState`:
`seats`, `minions`, `cards`, `layers`, …; entries are never removed — flags mark
what left play). Zones per rulebook: ready / controlled / uncontrolled / torpor /
incapacitated / ash heap / hand / library / crypt (+ removed). Counters, Edge, pool, labels as
plain data. `GameState::hash` for replay checks. Card behavior resolved to static
tables (`cards::lookup` is indexed by library index). `GameState::format`: the
rules a game is played under, set as it is dealt — VEKN's two-player variant
for 2 seats (rules-digest §18.5), the rulebook's otherwise.

## 5. RL-facing surface

`Engine::run`/`choose`, the legal view, observations (`zvtes_env::obs`),
the batched pool (`crates/py`), the model and its PPO loop (`train/`);
§11.2 "Observations" and "Training" describe them, the omniscient critic,
the league and the remaining-deck tokens included.

- **Legal view** (`GameState::legal_view(viewer)`, a redacted `GameState`:
  `Hash + Eq`): what `viewer` may know — the state, every hidden card identity
  replaced by `HIDDEN` in place (zone sizes, counters, blood, pool and flags
  kept). Hidden: every seat's library and crypt (order unknown; a search
  shuffles; the cards Heart of Nizchetus puts at the bottom are hidden too —
  the policy remembers its own decisions), the viewer's own keeping their
  contents, sorted (owner: a Methuselah knows their decklist and
  sees every card leaving or entering their library and crypt — what remains
  is theirs to know, its order is not; a client shows them face down); another
  seat's hand unless `hand_visible_to` (Revelations [AUS], Owl Companion);
  another seat's research area (Research); another seat's face-down uncontrolled
  vampires (rulebook: a Methuselah looks at their own), a banished vampire's
  slot excepted (it left play in view of all); cards stored face down (Shilmulo
  Tarot, `Effect::StoreTopCards`) except to the card's controller. Public: ash
  heaps, removed cards, minions and cards in play (contested cards keep their
  identity), counters, `held` counts. The rng is on `Engine`, not in the state.
  The action, combat and referendum contexts and the as-played stack are public
  (announced plays, targets, windows) except a card named from hand as a play or
  action is announced (`PlayCtx::named`, `ActionCtx::exchange`'s hand card),
  which need not be shown (advanced rules §1.14.2): the observation must redact
  it when it adds the contexts. Owner's calls: the banished slot and contested
  cards kept public; a vampire revealed on its way to the uncontrolled region
  (`RevealCryptTop`) hidden like any other.
- **Dual observations**: every request carries the decider's legal view; training
  can also request the omniscient view for the asymmetric critic. Nothing
  omniscient reaches the policy.
- **Options as feature vectors**: the policy scores the options presented
  (pointer-network style), not indices into a fixed vocabulary — pool growth
  changes data, not network shape.
- **Batched stepping**: `crates/py`'s `GamePool` advances N games to their
  next decisions and returns the batches for one GPU inference; the
  serialized protocol is for clients.
- **Encoding sketch**: entity tokens (players with relative-seat encoding,
  minions, hand, in-play cards; since `obs::Version::V2` the ash heaps'
  contents and the last public choices, since `V4` the cards revealed to
  all, §11.2), transformer trunk, auto-regressive head over options; the
  viewer's remaining deck pooled into the context token
  (`train/model.py`); reward = tournament scoring (VPs, GW) — a
  potential-based shaping by the position, tried in p15, cut the rushes
  and was dropped (§11.2 "p15"; `--shaping` kept), as was a survival
  shaping; league = current agent + snapshots + exploiters + scripted
  bots, PFSP (`train/league.py`); decks × seats sampled per episode.
- **The seats' context** (owner with Tom: "a better view of
  the opponents' action history: what they did, what they're capable of …
  as part of a context vector", the position for the policy too): every
  observation carries `Obs::ctx` (`zvtes_env::context`), for each seat from
  the viewer's, clockwise: its position (pool, VPs, ready and unlocked
  minions and their blood, capacity and equipment, torpor, hand and library
  sizes, the edge, the lunge both ways), the Disciplines of its ready
  vampires (summed levels, a fixed order), and its play over the whole game
  counted per round (`Engine::profiles`, `zvtes::profile`: the public
  choices by kind — cards played by type, bleeds, hunts, political
  actions, rushes, builds, blocks, strikes, stealth and intercept shown —
  and the outcomes all see: actions declared, blocks that land,
  hindrances, pool bled) — what the history's last 32 choices cannot hold.
  Public only (the legal view's encoding equals the full state's,
  `crates/env/tests/obs.rs`). The policy and the critic add it to the
  context token through a layer started at zero (`train/model.py`,
  `crates/policy`'s `Net`, `ctx.weight` in a `.zvp`): a network saved
  before it plays unchanged, and its first resume starts the optimizer's
  moments afresh. Laid out feature by feature, each across the seats
  (`context::names`, owner): a feature is appended at the end, never
  inserted (`crates/env/tests/context.rs` pins the names), so an older
  layer reads it as zero columns at the end (`remap_vocab`, `Net::load`).

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

Scale of the V5 pool (a regex census, indicative): prevention ~30 cards,
polling ~47, press 18, additional strike 11, events 20, cancel-as-played
9, wakes 5, bleed redirect 5; torpor/burn replacement and bidding **0**.

**Stress set** (the framework had to express these; all defined):
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
new ones in `tests/rulings.rs`) or `n/a:<reason>` (not engine-observable, or the
card is only the ruling's example); never `blocked:`, a gap a ruling names being
built with its card. Rows on unimplemented vampires have no status yet.
`scripts/rulings_v5.py --check data/rulings-v5.csv` verifies the ledger against
the tests offline; regeneration keeps the statuses of unchanged rows and drops
reworded ones as stale. The invariant is the ruling's assertion: a script is
corrected as a script (fixture, window order), but engine behavior is never bent
to a script — a real divergence is fixed in the engine. Scenario tests are
decision scripts using KRCG card names.

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
deeper: weekly at `main`'s tip, with the host's `HOST_SEEDS`
(`scripts/sweeps.py`, 600 and 400 seeds; Z-main's schedule, never beside a
training run), its report in the main checkout's `data/sweeps/`, a failure
through `intake` and its seed kept. The cost is the engine itself (half of it
`GameState::printed_mod` on the late tables), not the harness.

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
Direct Intervention out of turn, as-played cancels.

### 7.3 Golden replays

`tests/golden/<fuzz test>.txt` pins every §7.2 game, checked in the same sweep
(no extra runs): per seed the real decisions, final turn, VPs and state hash,
and a checkpoint every 64 decisions — the low 32 bits of an FNV fold of each
decision (window name, seat, option count, choice) and of the state hash at the
checkpoint. A mismatch names the test, the first diverging seed (and how many
differ) and the 64-decision range of its first divergence, with the command
tracing it (`FUZZ_TRACE=1 SEED_FROM=n`, decisions numbered). An intended change
re-blesses: `GOLDEN_BLESS=1 cargo test --profile ci --test fuzz`; a missing
file fails, a partial sweep (`SEED_FROM`) skips the check. Identical in every
profile; the state hash is stable on 64-bit little-endian targets (lengths hash
as `usize`).

### 7.4 Coverage checklist

Every mechanic answers "where is it handled, where is it tested" — docs/engine.md
§6 is that map. The in-play hook vocabulary is closed: every hook has
tenants (the `Hook` enum's docs, docs/engine.md §6.11).

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

- **The contract**: `crates/api` (serde only, no engine; semver,
  `docs/protocol.md`): JSON messages per seat over one WebSocket a game.
  Served by `crates/server` on `engine.krcg.org`, the only host: the
  server is authoritative (hidden information stays hidden, every game
  logged, one engine version for everyone; `docs/online-plan.md`). A
  platform plays through it, the server enforcing the rules; the
  Succubus Club fork is one such client (`docs/platforms/sc.md`).
- **Human ergonomics layer** (`crates/host`, not in the core): what a
  human needs beyond the decision protocol — undo by log truncation and
  resimulation, the opportunities a player turns off; the
  play-procedures document is the spec. `crates/text`: the optional
  English renderer, over the api only.
- **Bots**: server-side, the network's weights (`.zvp`, `train/export.py`)
  run on CPU by `crates/policy`, the scripted bots beside them.

## 9. Performance targets

- Engine step: µs-scale; a full 4p random-legal game in low single-digit ms.
- Self-play: thousands of concurrent games per box, NN inference the bottleneck.
- Full determinism under fixed `(seed, answers)` across platforms.
- Measured (`cargo run -p zero-vtes-engine --release --example perf`, the
  fastest of 5 batches; Apple M3, engine 8fcaaa3, 2026-10-25): a random
  4-seat precon game about 5.3 ms (92 turns, most to the 100-turn limit,
  520 decisions), 10 µs a decision: just above the game's low
  single-digit target; on a machine busy with other builds, its efficiency
  cores, up to 14 ms.

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
4. **Deferred windows**: burn replacement, bidding and torpor replacement
   outside combat — nothing in the pool opens them (§3).
5. **Fixed decklists**: the training decks of §3; deck building is out of v1.
6. **Tournament scoring as reward**: VPs + GW. Time limit `TURN_LIMIT` = 100
   player turns (owner), checked at the turn boundary, 0.5 VP per
   survivor; training and evaluation tables draw 60–90 per game
   (owner; `Engine::time_limit`); VPs stored in half-points
   (`Seat.half_vps`); `GameState::game_win` = 2+ VP and a strict lead,
   ousted or not (a tie is a draw). Tournament withdrawal: 0.5 VP, an
   unlock-phase decision (rules-qa 95); network seats never take it
   (`zvtes_policy::untrained`, in training and served), scripted styles
   score it under Pass, `random` plays any legal option.
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

## 11. Phasing

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
| P3 · 14a–14c | 09-27 | Observations (`obs.rs`); `crates/py`'s `GamePool`; `train/`: the pointer-scoring transformer, PPO self-play, evaluation |
| P4 · 16c | 09-30 | Remaining-deck tokens (`obs::T_DECK`), referendum tokens (`obs::T_REFERENDUM`) |
| P4 · 16b | 09-28 | The decision point before an oust (`ousting.rs`): Life Boon |
| P4 · 16n | 10-11 | TWDA legacy cards, round 2: damage reduction, a corruption strike |
| P4 · 16o | 10-11 | TWDA legacy cards, round 3: blood cursed vampires, hunt bonuses |
| P4 · 16p | 10-11 | Owner's list, round 4: a printed +1 hunt, blood hunt immunity |
| P4 · 16q | 10-11 | Owner's list, round 5: an action canceled before resolution |
| P4 · 16r | 10-11 | Owner's list, round 6: a combat queued as one would end (Psyche!) |
| P4 · 16s | 10-12 | TWDA legacy cards, round 7: the Laibon sect, scarce and sterile |
| P4 · 16t | 10-12 | TWDA legacy cards, round 8: Blood Brother circles |
| P4 · 16u | 10-12 | TWDA legacy cards, round 9: Shadow Play, merged advanced vampires |
| P4 · 16v | 10-13 | TWDA legacy cards, round 10: a second master phase action |
| P4 · 16w | 10-14 | TWDA legacy cards, round 11: allies kept for a pool cost |
| P4 · 16x | 10-14 | TWDA legacy cards, round 12: lasting minion bonuses |
| P4 · 16y | 10-14 | TWDA legacy cards, round 13: First Blood, Sabbat Preconstructed |
| P4 · 16z | 10-16 | TWDA legacy cards, round 14: Danse Macabre, Laibon titles |
| P4 · 16aa | 10-17 | TWDA legacy cards, round 15: Seraph, Dual Form |
| P4 · 16ab | 10-18 | TWDA legacy cards, round 16: a going-to-torpor combat step |
| P4 · 16ac | 10-18 | TWDA legacy cards, round 17: the 1996–2010 promos, Flight |
| P4 · 16ad | 10-19 | Legacy cards, round 18: the cards players asked for |
| P4 · 16ae | 10-22 | Legacy cards, round 19: the Imbued, their powers and convictions |
| P4 · 16af | 10-23 | Legacy cards, round 20: the cards players asked for, the two-player list's last two |
| P4 · 16k | 10-09 | TWDA steps 4–9: Become Anarch, general first strike, infernal |
| P4 · 16l | 10-09 | TWDA steps 10–15: Vampiric Disease, Trap |
| P4 · 16m | 10-09 | TWDA steps 16–20: a name's second version, the Great Beast |
| P4 · 16j | 10-08 | TWDA staples and steps 1–3: Anarch Convert |
| P4 · 16i | 10-07 | Sabbat V5 Path of Death and the Soul: zombie allies, aims |
| P4 · 16h | 10-07 | Sabbat V5 Path of Caine: a political action's would-be-success |
| P4 · 16g | 10-07 | Sabbat V5 Path of Cathari: a combat queued in an action |
| P4 · 16f | 10-07 | Sabbat V5 Path of Power and the Inner Voice: Sabbat titles, paths |
| P4 · 16e | 10-06 | The Endless Dance: Business Pressure's pool-for-votes round |
| P4 · 16d | 10-06 | Reign of Stanislava: archbishops, Inner Circle, Enkil Cog |
| P4 · 16a | 09-28 | Legacy cards for TWDA decks (`ADDED_CARDS`), first strike |
| P4 · 15a–15c | 09-28 | Several networks at one table, matchmaking, 4–5 seats; the league |
| P5 · 17a | 09-29 | Human-play protocol (its crates since replaced by `crates/api` and `crates/host`) |
| P5 · 17b | 09-30 | The Succubus Club fork's engine game type |
| P5 · 17c | 09-30 | The policy outside Python (`crates/policy`, `.zvp` weights) |
| P5 · 17d | 09-30 | The owner's first game's fixes (rules-qa Batch 46) |
| P5 · 17e | 10-02 | Narration of what happened since the last message (`narrate.rs`) |
| P5 · 17f | 10-02 | Additional strikes only after the first strike pair (rules-qa 47.2) |

### 11.2 Policy surface and training runs

The engine reference by mechanism is [`docs/engine.md`](docs/engine.md) (owner,
2026-10-14: the engine guide for implementers and reviewers). What stays here
is the policy side: the bots, the observations, the training, the bot layer
around the network, the measures, what measurement settled, and one entry a
run. The full run journal to p17, cut on 2026-11-09: `git show
47bf76e:DESIGN.md`, §11.2.

**Bots: agents and scripted styles** (`agent.rs`: `Agent`, `RandomLegal`,
`play_out`; `zvtes_env::bots`: `Scripted`, `Style`)
- `Agent::decide(&Engine, &DecisionRequest)`: a policy may read the engine, but
  only what its seat's legal view shows. `crates/env/tests/bots.rs` answers
  every 3rd decision of bot tables again from `legal_view` (a bot reads the
  action's `seat`, `acting`, `kind`, `bleed_target`, `directed_seats`, the
  referendum's `caller`; never `ActionCtx::card`/`exchange`, which name a card
  from hand and are not redacted). `Agent::fork`: a copy of the agent as it
  stands, for an undo against bots (the host's `Session::solo`).
- The scripted styles score the options of the windows that define a style;
  everything else scores 0, ties broken by a seeded draw ("random with a
  spine"). Deck-agnostic. Common to all: master cards played rather than
  passing; contests paid; influence brings out every funded vampire, transfers
  onto the slot nearest funding while under a vampire target (3; wall 4) and
  above a pool floor (6; wall 8), draws from the crypt with at most one
  uncontrolled vampire left, never withdraws; hunt when empty or under a third
  of capacity (wall: half); striking or preventing combat cards over other
  combat cards over the hand strike; the discard phase discards a card no
  controlled minion could play; own referendums voted for (Edge and political
  cards burned for them), everyone else's against; political actions taken.
  Referendum terms: damage at the prey when it may be named, else the
  predator; gains to the caller; minions to banish, expel or lock among the
  prey's and the predator's at random; titles on the caller's own; the clan
  with most of the caller's vampires (owner, 2026-09-27). A failed block
  attempt is not repeated with the same minion in the same action; an attempt
  that cannot reach the action's stealth (`Engine::block_can_reach`) is offered
  (rules-qa 73.8) but scores below the pass (owner, 2026-10-15).
- Aggro-bleed: bleeds with every minion (the biggest first), plays bleed
  modifiers, blocks only actions directed at it. Wall: keeps one ready unlocked
  minion home (other actions only with two or more), blocks everything, plays
  intercept. No vote style (owner, 2026-09-27). Against three random seats
  aggro-bleed wins 94% of games, wall 83%; two of each at one table, aggro 1.51
  VPs a seat, wall 0.49; 2–5 ms a game. Too weak to train against or evaluate
  by since p9 (owner, 2026-10-10).

**Observations** (`zvtes_env::obs`: `encode`, `Obs`, `schema_json`)
- One request as its seat sees it: tokens — seats in relative order (pool, VPs,
  Edge, zone sizes, prey/predator, phase budgets), minions in play (contested
  ones too), the viewer's virtual hand (`Choice::Play` indexing), cards in play
  (host as a pointer), uncontrolled slots (the viewer's face up, others'
  `HIDDEN` but a banished one), what remains of the viewer's own library and
  crypt (a token per card with its copies, never the order), the political
  referendum under way (`T_REFERENDUM`: its card, the votes cast so far, a
  chosen clan, its terms on the tokens they name), a last context token
  (window; action kind, acting minion, bleed target, stealth, bleed amount;
  combat minions, range, round; referendum caller, calling minion or
  diablerist). Each token: ids `[type, vocabulary, pointer, pointer]` and
  `TOK_F` floats. Options: ids `[choice kind, vocabulary, pointer ×3]` (hand
  card, actor, target; the source of an ability; the minion, seat, card or slot
  named) and `OPT_F` floats (amount, favor, mode, zone, title contest, action
  kind). A card or minion an option names carries its identity too.
  Vocabulary: crypt, library, hidden, windows, clans.
- `crates/env/tests/obs.rs`: the encoding of the full state equals the encoding
  from the seat's legal view (every 2nd decision of bot, random and fuzz-table
  games; a planted leak of face-down vampires fails it), and every pointer
  names a token. Ash heaps, removed cards and cards stored on cards are sizes
  or tokens of public identity; `ActionCtx::card` is never read.
- Versions (`obs::Version`, a network's `obs`): a network reads the encoding it
  was trained on, the pool each network's (`net_obs`), the critic the
  learner's; a `.zvp` carries `obs N` (absent: `V1`) and `crates/policy`
  encodes with the file's version, so a new version needs a server built with
  it; `crates/env/tests/obs.rs` checks each version is the one before plus
  (`V3`: less) its tokens. `V2` (owner, 2026-10-03: ground the decisions in
  what may be in hand) adds `T_ASH`, a token per seat and card in its ash heap,
  and `T_EVENT + kind`, the last `HISTORY` = 32 public choices
  (`Engine::history`), each as the option it was, with the chooser's relative
  seat, its age in choices and turns, and two pointers to tokens still
  present. Its trust boundary is `game::public_choice`, a whitelist the owner
  may veto an entry of: plays and what follows them face up; never a Pass, a
  search, a draw, a card put under the library or taken from a hidden zone, X
  choices or slot picks. `V3` keeps `V2`'s content in fewer tokens ("Faster
  iterations"): the viewer's library and crypt embedded apart and pooled into
  the context token, others' face-down uncontrolled vampires as counts, the
  transfers left on the context token (76 tokens a decision, p90 101). `V4`
  adds `T_REVEAL`, the last 8 cards revealed to all (`Engine::reveals`), what
  the host tells every seat (`revealed`); it is `T_EVENT` itself, so no table
  grows.

**Training** (`crates/py`: `zvtes_py.GamePool`, `replay`, `schema`,
`deck_names`, `deck_families`, `vocab_keys`; `train/`: `model.py`, `ppo.py`,
`league.py`)
- `GamePool(n, seats, seed, rotate, decks, budget, keep_logs, tables,
  time_limits)`: N games, each waiting on one network's decision; seats are
  "net:k" (a network answered from Python, 0 the learner), "random", "aggro"
  or "wall" (answered inside); decks drawn per seat from `decks`. `queue(specs)`
  replaces the tables new games take (4 or 5 seats, or 2, the two-player
  variant). `observe()` pads the batch (an `agent` column: the network to ask);
  `act(actions)` advances every game in parallel (rayon) and returns the games
  finished (seed, time limit, decks, seats, half-VPs, turns, truncated, log).
  Each game draws its time limit in 60–90 player turns (owner, 2026-09-28): 0.5
  VP per survivor, counting toward the game win (2 VP + 0.5 with a strict
  lead). `replay(seed, decks, log, time_limit)`. A game over `budget` decisions
  is cut (§11.3); a game the engine panics in ends as a truncated `panic`
  episode named on stderr with what replays it, its chains dropped (`panics`
  in the rollout's statistics).
- Vocabulary keys (`obs::vocab_keys`: `crypt:<KRCG id>`, `library:<KRCG id>`,
  `window:<k>`, `clan:<name>`): checkpoints record them; loading under another
  vocabulary moves each embedding row to its key's new id, new keys fresh
  (`model.remap_vocab`; the optimizer's moments dropped) — a card round keeps
  trained networks.
- Model: token embedding (vocabulary + type + features + its pointers' bases),
  transformer trunk (d 96, 2 layers since the start); each option scored from
  its kind, identity, features and the tokens it points at, with the context
  token; the seats' context (§5) added to the context token through a layer
  started at zero.
- PPO: a trained seat's decisions form its chain, rewarded at the end by (VPs +
  `gw_weight` × game win) / 5 (`gw_weight` 5, owner 2026-10-10: an oust then
  being ousted should pay little, a win much); γ 1, λ 0.95; truncated rollouts
  bootstrap from a chain's last decision. Entropy bonus adaptive toward
  `entropy_target` 0.65 (×1.1 or ÷1.1 after each update, within [0.001, 0.2],
  kept in the checkpoint): every early run peaked near 0.64 and declined under
  ~0.6. Minibatch 1024, ragged option sets, bucketed by token count; batch
  shapes rounded (rows to 64, tokens to 16, options to 8, padding masked) and
  minibatches chunked within `attention_budget` (rows × tokens²) for MPS's
  per-shape graph cache.
- The omniscient critic (`critic`; `model.Critic`, `obs::encode_critic`): the
  learner's values come from a separate network reading `encode`'s tokens plus
  every other hand and every face-down uncontrolled vampire, trained on the
  same returns (its own optimizer); the policy plays from the legal view only.
  A new critic, or a change of `obs` or `gw_weight` on resume, trains alone
  for `critic_warmup` (10) iterations, the policy frozen.
- The league (`--league true`, `train/league.py`): each table 4 or 5 seats, one
  the learner's, with `--exploiter true` one the exploiter's (a second trained
  network playing only the learner's tables, frozen into the pool and
  restarted from the learner's weights once the learner beats it under
  `exploiter_beaten` or after `exploiter_reset` iterations; at its own rate,
  `--exploiter-lr`, and on one deck, the next of `--exploiter-decks` at each
  restart, its retired copies keeping it, but at a table begun from a human
  game, which keeps that game's decks), the others by PFSP (weight (1 −
  p)² + 0.02, p the learner's beat rate against that seat) among the bots and
  the learner's last snapshots — or, with `--opponents a.pt,b.pt`, frozen
  checkpoints only (owner, 2026-10-03: "the opponents should be good enough to
  learn from"; each reads its own encoding), with the exploiter's retired
  copies at the training tables (never the evaluations', `vs_held` included).
  The learning rate tapers to `lr_final` × `lr` at the last iteration.
  Evaluation every `eval_every` (20): the learner at every seat against every
  order of the opponents (`eval_tables`: a fixed seating once made fake
  plateaus and fake gains), all games at once, `vs_frozen4`/`vs_frozen5`
  (VPs + `gw_weight` × wins, averaged); `best.pt` moves only when a candidate
  within `best_margin` (0.05) of the best is beaten again on `best_seeds` − 1
  more seeds.
  Game-win benchmarks at a 5-seat table (owner): over 16% very good, over 30%
  very strong, near 50% top level.
- Imitation (AlphaStar's recipe: RL leads, the human term guides): `--bc`
  over the records, `--bc-coef` 0.02 (0.1 moved the policy too much),
  `--bc-every 4` (its term on every 4th minibatch at 4× the weight); only the
  human who won a game played out (owner, 2026-10-15), among the options they
  were shown (a record's `shown`, read from the entry each decision took);
  held-out games named with `--bc-held-games` (out of imitation and starts);
  `--bc-weak-weight`, `--bc-games-weight` weights by deck and by game. Each
  iteration logs `bc_nll`, `bc_val_nll/acc` (held out) and `bc_fit_nll/acc`.
- Starts (`zvtes_host::start`, `GamePool(replays=)`): a training table begun
  from a decision in the last four fifths of a human game (`--starts`,
  `--start-tables`), every library and crypt shuffled again at the branch,
  hands, the uncontrolled region and the table kept, the time limit at least
  the position's turn + 20; `--start-winner` puts the learner in the winner's
  seat of a human win (p17). The trainer reads players' games only (played
  out, a human seat, not a test client's; owner, 2026-10-29).
- `--deck` pins the learner's deck at its tables and evaluations, over the
  weak-deck routing (the exploitability probe's).
- **Human games as data** (owner, 2026-10-03: "use the game log as a teaching
  vector"): a record replays exactly, every decision as (observation,
  options, choice) for any seat (`zvtes_py.replay_decisions`,
  `crates/py/src/human.rs`), by meaning (a decision entry's `choice`, its
  option key, not its index, so an option change does not stop a game; a rule
  change the game met does). `uv run python -m train.assess <replay.json>
  [checkpoint]` shows the human's choices the policy finds least likely. The
  corpus: `data/records/` (curated, committed, README) and the harvest
  (`data/server-records/`, ignored); a record from protocol 2.4.0 on carries a
  position digest and stops at the first divergence.

**The bot layer** (agent behaviour around the forward pass, applied alike by
the training pool, `NetAgent` (the server) and the examples; every option
stays offered and legal, rules-qa 17.4)
- **Default politics** (`zvtes_env::politics`; owner, 2026-10-04, after p4-vote:
  terms a coin flip, votes against its own interest; rewards could not teach
  it, a few decisions a game scored at its end): terms moving pool by rule
  (damage to the prey, then the predator, the caller only when forced; gains
  to the caller); votes masked to the side the terms put a Methuselah on, Pass
  masked while a free right-side vote is left; Banishment and Expulsion on a
  neighbour's minion (the network picks which); Revolutionary Council on the
  prey's pool and the others' locations; Disputed Territory for the caller;
  titles on the caller's biggest vampire; Consanguineous Boon naming the
  caller's main clan. Other referendums stay the network's.
  `tests/default_politics.rs` checks terms and masks over random games.
  Permanent (owner, 2026-11-09: no longer to be relaxed).
- **Chores** (`zvtes_policy::chores`; owner, 2026-10-08: "options a human is
  allowed to forget, but it's never good to do so"): Pass masked when the
  Edge's pool or a hunting ground's free blood is offered; with the Sabbat
  slices, the optional effects the rulings call never worse (Fever Pitch,
  Cursed Abattoir's counter). No measurable strength lost.
- **Nudges** (`zvtes_policy::Nudges`, the pool's `opt_bias`): a bias on every
  network seat's logits inside the distribution, sampled and trained alike
  (each row keeps its bias): `rescue` +6 (its own vampire's rescue), `bleed` −4
  (a bleed for 0), `idle` −3 (ending influence with a crypt draw offered and 5+
  pool), `draw` +7 (that crypt draw), `stuck` −6 (a transfer onto a copy of a
  unique vampire it cannot bring out), `nightmare` −6 (Week of Nightmares'
  counters onto its own minions); `pass` and `reaction` left to imitation.
  Permanent since p9: faded in p8, every behaviour went back. None costs
  measurable strength (`examples/strength.rs`, 512 games each). The served
  profile (temperature 0.5 and these nudges) is
  `zvtes_policy::profiles::NETWORK`; 0.5 against 1 is worth ~+0.17 VP a seat
  (the decisive bot).
- **Futile blocks masked** (owner, 2026-10-15):
  `zvtes_policy::reachable_blocks`, the outermost mask of `NetAgent` and the
  pool: a network never attempts a block that cannot reach the stealth (no
  network told futile from reachable, 61% taken), a window of only futile
  attempts passes without asking it.
- **Opponents that punish** (`zvtes_env::habits`; owner, 2026-10-04: the bots
  tap out, keep no blocker, never rescue or diablerize, and no opponent at
  their tables punishes it): a `net:k+habits` frozen seat rescues its own
  vampire, burns another's card as the Ⓓ action it grants, diablerizes
  another's vampire in torpor, keeps its last ready minion home
  (`--habit-seats`, `--habits`; `vs_habits4/5`). Forced on the learner they
  cost strength (all four −0.57 VP a seat; rescue alone nothing); no
  run's entry names them.

**Faster iterations** (owner, 2026-10-05: "aim for a 2 to 5 times speed
improvement total … to make this project viable"). In place: evaluations play
all their games at once (`refill=False`), every 20 iterations; observation
`V3`; `learner_seats` 1; `--bc-every 4`; no pairing. The rollout is mostly the
networks' forwards (the frozen opponents ~45% of it), the engine ~10%; the
update is GPU-bound (backward ~40%). On the M6 (32 GB, from 2026-11-08), PPO
from p17 without evaluations: ~62 s an iteration (rollout 25 s, update 36 s,
~14,600 samples) after ~10 warm-up iterations (Z-train, 2026-11-09); half
precision, no faster on the M3, is not measured on the M6. Memory (d16 at d
192, 4 layers): 11–12 GB in rollout, 14–16 GB in the update, no leak; the
update's extra is the layers' activations, which `attention_budget` does not
count.

**The measures** (the training skill's steps 4 and 5 run them; ± two standard
errors)
- `crates/policy/examples/waste.rs` (owner, 2026-10-05: "it plays cards for
  the sake of it … cannot construct its hand"): per group (humans, bots by
  profile and weights; record paths or self-play), the counters — rescues,
  bleeds for nothing, stuck transfers, own nightmares, wakes and unlocks with
  nothing after, cycling, crypt draws, idle influence, Wider View, block
  windows (reachable, futile), rushes and their targets, combats — and the
  impact lines. `--by-deck` splits each group by deck (**Combat play by
  deck**, owner 2026-10-20: the rush decks seldom rush and their combats seldom
  put the opposing minion down).
- **The impact ledger** (owner, 2026-10-25: "more actions and more impactful
  meaningful actions … missing opportunities in many places"; runs are judged
  by it, not by single counters): A&B a round — the seat's actions declared,
  blocks that landed and hindrances (another's action canceled, failed or
  unblocked-ended, a bleed bounced or cut to nothing), one a round whatever its
  minions' number; R = 2 × pool + the blood on its minions, its own, its prey's
  and its predator's change over its own turns; the hand (`--cards`): slots and
  dead slots a round, plays and cycles a round held. A measure, not a reward
  (p15). A&B rises with winning (seats that win 3.6 a round, the others 1.9):
  a yardstick, not the lever.
- `crates/policy/examples/decks.rs`: per-deck strength, and a network at one
  seat against three of another (its last line the VP a seat and game wins);
  `--file` writes the per-deck file (below). `examples/strength.rs`: one
  network's halves against each other, a rule or nudge on one (±0.1 VP is
  noise at 256 games).
- `crates/policy/examples/humanset.rs`: on human wins never trained on, the
  network's agreement with the human's choices, and from a quarter, half and
  three quarters of each game the network in the human's seat against the
  served bots, its wins (20 games a position) — the human-facing measure.
  `--apart` scores the held-out five apart from the newer wins; `--map`
  breaks the agreement down by window and kind of the human's choice, with
  the share under 1% ("Settled by measurement").
- **The exploitability probe** (owner, 2026-11-09): a learner from p17 on one
  deck (`--deck`) against p17 frozen at every other seat, no imitation or
  starts, p17's nudges and served temperature, `--lr 1e-4 --lr-final 1`; its
  `vs_frozen4/5` (512 games each) after 100 iterations, against its seed's
  baseline (`--lr 0`, one iteration). On Nosferatu, two seeds (2026-11-10,
  `066f47c`), VP a seat as the mean of 4 and 5 seats: p17 1.22, plain PPO
  1.33 (+0.10 ± 0.10, both seeds up). It shows exploitation; not step 4 of
  the training skill.
- **The critic against the position** (owner with Tom, 2026-10-22; `uv run
  python -m train.position`; `crates/env/src/position.rs`): the explained
  variance of each seat's result by the decision's share of the game, for the
  critic and a linear fit of 26 position features (pools of the seat, its prey,
  predator and cross-table, VPs, ready, unlocked and torpid minions and their
  blood, hand, library, turn, Edge, the lunge). p14 at training's settings, by
  quarter: the critic 0.16 / 0.21 / 0.39 / 0.72 (3,000 games), the features
  0.01 / 0.09 / 0.26 / 0.58, the deck alone ≤ 0.11; on the human records the
  critic is worse early (−0.23 / −0.12 / 0.10 / 0.48). The critic reads more
  than the features at every stage; at a pool's defaults (no politics, chores
  or nudges, which no network trained on) it reads far worse in the first
  half (0.02 / 0.11). `--phi` writes the features' fit as Φ
  (`runs/phi-p14.json`, p15's shaping, dropped).
- The harvest report (`scripts/games_report.py`, `reports/total.md`): the
  humans' win rate and VP a seat against each served weights, the bots' waste
  and ledger at human tables.
- **Bots on any deck** (owner, 2026-10-26, via Z-succubus: humans teach the bot
  the recent TWDA decks it plays worst; served at `GET /catalog/taught`,
  protocol 2.22.0). `decks.rs --file <served.zvp> <games a deck>`: every TWDA
  deck the engine keeps at one seat, precons (V5, anniversary, Sabbat) at the
  three others, none sharing more than 20% of their crypt, every seat the
  served bot (`NETWORK`), games ending at 75 player turns. Written beside the
  weights as `<served>.decks.json` (a `zvtes_api::DecksMeasure`: their SHA-256
  — the server serves the file only while it names the weights served — the
  engine, the day, the measure in a line, and per deck, weakest first, its
  catalog name, TWDA id, event, date, `recent` (within two years), games, game
  wins, VP a game and VP share, 0.25 even). It also sets the fork's bot pool
  (owner, 2026-11-08: beside the precons, the decks dated within six months
  with a VP share of at least 0.15; a deck the file lacks stays out until
  measured), hence the re-measure when the kept TWDA decks gain one. p17's
  (2026-11-11 at `b1de838`, 216 decks, 100 games a deck, ~5 h on a shared
  machine): a TWDA deck at one seat wins 15.1% and takes a VP share of 0.18,
  the decks from 0.005 to 0.71, 75 under 0.10; a deck's win rate at 100 games
  carries about ± 6 points.

**Settled by measurement** (one line each; the entry or run named holds the
numbers)
- Self-play alone does not reach the scripted bots; the omniscient critic gave
  the first steady climb (the critic run).
- A falling entropy takes the strength with it; hence the adaptive bonus
  (p4-twda).
- Voting well paid nothing at self-play tables: politics by rule (p4-vote).
- Beaten bots stop teaching: frozen opponents of the network's own line
  (p5-self, p6-frozen).
- An exploiter with the learner's reward and weights never led the learner
  (the league runs); a learner from p14 against p14 alone stayed level for 300
  iterations (p17). On one deck at a learning rate of 1e-4, plain PPO gains
  ~0.1 VP on its frozen self in 100 iterations, at two errors (the
  exploitability probe).
- An exploration share (`066f47c`: a share of the decisions offering the
  collapsed kinds drawn from the policy flattened over them, PPO's ratio
  against that mixture) loses that gain, −0.15 VP at two errors (the probe,
  share 0.1): a collapsed option it tries sits under the clip's floor and
  learns almost nothing. Dropped (owner, 2026-11-11).
- The network departs from the human winners mostly on the turn's own
  choices, barely on blocks' and combat's passes: p17 gives the human's
  choice under 1% on 49 imitated wins (and 3 held out) for 26.2% (100%) of
  the influence phase's crypt draws, 21.7% (61.5%) of the minion phase's
  abilities, 11.2% (25.0%) of its actions, 7.5% (18.4%) of its plays, 5.3%
  (12.7%) of transfers; blocks' passes 1.4% (6.2%) (`humanset --map`,
  2026-11-10).
- An option the network gives ~0 probability is never sampled, so never
  learned (the crypt draw: a median 6e-8 where the owner drew 11%); faded
  nudges let every nudged behaviour return (p8).
- The weak-deck routing (`--weak-rate`, the learner's seat on the weak decks)
  moves the decks round, not up (p10); not used from 2026-11-09
  (`--weak-decks` still names the decks `--bc-weak-weight` weights).
- A second learner across the table taught no cross-table restraint (p11, p12).
- Imitation of winners only holds the owner's style; imitation memorises (0.89
  on trained choices, 0.52 held out, the held-out loss rising through every
  run from p10 to p16).
- A potential-based shaping by the position cut the rushes (p15): no shaping.
- A larger network distilled from p14 plays p14's game, no gain, nor in RL
  from it (d16, p16).
- Search in the minion phase on legal views: +0.10 ± 0.17 VP, 10 CPU-seconds a
  decision; dropped (owner, 2026-11-02).
- Rules laid on p14 (block whenever reachable, keep a minion home) cost 0.10 to
  0.22 VP a seat at bot tables.
- Starts from human wins, the learner in the winner's seat: +5 points on the
  human positions, level bot against bot (p17).
- Bot-against-bot strength is flat since p10 (each run against its
  predecessor 0.88 to 1.03 VP a seat): a run's rising score is recovery from
  its first evaluation's dip. Of the gradient, ~10% is usable, most of it
  "more decisive", which the served temperature already cashes; the game's
  result enters none of the first quarter's advantages.

**Where the bots stand** (2026-11-09, the owner's review). Humans (mostly the
owner) win 79% of their games against p14 (39 games, 2.79 VP a seat; a fair
share 25%), 6 of 9 against p17 by 2026-11-11; over 80 counted games the bots
won 5 of 240 seats. At p14's tables its ledger: A&B 1.60 a round (the owner
3.03), its prey's R −2.18 a turn (the owner's prey −6.06), its own −2.81, its
predator's −0.34. Order of work (owner, 2026-11-09 and 11-11; the training
board): exploiters, judged on the human-facing measures (roadmap
M:stronger-bot).

**The runs** (one entry a run, oldest first: its launch, the flags it changed,
its numbers, what it settled; the last run's command in full, the next
starting from it; a run is prepared as a "pN, prepared" entry here and becomes
its entry when it ends — the training skill's "A run")
- **Self-play and the league runs** (2026-09-27/28): self-play beats random
  seats, not the scripted bots (0.4–0.7 VP against aggro/wall/aggro);
  the league runs' gains were the fixed seating's noise (hence `eval_tables`);
  the exploiter never led the learner (off since).
- **p4-twda** (2026-09-29): precons and TWDA decks; 1.44 VP at 150, then down
  as the entropy fell: the adaptive bonus.
- **Critic run** (2026-09-29/30): the omniscient critic, 1024 games, 3-seed
  selection; 1.44 → 2.12 VP against the bots, the first steady climb; it never
  rescues nor diablerizes (training removed both).
- **p4-deck**, **p4-vote** (2026-09-30 → 10-01): the remaining-deck and
  referendum tokens, each a higher plateau (2.31, 2.44); votes stayed a coin
  flip. p4-vote@1060 the first human-play bot.
- **p5-self**, **p6-frozen** (2026-10-02 → 04): bots one seat in ten, then the
  frozen opponents and `V2`; flat. MPS memory fixed (shapes rounded, chunks).
- **p7-politics** (2026-10-04 → 08): the default politics; +0.05 VP a 100
  iterations. Its self-play showed the habits no bot punishes.
- **p8-nudges** (2026-10-08 → 10): `V3`, lr 1e-4, the owner's starts 25%,
  nudges faded to 0 at 2600: 1.41 VP against p7; every nudged behaviour went
  back. Served 2026-10-10.
- **p9-imitation** (2026-10-10 → 12): imitation (bc 0.02), nudges permanent,
  the learner on the weak decks 40%; served 2026-10-12.
- **p10** (2026-10-13 → 14): five games held out by name; the weak list moved
  the decks round. Served 2026-10-14 (`7aaa9528fbf8`).
- **p11** (2026-10-14 → 16): `--partner-rate 0.5`; no restraint emerged, Wider
  View learned without a nudge; 0.99 against p10. Served 2026-10-16
  (`b9035931a527`).
- **p12** (2026-10-16 → 17): imitation winners only; 0.88 against p11, not
  served; the pairing off from p13.
- **p13** (2026-10-18 → 19): `--bc-every 4`; 0.97 against p11, better from the
  owner's positions. Served 2026-10-19 (`df66d54badf3`).
- **p14** (2026-10-21 → 22, `10b4369`): the rush and wall decks at the
  learner's seat 35%, their human choices ×3; rushes up only where the deck
  already rushed; 1.00 against p13. Served 2026-10-22 (`a78e74292acb`).
- **p15** (2026-10-23 → 25, `788eae4`): `--shaping runs/phi-p14.json`; 1.03
  against p14, the rushes cut (Brujah 41% → 24% of own turns); not served,
  the shaping dropped.
- **d16** (2026-10-25 → 26, `fed6d40`): d 192, 4 layers, distilled from p14
  (`--teacher`); 0.92 against p14, the human's choices more probable; not
  served.
- **p16** (2026-10-27 → 30, `a4d184d`): RL from d16, `gw_weight` 2 by
  omission; stopped at 478 of 680 (owner: going nowhere); not served.
- **p17** (`runs/p17`, launched at `08c5665`; 2026-11-03 → 04, 5281
  → 5580). Half the tables start from a human's win, the learner in the
  winner's seat (owner, 2026-11-02); p14 the only frozen opponent at the served
  temperature; a flat learning rate 3e-5; held out of imitation and starts, 13
  human wins, judged on the nine p14 never met (owner, 2026-11-03; the list,
  `runs/p17-held.txt`, lost with the machine's change). The command:
  `train.ppo --league true --out runs/p17 --resume runs/p14/best.pt
  --opponents runs/p14/best.pt --served-temperature 0.5 --decks
  precons,anniversary,sabbat,twda --gw-weight 5 --lam 0.95 --lr 3e-5
  --lr-final 1 --entropy-coef 0.033 --bc data/records,data/server-records
  --bc-coef 0.02 --bc-every 4 --bc-held-games H --bc-weak-weight 3
  --bc-games-weight zero-vtes-replay=2 --starts S --start-tables 0.5
  --start-winner true --held-games 208 --keep-evals true --weak-decks
  0,3,7,8,9,12,13 --weak-rate 0.35 --rescue-bonus 6 --bleed-bonus 4
  --idle-bonus 3 --draw-bonus 7 --stuck-bonus 6 --nightmare-bonus 6
  --iterations 5580 --eval-every 20 --best-seeds 1 --bots-every 0`. Level
  against p14 throughout (2.13 against the bar's 2.24). Step 4 on the last
  evaluation (5580, `f426338f6c80`): at one seat against three p14 1.065 ±
  0.09 VP, 25.5% ± 2.8 wins (1,000 games); `humanset` on the nine 45.4%
  against p14's 40.4% (t 2.4 game by game), agreement 49.4% / 48.4%; the
  ledger worse beyond two errors on A&B at 5 seats (2.42 / 2.51) and its
  prey's R at 4 and 5 seats (−3.48 / −3.70, −3.79 / −3.97). The not-degrading
  test failed; served as a trial (owner, 2026-11-06; back to p14 if the human
  games worsen, docs/watch.md), `runs/p17/last.zvp` (`f426338f6c80`), the
  previous `runs/served-prev-2026-11-06-p14.zvp` (`a78e74292acb`).
- **p18** (`runs/p18`, launched 2026-11-11 at `c8d753e`, the engine of
  2.26.1; 5581 → 5880; owner, 2026-11-09 and 11-11: a run against exploiters,
  without the exploration share). Why: plain PPO on one deck at 1e-4 found
  +0.10 VP on p17 frozen in 100 iterations (**The exploitability probe**);
  exploiters that keep finding such gains, each on a deck humans beat the bot
  with, their last 4 retired copies kept at the learner's tables, are what p17
  frozen alone never showed it. The command: p17's, with `--out runs/p18
  --resume runs/p17/best.pt --opponents runs/p17/best.pt --exploiter true
  --exploiter-lr 1e-4 --exploiter-decks 9,5,0,29,13,30 --exploiter-reset 50`
  (the named decks humans won most with against the bots, `reports/total.md`:
  Gangrel, Tremere, Hecata, The Endless Dance, Tzimisce, Path of Power and the
  Inner Voice; at most 50 iterations each, off its deck at the start tables),
  `--weak-rate 0 --weak-decks` with
  `160,87,182,186,111,151,163,179,178,144,133,122,180,173,181,134,124,171,`
  `102,143,110,183,99,112,93,96,98,89,107,136,132,154,158,187,188,91` (the
  36 recent decks under a VP share of 0.10 in `served-p17.decks.json`,
  2026-11-11), `--iterations 5880`, and held out (16): the five, then the
  human wins since p17's launch, `5dba3e6fc049f0b8`, `479c5dc6325e31b8`,
  `af299e69cc626225`, `2bee9103be47dfdc`, `1fdb12a004b14be8`,
  `3f390e5601f7bfd4`, `5af4d2bd173094f6`, `e79edc1df3520df7`,
  `7512b54e87ee02ee`, `40044b3e5a53807d`, `5150436d1548ad14`. Imitation reads
  10,482 decisions, 56 records stopping partway on this engine, none on the
  weak decks (`bc_weak` 0: no human has played them yet, so `--weak-decks`
  weighs nothing in this run). A first launch at `b1de838` was stopped after
  one iteration (its exploiter retired at once, p17's count carried over;
  `c8d753e`). Baseline: p17 (`f426338f6c80`), measured beside p18 by step 4
  on the run's engine.

### 11.3 Open gaps

What the engine does not yet do or does only in part, true today.

- **Inert clauses, no pool tenant**: Blood Trade's boons; The Fourth Cycle's
  conviction cards; self-damaging weapons (rows n/a); Winterlich's "as if
  he met the requirements" (no changeling with one); the [MERGED]
  text of Brunhilde, Danielle Diron, Claudio Severino, Masika, Bulscu and
  Maldavis (their base cards are not in the pool); Veneficti's demon steal
  or burn (no demon); Tupdog's Visceratika reduction (no Visceratika card);
  Orlando Oriundus's bar on other multipliers (no other multiplier);
  Shakar: the Hunt's "considered chosen" (read by no pool card);
  Abdelsobek's mummy (no mummy) and Onaedo's Otieno (not in the pool).
- **Paths in the observations**: the policy sees a vampire's sect, not its
  path, and the networks never follow one (`zvtes_policy::untrained`): the
  training board's "Paths in the observation".
- **Mechanisms missing**: immunity to damage from a source.
- **Known edges**:
  - Waiting for a pool card (none reaches them):
    `ImmuneToDirectedActions` (Aggressive Corpse) is read on card plays only
    (`GameState::immune_to_directed` in `plays.rs`), no granted or printed
    directed action requiring Dominate or Presence existing;
    `Effect::PoolToHostBlood` ousts through `oust_and_pass`, its users (Blood
    Doll's master phase, Vessel's own unlock phase) the active seat; a
    cancelling ability marks the play canceled as it is used
    (`as_played_apply`), none having terms its user could back out of.
  - Decisions are unbounded within an action: `TURN_LIMIT` bounds turns, but
    the rulebook allows block re-attempts as often as wished. A failed
    attempt is not offered again in the same situation (rules-qa 48.6,
    `ActionCtx::attempt_print`), so a repeat needs a change — a card played,
    blood moved; still, a driver with a deterministic policy (the P3 greedy
    evaluation, a human client) keeps a decision budget; the scripted bots
    remember their attempts.
- No printed sterile baron exists, so Childe of the Revolution's sterile
  mask is tested on a sterile vampire made a baron (Effie Lowery).

### 11.4 The phases

- **P2 — full engine pool**: every V5 card defined; legacy cards come in
  through the cards line (§3).
- **P3 — training loop**: built (slice 14), runs judged by the impact
  ledger (§11.2).
- **P4 — league & 5p**: the league, PFSP, 4- and 5-seat tables and seat
  sampling, the exploiter built (slice 15).
- **P5 — community**: online play (§8).

The objectives still open are the roadmap's (`docs/roadmap.md`).

Definitions: 1356 library, 774 vampires

## Outlook: deck building (post-v1)

Once engine + pilot work, deck building is the natural extension: fast simulator
+ competent pilot = the deck-evaluation oracle prior deckbuilding AIs lacked;
pointer-style policy makes new decklists data; engine pool ⊃ training roster.

- **Formulation**: co-evolve a metagame, not "find the best deck" — population /
  quality-diversity search (archetype descriptors as diversity axes), fitness =
  expected tournament score vs the current population.
- **Pilot–deck chicken-and-egg**: unfamiliar archetypes score low through the
  pilot. Mitigations: a deck-conditioned policy (the remaining-deck tokens,
  §5), per-candidate fine-tuning, full co-evolution.
- **Assets**: the TWDA (~4k winning decklists via KRCG) as a generative prior;
  deck legality already solved in krcg.
- **Bridge study — precon tuning**: frozen pilot, ≤10 swaps from a precon,
  hill-climb; exercises the loop, measures pilot bias, and an improved precon
  is a community artifact.

## Glossary

Terms the owner uses, as recorded in rules Q&A:

- **Combo**: a single line of Discipline icons, "[dom][pre] X": every one
  is needed (Iron Glare; KRCG's discipline requirement `Combo`).
- **Choice**: separate lines, "[pre] X / [tem] Y": one Discipline is
  chosen, at one level (Line Brawl). A card printing "more than one
  Discipline can be used" lets any set of them combine, each effect
  applying (Break the Bonds).
- **Outferior**: the inferior-only option of a card that a vampire can use
  without the card's bloodline Discipline (Outside the Hourglass; rules-qa
  25.2).

## Resolved questions

The owner's settled answers; they hold until the owner reopens them.

- **Training decks**: the precons and the decks of §3; the New Blood
  minis are never trained on (half decks); the singles-only cards are
  implemented, in no precon.
- **Polling rotation order**: starts at the acting Methuselah, clockwise; closure
  on a full rotation of consecutive passes guarantees universal response
  opportunity to every commitment (§4.2).
- **Events**: implemented with the rest of the engine pool. Playing one is
  an optional discard-phase action (discard a card, *or* play an event; each event
  once per game per player) — nothing forces an event play, an event in hand can
  always simply be discarded. Hence the non-minion play windows (Master
  phase, Discard phase) are first-class offer windows (§4.4).
- **Contest upkeep factoring**: each Methuselah resolves their side of a
  contest during their own unlock phase, two of their own vampires claiming
  one title included (rules-qa 3.1); per-object binary pay/yield decisions
  in canonical order; forced yields elided (§4.4).
- **Window census vs per-card reality**: the matrix is generated, then
  hand-reviewed card by card and arbitrated by the owner (rules-qa Batch
  2); the generator reproduces the reviewed matrix (§6).

## Process: rules Q&A pass

Rules questions go to the project owner (VEKN judge) in batches organized
by subsystem; the answers are recorded in `docs/rules-qa.md`, each backed
by a scenario test so the engine's behavior and the recorded ruling can
never drift apart.
