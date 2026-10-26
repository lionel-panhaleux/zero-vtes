# Human games

Game records (docs/protocol.md §11: the spec by KRCG ids, each decision
by its option index and its `ChoiceKey` with the engine's copies), kept
as the corpus for human games as data (DESIGN §11.2). The server records
every game it hosts in Postgres; 24 of these were played in the Succubus
Club fork (its "Export replay": a spec and a decision log) and converted
to records on 2026-10-13 (`p6-online`; the originals are in the history
as `data/replays`, read whole by the legacy reader then: every decision
kept); the others are the server's records, promoted from the harvest
(`data/server-records`, the training skill's "Promotion",
`scripts/promote.py`) under their game id, after a check of what a
stranger's record holds (no field the script does not know, no text but
the server's seat names, the deck names Succubus Club gives its bots and
a known client; a feedback text only once the owner has read it). From protocol 2.4.0 a record carries each
decision's position digest: the corpus test then stops a game whose
position an engine change moved (hidden state included), not only one
whose options changed. Read one with `uv run python -m train.assess
data/records/<file>.json [checkpoint]`, or `cargo run -p zvtes-host
--profile ci --example replay -- <file>`; `crates/host/tests/records.rs`
checks they read whole, or stop where `KNOWN_STOPS` says.

A record is read back by key: a decision takes the option that is the recorded
choice, whatever its index, so a game keeps reading when the engine changes
the options it offers. A change in the rules the game met still stops it (the
engine asks a decision the game never had, or no longer offers what was
chosen) — but for what a record's engine did not ask, read past
(`Replay::elided`, `Reader::next`): the block windows offering only attempts
that cannot reach the stealth, which an engine before rules-qa 73.8 skipped;
the block window an engine before rules-qa 82.1 reopened after an attempt
failed short of the stealth, its passes through the blocker read past, and
after a play there (read as the engine asks it) the round of passes that
closed it; the unlock riders' order recorded for Anarch Revolt before rules-qa
88.5, a rider the engine offers by its key only; the options a record's
protocol did not offer yet (`Then`: the burn option of a master card requiring
a ready Seraph before 2.12.0, a withdrawal before 2.13.0, a path before
2.14.0, the combat windows of a Methuselah outside the combat before 2.15.0,
rules-qa 96's before 2.16.0); and a Pass recorded by a seat the engine now
elides there (`Engine::elided_passes`; a second Wall of Filth, refused by
rules-qa 99.1, left the seat a lone Pass), as sure as the digest after it. A
record a rules change stops for good — it took a choice the rules now refuse
(an attempt after its blocker's decline, rules-qa 82.1), or the change moved
its hidden state (the library a diablerie's Discipline search shuffles,
advanced rules 4), or, from before 2.4.0, which records no windows, it is read
past a window the engine no longer asks — is listed in
`crates/host/tests/records.rs` (`KNOWN_STOPS`: the file, the decision, the
change), asserted to stop exactly there: `…10-08T21-17-05` at decision 215,
the second Wall of Filth its owner was offered (rules-qa 99.1). Each converted
game names the engine commit its wasm was built from below — check out that
commit to read it as it was played. These converted games carry neither their
bots' nudges nor their turn count (the Bots column says the run); a server
record does from protocol 1.7.0 on (`bots[].nudges`, `result.turns`).

| File (`.json`) | Engine | Seats (in order; the human's in bold) | Bots | Result |
|---|---|---|---|---|
| `zero-vtes-replay-2026-10-04T12-09-51-567Z.json` | after `3cc7ae3`, before `601bf3c` | **Salubri (V5 Companion)**, Brujah (V5 Anarch), Ministry (V5 Anarch), Gangrel (V5 Anarch) | `p5-self@1250` (network) | owner 4 VP, the bots 0 |
| `zero-vtes-replay-2026-10-04T15-55-52-662Z.json` | `601bf3c` or later | Tzimisce (V5 Companion), **Hecata (V5)**, Ministry (V5 Anarch), Toreador (V5) | `p5-self@1250` (network, default politics) | owner 3 VP, Ministry 1 |
| `zero-vtes-replay-2026-10-04T19-47-18-904Z.json` | `65d1539` or later | Salubri, Nosferatu, **The Unnamed**, Lasombra | `p7-politics@1400` (network, default politics) | owner 3 VP, Salubri 1 |
| `zero-vtes-replay-2026-10-05T06-24-53-391Z.json` | `cf4a56c` or later | Malkavian, Hecata, Gangrel, **The Unnamed** | `p7-politics@1400` (network, default politics) | owner 3 VP, Malkavian 1 |
| `zero-vtes-replay-2026-10-05T21-14-26-429Z.json` | `cf4a56c` or later | Salubri, **The Unnamed**, Nosferatu, Toreador | `p7-politics@1400` (network, default politics) | owner 4 VP, the bots 0 |
| `zero-vtes-replay-2026-10-08T20-32-01-306Z.json` | the fork's build of that day | Nosferatu, Tzimisce, Salubri, **The Unnamed** | `p7-politics@1800` (network, default politics) | owner 4 VP, the bots 0 |
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
| `zero-vtes-replay-2026-10-11T06-58-41-552Z.json` | the fork's build of `7fedb25` | Nosferatu, Tzimisce, **The Unnamed**, Hecata | `p8-nudges@2600` (network, default politics) | owner 3 VP, Nosferatu 1 |
| `zero-vtes-replay-2026-10-11T12-31-56-022Z.json` | the fork's build of that day | Ministry, **The Unnamed**, Toreador, Malkavian | `p8-nudges@2600` (network, default politics) | owner 3 VP, Malkavian 1 |
| `zero-vtes-replay-2026-10-11T15-45-47-868Z.json` | the fork's build of that day | **The Unnamed**, Nosferatu, Ventrue, Lasombra | `p8-nudges@2600` (network, default politics) | owner 3 VP, Nosferatu 1 |
| `zero-vtes-replay-2026-10-12T21-02-13-473Z.json` | the fork's build of `ff2c6e8` | Ministry, **The Unnamed**, Banu Haqim, Ventrue | `p9@3400` (network, its training's nudges) | owner 3 VP, Ventrue 1 |
| `zero-vtes-replay-2026-10-12T21-50-21-712Z.json` | the fork's build of `ab7acd0` (bots dealt every full precon, SC `bd3eb72`) | Brujah, Banu Haqim, **The Unnamed**, Path of Death | `p9@3400` (network, its training's nudges) | owner 3 VP, Brujah 1 |

`…T12-09-51` was played on a wasm build of the working tree between
commits (its engine column names the last commit under it), exported
before choices were logged and converted at `c968dcf`; the others logged
their choices as played. Its bots sample every decision, terms and votes
included: the default politics (`zvtes_env::politics`, DESIGN §11.2) came
after.

The server's records promoted (each names the engine it was played on, by
its commit prefix, and its protocol):

| File (`.json`) | Engine | Seats (in order; the human's in bold) | Bots | Result |
|---|---|---|---|---|
| `d8ec2c01edab010d.json` | `237fd5d42a79` (protocol 2.4.0), 2026-10-19 | **Lasombra (Fifth Edition)**, Banu Haqim (Fifth Edition (Anarch)), Brujah (Fifth Edition (Anarch)), Ministry (Fifth Edition (Anarch)) | `df66d54badf3` (network, t 0.5) | the human 3 VP, Brujah (Fifth Edition (Anarch)) 1 VP, last standing; 2 undos |
| `da59eaaa7e4abda6.json` | `1f616016b7cd` (protocol 2.6.0), 2026-10-22 | **Salubri (Fifth Edition (Companion))**, Tzimisce (Fifth Edition (Companion)), The Endless Dance (Thirtieth Anniversary), Toreador (Fifth Edition) | `a78e74292acb` (network, t 0.5) | the human 3 VP, The Endless Dance (Thirtieth Anniversary) 1 VP, last standing; 4 undos |
| `57b15cfc6c44ad37.json` | `aa50ca0624fe` (protocol 1.0.0), 2026-10-15 | Brujah (Fifth Edition (Anarch)), **Path of Death (Sabbat V5)**, Path of Caine (Sabbat V5), Tzimisce (Fifth Edition (Companion)) | `7aaa9528fbf8` (network, t 0.5) | the human 3 VP, Tzimisce (Fifth Edition (Companion)) 1 VP, last standing |
| `ec8d47141b6b38e9.json` | `c5d9f13c87e2` (protocol 1.2.0), 2026-10-15 | Ventrue (Fifth Edition), **Nosferatu (Fifth Edition)**, Malkavian (Fifth Edition), Tzimisce (Fifth Edition (Companion)) | `7aaa9528fbf8` (network, t 0.5) | the human 3 VP, Tzimisce (Fifth Edition (Companion)) 1 VP, last standing |
| `5a98a36c86873c35.json` | `814c2a90030b` (protocol 2.10.0), 2026-10-24 | **custom**, Nosferatu (Fifth Edition), The Endless Dance (Thirtieth Anniversary), Hecata (Fifth Edition) | `a78e74292acb` (network, t 0.5) | the human 2 VP, Nosferatu (Fifth Edition) 2 VP, last standing; 6 undos |

Every game here reads whole on the current engine, but those `KNOWN_STOPS`
lists stop where it says (`crates/host/tests/records.rs`). A game a rules
fix stops partway, or an unfinished one, is otherwise removed (owner,
2026-10-10, 2026-10-11); the removed games and where each stopped are in
the history (`e5f8f54` and after). The engines from 22cf131 (the first
1.9.0) to 8587a25, never deployed, left no record here.
