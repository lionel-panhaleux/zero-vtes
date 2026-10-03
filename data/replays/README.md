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
