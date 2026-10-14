# zero-vtes roadmap (2026-10-14)

The objectives, the near milestone, and where every open step of the
earlier plans went. The work itself is on four boards, one per line of
work (CLAUDE.md "Lines of work"): [cards](boards/cards.md),
[engine](boards/engine.md), [training](boards/training.md),
[platforms](boards/platforms.md). `docs/findings.md` stays the shared
inbox; each open row there has a board item.

DESIGN.md keeps the settled design and the history of the slices and the
runs; `docs/online-plan.md` and `docs/human-play.md` keep the online and
in-browser plans as built. Their open steps are carried over below and
now live on the boards.

## Where things stand

- **Engine**: every V5 card defined, plus 21 batches of added cards (the
  anniversary and Sabbat decks, legacy cards for TWDA decks): 739 library
  cards and 483 crypt cards (297 vampire definitions); 440 TWDA decks fully in the pool, 114 in the kept
  corpus; 1,748 Rust tests; rulings as tests (`data/rulings-v5.csv`);
  owner rulings in `docs/rules-qa.md` (Batches 1–73), digested by
  mechanism in `docs/rules-digest.md`.
- **Bot**: p10 served on `engine.krcg.org`; p11 training (a learner
  partner across the table); ~30 owner games as the imitation set.
- **Online**: the server on `engine.krcg.org` is authoritative (bots
  server-side, every game in Postgres, kept two years); the contract is
  `crates/api` (`docs/protocol.md`, semver); the in-browser engine is gone.
- **Succubus Club**: the fork plays on the server; four PRs upstream
  (#282–#285, the engine game type in #285), waiting on Thomas.

## The first milestone: a community beta

The owner (2026-10-14): the first milestone is close — a working VTES
bot online; bot + Succubus Club is good enough to beta test with players
of the community.

What it still needs:

1. **A way in for testers** (P1, the owner's call): today only the
   owner's local fork serves engine games. Either the fork is deployed
   or the beta waits on #285 upstream (P5).
2. **The best bot served**: p11 finished, measured and served if it
   beats p10 (T1, T2).
3. **Games that survive the server**: games a redeploy left open ended
   (E1); a refresh or a dropped socket rejoins (P2); later, a game
   rebuilt after a restart (E17).
4. **Complete records**: the bots' nudges and the turn count in each
   record (E4).
5. **The loop on testers' games**: the harvest scheduled and its report
   read (T3), promotion to `data/records` (T5), the human win rate as
   the measure (T9).
6. **Feedback intake**: a "report a problem" from inside the game,
   stored with the game's record at its decision (E2, P3), plus one
   community channel the notice names (P4); the owner triages it into
   `docs/findings.md`.
7. **Known rough edges fixed**: the panel overflow (P9); the two card
   bugs found by the comment pass (C2, C3); diablerie's equipment (E3).
8. **What players asked for**: the cards their refused decks named,
   counted (E5), to steer the next rounds (C5).

Not needed for the beta but in the same order of work: the repository
public (E11; online-plan settled it "shortly before" contacting Thomas,
whose PRs are already open), the demo video (P7) and the proposal (P8).

## Objectives

### A. A great opponent

The owner: the bot is already convincing; make it a really great
opponent.

Done means: human win rates against the served bot (T9) at or below a
bar the owner sets (a table's fair share, 1 in 4 or 5, is the natural
one) over a sample the report marks as large enough; the
owner's waste counters (`waste`) at the owner's own rates; no recurring
weakness in the harvest (T10) without a counter; strong on decks it never
trained with (T11). Measured on the "positions where a human beat the
bot" set (T8), never trained on.

### B. Every card

The owner: include all the cards, with a seamless integration loop where
the owner almost never intervenes; easily add new cards as they are
released. So a player can use a bot at their table when missing a
player, or play a full bot table to train and test decks.

Size (KRCG `vtes.json` of 2026-06-30): the full pool is 4,149 cards —
1,785 crypt (1,765 vampires, 20 Imbued) and 2,364 library; 2 crypt and
18 library cards are banned. The engine holds 483 crypt (27%) and 739
library (31%): 2,927 cards to go (2,907 without the 20 Imbued; 2,887
without the 20 banned as well; the Imbued's own library is 14 Powers and
3 Convictions). Whether the Imbued and their cards count, and whether banned cards are out (Lilith's Blessing was
removed), is the owner's (C23).

Done means: every card in scope defined with all its text, its ledger
rows `test:` or `n/a:` (no `blocked:`), its readings graded, and only red
ones asked (add-cards §4); a new set taken in by the recipe (C6) with the
owner answering only red readings; the round choice a script (C5); the
coverage report (C4) at 100%; a player's own deck accepted, a bot in a
free seat (E15, P19), a full bot table to test a deck.

### C. The engine every platform uses

The owner: improve the engine–platform integration — the engine so good
every online platform uses it, even without bots; easy to plug in. The
platform must not help players too much: they may forget effects or miss
opportunities; the frontier between good guidance and an unnatural
advantage is thin.

Done means: a platform plugs in from `docs/protocol.md` and the schema
alone, with a runnable minimal client (E12); no platform infers what the
protocol could say (sc.md's leak list empty: E6–E9, P16); all-human
engine tables (E16); the guidance rule written in protocol §8 and every
setting and the fork checked against it (E13, P17); a second platform
considered (P20).

### Architecture

The owner: the engine clearly separated, complete and legal, agnostic of
humans and bots; a convenient API, easy for AI agents and human
developers; an open-source definition of the game's interactions,
effects and windows; bots plug in as an option; high performance for the
training loop; a layer between the engine and the network for
training-only optimizations (elision, critic …); platform changes
agnostic of engine and bot; open to external contributions and use.

Done means: the engine crate holds the rules only — observations,
habits and scripted bots in `zvtes-env` (E23, E24), bots behind
`host`'s `Bots` with named profiles (E18); every rulebook mechanism
implemented (E3, E10, E27, E28, E29); every card's definition exported as
documented data (E26); performance measured against DESIGN §9 (E30) and
the training iteration's levers measured (T14); the repository public
with a contributor's guide (E11); the protocol versioned with a
changelog and a schema-diff check (E12).

One tension to settle (E25): the owner's words put elision in the
training layer; DESIGN §4.3 holds it as engine semantics (observational
equivalence is tested; replays depend on it). The engine board waits on
the owner's word before moving it.

## Later tracks

Named, not planned yet:

- **A judge bot**: improve `../vtes-advanced-rules`, the rules docs and
  the skills toward a "judge" bot, tested against players' rules
  questions on Discord and the official judges' answers. First inputs:
  the advanced-rules corrections (C24) and the exported definitions
  (E26).
- **Deck evaluation and deck building**: bot simulations to measure a
  deck's strength and weaknesses in a meta, advise, and build strong
  decks (DESIGN "Outlook: deck building": a co-evolved metagame,
  decklist-conditioned pilots, the TWDA as a prior; the precon-tuning
  bridge study first).
- **An MCP server** over these tools, for anyone's agentic AI.

## How the lines work

CLAUDE.md "Lines of work" says who does what and how the lines meet. On
the boards: one table each, `| # | Item | Why / source | Status |`, ids
prefixed C, E, T, P and never renumbered (a done item stays until the
board is pruned; new items take the next number). Status is `todo`,
`doing`, `blocked:<what>` or `done`. The split: card definitions, their
readings and the ledger are the cards board's; a rulebook mechanism no
card carries, the API, host, server and performance the engine board's;
runs, evaluation and the human-game loop the training board's; Succubus
Club and other clients the platforms board's. A finding lands in
`docs/findings.md`, then as an item on the board of the line that fixes
it.

## Owner decisions

Answered 2026-10-14:

1. Testers reach engine games through SC once Thomas ships #285; if he
   does not move swiftly enough, our own SC on krcg, or a platform made
   for the engine (always engine games, no free play) (P1).
2. The repository is public: github.com/lionel-panhaleux/zero-vtes, MIT
   (E11).
3. The Imbued are in "every card"; the banned cards are not (C23).
4. The engine offers every legal play, even one not worth considering
   (E25: the block reachability pruning, to confirm).
5. Guidance border cases, stops, the "waiting on X" tell, spectators:
   a later step (E13, E14, E15, E19).
6. Survival shaping and strength levels: later, on the training side
   (T15, T16).
7. Other platforms: JOL (VTES Online) and Elysium (P20).
8. The daily harvest runs from a pilot session (T3).

Still open: E25's proposal; the fork's target prompt (P12); the texture
check in a browser (P14).

## Carried over

Every open step of the earlier plans, with its board item; steps done or
dropped are listed so nothing is lost silently. "Harvest" = the cards,
training and SC sessions' reviews of 2026-10-14 (drafts folded into the
skills in 24a385d, cab3b1b, c7788c0; what no skill holds is carried
here).

### DESIGN.md

| Step | Where now |
|---|---|
| §11.1 phases and slices | every row done; legacy rounds continue as C1 |
| §11.2 p11 run | T1 |
| §11.2 p11's cross-table and Wider View measures | T2 |
| §11.2 "Faster iterations" (2–5×, ~2.3× reached) | T14 |
| §11.2 chores, nudges, p8–p10 plans | done (their notes) |
| §11.3 inert clauses, no pool tenant | C22 |
| §11.3 paths deferred (the choice; paths in the observations) | E28, T13 |
| §11.3 mechanisms missing: immunity to damage from a source | E29 |
| §11.3 mechanisms missing: tournament withdrawal (§10.6) | E27 |
| §11.3 known edge: `ImmuneToDirectedActions` on card plays only | C19 |
| §11.3 known edge: `Terms::XBlood` past a surcharge (verify) | C17 |
| §11.3 known edge: Disarm's torpor ousting inside the play | C18 |
| §11.3 known edge: True Love's Face for a last pool | C18 |
| §11.3 known edge: Meditative Grove and Regina Blake's blood | C18 |
| §11.3 known edge: `PoolToHostBlood` ousting a non-active seat | C19 |
| §11.3 known edge: a cancelling ability's mark before its terms | C19 |
| §11.3 known edge: a unique title's self-contest by terms (verify) | C17 |
| §11.3 known edge: directed actions name one Methuselah | C19 |
| §11.3 known edge: Rapid Change, Wolf Claws by fuzz only | C20 |
| §11.3 known edge: unbounded decisions within an action | no item: a documented property (CLAUDE.md working lessons; drivers keep a budget) |
| §11.3 a deferred oust (Life Boon) listed in windows | C18 |
| §11.3 legacy additions covered by code paths only | C20 |
| §11.3 P2 deliverable not started: the protocol server | done (P6: `crates/server`, `engine.krcg.org`) |
| §11.3 owner suggestions: hard cards, a sterile baron, the next TWDA ranking step | C21 (its Lasombra and Ravnos G2 done in round 9, 16u) |
| §11.3 owner suggestions: more `ADDED_CARDS`, TWDA unlocks | C1, C5 |
| §11.4 P2 full engine pool | done (every V5 card); the rest of the pool is objective B |
| §11.4 P3 training long enough to beat the bots | done (p9 on: the scripted bots out of evaluation, "too bad to matter") |
| §11.4 P3 frozen snapshots on the ladder | done (the league's `--opponents`) |
| §11.4 P3 the early survival shaping of §5 | T15 |
| §11.4 P4 evaluation vs humans | T8, T9 |
| §11.4 P5 community: bot service, client adapter, ergonomics | done as P6 (server, host, text); the rest E14–E22, P-board |
| §8 preemptive impulse requests ("stops") | E19 |
| §8 bot service on ONNX | dropped: native `.zvp` inference in `policy`, served by the server |
| Outlook: deck building, the precon-tuning bridge study | later track "deck evaluation and deck building" |

### docs/online-plan.md

| Step | Where now |
|---|---|
| A1, A1b, A9, A2a–A2c, A3a–A3c, A5, A8 | done (their rows) |
| A2, A3 (whole) | done through their lettered steps |
| A4 profiles in `policy` (the shells implement `Bots` for now) | E18 |
| A6 panics caught, step budget | done (A1b: caught per command, `engine_error`) |
| A7 left: markers' and strikes' text, unreached purposes, wording | E21 |
| A10 redaction audit | E14 |
| Contract rules: events as a complete delta stream ("later") | E22 |
| B0–B8 | done (their rows) |
| B7 releases from GitHub once the repository is public | E11 |
| Settled 2 / order 6: multi-human after A10 | E15, P19 |
| Settled 7: the repository public | E11 |
| C small PRs (textures, precon update, layout) | P5 (#282–#284 open) |
| C the engine PR re-cut | P5 (#285 open), P6 (kept current) |
| C the engine mode's notice | P4 |
| C resync on reconnect | P2 |
| E demo video | P7 |
| D contact SC's author with the doc, the PR and the video | P8 |
| "Where things stand" audit (prose as data, hand-written replay notes, wire weight, the wasm shell, ungated fork changes, the `file:` link) | done by A–C, or gone with the wasm module |

### docs/human-play.md (its board)

| Step | Where now |
|---|---|
| Slice 1 rows, slice 2 rows marked done | done |
| 2.20 Oblivion not loading → 2.25 | P14 |
| 2.25 textures (fix unseen in a browser) | P14 |
| 2.28 the fork's display of the opposing strike | P11 |
| 2.30 the fork's target prompt | P12 |
| 2.31 the fork's Opportunities button | P13 |
| 2.10 choose_seat, searches, cast_votes / SC's referendum widget | P18, P10 |
| 3.5 the weights of a run to play in the browser | dropped: p10 served on the server, the in-browser policy gone |
| M1 first game | done (2026-09-30) |
| 4.1 auto-pass preferences, stops | E19 |
| 4.2 narration: cards canceled, what a card did | E20 |
| 4.3 findings-driven items | the findings inbox, triaged into the boards |
| 5.1 npm publish; engine repo public | npm dropped (wasm gone); the repository E11 |
| 5.2 SC PR + proposal | P5, P8 |
| 5.3 `crates/server` | done (P6) |
| 5.4 views optional or diffed | done (`views` setting); the delta stream E22 |
| 5.5 npm metadata; 5.6 where the weights ship | dropped: no npm package; weights beside the server binary (B6) |
| 5.7 resume an engine game | E17, P2 |
| 6.2 starts from human games | done (`--starts`) |
| 6.3 imitation term | done (`--bc`) |
| Open question 1: the "waiting on X" tell | E14 |
| Open question 2: spectators | E15 |
| Open question 3: when to contact Thomas | P8 (settled by online-plan settled 6; PRs open, the owner told him in person) |

### docs/findings.md (every row not fixed, closed or dropped)

| Row | Where now |
|---|---|
| F15 records lack nudges and turns | E4 |
| F16 games never ended in Postgres | E1 |
| E7 Wider View | T2 |
| P8 panel overflow | P9 |
| P13 counted markers | P15 (done in the fork and #285; the row closes with Thomas's answer); E9 for the data |
| L14 Sniper Rifle in the slave's combat | C2 |
| L15 strike-only prevention (Rego Motum, Opikun) | C3 |
| L16 diablerie steps 2 and advanced rule 4 | E3, E10 |

### docs/platforms/sc.md

| Step | Where now |
|---|---|
| Boundary leak: ability labels from the text's tail | E6, P16 |
| Boundary leak: an absent actor read as "your Methuselah" | E7, P16 |
| Boundary leak: no reason for a forced hunt | E8, P16 |
| Boundary leak: markers without names, minions alike | E9, P16 |
| PRs #282–#285 waiting; Thomas's reactions to record | P5 |
| #285's question (the chips in `CardGO.vue`) | P15 |
| Trap: "an engine game has no resume" | P2, E17 |

### docs/rules-digest.md §23–§24

| Step | Where now |
|---|---|
| §24.1 implemented readings awaiting the owner (77 in Batches 62–70, 3 question tails) | C7–C11 |
| Open: K5, K6, K7, K15, K16, K18, K23, K26, K27, K29 | C12 |
| Open: K14, K30, K31, K35, K37; K44 open at the source; K25 unverified; K41, K42, K43 not in rules-qa | C13 |
| K11, K12, K13, K19, K28, K34 (the advanced rules' or the KRCG rulings' side) | C24 |
| K1–K4, K8–K10, K17, K20–K22, K24, K32, K33, K36, K38–K40 | settled or stated in the digest: no item |
| §24.2 ledger questions (the Fury's block, Erciyes, Nergal, I am Legion, banished Great Beast, Ferraille) | C14 |
| §24.3 principles not in rules-qa | reference for every round: no item |

### docs/engine-review-2026-10-14.md

| Step | Where now |
|---|---|
| F1–F10 | done (83bb6f3, 37b05e0, 1afa43e, 09da5bb, aee698b, c61c25e, f97107c, 6e6a8bf, d964204, 3050714, cceab76) |
| F11 the pay-or-suffer family (leave) | E33 |
| F12 clans and misplaced comments | done (9860453) |
| F12 the host's silent effect drop | E31 |
| §3 hazards, §4 checklist | done: the `engine-review` skill (241cd9a) and `Effect::family()` / `Requirement::slots()` |
| §5 unchecked: ability targets at resolution, `ChosenController`'s `requires` | E32 |

### Harvest (the sessions' reviews, 2026-10-14)

| Step | Where now |
|---|---|
| Cards: a coverage report of cards defined vs the full pool | C4 |
| Cards: the grading plan, the corrected readings' lessons | done: add-cards §4, §7 |
| Training: weighting the owner's games in `--bc` | T7 |
| Training: "positions where a human beat the bot" | T8 |
| Training: `waste` accepting a directory; its lines in the report | T4 |
| Training: records lacking turns and nudges | E4 |
| Training: a scheduled harvest | T3 |
| Training: privacy of strangers' records | T5 |
| Training: the `zvtes-env` split (obs, habits, bots after `bots::does` moves; agent and elision stay) | E23, E24, T12 |
| SC: the boundary leaks | E6–E9 |
| SC: the re-cut recipe, traps, Thomas's style | done: sc.md and `sc-change` |

### Ledger and pool (no earlier plan)

| Step | Where now |
|---|---|
| 54 `blocked:` ledger rows (44 untested, 10 gaps) | C15, C16 |
| The Imbued and banned cards in or out of scope | C23 |
| `docs/sc-engine-ui-review.md` leftovers (untracked in the shared checkout) | P10 |
