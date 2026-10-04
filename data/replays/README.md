# Human games

Games played in the Succubus Club fork ("Export replay": the spec by KRCG
ids and the decision log), kept as the corpus for human games as data
(DESIGN §11.2, `docs/human-play.md` slice 6). Read one with
`uv run python -m train.assess data/replays/<file>.json [checkpoint]`, or
`cargo run -p zvtes-protocol --profile ci --example replay -- <file>`.

A replay holds each decision twice: its option index (`log`) and what
was chosen (`choices`: the card by KRCG id, the minion, seat, target,
mode or amount — `zvtes_protocol::logged`). Read back, a decision takes
the option that is that choice, whatever its index, so a game keeps
reading when the engine changes the options it offers. A change in the
rules the game met still stops it (the engine asks a decision the game
never had, or no longer offers what was chosen). Each game names the
engine commit its wasm was built from — check out that commit to read it
as it was played.

| File | Engine | Seats (in order; the human's in bold) | Bots | Result |
|---|---|---|---|---|
| `2026-10-03T09-03-44.json` | `8c53ed9` | Tzimisce (V5 Companion), **Lasombra (V5)**, Banu Haqim (V5 Anarch), Lasombra (V5) | `p5-self@1250` (network) | owner 4 VP, the bots 0 |
| `zero-vtes-replay-2026-10-03T14-08-09-808Z.json` | uncommitted, after `ea6e93d` | **Gangrel (V5 Anarch)**, Ventrue (V5), Tremere (V5), Salubri (V5 Companion) | `p5-self@1250` (network) | — |
| `zero-vtes-replay-2026-10-03T14-41-09-643Z.json` | uncommitted, after `ea6e93d` | Nosferatu (V5), Tremere (V5), Brujah (V5 Anarch), **Hecata (V5)** | `p5-self@1250` (network) | owner 3 VP, Tremere 1 |
| `zero-vtes-replay-2026-10-04T05-06-55-138Z.json` | uncommitted, after `ea6e93d` | Tzimisce (V5 Companion), **Brujah (V5 Anarch)**, Ventrue (V5), Ministry (V5 Anarch) | `p5-self@1250` (network) | owner 4 VP, the bots 0 |
| `zero-vtes-replay-2026-10-04T05-34-07-935Z.json` | uncommitted, after `ea6e93d` | **Tremere (V5)**, Toreador (V5), Gangrel (V5 Anarch), Ventrue (V5) | `p5-self@1250` (network) | owner 3 VP, Gangrel 1 |
| `zero-vtes-replay-2026-10-04T06-45-40-714Z.json` | uncommitted, after `ea6e93d` | **Ventrue (V5)**, Salubri (V5 Companion), Toreador (V5), Ravnos (V5 Companion) | `p5-self@1250` (network) | owner 4 VP, the bots 0 |
| `zero-vtes-replay-2026-10-04T07-20-42-703Z.json` | uncommitted, after `ea6e93d` | **Toreador (V5)**, Gangrel (V5 Anarch), Banu Haqim (V5 Anarch), Ministry (V5 Anarch) | `p5-self@1250` (network) | owner 3 VP, Banu Haqim 1 |
| `zero-vtes-replay-2026-10-04T08-31-11-014Z.json` | uncommitted, after `a963b55` | Hecata (V5), **The Unnamed**, Ministry (V5 Anarch), Lasombra (V5) | `p5-self@1250` (network) | — (log ends at 642) |

Since rules-qa 48.1 (intercept not floored at 0), `2026-10-03T09-03-44`
replays on later engines only up to decision 72: its Form of the Bat
[pro] at #73 left a 0-intercept blocker at −1, so the fixed engine asks
a decision the game never had. Read it whole at `8c53ed9`.

The 2026-10-03 and 2026-10-04 games were played on wasm builds of the
working tree between commits (the engine column names the last commit
under them). On the tree of 2026-10-04 09:30 the four 2026-10-04 games
replay whole; `…T14-08-09` stops at decision 228 and `…T14-41-09` at
242 (both readable up to there). The bots in all of them sample every
decision, terms and votes included: the default politics
(`zvtes::politics`, DESIGN §11.2) came after.

The games above were exported before choices were logged, and were
converted (2026-10-04, `cargo run -p zvtes-protocol --profile ci
--example replay -- <file> --upgrade <out>`) on the commit each read
furthest on: the 2026-10-03 games and `…T05-34-07` at `ea6e93d` (the
three 2026-10-03 whole), the others at `a963b55` (`…T06-45-40` and
`…T07-20-42` whole; `…T05-06-55` to decision 486, `…T08-31-11` to 610).
Past its converted choices a game reads by its indices. Since copies
alike in hand are one choice (`3cc7ae3`: option indices changed), the
games read by their choices: whole for `…T06-45-40` and `…T07-20-42`
(the latter passing over one discard between two copies alike); the
others up to a rules fix since their game (rules-qa batch 48,
`a963b55`): decisions 74, 203, 47, 486, 56 and 609 of
`2026-10-03T09-03-44`, `…T14-08-09`, `…T14-41-09`, `…T05-06-55`,
`…T05-34-07` and `…T08-31-11`.
