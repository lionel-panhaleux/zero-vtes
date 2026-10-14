# Platforms board

Goal: the engine played where players already are, first Succubus Club,
with guidance and never an unnatural advantage (roadmap objective C and
the community beta). Skill: `sc-change`. Docs: `docs/platforms/sc.md`
(Thomas's style, branches, PRs, recipes, traps, the boundary leaks),
`docs/protocol.md` (all a platform reads), `docs/human-play.md` (how the
first fork was built; its "Checks" and "Running the fork" are current).
Platforms read the protocol only: a platform inferring what the protocol
could say is an engine-board item.

Status: `todo` · `doing` · `blocked:<what>` · `later` · `done`. Order: what the
community beta needs from Succubus Club first, then Thomas's PRs and the
proposal, then the fork's remaining interface items, then other platforms.

| # | Item | Why / source | Status |
|---|---|---|---|
| P1 | How beta players reach engine games (owner, 2026-10-14): first, Thomas gets the chance to ship SC with #285 merged so SC plays against the bots; if he does not move swiftly enough, we deploy our own SC on krcg, or build a platform made for the engine (always engine games, no free-play mode) | beta (roadmap) | blocked:Thomas (#285) |
| P2 | Rejoin checked on a local server: a refresh or a dropped socket mid-game rejoins (`join {after: seq}`, `RemoteEngineHost`'s resync); sc.md's "an engine game has no resume" corrected, or the gap filed on the engine board | beta; online-plan C (resync on reconnect); human-play 5.7 | todo |
| P3 | A "report a problem" entry in engine games: text tied to the game and the decision (E2), and the community channel the notice names | beta (feedback intake) | blocked:E2 |
| P4 | The engine mode's notice checked in #285 and the fork: recorded two years, anonymous (as `docs/protocol.md` says), where to send feedback | online-plan C; settled 5 | todo |
| P5 | PRs #282 (textures), #283 (precon update), #284 (layout), #285 (engine game): Thomas's review answered on the owner's word, every reaction logged in sc.md | sc.md PRs; online-plan C; human-play 5.2 | blocked:Thomas |
| P6 | The re-cut kept current: every fork fix after `a7515b1` translated into `engine-game` (sc.md recipe), on the owner's go | sc.md branches; online-plan C | todo |
| P7 | The demo video: a storyboard and a fast-track seed (deck pick, glows and staging, an action with a block and a combat, a referendum with all votes one way, the bots' narration, Opportunities, undo, the record); the owner records it on `engine.krcg.org` | online-plan E | todo |
| P8 | The proposal to Thomas with the protocol doc, the PR and the video (the PRs are open and the owner told him in person about the AI involvement, 2026-10-14) | online-plan D, order 5; human-play open question 3 | blocked:P7, E11 |
| P9 | The referendum and action panels run over the seat's board (vote totals, "Played during the action" cut off) | findings P8 | todo |
| P10 | The interface review's leftovers: glow forms (recommendation 5: actor vs usable, the staged colour) and the referendum trims (recommendation 4); which of its patch (batches 1–5, the side panel removed) is in `zero-vtes-engine` checked | `docs/sc-engine-ui-review.md` (2026-10-11; untracked in the shared checkout) | todo |
| P11 | The opposing declared strike shown before one's own (`CombatView.sides`) | human-play 2.28 (the fork's display pending) | todo |
| P12 | The target step's prompt from `ModeView.target` (Toreador Grand Ball, Priority Contract) | human-play 2.30 (waits the owner's go) | blocked:owner |
| P13 | The Opportunities control: checked in #285's controls commit, then closed | human-play 2.31 | todo |
| P14 | The texture fix seen in a browser (Oblivion, 47th Street Royals; #282) by the owner | human-play 2.20, 2.25 | blocked:owner |
| P15 | Counted markers as chips: in the fork (22fa13d) and folded into #285; Thomas asked whether they stay in `CardGO.vue`; the findings row closed with his answer | findings P13; sc.md PRs (#285's question) | blocked:Thomas |
| P16 | The fork's inferences dropped as the protocol says them: ability labels (E6), the user (E7), the reason (E8), marker names and numbered minions (E9) | sc.md boundary leaks | blocked:E6–E9 |
| P17 | Guidance audit of the fork's engine mode against E13's rule: no reminder of what the engine leaves to the player | objective C; sc-change rule 7 | blocked:E13 |
| P18 | The remaining option placements (choose_seat, searches, per-vampire votes and SC's own referendum widget) checked against the interface review; closed or kept | human-play 2.10 | todo |
| P19 | Several humans at an SC engine table, bots in the free seats (after E15) | objective B; online-plan order 6 | blocked:E15 |
| P20 | Other platforms (owner, 2026-10-14): JOL (VTES Online) and Elysium (https://elysium.cards, https://github.com/dtagent2000/VTES-Elysium): how each runs games, what each needs of the API (E12), what plugging the engine in would take | objective C | todo |
