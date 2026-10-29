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

- **Two pool gains told as one uncaused**: Z-add's deep host sweep (`HOST_SEEDS=700` events test, 2026-10-29, on its round 23 tables, `../zero-vtes-r30`) panics at seed 392 "no cause: Pool { seat: 2, delta: 10, pool: 29, cause: None }": the oust's 6 pool to the predator (`Rule::Oust`) and Khabar: Glory's +4 under its card's cause told as one event, no cause. Done when each gain is told with its cause (the host's pool events per cause) and the seed is kept in the sweep (`KEPT` in crates/host/tests/events.rs), after round 23 lands. Docs: none.
- **"The unnamed" named in the text**: a deeper host sweep (`HOST_SEED_FROM=401 HOST_SEEDS=1200 cargo test --profile ci -p zvtes-host --test events`, Z-add 2026-10-28) fails "no line falls back to an unnamed minion" at seeds 525 and 907 (and, at `HOST_SEEDS=400` on c492a50, seed 121 in `every_option_and_choice_is_told`: "a minion: −1 blood (0), by Helena", "Use a minion: burn 1 blood"): The unnamed (crypt 201411, KRCG's "unnamed, The"), influenced out, blocks, and the text reads "Block with a minion", "Lana Butcher's hunt is blocked by a minion"; `crates/text`'s `Names::minion` falls back because the vampire's name never reaches `Names::minions` (a likely cause: `see` inserts a name only through `numbered`, when the view gives the card a copy). Done when the three seeds pass and are kept in the sweep (`KEPT` in crates/host/tests/events.rs), the cause fixed for any minion it reaches, not that vampire alone. Docs: none.
- **The decks the bot needs taught, served**: the platforms board's "Teach the bot" tab (the owner's idea, 2026-10-26) lists the recent TWDA decks the served weights play worst; the server reads the training line's per-deck file for the weights it serves and offers it as data, weakest first, each deck with its decklist, event, date and the measure (games, VP share, the weights' version), beside `/catalog` (a query or its own route), so the fork renders it without inferring. After the training board's "Bots on any deck, and its weakest recent TWDA decks". Done when the route is in protocol §10 and the schema, a server test serves a fixture file, and engine.krcg.org's list carries the served weights' version. Docs: `docs/protocol.md`, `crates/api` (`schema.json`, `openapi.json`, `protocol.ts`), `CHANGELOG.md`.
