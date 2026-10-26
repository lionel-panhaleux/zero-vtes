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
  full-size precons (Fifth Edition 7 · Anarch 4 · Companion 3, from KRCG
  `_set` bundle codes: 379 unique cards), the reviewed anniversary decks
  (`Deck::anniversary`), the Sabbat V5 precons (`Deck::sabbat`) and the
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
  the caller last: rules-qa 64.12)
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
- **Behavior**: a card is a set of
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
  contents and the last public choices, §11.2), transformer trunk,
  auto-regressive head over options; the viewer's remaining deck pooled
  into the context token (`train/model.py`); reward = tournament scoring
  (VPs, GW) — a potential-based shaping by the position, tried in p15,
  cut the rushes and was dropped (§11.2 "p15"; `--shaping` kept), as was
  a survival shaping; league = current agent + snapshots + exploiters +
  scripted bots, PFSP (`train/league.py`); decks × seats sampled per
  episode.
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
  hindrances, pool bled) — what the
  history's last 32 choices cannot hold. Public only (the legal view's
  encoding equals the full state's, `crates/env/tests/obs.rs`). The policy
  and the critic add it to the context token through a layer started at
  zero (`train/model.py`, `crates/policy`'s `Net`, `ctx.weight` in a
  `.zvp`): a network saved before it plays unchanged, and its first resume
  starts the optimizer's moments afresh. Laid out feature by feature,
  each across the seats (`context::names`, owner): a feature
  is appended at the end, never inserted (`crates/env/tests/context.rs`
  pins the names), so an older layer reads it as zero columns at the end
  (`remap_vocab`, `Net::load`).

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
checkpoint. A mismatch names the test, the
first diverging seed (and how many differ) and the 64-decision range of its
first divergence, with the command tracing it (`FUZZ_TRACE=1 SEED_FROM=n`,
decisions numbered). An intended change re-blesses: `GOLDEN_BLESS=1 cargo test
--profile ci --test fuzz`; a missing file fails, a partial sweep (`SEED_FROM`)
skips the check. Identical in every profile; the state hash is stable on
64-bit little-endian targets (lengths hash as `usize`).

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
   (`Seat.half_vps`); `GameState::game_win`
   = 2+ VP and a strict lead, ousted or not (a tie is a draw). Tournament
   withdrawal: 0.5 VP, an unlock-phase decision (rules-qa 95); network
   seats never take it (`zvtes_policy::untrained`, in training and
   served), scripted styles score it under Pass, `random` plays any legal
   option.
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

The engine reference by mechanism is [`docs/engine.md`](docs/engine.md)
(owner, 2026-10-14: the engine guide for implementers and reviewers). What
stays here is the policy side: the scripted bots, the observations and the
training runs.

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
  remembers its attempts). An attempt that cannot reach the action's stealth
  (`Engine::block_can_reach`) is offered (rules-qa 73.8) but never made: it
  scores below the pass (owner, 2026-10-15).
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

**Observations** (`zvtes_env::obs`: `encode`, `Obs`, `schema_json`)
- One request as its seat sees it: tokens — seats in relative order (pool,
  VPs, Edge, zone sizes, prey/predator, phase budgets), minions in play
  (contested ones too), the viewer's virtual hand (`Choice::Play` indexing),
  cards in play (host as a pointer), uncontrolled slots (the viewer's face up,
  others' `HIDDEN` but a banished one), what remains of the viewer's own
  library and crypt (`T_DECK`, since 2026-09-30: a token per card with its
  copies, never the order — the decklist at the start, the cards left
  afterwards), the political referendum under way (`T_REFERENDUM`, since
  2026-09-30: its card — the political action card or the card granting
  the action — and the votes cast for and against so far; a chosen clan as
  a second one; its terms on the tokens they name: points allocated on
  seat and card tokens, chosen Methuselahs, minions and cards flagged), a
  last context token (window; action
  kind, acting minion, bleed target, stealth, bleed amount; combat minions,
  range, round; referendum caller, calling minion or diablerist). Each token:
  ids `[type, vocabulary, pointer, pointer]` and `TOK_F` floats. Options:
  ids `[choice kind, vocabulary, pointer ×3]` (hand card, actor, target; the
  source of an ability; the minion, seat, card or slot named) and `OPT_F`
  floats (amount, favor, mode, zone, title contest, action kind). A card or
  minion an option names carries its identity too (a rider's card may have
  left play). Vocabulary: crypt, library, hidden, windows, clans.
- `crates/env/tests/obs.rs`: the encoding of the full state equals the encoding
  from the seat's legal view (every 2nd decision of bot, random and fuzz-table
  games; a planted leak of face-down vampires fails it), and every pointer
  names a token. At most 63 tokens and 104 options met.
- Review points: ash heaps, removed cards and cards stored on cards are sizes
  or absent, not tokens; `ActionCtx::card` (a card
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
  replaces the tables new games take (seats, optional decks; 4 or 5 seats,
  tournament tables, or 2, the two-player variant), the default table once it
  runs out. `observe()` pads the batch (with an `agent` column: the network to
  ask); `act(actions)` advances every game in parallel (rayon) to its next
  policy decision and returns the games finished (slot, game number in start
  order, seed, time limit, decks, seats, half-VPs, turns, decisions,
  truncated, log). Each game draws its time limit in `time_limits` (60–90
  player turns inclusive, owner 2026-09-28: player turns, so a table losing a
  Methuselah early plays more rounds; `Engine::time_limit`, default
  `TURN_LIMIT` for the engine's own tests): 0.5 VP per survivor, counting
  toward the game win (2 VP + 0.5 wins with a strict lead). `replay(seed,
  decks, log, time_limit)`. A game over `budget` decisions is cut (§11.3).
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
  wins first; 1 before); entropy bonus 0.02 (0.01 before; 0.03 made the entropy climb, 0.62 → 0.89 in 30 iterations, with every evaluation flat or down), adaptive since the TWDA run: scaled ×1.1 or ÷1.1 after each update toward `entropy_target` 0.65 (every run peaked against the bots near 0.64 and declined under ~0.6), within [0.001, 0.2], kept in the checkpoint; truncated
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
  drawn by PFSP among the bots (`bots`, all three by default; at most `table_bots` a table) and the last `opponents_kept` (4)
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
  present. The pool refuses tables other than 2, 4 or 5 seats. 1024 games in
  parallel since 2026-09-29 (512 from the third league run, 256 before:
  one learner seat a table halved the samples). An opponent seat is a bot
  with probability `bot_seats` (0.6, since 2026-09-29; before, all bots
  and snapshots were one PFSP pool, ~30% bots), else a snapshot, PFSP
  within the group. The learning rate tapers linearly from the run's
  first iteration to `lr_final` (0.1) × `lr` at the last; the evaluation
  with the most VPs against the bots is kept as `best.pt` (a checkpoint
  like `model.pt`, resumable): the mean of aggro/wall/aggro and
  aggro/wall/aggro/wall, a candidate within `best_margin` (0.1 VP) of the
  best measured again on `best_seeds` − 1 (2) more evaluation seeds and
  kept on the mean (since 2026-09-29; a single aggro/wall/aggro evaluation
  before).
- The omniscient critic (`critic`, on since 2026-09-29; `model.Critic`,
  `obs::encode_critic`, the pool's `critic=True` adding `critic_tok_*`):
  the learner's values come from a separate network reading `encode`'s
  tokens plus every other hand and every face-down uncontrolled vampire,
  trained on the same returns in the same minibatches (its own optimizer,
  the same learning-rate taper), started as a copy of the policy's
  encoder and value head; the policy still plays from the legal view (its
  own value head untrained since). Saved in checkpoints (`critic`,
  `critic_optimizer`). A new critic trains alone for its first `critic_warmup` (10)
  iterations, the policy frozen: copied from the policy it values a new
  input poorly, and the first critic run fell from 1.36 / 1.51 to 1.20 /
  1.24 VP in 10 iterations without it.
- Update speed (2026-09-29): the update is GPU-bound (Python collation
  ~5%); options were padded to the batch's largest set (mean 5, max
  100+), so only the offered options are scored now (ragged, identical
  logits and parameters), and minibatches are bucketed by token count
  (chunks of 8 minibatches sorted, then shuffled: every sample once an
  epoch). Measured A/B under the same load: 1.4–1.5× faster; minibatch
  1024 is the fastest (512 and 2048 slower, 4096 exhausts GPU memory).
- Evaluation against humans: the harvest report's human win rate by weights (`scripts/games_report.py`),
  and the positions where a human beat the bot (`examples/humanset.rs`:
  games a human won, played out and never trained on; per network the
  agreement with the human's choices, and from a quarter, half and three
  quarters of each game the network in the human's seat against the
  served bots, its wins).
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
- TWDA run (2026-09-29: precons + the 13 TWDA decks, clan-balanced, from
  iteration 90 to 300; evaluated on those decks, fair seating, 60–90-turn
  limits — the start measured 1.13 VP / 26% game wins against
  aggro/wall/aggro, 1.23 / 24% against aggro/wall/aggro/wall, 2.62 / 59%
  against random seats). Five evaluations rising to 1.44 / 32% and 1.51 /
  30% at 150 (entropy 0.64; random seats 2.90) — the first gain on the
  fair evaluation — then back down as the entropy fell (0.57 at 200, 0.42
  at 300): 1.12 / 25% and 1.14 / 20% at 300, 0.87 VP against the latest
  snapshot. `best.pt` keeps iteration 150. Hence the adaptive entropy
  bonus. Iteration 150 re-measured on three other evaluation seeds: 1.32–1.44 VP
  (30–34%) and 1.42–1.58 (27–32%) — a real peak, the best model so far
  (`runs/p4-twda/best.pt`).
- Resumed from that `best.pt` with the adaptive entropy bonus: at the full
  learning rate it fell to 1.19 / 1.20 in 10 iterations; at 1e-4 (tapering)
  1.48 / 1.40 at 160, then 1.28, 1.12, 1.20, 1.18, 1.14, 1.13 against
  aggro/wall/aggro (1.08 against aggro/wall/aggro/wall at 220), with the
  entropy held at 0.63–0.70 — stopped at 220. The falling entropy is not
  the cause: the policy drifts. The training tables' pairwise beat rates
  stayed flat all along (aggro ~0.5, wall ~0.68, random ~0.77, snapshots
  0.47–0.57) while the evaluation rose and fell: training does not steer
  toward beating the bots; a good checkpoint is found, not converged to.
  Next levers (owner to choose): larger batches (less noisy updates), more
  bot seats in training tables, the omniscient critic, checkpoint selection
  on several evaluation seeds.
- Critic run (2026-09-29, all four levers: the omniscient critic with a
  10-iteration warm-up, 1024 games, 60% bot seats, 3-seed selection; from
  the TWDA run's `best.pt`, learning rate 1e-4 tapering, iterations 151 to
  350, ~70 s an iteration). The warm-up held the start (1.44 / 1.51 VP;
  3-seed mean of both bot tables 1.43); then a dip as the critic and the
  policy adapted — 1.20 / 1.26 at 170, 0.49 / 0.43 at 190 — and a steady
  climb: 1.33 / 1.35 at 250, 1.60 / 1.55 at 300, 1.70 / 1.71 at 340 (39%
  / 34% game wins), 1.71 / 1.67 at 350, still rising; against the latest
  snapshot ~1.0 (an even share). `best.pt`: iteration 340, 3-seed mean
  1.68. The first run to improve steadily rather than wander.
- Continued (2026-09-29/30, the same run to iteration 600, learning rate
  5e-5 tapering): no dip, a steady climb — 1.75 / 1.91 at 400, 1.90 / 2.14
  at 450, 2.06 / 2.15 at 520, 2.11 / 2.12 at 580 (51% / 43% game wins),
  2.12 / 2.22 at 600 (50% / 47%), flattening from 520 on; ~3.1 VP and 70%
  game wins against random seats, ~1.0 against the latest snapshot.
  `best.pt`: iteration 580, 3-seed mean 2.18.
- What it does not do (2026-09-30, 200 five-seat games of iteration 580
  against itself): it never rescues — a cross-table, prey's, predator's or
  its own vampire in torpor alike (9 rescues in all, probability ~0 where
  a uniform pick gives 0.11–0.12) — and never diablerises (12); a torpid
  vampire leaves torpor by itself about half the times it can. Iteration
  150 still rescued 32 times and diablerised 118 times: training removed
  both. No table-balance play is learned from these opponents and rewards.
- Remaining-deck tokens (2026-09-30, `T_DECK`): ~41 → ~71 tokens a
  decision. A checkpoint from before loads with the new token type's
  embedding at zero (`model.remap_vocab`) and fresh optimizers. Run
  (`runs/p4-deck`, then `runs/p4-deck2` on the rules of Batch 46 from
  iteration 621; from the critic run's `best.pt`, learning rate 5e-5
  tapering, ~100–160 s an iteration): no dip — 1.98 / 2.18 at 590,
  2.11 / 2.28 at 620, then a plateau above the one before: 3-seed means
  2.14 (640), 2.25 (690), 2.29 (720), 2.29 (760, 800), 2.31 (820); 5-seat
  game wins around 50% from 700 on (2.41–2.46 VP at 760–810). `best.pt`:
  iteration 820, 3-seed mean 2.31 (2.19 / 2.38 VP, 53% / 51%).
- Votes (2026-09-30, owner's first game: the bots did not vote against
  referendums against their interest): the observation did not name the
  referendum's card, terms or tally, and iteration 820 votes for and
  against with the same probability (0.24 / 0.24 on its own referendums,
  0.31 / 0.31 when the terms put points on its seat, 200 five-seat
  self-play games) — a coin flip. `T_REFERENDUM` and the term features
  added; retrained from there (`runs/p4-vote`, iterations 821–1070,
  learning rate 5e-5 tapering, ~125 s an iteration): no dip, a higher
  plateau — 3-seed means 2.33 (830), 2.36 (890), 2.39 (910), 2.44 (960),
  2.43 (970, 1000, 1010), 2.44 (1020, 1060); single evaluations up to
  2.63 VP / 59% game wins at 5 seats (960) and 62% at 4 seats (1050).
  `best.pt`: iteration 1060, 3-seed mean 2.44 (2.34 / 2.48 VP, 59% /
  53%). Its votes are still a coin flip (0.22 / 0.23 on its own
  referendums, 0.26 / 0.27 when hit). Voting well does not pay at these
  tables: iteration 880 with one seat voting by rule (for its own
  referendums, against those putting points on it) scored 0.91 ± 0.05 VP
  against 0.97 ± 0.05 unchanged (1000 five-seat games each; the rule
  changed 725 votes in all). Learning votes needs politics in the
  training pool (political TWDA decks), not a richer observation alone.
- Self-play against snapshots (2026-10-02/03, `runs/p5-self`, iterations
  1061–1310, from the vote run's `best.pt`): at the vote run's end the
  bots had stopped teaching — beat rates aggro 0.72, wall 0.81, random
  0.86, flat over 70 iterations, every snapshot ~0.5, yet 60% of
  opponent seats were bots. So `--bots aggro --table-bots 1 --bot-seats
  0.1 --opponents-kept 8 --lr 5e-5 --entropy-coef 0.04` (precons + TWDA):
  at most one aggro a table (~⅓ of tables), the rest the learner's last
  8 snapshots by PFSP. Flat to slightly up, within the noise: 4 seats
  2.16–2.42 VP (55–61% game wins) throughout, 5 seats ~2.4 early and
  ~2.6 late (55–60%), random seats ~2.9, ~1.0 against the latest
  snapshot; aggro's beat rate held at 0.66–0.71, entropy at 0.62–0.68.
  `best.pt`: iteration 1250, 3-seed mean 2.49 (2.42 / 2.62 VP, 61% /
  56%), against 2.44 at the start.
- Next run (owner, 2026-10-03): the opponents should be good enough to
  learn from — the current best is decent, only barely, and weaker
  opponents are probably worthless for now; skewing the training is not
  a concern yet. Random and wall never as training opponents; aggro
  sparingly if at all. Settled (owner, 2026-10-03): the opponent is "the
  latest best" — the bot built from the previous training, the network
  the human-play interface (Succubus Club) plays, exported by
  `train.export` to `zvtes-policy.zvp` — as the only opponent at 4- and
  5-seat tables (3 or 4 copies, each seat its own deck), not the rolling
  periodic snapshots; and `best.pt` is selected on those games (against
  that opponent), not against aggro/wall. Needs a league change: today
  the opponents are the last `opponents_kept` snapshots by PFSP and the
  selection plays the bots. The interface's `.zvp` (2026-10-01) is
  `runs/p4-vote/best.pt` (iteration 1060; its export is byte-identical):
  the frozen opponent. The learner can start from `runs/p5-self/best.pt`
  (1250; 2.49 against the bots, 2.44 for 1060). The rescue/diablerie
  probe (200 five-seat self-play games, 2026-09-30) is deferred until
  play is stronger.
- What the observation holds (owner's question, 2026-10-03): every seat's
  public state (pool, VPs, Edge, prey/predator, ousted, hand, library,
  crypt and ash heap sizes), every minion and card in play at the table
  (blood, capacity, lock, torpor, titles, counters, hosts), the viewer's
  own hand and remaining library/crypt contents, the action, combat and
  referendum under way. Not in it then: the ash heaps' contents and any
  history (V2 below added both, the seats' context of §5 the whole game's
  play).
- Observation V2 (owner, 2026-10-03: "more input (past actions, ash
  heap) … to ground the bot decisions with assessment of possible cards
  in hand"): `obs::Version` — a network reads the encoding it was trained
  on (`V1` byte-identical to before: a digest of 3,274 decisions' policy
  and critic encodings matched the previous commit; `crates/env/tests/obs.rs`
  checks that `V2` minus its new tokens is `V1`). `V2` adds, after every token a
  pointer names and before the context token: `T_ASH`, a token per seat
  and card in its ash heap (library and crypt), with its copies; and
  `T_EVENT + kind`, the last `HISTORY` = 32 public choices
  (`Engine::history`, recorded where `choose` and the run loop's elision
  both pass, the card a hand index named resolved then), each as the
  option it was — a token type per choice kind, the chooser's relative
  seat, its age in choices and game turns, the option's features, two
  pointers (actor else the card or minion it names; target) to tokens
  still present. The trust boundary is `game::public_choice`, a
  whitelist: plays and what follows them face up (actions, blocks,
  strikes, votes and allocations, contests, chosen seats, minions, cards
  and clans, discards to the ash heap, transfers and influence, rescue
  and toll payments, diablerie); never a Pass, a search, a draw, a card
  put under the library or taken from a hidden zone, X choices or slot
  picks (`crates/env/tests/obs.rs`: a choice outside it leaves no event; the
  legal view swap cannot see the history, which lives on `Engine`). Owner to
  veto an entry. Cost: the learner's tokens go from ~66 to ~136 a
  decision (p95 164, max 190; frozen opponents unchanged at ~67), from
  the opening of 256 four/five-seat games. The critic reads `V2`.
  `.zvp` files carry `obs 2` for a `V2` network (absent: `V1`, every file
  so far): `crates/policy` encodes with the file's version, so the
  server's bot keeps playing; a `V2` model needs a server built with it.
- Frozen-opponent league (`--opponents a.pt,b.pt`, with `--league
  true`): every non-learner seat is one of those checkpoints (net ids
  20000 + k, PFSP among them, labeled `<run>@<iteration>`), reading their
  own encoding (`GamePool(obs=, legacy=)`); no bots, no snapshots;
  `best.pt` is selected on `vs_frozen4`/`vs_frozen5` (the opponents in
  turn around the learner, 3 seeds), `vs_bots4/5` kept as a trend line.
  A change of `obs` on resume re-warms the critic (`critic_warmup`).
  Opponents (2026-10-03): `p4-vote@1060` (the interface's) and
  `p5-self@1250` — the owner's message asked for "previous versions of
  itself", plural, against the earlier note naming the first alone
  (`--opponents runs/p4-vote/best.pt` reverts). Not built: promoting the
  learner's own best into the opponents (gating) — owner's question.
  Cooperation across the table: nothing added (the reward stays VPs and
  game win). Start (512 games a size against the pair): `p5-self@1250`
  reading `V1` 1.04 VP / 23% wins at 4 seats, 1.01 / 20% at 5 —
  parity; reading `V2` untrained (new token types at zero, card rows
  not) 1.03 / 22% and 0.94 / 17%: a small perturbation. Run
  `runs/p6-frozen` from it: `--league true --opponents
  runs/p4-vote/best.pt,runs/p5-self/best.pt --decks precons,twda --lr
  5e-5 --entropy-coef 0.04 --iterations 1650` (iterations 1251–1650,
  the first 10 critic warmup). Its first launch thrashed: the MPS
  allocator's cache grew ~2 GB an iteration (attention maps of 1024-row
  minibatches at ~200 tokens, every shape cached apart) to 31 GB on a
  24 GB machine, 2.5 h an iteration by 1255 (`runs/p6-frozen-oom`).
  Since: minibatches run in chunks within `attention_budget` (rows ×
  tokens², gradients accumulated: the same step) and the cache is
  emptied after rollout, update and evaluation. That left a second
  leak, ~1 GB an iteration in small host allocations (67 M of them by
  1259): the MPS backend keeps a compiled graph per tensor shape, and
  V2's rows (and the chunks) brought ever new shapes. Batch shapes are
  now rounded (rows to 64, tokens to 16, options to 8, padding masked;
  chunks halve a 1024-row minibatch; a last partial minibatch is
  dropped): resumed from 1260's checkpoint (`runs/p6-frozen-1.log`
  before), the trainer levels at ~7.9 GB by 1267 (1.6 M allocations),
  ~3.5–4 min an iteration (rollout 34–105 s, update 85–150 s): 1650 in
  ~24 h. First evaluation (1260, end of warmup): 1.10 VP / 23% at 4
  seats, 0.94 / 19% at 5 against the pair — the start's parity.
- Human games as data (owner, 2026-10-03, after a 4-VP win over three
  `p5-self@1250` bots: "use the game log as a teaching vector"). A game
  from the Succubus Club fork ("Export replay": spec by KRCG ids + the
  decision log) replays exactly, so each decision becomes (observation,
  options, choice) for any seat. §2's "imitation bootstrap: rejected (no
  corpus)" is revisited with the owner's games as the corpus, in three
  steps, by value at the corpus's size: (1) **diagnostic** — one game is
  a few hundred human decisions, too few to train on, but enough to show
  where the network's play and a human's part: the human's choices the
  policy finds least likely, and the bots' falls in the critic's value
  with what happened in between (the policy samples, so a bot's
  unlikely choice is noise: bots are read by value, the human by
  probability); (2) **start positions** — self-play continued from a
  human game's prefixes, reaching states self-play rarely makes (a
  strong predator pressing); (3) **an imitation term** — cross-entropy on
  the human's choices beside PPO's loss, small coefficient, once there
  are dozens of games. Built: (1) — `zvtes_py.replay_decisions(json,
  obs)` (`crates/py/src/human.rs`: the protocol `Session` replays the
  log, every seat answered from it; per decision the policy and critic
  encodings, options, choice, events before it; `Session::request` gives
  the encoders the pending request), `zvtes_py.deck_cards(d)`, and
  `uv run python -m train.assess <replay.json> [checkpoint]` (the
  checkpoint's `obs` version; `*` marks a card outside its training
  decks). The corpus: `data/records/` (tracked, indexed in its README
  with each game's engine commit: a rule change can stop an old log
  replaying). Replays by meaning (owner, 2026-10-04: "so the replays I do
  are not useless"): beside its indices a replay logs each decision as
  its choice (a decision entry's `choice`, its option key: the engine's
  `Choice` with hand and uncontrolled positions and card-table indices
  replaced by KRCG ids), and a reader takes the option that is that choice, whatever its
  index; a decision the replay's engine asked between copies alike
  (`forced`) is passed over where a later engine asks nothing. An option
  change no longer stops a game; a rule change the game met still does.
  The corpus was converted on the engine each game read furthest on
  (README). First game (469 decisions, 221 the owner's): the policy's
  favourite was the human's 36% of the time; wherever a vampire could
  be influenced out, it would have (≈100%) where the owner kept
  transferring to another vampire, and it gave a crypt draw no weight;
  in referendums its favourite is to burn a card for votes first (an
  ordering, not a disagreement); the bots' values fell
  gradually (−0.04 to −0.10 a step) through the owner's political turns
  (Parity Shift, Camarilla's Iron Fist ×3 at 4 pool on the Banu Haqim):
  the critic rated the owner's seat lowest at the start (+0.18 against
  +0.44/+0.46).
  More games (2026-10-03/04, five, all won by the owner — 3 or 4 VP, a
  bot 1 at most; `data/records/README.md`): the policy's favourite was
  the owner's 37–46% of the time, lowest in the minion (13–17%),
  influence (22–38%) and discard (0–17%) phases; it still influences
  out at ~100% and gives a crypt draw no weight; one action is its
  favourite in every minion phase of a game (Line Brawl, Govern the
  Unaligned, Shroud of Decay) where the owner hunted.
- Default politics (owner, 2026-10-04, after a Ventrue and a Toreador
  game to show it: "votes are very bad too … too bad at setting the
  terms. Parity Shift should by default damage prey and give blood to
  the player playing it. Kine Resources should damage prey and
  predator"). Measured first (`p5-self@1250` / `p6-frozen@1360`, 120
  five-seat self-play games, political decks in half the seats): of
  the points it chose, Kine Resources Contested and Conservative
  Agitation gave 46% / 43% to the caller itself (40% / 42% to the
  prey); Parity Shift gave 58% / 58% to the caller, 27% across, 15% to
  the prey; votes a coin flip — 48% / 54% for on its own referendums,
  51% / 49% for when the terms burn its pool. In the owner's two games:
  a Salubri hit for 3 by Kine Resources passed four times (87–88%), a
  Ravnos and a Gangrel hit by it burned the Edge for it, a Toreador and
  a Banu Haqim voted both ways on one referendum, and in the owner's
  seat the policy's favourite Parity Shift term was "0 to me". Rewards
  cannot teach this (a few decisions a game, scored at its end; voting
  by rule did not score more at self-play tables, the vote run above).
  So, as temporary guidance to help training (owner: the goal is to
  relax these automated choices later in training), the network seats
  play their politics by rule —
  `zvtes_env::politics`, agent behaviour around the forward pass, not an
  engine mask (rules-qa 17.4 untouched: every option stays legal):
  (1) terms of the referendums moving pool are chosen by rule: damage
  to the prey first, then the predator, then across, the caller only
  when forced (two Methuselahs left); gains to the caller (Camarilla's
  Iron Fist's chosen Methuselah, Parity Shift's allocation), Parity
  Shift's pool taken from the prey (else the predator); points go
  stepwise, so an allocation scores n × (its seat's weight − the best
  later seat's); (2) votes: the caller votes for; a Methuselah the
  terms burn or take from votes against, one they give to votes for —
  the wrong-side votes, Edge, burned cards and one-sided vote cards
  masked, and Pass masked while a free right-side vote (a vampire's, the
  political card's) is left; Edge and burned cards stay the network's
  call on the right side; a seat the terms leave alone is not steered
  (no forced cross-table play, owner 2026-10-03). Extended the same
  day (owner: "banish opponents vampires, not yours, choose your own
  clan when you play consanguineous boon, oppose opponents gaining
  titles, give titles to your own vampires"; then: "banishment
  typically goes to predator first (lowest in blood compared to cap), or
  the prey but only when there's an interesting play in it … leave the
  target to the network as long as it's on a neighbour and not oneself.
  Revolutionary council should go to destroy cumbersome locations and
  damage to the prey. Disputed Territory to steal an interesting
  location for oneself"). Bounds where the choice is a judgement
  (`politics::mask` in the terms): Banishment and Expulsion on a
  neighbour's minion, the network picking which (another opponent's
  only when no neighbour's is offered, one's own never unless nothing
  else is), its controller voting against; Revolutionary Council's
  points on the prey's pool and on the others' locations, one point
  each (none on oneself, one's cards, other Methuselahs or equipment
  beyond what the engine forces; which locations is the network's); its
  Anarchs to lock stay the network's; a Methuselah losing pool or a
  card to it votes against. By rule: Disputed Territory names the
  caller as the one taking control, the location the network's among
  the others', its controller voting against and the caller for; a title (the Justicars, Archon: `PutOnChosen`) goes on one of
  the caller's vampires (the biggest), its controller votes for and
  every other Methuselah against (the one steer of an untouched seat,
  at the owner's word); Consanguineous Boon names the clan with the
  most of the caller's vampires (then the fewest of the others'), and a
  Methuselah it pays votes for. Other referendums (Anarch Salon, the
  burns per minion) stay the network's. `NetAgent` applies it
  (the human-play bot); `tests/default_politics.rs` checks the terms
  and masks over random games (120: ~200 Kine Resources, ~40 Parity
  Shifts, Boons, titles, banishments and expulsions, Disputed
  Territories, Revolutionary Councils, each bound checked option by
  option). Re-measured with it (60 games): Kine
  Resources all to the prey but the last point (to the predator) and
  to the caller only at two Methuselahs; Parity Shift 3 to the caller,
  from the prey unless it is not richer; no named seat votes for its
  own burn, no caller against its referendum. Training still samples
  these decisions (next: the same rule for the learner's seats, its
  answers kept out of PPO's samples). Temporary by design: once the
  rest of its play is solid, the rule is relaxed step by step — a
  flag per window (terms by rule → terms masked → free; votes masked →
  free), each step kept only if the network's own choices hold the
  rule's results in self-play and in the owner's games.
- Politics run (2026-10-04, `runs/p7-politics`; owner: "stop the
  about-even training and relaunch with the new rules and new
  engine"). `runs/p6-frozen` stopped at 1413: its frozen mean stayed
  0.93–1.05 from 1270 on (best 1.05 at 1360) — even with the pair, no
  trend. Relaunched from its `best.pt` (1360) on `3cc7ae3` (rules-qa
  batch 48, copies alike in hand one choice) with `--politics` on (the
  default: `GamePool(politics=True)` answers every network seat's
  terms by rule and leaves the masked options out of the request
  Python sees, so the sampled index, its log-probability and PPO's
  ratio all read the offered options — the learner and the frozen
  pair alike, in training and evaluation); the same pair, learning
  rate 5e-5 tapering, entropy 0.04, to iteration 1800. Its first
  evaluation is the new baseline (the pair now votes sensibly too).
- Opponents that punish, games from the owner's positions (owner,
  2026-10-04, after the review of 15 games: "the engine is missing
  sometimes interesting plays … when will it learn to get out of torpor
  and hunt to offset combat decks?"). The review (three readers, every
  decision of 9 games; counts over the 15): the bots tap out every turn
  and keep no blocker, never remove a drain in play (Creeping Sabotage
  offered ~60 times in one game, The Gate of Acheron, Toreador Grand
  Ball), never rescue (0 of 107 offers) or diablerize (0 of 136), hunt
  only when forced, bleed into known blockers. Self-play shows it is
  learned, not these games' luck: 40 games of `p7-politics@1400`, rescue
  0 of 998 offers, diablerie 2 of 1174, leave torpor 14 of 41, hunt 128
  of 2717 (the earlier served network alike). Against the training
  opponents these habits cost nothing — nobody diablerizes, every table
  races to bleed, the combat decks are piloted by the same bleeders —
  so rewards cannot teach them. Two levers, kept within the settled
  rule that training opponents be good ones (2026-10-03: no random or
  wall, aggro sparingly), hence no new scripted style:
  (1) Punishing habits on frozen opponents (`zvtes_env::habits`, agent
  behaviour like `politics`, not an engine mask): a `net:k+habits` seat
  plays its network and, by rule in the minion phase, rescues its own
  vampire (by a minion with 2 blood), burns another Methuselah's card
  as the Ⓓ action it grants, diablerizes another's vampire in torpor,
  in that order; once some of its minions acted, its last ready
  unlocked one takes no action but a hunt or a rescue (a blocker for
  the predator's turn). `--habit-seats` is the share of frozen seats
  given them (their beat rates apart in `pfsp`), evaluated as
  `vs_habits4/5`; `best.pt` is still selected on the plain frozen
  opponents. `crates/env/tests/habits.rs` checks every rule choice and mask over
  40 random games, from the legal view too. Each habit can be switched
  off (`--habits`, `habits::Set`), because forced, most of them cost
  the network strength — measured on `p7-politics@1400` with
  `crates/policy/examples/strength.rs` (native self-play, half
  the seats with the habits, each deal played twice with the halves
  swapped; VPs a seat, plain half against habit half; the control,
  the same network on both halves, spreads 1.02 / 0.98 at 256 games
  and 1.07 / 0.92 at 512, so ±0.1 is noise): all four, 1.22 / 0.65
  (256 games); rescue alone 0.99 / 0.99 (512; 0.91 / 1.08 at 256: no
  measurable cost); burn alone 1.13 / 0.83, diablerie alone 1.15 /
  0.85, kept home alone 1.17 / 0.78 (256 each); rescue and diablerie
  1.15 / 0.83 (512). In the smoke run (128 games, the learner the
  frozen network itself) the learner scored 1.42 / 1.21 against habit
  seats, 0.84 / 1.08 against plain ones.
  (2) Starts (`zvtes_host::start`, `GamePool(replays=)`, a queued
  table's `start`): a training table begun from a decision in the last
  four fifths of one of the owner's games (`--starts data/records
  --start-tables`), the learner at a random seat, every library and
  crypt shuffled again from the pool's seed at the branch (from one
  position the replay's order would fix every draw: a future to
  memorize); hands, the uncontrolled region and the table kept; the
  time limit at least the position's turn + 20. A start reads its game
  by choices to the log's end or a rules change since (the pool prints
  how far, per game). `train/tests/test_pool.py`
  (`test_tables_start_from_an_owner_game`: a game replayed to a decision,
  every seat played on from there).
  The measure of both: the pool counts the learner's minion-phase
  decisions offering each habit (its own rescue, a burn of another's
  card, a diablerie) and those taking it, logged per iteration as
  `habits` (offered, taken).
- Rescue encouraged, a decisive bot, waste measured (owner,
  2026-10-05: diablerie is risky — the blood hunt — and not for now;
  "rescuing one's own vampire often pays down the road … not forced
  like 100% but encouraged more as a guiding harness like we do for the
  votes … until it learns by itself"; and "it has a tendency to play
  cards for the sake of it … throws away wakes, unlocks … cannot
  construct its hand"). Declining is always on the map: over 300
  random games on every deck, of 7171 decisions offering a card play
  none offered only plays (a Pass, end of phase or hand strike each
  time); the discard phase always offers Pass and a discard.
  (1) The rescue bonus: added to the logit of every network seat's
  rescue of its own vampire, inside the distribution — the pool's
  `opt_bias` (`GamePool.nudges`, `zvtes_policy::Nudges`), added in `act`
  and in the PPO update alike (each row keeps the bias it was sampled
  with), so the ratio is the policy's own; `--rescue-bonus B --nudge-from
  F --nudge-until U`: full to iteration F, fading linearly to 0 at U (the
  harness relaxed as the network learns; the `habits` counter shows
  whether its own rescues hold). The network's rescue logits are so low
  that the bonus must be large to matter: taken 0% of the rescues
  offered without it, 1.4% at +3, 7.1% at +6 (24 self-play games
  each); +6 costs nothing measurable (`examples/strength.rs`, 512
  games: 0.94 / 1.05 VP a seat, within the control's 0.92 / 1.07).
  The owner, by the same count, took 5 of 5; the bots in the owner's
  games 0 of 36.
  (2) A decisive network bot: `NetAgent::temperature` divides the
  logits before sampling; the human-play bots play at 0.5 with the
  rescue bonus (now `zvtes_policy::profiles::NETWORK`). Measured (512 games each, against the
  network sampling as trained): 0.7 1.06 / 0.92, 0.5 1.16 / 0.82, 0.3
  1.18 / 0.80, 0.5 with +6 1.17 / 0.81. Training still samples at 1.
  (3) Waste (`crates/policy/examples/waste.rs`: the owner's games,
  the owner's seats apart, and self-play): wakes followed by no block
  from the woken vampire during the action — owner 22% of 18, bots in
  the owner's games 86% of 49, self-play 85% of 562 (74% at
  temperature 0.5); cycling (a discard at the own discard phase) —
  owner 53% of 122 phases, bots 97% of 306, self-play 95% (95% at 0.5),
  the bots' discards spread as their hands are (each type's share of
  the cycled cards ≈ its share of the hand) where the owner keeps
  reactions (31% of the hand, 23% of the cycled) and political actions
  (8% / 3%) and cycles action modifiers (20% / 34%). Not told apart by
  these counts: unlocks after an action followed by no action or block
  before the next own turn (owner 44%, bots 46–53%), a playable card
  cycled while an unplayable one stays (owner 52%, bots 38–55%: a clan
  card kept for a vampire to come). The two clear gaps — wakes and
  cycling — are the candidates for the next harness (a fading penalty,
  as the rescue bonus).
- **Nudges: keep the hand, keep reactions, no empty bleed, spend the
  transfers** (owner, 2026-10-05: "a fading bonus on passing in the
  discard and against cycling reactions"; "it has a tendency to stay
  idle with all in torpor when it could draw a crypt card or try to
  influence one more vampire"; "it bleeds with Aggressive Corpse and
  that minion has zero bleed … doing an action with no effect should not
  be favored"). The rescue bonus generalized: `zvtes_policy::Nudges`,
  shared by the pool (`GamePool.nudges`, `opt_bias`), `NetAgent` and the
  examples, every option still offered and legal (17.4). On every network
  seat's logits:
  `rescue` (+, its own vampire's rescue); `pass` (+, passing its own
  discard phase with a card to cycle); `reaction` (−, cycling a
  reaction); `bleed` (−, `empty_bleed`: a rulebook bleed by a minion
  bleeding for 0 before any action modifier, base plus borne cards — an
  ally's printed 0); `idle` (−, `idle_influence`: ending the influence
  phase while a crypt draw is still offered, i.e. 4 transfers unspent,
  with 5 pool or more). `--rescue-bonus --pass-bonus --reaction-bonus
  --bleed-bonus --idle-bonus`, all fading together (`--nudge-from F
  --nudge-until U`); the `habits` row adds the learner's `keep` (discard
  phases with a card to cycle, passed) and `reaction` (holding one,
  cycling one). Rates (`examples/waste.rs`, 24 self-play games each;
  owner and the bots in the owner's games for reference): cycling 96%
  plain, 91% at pass +1.5, 76% at +3, 46% at +4.5 (owner 53%), 65% at
  pass +3 with reaction 2 — selection does not follow (cycling while
  holding an unplayable card 50% at +4.5, overall 46%: the nudge moves
  how often, not which); reactions' share of the cycled cards 24% plain
  (26% of the hand), 15% at 1.5, 7% at 3, 9% at pass +3 with 2 (owner
  23% of 31%); bleeds for nothing taken 18% (bots in the owner's games
  16%, owner 0 of 43), 13% at 2, 4% at 4; the influence phase ended with
  a crypt draw offered and 5+ pool 38% (bots 22%, owner 6%), 28% at
  1.5, 17% at 3, and influence phases with no transfer made at all 69%
  (pool 11.7; owner 25%, pool 8) → 56%, 41%. Wakes followed by no block
  do not move (79–85%: no nudge reads blocks). Strength (512 games, as
  before; the control's spread 1.07 / 0.92): pass +3 0.90 / 1.07,
  reaction 2 0.98 / 1.00, both 0.94 / 1.03, pass +4.5 0.94 / 1.03, bleed
  4 0.93 / 1.05, idle 3 0.97 / 1.01, all five (rescue 6, pass 3,
  reaction 2, bleed 4, idle 3) 0.98 / 1.00: none measurable, so the
  frozen opponents carry them too. Not read: the transfers left in the
  phase are not in the observation (the network infers them from the
  options offered: a crypt draw at 4, a withdraw at 2) — an observation
  version's change.
- **Faster iterations** (owner, 2026-10-05: "aim for a 2 to 5 times
  speed improvement total … to make this project viable"). p7's
  iterations took ~5.5 min: training ~190 s (rollout 45–70 s, update
  80–120 s), evaluations the rest (~30 min every 10 iterations). (1)
  An evaluation plays all its games at once (the pool was capped at 64)
  and none after (`GamePool(refill=False)`: a finished slot leaves once
  the queue is empty, no throwaway game in the batches): 256 games 219 s
  against 576 s (p7's frozen table, the GPU shared with p7). (2)
  `eval_every` 20 (was 10). (3) `bots_every` 50: once frozen opponents
  select `best.pt`, the bots are evaluated only from each multiple of
  50 and last (their scores sat at 2.4–2.8 VP for hundreds of
  iterations). The update, profiled on a p7 rollout (13 371 samples, 3
  epochs, 1024 a minibatch): backward 46%, critic forward 18%, policy
  forward 16%, collation 11%. Compute-bound on the M3's 10-core GPU:
  the model's small matmuls reach 300–400 GFLOPS (1 TFLOPS on large
  ones); a layer's forward and backward on 512 rows of 96 tokens ~170
  ms (a third attention, a third feed-forward); half precision (fp16,
  bf16 autocast) no faster. The levers are less work: tokens (the
  policy reads ~92 a decision: 32 for its own library and crypt, 16
  uncontrolled slots — others' face down, all alike — ~22 history, 7
  hand, 6 ash; the critic ~122 with every hand) and passes (epochs).
  Observation `V3` (`zvtes_env::obs`; owner, 2026-10-06: "make our
  training as lightweight as possible"): `V2`'s content in fewer tokens
  — the viewer's library and crypt apart (`Obs::deck_ids`/`deck_f`: the
  network embeds them as tokens and adds their mean to the context
  token, out of attention, no weights of its own: a `V2` checkpoint
  resumes as is), other Methuselahs' face-down uncontrolled vampires as
  their seat's count and counters (events pointing at them point at
  none), and the transfers left in the influence phase on the context
  token (the network had to infer them). 256 self-play games of random
  moves: the policy's tokens 120 → 76 a decision (p90 145 → 101), the
  critic's 141 → 110. `crates/env/tests/obs.rs` checks `V3` is `V2` less those
  tokens, pointers following, and reads only the legal view. The pool
  reads each network's version (`net_obs`: a frozen `V2` network beside
  a `V3` learner); `--obs 3` is the default. The update's collation
  costs the copy to the device, not the padding (4 ms for 1024 rows):
  features cross as float16 (as kept). Each iteration logs the last
  epoch's `kl` (k3, from the playing policy) and `clipfrac`: what the
  epochs add.
  Measured once p7 had finished (the GPU free; 20 iterations of p8's
  configuration from `p7-politics@1800`'s best, one evaluation each):
  `V2`, 3 epochs, 176 s an iteration (rollout 49 s, update 87 s, kl
  0.0023, clipfrac 2.4%); `V3`, 3 epochs, 161 s (50, 69; 0.0023, 2.3%);
  `V3`, 2 epochs, 146 s (51, 47; 0.0019, 1.9%) — against ~330 s for
  p7's iterations. The rollout is inference: the policy's forward passes
  70% of it, the critic's 16%, the engine ~4%; a forward on 1024 rows of
  80 tokens 129 ms, linear in the rows, fp16 autocast 119 ms (fp16
  weights abort in MPS's matmul). ~65 000 decisions are played an
  iteration for ~14 000 learner samples: ~80% of the inference is the
  opponents'. The update moves the policy little (kl ~0.002, a tenth of
  PPO's usual target): the third epoch adds ~20% of the movement for
  ~45% of the update — per second of wall time, 3 epochs move the
  policy slightly more than 2.
  Three more levers tried (owner, 2026-10-07; a side-by-side training
  run could not tell learning apart: p7 gained ~0.05 VP a 100
  iterations, under one evaluation's noise, so each got the check that
  answers it). (1) `learner_seats` 2 (the learner on two seats a table,
  1024 games): rollout 65 s, update 168 s for 28 900 samples — 8.1 ms a
  sample against 8.3 (the rollout's saving goes to an update slower a
  sample): not worth the learner facing itself; kept at 1. (2)
  `best_margin` 0.05 (was 0.1), from p7's last 44 evaluations: 24
  re-measures instead of 39, none of its 15 bests missed (0 would miss
  2). (3) `lr` 1e-4: the same time, kl 0.0046 and clipfrac 4.6% (from
  0.0023, 2.3%; PPO's usual bounds ~0.01–0.02 and ~10%): twice the
  movement an update, still inside the clip. p8 runs at 1e-4; its
  frozen evaluations decide (p7's pace, ~+0.05 a 100 iterations: flat
  or falling after ~200, back to 5e-5 from p8's best).
  p12's configuration, profiled (2026-10-16; a quarter-size rollout from
  p11@4340, 5 339 samples, the GPU shared with p12 — shares, not times):
  an iteration is ~30% rollout (52 s) and ~70% update (118 s; p11's
  pairing brought ~22k samples an iteration against p10's ~15.5k: 113 s
  against 77). The update: backward 37%, the imitation batch 24% (256
  human rows a minibatch: long rows, mid- and late-game; sorting them by
  length into chunks of 64 was slower, 10.9 s against 7.4–7.8), critic
  forward 16%, policy forward 12%, collation 7%, optimizer 4%, `.item()`
  syncs ~0; bf16 / fp16 autocast no faster (34.4 / 34.8 s against
  34.1). The imitation term on every 4th minibatch at 4× its weight
  (`--bc-every 4`, the same expected imitation gradient before clipping —
  the clip and Adam see a spike every 4th step: compare p13's `bc_nll`
  with p12's): expected ~13–14% of an iteration (¾ of the stage's 24% of
  a 70% update); measured 23.1 s against 34.1, an overshoot of the
  shared GPU's noise. Kept for p13 (default 1: p12 as launched). The
  rollout: the networks' forwards 70% (rows: the learner 35%, p10 31%,
  p11 34% — the frozen opponents ~45% of the rollout, ~14% of an
  iteration), the engine ~10%, the critic's values and bookkeeping the
  rest. Left to a run's preparation, as they change the training: 2
  epochs (~23% of an iteration), the pairing off (`--partner-rate 0`:
  ~25–30%, the samples back to ~15.5k; settled off for p13, see p12), one frozen opponent (≤5%).
- **Chores: options never declined** (owner, 2026-10-08: "some things
  are only 'options' in the sense a human is allowed to forget them, but
  it's never good to do so"). The engine elides only empty and
  single-option windows; the network decided every other option.
  `zvtes_policy::chores` (agent behaviour like the default politics,
  17.4 untouched): when a chore is offered, network seats' `Pass` is
  masked (one option left: taken, no forward) — the Edge's pool
  (declining it for a later Parity Shift: the accepted rare exception),
  a hunting ground's blood to a ready vampire with room (free: no lock,
  no cost). `--chores` (on), the pool's `chores`, `NetAgent::chores`
  (on). Found by `examples/census.rs` (self-play, decisions by window and
  option kinds, what the network picks; `p7-politics@1800`, 32 games):
  the Edge's pool taken 97%, the hunting grounds declined 26–33%;
  Homunculus, Vessel, Garibaldi-Meucci Museum and Anarch Troublemaker,
  often declined, have a cost or a trade-off: left to the network. With
  the chores: every offered chore taken; strength 0.994 / 0.997 (512
  games: none lost). With the Sabbat slices, the optional effects the
  rulings call never worse to use, as the scripted bots take them:
  Fever Pitch put on its vampire (rules-qa 53.5), Cursed Abattoir's
  counter (55.6); Forward Momentum's counter is no option (the engine
  adds it; only its order with the unlock is chosen, 53.6).
- **p8 run** (`runs/p8-nudges`, 2026-10-08 → 10): from `p7-politics@1800`
  (frozen opponent), observation `V3`, lr 1e-4, decks precons +
  anniversary + Sabbat + TWDA, owner starts 25%, nudges (rescue 6, pass
  3, reaction 2, bleed 4, idle 3) full to 2200 then faded to 0 at 2600,
  chores on; ~3 min an iteration with evaluations. Best = last
  (`p8-nudges@2600`, kept as `runs/p8-nudges-best-2600.pt`): 1.41 VP a
  seat against p7 (3-seed mean; game wins 34% at 4 seats, 29% at 5),
  2.56 / 2.93 against the bots; rising throughout. Served in the fork
  since 2026-10-10 (wasm built from `adfd807`; previous pkg and model in
  `runs/wasm-pkg-prev-2026-10-10`, `runs/served-prev-2026-10-10-p7at1800.zvp`).
  **The nudges taught nothing that lasted**: faded to 0, the behaviours
  went back to p7's (self-play, `examples/waste.rs`: rescues 0% p7 → 24%
  nudged → 1.6% at 2600, owner 92%; bleed for nothing 18% → 4% → 11%,
  owner 0%; influence ended with a crypt draw offered 38% → 17% → 36%,
  owner 8%; cycling 95% → 76% → 96%, owner 63%; wakes with no block 86%,
  owner 29%) — bot opponents never punish their absence, so self-play
  gives no reason to keep them. Per deck (`examples/decks.rs`, p8
  self-play ~250 seats a deck / p8 at one seat against three p7):
  Ravnos 1.58 / 1.89, Nosferatu 1.47 / 1.24, Lasombra 1.46 / 1.80,
  Path of Power 1.40 / 2.11, Gangrel 1.36 / 1.91, Ventrue 1.34 / 1.84,
  Brujah 1.29 / 1.57, Malkavian 1.22, Ministry 1.19, Banu Haqim 1.16,
  Tzimisce 1.11, Toreador 1.07, Endless Dance 1.00, Path of Death 0.85 /
  1.01, Tremere 0.65 / 1.30, Hecata 0.46 / 1.29, Path of Caine 0.32 /
  0.73, Reign of Stanislava 0.30 / 0.60, Salubri 0.28 / 0.72, Path of
  Cathari 0.18 / 0.32. Salubri, Hecata, Tremere trained all along:
  misplayed (or weak here); the Sabbat and anniversary decks met only in
  p8's 800 iterations. Ravnos, Gangrel, Brujah strong in self-play but,
  by the owner, underperforming against him: aggressive decks may score
  against bots that defend badly (the owner to judge).
- **Plan for p9** (owner, 2026-10-10: "mark this down as the plan"):
  (1) imitation from the owner's replays — a small behaviour-cloning
  term in the PPO update toward the owner's choices at his seats in
  `data/records` (~30 games; AlphaStar's recipe: RL leads, the human
  term guides; small weight, watch overfitting; grows with every replay)
  — the lever for the skills self-play never rewards (rescues 92%,
  selective cycling, blocks after wakes); (2) permanent light nudges
  for the behaviours the owner calls simply correct (rescue, no bleed
  for nothing, spending transfers): fading loses them, full-size cost
  nothing measurable; (3) the learner's seat oversampling the weak decks
  (Salubri, Hecata, Tremere, the Sabbat paths, Reign of Stanislava),
  opponents keeping the full mix; (4) in owner starts, the learner
  seated on the decks it struggles with. Open with the owner: the
  Ravnos / Gangrel / Brujah reading, to calibrate (3). Then p9 from
  `p8-nudges@2600` (frozen opponent: p8 or p7+p8).
  Settled with the owner (2026-10-10): the aggro decks (Ravnos, Gangrel,
  Brujah) not oversampled; the weak paths Cathari, Caine and Death (Path
  of Power 1.40 / 2.11 is not weak); opponents p7 + p8, and the scripted
  bots out of evaluation too ("too bad to matter"); what p8 lacks, by
  the owner: "going forward" (keep influencing when more vampires give a
  chance) and rush decks (Brujah, Hecata) not rushing backward to
  control their predator — the replays are the lever.
- **p9 machinery** (2026-10-10). (1) Imitation: `--bc` (replays, as
  `--starts`), `--bc-coef`, `--bc-batch` (256), `--bc-holdout` (every
  8th game held out, its names logged at launch): each PPO minibatch
  adds `bc_coef` × the cross-entropy of the policy's own logits (no
  nudge) on 256 of the humans' choices with an alternative (since
  2026-10-15, owner: only the human who won a game played out — last
  standing or the time limit —, among the options they were shown — a
  record's `shown`, the hidden ones out of the target, read only where the
  replay keeps the recorded option; the replay's decisions pair with the
  record's by position, `made`, since a later engine shifts their numbers;
  `data/records`: 5,323 choices trained instead of 5,974, the held-out
  five all won); each
  iteration logs `bc_nll` (the update's), `bc_val_nll/acc` (held out)
  and `bc_fit_nll/acc` (as many trained ones). A game exported twice
  (the same spec, one log the other's start: `…T19-28-48` and
  `…T19-39-10`) is read once, for starts too (`load_replays`). (3) A
  table spec's `decks` takes None for a seat to draw (`GamePool`; the
  drawn ones still avoid the pinned ones' crypts); `--weak-decks`
  (indices) and `--weak-rate`: the learner's seat given one of them on
  that share of the league tables. (4) In a start, the learner sits at a
  seat whose deck (matched to the pool by its list, `deck_index`; a
  human's own list matches none) is weak, when one is (`League.spec(n,
  at)`). `--bots-every 0`: no bots once frozen opponents select.
  The reward's game-win weight (owner, 2026-10-10: "penalize doing only
  1vp and not winning … if it does not get a reward for ousting then
  dying, but only when it wins in the end"): `gw_weight` 2 → 5 (a 3-VP
  win 8, an oust then ousted 1; win-only would leave ~70% of the
  learner's games without a gradient), `best.pt` selected on the same
  measure (VPs + `gw_weight` × game wins, at 4 and 5 seats averaged), the
  critic warmed up again when the weight changes (as for a new
  observation). Calibration (10 iterations from `p8-nudges@2600`, `gw`
  2): bc 0.1 moved too much — kl 0.032 the first iteration, entropy
  0.68 → 0.89, against p7 + p8 0.76 VP (p8 itself 1.04 nudged, 1.07
  plain); bc 0.02: kl 0.010 → 0.001, entropy 0.64 → 0.80 (flat from the
  6th), 0.94 VP, held-out nll 2.10 → 1.80 (acc 42 → 44%), trained
  1.57. Entropy rose by ~0.15 at either weight: the policy spreads
  toward the owner's choices it gave little weight to (the evaluation
  samples it: costs some strength, the plateau to watch).
- **p9 run** (`runs/p9-imitation`, from 2026-10-10): from
  `p8-nudges@2600`, frozen opponents `p7-politics@1800` and
  `p8-nudges@2600`, lr 1e-4, entropy coefficient 0.048 (p8's last),
  `gw_weight` 5, decks precons + anniversary + Sabbat + TWDA, owner
  starts 25%, permanent nudges rescue 6, bleed 4, idle 3 (pass and
  reaction 0: imitation's), chores on, bc 0.02 on `data/records` (37
  games: 5456 choices trained, 894 held out —
  `…T08-31-11`, `…T19-47-18`, `…T20-32-01`, `…T22-52-40`), weak
  decks Salubri, Hecata, Tremere, Reign of Stanislava, Cathari, Caine,
  Death (12, 0, 5, 28, 31, 32, 33) at the learner's seat on 40% of the
  tables, no bots; to 3400. The bar (`best.pt`'s measure): p8 itself
  scores 2.16 (VPs 1.13 / 0.95, wins 27% / 18% at 4 / 5 seats). Check
  at ~200 iterations: `examples/waste.rs` (rescues, cycling, blocks
  after wakes) — what the nudges did not teach.
  It stopped at iteration ~2636 on an engine panic ("combat context"):
  Pack Alpha played out of turn, its retainer (Dog Pack, 2 pool) paying
  the defender's last pool — the employ ousted them *and passed the
  turn*, leaving a combat flow without a combat (fixed: out of turn the
  play's resolution ousts and the turn goes on, as for any out-of-turn
  play paid down to 0; `pack_alpha_paid_down_to_zero_out_of_turn_…`).
  Found again in ~21 000 games of random play from the same decks (1
  panic). The pool now survives one: a game the engine panics in ends
  as a truncated `panic` episode named on stderr (seed, decks, seats,
  start, log: it replays), its chains dropped (`Game::advance_caught`;
  `panics` in the rollout's statistics). Resumed from 2620.
- **The crypt draw, never taken** (owner, 2026-10-10: "I see the bot
  never pays 4 transfers to draw from the crypt, is it even a move
  offered to it?"). Offered (census, 32 games: ~1300 influence decisions
  offering it), taken by neither p8 nor p9 — p8 gives it a median 6e-8
  where the owner drew (p9@2640 1.4e-4: imitation lifts it, far too
  slowly); the owner draws at 42 of his 389 offers (11%). A collapsed
  option is never sampled, so never learned. The `draw` nudge
  (`Nudges::draw`, `--draw-bonus`; the pool's "draw"; `waste --draw`,
  `strength draw:`): added to a crypt draw where `idle_influence` holds
  (5 pool or more). Self-play of p9@2640 (`waste`, 24 games, rescue 6,
  bleed 4, idle 3): drew 0% of those offers at 0, 7% at 6, 21% at 9, 45%
  at 12; influence phases with nothing done 46% → 28%, 15%, 3%. p9 runs
  at 7 (~the owner's rate) from its restart, permanent like the others.
  The owner on the decks (2026-10-10): Cathari "is weak alright … we
  might want to remove it from the priority list" — out of `--weak-decks`
  from the restart.
  Restarted from 2640 (2026-10-10, `dfbf114` plus the end-of-round and
  bleed-against-prey fixes): draw 7, weak decks 12, 0, 5, 28, 32, 33,
  44 replays (6129 choices trained, 1008 held out: the four before and
  `…T15-48-57` — the held-out loss is not comparable across the restart).
- **Stuck pool and own nightmares** (owner, 2026-10-10, via the play
  sessions): a Toreador bot transferred onto a second copy of a unique
  vampire it had in play — never offered out while the first stays (no
  voluntary self-contest), the pool on it stuck; a Ravnos bot moved its
  Week of Nightmares' counters onto its own minions ("never a good
  move"). Both legal, both kept (rules-qa 17.4). `waste` counts them
  (`zvtes_policy::stuck_transfer`, `own_nightmare`). p9@2700: onto the
  stuck copy in 35% of the influence decisions offering one in
  self-play, 44% for the bots in the owner's games (`…T16-45-56`: 11 of
  50), the owner 0 of 178; nightmare counters 2 of 9 onto its own in
  self-play (`…T22-05-29`: 2 of 7). Two permanent nudges taken off them
  (`Nudges::stuck`, `nightmare`; `--stuck-bonus`, `--nightmare-bonus`;
  `waste --stuck/--nightmare`, `strength stuck:/nightmare:`): at 4 and 4
  the stuck rate is 11%, own nightmares 0; at 6, 5%.
  Same `waste` run (p9@2700, 24 games, the restart's nudges): rescues
  taken 44% of offers, bleeds for nothing 0, crypt draws 1% of offers
  (the owner 11%: the nudge acts through training), cycling 91% of
  discard phases (the owner 64%), 82% of wakes followed by no block.
  Restarted from 2800 (2026-10-11; 2800 scored 2.50, `best.pt` a mean
  of 2.56, p8's bar 2.16): stuck 6, nightmare 4, the Ilonka peek
  (`7fedb25`), 31 replays — the 17 stopped ones removed, six new — 6240
  choices trained, 1000 held out (`…T21-45-18`, `…T21-51-54`,
  `…T15-01-52`, all three trained on before: the held-out accuracy,
  0.76 at 2801, is not comparable and no longer a held-out measure for
  this run).
  Restarted twice more for the owner's new games and engine fixes
  (3000 on `848e890`; 3080 on `52b6399`: Alamut's controller, the
  pre-vote rotation). Ended at 3400 (2026-10-12), best = last: 3.54,
  re-measured 3.57 (p8's bar 2.16). `waste` on p9@3400 (24 games, its
  nudges): stuck transfers 4.4% (35% at 2700), own nightmares 3 of 24
  (12%, nightmare 4 not quite enough), rescues 29%, bleeds for nothing
  0, crypt draws 6% (the owner 11%), cycling 83% (the owner 64%), wakes
  with no block 81%, unlocks with no action 52%. Served in the fork since
  2026-10-12 with all six training nudges (`ff2c6e8`; previous model
  `runs/served-prev-2026-10-12-p8at2600.zvp`).
- **p10, prepared** (owner, 2026-10-12: prepare, not launched — the owner
  first plays p9 and adds cards). Per deck (`examples/decks.rs` on
  `f160de4`, p9@3400 self-play ~240 seats a deck / p9 at one seat
  against three p8, 60 seats): the weak decks trained came up — Reign of
  Stanislava 0.30 → 1.52, Hecata 0.46 → 1.38, Tremere 0.65 → 1.01, Path
  of Death 0.85 → 1.14 (p8 → p9 self-play) — Salubri and Path of Caine
  barely (0.28 → 0.62, 0.32 → 0.77); the low ones now Tzimisce 0.57 /
  1.01, Salubri 0.62 / 0.67, Nosferatu 0.67 / 0.92 (1.47 under p8),
  Path of Caine 0.77 / 0.78, Toreador 0.79 / 0.64, Gangrel 0.91 / 1.44
  (Cathari 0.38, out). Weak decks for p10: 13, 12, 3, 32, 4 (Tzimisce,
  Salubri, Nosferatu, Path of Caine, Toreador). Held out: `--bc-held-games`
  (parts of file names) names the games instead of every 8th — the
  owner's games p9 never trained on (`…T12-31-56`, `…T15-45-47`,
  `…T21-28-54`, 2026-10-11). From p9@3400, opponents p8@2600 and
  p9@3400 frozen, nightmare 6, the other nudges as p9's.
- **p10 run** (`runs/p10`, from 2026-10-13 on `971f7d3`: round 9's
  cards, the vocabulary remapped): as prepared, to 4000; 30 replays,
  5974 choices trained, 1309 held out (the three above and the owner's
  2026-10-12 `…T21-02-13`, `…T21-50-21`, none ever trained on: a true
  held-out, 0.46 at 3401 against p9's trained-on 0.65). The bar is the
  first evaluation (p8 and p9 as opponents, not comparable with p9's
  3.57): 2.19 at 3420 (re-measured); 2.60 at 3520, the held-out flat
  near 0.45 to 3600. Ended at 4000 (2026-10-14): best 3840, re-measured
  2.84; win shares against p8 0.53 → 0.59, against p9 0.47 → 0.53; the
  held-out stayed 0.43–0.49 (0.48 at 4000). `waste` on p10@3840 (24
  games, its nudges): own nightmares 0 of 19 (nightmare 6), stuck
  transfers 4.9%, rescues 33%, crypt draws 10% (the owner 11%), cycling
  82% (a playable card cycled while an unplayable stays 37%, p9 50%),
  wakes with no block 81%, unlocks with no action 50%. Per deck
  (`decks.rs` on `991b540`, p10 self-play / at one seat against three
  p9): the five trained weak came up — Nosferatu 0.67 → 1.03, Toreador
  0.79 → 1.03, Tzimisce 0.57 → 0.80, Path of Caine 0.77 → 0.83, Salubri
  0.62 → 0.68 — and the ones dropped from the list fell back (Path of
  Death 1.14 → 0.65, Tremere 1.01 → 0.86, Reign of Stanislava 1.52 →
  1.19): the weak list moves the decks round, not up. Lowest now Cathari
  0.45 / 0.27, Path of Death 0.65 / 0.97, Salubri 0.68 / 0.40. Served
  in the fork since 2026-10-14 (nightmare 6 in the wasm nudges; p9's
  model `runs/p9-imitation/best.zvp`); engine.krcg.org on p10 the same
  day (nightmare 6 in the server's bots), the only engine since P6's
  merge (`243bf6b`: the wasm gone, bot settings in
  `crates/server/src/games.rs`).
- **p11** (`runs/p11`, `../zero-vtes-p11` at `45debc9`, round 11's cards;
  2026-10-14 to 10-16, 3841 → 4400). The question (owner): until a seat
  is ousted, damage between cross-table seats "only benefits the bouncing
  prey" — accept the block, keep the combat harmless, yet oust a
  threatening cross-table seat — so restraint should emerge, not be
  nudged; it pays only once both sides learn it, which frozen opponents
  never do: `--partner-rate 0.5` (a second learner across the table on
  half the league tables, ~22k samples an iteration instead of ~15.5k;
  13–17 GB resident, cycling within each iteration, no leak). No weak
  decks (owner: p10's per-deck order matches the players' sense of
  strength); Wider View (findings E7) left to emerge (owner: no nudge).
  From p10@3840, opponents p9@3400 and p10@3840 frozen, the rest as p10:
  `--lr 1e-4 --entropy-coef 0.048 --gw-weight 5 --decks
  precons,anniversary,sabbat,twda --starts data/records --start-tables
  0.25`, nudges rescue 6 bleed 4 idle 3 draw 7 stuck 6 nightmare 6
  (`--nudge-until 0`), `--bc data/records --bc-coef 0.02`, the five
  held-out games named, `--bots-every 0`. Scores (re-measured bests):
  1.93 at 3860, 2.13, 2.30, 2.38, **2.67 at 4340** (4 seats 1.34 VP a
  seat, 0.32 game wins; 5 seats 1.11, 0.21); win shares against p9 0.50
  → 0.55, against p10 0.47 → 0.54; held-out imitation 0.50 → 0.44 (the
  RL pulls off the owner's style). `waste` on `45debc9`, adjacent /
  cross-table while no seat is ousted, p10@3840 → p11@3980 → p11@4340, 160
  games: blood the opposing minion lost a combat, 4 seats 0.95 / 1.19 →
  1.04 / 1.22 → 0.95 / 1.21, 5 seats 0.98 / 1.18 → 0.98 / 1.37 → 0.95 /
  1.29; sent to torpor or burned, 4 seats 11% / 19% → 9% / 13%; pool a
  bleed took from a cross-table seat 0.14 → 0.18 (its prey 1.36 → 1.21):
  **no restraint emerged** — pairing alone does not teach it. Wider View
  (draw-and-remove taken, of the offers): precons 39% → 16%, TWDA 29% →
  15%; with under 4 crypt cards 26% → 8–15% (the owner 12%, none):
  **learned without a nudge**. With the server's nudges (24 games):
  stuck transfers 2.9% (p10 4.9%), own nightmares 0 of 13, rescues 42%,
  crypt draws 5% (the owner 11%), cycling 86%, a playable card cycled
  while an unplayable stays 34%, wakes with no block 86%, unlocks with no
  action 43%. Per deck (`decks.rs`, 600 games a measure, self-play VP a
  seat / at one seat against three p10): Malkavian 1.54 / 1.80, Ravnos
  1.42 / 1.23, Ministry 1.37 / 1.50, Hecata 1.20 / 1.53 …; the lowest
  Salubri 0.56 / 0.77, Path of Cathari 0.58 / 0.23, Gangrel 0.64 / 0.62,
  Nosferatu 0.65 / 0.57, Path of Death 0.69 / 1.13; against three p10
  0.99 VP a seat on average. Exported `runs/p11/best.zvp` (sha256
  `b9035931a527`); served on engine.krcg.org since the 1.6.0 deploy
  (owner, 2026-10-16), kept at 1.8.0 over p12, replaced by p13
  (2026-10-19); the previous `runs/served-prev-2026-10-16-p10at3840.zvp`
  (`7aaa9528fbf8`). The positions where the owner beat the bot
  (`humanset`, the five held-out games, the bots p10 as served, 20 games a
  position): p10 / p11 agree with 46.1% / 45.3% of the owner's 1,309
  choices; won from a quarter in 33% / 43%, halfway 52% / 54%, three
  quarters in 91% / 83% (±10 at 100 games); from …T15-45-47's first two
  positions neither ever wins (0 of 40), where the owner came back.
  Humans against p10 (the harvest, 14 games): 11 won
  (79%), 2.79 VP a seat, the bots 0.33 — p11's bar; against p11 on its first served day (2026-10-17, 7
  games) 4 seats of 7 (57%), 1.86 VP a seat, the bots 0.43 (a small
  sample).
- **p12** (`runs/p12`, `../zero-vtes-p12` at `cb3b4a6`, round 13's cards,
  E25 with the futile-block mask; launched 2026-10-16, 4341 → 4842 of a planned 4900). From
  p11@4340, opponents p10@3840 and p11@4340 frozen; imitation winners
  only (owner, 2026-10-15) over `data/records` and the 15 harvested games
  played out (8,381 choices, 3,058 harvested; the same five held out),
  the same games as starts; the rest as p11 (`--partner-rate 0.5`, the
  nudges, no weak decks). Stopped at 4842 (2026-10-17, killed by memory
  pressure under a parallel release build: no traceback; the owner chose
  to evaluate the best rather than resume). Scores (re-measured bests,
  against p10 and p11): 1.84 at 4360, 2.06, 2.16, 2.19, 2.40, **2.51 at
  4800** (4 seats 1.19 VP a seat, 0.29 game wins; 5 seats 1.21, 0.25);
  win share against p11 only 0.48 → 0.50. Held-out imitation 0.54 at 4360, 0.51 at 4800
  (p11's 0.44 at its end: the winners-only imitation holds the owner's
  style). **Not
  served**: it does not beat p11 (owner's rule). Measured against p11 on
  `cb3b4a6`: at one seat against three p11 (`decks.rs`, 600 games) 0.88
  VP a seat (~2 standard errors under even; Malkavian 1.80, Path of Power
  1.50 …; Banu Haqim and The Endless Dance 0.27, Nosferatu 0.33);
  `humanset` (the five held-out games, bots p11 as served): agreement
  50.2% against p11's 45.3%, won from the owner's positions 162 / 300
  against 166 (a quarter in 35% / 34%, halfway 41% / 53%, three quarters
  86% / 79%); Wider View regressed — taken at 27–30% of the offers on the
  precons (p11 16%), 20% on TWDA decks (13%), under 4 crypt cards 15–21%
  (7–12%); cross-table minions sent to torpor at 4 seats 23% (p11 13%;
  adjacent 11% / 9%), the pool a bleed takes from a cross-table seat
  0.15 (0.18). Exported `runs/p12/best.zvp` (`2dbb922666c9`), kept. Settled for p13
  (owner, 2026-10-17): the pairing off (two runs, no cross-table
  restraint, ~25–30% of an iteration); from p11@4340, not p12; Wider
  View still without a nudge (from p11, which had learned it).
- **p13** (`runs/p13`, `../zero-vtes-p13` at `331ce66`, rules-qa 82.1's
  block loop; launched 2026-10-18, 4341 → 4900). From p11@4340 (owner,
  2026-10-17), opponents p11@4340 and p12@4800 frozen; no pairing;
  `--bc-every 4`; imitation winners only over `data/records` and 17
  harvested games (7,052 choices; held out 1,349 in five games: four of
  `data/records`, `e33fab34fee8ca4b`), the 20 harvested games as starts;
  nudges, lr and entropy as p12. Nine harvested games left out of
  imitation and the held-out set, six since 82.1 (a block window the old
  engines reopened) and three drifted earlier (a second intercept burn
  under one grant, rules-qa 79.1; diableries of an older vampire whose
  master search shuffles, 7d7d339, rules-qa 86.1); four of them kept as
  starts (their readable prefixes), five dropped; ~105–115 s an iteration.
  Scores (bests, against p11 and p12): 1.80 at 4360, 1.81, 1.89, 1.95,
  2.00, 2.09, **2.30 at 4740** (4 seats 1.25 VP a seat, 0.27 game wins; 5
  seats 1.19, 0.23), then 2.11–2.20; held-out imitation 0.52 → 0.54 at
  4440, 0.47 at 4740. Seventeen games hit a Dual Form panic on a clanless
  vampire (seven in training, dropped; ten in evaluations; fixed in round
  16). Measured on `331ce66`: at one seat against three p11 (600 games)
  0.97 VP a seat (self-play 0.99: even within a standard error; Lasombra
  1.60, Reign of Stanislava 1.53 …; Nosferatu 0.20, Path of Cathari 0.30);
  `humanset` (the five, bots p11 as served): agreement 47.1% against p11's
  44.8%, won 139 / 300 against 125 (a quarter in 40% / 25%, halfway 33% /
  39%, three quarters 66% / 61%); Wider View back to 11–15% (p11 15–16%),
  wakes with no block after 80–86% (p11 79–86%); influence phases with no
  ready minion and no transfer or crypt draw on the precons 59–75% (p11
  85–93%), on TWDA decks 67% (58%); at the served settings transfers onto
  a duplicate unique 60% (68%) and bleeds for nothing 6.6% (3.9%).
  **Served** (owner, 2026-10-19: even with p11 head to head, better from
  the owner's positions), `runs/p13/best.zvp` (`df66d54badf3`), the
  previous `runs/served-prev-2026-10-19-p11at4340.zvp` (`b9035931a527`).
- **Combat play by deck** (owner, 2026-10-20, the training board's "Rush
  and wall play": "What matters is they have specific actions and
  reactions to enter combat and they need their combat to be played
  efficiently on the right targets for the deck to work against good
  opponents"). `waste --by-deck`, a rush being an action entering combat
  with its target (`zvtes_policy::rush`); combats counted from the combat
  notes from now on, those with every window elided included (the
  per-combat figures above leave those out). p13 / p11 self-play (400
  games at 4 seats, no nudges) against the humans (`data/records` and the
  2026-10-19 harvest pooled, 22 to 81 combats a deck, none on Banu Haqim:
  small, and against older bots). Own turns with a rush offered and one
  declared: Nosferatu 4% / 4% (the humans 42%), Gangrel 4% / 3% (21%),
  Banu Haqim 16% / 14% (none offered to the humans), Tzimisce 29% / 15%
  (100%), Salubri 26% / 33% (30%), Brujah 29% / 31% (69%), Hecata 42% /
  60% (100%). The opposing minion to torpor or burned, for p13: Banu Haqim
  12%, Hecata 14%, Tzimisce 18%, Gangrel 19%, Salubri 22%, Nosferatu 28%,
  Brujah 36% (p11 within 6 points), the humans 26 to 91% (Brujah 91%,
  Nosferatu 45%, Gangrel and Salubri 44%); its own minion down 8 to 31%
  (the humans 2 to 18%). Block windows with a reachable attempt,
  attempted: p13 48 to 74%, the humans 44 to 65%. Wakes with no block
  after 77 to 88% (the humans 7 to 67%, 3 to 15 wakes a deck). No gap in
  the targets (the rushes on the prey 26 to 75%, the humans 31 to 83%; on
  a minion that had declared a bleed 53 to 88%, the humans 25 to 100%; on
  one carrying equipment 6% at most, the humans none) nor in holding back
  (combats with combat cards held and none played 12 to 38%, the humans 5
  to 58%). The bots block as often as the humans; the gap is the combats
  they enter and what those achieve: the rush decks seldom rush
  (Nosferatu, Gangrel, Banu Haqim under one turn in six), and their
  combats seldom put the opposing minion down. Per deck (`decks.rs` on
  `331ce66`, p13 at one seat against three p11, 30 seats a deck):
  Nosferatu 0.20, Banu Haqim 0.37, Hecata 0.77, Salubri 0.90, Tzimisce
  0.93, Brujah 0.97, Gangrel 1.13 (mean 0.97); in self-play 0.61 to 0.93.
- **p14** (`runs/p14`, `../zero-vtes-p14` at `10b4369`; launched
  2026-10-21, 4741 → 5300, ~111 s an iteration, no panic, nothing
  truncated). The owner's lever of 2026-10-21 for "Combat play by deck"
  above: the deck mix and an imitation weight toward the rush and wall
  decks, no wake nudge. From p13@4740 (served), opponents p13@4740 and
  p11@4340 frozen; as p13 otherwise (no pairing, the nudges, lr, entropy,
  `--bc-every 4`, the same five held out). The learner's seat on one of
  Hecata, Nosferatu, Banu Haqim, Brujah, Gangrel, Salubri, Tzimisce
  (`--weak-decks 0,3,7,8,9,12,13`) on 35% of the league tables
  (`--weak-rate 0.35`), its opponents from the whole pool, and in a start
  at a seat playing one. Imitation over `data/records` and the 26
  harvested games that read whole (of the 35 counted): 9,231 choices,
  3,263 on those decks (none on Banu Haqim), drawn 3 times as often
  (`--bc-weak-weight 3`). Scores (bests, against p13 and p11): 1.70 at
  4760, 1.88, 1.97, 1.98, 1.98, 2.03, 2.05, 2.10 at 5120, **2.13 at
  5280** (4 seats 1.01 VP a seat, 0.24 game wins; 5 seats 1.04, 0.22);
  held-out imitation
  0.49 → 0.52. Measured on `10b4369`, p13 alongside on the same engine:
  at one seat against three p13 (600 games) 1.00 VP a seat, even; on the
  seven decks Brujah 1.43, Hecata 0.87, Tzimisce 0.93, Gangrel 0.83,
  Nosferatu 0.57, Banu Haqim 0.40, Salubri 0.40 (p13 against p11: 0.97,
  0.77, 0.93, 1.13, 0.20, 0.37, 0.90; 30 seats a deck, a standard error
  ~0.2). `humanset` (the five, bots p11 as in p13's measure): agreement
  51.3% against p13's 47.1%, won from the owner's positions 140 / 300 against
  139 (a quarter in 36% / 40%, halfway 33% / 33%, three quarters 71% /
  66%). `waste --by-deck` (400 self-play games, p14 / p13): own turns with
  a rush offered and one declared Brujah 41% / 29%, Hecata 56% / 42%,
  Gangrel 8% / 4%, Nosferatu 4% / 4%, Banu Haqim 11% / 16%, Salubri 21% /
  26%, Tzimisce 16% / 29%; the opposing minion to torpor or burned within
  3 points of p13 on every deck (Banu Haqim 12%, Tzimisce 16%, Hecata 16%,
  Salubri 19%, Gangrel 22%, Nosferatu 26%, Brujah 36%); wakes with no
  block after 75–88% (p13 77–88%); targets and holding back as p13. The
  lever moved the rushes of the decks that already rushed (Brujah,
  Hecata) and none of the combats' outcomes: the rush decks still seldom
  rush, and their combats still seldom put the opposing minion down.
  Regressed as p12 did: Wider View taken at 18–23% of the offers (p13
  12–14%), cross-table minions sent to torpor at 4 seats 16% (p13 10%).
  **Served** (owner, 2026-10-22: at least even with p13, its behaviour
  maybe better), `runs/p14/best.zvp` (`a78e74292acb`), the previous
  `runs/served-prev-2026-10-22-p13at4740.zvp` (`df66d54badf3`).
- **The critic against the position** (owner with Tom, 2026-10-22: "some
  measure of the quality of the position", the pools of every seat, the
  lunge, the actions and blocks each side has). `train.position` (the
  features: `zvtes_py.position_names`, `crates/py/src/position.rs`, for
  the deciding seat: the pools of the seat, its prey, its predator and
  the cross-table seats, VPs, each side's ready, unlocked and torpid
  minions and the blood on the ready ones, hand, library, turn, the edge,
  the lunge — a bleed of one a ready minion reaching the prey's pool, or
  the predator's reaching one's own): on 1,024 self-play games of
  p14@5280 (every seat its policy, 4 and 5 seats alternating, 578,503
  decisions), the explained variance of each seat's result ((VPs + 5 ×
  win) / 5, the critic's target) by the decision's share of the game,
  for the critic as it is, a linear fit of the 26 features per stage
  (fitted on 70% of the games, scored on the others) and both together.
  Self-play: the first quarter −0.03 / 0.02 / 0.08, the second 0.07 /
  0.09 / 0.17, the third 0.34 / 0.29 / 0.38, the last 0.71 / 0.57 /
  0.72; the human records (`data/records` and the harvest, read whole,
  28,554 decisions, the self-play fit): −0.13 / −0.03 / −0.05, −0.01 /
  −0.09 / −0.04, 0.23 / 0.10 / 0.21, 0.66 / 0.47 / 0.66. Through the
  first half the critic predicts almost nothing, no better than the
  features, and the two together double it in the second quarter: the
  critic does not read all the position holds there, so the first half's
  advantages rest on a near-constant baseline; from the third quarter it
  reads more than the features. On the humans' games it is worse early
  (their play is not the self-play it learned). The features' weights (a
  standard deviation each, the result's scale): in the second quarter the
  seat's pool +0.10, the blood on its ready minions +0.08, the prey's
  pool −0.07 and its margin over one's ready minions −0.08, the
  predator's ready blood −0.05, the prey's −0.05; in the third the ready
  blood +0.14, the pool +0.12, VPs +0.12; the lunge +0.04 in the third
  and +0.05 in the last, being lunged −0.04 in the last; the unlocked
  minions +0.03 to +0.04 (the actions and blocks left this turn).
- **p15** (`runs/p15`, `../zero-vtes-p15` at `788eae4`; launched
  2026-10-23, 5281 → 5840, no panic, nothing truncated, 100–200 s an
  iteration on a machine shared with builds). From p14@5280 (served),
  opponents p14@5280 and p13@4740 frozen; one change from p14:
  `--shaping runs/phi-p14.json` (coefficient 1), Φ the linear fit of the
  26 position features over the whole game on p14's 1,024 self-play games
  ("The critic against the position" above: a VP +0.30, the seat's pool
  +0.58 a 30 pool, the blood on its ready minions +0.41 a 20 blood, the
  prey's pool −0.32), each learner decision gaining Φ(next) − Φ(this) and
  the last −Φ(last) (γ 1, Ng 1999); the critic re-warmed 10 iterations.
  Imitation 8,986 choices (`ec8d47141b6b38e9` read partway since round
  20's Anarch Revolt change, left out). Scores (bests, against p14 and
  p13): 1.77 at 5300, 1.91, 2.03, **2.09 at 5820**. Measured on `8fcaaa3`
  against the served p14 (the training skill's step 4): at one seat
  against three p14 (600 games) 1.03 VP a seat; `humanset` (the five,
  bots p14) agreement 48.9% against p14's 51.4%, won from the owner's
  positions 132 / 300 against 124; `waste`, beyond two standard errors:
  worse — influence ended with a transfer or draw possible and none made
  62% / 58% at 5 seats and 63% / 55% on TWDA, wakes with no block after
  85% / 80% on TWDA, adjacent combats to torpor 10% / 13% on TWDA;
  better — Wider View used 7–10% / 19–20%, rescues 3–6% / 2–4%. The
  shaping cut the rushes instead of raising them (own turns with one
  declared, p15 / p14: Brujah 24% / 41%, Hecata 33% / 56%, Salubri 5% /
  21%, Banu Haqim 5% / 11%, Gangrel 1% / 8%; Tzimisce 26% / 18%) — Φ paid
  at once for the blood a combat costs and only later for what it took —
  and did not help the critic (with Φ added back, `train.position`:
  self-play 0.03 / 0.09 / 0.29 / 0.64 by quarter against p14's −0.03 /
  0.07 / 0.34 / 0.71). **Not served** (owner, 2026-10-25: it did not
  improve as hoped); the shaping dropped. Exported `runs/p15/best.zvp`
  (`ee41b58bef28`), kept.
- **The impact ledger** (owner, 2026-10-25: "do the bot network improve
  on getting more actions and more impactful meaningful actions? … missing
  opportunities in many places"; runs are judged by it, not by single
  counters). `waste`'s impact lines, per group: A&B a round — the seat's
  actions declared, its blocks that landed and its hindrances (another
  seat's action canceled, failed or ended unblocked, a bleed bounced or
  cut to nothing, by its card: `zvtes::profile`), over its own turns, one
  a round whatever its minions' number ("a single minion with a lot of
  unlock … can produce equal A&B as multiple smaller minions"); a
  reaction neither blocking nor hindering counts nothing; R = 2 × pool +
  the blood on its minions (pool the more important), its own, its
  prey's and its predator's change over its own turns; the hand: slots
  and dead slots a round, A&B and the prey's R a hand slot a round, and
  with `--cards` per card the rounds held, its plays a round held, its
  cycles and its prey's R a round held — holding a card costs its rounds,
  so cycling toward the right hand counts for it (owner: "cycling cards
  to get a more appropriate hand is even more important than keeping
  cards until they're really efficient"). A measure, not a reward (p15).
  p14 (100 self-play games at 4 seats, and its seats in the owner's
  records) against the owner (298 turns, against p5–p9's older bots):
  A&B a round 2.13 / 3.03 (actions 1.70 / 2.32, blocks landed 0.37 /
  0.57, hindrances 0.06 / 0.14); R over its own turns: its own −1.48 /
  +0.79, its prey's −3.37 / −6.06, its predator's −1.02 / −2.51; dead
  hand slots a round 1.10 / 0.87; a hand slot a round: A&B 0.31 / 0.41,
  the prey's R −0.48 / −0.83; Govern the Unaligned played 0.42 / 0.89 a
  round held, Telepathic Misdirection 0.18 / 0.24 (the bots' most held
  card, 212 rounds against the owner's seats), Freak Drive 1.03 for the
  owner. The gap is broad — about 40% more A&B, every kind, and twice
  the prey's loss a turn, the owner's resource growing where the bot's
  shrinks — so a planning lever before a counter's.
- **Futile blocks on the E25 engine** (owner, 2026-10-15: before any
  redeploy). E25 (`97c4880`) offers every legal block attempt, those
  that cannot reach the stealth too (`Engine::block_can_reach`); no
  network was trained on them. `waste` (block windows), 80 four-seat
  self-play games at the server's settings (temperature 0.5, its nudges):
  p10@3840 takes a futile attempt in 61% of the windows offering one and
  attempts in 66% of those offering only futile ones (the windows elided
  before E25), against 58% for a reachable attempt; p11@3920 63%, 66%, 62%.
  The networks do not tell futile from reachable. The owner's records
  (before E25, replayed on it, read whole since `waste` passes the
  windows their engine elided, `Replay::elided`): 92 windows with a futile
  attempt, none taken. Owner, 2026-10-15: "mask in the bot layer, train and serve" —
  `zvtes_policy::reachable_blocks`, the outermost mask of `NetAgent`
  (the server, `waste`) and of the training pool (`crates/py`): a network
  never attempts a futile block, a window of futile attempts only passes
  without asking it (no training decision). A failed attempt costs
  nothing, so no reward would teach it. The engine still offers them
  (rules-qa 73.8); the scripted bots score them below the pass (§5).
  Masked, p10 at the server's settings (80 games): 0% of 2,102 windows
  with a futile attempt, 0% of 1,370 with only futile ones.

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
- **Mechanisms missing** (ledger `blocked:` rows name the rest): immunity
  to damage from a source.
- **Known edges**:
  - `ImmuneToDirectedActions` is read on card plays only
    (`GameState::immune_to_directed` in `plays.rs`): a granted or printed
    action requiring a Discipline would need it in `ability_targets` too.
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
  - Directed actions name one Methuselah at most (`ActionCtx::directed_seats`);
    Shadow Cast's remembered-set test is written for several.
  - Wolf Claws [PRO] is covered by fuzz only.
  - Decisions are unbounded within an action: `TURN_LIMIT` bounds turns, but
    the rulebook allows block re-attempts as often as wished. A failed
    attempt is not offered again in the same situation (rules-qa 48.6,
    `ActionCtx::attempt_print`), so a repeat needs a change — a card played,
    blood moved; still, a driver with a deterministic policy (the P3 greedy
    evaluation, a human client) keeps a decision budget; the scripted bots
    remember their attempts.
- A deferred oust (Life Boon) lets the effect that ran the pool out finish
  first: windows of an action or referendum under way may still list the
  Methuselah about to be ousted until the Boon decision (fuzzed by
  `random_games_with_legacy_additions…`, not proven for every flow).
- Legacy additions partly covered by tests only through code paths: a
  Rötschreck range check (a strike ineffective at the range), Powerbase:
  Savannah with Deep Song's acting target, Saatet-ta's stealth and intercept
  locks, Carlton's strike and dodge.
- No printed sterile baron exists, so Childe of the Revolution's sterile
  mask is tested on a sterile vampire made a baron (Effie Lowery).

### 11.4 What remains

- **P2 — full engine pool**: every V5 card defined; legacy cards come in
  through the cards line (§3).
- **P3 — training loop**: built (slice 14), runs judged by the impact
  ledger (§11.2).
- **P4 — league & 5p**: the league, PFSP, 4- and 5-seat tables and seat
  sampling, the exploiter built (slice 15).
- **P5 — community**: online play (§8).

The objectives still open are the roadmap's (`docs/roadmap.md`).

Definitions: 954 library, 482 vampires; 2645 Rust tests, 62 Python tests

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
  is needed (Iron Glare; KRCG `combo`).
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
