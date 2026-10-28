# Engine board

Goal: the engine clearly separated, complete and legal, agnostic of humans
and bots, behind an API convenient for platforms, agents and developers
(roadmap: architecture, objective C). Skill: `engine-review` on every
engine commit. Docs: `docs/engine.md`, `docs/protocol.md` (the contract,
semver), `crates/api` (`schema.json`, `openapi.json`, `protocol.ts`),
`docs/online-plan.md` (the architecture as built), DESIGN §4, §9. Covers
the engine's architecture, `api`, `host`, `text`, `server`, performance,
the rulebook mechanisms no card carries (card definitions and readings
are the cards board's), and the harness (CLAUDE.md, the `intake`, `ship`
and `upkeep` skills, `docs/index.md`).

A shrinking list (CLAUDE.md "Harness"): one line an ask, its done-condition
and the docs it changes; done is deleted; order is priority; the only
state is `[doing: <session>]`. Order: what the community beta's games meet
first (the server's records and restarts, a rule players hit),
then the protocol leaks the Succubus Club fork works around, then the API
for outside platforms and contributors, then the training layer split,
the remaining mechanisms, the review leftovers, the harness.

- **A contest paid into a negative pool**: the first deep sweep (`data/sweeps/2026-10-28-876f2f81bc3e.txt`, Z-main) fails the fuzz invariant "negative pool at seat 3" in `twda_round_17b` at seed 156 (`FUZZ_TRACE=1 SEED_FROM=156 FUZZ_SEEDS=157 cargo test --profile ci --test fuzz random_games_with_twda_round_17b`), seat 3's last move a `PayContest(Library(CardId(10)))` at its unlock; it fails on e4aa4bd too, so it predates rules-qa Batch 101. Done when the seed passes, kept in the fixture's `seeds`, the cost refused or the oust taken as the rules say. Docs: none.
- **A resumed game asked where its record was not**: the first deep sweep's host run (`HOST_SEEDS=400`) fails `a_game_resumed_from_its_record_plays_on_as_it_was` at seed 110: "decision 44: seat 2 asked in MasterPhase, the record's in CardAsPlayed"; a server restart resumes the beta's games this way. Done when the seed passes and is kept in the sweep. Docs: none.
- **A blood loss with no cause in the events**: the first deep sweep's host run fails `the_events_tell_what_the_views_show` at seed 105: `Blood { minion: 36, delta: -1, blood: 9, cause: None }`. Done when the event names its cause, the seed kept in the sweep (`KEPT` in crates/host/tests/events.rs). Docs: `docs/protocol.md` if a cause is added.
- **"The unnamed" named in the text**: a deeper host sweep (`HOST_SEED_FROM=401 HOST_SEEDS=1200 cargo test --profile ci -p zvtes-host --test events`, Z-add 2026-10-28) fails "no line falls back to an unnamed minion" at seeds 525 and 907: The unnamed (crypt 201411, KRCG's "unnamed, The"), influenced out, blocks, and the text reads "Block with a minion", "Lana Butcher's hunt is blocked by a minion"; `crates/text`'s `Names::minion` falls back because the vampire's name never reaches `Names::minions` (a likely cause: `see` inserts a name only through `numbered`, when the view gives the card a copy). Done when both seeds pass and are kept in the sweep (`KEPT` in crates/host/tests/events.rs), the cause fixed for any minion it reaches, not that vampire alone. Docs: none.
- **The decks the bot needs taught, served**: the platforms board's "Teach the bot" tab (the owner's idea, 2026-10-26) lists the recent TWDA decks the served weights play worst; the server reads the training line's per-deck file for the weights it serves and offers it as data, weakest first, each deck with its decklist, event, date and the measure (games, VP share, the weights' version), beside `/catalog` (a query or its own route), so the fork renders it without inferring. After the training board's "Bots on any deck, and its weakest recent TWDA decks". Done when the route is in protocol §10 and the schema, a server test serves a fixture file, and engine.krcg.org's list carries the served weights' version. Docs: `docs/protocol.md`, `crates/api` (`schema.json`, `openapi.json`, `protocol.ts`), `CHANGELOG.md`.
