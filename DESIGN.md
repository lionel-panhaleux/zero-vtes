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
ledger), and legacy cards making tournament-winning decks (TWDA) playable,
appended after the V5 tables in batches so existing indices stay put
(`ADDED_BATCHES`, merged as `ADDED_CARDS`). The script holds the lists; the
cards line adds them in rounds (the `add-cards` skill).

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
  contest during *their own* unlock phase, so nobody orders both sides and no
  cross-player simultaneity exists (a contest with oneself is masked where
  chosen, the incoming title lost where forced: rules-qa 100.1). Each contested
  object is a binary pay/yield decision (1 pool for a card, 1 blood for a
  contesting titled vampire); forced yields (no resource, torpor) are elided;
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
  the contexts. Owner's calls: the banished slot and contested cards kept
  public; a vampire revealed on its way to the uncontrolled region
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
is the policy side: the scripted bots, the observations and the training runs.

**Agents and scripted bots** (`agent.rs`: `Agent`, `RandomLegal`, `play_out`;
`zvtes_env::bots`: `Scripted`, `Style`)
- `Agent::decide(&Engine, &DecisionRequest)`: a policy may read the engine, but
  only what its seat's legal view shows. `crates/env/tests/bots.rs` answers
  every 3rd decision of bot tables again from `legal_view` (a bot reads the
  action's `seat`, `acting`, `kind`, `bleed_target`, `directed_seats`, the
  referendum's `caller`; never `ActionCtx::card`/`exchange`, which name a card
  from hand and are not redacted).
- `Agent::fork`: a copy of the agent as it stands, for an undo against bots
  (the host's `Session::solo`) to put back with the engine.
- The bots score the options of the windows that define a style; everything
  else scores 0 and ties are broken by a seeded random draw ("random with a
  spine": the difference from `RandomLegal` is exactly these rules).
  Deck-agnostic. Common to all styles: master cards played rather than passing;
  contests paid; influence brings out every funded vampire, transfers onto the
  slot nearest funding while under a vampire target (3; wall 4) and above a
  pool floor (6; wall 8), draws from the crypt with at most one uncontrolled
  vampire left, never withdraws; hunt when empty or under a third of capacity
  (wall: half); combat cards that strike or prevent over other combat cards
  over the hand strike; the discard phase discards a card no controlled minion
  could play; own referendums voted for (the Edge and political cards burned
  for them, vote cards played on the side they add to), everyone else's
  against; political actions taken (over a bleed of 1 for aggro-bleed).
  Referendum terms by what the referendum does: damage (pool burned by
  allocation or choice, a victim of Parity Shift) at the prey when it may be
  named, the predator otherwise; gains (Parity Shift's points, Camarilla's Iron
  Fist's chosen Methuselah, Disputed Territory's taker) to the caller; minions
  to banish, expel or lock among the prey's and the predator's at random; title
  cards on the caller's own; the clan with the most of the caller's vampires
  (owner, 2026-09-27). A failed block attempt is not repeated with the same
  minion in the same action (the rulebook allows it as often as wished: an
  always-block policy loops, so the bot remembers its attempts). An attempt
  that cannot reach the action's stealth (`Engine::block_can_reach`) is offered
  (rules-qa 73.8) but never made: it scores below the pass (owner, 2026-10-15).
- Aggro-bleed: bleeds with every minion (the biggest bleed first), plays bleed
  modifiers, blocks only actions directed at it. Wall: keeps one ready unlocked
  minion home (other actions only with two or more), blocks everything, plays
  intercept. No vote style: no style is assigned to a precon, and political
  actions are part of both (owner, 2026-09-27).
- Measured 2026-09-27 (precons rotated). Random play alone (200 games): 94.7
  turns a game, 1.58 ousts, 75% of games at the time limit. Against three
  random seats (200 games per style): aggro-bleed 94% game wins and 3.75 VPs a
  game, wall 83% and 3.29, the random seats 0.08–0.21. Two of each style at one
  table (480 games): aggro-bleed 1.51 VPs a seat, wall 0.49; 45 turns a game,
  none at the time limit; 2–5 ms a game.

**Observations** (`zvtes_env::obs`: `encode`, `Obs`, `schema_json`)
- One request as its seat sees it: tokens — seats in relative order (pool, VPs,
  Edge, zone sizes, prey/predator, phase budgets), minions in play (contested
  ones too), the viewer's virtual hand (`Choice::Play` indexing), cards in play
  (host as a pointer), uncontrolled slots (the viewer's face up, others'
  `HIDDEN` but a banished one), what remains of the viewer's own library and
  crypt (`T_DECK`, since 2026-09-30: a token per card with its copies, never
  the order — the decklist at the start, the cards left afterwards), the
  political referendum under way (`T_REFERENDUM`, since 2026-09-30: its card —
  the political action card or the card granting the action — and the votes
  cast for and against so far; a chosen clan as a second one; its terms on the
  tokens they name: points allocated on seat and card tokens, chosen
  Methuselahs, minions and cards flagged), a last context token (window; action
  kind, acting minion, bleed target, stealth, bleed amount; combat minions,
  range, round; referendum caller, calling minion or diablerist). Each token:
  ids `[type, vocabulary, pointer, pointer]` and `TOK_F` floats. Options: ids
  `[choice kind, vocabulary, pointer ×3]` (hand card, actor, target; the source
  of an ability; the minion, seat, card or slot named) and `OPT_F` floats
  (amount, favor, mode, zone, title contest, action kind). A card or minion an
  option names carries its identity too (a rider's card may have left play).
  Vocabulary: crypt, library, hidden, windows, clans.
- `crates/env/tests/obs.rs`: the encoding of the full state equals the encoding
  from the seat's legal view (every 2nd decision of bot, random and fuzz-table
  games; a planted leak of face-down vampires fails it), and every pointer
  names a token. At most 63 tokens and 104 options met.
- Review points: ash heaps, removed cards and cards stored on cards are sizes
  or absent, not tokens; `ActionCtx::card` (a card named from hand) is never
  read, by construction only.
- Versions (`obs::Version`, a network's `obs`): a network reads the encoding it
  was trained on, the pool each network's (`net_obs`), the critic the
  learner's; a `.zvp` carries `obs N` (absent: `V1`) and `crates/policy`
  encodes with the file's version, so the server's bot keeps playing and a new
  version needs a server built with it; `crates/env/tests/obs.rs` checks each
  version is the one before plus (`V3`: less) its tokens. `V2` (owner,
  2026-10-03: "more input (past actions, ash heap) … to ground the bot
  decisions with assessment of possible cards in hand") adds, after every token
  a pointer names and before the context token, `T_ASH`, a token per seat and
  card in its ash heap (library and crypt), with its copies, and `T_EVENT +
  kind`, the last `HISTORY` = 32 public choices (`Engine::history`, recorded
  where `choose` and the run loop's elision both pass, the card a hand index
  named resolved then), each as the option it was — a token type per choice
  kind, the chooser's relative seat, its age in choices and game turns, the
  option's features, two pointers (actor else the card or minion it names;
  target) to tokens still present: ~66 → ~136 tokens a decision (p95 164, max
  190). Its trust boundary is `game::public_choice`, a whitelist the owner may
  veto an entry of: plays and what follows them face up (actions, blocks,
  strikes, votes and allocations, contests, chosen seats, minions, cards and
  clans, discards to the ash heap, transfers and influence, rescue and toll
  payments, diablerie); never a Pass, a search, a draw, a card put under the
  library or taken from a hidden zone, X choices or slot picks (`obs.rs`: a
  choice outside it leaves no event; the legal view swap cannot see the
  history, which lives on `Engine`). `V3` lightens it ("Faster iterations"
  below), `V4` adds a token per card revealed to all (`T_REVEAL`, the last 8 of
  `Engine::reveals`: the card, whose library or crypt, how far back), what the
  host tells every seat (`revealed`), Mesu Bedshet's vampire too, its slot
  still face down; `T_REVEAL` is `T_EVENT` itself, which no choice kind takes,
  so no table grows.

**Training** (`crates/py`: `zvtes_py.GamePool`, `replay`, `schema`,
`deck_names`, `deck_families`, `vocab_keys`; `train/`: `model.py`, `ppo.py`,
`league.py`)
- `GamePool(n, seats, seed, rotate, decks, budget, keep_logs, tables,
  time_limits)`: N games, each waiting on one network's decision; seats are
  "net:k" (a network answered from Python, 0 the learner; "policy" = "net:0"),
  "random", "aggro" or "wall" (answered inside); `rotate` shifts seating per
  game; decks drawn per seat from `decks` (default the 14 precons).
  `queue(specs)` replaces the tables new games take (seats, optional decks; 4
  or 5 seats, tournament tables, or 2, the two-player variant), the default
  table once it runs out. `observe()` pads the batch (with an `agent` column:
  the network to ask); `act(actions)` advances every game in parallel (rayon)
  to its next policy decision and returns the games finished (slot, game number
  in start order, seed, time limit, decks, seats, half-VPs, turns, decisions,
  truncated, log). Each game draws its time limit in `time_limits` (60–90
  player turns inclusive, owner 2026-09-28: player turns, so a table losing a
  Methuselah early plays more rounds; `Engine::time_limit`, default
  `TURN_LIMIT` for the engine's own tests): 0.5 VP per survivor, counting
  toward the game win (2 VP + 0.5 wins with a strict lead). `replay(seed,
  decks, log, time_limit)`. A game over `budget` decisions is cut (§11.3).
- Vocabulary keys (`obs::vocab_keys`: `crypt:<KRCG id>`, `library:<KRCG id>`,
  `window:<k>`, `clan:<name>`): checkpoints and snapshot directories
  (`vocab.json`) record them; loading under another vocabulary moves each
  embedding row to its key's new id, new keys initialized fresh
  (`model.remap_vocab`; the optimizer's moments are dropped then) — a pool
  extension keeps trained networks. `train/tests`: batches well formed, games
  replay from their log, only 4- and 5-seat tables, queued tables route rows to
  their networks, one PPO iteration and two league iterations run (the
  exploiter trains; resume keeps the beat rates), beat rates move once per
  iteration and retire the random seat and the exploiter; vocabulary keys match
  the schema and embeddings follow their keys; deck families (minis refused for
  training); TWDA tables train; evaluation seatings cover every opponent order;
  games replay with their drawn time limit.
- ~170k decisions/s (300 random games/s) through Python on 8 cores; the model
  is the bottleneck: a d=96, 2-layer trunk takes ~0.3 s per 1,024 decisions for
  a training step on the M3's GPU (MPS; half precision measured slower, CPU
  slower): an iteration of 16k decisions, 3 epochs, is ~2 s of rollout and
  ~12–15 s of update.
- Model: token embedding (vocabulary + type + features + its pointers' bases),
  transformer trunk; each option scored from its kind, identity, features and
  the tokens it points at, with the context token; value from the context and
  mean tokens. PPO: without the league one network plays every seat of 4- and
  5-seat tables; a trained seat's decisions form its chain, rewarded at the end
  by (VPs + `gw_weight` × game win) / 5 (`gw_weight` 5 since p9 (owner,
  2026-10-10: "penalize doing only 1vp and not winning … if it does not get a
  reward for ousting then dying, but only when it wins in the end": a 3-VP win
  8, an oust then ousted 1; win-only would leave ~70% of the learner's games
  without a gradient), d16 and p16 at 2 by omission; 2 from the second league
  run: tournament ranking puts game wins first; 1 before); entropy bonus 0.02
  (0.01 before; 0.03 made the entropy climb, 0.62 → 0.89 in 30 iterations, with
  every evaluation flat or down), adaptive since p4-twda: scaled ×1.1 or ÷1.1
  after each update toward `entropy_target` 0.65 (every run peaked against the
  bots near 0.64 and declined under ~0.6), within [0.001, 0.2], kept in the
  checkpoint; truncated rollouts bootstrap from a chain's last decision,
  carried to the next rollout. Evaluation samples the learner at every seat
  and, for each, every distinct order of the opponents around it
  (`eval_tables`; before 2026-09-28 the order was fixed and only rotated, so at
  4 seats the wall always sat across the table: the starting checkpoint
  measured 0.60 VP / 12.5% game wins against aggro/wall/aggro that way, 1.09 /
  24% with the orders mixed — the earlier "plateau" was largely the seating),
  and counts exactly the queued games (not the first to finish, which favored
  short games): `vs_random` (four random seats; three before the second league
  run), `vs_bots4` (aggro/wall/aggro, the earlier `vs_bots`), `vs_bots5`
  (aggro/wall/aggro/wall), `vs_snapshot` (four copies of the latest snapshot),
  256 games each (96 before: one snapshot measured 0.44–0.76 VP against
  aggro/wall/aggro over six evaluation seeds). A pool game that reaches an
  empty request panics with its seed, decks and log (the engine's own check is
  a debug assertion) — how training found rules-qa 43.1; and a game the engine
  panics in ends as a truncated `panic` episode named on stderr (seed, decks,
  seats, start, log: it replays), its chains dropped (`Game::advance_caught`;
  `panics` in the rollout's statistics; since p9's Pack Alpha panic). Game-win
  benchmarks at a 5-seat table (owner): over 16% is already very good, over 30%
  very strong, close to 50% top level.
- The league (`--league true`, `train/league.py`; revised after the first
  league run, owner 2026-09-28): each new table is 4 or 5 seats, one the
  learner's (`net:0`), with `--exploiter true` one the exploiter's
  (`net:30000`; off by default since the second league run), the others drawn
  by PFSP among the bots (`bots`, all three by default; at most `table_bots` a
  table) and the last `opponents_kept` (4) snapshots of the learner (saved
  every `snapshot_every` (10) iterations) and of retired exploiters
  (`net:30000+<it>`), in `runs/<name>/snapshots/`: weight (1 − p)² + 0.02, p
  the learner's pairwise beat rate against it (more VPs than that seat, a tie
  ½; from ½, moved once per iteration by `pfsp_rate` (0.1) toward that
  iteration's mean; truncated games not counted). The random seat retires once
  p passes `retire_random` (0.85). The exploiter is a second trained network
  (own optimizer and chains) that plays only the learner's tables; it is frozen
  into the pool and restarted from the learner's weights once the learner's
  beat rate against it falls under `exploiter_beaten` (0.3), or after
  `exploiter_reset` (50) iterations. One forward pass per network present. The
  pool refuses tables other than 2, 4 or 5 seats. 1024 games in parallel since
  2026-09-29 (512 from the third league run, 256 before: one learner seat a
  table halved the samples). An opponent seat is a bot with probability
  `bot_seats` (0.6, since 2026-09-29; before, all bots and snapshots were one
  PFSP pool, ~30% bots), else a snapshot, PFSP within the group. The learning
  rate tapers linearly from the run's first iteration to `lr_final` (0.1) ×
  `lr` at the last; the evaluation with the most VPs against the bots is kept
  as `best.pt` (a checkpoint like `model.pt`, resumable): the mean of
  aggro/wall/aggro and aggro/wall/aggro/wall, a candidate within `best_margin`
  (0.1 VP) of the best measured again on `best_seeds` − 1 (2) more evaluation
  seeds and kept on the mean (since 2026-09-29; a single aggro/wall/aggro
  evaluation before).
- Frozen opponents (owner, 2026-10-03, after p5-self: "the opponents should be
  good enough to learn from" — weaker ones probably worthless for now, skewing
  the training no concern yet; random and wall never as training opponents,
  aggro sparingly if at all; the opponent "the latest best", the network the
  human-play bots play, not the rolling snapshots): `--opponents a.pt,b.pt`
  with `--league true` makes every non-learner seat one of those checkpoints
  (net ids 20000 + k, PFSP among them, labeled `<run>@<iteration>`), reading
  their own encoding (`GamePool(obs=, legacy=)`); no bots, no snapshots.
  `best.pt` is selected on `vs_frozen4`/`vs_frozen5` (the opponents in turn
  around the learner, 3 seeds; from p9 VPs + `gw_weight` × game wins, at 4 and
  5 seats averaged), the scripted bots out of evaluation from p9 too
  (`--bots-every 0`; owner, 2026-10-10: "too bad to matter"). A change of `obs`
  or of `gw_weight` on resume re-warms the critic (`critic_warmup`). Not built:
  promoting the learner's own best into the opponents (gating). Cooperation
  across the table: nothing added (the reward stays VPs and game win).
- The omniscient critic (`critic`, on since 2026-09-29; `model.Critic`,
  `obs::encode_critic`, the pool's `critic=True` adding `critic_tok_*`): the
  learner's values come from a separate network reading `encode`'s tokens plus
  every other hand and every face-down uncontrolled vampire, trained on the
  same returns in the same minibatches (its own optimizer, the same
  learning-rate taper), started as a copy of the policy's encoder and value
  head; the policy still plays from the legal view (its own value head
  untrained since). Saved in checkpoints (`critic`, `critic_optimizer`). A new
  critic trains alone for its first `critic_warmup` (10) iterations, the policy
  frozen: copied from the policy it values a new input poorly, and the first
  critic run fell from 1.36 / 1.51 to 1.20 / 1.24 VP in 10 iterations without
  it.
- Update speed (2026-09-29): the update is GPU-bound (Python collation ~5%);
  options were padded to the batch's largest set (mean 5, max 100+), so only
  the offered options are scored now (ragged, identical logits and parameters),
  and minibatches are bucketed by token count (chunks of 8 minibatches sorted,
  then shuffled: every sample once an epoch). Measured A/B under the same load:
  1.4–1.5× faster; minibatch 1024 is the fastest (512 and 2048 slower, 4096
  exhausts GPU memory).
- Imitation (the plan of 2026-10-10, AlphaStar's recipe: RL leads, the human
  term guides, small weight, watch overfitting): `--bc` (replays, as
  `--starts`), `--bc-coef`, `--bc-batch` (256), held-out games every 8th
  (`--bc-holdout`) or named by parts of file names (`--bc-held-games`, from
  p10; their names logged at launch): each PPO minibatch adds `bc_coef` × the
  cross-entropy of the policy's own logits (no nudge) on 256 of the humans'
  choices with an alternative — since 2026-10-15 (owner) only the human who won
  a game played out (last standing or the time limit), among the options they
  were shown (a record's `shown`, the hidden ones out of the target, read only
  where the replay keeps the recorded option; the replay's decisions pair with
  the record's by position, `made`, since a later engine shifts their numbers);
  each iteration logs `bc_nll` (the update's), `bc_val_nll/acc` (held out) and
  `bc_fit_nll/acc` (as many trained ones). A game exported twice (the same
  spec, one log the other's start) is read once, for starts too
  (`load_replays`). `bc_coef` 0.02 (calibrated from `p8-nudges@2600`: 0.1 moved
  too much, kl 0.032 the first iteration, 0.76 VP against p7 + p8 where p8
  itself scores 1.04; 0.02 0.94 VP, kl 0.010 → 0.001); at either weight the
  entropy rose ~0.15 (0.64 → 0.80), the policy spreading toward the owner's
  choices it gave little weight to. Weak decks: a table spec's `decks` takes
  None for a seat to draw (`GamePool`; the drawn ones still avoid the pinned
  ones' crypts); `--weak-decks` (indices) and `--weak-rate` give the learner's
  seat one of them on that share of the league tables, and in a start the
  learner sits at a seat whose deck (matched to the pool by its list,
  `deck_index`; a human's own list matches none) is weak, when one is
  (`League.spec(n, at)`); `--bc-weak-weight` draws those decks' human choices
  more often (from p14).
- Evaluation against humans: the harvest report's human win rate by weights
  (`scripts/games_report.py`), and the positions where a human beat the bot
  (`examples/humanset.rs`: games a human won, played out and never trained on;
  `--bc-held-games` holds them out of the starts too since 2026-11-02, before
  which p10 to p16 began tables from them, p13 from four of its five (their
  logs' `starts` lines): a quarter of a run's tables began in the starts, about
  330 tables a game for p14 and 60 for p16, at a decision past the first fifth,
  the learner at any seat, so their `humanset` scores read games they trained
  from, and a network resumed from them keeps that: the wins newer than its
  ancestors' starts are the clean ones; per network the agreement with the
  human's choices, and from a quarter, half and three quarters of each game the
  network in the human's seat against the served bots, its wins).
- **Human games as data** (owner, 2026-10-03, after a 4-VP win over three
  `p5-self@1250` bots: "use the game log as a teaching vector"). A game from
  the Succubus Club fork ("Export replay": spec by KRCG ids + the decision log)
  replays exactly, so each decision becomes (observation, options, choice) for
  any seat. §2's "imitation bootstrap: rejected (no corpus)" is revisited with
  the owner's games as the corpus, in three steps, by value at the corpus's
  size: (1) **diagnostic** — one game is a few hundred human decisions, too few
  to train on, but enough to show where the network's play and a human's part:
  the human's choices the policy finds least likely, and the bots' falls in the
  critic's value with what happened in between (the policy samples, so a bot's
  unlikely choice is noise: bots are read by value, the human by probability);
  (2) **start positions** — self-play continued from a human game's prefixes,
  reaching states self-play rarely makes (a strong predator pressing); (3) **an
  imitation term** — cross-entropy on the human's choices beside PPO's loss,
  small coefficient, once there are dozens of games. Built: (1) —
  `zvtes_py.replay_decisions(json, obs)` (`crates/py/src/human.rs`: the
  protocol `Session` replays the log, every seat answered from it; per decision
  the policy and critic encodings, options, choice, events before it;
  `Session::request` gives the encoders the pending request),
  `zvtes_py.deck_cards(d)`, and `uv run python -m train.assess <replay.json>
  [checkpoint]` (the checkpoint's `obs` version; `*` marks a card outside its
  training decks). The corpus: `data/records/` (tracked, indexed in its README
  with each game's engine commit: a rule change can stop an old log replaying).
  Replays by meaning (owner, 2026-10-04: "so the replays I do are not
  useless"): beside its indices a replay logs each decision as its choice (a
  decision entry's `choice`, its option key: the engine's `Choice` with hand
  and uncontrolled positions and card-table indices replaced by KRCG ids), and
  a reader takes the option that is that choice, whatever its index; a decision
  the replay's engine asked between copies alike (`forced`) is passed over
  where a later engine asks nothing. An option change no longer stops a game; a
  rule change the game met still does. The corpus was converted on the engine
  each game read furthest on (README). In the owner's first six games
  (2026-10-03/04, all won) the policy's favourite was the owner's 36–46% of the
  time, lowest in the minion, influence and discard phases; it would have
  influenced out wherever it could (≈100%) and gave a crypt draw no weight.
  Steps (2) and (3) are the starts ("Opponents that punish" below) and
  imitation (above).
- **Default politics** (owner, 2026-10-04, after a Ventrue and a Toreador game
  to show it: "votes are very bad too … too bad at setting the terms. Parity
  Shift should by default damage prey and give blood to the player playing it.
  Kine Resources should damage prey and predator"). Measured first
  (`p5-self@1250` / `p6-frozen@1360`, 120 five-seat self-play games, political
  decks in half the seats): Kine Resources Contested and Conservative Agitation
  gave 46% / 43% of their points to the caller itself, Parity Shift 58% / 58%;
  votes a coin flip (48% to 54% for, on its own referendums as when the terms
  burn its pool); in the owner's seat the policy's favourite Parity Shift term
  was "0 to me". Rewards cannot teach this (a few decisions a game, scored at
  its end; voting by rule did not score more at self-play tables, p4-vote
  below). So, as temporary guidance to help training (owner: the goal is to
  relax these automated choices later in training), the network seats play
  their politics by rule — `zvtes_env::politics`, agent behaviour around the
  forward pass, not an engine mask (rules-qa 17.4 untouched: every option stays
  legal): (1) terms of the referendums moving pool are chosen by rule: damage
  to the prey first, then the predator, then across, the caller only when
  forced (two Methuselahs left); gains to the caller (Camarilla's Iron Fist's
  chosen Methuselah, Parity Shift's allocation), Parity Shift's pool taken from
  the prey (else the predator); points go stepwise, so an allocation scores n ×
  (its seat's weight − the best later seat's); (2) votes: the caller votes for;
  a Methuselah the terms burn or take from votes against, one they give to
  votes for — the wrong-side votes, Edge, burned cards and one-sided vote cards
  masked, and Pass masked while a free right-side vote (a vampire's, the
  political card's) is left; Edge and burned cards stay the network's call on
  the right side; a seat the terms leave alone is not steered (no forced
  cross-table play, owner 2026-10-03). Extended the same day (owner: "banish
  opponents vampires, not yours, choose your own clan when you play
  consanguineous boon, oppose opponents gaining titles, give titles to your own
  vampires"; then: "banishment typically goes to predator first (lowest in
  blood compared to cap), or the prey but only when there's an interesting play
  in it … leave the target to the network as long as it's on a neighbour and
  not oneself. Revolutionary council should go to destroy cumbersome locations
  and damage to the prey. Disputed Territory to steal an interesting location
  for oneself"). Bounds where the choice is a judgement (`politics::mask` in
  the terms): Banishment and Expulsion on a neighbour's minion, the network
  picking which (another opponent's only when no neighbour's is offered, one's
  own never unless nothing else is), its controller voting against;
  Revolutionary Council's points on the prey's pool and on the others'
  locations, one point each (none on oneself, one's cards, other Methuselahs or
  equipment beyond what the engine forces; which locations is the network's);
  its Anarchs to lock stay the network's; a Methuselah losing pool or a card to
  it votes against. By rule: Disputed Territory names the caller as the one
  taking control, the location the network's among the others', its controller
  voting against and the caller for; a title (the Justicars, Archon:
  `PutOnChosen`) goes on one of the caller's vampires (the biggest), its
  controller votes for and every other Methuselah against (the one steer of an
  untouched seat, at the owner's word); Consanguineous Boon names the clan with
  the most of the caller's vampires (then the fewest of the others'), and a
  Methuselah it pays votes for. Other referendums (Anarch Salon, the burns per
  minion) stay the network's. `NetAgent` applies it (the human-play bot);
  `tests/default_politics.rs` checks the terms and masks over random games
  (120: ~200 Kine Resources, ~40 Parity Shifts, Boons, titles, banishments and
  expulsions, Disputed Territories, Revolutionary Councils, each bound checked
  option by option). Re-measured with it (60 games): Kine Resources all to the
  prey but the last point (to the predator), to the caller only at two
  Methuselahs; Parity Shift 3 to the caller; no named seat votes for its own
  burn, no caller against its referendum. In training too since p7
  (`--politics`, on by default: `GamePool(politics=True)` answers every network
  seat's terms by rule and leaves the masked options out of the request Python
  sees, so the sampled index, its log-probability and PPO's ratio all read the
  offered options — the learner and the frozen opponents alike, in training and
  evaluation). Temporary by design: once the rest of its play is solid, the
  rule is relaxed step by step — a flag per window (terms by rule → terms
  masked → free; votes masked → free), each step kept only if the network's own
  choices hold the rule's results in self-play and in the owner's games.
- **Opponents that punish, games from the owner's positions** (owner,
  2026-10-04, after the review of 15 games: "the engine is missing sometimes
  interesting plays … when will it learn to get out of torpor and hunt to
  offset combat decks?"). The review (every decision of 9 games; counts over
  the 15): the bots tap out every turn and keep no blocker, never remove a
  drain in play (Creeping Sabotage offered ~60 times in one game), never rescue
  (0 of 107 offers) or diablerize (0 of 136), hunt only when forced, bleed into
  known blockers; self-play shows it is learned, not these games' luck (40
  games of `p7-politics@1400`: rescue 0 of 998 offers, diablerie 2 of 1174,
  leave torpor 14 of 41, hunt 128 of 2717). Against the training opponents
  these habits cost nothing — nobody diablerizes, every table races to bleed,
  the combat decks are piloted by the same bleeders — so rewards cannot teach
  them. Two levers, kept within the settled rule that training opponents be
  good ones ("Frozen opponents" above), hence no new scripted style: (1)
  Punishing habits on frozen opponents (`zvtes_env::habits`, agent behaviour
  like `politics`, not an engine mask): a `net:k+habits` seat plays its network
  and, by rule in the minion phase, rescues its own vampire (by a minion with 2
  blood), burns another Methuselah's card as the Ⓓ action it grants,
  diablerizes another's vampire in torpor, in that order; once some of its
  minions acted, its last ready unlocked one takes no action but a hunt or a
  rescue (a blocker for the predator's turn). `--habit-seats` is the share of
  frozen seats given them (their beat rates apart in `pfsp`), evaluated as
  `vs_habits4/5`; `best.pt` is still selected on the plain frozen opponents.
  `crates/env/tests/habits.rs` checks every rule choice and mask over 40 random
  games, from the legal view too. Each habit can be switched off (`--habits`,
  `habits::Set`), because forced, most of them cost the network strength —
  measured on `p7-politics@1400` with `crates/policy/examples/strength.rs`
  (native self-play, half the seats with the habits, each deal played twice
  with the halves swapped; VPs a seat, plain half against habit half; the
  control, the same network on both halves, spreads 1.02 / 0.98 at 256 games
  and 1.07 / 0.92 at 512, so ±0.1 is noise): all four, 1.22 / 0.65 (256 games);
  rescue alone 0.99 / 0.99 (512; 0.91 / 1.08 at 256: no measurable cost); burn
  alone 1.13 / 0.83, diablerie alone 1.15 / 0.85, kept home alone 1.17 / 0.78
  (256 each); rescue and diablerie 1.15 / 0.83 (512). (2) Starts
  (`zvtes_host::start`, `GamePool(replays=)`, a queued table's `start`): a
  training table begun from a decision in the last four fifths of one of the
  owner's games (`--starts data/records --start-tables`), the learner at a
  random seat, every library and crypt shuffled again from the pool's seed at
  the branch (from one position the replay's order would fix every draw: a
  future to memorize); hands, the uncontrolled region and the table kept; the
  time limit at least the position's turn + 20. A start reads its game by
  choices to the log's end or a rules change since (the pool prints how far,
  per game). `train/tests/test_pool.py`
  (`test_tables_start_from_an_owner_game`: a game replayed to a decision, every
  seat played on from there). The measure of both: the pool counts the
  learner's minion-phase decisions offering each habit (its own rescue, a burn
  of another's card, a diablerie) and those taking it, logged per iteration as
  `habits` (offered, taken).
- **Nudges** (owner, 2026-10-05: rescue "not forced like 100% but encouraged
  more as a guiding harness like we do for the votes … until it learns by
  itself", diablerie not for now (the blood hunt); "a fading bonus on passing
  in the discard and against cycling reactions"; "it has a tendency to stay
  idle with all in torpor when it could draw a crypt card or try to influence
  one more vampire"; "it bleeds with Aggressive Corpse and that minion has zero
  bleed … doing an action with no effect should not be favored"):
  `zvtes_policy::Nudges`, a bias on every network seat's logits inside the
  distribution — the pool's `opt_bias` (`GamePool.nudges`), added in `act` and
  in the PPO update alike (each row keeps the bias it was sampled with), so the
  ratio is the policy's own — shared by `NetAgent` and the examples, every
  option still offered and legal (17.4). `rescue` (+, its own vampire's
  rescue); `pass` (+, passing its own discard phase with a card to cycle);
  `reaction` (−, cycling a reaction); `bleed` (−, `empty_bleed`: a rulebook
  bleed by a minion bleeding for 0 before any action modifier, base plus borne
  cards — an ally's printed 0); `idle` (−, `idle_influence`: ending the
  influence phase while a crypt draw is still offered, i.e. 4 transfers
  unspent, with 5 pool or more); `draw`, `stuck` and `nightmare` below.
  `--rescue-bonus --pass-bonus --reaction-bonus --bleed-bonus --idle-bonus`
  (and `--draw-bonus --stuck-bonus --nightmare-bonus`), fading together from
  `--nudge-from F` to 0 at `--nudge-until U` (0: permanent); the `habits` row
  adds the learner's `keep` (discard phases with a card to cycle, passed) and
  `reaction` (holding one, cycling one). Permanent since p9 at rescue 6, bleed
  4, idle 3, draw 7, stuck 6, nightmare 6 (pass and reaction left to
  imitation): faded in p8, every behaviour went back, and full-size they cost
  nothing measurable. Declining is always on the map: over 300 random games on
  every deck, of 7171 decisions offering a card play none offered only plays (a
  Pass, end of phase or hand strike each time); the discard phase always offers
  Pass and a discard. The network's rescue logits are so low that the bonus
  must be large to matter: taken 0% of the rescues offered without it, 1.4% at
  +3, 7.1% at +6 (24 self-play games each; the owner 5 of 5, the bots in the
  owner's games 0 of 36). Rates of the others (`examples/waste.rs`, 24
  self-play games each, the owner for reference): cycling 96% plain, 46% at
  pass +4.5 (owner 53%), the nudge moving how often, not which; reactions'
  share of the cycled cards 24% plain, 7% at reaction 3 (owner 23%); bleeds for
  nothing 18%, 4% at bleed 4 (owner 0 of 43); the influence phase ended with a
  crypt draw offered and 5+ pool 38%, 17% at idle 3 (owner 6%); wakes followed
  by no block unmoved (no nudge reads blocks). Strength
  (`examples/strength.rs`, 512 games, VPs a seat plain half against nudged
  half; the control's spread 1.07 / 0.92): rescue 6 0.94 / 1.05, pass +3 0.90 /
  1.07, reaction 2 0.98 / 1.00, bleed 4 0.93 / 1.05, idle 3 0.97 / 1.01, all
  five 0.98 / 1.00: none measurable, so the frozen opponents carry them too.
- The decisive bot: `NetAgent::temperature` divides the logits before sampling;
  the human-play bots play at 0.5 with the nudges
  (`zvtes_policy::profiles::NETWORK`), training samples at 1. Measured (512
  games each, against the network sampling as trained): 0.7 1.06 / 0.92, 0.5
  1.16 / 0.82, 0.3 1.18 / 0.80, 0.5 with rescue +6 1.17 / 0.81.
- Waste (`crates/policy/examples/waste.rs`, 2026-10-05; owner: "it has a
  tendency to play cards for the sake of it … throws away wakes, unlocks …
  cannot construct its hand"): the clear gaps are wakes followed by no block
  from the woken vampire (owner 22% of 18, self-play 85% of 562, 74% at
  temperature 0.5) and cycling (a discard at the own discard phase: owner 53%
  of 122 phases, self-play 95%, the bots' discards spread as their hands are
  where the owner keeps reactions and political actions and cycles action
  modifiers); unlocks followed by no action or block and a playable card cycled
  while an unplayable one stays do not tell them apart (owner 44% and 52%, bots
  46–53% and 38–55%: a clan card kept for a vampire to come).
- **The crypt draw, never taken** (owner, 2026-10-10: "I see the bot never pays
  4 transfers to draw from the crypt, is it even a move offered to it?").
  Offered (census, 32 games: ~1300 influence decisions offering it), taken by
  neither p8 nor p9 — p8 gives it a median 6e-8 where the owner drew (p9@2640
  1.4e-4: imitation lifts it, far too slowly); the owner draws at 42 of his 389
  offers (11%). A collapsed option is never sampled, so never learned. The
  `draw` nudge (`Nudges::draw`, `--draw-bonus`; the pool's "draw"; `waste
  --draw`, `strength draw:`): added to a crypt draw where `idle_influence`
  holds (5 pool or more). Self-play of p9@2640 (`waste`, 24 games, rescue 6,
  bleed 4, idle 3): drew 0% of those offers at 0, 7% at 6, 21% at 9, 45% at 12;
  influence phases with nothing done 46% → 28%, 15%, 3%. Permanent at 7 (~the
  owner's rate) from p9's restart at 2640.
- **Stuck pool and own nightmares** (owner, 2026-10-10, via the play sessions):
  a Toreador bot transferred onto a second copy of a unique vampire it had in
  play — never offered out while the first stays (no voluntary self-contest),
  the pool on it stuck; a Ravnos bot moved its Week of Nightmares' counters
  onto its own minions ("never a good move"). Both legal, both kept (rules-qa
  17.4). `waste` counts them (`zvtes_policy::stuck_transfer`, `own_nightmare`).
  p9@2700: onto the stuck copy in 35% of the influence decisions offering one
  in self-play, 44% for the bots in the owner's games (`…T16-45-56`: 11 of 50),
  the owner 0 of 178; nightmare counters 2 of 9 onto its own in self-play
  (`…T22-05-29`: 2 of 7). Two permanent nudges taken off them (`Nudges::stuck`,
  `nightmare`; `--stuck-bonus`, `--nightmare-bonus`; `waste
  --stuck/--nightmare`, `strength stuck:/nightmare:`): at 4 and 4 the stuck
  rate is 11%, own nightmares 0; at 6, 5%. Permanent from p9's restart at 2800
  (stuck 6, nightmare 4), from p10 nightmare 6 (own nightmares 3 of 24 at 4).
- **Chores: options never declined** (owner, 2026-10-08: "some things are only
  'options' in the sense a human is allowed to forget them, but it's never good
  to do so"). The engine elides only empty and single-option windows; the
  network decided every other option. `zvtes_policy::chores` (agent behaviour
  like the default politics, 17.4 untouched): when a chore is offered, network
  seats' `Pass` is masked (one option left: taken, no forward) — the Edge's
  pool (declining it for a later Parity Shift: the accepted rare exception), a
  hunting ground's blood to a ready vampire with room (free: no lock, no cost).
  `--chores` (on), the pool's `chores`, `NetAgent::chores` (on). Found by
  `examples/census.rs` (self-play, decisions by window and option kinds, what
  the network picks; `p7-politics@1800`, 32 games): the Edge's pool taken 97%,
  the hunting grounds declined 26–33%; Homunculus, Vessel, Garibaldi-Meucci
  Museum and Anarch Troublemaker, often declined, have a cost or a trade-off:
  left to the network. With the chores: every offered chore taken; strength
  0.994 / 0.997 (512 games: none lost). With the Sabbat slices, the optional
  effects the rulings call never worse to use, as the scripted bots take them:
  Fever Pitch put on its vampire (rules-qa 53.5), Cursed Abattoir's counter
  (55.6); Forward Momentum's counter is no option (the engine adds it; only its
  order with the unlock is chosen, 53.6).
- **Faster iterations** (owner, 2026-10-05: "aim for a 2 to 5 times speed
  improvement total … to make this project viable"). p7's iterations took ~5.5
  min: training ~190 s (rollout 45–70 s, update 80–120 s), evaluations the rest
  (~30 min every 10 iterations). (1) An evaluation plays all its games at once
  (the pool was capped at 64) and none after (`GamePool(refill=False)`: a
  finished slot leaves once the queue is empty, no throwaway game in the
  batches): 256 games 219 s against 576 s. (2) `eval_every` 20 (was 10). (3)
  `bots_every` (50, 0 from p9): the bots evaluated only at its multiples and
  last. The update is compute-bound on the M3's 10-core GPU (on a p7 rollout:
  backward 46%, critic forward 18%, policy forward 16%, collation 11%; the
  model's small matmuls reach 300–400 GFLOPS; fp16 or bf16 autocast no faster,
  fp16 weights abort in MPS's matmul): the levers are less work, tokens and
  passes. Observation `V3` (`zvtes_env::obs`; owner, 2026-10-06: "make our
  training as lightweight as possible"): `V2`'s content in fewer tokens — the
  viewer's library and crypt apart (`Obs::deck_ids`/`deck_f`: the network
  embeds them as tokens and adds their mean to the context token, out of
  attention, no weights of its own: a `V2` checkpoint resumes as is), other
  Methuselahs' face-down uncontrolled vampires as their seat's count and
  counters (events pointing at them point at none), and the transfers left in
  the influence phase on the context token (the network had to infer them): the
  policy's tokens 120 → 76 a decision (p90 145 → 101), the critic's 141 → 110;
  `--obs 3` the default. Features cross to the device as float16 (the
  collation's cost is the copy, not the padding). Each iteration logs the last
  epoch's `kl` (k3, from the playing policy) and `clipfrac`. On p8's
  configuration (20 iterations from `p7-politics@1800`): `V2` 3 epochs 176 s an
  iteration, `V3` 3 epochs 161 s, `V3` 2 epochs 146 s, against ~330 s for p7's.
  The rollout is inference (the policy's forwards 70%, the critic's 16%, the
  engine ~4%; ~65 000 decisions an iteration for ~14 000 learner samples: ~80%
  of the inference is the opponents'); the update moves the policy little (kl
  ~0.002, a tenth of PPO's usual target), the third epoch ~20% of the movement
  for ~45% of the update. Settled (owner, 2026-10-07; each by the check that
  answers it, as a side-by-side run could not tell learning apart: p7 gained
  ~0.05 VP a 100 iterations, under one evaluation's noise): `learner_seats` 1
  (2 gave 8.1 ms a sample against 8.3, the learner facing itself);
  `best_margin` 0.05 (was 0.1: on p7's last 44 evaluations 24 re-measures
  instead of 39, none of its 15 bests missed, 0 would miss 2); `lr` 1e-4 from
  p8 to p16 (kl 0.0046 and clipfrac 4.6% from 0.0023 and 2.3%, PPO's usual
  bounds ~0.01–0.02 and ~10%: twice the movement, still inside the clip). p12's
  configuration, profiled (2026-10-16; a quarter-size rollout from p11@4340,
  shares on a shared GPU): ~30% rollout, ~70% update (p11's pairing brought
  ~22k samples an iteration against p10's ~15.5k: 113 s against 77); the update
  backward 37%, the imitation batch 24% (256 long human rows a minibatch;
  sorting them by length into chunks of 64 was slower), critic forward 16%,
  policy forward 12%, collation 7%, optimizer 4%; the rollout the networks'
  forwards 70% (the frozen opponents ~45% of it, ~14% of an iteration), the
  engine ~10%. Hence `--bc-every 4` (from p13; default 1): the imitation term
  on every 4th minibatch at 4× its weight, the same expected gradient before
  clipping (the clip and Adam see a spike every 4th step), ~14% of an iteration
  (measured 23.1 s against 34.1). Left to a run's preparation, as they change
  the training: 2 epochs (~23% of an iteration), one frozen opponent (≤5%); the
  pairing (~25–30%) off from p13.
- **Futile blocks on the E25 engine** (owner, 2026-10-15: before any redeploy).
  E25 (`97c4880`) offers every legal block attempt, those that cannot reach the
  stealth too (`Engine::block_can_reach`); no network was trained on them.
  `waste` (block windows), 80 four-seat self-play games at the server's
  settings (temperature 0.5, its nudges): p10@3840 takes a futile attempt in
  61% of the windows offering one and attempts in 66% of those offering only
  futile ones (the windows elided before E25), against 58% for a reachable
  attempt; p11@3920 63%, 66%, 62%. The networks do not tell futile from
  reachable. The owner's records (before E25, replayed on it, read whole since
  `waste` passes the windows their engine elided, `Replay::elided`): 92 windows
  with a futile attempt, none taken. Owner, 2026-10-15: "mask in the bot layer,
  train and serve" — `zvtes_policy::reachable_blocks`, the outermost mask of
  `NetAgent` (the server, `waste`) and of the training pool (`crates/py`): a
  network never attempts a futile block, a window of futile attempts only
  passes without asking it (no training decision). A failed attempt costs
  nothing, so no reward would teach it. The engine still offers them (rules-qa
  73.8); the scripted bots score them below the pass (§5). Masked, p10 at the
  server's settings (80 games): 0% of 2,102 windows with a futile attempt, 0%
  of 1,370 with only futile ones.
- **Combat play by deck** (owner, 2026-10-20, the training board's "Rush and
  wall play": "What matters is they have specific actions and reactions to
  enter combat and they need their combat to be played efficiently on the right
  targets for the deck to work against good opponents"). `waste --by-deck`, a
  rush being an action entering combat with its target (`zvtes_policy::rush`);
  combats counted from the combat notes, those with every window elided
  included. p13 / p11 self-play (400 games at 4 seats, no nudges) against the
  humans (`data/records` and the 2026-10-19 harvest pooled, 22 to 81 combats a
  deck, none on Banu Haqim: small, and against older bots). Own turns with a
  rush offered and one declared: Nosferatu 4% / 4% (the humans 42%), Gangrel 4%
  / 3% (21%), Banu Haqim 16% / 14% (none offered to the humans), Tzimisce 29% /
  15% (100%), Salubri 26% / 33% (30%), Brujah 29% / 31% (69%), Hecata 42% / 60%
  (100%). The opposing minion to torpor or burned, for p13, from 12% (Banu
  Haqim) to 36% (Brujah), p11 within 6 points, the humans 26 to 91%; its own
  minion down 8 to 31% (the humans 2 to 18%). No gap in the blocks (a reachable
  attempt made in 48 to 74% of the windows, the humans 44 to 65%), the targets
  (the prey, a minion that had bled, one carrying equipment) nor holding back;
  wakes with no block after 77 to 88% (the humans 7 to 67%). The gap is the
  combats they enter and what those achieve: the rush decks seldom rush
  (Nosferatu, Gangrel, Banu Haqim under one turn in six), and their combats
  seldom put the opposing minion down. p13 at one seat against three p11
  (`decks.rs` on `331ce66`, 30 seats a deck): Nosferatu 0.20, Banu Haqim 0.37,
  Hecata 0.77, Salubri 0.90, Tzimisce 0.93, Brujah 0.97, Gangrel 1.13 (mean
  0.97).
- **The critic against the position** (owner with Tom, 2026-10-22: "some
  measure of the quality of the position", the pools of every seat, the lunge,
  the actions and blocks each side has). `train.position` (the features:
  `zvtes_py.position_names`, `crates/env/src/position.rs`, for the deciding
  seat: the pools of the seat, its prey, its predator and the cross-table
  seats, VPs, each side's ready, unlocked and torpid minions and the blood on
  the ready ones, hand, library, turn, the edge, the lunge — a bleed of one a
  ready minion reaching the prey's pool, or the predator's reaching one's own):
  on 1,024 self-play games of p14@5280 (every seat its policy, 4 and 5 seats
  alternating, 578,503 decisions), the explained variance of each seat's result
  ((VPs + 5 × win) / 5, the critic's target) by the decision's share of the
  game, for the critic as it is, a linear fit of the 26 features per stage
  (fitted on 70% of the games, scored on the others) and both together.
  Self-play: the first quarter −0.03 / 0.02 / 0.08, the second 0.07 / 0.09 /
  0.17, the third 0.34 / 0.29 / 0.38, the last 0.71 / 0.57 / 0.72; the human
  records (`data/records` and the harvest, read whole, 28,554 decisions, the
  self-play fit): −0.13 / −0.03 / −0.05, −0.01 / −0.09 / −0.04, 0.23 / 0.10 /
  0.21, 0.66 / 0.47 / 0.66. The 1,024 self-play games were played without
  politics, chores or nudges, a pool's defaults, which the network never
  trained on; `runs/phi-p14.json` (p15's Φ, the search's leaf) is fitted on
  them, and p15's reading below compares two such measures. At training's
  settings (`train.position` plays them since 2026-11-02; p14@5280, 500 games,
  the same seed, 30% held out): the critic 0.16 / 0.17 / 0.37 / 0.68 against
  0.02 / 0.11 / 0.30 / 0.72 at the defaults (on `a4d184d`), the features 0.01 /
  0.09 / 0.26 / 0.58, both 0.19 / 0.22 / 0.39 / 0.69; over 3,000 games it never
  trained on, the critic 0.16 / 0.21 / 0.39 / 0.72, the seat's deck alone 0.05
  / 0.08 / 0.11 / 0.07. The critic reads more than the features at every stage,
  and the first half it seemed not to read was the tool's. On the humans' games
  it is worse early (their play is not the self-play it learned). The features'
  weights (a standard deviation each, the result's scale): in the second
  quarter the seat's pool +0.10, the blood on its ready minions +0.08, the
  prey's pool −0.07 and its margin over one's ready minions −0.08, the
  predator's ready blood −0.05, the prey's −0.05; in the third the ready blood
  +0.14, the pool +0.12, VPs +0.12; the lunge +0.04 in the third and +0.05 in
  the last, being lunged −0.04 in the last; the unlocked minions +0.03 to +0.04
  (the actions and blocks left this turn).
- **The impact ledger** (owner, 2026-10-25: "do the bot network improve on
  getting more actions and more impactful meaningful actions? … missing
  opportunities in many places"; runs are judged by it, not by single
  counters). `waste`'s impact lines, per group: A&B a round — the seat's
  actions declared, its blocks that landed and its hindrances (another seat's
  action canceled, failed or ended unblocked, a bleed bounced or cut to
  nothing, by its card: `zvtes::profile`), over its own turns, one a round
  whatever its minions' number ("a single minion with a lot of unlock … can
  produce equal A&B as multiple smaller minions"); a reaction neither blocking
  nor hindering counts nothing; R = 2 × pool + the blood on its minions (pool
  the more important), its own, its prey's and its predator's change over its
  own turns; the hand: slots and dead slots a round, A&B and the prey's R a
  hand slot a round, and with `--cards` per card the rounds held, its plays a
  round held, its cycles and its prey's R a round held — holding a card costs
  its rounds, so cycling toward the right hand counts for it (owner: "cycling
  cards to get a more appropriate hand is even more important than keeping
  cards until they're really efficient"). A measure, not a reward (p15). p14
  (100 self-play games at 4 seats, and its seats in the owner's records)
  against the owner (298 turns, against p5–p9's older bots): A&B a round 2.13 /
  3.03 (actions 1.70 / 2.32, blocks landed 0.37 / 0.57, hindrances 0.06 /
  0.14); R over its own turns: its own −1.48 / +0.79, its prey's −3.37 / −6.06,
  its predator's −1.02 / −2.51; dead hand slots a round 1.10 / 0.87; a hand
  slot a round: A&B 0.31 / 0.41, the prey's R −0.48 / −0.83; Govern the
  Unaligned played 0.42 / 0.89 a round held, Telepathic Misdirection 0.18 /
  0.24 (the bots' most held card, 212 rounds against the owner's seats), Freak
  Drive 1.03 for the owner. The gap is broad — about 40% more A&B, every kind,
  and twice the prey's loss a turn, the owner's resource growing where the
  bot's shrinks — so a planning lever before a counter's.
- **Search in the minion phase** (the owner's second p16 step, 2026-10-25;
  dropped by the owner, 2026-11-02: not served, no stricter margin, no run on
  it; its example in git): at each minion-phase choice of one seat, p14's
  favourite options each played out to the start of the seat's next turn, every
  seat p14, scored by the position's potential there (`runs/phi-p14.json`, "The
  critic against the position"), the favourite kept unless another's paired
  gain passes a standard error. The first measures (+0.43 ± 0.25 VP a seat
  seeing the hidden cards, +0.29 ± 0.25 on a "fair" deal of the others' hands)
  leaked what a seat may not read (the others' true cards, only shuffled, their
  face-down vampires and stored cards, the engine's random stream). The measure
  that stands (2026-11-02; `runs/search-2026-11-02/`, its README; engine
  `946fd78` with an `Engine::reseed` accessor, no rule changed; p14
  `a78e74292acb`; `rust/searchf.rs`; 3 options, 8 worlds, the served settings):
  each world is built from `GameState::legal_view`, so no hidden card can reach
  it, the seat's own library and crypt shuffled, each other seat's hidden cards
  from one TWDA decklist drawn by the likelihood of the cards it has shown, a
  new random stream (two engines with one legal view and different hidden cards
  scored bit for bit alike at all of 380 positions). 400 games on new seeds: VP
  1.03 → 1.13 (+0.10 ± 0.17), wins 24.0% → 25.2% (+1.2 ± 5.2), its prey's R
  −0.11 ± 0.22, none told from zero; A&B +0.08 ± 0.07, past two errors, +3.8%,
  one of four measures. `humanset` (four of the five held-out games, all of
  them starts of p14's run; 20 games a position, bots p14): won 94 → 111 of 240
  (+7.1% ± 5.9), VP +0.21 ± 0.16, four human games behind them, not significant
  game by game. It overrode the favourite at 31% of 7,305 decisions, about what
  chance gives at that margin; a play-out assumes the opponents play p14, exact
  at these tables, not against a human; up to ~10 CPU-seconds for each of the
  ~18 searched decisions a seat makes a game.
- **Where p14 stands** (2026-11-02, the owner's question after p16;
  `runs/search-2026-11-02/`, its README: commands, engines, raw results; ± two
  standard errors). Strength: p14 at one seat against three, 500 games at the
  served settings: p8 1.49 ± 0.14 VP, 36% wins; p10 1.11 ± 0.13, 25%; p13 0.93
  ± 0.12, 22% (temperature 1, no nudges: p8 1.22, 29%; p10 1.12, 25%). With the
  head-to-heads below (p11 0.99, p12 0.88, p13 0.97, p14 1.00, p15 1.03) and
  the humans' wins (p10 11 of 14, p11 13 of 19, p14 26 of 33), strength has not
  measurably moved since p10. A run's rising score is mostly recovery: even is
  2.125; p14 read 1.70 at its first evaluation and came back as its learning
  rate tapered (means by third 1.87, 1.94, 2.02), p13 and p15 alike. What the
  gradient says, and how little four folds can: 3,000 self-play games of
  p14@5280 at training's settings, on `a4d184d`; each advantage estimator's
  gradient on four folds sharing no game, which tells signal from noise; Monte
  Carlo (the result − V) unbiased but weak, its signal 1.9 standard errors on
  the six pairs of folds; a shuffled control reading zero. Most of the signal
  is "more decisive" (cosine −0.80 to −0.92 with the entropy gradient, by
  estimator): the adaptive bonus cancels that by design, and the served
  temperature cashes it (0.5 against 1, no nudges: 1.17 ± 0.13 VP). With that
  out, Monte Carlo's signal leans away from the imitation term (read on 14,831
  choices, p14 imitated 9,231) on all four folds (−2.9 standard errors on 3
  degrees of freedom, p ≈ 0.07, its size unmeasured; −0.7 at λ 0.95, the two
  not told apart). Beyond those two directions Monte Carlo finds nothing but
  has no power there; training's own estimator keeps 34% of its signal (2.8
  standard errors), and the forces do not sum to zero (1.11 of the signal's
  length): no fixed point is shown, and the plateau rests on the measures
  above. The usable share of a fold's gradient reads 0.10 today, 0.23 at λ 0.8
  on whole games, 0.09 at 0.95, 0.06 at 0.99, 0.03 Monte Carlo; leaving a fold
  out moves it 0.03–0.33 today and 0.01–0.78 at 0.8, and reverses the order
  once: four folds do not rank λ. The game's result enters none of the first
  quarter's advantages and 28% of the last's. A critic trained on the results
  (warm from p14, 2,500 games) gains 0.02 explained variance at best, then
  overfits; a second head on the ledger two rounds on gains nothing more.
  Imitation memorises: 0.89 accuracy on trained choices, 0.52 held out at p14's
  end, the held-out loss rising through every run from p10 to p16; d16,
  distilled from p14 with no imitation of its own, gives the humans' choices
  more probability (−1.54 / −1.76) and makes them its favourite less (47.6% /
  51.4%). Rules laid on p14 unretrained (paired, 500 games, each ± 0.16): a
  block whenever one is reachable and the action is directed at the seat −0.10
  VP, every reachable block −0.18, the last unlocked minion kept home with 3
  ready −0.20, the first and third −0.22. Which choices pay, crudely (the
  result − V against the window's mean, by kind and third of the game, the
  state not controlled beyond V): nothing above +0.06; an ability used at a
  strike −0.10, in the influence phase −0.06 to −0.09, an action taken late in
  the minion phase −0.04 (6.5 standard errors, 56% of that window). The ledger
  (13,490 seats): seats that win make 3.6 A&B a round, the others 1.9, so A&B
  rises with winning, and p14's rates reach the owner's 3.03 at a two-thirds
  share of winners. Own and prey's R explain 0.18 and 0.34 of the result's
  variance in the last two quarters, A&B 0.03 and 0.07, blocks landed and
  hindrances nothing. Humans at p14's tables (28 games): level on actions,
  ready vampires and blocks landed through round 6; blood ahead from rounds 4–5
  (11.3 / 8.8 at round 5, custom decks unaccounted); blocks landed 1.5 times
  theirs at round 7 and twice from round 8 (21, 18 and 12 human seats).
- **Bots on any deck** (owner, 2026-10-26, via Z-succubus: humans teach the bot
  the recent TWDA decks it plays worst; served at `GET /catalog/taught`,
  protocol 2.22.0). `decks.rs --file <served.zvp> <games a deck>`: every TWDA
  deck the engine keeps (177 on `83b135c`; the trainer's `twda` family draws
  from them all, so the older ones are measured whole, not sampled) at one seat
  drawn uniformly, precons (V5, anniversary, Sabbat) at the three others, none
  sharing more than 20% of their crypt, every seat the served bot (`NETWORK`:
  temperature 0.5, its nudges), games ending at 75 player turns (the server's
  at 100). Written beside the weights as `<served>.decks.json`: their SHA-256
  (the server serves the file only while it names the weights served), the
  engine, the day, the measure in a line, and per deck, weakest first, its
  catalog name, TWDA id, event, date, `recent` (dated within two years of the
  measure), games, game wins, VP a game and VP share (its VPs over all those
  scored at its tables, 0.25 even), the file a `zvtes_api::DecksMeasure`, the
  type the server reads. It also sets the fork's bot pool (owner, 2026-11-08: a
  floor at 0.15, the decks below for humans to teach): beside the precons, the
  decks dated within six months of the game's day with a VP share of at least
  0.15; a deck the file lacks stays out until measured (the platforms line,
  `docs/platforms/sc.md`); hence the training skill's re-measure when the kept
  TWDA decks gain one. p17's measure of 2026-11-06
  (`runs/p17/decks-2026-11-06.json`, a copy of the file then served; weights
  `f426338f6c80`, engine `83b135c56b17`, 100 games a deck, 2 h 30 on 8 threads;
  its measure line, from an earlier build of the tool, omits the 75-turn
  limit): with a TWDA deck at one seat against three precons the bot wins 16.4%
  of its games and scores 0.77 VP a game, a VP share of 0.196 on the 97 recent
  decks (from 2024-11-06) and 0.189 on the 80 older, below even; the median
  deck 0.18, the decks from 0.00 to 0.64, 51 under 0.10 and 54 at or above
  0.25. The weakest recent: The Path of Enkidu (Adeus 2025) 0.005, no win in
  100, then As carequinha (Nordestão 2025) 0.021 and Cardinalis Benedictio's
  0.028; the floor, Unnamed (Praxis Seizure: Barcelona, 2004), no VP in 100
  games; the strongest, The Asylum (Italian NC 2022), 0.64. A deck's win rate
  at 100 games carries about ± 6 points (two standard errors at 10%); its VP
  share's error is not computed (the tool keeps sums).

**The runs** (one entry a run, oldest first: its launch, the command or the
flags it changed, its numbers, what it settled; a run is prepared as a "pN,
prepared" entry here and becomes its entry when it ends — the training skill's
"A run")
- **Self-play** (2026-09-27, defaults, 140 iterations ≈ 2.3M decisions, ~1 h on
  the M3): against three random seats 0.71 VP a game at iteration 10 (random
  level: 0.73), ~2.9–3.1 VP and 72–77% game wins from 60 on; against
  aggro/wall/aggro 0.07–0.17 VP up to 50, then 0.4–0.7 (an even share ~0.75)
  and 5–12% game wins, a plateau from about 90 — largely the fixed seating's
  (Training above: 1.05–1.09 VP / ~25% with the orders mixed). Self-play alone
  does not reach the bots; its iteration 90 started the league runs and
  p4-twda.
- **League runs** (2026-09-28, three, each from the self-play run's iteration
  90 to 200, ~1–1.5 h). The first, on the defaults of the time: against
  aggro/wall/aggro 0.8–1.03 VP from 110 to 180 on its fixed-seating evaluation,
  0.58 at 200 as the entropy fell from ~0.4 to 0.27; 3.57 VP and 90% game wins
  against random seats at 190; its beat rates' 0.05 EMA spanned ~20 games
  (aggro 0.67 → 0.27 → 0.58 in 20 iterations), hence the per-iteration rates.
  The second, the revised league (one learner seat, the exploiter,
  per-iteration beat rates, `gw_weight` 2, entropy 0.02, 256-game evaluations):
  the same shape, 0.85–0.89 from 150 to 180, 0.61 at 200 as the entropy fell
  from ~0.68 to 0.59; the exploiter never led the learner (beat rates
  0.49–0.53), retired twice by the 50-iteration clock — with the learner's
  reward and weights it has no reason to find anything the learner does not,
  hence off by default; each learner seat ~6k samples an iteration, half the
  first's, hence 512 then 1024 games. The third (512 games, the learning rate
  tapering to 0.1×, `best.pt`, no exploiter): no late collapse (0.57 → 0.86 at
  180, 0.81 at 200). Re-measured on the fair evaluation (every opponent order,
  60–90-turn limits, precons, 256 games), none improved on the start (1.05 VP /
  25% game wins at 4 seats, 1.28 / 26% at 5): the second's 160 1.06 / 23% and
  1.47 / 27%, the third's best 1.11 / 23% and 1.33 / 25% — their "gains" were
  the seating's noise, hence `eval_tables`.
- **p4-twda** (2026-09-29, `runs/p4-twda`: precons + the 13 TWDA decks,
  clan-balanced, from iteration 90 to 300, evaluated on those decks; the start
  1.13 VP / 26% game wins against aggro/wall/aggro, 1.23 / 24% against
  aggro/wall/aggro/wall, 2.62 / 59% against random seats): 1.44 / 32% and 1.51
  / 30% at 150 (entropy 0.64), the first gain on the fair evaluation (on three
  other seeds 1.32–1.44 VP, 30–34%, and 1.42–1.58, 27–32%), then back down as
  the entropy fell (0.57 at 200, 0.42 at 300: 1.12 / 25% and 1.14 / 20%) —
  hence the adaptive entropy bonus. `best.pt` 150 (`runs/p4-twda/best.pt`).
  Resumed from it with that bonus: at the full learning rate 1.19 / 1.20 in 10
  iterations; at 1e-4 tapering 1.48 / 1.40 at 160, then down to 1.13 by 220
  (stopped) with the entropy held at 0.63–0.70 and the training tables' beat
  rates flat all along (aggro ~0.5, wall ~0.68, random ~0.77, snapshots
  0.47–0.57): the policy drifts, training does not steer toward beating the
  bots, a good checkpoint is found, not converged to. Its levers (larger
  batches, more bot seats, the omniscient critic, selection on several seeds)
  made the critic run.
- **Critic run** (2026-09-29/30: the omniscient critic with a 10-iteration
  warm-up, 1024 games, 60% bot seats, 3-seed selection; from p4-twda's
  `best.pt`, iterations 151 to 350 at 1e-4 tapering, ~70 s an iteration, then
  to 600 at 5e-5 tapering): the warm-up held the start (1.44 / 1.51 VP), a dip
  as critic and policy adapted (0.49 / 0.43 at 190), then a steady climb — 1.70
  / 1.71 at 340 (39% / 34% game wins), 2.06 / 2.15 at 520, 2.11 / 2.12 at 580
  (51% / 43%), 2.12 / 2.22 at 600, flattening from 520; ~3.1 VP and 70% against
  random seats, ~1.0 against the latest snapshot. `best.pt` 580, 3-seed mean
  2.18. The first run to improve steadily rather than wander. In 200 five-seat
  games against itself 580 never rescues (9 in all, probability ~0 where a
  uniform pick gives 0.11–0.12) nor diablerises (12), where iteration 150 did
  32 and 118, and a torpid vampire leaves torpor by itself about half the times
  it can: training removed both, and no table-balance play is learned from
  these opponents and rewards.
- **p4-deck** (2026-09-30, `runs/p4-deck`, then `runs/p4-deck2` on the rules of
  Batch 46 from 621): the remaining-deck tokens (`T_DECK`, ~41 → ~71 tokens a
  decision; a checkpoint from before loads the new token type at zero,
  `model.remap_vocab`, with fresh optimizers); from the critic run's `best.pt`,
  5e-5 tapering, ~100–160 s an iteration. No dip, a plateau above the one
  before: 3-seed means 2.14 (640) to 2.31 (820), 5-seat game wins around 50%
  from 700 on. `best.pt` 820 (2.19 / 2.38 VP, 53% / 51%).
- **p4-vote** (2026-09-30 → 10-01, `runs/p4-vote`, 821 → 1070, 5e-5 tapering,
  ~125 s an iteration): after the owner's first game (the bots did not vote
  against referendums against their interest; 820 voted for and against alike,
  0.24 / 0.24 on its own referendums, 0.31 / 0.31 when the terms put points on
  its seat, the observation naming neither the referendum's card, its terms nor
  its tally), `T_REFERENDUM` and the term features added. A higher plateau,
  3-seed means 2.33 (830) to 2.44 (1020, 1060); single evaluations up to 2.63
  VP / 59% at 5 seats. `best.pt` 1060 (2.34 / 2.48 VP, 59% / 53%), the
  human-play interface's `.zvp` from 2026-10-01 (its export byte-identical).
  Its votes stayed a coin flip (0.22 / 0.23), and voting well did not pay at
  these tables (880 with one seat voting by rule 0.91 ± 0.05 VP against 0.97 ±
  0.05 unchanged, 1000 five-seat games each, 725 votes changed): learning votes
  needs politics in the training pool, not a richer observation alone — hence
  the default politics.
- **p5-self** (2026-10-02/03, `runs/p5-self`, 1061 → 1310, from p4-vote's
  `best.pt`, precons + TWDA): at p4-vote's end the bots had stopped teaching
  (beat rates aggro 0.72, wall 0.81, random 0.86, flat over 70 iterations,
  every snapshot ~0.5, yet 60% of opponent seats bots), so `--bots aggro
  --table-bots 1 --bot-seats 0.1 --opponents-kept 8 --lr 5e-5 --entropy-coef
  0.04`: at most one aggro a table (~⅓ of tables), the rest the learner's last
  8 snapshots by PFSP. Flat within the noise: 4 seats 2.16–2.42 VP (55–61% game
  wins), 5 seats ~2.4 early and ~2.6 late (55–60%), ~1.0 against the latest
  snapshot, aggro's beat rate 0.66–0.71. `best.pt` 1250, 3-seed mean 2.49 (2.42
  / 2.62 VP, 61% / 56%) against 2.44 at the start. It settled the frozen
  opponents (Training above).
- **p6-frozen** (2026-10-03 → 04, `runs/p6-frozen`, 1251 → 1413 of a planned
  1650): the frozen-opponent league's first run, from p5-self@1250 reading
  `V2`, opponents `p4-vote@1060` (the interface's) and `p5-self@1250` (the
  owner asked for "previous versions of itself", plural; `--opponents
  runs/p4-vote/best.pt` alone reverts): `--league true --opponents
  runs/p4-vote/best.pt,runs/p5-self/best.pt --decks precons,twda --lr 5e-5
  --entropy-coef 0.04 --iterations 1650`, the first 10 critic warm-up. The
  start, 512 games a size against the pair: p5-self@1250 reading `V1` 1.04 VP /
  23% game wins at 4 seats, 1.01 / 20% at 5 — parity; reading `V2` untrained
  (new token types at zero, card rows not) 1.03 / 22% and 0.94 / 17%. Memory on
  MPS: the first launch thrashed (`runs/p6-frozen-oom`), the allocator's cache
  growing ~2 GB an iteration (attention maps of 1024-row minibatches at ~200
  tokens, every shape cached apart) to 31 GB on a 24 GB machine, 2.5 h an
  iteration by 1255; since, minibatches run in chunks within `attention_budget`
  (rows × tokens², gradients accumulated: the same step) and the cache is
  emptied after rollout, update and evaluation. That left a second leak, ~1 GB
  an iteration in small host allocations (67 M of them by 1259): the MPS
  backend keeps a compiled graph per tensor shape, and `V2`'s rows (and the
  chunks) brought ever new shapes, so batch shapes are rounded (rows to 64,
  tokens to 16, options to 8, padding masked; chunks halve a 1024-row
  minibatch; a last partial minibatch dropped): resumed from 1260
  (`runs/p6-frozen-1.log` before), the trainer leveled at ~7.9 GB (1.6 M
  allocations), ~3.5–4 min an iteration (rollout 34–105 s, update 85–150 s).
  Stopped at 1413 (owner, 2026-10-04: "stop the about-even training and
  relaunch with the new rules and new engine"): its frozen mean stayed
  0.93–1.05 from 1270 on (best 1.05 at 1360) — even with the pair, no trend.
- **p7-politics** (2026-10-04 → 08, `runs/p7-politics`, 1361 → 1800, on
  `3cc7ae3`: rules-qa batch 48, copies alike in hand one choice): from
  p6-frozen's `best.pt` (1360) with `--politics` on (the default politics,
  above), the same pair, 5e-5 tapering, entropy 0.04; ~5.5 min an iteration
  (training ~190 s, evaluations the rest: "Faster iterations"). About +0.05 VP
  a 100 iterations, under one evaluation's noise. `p7-politics@1800` was served
  until p8 (`runs/served-prev-2026-10-10-p7at1800.zvp`) and p8's frozen
  opponent; its self-play showed the habits the bots never punish ("Opponents
  that punish").
- **p8-nudges** (`runs/p8-nudges`, 2026-10-08 → 10, 1801 → 2600): from
  `p7-politics@1800` (frozen opponent), observation `V3`, lr 1e-4, decks
  precons + anniversary + Sabbat + TWDA, owner starts 25%, nudges (rescue 6,
  pass 3, reaction 2, bleed 4, idle 3) full to 2200 then faded to 0 at 2600,
  chores on; ~3 min an iteration with evaluations. Best = last
  (`p8-nudges@2600`, kept as `runs/p8-nudges-best-2600.pt`): 1.41 VP a seat
  against p7 (3-seed mean; game wins 34% at 4 seats, 29% at 5), 2.56 / 2.93
  against the bots; rising throughout. Served in the fork since 2026-10-10
  (wasm built from `adfd807`; previous pkg and model in
  `runs/wasm-pkg-prev-2026-10-10`, `runs/served-prev-2026-10-10-p7at1800.zvp`).
  The nudges taught nothing that lasted: faded to 0, the behaviours went back
  to p7's (self-play, `examples/waste.rs`: rescues 0% p7 → 24% nudged → 1.6% at
  2600, owner 92%; bleed for nothing 18% → 4% → 11%, owner 0%; influence ended
  with a crypt draw offered 38% → 17% → 36%, owner 8%; cycling 95% → 76% → 96%,
  owner 63%; wakes with no block 86%, owner 29%) — bot opponents never punish
  their absence, so self-play gives no reason to keep them: permanent from p9.
  Per deck (`examples/decks.rs`, p8 self-play ~250 seats a deck / p8 at one
  seat against three p7): Ravnos 1.58 / 1.89 and Path of Power 1.40 / 2.11 near
  the top; Path of Death 0.85 / 1.01, Tremere 0.65 / 1.30, Hecata 0.46 / 1.29,
  Path of Caine 0.32 / 0.73, Reign of Stanislava 0.30 / 0.60, Salubri 0.28 /
  0.72, Path of Cathari 0.18 / 0.32 at the bottom: p9's weak decks.
- **p9-imitation** (2026-10-10 → 12, `runs/p9-imitation`, 2601 → 3400). The
  plan (owner, 2026-10-10): imitation for the skills self-play never rewards,
  permanent nudges, the learner on the weak decks but not the aggro ones
  (Ravnos, Gangrel, Brujah: strong in self-play, by the owner underperforming
  against him), opponents p7 + p8; what p8 lacked, by the owner: "going
  forward" (keep influencing when more vampires give a chance), and rush decks
  (Brujah, Hecata) not rushing backward to control their predator. From
  `p8-nudges@2600`, frozen opponents `p7-politics@1800` and `p8-nudges@2600`,
  lr 1e-4, entropy coefficient 0.048 (p8's last), `gw_weight` 5, decks
  precons + anniversary + Sabbat + TWDA, owner starts 25%, nudges rescue 6,
  bleed 4, idle 3 permanent (pass and reaction 0: imitation's), chores on, bc
  0.02 on `data/records` (37 games: 5456 choices trained, 894 held out —
  `…T08-31-11`, `…T19-47-18`, `…T20-32-01`, `…T22-52-40`), weak decks Salubri,
  Hecata, Tremere, Reign of Stanislava, Cathari, Caine, Death (12, 0, 5, 28,
  31, 32, 33) at the learner's seat on 40% of the tables, no bots; to 3400. The
  bar: p8 itself scores 2.16 (VPs 1.13 / 0.95, wins 27% / 18% at 4 / 5 seats).
  Stopped at ~2636 on an engine panic (Pack Alpha played out of turn, its
  retainer paying the defender's last pool: the employ ousted them and passed
  the turn, leaving a combat flow without a combat; fixed,
  `pack_alpha_paid_down_to_zero_out_of_turn_…`; found again in ~21 000 random
  games once), since which the pool survives a panic (Training above); resumed
  from 2620. Restarted from 2640 (`dfbf114` plus the end-of-round and
  bleed-against-prey fixes): draw 7, Cathari out of the weak decks (owner:
  "weak alright … remove it from the priority list"), 44 replays (6129 choices
  trained, 1008 held out with `…T15-48-57`); from 2800 (2026-10-11; 2800 scored
  2.50, `best.pt` 2.56): stuck 6, nightmare 4, the Ilonka peek (`7fedb25`), 31
  replays (the 17 stopped ones removed, six new; 6240 trained, the 1000 held
  out all trained on before: no longer a held-out measure); from 3000
  (`848e890`) and 3080 (`52b6399`: Alamut's controller, the pre-vote rotation).
  Ended at 3400 (2026-10-12), best = last: 3.54, re-measured 3.57; `waste` on
  p9@3400: stuck transfers 4.4% (35% at 2700), crypt draws 6% (the owner 11%).
  Served in the fork from 2026-10-12 with all six nudges (`ff2c6e8`; previous
  `runs/served-prev-2026-10-12-p8at2600.zvp`).
- **p10** (`runs/p10`, 2026-10-13 → 14, 3401 → 4000, on `971f7d3`: round 9's
  cards, the vocabulary remapped; prepared 2026-10-12 while the owner played p9
  and added cards). From p9@3400, opponents p8@2600 and p9@3400 frozen,
  nightmare 6, the rest as p9's. Weak decks 13, 12, 3, 32, 4 (Tzimisce,
  Salubri, Nosferatu, Path of Caine, Toreador) from p9's per deck (`decks.rs`
  on `f160de4`, p9@3400 self-play): the trained weak decks came up (Reign of
  Stanislava 0.30 → 1.52, Hecata 0.46 → 1.38, Tremere 0.65 → 1.01, Path of
  Death 0.85 → 1.14), Salubri and Path of Caine barely (0.62, 0.77), the others
  low (Tzimisce 0.57, Nosferatu 0.67, Toreador 0.79). Held out by name
  (`--bc-held-games`): the owner's games no network trained on, `…T12-31-56`,
  `…T15-45-47`, `…T21-28-54` (2026-10-11), `…T21-02-13`, `…T21-50-21`
  (2026-10-12); 30 replays, 5974 choices trained, 1309 held out (0.46 at 3401,
  a true held-out, against p9's trained-on 0.65). The bar its first evaluation
  (p8 and p9 as opponents, not comparable with p9's 3.57): 2.19 at 3420; best
  3840, re-measured 2.84; win shares against p8 0.53 → 0.59, against p9 0.47 →
  0.53; the held-out 0.43–0.49; `waste` on p10@3840: own nightmares 0 of 19
  (nightmare 6), crypt draws 10%. Per deck (`decks.rs` on `991b540`, p10
  self-play / at one seat against three p9): the five trained weak came up
  (Nosferatu 0.67 → 1.03, Toreador 0.79 → 1.03, Tzimisce 0.57 → 0.80, Path of
  Caine 0.77 → 0.83, Salubri 0.62 → 0.68), the ones dropped from the list fell
  back (Path of Death 1.14 → 0.65, Tremere 1.01 → 0.86, Reign of Stanislava
  1.52 → 1.19): the weak list moves the decks round, not up. Served 2026-10-14
  in the fork (nightmare 6 in the wasm nudges; the previous, p9's,
  `runs/p9-imitation/best.zvp`) and on engine.krcg.org the same day (nightmare
  6 in the server's bots), the fork's only engine since `243bf6b` (P5: the wasm
  gone, bot settings in `crates/server/src/games.rs`).
- **p11** (`runs/p11`, `../zero-vtes-p11` at `45debc9`, round 11's cards;
  2026-10-14 → 16, 3841 → 4400). The question (owner): until a seat is ousted,
  damage between cross-table seats "only benefits the bouncing prey" — accept
  the block, keep the combat harmless, yet oust a threatening cross-table seat
  — so restraint should emerge, not be nudged; it pays only once both sides
  learn it, which frozen opponents never do: `--partner-rate 0.5` (a second
  learner across the table on half the league tables, ~22k samples an iteration
  instead of ~15.5k; 13–17 GB resident, no leak). No weak decks (owner: p10's
  per-deck order matches the players' sense of strength); Wider View (findings
  E7) left to emerge (owner: no nudge). From p10@3840, opponents p9@3400 and
  p10@3840 frozen, the rest as p10: `--lr 1e-4 --entropy-coef 0.048 --gw-weight
  5 --decks precons,anniversary,sabbat,twda --starts data/records
  --start-tables 0.25`, nudges rescue 6 bleed 4 idle 3 draw 7 stuck 6 nightmare
  6 (`--nudge-until 0`), `--bc data/records --bc-coef 0.02`, the five held-out
  games named, `--bots-every 0`. Best **2.67 at 4340** (re-measured; 4 seats
  1.34 VP a seat, 0.32 game wins; 5 seats 1.11, 0.21), from 1.93 at 3860; win
  shares against p9 0.50 → 0.55, against p10 0.47 → 0.54; held-out imitation
  0.50 → 0.44 (the RL pulls off the owner's style). **No restraint emerged** —
  pairing alone does not teach it (`waste` on `45debc9`, 160 games, p10@3840 →
  p11@4340, while no seat is ousted: blood the opposing minion lost a combat,
  adjacent / cross-table, 0.95 / 1.19 → 0.95 / 1.21 at 4 seats, 0.98 / 1.18 →
  0.95 / 1.29 at 5; the pool a bleed took from a cross-table seat 0.14 → 0.18).
  Wider View **learned without a nudge** (draw-and-remove taken, of the offers:
  precons 39% → 16%, TWDA 29% → 15%; the owner 12%). At one seat against three
  p10 (`decks.rs`, 600 games) 0.99 VP a seat. `humanset` (the five held-out
  games, starts of this run too, bots p10, 20 games a position): p10 / p11
  agree with 46.1% / 45.3% of the owner's 1,309 choices; won from a quarter in
  33% / 43%, halfway 52% / 54%, three quarters 91% / 83% (±10 at 100 games).
  Humans against p10 (the harvest, 14 games) won 11 (79%), 2.79 VP a seat, the
  bots 0.33 — p11's bar; against p11 on its first served day (2026-10-17, 7
  games) 4 seats of 7 (57%), 1.86 VP a seat, the bots 0.43. Exported
  `runs/p11/best.zvp` (sha256 `b9035931a527`), served on engine.krcg.org from
  the 1.6.0 deploy (owner, 2026-10-16), kept at 1.8.0 over p12, replaced by p13
  (2026-10-19); the previous `runs/served-prev-2026-10-16-p10at3840.zvp`
  (`7aaa9528fbf8`).
- **p12** (`runs/p12`, `../zero-vtes-p12` at `cb3b4a6`, round 13's cards, E25
  with the futile-block mask; 2026-10-16 → 17, 4341 → 4842 of a planned 4900):
  from p11@4340, opponents p10@3840 and p11@4340 frozen; imitation winners only
  (owner, 2026-10-15) over `data/records` and the 15 harvested games played out
  (8,381 choices, 3,058 harvested; the same five held out), the same games as
  starts; the rest as p11 (`--partner-rate 0.5`, the nudges, no weak decks).
  Killed at 4842 by memory pressure under a parallel release build (no
  traceback; the owner chose to evaluate the best rather than resume). Best
  **2.51 at 4800** (4 seats 1.19 VP a seat, 0.29 game wins; 5 seats 1.21,
  0.25), from 1.84 at 4360; win share against p11 0.48 → 0.50; held-out
  imitation 0.54 → 0.51 (the winners-only imitation holds the owner's style).
  **Not served**: it does not beat p11 (owner's rule) — at one seat against
  three p11 (`decks.rs` on `cb3b4a6`, 600 games) 0.88 VP a seat, ~2 standard
  errors under even; `humanset` (bots p11) agreement 50.2% against p11's 45.3%,
  won 162 / 300 against 166; Wider View regressed (27–30% of the offers on the
  precons, p11 16%), cross-table minions sent to torpor at 4 seats 23% (p11
  13%). Exported `runs/p12/best.zvp` (`2dbb922666c9`), kept. Settled for p13
  (owner, 2026-10-17): the pairing off (two runs, no cross-table restraint,
  ~25–30% of an iteration); from p11@4340, not p12; Wider View still without a
  nudge (p11 had learned it).
- **p13** (`runs/p13`, `../zero-vtes-p13` at `331ce66`, rules-qa 82.1's block
  loop; 2026-10-18 → 19, 4341 → 4900, ~105–115 s an iteration): from p11@4340,
  opponents p11@4340 and p12@4800 frozen; no pairing; `--bc-every 4`; imitation
  winners only over `data/records` and 17 harvested games (7,052 choices; held
  out 1,349 in five games: four of `data/records` and `e33fab34fee8ca4b`), the
  20 harvested games as starts; nudges, lr and entropy as p12. Nine harvested
  games left out of imitation and the held-out set: six since 82.1 (a block
  window the old engines reopened), three drifted earlier (rules-qa 79.1,
  86.1); four of them kept as starts (their readable prefixes). Seventeen games
  hit a Dual Form panic on a clanless vampire (seven in training, dropped;
  fixed in round 16). Best **2.30 at 4740** (4 seats 1.25 VP a seat, 0.27 game
  wins; 5 seats 1.19, 0.23), from 1.80 at 4360; held-out imitation 0.52 → 0.47.
  On `331ce66`: at one seat against three p11 (600 games) 0.97 VP a seat, even
  within a standard error (Nosferatu 0.20, Path of Cathari 0.30); `humanset`
  (bots p11): agreement 47.1% against p11's 44.8%, won 139 / 300 against 125 (a
  quarter in 40% / 25%, halfway 33% / 39%, three quarters 66% / 61%); Wider
  View back to 11–15%. **Served** (owner, 2026-10-19: even with p11 head to
  head, better from the owner's positions), `runs/p13/best.zvp`
  (`df66d54badf3`), the previous `runs/served-prev-2026-10-19-p11at4340.zvp`
  (`b9035931a527`).
- **p14** (`runs/p14`, `../zero-vtes-p14` at `10b4369`; 2026-10-21 → 22, 4741 →
  5300, ~111 s an iteration, no panic, nothing truncated): the owner's lever of
  2026-10-21 for "Combat play by deck" above — the deck mix and an imitation
  weight toward the rush and wall decks, no wake nudge. From p13@4740 (served),
  opponents p13@4740 and p11@4340 frozen; as p13 otherwise (no pairing, the
  nudges, lr, entropy, `--bc-every 4`, the same five held out). The learner's
  seat on one of Hecata, Nosferatu, Banu Haqim, Brujah, Gangrel, Salubri,
  Tzimisce (`--weak-decks 0,3,7,8,9,12,13`) on 35% of the league tables
  (`--weak-rate 0.35`), its opponents from the whole pool, and in a start at a
  seat playing one; imitation over `data/records` and the 26 harvested games
  that read whole (of the 35 counted): 9,231 choices, 3,263 on those decks
  (none on Banu Haqim), drawn 3 times as often (`--bc-weak-weight 3`). Best
  **2.13 at 5280** (4 seats 1.01 VP a seat, 0.24 game wins; 5 seats 1.04,
  0.22), from 1.70 at 4760; held-out imitation 0.49 → 0.52. On `10b4369`: at
  one seat against three p13 (600 games) 1.00 VP a seat, even; on the seven
  decks Brujah 1.43, Hecata 0.87, Tzimisce 0.93, Gangrel 0.83, Nosferatu 0.57,
  Banu Haqim 0.40, Salubri 0.40 (p13 against p11: 0.97, 0.77, 0.93, 1.13, 0.20,
  0.37, 0.90; 30 seats a deck, a standard error ~0.2); `humanset` (bots p11):
  agreement 51.3% against p13's 47.1%, won 140 / 300 against 139 (a quarter in
  36% / 40%, halfway 33% / 33%, three quarters 71% / 66%). The lever moved the
  rushes of the decks that already rushed (own turns with one declared, p14 /
  p13: Brujah 41% / 29%, Hecata 56% / 42%; the others within 5 points, Tzimisce
  16% / 29%) and none of the combats' outcomes (the opposing minion to torpor
  or burned within 3 points of p13 on every deck); Wider View regressed as in
  p12 (18–23%, p13 12–14%), cross-table minions sent to torpor at 4 seats 16%
  (p13 10%). **Served** (owner, 2026-10-22: at least even with p13, its
  behaviour maybe better), `runs/p14/best.zvp` (`a78e74292acb`), the previous
  `runs/served-prev-2026-10-22-p13at4740.zvp` (`df66d54badf3`).
- **p15** (`runs/p15`, `../zero-vtes-p15` at `788eae4`; 2026-10-23 → 25, 5281 →
  5840, no panic, nothing truncated, 100–200 s an iteration on a machine shared
  with builds): from p14@5280 (served), opponents p14@5280 and p13@4740 frozen;
  one change from p14: `--shaping runs/phi-p14.json` (coefficient 1), Φ the
  linear fit of the 26 position features over the whole game on p14's 1,024
  self-play games ("The critic against the position" above: a VP +0.30, the
  seat's pool +0.58 a 30 pool, the blood on its ready minions +0.41 a 20 blood,
  the prey's pool −0.32), each learner decision gaining Φ(next) − Φ(this) and
  the last −Φ(last) (γ 1, Ng 1999); the critic re-warmed 10 iterations;
  imitation 8,986 choices (`ec8d47141b6b38e9` left out, read partway since
  round 20's Anarch Revolt change). Best **2.09 at 5820**, from 1.77 at 5300.
  Against the served p14 on `8fcaaa3`: at one seat against three p14 (600
  games) 1.03 VP a seat; `humanset` (bots p14) agreement 48.9% against p14's
  51.4%, won 132 / 300 against 124; `waste`, beyond two standard errors: worse
  — influence ended with a transfer or draw possible and none made 62% / 58% at
  5 seats and 63% / 55% on TWDA, wakes with no block after 85% / 80% on TWDA;
  better — Wider View used 7–10% / 19–20%, rescues 3–6% / 2–4%. The shaping cut
  the rushes instead of raising them (own turns with one declared, p15 / p14:
  Brujah 24% / 41%, Hecata 33% / 56%, Salubri 5% / 21%, Banu Haqim 5% / 11%,
  Gangrel 1% / 8%; Tzimisce 26% / 18%) — Φ paid at once for the blood a combat
  costs and only later for what it took — and did not help the critic (with Φ
  added back, `train.position`, by quarter: 0.03 / 0.09 / 0.29 / 0.64 against
  p14's −0.03 / 0.07 / 0.34 / 0.71). **Not served** (owner, 2026-10-25: it did
  not improve as hoped); the shaping dropped. Exported `runs/p15/best.zvp`
  (`ee41b58bef28`), kept.
- **d16** (`runs/d16`, `../zero-vtes-d16` at `fed6d40`; 2026-10-25 → 26, 200
  iterations, no panic, nothing truncated, ~290 s an iteration, rollout ~80,
  update ~210): the owner's first p16 step (2026-10-25: "a bigger network
  distillation before we launch p16"), a fresh network at d 192 and 4 layers
  (p14: 96 and 2) distilled from p14@5280, `--teacher runs/p14/best.pt` — the
  learner plays p14's tables (its deck mix, weak decks, starts and nudges), p14
  labels the decisions it reaches, the policy term KL(p14 ‖ learner); opponents
  p14@5280 and p13@4740 frozen, evaluated every 10. KL 1.09 at the first
  iteration, 0.14 at 100, 0.13 from 160; against the frozen pair **1.13 / 1.02
  VP a seat at 4 / 5 seats at 180** (best). Against p14 on `fed6d40`
  (`best.zvp` `714054ed0ba2`): at one seat against three p14 (600 games) 0.92
  VP a seat; `humanset` (bots p14) agreement 47.6% against 51.4%, the human's
  choice more probable (mean log-probability −1.54 against −1.76), won 126 /
  300 against 126; the ledger (400 self-play games, d16 / p14, two standard
  errors ~0.04 on A&B, ~0.10 on R): A&B a round 2.09 / 2.14 at 4 seats, 2.10 /
  2.20 at 5, 1.72 / 1.73 on TWDA; its prey's R a turn −3.46 / −3.35, −3.54 /
  −3.59, −2.88 / −2.96. p14's play in a network with room to grow, no gain of
  its own: **not served**, p16's start (owner, 2026-10-27).
- **p16** (`runs/p16`; owner, 2026-10-27: RL from d16). From d16@180 (the
  seats' context read from the start), on main `6ad5083` (round 22's cards); as
  p14 but `gw_weight`, left at the default of the time by omission (2, d16 too;
  5 from p9 to p15, and the default since): another reward and selection score.
  The owner's games twice as often, no shaping, d16 for p13 among the frozen
  opponents, 500 iterations (the paths absolute into the main checkout's
  `runs/` at launch): `train.ppo --league true --out runs/p16 --resume
  runs/d16/best.pt --d 192 --layers 4 --attention-budget 8000000 --opponents
  runs/p14/best.pt,runs/d16/best.pt --decks precons,anniversary,sabbat,twda
  --bc data/records,data/server-records --bc-coef 0.02 --bc-every 4
  --bc-held-games H --bc-weak-weight 3 --bc-games-weight zero-vtes-replay=2
  --starts data/records,data/server-records --start-tables 0.25 --weak-decks
  0,3,7,8,9,12,13 --weak-rate 0.35 --rescue-bonus 6 --bleed-bonus 4
  --idle-bonus 3 --draw-bonus 7 --stuck-bonus 6 --nightmare-bonus 6 --lr 1e-4
  --iterations 680 --eval-every 20 --bots-every 0`, H the five held games
  (2026-10-11T12-31-56, 2026-10-11T15-45-47, 2026-10-12T21-02-13,
  2026-10-12T21-50-21, e33fab34fee8ca4b, comma-joined). Launched 2026-10-29
  from `../zero-vtes-p16` at `a4d184d`, before the trainer read players' games
  only: `--starts` as the list of the 150 harvested games the pool reads (49
  abandoned, unfinished or test clients' games read under 10 decisions), and
  imitation over the raw harvest, two Succubus Club demo games among its
  targets (their human seat replays the network). Stopped at iteration 478 of
  680 (owner, 2026-10-30: the machine's resources, going nowhere), not served,
  not measured against p14 on the ledger. Its best stayed its first evaluation,
  iteration 200 (1.067 on the mean of the selection score against p14 and d16,
  0.84 and 0.76 VP a seat at 4 and 5 seats); later ones ran 0.61–0.87 and
  0.51–0.74. Imitation memorised: on the means of the first and last 20 logged
  iterations, accuracy on the trained human choices 0.64 → 0.78, on the
  held-out games 0.57 → 0.50. At the same learning rate it moved the policy
  about twice as far as p14 (kl 0.021 and 0.011 at 1e-4). No evidence against a
  larger network, or for one.
- **p17** (`runs/p17`, `../zero-vtes-p17` at `08c5665`; 2026-11-03 → 04, 5281 →
  5580, no stop fired, no panic, nothing truncated in training (two evaluation
  games); ~2.3 minutes an iteration, the machine's idle sleep stretching it to
  ~26 hours). The owner's run (2026-11-02, through Z-main: search dropped, no
  shaping): half the tables start from a human's win, the learner in the
  winner's seat, where play like theirs has paid against these bots
  (`--start-winner`, `--start-tables 0.5`). From p14@5280 (`a78e74292acb`), as
  p14 otherwise: λ 0.95, `gw_weight` 5, the nudges, the weak decks, imitation
  as p16's (the owner's games twice as often). Around it: p14 the only frozen
  opponent, at the served temperature, every seat at it in evaluation
  (`--served-temperature 0.5`); a flat learning rate, 3e-5; `--entropy-coef
  0.033` to start, the controller on; `--best-seeds 1`, a checkpoint per
  evaluation. Also moving, untested: the seats' context layer from zero; 266
  vocabulary rows p14 never met (21 of the 171 TWDA decks), random in frozen
  p14; 5-seat tables at about a quarter. Held out of imitation and starts: 13
  human wins reading whole, four of `humanset`'s five and every second of the
  18 wins over p14; judged on those nine, which p14 never met (owner,
  2026-11-03). Trained from 46 of the 74 human wins (39 whole, 7 partial with a
  position digest; 8 short and 6 undigested left out), ~21,000 decisions to
  start from, ~450 tables a game. The command: `train.ppo --league true --out
  runs/p17 --resume runs/p14/best.pt --opponents runs/p14/best.pt
  --served-temperature 0.5 --decks precons,anniversary,sabbat,twda --gw-weight
  5 --lam 0.95 --lr 3e-5 --lr-final 1 --entropy-coef 0.033 --bc
  data/records,data/server-records --bc-coef 0.02 --bc-every 4 --bc-held-games
  H --bc-weak-weight 3 --bc-games-weight zero-vtes-replay=2 --starts S
  --start-tables 0.5 --start-winner true --held-games 208 --keep-evals true
  --weak-decks 0,3,7,8,9,12,13 --weak-rate 0.35 --rescue-bonus 6 --bleed-bonus
  4 --idle-bonus 3 --draw-bonus 7 --stuck-bonus 6 --nightmare-bonus 6
  --iterations 5580 --eval-every 20 --best-seeds 1 --bots-every 0`, H and S in
  `runs/p17-held.txt` and `runs/p17-starts.txt`, the paths absolute into the
  main checkout. The stops, fixed before launch against p14 measured so first
  (`runs/p17-baseline`: `vs_held` 0.495, the nine 0.493, 2.24 against p14):
  none fired (`vs_held` 0.49–0.57 over 15 evaluations, its three-evaluation
  mean 0.506–0.521; against p14 1.86–2.41, never three running under 1.94; the
  starts' wins 0.47 → 0.58). No fall and no climb against p14: 2.22 at the
  first evaluation, 2.13 on average (by thirds 2.10, 2.24, 2.04) against the
  bar's 2.24, where p13 to p15, at 1e-4 tapering, opened ~0.4 down and climbed.
  Step 4 on `08c5665`, on the last evaluation (5580, `f426338f6c80`) and
  `best.pt` (5400, `b3d990e32f8b`), p14 alongside (`runs/p17/step4/`: its
  README the commands, `summary.py` the numbers): at one seat against three
  p14, `decks.rs` 1.03 VP a seat (600 seats, ± 0.12), and at the served
  settings 1.065 ± 0.09 VP and 25.5% ± 2.8 wins (1,000 games, the scratch
  `runs/search-2026-11-02/rust/ladder.rs`): level. `humanset` (20 games a
  position, bots p14): on the nine, won 245 of 540 against p14's 218 (45.4% /
  40.4%; better on seven games, worse on one; t 2.4 on 8 degrees of freedom
  game by game; `best.pt` 236); on the 13, 347 and 313 of 780 (`best.pt` 349);
  agreement 49.4% / 48.4%. The ledger (400 self-play games each at the served
  settings): A&B a round 2.41 / 2.45 at 4 seats, 2.42 / 2.51 at 5, 2.05 / 2.08
  on TWDA; its prey's R a turn −3.48 / −3.70, −3.79 / −3.97, −3.33 / −3.48; its
  own R −1.21 / −1.06 at 4, −1.15 / −0.93 at 5, −0.76 / −0.58 on TWDA: worse
  beyond two standard errors on A&B at 5 seats, the prey's R at 4 and 5 and its
  own at 5 and on TWDA. The critic (`train.position`, by quarter, one run
  each): its own self-play 0.15 / 0.22 / 0.43 / 0.74, p14's 0.16 / 0.17 / 0.37
  / 0.68; the human records (50,402 decisions) −0.18 / −0.06 / 0.14 / 0.57,
  p14's −0.23 / −0.12 / 0.10 / 0.48. It wins more from the humans' positions
  (+5.0 points on the nine, p ≈ 0.05 game by game, within the rates' two
  standard errors of 6.0) and in self-play harms its prey less and loses more
  of its own: the not-degrading test fails, so the run went to the owner
  (2026-11-04): served as a trial (owner, 2026-11-06; back to p14 if the human
  games worsen, read in the harvest's total report
  (`data/server-records/reports/total.md`, "Humans against the bots") by
  weights against p14's own: humans winning 79% ± 13 points and scoring 2.79 ±
  0.39 VP a seat (two standard errors) over its 39 counted games; p17's games
  with a human on a deck from `GET /catalog/taught` or a bot on a TWDA deck
  read apart (each game's decks: the daily reports' "Every game" rows), p14's
  having neither: its bots played precons only), `runs/p17/last.zvp`
  (`f426338f6c80`), the previous `runs/served-prev-2026-11-06-p14.zvp`
  (`a78e74292acb`).

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

Definitions: 1240 library, 726 vampires; 3589 Rust tests, 72 Python tests

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
  contest during their own unlock phase, so no player orders both sides (a
  contest with oneself is masked where chosen, the incoming title lost where
  forced: rules-qa 100.1); per-object binary pay/yield decisions in canonical
  order; forced yields elided (§4.4).
- **Window census vs per-card reality**: the matrix is generated, then
  hand-reviewed card by card and arbitrated by the owner (rules-qa Batch
  2); the generator reproduces the reviewed matrix (§6).

## Process: rules Q&A pass

Rules questions go to the project owner (VEKN judge) in batches organized
by subsystem; the answers are recorded in `docs/rules-qa.md`, each backed
by a scenario test so the engine's behavior and the recorded ruling can
never drift apart.
