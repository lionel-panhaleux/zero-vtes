# Human games

Games played in the Succubus Club fork ("Export replay": the spec by KRCG
ids and the decision log), kept as the corpus for human games as data
(DESIGN §11.2, `docs/human-play.md` slice 6). Read one with
`uv run python -m train.assess data/replays/<file>.json [checkpoint]`, or
`cargo run -p zvtes-protocol --profile ci --example replay -- <file>`.

A replay holds only option indices: a rule change since the game can make
its log stop replaying (an option refused) or diverge. Each game names the
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

Since copies alike in hand are one choice (owner, 2026-10-04: a discard,
a card to the library's bottom, a play — one option per card, not per
copy), the option indices changed: every game above replays on later
engines only up to its first such choice (within its first ~60
decisions). Read them at `a963b55`, the last commit before.
