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
| `zero-vtes-replay-2026-10-04T12-09-51-567Z.json` | after `3cc7ae3`, before `601bf3c` | **Salubri (V5 Companion)**, Brujah (V5 Anarch), Ministry (V5 Anarch), Gangrel (V5 Anarch) | `p5-self@1250` (network) | owner 4 VP, the bots 0 |
| `zero-vtes-replay-2026-10-04T13-51-13-838Z.json` | `601bf3c` | **Nosferatu (V5)**, Ministry (V5 Anarch), Toreador (V5), Salubri (V5 Companion) | `p5-self@1250` (network, default politics) | owner 2 VP, Ministry 2 |
| `zero-vtes-replay-2026-10-04T14-37-29-282Z.json` | `601bf3c` | **Hecata (V5)**, Malkavian (V5), Toreador (V5), Ventrue (V5) | `p5-self@1250` (network, default politics) | owner 4 VP, the bots 0 |
| `zero-vtes-replay-2026-10-04T15-55-52-662Z.json` | `601bf3c` or later | Tzimisce (V5 Companion), **Hecata (V5)**, Ministry (V5 Anarch), Toreador (V5) | `p5-self@1250` (network, default politics) | owner 3 VP, Ministry 1 |
| `zero-vtes-replay-2026-10-04T19-47-18-904Z.json` | `65d1539` or later | Salubri, Nosferatu, **The Unnamed**, Lasombra | `p7-politics@1400` (network, default politics) | owner 3 VP, Salubri 1 |
| `zero-vtes-replay-2026-10-05T06-24-53-391Z.json` | `cf4a56c` or later | Malkavian, Hecata, Gangrel, **The Unnamed** | `p7-politics@1400` (network, default politics) | owner 3 VP, Malkavian 1 |
| `zero-vtes-replay-2026-10-05T21-14-26-429Z.json` | `cf4a56c` or later | Salubri, **The Unnamed**, Nosferatu, Toreador | `p7-politics@1400` (network, default politics) | owner 4 VP, the bots 0 |
| `zero-vtes-replay-2026-10-05T21-45-18-313Z.json` | `cf4a56c` or later | **The Unnamed**, Gangrel, Lasombra, Hecata | `p7-politics@1400` (network, default politics) | owner 4 VP, the bots 0 |
| `zero-vtes-replay-2026-10-06T05-40-36-622Z.json` | uncommitted, after `cf4a56c` | Nosferatu, Brujah, Malkavian, **The Unnamed** | `p7-politics@1400` (network, default politics) | — (log ends at 390) |
| `zero-vtes-replay-2026-10-06T06-18-50-630Z.json` | uncommitted, after `cf4a56c` | Brujah, **The Unnamed**, Salubri, Toreador | `p7-politics@1400` (network, default politics) | owner 4 VP, the bots 0 |
| `zero-vtes-replay-2026-10-07T05-33-18-077Z.json` | uncommitted, after `cf4a56c` | Ravnos, **The Unnamed**, Lasombra, Malkavian | `p7-politics@1800` (network, default politics, temperature 0.5, rescue bonus) | owner 3 VP, Malkavian 1 |
| `zero-vtes-replay-2026-10-08T20-32-01-306Z.json` | the fork's build of that day | Nosferatu, Tzimisce, Salubri, **The Unnamed** | `p7-politics@1800` (network, default politics) | owner 4 VP, the bots 0 |
| `zero-vtes-replay-2026-10-08T20-49-30-439Z.json` | the fork's build of that day | Hecata, Malkavian, Ravnos, **The Unnamed** | `p7-politics@1800` (network, default politics) | owner 3 VP, Malkavian 1 |
| `zero-vtes-replay-2026-10-08T21-17-05-603Z.json` | the fork's build of that day | Malkavian, Ravnos, **The Unnamed**, Toreador | `p7-politics@1800` (network, default politics) | owner 3 VP, Malkavian 1 |
| `zero-vtes-replay-2026-10-09T21-05-23-015Z.json` | the fork's build of that day | Tremere, Salubri, **The Unnamed**, Tzimisce | `p7-politics@1800` (network, default politics) | owner 3 VP, Tremere 1 |
| `zero-vtes-replay-2026-10-09T21-51-54-561Z.json` | the fork's build of that day | Hecata, Salubri, Toreador, **The Unnamed** | `p7-politics@1800` (network, default politics) | owner 4 VP, the bots 0 |
| `zero-vtes-replay-2026-10-09T22-19-25-019Z.json` | the fork's build of that day | Gangrel, Tremere, Toreador, **The Unnamed** | `p7-politics@1800` (network, default politics) | owner 2 VP, Gangrel 1, Tremere 1 |
| `zero-vtes-replay-2026-10-09T22-52-40-908Z.json` | the fork's build of that day | Hecata, Ravnos, Malkavian, **The Unnamed** | `p7-politics@1800` (network, default politics) | owner 3 VP, Ravnos 1 |
| `zero-vtes-replay-2026-10-09T23-20-20-474Z.json` | the fork's build of that day | **The Unnamed**, Nosferatu, Brujah, Ravnos | `p7-politics@1800` (network, default politics) | owner 4 VP, the bots 0 |
| `zero-vtes-replay-2026-10-10T08-03-34-644Z.json` | the fork's build of that day | Tzimisce, **The Unnamed**, Ministry, Brujah | `p8-nudges@2600` (network, default politics) | owner 3 VP, Brujah 1 |
| `zero-vtes-replay-2026-10-10T08-23-07-092Z.json` | the fork's build of that day | Gangrel, Malkavian, **The Unnamed**, Lasombra | `p8-nudges@2600` (network, default politics) | Lasombra 3 VP, Malkavian 1 |
| `zero-vtes-replay-2026-10-10T09-00-31-909Z.json` | the fork's build of that day | Nosferatu, Tremere, Hecata, **The Unnamed** | `p8-nudges@2600` (network, default politics) | owner 3 VP, Tremere 1 |
| `zero-vtes-replay-2026-10-10T14-28-58-609Z.json` | the fork's build of that day | Malkavian, Lasombra, **The Unnamed**, Ventrue | `p8-nudges@2600` (network, default politics) | Lasombra 3 VP, Ventrue 1 |
| `zero-vtes-replay-2026-10-10T15-01-52-455Z.json` | the fork's build of that day | Nosferatu, Ventrue, Tzimisce, **The Unnamed** | `p8-nudges@2600` (network, default politics) | owner 3 VP, Ventrue 1 |
| `zero-vtes-replay-2026-10-10T15-16-15-595Z.json` | the fork's build of that day | Banu Haqim, **The Unnamed**, Salubri, Tremere | `p8-nudges@2600` (network, default politics) | owner 3 VP, Tremere 1 |
| `zero-vtes-replay-2026-10-10T16-45-56-513Z.json` | the fork's build of that day | Toreador, **The Unnamed**, Ventrue, Nosferatu | `p8-nudges@2600` (network, default politics) | owner 4 VP, the bots 0 |
| `zero-vtes-replay-2026-10-10T18-44-21-018Z.json` | the fork's build of that day | Salubri, Nosferatu, **The Unnamed**, Ravnos | `p8-nudges@2600` (network, default politics) | owner 3 VP, Ravnos 1 |
| `zero-vtes-replay-2026-10-10T21-00-19-649Z.json` | the fork's build of that day | Tremere, Banu Haqim, Malkavian, **The Unnamed** | `p8-nudges@2600` (network, default politics) | owner 3 VP, Banu Haqim 1 |
| `zero-vtes-replay-2026-10-10T21-16-37-859Z.json` | the fork's build of that day | Hecata, Ventrue, Toreador, **The Unnamed** | `p8-nudges@2600` (network, default politics) | owner 4 VP, the bots 0 |
| `zero-vtes-replay-2026-10-10T22-05-29-762Z.json` | the fork's build of that day | Tremere, Hecata, Ravnos, **The Unnamed** | `p8-nudges@2600` (network, default politics) | owner 3 VP, Hecata 1 |
| `zero-vtes-replay-2026-10-10T22-26-31-637Z.json` | the fork's build of that day | Tzimisce, Brujah, Malkavian, **The Unnamed** | `p8-nudges@2600` (network, default politics) | owner 3 VP, Brujah 1 |

The games before 2026-10-04 12:00 were played on wasm builds of the
working tree between commits (the engine column names the last commit
under them); `…T12-09-51` was exported before choices were logged and
converted at `c968dcf`; the others logged their choices as played. The
bots of the games before `601bf3c` sample every decision, terms and
votes included: the default politics (`zvtes::politics`, DESIGN §11.2)
came after.

Every game here reads whole on the current engine (checked with
`zvtes_py.replay_decisions`). The games a rules fix stopped partway were
removed (owner, 2026-10-10: "remove games that are now longer
replayable fully because of fixes"): 17 of them, the 2026-10-03 games,
nine of 2026-10-04 (`…T05-06-55`, `…T05-34-07`, `…T06-45-40`,
`…T07-20-42`, `…T08-31-11`, `…T08-47-02`, `…T09-22-57`, `…T15-07-24`,
`…T22-26-33`), three of 2026-10-09 (`…T19-28-48` and `…T19-39-10`, one
game exported twice, and `…T20-04-50`) and two of 2026-10-10
(`…T15-48-57`, `…T16-13-44`). They and the notes on where each stops
are in the history at `e5f8f54`. A game a future fix stops goes the same
way.
