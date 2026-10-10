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
| `zero-vtes-replay-2026-10-04T08-47-02-271Z.json` | uncommitted, after `ea6e93d` | Tzimisce (V5 Companion), **Ministry (V5 Anarch)**, Banu Haqim (V5 Anarch), Nosferatu (V5) | `p5-self@1250` (network) | owner 4 VP, the bots 0 |
| `zero-vtes-replay-2026-10-04T09-22-57-297Z.json` | uncommitted, after `ea6e93d` | Tremere (V5), Gangrel (V5 Anarch), **Ravnos (V5 Companion)**, Toreador (V5) | `p5-self@1250` (network) | owner 2 VP, Toreador 2 |
| `zero-vtes-replay-2026-10-04T12-09-51-567Z.json` | after `3cc7ae3`, before `601bf3c` | **Salubri (V5 Companion)**, Brujah (V5 Anarch), Ministry (V5 Anarch), Gangrel (V5 Anarch) | `p5-self@1250` (network) | owner 4 VP, the bots 0 |
| `zero-vtes-replay-2026-10-04T13-51-13-838Z.json` | `601bf3c` | **Nosferatu (V5)**, Ministry (V5 Anarch), Toreador (V5), Salubri (V5 Companion) | `p5-self@1250` (network, default politics) | owner 2 VP, Ministry 2 |
| `zero-vtes-replay-2026-10-04T14-37-29-282Z.json` | `601bf3c` | **Hecata (V5)**, Malkavian (V5), Toreador (V5), Ventrue (V5) | `p5-self@1250` (network, default politics) | owner 4 VP, the bots 0 |
| `zero-vtes-replay-2026-10-04T15-07-24-918Z.json` | `601bf3c` or later | Malkavian (V5), Ministry (V5 Anarch), Gangrel (V5 Anarch), **Nosferatu (V5)** | `p5-self@1250` (network, default politics) | owner 3 VP, Malkavian 1 |
| `zero-vtes-replay-2026-10-04T15-55-52-662Z.json` | `601bf3c` or later | Tzimisce (V5 Companion), **Hecata (V5)**, Ministry (V5 Anarch), Toreador (V5) | `p5-self@1250` (network, default politics) | owner 3 VP, Ministry 1 |
| `zero-vtes-replay-2026-10-04T19-47-18-904Z.json` | `65d1539` or later | Salubri, Nosferatu, **The Unnamed**, Lasombra | `p7-politics@1400` (network, default politics) | owner 3 VP, Salubri 1 |
| `zero-vtes-replay-2026-10-04T22-26-33-065Z.json` | `cf4a56c` or later | Brujah, **The Unnamed**, Hecata, Ministry | `p7-politics@1400` (network, default politics) | owner 4 VP, the bots 0 |
| `zero-vtes-replay-2026-10-05T06-24-53-391Z.json` | `cf4a56c` or later | Malkavian, Hecata, Gangrel, **The Unnamed** | `p7-politics@1400` (network, default politics) | owner 3 VP, Malkavian 1 |
| `zero-vtes-replay-2026-10-05T21-14-26-429Z.json` | `cf4a56c` or later | Salubri, **The Unnamed**, Nosferatu, Toreador | `p7-politics@1400` (network, default politics) | owner 4 VP, the bots 0 |
| `zero-vtes-replay-2026-10-05T21-45-18-313Z.json` | `cf4a56c` or later | **The Unnamed**, Gangrel, Lasombra, Hecata | `p7-politics@1400` (network, default politics) | owner 4 VP, the bots 0 |
| `zero-vtes-replay-2026-10-06T05-40-36-622Z.json` | uncommitted, after `cf4a56c` | Nosferatu, Brujah, Malkavian, **The Unnamed** | `p7-politics@1400` (network, default politics) | — (log ends at 390) |
| `zero-vtes-replay-2026-10-06T06-18-50-630Z.json` | uncommitted, after `cf4a56c` | Brujah, **The Unnamed**, Salubri, Toreador | `p7-politics@1400` (network, default politics) | owner 4 VP, the bots 0 |
| `zero-vtes-replay-2026-10-07T05-33-18-077Z.json` | uncommitted, after `cf4a56c` | Ravnos, **The Unnamed**, Lasombra, Malkavian | `p7-politics@1800` (network, default politics, temperature 0.5, rescue bonus) | owner 3 VP, Malkavian 1 |
| `zero-vtes-replay-2026-10-08T20-32-01-306Z.json` | the fork's build of that day | Nosferatu, Tzimisce, Salubri, **The Unnamed** | `p7-politics@1800` (network, default politics) | owner 4 VP, the bots 0 |
| `zero-vtes-replay-2026-10-08T20-49-30-439Z.json` | the fork's build of that day | Hecata, Malkavian, Ravnos, **The Unnamed** | `p7-politics@1800` (network, default politics) | owner 3 VP, Malkavian 1 |
| `zero-vtes-replay-2026-10-08T21-17-05-603Z.json` | the fork's build of that day | Malkavian, Ravnos, **The Unnamed**, Toreador | `p7-politics@1800` (network, default politics) | owner 3 VP, Malkavian 1 |
| `zero-vtes-replay-2026-10-09T19-28-48-837Z.json` | the fork's build of that day | Hecata, Banu Haqim, **The Unnamed**, Brujah | `p7-politics@1800` (network, default politics) | — (reads up to 305) |
| `zero-vtes-replay-2026-10-09T19-39-10-276Z.json` | the fork's build of that day | Hecata, Banu Haqim, **The Unnamed**, Brujah | `p7-politics@1800` (network, default politics) | — (reads up to 305) |
| `zero-vtes-replay-2026-10-09T20-04-50-919Z.json` | the fork's build of that day | Tremere, **The Unnamed**, Nosferatu, Malkavian | `p7-politics@1800` (network, default politics) | — (reads up to 195) |
| `zero-vtes-replay-2026-10-09T21-05-23-015Z.json` | the fork's build of that day | Tremere, Salubri, **The Unnamed**, Tzimisce | `p7-politics@1800` (network, default politics) | owner 3 VP, Tremere 1 |
| `zero-vtes-replay-2026-10-09T21-51-54-561Z.json` | the fork's build of that day | Hecata, Salubri, Toreador, **The Unnamed** | `p7-politics@1800` (network, default politics) | owner 4 VP, the bots 0 |
| `zero-vtes-replay-2026-10-09T22-19-25-019Z.json` | the fork's build of that day | Gangrel, Tremere, Toreador, **The Unnamed** | `p7-politics@1800` (network, default politics) | owner 2 VP, Gangrel 1, Tremere 1 |
| `zero-vtes-replay-2026-10-09T22-52-40-908Z.json` | the fork's build of that day | Hecata, Ravnos, Malkavian, **The Unnamed** | `p7-politics@1800` (network, default politics) | owner 3 VP, Ravnos 1 |
| `zero-vtes-replay-2026-10-09T23-20-20-474Z.json` | the fork's build of that day | **The Unnamed**, Nosferatu, Brujah, Ravnos | `p7-politics@1800` (network, default politics) | owner 4 VP, the bots 0 |

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

Since Ventrue Headquarters is usable by its controller only (a fix of
2026-10-04, after `c968dcf`), `…T06-45-40` reads only up to decision 390,
where a bot used the owner's Headquarters; read it whole at `c968dcf`.

The later 2026-10-04 games: `…T08-47-02` and `…T09-22-57` were
exported before choices were logged and converted at `a963b55` (both
whole), `…T12-09-51` converted at `c968dcf` (whole); the others logged
their choices as played. `…T15-07-24` reads up to decision 216 since
the rescue's payer is chosen as it is announced (rules-qa 48.8,
`fd2a9f0`): whole at `779ee91`.

Since a minion whose block attempt was left for another's is not
offered the attempt again while nothing changed (rules-qa 48.6,
`37ad627`), five games stop where a Methuselah swapped back to the
minion left or where the window that offered it is now elided:
`…T07-20-42` at decision 389, `…T08-47-02` at 151 and `…T09-22-57`
at 400 (whole at `65d1539`), `…T08-31-11` at 294 and `…T15-07-24` at
82 (to 609 and 216 at `65d1539`). `…T12-09-51`, `…T13-51-13`,
`…T14-37-29` and `…T15-55-52` still read whole.

The games of 2026-10-04 evening to 2026-10-07 (`…T19-47-18` to
`…T05-33-18`: the bots' decks as the fork names them, the owner's own
deck "The Unnamed") read whole at `cf4a56c` and after.

The games of 2026-10-08 and 2026-10-09 read whole at `adfd807` and
after, but three: `…T19-28-48` and `…T19-39-10` stop at decision 306
and `…T20-04-50` at 196, where an "after" or "during" ability used a
second time is no longer offered (once each, `36ce7d0`).
