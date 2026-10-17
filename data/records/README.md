# Human games

Game records (docs/protocol.md §11: the spec by KRCG ids, each decision
by its option index and its `ChoiceKey` with the engine's copies), kept
as the corpus for human games as data (DESIGN §11.2). The server records
every game it hosts in Postgres; these 28 were played in the Succubus Club
fork (its "Export replay": a spec and a decision log) and converted to
records on 2026-10-13 (`p6-online`; the originals are in the history as
`data/replays`, read whole by the legacy reader then: every decision
kept). Read one with `uv run python -m train.assess
data/records/<file>.json [checkpoint]`, or `cargo run -p zvtes-host
--profile ci --example replay -- <file>`; `crates/host/tests/records.rs`
checks they all read whole.

A record is read back by key: a decision takes the option that is the
recorded choice, whatever its index, so a game keeps reading when the
engine changes the options it offers. A change in the rules the game met
still stops it (the engine asks a decision the game never had, or no
longer offers what was chosen) — but for the block windows offering only
attempts that cannot reach the stealth, which an engine before rules-qa
73.8 skipped: a record reading further past them is read so
(`Replay::elided`). Each game names the engine commit its wasm was built
from below — check out that commit to read it as it was played. These
converted games carry neither their bots' nudges nor their turn count
(the Bots column says the run); a server record does from protocol
1.7.0 on (`bots[].nudges`, `result.turns`).

| File (`.json`) | Engine | Seats (in order; the human's in bold) | Bots | Result |
|---|---|---|---|---|
| `zero-vtes-replay-2026-10-04T12-09-51-567Z.json` | after `3cc7ae3`, before `601bf3c` | **Salubri (V5 Companion)**, Brujah (V5 Anarch), Ministry (V5 Anarch), Gangrel (V5 Anarch) | `p5-self@1250` (network) | owner 4 VP, the bots 0 |
| `zero-vtes-replay-2026-10-04T15-55-52-662Z.json` | `601bf3c` or later | Tzimisce (V5 Companion), **Hecata (V5)**, Ministry (V5 Anarch), Toreador (V5) | `p5-self@1250` (network, default politics) | owner 3 VP, Ministry 1 |
| `zero-vtes-replay-2026-10-04T19-47-18-904Z.json` | `65d1539` or later | Salubri, Nosferatu, **The Unnamed**, Lasombra | `p7-politics@1400` (network, default politics) | owner 3 VP, Salubri 1 |
| `zero-vtes-replay-2026-10-05T06-24-53-391Z.json` | `cf4a56c` or later | Malkavian, Hecata, Gangrel, **The Unnamed** | `p7-politics@1400` (network, default politics) | owner 3 VP, Malkavian 1 |
| `zero-vtes-replay-2026-10-05T21-14-26-429Z.json` | `cf4a56c` or later | Salubri, **The Unnamed**, Nosferatu, Toreador | `p7-politics@1400` (network, default politics) | owner 4 VP, the bots 0 |
| `zero-vtes-replay-2026-10-05T21-45-18-313Z.json` | `cf4a56c` or later | **The Unnamed**, Gangrel, Lasombra, Hecata | `p7-politics@1400` (network, default politics) | owner 4 VP, the bots 0 |
| `zero-vtes-replay-2026-10-08T20-32-01-306Z.json` | the fork's build of that day | Nosferatu, Tzimisce, Salubri, **The Unnamed** | `p7-politics@1800` (network, default politics) | owner 4 VP, the bots 0 |
| `zero-vtes-replay-2026-10-08T20-49-30-439Z.json` | the fork's build of that day | Hecata, Malkavian, Ravnos, **The Unnamed** | `p7-politics@1800` (network, default politics) | owner 3 VP, Malkavian 1 |
| `zero-vtes-replay-2026-10-08T21-17-05-603Z.json` | the fork's build of that day | Malkavian, Ravnos, **The Unnamed**, Toreador | `p7-politics@1800` (network, default politics) | owner 3 VP, Malkavian 1 |
| `zero-vtes-replay-2026-10-09T21-51-54-561Z.json` | the fork's build of that day | Hecata, Salubri, Toreador, **The Unnamed** | `p7-politics@1800` (network, default politics) | owner 4 VP, the bots 0 |
| `zero-vtes-replay-2026-10-09T22-19-25-019Z.json` | the fork's build of that day | Gangrel, Tremere, Toreador, **The Unnamed** | `p7-politics@1800` (network, default politics) | owner 2 VP, Gangrel 1, Tremere 1 |
| `zero-vtes-replay-2026-10-09T22-52-40-908Z.json` | the fork's build of that day | Hecata, Ravnos, Malkavian, **The Unnamed** | `p7-politics@1800` (network, default politics) | owner 3 VP, Ravnos 1 |
| `zero-vtes-replay-2026-10-09T23-20-20-474Z.json` | the fork's build of that day | **The Unnamed**, Nosferatu, Brujah, Ravnos | `p7-politics@1800` (network, default politics) | owner 4 VP, the bots 0 |
| `zero-vtes-replay-2026-10-10T09-00-31-909Z.json` | the fork's build of that day | Nosferatu, Tremere, Hecata, **The Unnamed** | `p8-nudges@2600` (network, default politics) | owner 3 VP, Tremere 1 |
| `zero-vtes-replay-2026-10-10T14-28-58-609Z.json` | the fork's build of that day | Malkavian, Lasombra, **The Unnamed**, Ventrue | `p8-nudges@2600` (network, default politics) | Lasombra 3 VP, Ventrue 1 |
| `zero-vtes-replay-2026-10-10T15-01-52-455Z.json` | the fork's build of that day | Nosferatu, Ventrue, Tzimisce, **The Unnamed** | `p8-nudges@2600` (network, default politics) | owner 3 VP, Ventrue 1 |
| `zero-vtes-replay-2026-10-10T15-16-15-595Z.json` | the fork's build of that day | Banu Haqim, **The Unnamed**, Salubri, Tremere | `p8-nudges@2600` (network, default politics) | owner 3 VP, Tremere 1 |
| `zero-vtes-replay-2026-10-10T16-45-56-513Z.json` | the fork's build of that day | Toreador, **The Unnamed**, Ventrue, Nosferatu | `p8-nudges@2600` (network, default politics) | owner 4 VP, the bots 0 |
| `zero-vtes-replay-2026-10-10T18-44-21-018Z.json` | the fork's build of that day | Salubri, Nosferatu, **The Unnamed**, Ravnos | `p8-nudges@2600` (network, default politics) | owner 3 VP, Ravnos 1 |
| `zero-vtes-replay-2026-10-10T21-16-37-859Z.json` | the fork's build of that day | Hecata, Ventrue, Toreador, **The Unnamed** | `p8-nudges@2600` (network, default politics) | owner 4 VP, the bots 0 |
| `zero-vtes-replay-2026-10-10T22-05-29-762Z.json` | the fork's build of that day | Tremere, Hecata, Ravnos, **The Unnamed** | `p8-nudges@2600` (network, default politics) | owner 3 VP, Hecata 1 |
| `zero-vtes-replay-2026-10-10T22-26-31-637Z.json` | the fork's build of that day | Tzimisce, Brujah, Malkavian, **The Unnamed** | `p8-nudges@2600` (network, default politics) | owner 3 VP, Brujah 1 |
| `zero-vtes-replay-2026-10-11T06-58-41-552Z.json` | the fork's build of `7fedb25` | Nosferatu, Tzimisce, **The Unnamed**, Hecata | `p8-nudges@2600` (network, default politics) | owner 3 VP, Nosferatu 1 |
| `zero-vtes-replay-2026-10-11T12-31-56-022Z.json` | the fork's build of that day | Ministry, **The Unnamed**, Toreador, Malkavian | `p8-nudges@2600` (network, default politics) | owner 3 VP, Malkavian 1 |
| `zero-vtes-replay-2026-10-11T15-45-47-868Z.json` | the fork's build of that day | **The Unnamed**, Nosferatu, Ventrue, Lasombra | `p8-nudges@2600` (network, default politics) | owner 3 VP, Nosferatu 1 |
| `zero-vtes-replay-2026-10-11T21-28-54-517Z.json` | the fork's build of `1307d09` | Hecata, Tzimisce, **The Unnamed**, Brujah | `p8-nudges@2600` (network, default politics) | owner 4 VP, the bots 0 |
| `zero-vtes-replay-2026-10-12T21-02-13-473Z.json` | the fork's build of `ff2c6e8` | Ministry, **The Unnamed**, Banu Haqim, Ventrue | `p9@3400` (network, its training's nudges) | owner 3 VP, Ventrue 1 |
| `zero-vtes-replay-2026-10-12T21-50-21-712Z.json` | the fork's build of `ab7acd0` (bots dealt every full precon, SC `bd3eb72`) | Brujah, Banu Haqim, **The Unnamed**, Path of Death | `p9@3400` (network, its training's nudges) | owner 3 VP, Brujah 1 |

The games before 2026-10-04 12:00 were played on wasm builds of the
working tree between commits (the engine column names the last commit
under them); `…T12-09-51` was exported before choices were logged and
converted at `c968dcf`; the others logged their choices as played. The
bots of the games before `601bf3c` sample every decision, terms and
votes included: the default politics (`zvtes::politics`, DESIGN §11.2)
came after.

Every game here reads whole on the current engine
(`crates/host/tests/records.rs`). A game a rules fix stops partway, or an
unfinished one, is removed (owner, 2026-10-10, 2026-10-11); the removed
games and where each stopped are in the history (`e5f8f54` and after). A
record from before rules-qa 82.1 reads the acting seat's pass after a failed
block attempt where this engine asks it, at the open window's end.
