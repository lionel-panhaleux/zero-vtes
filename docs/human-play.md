# Human play (P5) — plan and board

Working document for P5: a human plays the engine and its bots through a
[Succubus Club](https://github.com/thomasWajs/succubus-club) (SC) interface.
Settled decisions graduate to DESIGN §8 once the experiment confirms them;
this file carries the plan, the board and the findings until then.

## Goal

**Milestone M1**: the owner plays a full game on this machine against 3
network-policy bots in a local SC fork, SC's training mode reshaped around the
engine. The experiment's purpose is to learn what human–bot interaction needs
(pacing, auto-passes, narration, which inputs are painful) before anything is
proposed upstream.

**Milestone M2**: a proposal to SC's author (Thomas Wajs): an npm package of
the engine (+ policy) and an SC PR adding the engine game type; later a
separately deployed engine server for multiplayer, on the same messages.

## Settled (owner, 2026-09-29)

- **Contract = JSON messages**, not a transport: `new_game(spec)`,
  `step() → Request | Over`, `answer(decision_no, idx)`, `view(seat) →
  TableView`. The wasm module speaks them in-process now; a future server
  speaks the same over WebSocket (+ `GET pending` / `POST answer` for bots and
  async play).
- **Local first**: single client, the engine in the browser (wasm32 builds
  unchanged, verified 2026-09-29). No server, rooms, auth or per-recipient
  redaction for M1; the human could read hidden state through devtools — as in
  SC's TrainBot mode today.
- **`TableView`, a dedicated DTO** in Rust — never `GameState` serialized: the
  engine's layout keeps changing, SC must not depend on it.
- **SC side**: `GameType.Engine` beside `TrainBot`; code against an
  `EngineHost` interface (local wasm now, remote later). Keep TrainBot's
  disclaimer, deck picker, mutation queue and `BOT_PAUSE_TIME` pacing; replace
  `Conductor`, `GovernBot`, `cardImpl/`.
- **Packaging**: `wasm-pack` npm package built from zero-vtes; the SC PR holds
  only the TS adapter, UI and a dependency — no Rust in SC.
- **Opponents**: the trained network (M1 waits for it), not the scripted bots.
- **License**: MIT + the Dark Pack notice (`LICENSE`, `README.md`).
- **Fork**: `../succubus-club`, branch `zero-vtes-engine`, based on upstream
  `3099b59` (2026-09-28).
- **JOL**: out of scope (no license, no structured state API).

## Architecture

```
zero-vtes                                       ../succubus-club (fork)
┌───────────────┐   ┌────────────────────┐      ┌──────────────────────────┐
│ crates/engine │ ← │ crates/protocol    │      │ EngineHost (TS interface)│
│ rules, no deps│   │ serde DTOs:        │      │  ├ LocalEngineHost (wasm)│
└───────────────┘   │  TableView, Option │      │  └ RemoteEngineHost (ws) │
        ↑           │  descriptors, deck │      │ projection: TableView    │
 ┌──────┴──────┐    │  from KRCG ids,    │      │  diffs → SC mutations    │
 │ policy (Rust│    │  copy tracker,     │      │ decision panel; gestures │
 │ inference)  │    │  messages          │      │  → matching options      │
 └─────────────┘    └─────────┬──────────┘      └────────────▲─────────────┘
                     ┌────────┴─────────┐                    │ JSON
                     │ crates/wasm      │  npm package ──────┘
                     │ (wasm-bindgen)   │
                     │ crates/server    │  (M2: WebSocket, same messages)
                     └──────────────────┘
```

- `crates/protocol` keeps the engine dependency-free (serde lives here); both
  the wasm module and a future server are thin shells over it.
- Bots run inside the wasm module as `agent::Agent`s: the host steps the engine
  until a human seat must decide, answering bot seats itself. The same agent
  runs in the server and (if the runtime allows) the Python pool.

### Design items

- **Copy identity.** Hand, library, ash heap are `Vec<u16>` of card *types*;
  only cards in play have arena ids (`CardId`, `MinionId`, never reused). SC
  keys sprites by a per-copy `CardOid`. The protocol keeps a *copy tracker*
  across steps: arena entries map 1:1; zone copies are matched by diffing
  zones (a card leaving hand as a `CardInPlay` of the same card appears is the
  same copy); unmatched copies get fresh ids (as SC's own server re-ids a
  shuffled region). Alternative rejected for now: stable copy ids in the
  engine (touches every zone, hashes, goldens).
- **Phase/turn in `TableView`**: `Engine.flow` is `pub(crate)`; the engine
  needs a small public accessor (phase, active seat, action/combat/referendum
  summaries — `action()`, `combat()`, `referendum()` exist).
- **Option descriptors**: each of the ~60 `Choice` variants gets a structured
  descriptor (kind, card KRCG id, actor, target, amount) — for SC highlights
  and gesture matching — and a text label (card names from `CRYPT`/`LIBRARY`).
  The request also carries the window label in text.
- **Decks**: SC gives `Record<KrcgId, number>`. The protocol builds an owned
  deck from KRCG ids (the engine's `Deck` holds `&'static` slices: needs an
  owned variant) and names unsupported cards.
- **Decision number** on every answer (`Engine.decisions`); a stale answer is
  rejected with the current request. Moot locally, kept for the server.
- **Replay** = (seed, decklists by KRCG id, log): an export from day one, for
  bug reports.
- **Projection** (SC side): diff consecutive `TableView`s into SC mutations
  (`moveCardToRegion`, `changeBlood`, `changePool`, `setLock`, …) so history
  and animations work; engine-computed stats (capacity, disciplines, titles)
  may later overwrite SC's per-card attributes. Card placement (x, y) needs
  SC's layout helpers.
- **Policy inference**: `Policy.forward(tok_ids, tok_f, tok_n, opt_ids, opt_f,
  opt_n)` → ONNX with dynamic axes; inference in Rust (tract/candle, keeps the
  bot an `Agent`) or onnxruntime-web (TS). Decide after the export check.

## Board

Status: `todo` · `doing` · `done` · `blocked(<why>)`. Ordered by slice.

### Slice 1 — protocol (zero-vtes; DESIGN slice 17a)

| # | Item | Status |
|---|---|---|
| 1.1 | `crates/protocol`: crate, serde, JSON message types (`Out`, `AnswerError`), decision numbers | done |
| 1.2 | Engine: `phase()`, `in_flight()` (plays under way), `virtual_hand` public | done |
| 1.3 | `TableView` (per viewer, from `legal_view`) + copy tracker (`track.rs`) | done |
| 1.4 | Option descriptors: structured + text for every `Choice` variant; narration hides what the decision keeps private | done |
| 1.5 | Decks from KRCG ids (`Engine::with_lists`), validation naming unsupported cards, the shipped catalog | done |
| 1.6 | `Session`: one message per decision, bots answered inside, `set_agent` for installed bots; replay | done |
| 1.7 | `crates/wasm` (wasm-bindgen, JSON strings), `wasm-pack` build (1.1 MB), Node smoke test | done |
| 1.8 | `tests/session.rs` sweep: JSON round-trip, unique copies, in-play copies stable, card conservation (per id, omniscient; in total, per viewer), all seats' options described (50 kinds at 300 seeds), no leak, replay; unknown bot styles refused up front | done |

### Slice 2 — SC fork (`../succubus-club`; DESIGN slice 17b)

| # | Item | Status |
|---|---|---|
| 2.1 | Build/run the fork locally: no Firebase/Ably keys needed for solo; `zvtes-wasm` linked from `crates/wasm/pkg` | done |
| 2.2 | `GameType.Engine`, `engine/setup.ts` (`setupEngineGame`: you + 3 bots on shipped precons, random seat), main-menu entry | done |
| 2.3 | `engine/host.ts`: `EngineHost` interface + `LocalEngineHost` over the wasm package; `engine/protocol.ts` types | done |
| 2.4 | `engine/projection.ts`: view → SC mutations (moves, blood, counters, lock, pool, Edge, turn, phase); copy → card map; placement on arrival; gone cards kept as `staleCards` | done |
| 2.5 | `EngineDecisionPanel.vue` in the right column: window, options, hover shows the cards an option names, status while bots decide, result | done |
| 2.6 | `engine/gestures.ts`: the user's moves read as decisions (owner, 2026-09-30: every option should be reachable by a real action; the panel is the fallback) — influence / blood on an uncontrolled vampire → transfer / withdraw (amount n repeats), drag uncontrolled → ready → influence out, hand → ready → play (the minion it is dropped on: actor, else target; another player's region: that player), hand → ash heap → discard, phase / turn buttons → the pass, lock → the minion's action or block, SC's action / block / modifier / reaction declarations, crypt draw; one match is taken, several are put forward in the panel (as are the options about selected cards) | done, untested in a browser by me |
| 2.11 | The console on the table (the free seat, bottom right, scaled with it; floating with 5 players or in focus mode); "Undo the last step" within a multi-step choice (`Session::undo`: terms, allocations, distributions; extended in 2.15) | done, position unseen by me |
| 2.12 | A proper allocation menu for referendum terms (all recipients at once, commit at the end) — the owner's preferred fix over step-by-step + undo. Engine: `Engine::allocation()` (read-only: points left, recipients in asking order, caps, minimum of recipients; terms, cards, shortfall). Protocol: `Out::Decision::allocation`, `Session::allocate(decision, amounts)` (checked whole, then one step per recipient the engine asks; refused amounts change nothing). Fork: steppers per recipient, "Allocate" once valid; the step options hidden meanwhile | done, unseen in a browser by me |
| 2.13 | All votes one way (owner, 2026-09-30: "the most common way to work"): `Out::Decision::votes` (totals each way), `Session::vote_all(decision, favor)` — a standing order for the rest of the polling casting the seat's vampires' votes and its calling card's vote as each turn comes; the Edge, cards burned for a vote, plays and abilities stay single options. Fork: "All my votes for (n)" / "against (n)" | done, unseen in a browser by me |
| 2.10 | Options with no gesture yet (the panel only): each kind given a gesture, an on-card button or its place in the panel — table below | done, unseen in a browser by me; superseded by 2.14–2.17 (the table as the interface): the on-card buttons stay for verbs a click can't say, picks become candidate glows |
| 2.7 | `engine/driver.ts` (the Conductor's counterpart): paced bot decisions narrated in the log, replay export link; SC autosave off for engine games | done |
| 2.8 | Dev soak: `VITE_ENGINE_AUTOPLAY` plays the user's seat at random and checks the game state against every view (`Projection.check`) | done |
| 2.9 | Visual pass in a visible tab: layout of minions/attached cards, panel height vs log, 4-seat readability | done (owner) |

#### The table as the interface (owner, 2026-10-01)

The engine knows exactly what can be done: it replaces SC's guesses
(`selfCanPlay`'s hand glow, the declaration's valid targets, the usage
editor's chips) instead of a list of options. The user looks at the
table, not at a console:

- **Highlights carry the meaning**, one look per kind: *usable* (a card
  in hand that can be played, a minion with something to do), *candidate*
  (click to choose it: who plays the card, its target, a pick), *staged*
  (the card being played, its choice under way).
- **A click narrows the engine's options** (each one is a whole
  combination: card, mode, actor, target). A card in hand clicked keeps
  its plays; then whatever still differs is asked, in table order: the
  minion playing it (candidates glow), the Disciplines and level (chips),
  the target (candidates glow; a seat or "no target" as buttons). One
  option left: it is played. Escape or a right click cancels. Drags
  (2.6) stay: two ways to say the same.
- **The central box** (SC's `CentralBox`) shows what is going on (the
  action, the referendum, the combat: projected onto SC's own panels) and
  the choice under way (the staging prompt, chips, allocation, votes, X).
  SC's controls there that mean an engine option are that option; the
  others are hidden or read-only in engine games.
- **The console shrinks** to the fallback: options with no table object
  (a clan, a seat, searches) in a collapsed list, Undo, the replay link.
- **A mis-click is taken back**: `Session::undo` extended to the user's
  last decision while nothing else was decided since (nothing revealed).

| # | Item | Status |
|---|---|---|
| 2.14 | Plays from hand: engine-driven hand glow, click → staging (actor, mode chips, target) with candidate glows on the table, the prompt in the central box; protocol: `mode` and its printed `tags` on play options (checked against the text in `tests/session.rs`). Fork: `engine/interaction.ts` (the narrowing, the highlights), glows in `CardInHandGO`/`CardGO` (orange usable, blue candidate, yellow staged; SC's glow preferences don't apply in engine games), clicks in `useCardClick` (released without a drag; a glowing card is not selected), `central/EngineStaging.vue`; dev autoplay plays cards by narrowing now and then (throws if a step doesn't narrow) | done, unseen in a browser by me |
| 2.15 | Minions: those with something to do glow; a click opens SC's minion menu (Bleed, Hunt, Card In Hand…) with what the engine doesn't offer greyed; picks (choose a minion or a card) as candidates; undo of the last decision. Protocol: `Session::undo` takes back the seat's last decision when nothing was decided since (no bot, no standing vote order) and nothing hidden was revealed to it (`view::knowledge`: how many of each card it sees; a draw or a reveal grows it); the sweep checks every undo independently (the last decision was the human's, it drew nothing, the same decision and options are asked again: 952 at 24 seeds). Fork: highlights `actor` (orange) and picks as candidates (blue, the 2.10 buttons for them dropped), a "card" step first in the narrowing (what a clicked minion plays: the cards glow), `primedMinion` = a minion with actions offered, SC's menu entries narrowed to the engine's options (greyed when none) plus "Other Actions", SC's Cancel (⟵ / Backspace) = the engine undo, a "Click a glowing card" prompt for picks | done, unseen in a browser by me |
| 2.16 | Central box: the action / referendum / combat projected onto SC's panels (protocol: stealth, intercept, bleed amount, block attempts, each seat's votes); SC's controls mapped or hidden; the allocation, votes and X menus moved there. Protocol: `ActionView` + `source`, `step`, `stealth`, `bleed`, `blocker`, `intercept`, `declined`; `ReferendumView` + `step`, `votes` (each vote cast: seat, source, minion or card, side, n, canceled; the sweep checks they add up to the tally); a `cast_votes` option carries its votes (`amount`). Fork: the projection writes SC's `action` (kind → SC's action type; the action card from limbo; block decisions; impulse = the seat the engine asks) and `referendum` (each vampire's votes, the rest per Methuselah), display only; `ActionInfos` read-only with the engine's pass as its one button; `ReferendumInfos` read-only; SC's vote boxes under the vampires the engine lets vote (its sides, its count); "Attempt Block" under the minions it offers a block with; SC's usage editor hidden; `central/EngineChoices.vue` (allocation, all votes one way, X, combat: no longer in the console); the 2.15 card-granted actions: the card glows, a click asks who takes it | done, unseen in a browser by me |
| 2.17 | The console as the fallback: collapsed option list, undo, replay | done, unseen in a browser by me: the console lists the options with no place on the table or in the central box (`interaction.ts` `answeredOnTable`: glowing cards, minion menus, on-card buttons, vote boxes, Attempt Block, the action's / referendum's pass, the central box's menus) and those the last gesture may mean; "All options (n)" unfolds the rest; undo and replay as before |
| 2.18 | What the bots do, told (owner, after a first game: "hard to track what the bots are doing"). Protocol: every `Out` carries `events` — what happened since the previous message, the engine's elided run included: how an action ended (`Engine::last_outcome`: succeeds, is blocked by X, fails, fizzles), a referendum's result and tally, pool and blood moved, minions and cards in and out of play, torpor, ousts (`narrate.rs`: the table marked after each decision and at the next message; a decision's own effect is its text). Narration: a bleed card says its target ("Play Govern the Unaligned at inferior [dom] with Yael → Bot 2"); terms decisions (of a play, an action, a referendum) name their card ("Kine Resources Contested: Allocate 3 to Bot 2"; the "then …" tail only for the one allocating); a vampire's votes their count. The sweep checks every action's and referendum's end is told with the next message. `ActionView` + the action card's `mode` and `x`, its target when the card has one; `ReferendumView` + the terms chosen (`seats`, `minions`, `clan`, `cards`, `card_allocated`). Fork: events in the log as "Table"; SC's action panel shows the card's Disciplines (icons, level) and X; the referendum panel its card, caller and terms | done, unseen in a browser by me |
| 2.19 | Second live game's feedback (owner). Engine: the Edge's unlock-phase pool keeps the Edge ("if you have the Edge, you may gain 1 pool"; once a phase, `Flow::Abilities::edge_taken`; the scripted bots take it; `Choice::BurnEdge` keeps its name, wire kind `edge_pool`, text "Gain 1 pool (the Edge)"); goldens re-blessed. Protocol: a card's ability names the vampire it is on and what it moves ("Use Yael's Blood Doll: 1 blood to pool"; `OptionView.minion` the host, `blood` ±n); the replay reader (`examples/replay.rs`, `Session::show_all`). Fork: blood added to or taken off a vampire is its Blood Doll's ability that way; pool gained is the Edge's pool; the central box scrolls (a referendum's terms, votes and pass overflowed), the pass in the votes panel; the card to play and a pick's cards also as buttons in the central box | done, unseen in a browser by me |
| 2.20 | Third game's feedback (owner). Engine: "put this card on …" puts the card itself in play, out of the ash heap (rules-qa 47.1: Touch of Oblivion, Sculpt the Flesh, Disarm, Dogged Pursuit, Melange; Pass Through Shadow already, 46.4) — the table showed it in the ash heap with its effect invisible; goldens re-blessed. "Oblivion" (master, 102277) not loading in SC: its image and cardbase entry are there, cause unknown — needs what the owner saw | engine done; Oblivion open |
| 2.21 | Additional strike cards only once the first strike exchange is over, both combatants still ready (owner, 2026-10-02: Arms of Ahriman [obl] offered before strikes; rules-qa 47.2 overrides 4.4): the additional-strike window is no longer open before strikes nor before the first pair resolves (`Combat::windows`); Chun Hei's ability likewise. The KRCG "before or after" rulings (Quickness, Hell-for-Leather: [LSJ 20001206] [ANK 20211124]) order the unlimited [CEL] against a limited card, unchanged. Goldens re-blessed. Also 47.1 stated as the general rule for future cards (the engine's `Held` always moves the card itself) | done |
| 2.22 | Combats told (owner: a block against a hunt "resolved with no combat" — a combat with hand strikes only elides whole). Engine: `Engine::combat_notes` (`None` by default: training and fuzz keep nothing), `CombatNote` at combat start, after each strike pair (round, pair, range, both strikes, damage before prevention) and at combat end. Session: on from the deal, reset after an undo's replay, drained before the other events ("Combat: A vs B", "Round 1, close range: A hand strike (1 damage) — B dodge", "The combat ends"; the blood lost follows as changes). The sweep checks each combat is told begun, then its strikes, then ended | done |
| 2.23 | The log, shorter and whole (owner: "too verbose … still exhaustive but shorter"; the cards played during an action kept in sight). The engine's narration is the log: SC's own lines for the table's moves are no longer written in engine games (`History.addGameMutation(…, log)`, off while projecting) — no more "Move hidden card from …", "Discard X" for a card played, pool and blood twice. Narration adds what only those lines told: draws ("Bot 1 draws 2 cards", "… draws 1 crypt card": a copy from the library to the hand, the crypt to the uncontrolled region, in the full view) and the turn ("Turn 5: Bot 2"). Decisions are their text alone (a bare pass with its window); the same line again from the same author is counted ("( ×3 )"). The cards played during an action (or its referendum, its combat) are listed by whom in the central box (`EnginePlayed`) until the action is over, hovering shows the card | done, unseen in a browser by me |
| 2.24 | A card put on a minion lands under it (owner: Villein "should be attached below the vampire, not put in the middle"). A card played waits in limbo, mid-region; attached, it stayed in the same region and the projection moved cards only between regions. Fork: `Projection.move` also moves a card whose spot or host changed within its region (limbo → on a minion, from one host to another; the user's own arrangement otherwise kept); a card on a minion peeks out below and to the right of it, its host brought back on top | done, unseen in a browser by me |

#### 2.10 — where each option kind is answered

The 58 option kinds (`describe.rs`), by how the user reaches them. Rule: a
table move where SC has the UI for it; no imitation of a UI SC lacks
(combat is a stub there): buttons instead.

| Kinds | How | Status |
|---|---|---|
| play, take_action, declare_block, discard, bottom, burn_in_play, transfer, withdraw, influence_out, draw_crypt, draw_library, edge_pool, cast_edge, burn_for_vote, burn_pool, pay_contest, yield_contest, pay_block_toll, pay_to_cancel, pass | Table moves (2.6) | done |
| choose_minion, choose_card, take_equipment, slot, resolve_rider, resolve_printed, burn_blood_from, burn_blood_for_intercept, unlock_for_blood, burn_corruption_to_fail, burn_corruption_to_unlock, draw_stored, round_strike (at a card), burn_weapon_strike, cast_votes | **On-card buttons**: SC's floating actions (as its Bleed/Hunt/Block) under the card the option is about, while that decision is pending; the panel keeps them too | done, unseen |
| set_x | **Stepper** in the panel: the offered values, then "X = n" | done, unseen |
| hand_strike, round_strike (no card), strike_combat_ends, committed_strike, use_maneuver, use_press, use_prevent, take_damage_after_resolution, gain_strength, burn_blood_to_avoid, continue_action, diablerise | **Combat strip** in the panel: who fights, round, range, then these buttons; strike cards and maneuvers from hand stay plays (drag them) | done, unseen |
| choose_seat | Panel (a seat is not a card; a button on each player's pool widget would do: later if missed) | panel |
| find_card, take_from, ash_vampire, employ_from_hand (searches: the cards are off the table) | Panel; a search viewer (the library or ash heap opened, a card dragged out) is real work: after M1 if searches feel bad | panel |
| choose_clan, pay_rescue, use_ability | Panel for the first two (no table object); use_ability is reached by locking the card (a minion's, a location's, a master card's) and in the panel | panel / done |
| allocate_to, allocate_to_card | Allocation menu (2.12) | done |
| cast_votes, cast_card_vote | All votes one way (2.13); per vampire: on-card buttons. **Owner's call**: SC's own referendum widget (`ReferendumVoteBoxes`, `REFERENDUM_castVote` = cast_votes 1:1) needs the projection to drive SC's referendum state (call, tally, close) from `view.referendum` | open |

### Slice 3 — network policy in the browser (DESIGN slice 17c)

| # | Item | Status |
|---|---|---|
| 3.1 | ONNX export check: works (`scripts/export_onnx.py`, dynamo exporter, a `Dense` wrapper; onnxruntime diff ≤ 1.1e-5) | done |
| 3.2 | Inference choice: tract runs it in wasm but adds 3.3–4.9 MB gzipped; **by hand in Rust** chosen (no dependency, the bot stays an `Agent`) | done |
| 3.3 | `crates/policy`: `Net` (the forward pass of `Policy`, one decision), `NetAgent` (samples as training does); `train/export.py` → `.zvp`; parity in `train/tests/test_export.py` through `zvtes_py.policy_forward` (re-keyed vocabulary included) | done |
| 3.4 | `Game.withPolicy(spec, weights)` (style `network`); the fork fetches `/assets/zvtes-policy.zvp` (1.53 MB, git-excluded there) and falls back to the scripted bots without it; ~27 ms per bot decision in wasm | done (packaging: 5.6) |
| 3.5 | The weights of the run to play: `runs/p4-deck/best.pt` once trained (the fork holds `p4-critic/best.pt`, trained before the deck tokens) | todo |
| — | **M1: first game** | ready for the owner (after 3.5, or now against p4-critic) |

### Slice 4 — ergonomics (driven by M1 findings)

| # | Item | Status |
|---|---|---|
| 4.1 | Auto-pass preferences per window / per card; "stops" | todo |
| 4.2 | Narration events ("Bot 2 bleeds you for 2") where diffs fall short | first cut in 2.18 (events); combat told in 2.22; cards canceled, what a card did in words: todo |
| 4.3 | Findings-driven items | todo |

### Slice 5 — upstream (M2)

| # | Item | Status |
|---|---|---|
| 5.1 | npm publish; engine repo public | todo |
| 5.2 | SC PR + proposal to Thomas | todo |
| 5.3 | `crates/server` (WebSocket, rooms, redaction, stale answers) | todo |
| 5.4 | Server messages: `view` optional or diffed (every `Out` carries a full `TableView` — fine in-process, heavy on a wire) | todo |
| 5.5 | npm package metadata: `license-file`, repository (wasm-pack warns) | todo |
| 5.6 | Where the weights ship: in the npm package, or an asset the app serves; versioned with the engine (the `.zvp` re-keys its vocabulary, not its features) | todo |
| 5.7 | Resume an engine game: `Session` from a replay (bots' decisions from the log) | todo |

## Protocol as built (slice 1)

- `Session::advance()` → `Out`: `decision` (a human seat must answer; repeated
  until answered), `decided` (one decision taken — a bot's now, or the human's
  answer), `over`. Every message carries `events` (2.18: what happened
  since the previous message, before it) and `view`: the viewer's table *as of that
  decision* (elided steps applied), so a client diffs its last view against it,
  animates, then narrates or asks. `answer(decision, index)` refuses a stale
  number.
- `TableView`: per seat pool, VPs, Edge, prey/predator, zones as `CardView
  {copy, id (KRCG, null when hidden), crypt}`, uncontrolled slots, minions
  (ready/torpor, with their cards, blood, capacity, title, markers), cards in
  play on no minion, limbo (plays under way); action/combat/referendum
  summaries; phase.
- Options: `{index, kind, text, card {id, copy, card_id}, minion, actor,
  target, amount, favor}`.
- Bots: `Player::Bot {style}`: `aggro_bleed`, `wall`, `random` built in; any
  other style is installed by the host (`Session::set_agent`) — the network
  policy's entry point (slice 3).
- Dev tools: `cargo run -p zvtes-protocol --example narrate -- [seed] [n]`
  prints a game as the human reads it. A game exported from the fork
  ("Export replay") reads back with `cargo run -p zvtes-protocol --profile ci
  --example replay -- <replay.json> [--options] [--table N]...`: every
  decision of every seat, its choice and the events, every card shown;
  `--options` lists each decision's options, `--table N` prints the table
  (pools, minions, blood, cards, hands) before decision N.

### Checks

- `cargo test --profile ci -q` covers the protocol (default member);
  `PROTOCOL_SEEDS=n` deepens `tests/session.rs`.
- `tests/session.rs` `allocations_and_vote_orders_at_once`: every seat
  human on the decks holding allocation cards; engine-side shapes (cards,
  shortfall caps) in `tests/politics.rs` / `tests/precon.rs`
  (`…allocation…_read_whole`).
- `cargo clippy -p zvtes-wasm --profile ci -- -D warnings`;
  `wasm-pack build crates/wasm --target nodejs --out-dir ../../target/wasm-node
  && node crates/wasm/tests/smoke.mjs`.

## Running the fork (slice 2)

```
wasm-pack build crates/wasm --target web          # in zero-vtes: crates/wasm/pkg
cd ../succubus-club && npm install                # once; links zvtes-wasm
npx vite --config vite/config.dev.mjs --port 5188 # then http://localhost:5188
```

Network bots: `uv run python -m train.export runs/<name>/best.pt
../succubus-club/public/assets/zvtes-policy.zvp` (without the file the bots
are the scripted ones). `node crates/wasm/tests/smoke.mjs` plays a game
against them too when `target/policy.zvp` exists.

Pick a deck (Preconstructed → Fifth Edition …: the engine refuses a deck with
cards outside its pool, naming them), then "Play against Bots (rules engine)".
Dev flags: `VITE_FAST_TRACK_ENGINE_GAME=1` starts a game at once;
`VITE_ENGINE_AUTOPLAY=1` plays your seat at random with the consistency check.
The fork's own checks: `npx vue-tsc --noEmit`, `npx eslint .`,
`npx prettier --check .`.

What the fork changes in SC: `src/client/engine/` (new), `store/engine.ts`,
`EngineDecisionPanel.vue`, and small hooks in `types/state.ts` (`GameType`),
`state/gameMutations.ts` (the gate), `state/setup.ts` (stop on reset),
`gateway/savedGames.ts`, `GameRightColumn.vue`, `MainMenu.vue`, Vite config.

## Open questions (owner)

1. "Waiting on seat X" leaks that X holds a play (elision hides only what the
   log shows). Hide, delay, or accept? (server only; a rules-qa batch.)
2. Spectator visibility: public only, or omniscient with delay? (server only.)
3. When to contact Thomas: after M1, or earlier to shape the protocol?

## Findings (M1 experiment)

Before M1 (from the narration example):

- Influence is one transfer per decision: a bot's influence phase is 4–5
  messages, a human's as many clicks. Candidate for slice 4 ("transfer N" as
  a client-side macro over unit decisions — the factoring rule stays).
- A card's definition may have more modes than its printed Discipline lines
  (`cards::lookup(c).modes` vs `LIBRARY[c].modes`): option text falls back to
  "(option n)" there; a better label needs the definition's own mode names.
- `UseAbility` text is "Use <card>" (+ ability number): no ability text yet.
- Card conservation found the action card of a political action shown twice
  once it entered play (Toreador Justicar): limbo now drops the action's card
  once it entered play or returned to hand. An ousted Methuselah's cards
  another controls stay on the table (rulebook "Ousting"), so conservation is
  bounded (surviving decks ≤ shown ≤ all decks), not exact.
- In SC (soak, 5 full games): a card removed from the game must stay
  resolvable for SC's history (`staleCards`), else archiving throws and — in
  dev, where SC rethrows — stops the loop. SC's `changePool` ousts at 0 pool
  and awards the VP itself: the projection overwrites ousted/VP from the view.
- A hidden browser tab gets no animation frame (Phaser draws nothing) and
  throttled timers: look at the table in a visible tab; the soak mode uses no
  timer.
- An opponent's play or discard used to show as "library → ash heap" (their
  hidden hand keeps its size): the tracker now routes it through the hand.
- The training session's `legal_view` change (own library/crypt visible,
  sorted) is for the policy only: `TableView` shows every crypt and library
  face down to a viewer.
- The policy by hand must follow `train/model.py`'s `Policy.forward`: a change
  there that `crates/policy` lacks fails `train/tests/test_export.py`. A
  checkpoint older than the observation's features (not just its vocabulary)
  still loads but plays off-distribution.
- First owner session: refusing table moves and asking to "pick an option"
  does not work as an interface — the natural move must be the input
  (gestures, 2.6). Editing the fork while a game runs hot-reloads the game
  screen: don't.
- First full game (owner, 2026-09-30) — "an excellent run". Fixed since:
  the console off the log's column; level and targets in the log; undo in
  multi-step choices; three engine rules (rules-qa 46.1–46.3: same modifier
  or reaction once per minion per action; no burned vote for the caller; vote
  locations are their controller's — a bot had locked the owner's Power
  Structure). Clarified, engine unchanged: Camarilla's Iron Fist always
  allocated all 5 points (the last Methuselah got the rest, unasked: the
  options now say so); Amici Noctis' distribution is a walk over the eligible
  Lasombra (now worded "1 blood to X" / "No blood to X"), automatic when each
  gets 1. Pass Through Shadow [OBL] now goes on the vampire as itself (rules-qa
  46.4); effects whose card stays in the ash heap (21.1: Touch of Oblivion,
  Sculpt the Flesh) show as a marker on their minion. The bots did not vote
  against the owner when
  it paid to (policy quality — the checkpoint predates the deck tokens).
- Log noise: every bot pass is narrated, and SC logs each mutation beside the
  narration (slice 4: fold passes, drop redundant lines).
- All votes one way casts one source per rotation (polling commits one
  source per decision): the order answers the seat's later turns itself.
  A vote bonus reaches only a vampire that has not cast yet — play cards
  before ordering the votes (the panel says so).
- Not reached by the sweep yet: a banished vampire's face-down cards (out of
  play, shown nowhere) — if the bounds ever fail there, list them under the
  uncontrolled slot.
