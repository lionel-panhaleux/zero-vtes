# Engine board

Goal: the engine clearly separated, complete and legal, agnostic of humans
and bots, behind an API convenient for platforms, agents and developers
(roadmap: architecture, objective C). Skill: `engine-review` on every
engine commit. Docs: `docs/engine.md`, `docs/protocol.md` (the contract,
semver), `crates/api` (`schema.json`, `openapi.json`, `protocol.ts`),
`docs/online-plan.md` (the architecture as built), DESIGN §4, §9. Covers
the engine's architecture, `api`, `host`, `text`, `server`, performance,
and the rulebook mechanisms no card carries (card definitions and
readings are the cards board's).

Status: `todo` · `doing` · `blocked:<what>` · `later` · `done`. Order: what the
community beta's games meet first (the server's records and restarts, a
rule players hit, feedback), then the protocol leaks the Succubus Club
fork works around, then the API for outside platforms and contributors,
then multi-human play, then the training layer split, the remaining
mechanisms and the review leftovers.

| # | Item | Why / source | Status |
|---|---|---|---|
| E1 | Games a redeploy left open: at startup the server ends the games its store lists as going (`server_restart`, recorded); `/admin/games` and the harvest cursor stop waiting on them | findings F16 | todo |
| E2 | Feedback intake: a `feedback {text}` command (a minor protocol version) stored with the game's entries at its decision number; `/admin/feedback` lists them for triage into `docs/findings.md` | beta (roadmap); P3 is the button | todo |
| E3 | Diablerie step 2: the diablerist may take any equipment on the victim (rulebook), an `Interrupt` and its test; today the equipment burns with the victim | findings L16 | todo |
| E4 | Records complete: `BotRecord` carries the nudges each network bot played and the result its turn count; `games_report.py` stops replaying records for the turns | findings F15; harvest (training session) | todo |
| E5 | Demand signal for cards: the server counts the ids `SpecError::Unsupported` refusals name (ids and counts only, no player data), `/admin/unsupported`; read by the cards line's round choice (C5) | objective B | todo |
| E6 | Ability labels as data: an option carries its ability's label (the printed line or an effect summary), so a client names the "Which ability ?" step without the English text's tail (null with `text` off) | sc.md boundary leak 1 | todo |
| E7 | Who uses an ability, said: an explicit user (the Methuselah or the minion) on the option instead of an absent `actor` read as "your Methuselah" (War Ghoul, findings P12) | sc.md boundary leak 2 | todo |
| E8 | A decision's reason in data: why options are missing (the forced hunt first, rules-qa 17.2; findings P14), so the fork stops inferring it from the minion phase with no `pass` | sc.md boundary leak 3 | todo |
| E9 | Markers and names in data: a marker's kind and count with its name on request; minions of one name told apart in the view (the `nth` the text crate's `Names::numbered` computes, findings E8) | sc.md boundary leak 4; findings P13 | todo |
| E10 | Diablerie of an older vampire: the controller may fetch a master Discipline card (library, ash heap or hand) onto the diablerist, the library shuffled or the hand drawn up (advanced rules 4; eleven such cards in the pool) | findings L16 | todo |
| E11 | Repository public: done (owner, 2026-10-14: https://github.com/lionel-panhaleux/zero-vtes, MIT). Left: a README for outsiders, CONTRIBUTING (CLAUDE.md and the skills as the contributor's guide), a secrets audit of the history, the server's GitHub release asset (B7 waits on it) | online-plan settled 7, B7; human-play 5.1 | todo |
| E12 | Easy to plug in: `docs/protocol.md` §2's minimal client as a runnable example outside JavaScript (Python over the WebSocket), a CHANGELOG of every `PROTOCOL_VERSION`, a schema-diff check failing a removed or renamed field unless the major moves | objective C; protocol §14 | todo |
| E13 | Guidance vs advantage written down: protocol §8 gains the rule (the host may hide what play leaves out of its moment and help a player act; never remind of an effect or a trigger the player forgot) with every setting checked against it; the border cases to the owner | objective C; sc-change rule 7 | later (owner, 2026-10-14: a later step) |
| E14 | Redaction audit, the multi-human gate: cards named from hand, `limbo`, `ActionView.card`, the "waiting on seat X" tell (hide, delay or accept: the owner) | online-plan A10; human-play open question 1 | later (owner, 2026-10-14: a later step) |
| E15 | Several humans at one table, bots in the free seats (a player missing at a table), after E14; spectators (public only, or omniscient with a delay: the owner) | online-plan settled 2, order 6; objective B; human-play open question 2 | later, after E14 |
| E16 | All-human engine tables: the engine as the table's rules oracle with no bot | objective C | blocked:E15 |
| E17 | A game survives a restart: rebuilt from its record at server start (replayed to its last decision), so a redeploy loses no game | human-play 5.7; sc.md traps ("no resume") | todo |
| E18 | Bot profiles by name in `policy` (style, weights, temperature, nudges), implementing `host`'s `Bots`; the shells stop implementing it | online-plan A4 | todo |
| E19 | Stops and auto-pass preferences per window or card, in the settings, within E13's rule | human-play 4.1; DESIGN §8 (preemptive impulse requests) | later (owner, 2026-10-14: a later step) |
| E20 | Narration left: a card canceled told; what a card did, as typed events | human-play 4.2 | todo |
| E21 | Renderer left: markers' and strikes' text from data (an oracle each first), the purposes no sweep reaches (`text/src/option.rs` `asked`, declined); wording open to improvement (owner) | online-plan A7 | todo |
| E22 | Events as a complete delta stream (view N + events = view N+1), views on decisions only | online-plan contract rules ("later, decided now as the direction") | todo |
| E23 | `zvtes-env`, step 1: `bots::does` out of `bots.rs` into a neutral engine module (`politics.rs` uses it) | architecture (a layer for training-only code); harvest (training session) §5 | todo |
| E24 | `zvtes-env`, step 2: `obs.rs` (with `encode_critic`), `habits.rs` and `bots.rs` into `crates/env` over `engine`; `policy` and `py` on it, `host` for the scripted bots; `agent.rs` and elision stay; the training line told before it merges (T12) | architecture; harvest (training session) §5 | blocked:E23 |
| E25 | Every legal play offered (owner, 2026-10-14: "the engine should allow all legal plays even if they're not worth considering"). Elision proper is not in question: it only skips a decision with a single option (DESIGN §4.3). The one engine pruning of legal plays is the block reachability pruning (`intercept_reachable`, rules-qa 48.5, docs/engine.md §6.6): a block attempt that cannot reach the stealth is not offered. Proposal: the engine offers every legal attempt; the pruning moves to the host's `opportunities` filter (humans) and the training layer (E24, bots). A behaviour change: goldens move, the training line told first; the owner confirms | architecture | blocked:owner (confirm the proposal) |
| E26 | An open definition of the game's interactions: every card's definition (modes, requirements, effects, windows; the window matrix) exported as documented JSON with a schema, generated and `--check`ed | architecture (open-source definition); the judge track | todo |
| E27 | Tournament withdrawal: the unlock-phase decision, 0.5 VP (rules-digest K41) | DESIGN §10.6, §11.3 mechanisms missing | todo |
| E28 | Paths: the influence-phase "burn 1 blood to follow a path" for a Sabbat vampire following none (Hartmut Stover, Styles Margs); paths in the observations through the training line (T13) | DESIGN §11.3 paths | todo |
| E29 | Immunity to damage from a source | DESIGN §11.3 mechanisms missing | todo |
| E30 | Performance baseline: an example timing a random 4-seat game and decisions a second (DESIGN §9 targets), the numbers in DESIGN §9, re-measured each release | architecture (high performance for training) | todo |
| E31 | `host/src/effects.rs` exhaustive: its `_ => {}` replaced by a match on `Effect::family()`, `Family::Marker` the only silent arm, so a new player-visible effect cannot go unmapped | engine-review F12 (cross-crate silent drop) | todo |
| E32 | The review's unchecked points: abilities' targets re-validated at resolution (play vs ability); the `ChosenController` branch's `requires` | engine-review §5 | todo |
| E33 | The pay-or-suffer `Interrupt` family (`PayOrBurn`, `BloodOrBurned`, `LockOrPay` …): factored only when a round brings several more, keeping the `Choice` values | engine-review F11 (leave) | blocked:several more such cards |
| E34 | Deep sweeps on a schedule: `FUZZ_SEEDS` and `HOST_SEEDS` sweeps weekly at `main`'s tip, a panic or a key collision a findings row | DESIGN §7.2; online-plan audit (~500 panic sites) | todo |
