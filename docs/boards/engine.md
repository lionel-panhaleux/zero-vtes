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

A shrinking list (CLAUDE.md "Harness"; the milestone tokens: roadmap
"Milestones"). Order: what the community beta's games meet first (the server's
records and restarts, a rule players hit), then the protocol leaks the Succubus
Club fork works around, then the API for outside platforms and contributors,
then the training layer split, the remaining mechanisms, the review leftovers,
the harness.

- **A card put in play by another card is played, with no as-played window and no minion-plays triggers** (owner, 2026-11-11): the default readings, 40.10, 44.4 and 77.3 corrected: such a card has been played, but not in the normal fashion: no as-played window (never canceled as played), and triggers keyed to a minion playing a card do not fire, the card plays it, not the minion (ANK 20201229: Concealed Weapon's weapon "is not played by the minion", Louhi cannot cancel it); requirement and cost bypass unchanged. Done when engine tests cover the window and a minion-plays trigger, and CLAUDE.md's "Default readings" line and the items are corrected, with the digest's entries and §23 K items. Docs: `docs/rules-qa.md`, `docs/rules-digest.md`, CLAUDE.md "Owner rules", `docs/engine.md`.
- **The "before votes and ballots are cast" stage in normal order, the acting Methuselah first** (owner, 2026-11-11): 64.12 corrected: that stage follows normal sequencing, the acting Methuselah first; no vote is cast until all have passed in it (rulebook politics step 2 and sequencing); the engine's caller-last rotation changes. Done when an engine test shows the order and 64.12 is corrected in place, with the digest's entry and its §23 K item; Z-train told if the window order changes anything it encodes. Docs: `docs/rules-qa.md`, `docs/rules-digest.md`, `docs/engine.md`.
- **The burn option under torpor by card type** (owner, 2026-11-11): 33.8, 78.31, 104.8 and 113.49 adjusted: a torpid vampire who meets a card's requirement counts as not meeting it for an action card or a reaction (he cannot play them), so the burn option opens (LSJ 20091203: Emergency Powers, Legacy, Tattoo Signal, Unleash Hell's Fury, Veneficti…); as meeting it for an action modifier (playable on his own leave-torpor action), so it stays closed (LSJ 20091204: Council of Seraphim; ANK 20240610: High Orun "even if they are in torpor"); The Red Question a ruled exception (LSJ 20091204). Done when the burn-option offer follows the test, each case is engine-tested, and the four items are corrected in place, with the digest's entries and §23 K items. Docs: `docs/rules-qa.md`, `docs/rules-digest.md`, `docs/engine.md`.
- **The events tell a card put on an uncontrolled vampire** (Z-add, 2026-11-12, met in round 29, reproduced on main b8d196f): `HOST_SEED_FROM=2 HOST_SEEDS=700 cargo test --profile ci -p zvtes-host --test events the_events_tell` fails "3 paths the events miss": `.seats[].uncontrolled[].cards` (with `.card.copy`, `.card.nth`), seed 59 decision 173, null -> [{"copy":46,"id":100633,"nth":1}]: The Embrace placed in an uncontrolled region is shown by the view and told by no event, so a client following the events misses it; the default sweep (40 seeds and the kept ones) passes. Done when that sweep passes with seed 59 among `crates/host/tests/events.rs`'s kept seeds. Docs: `docs/protocol.md` and its CHANGELOG if an event or a field is added (semver), else none.
- **harness.py stale names a changed rules-qa item's citations**: when a diff changes a rules-qa item's line (a superseded or narrowed reading), `stale` lists every page, digest entry, ledger row and code comment citing that item (`rules-qa N.M`, `(N.M)`), to re-read before review; the friction it closes: 59.14's three blocking rounds (e39549c), 100.1 and 76.2 left cited (b1de838), DESIGN:199 and Screamer after ac981e7. Done when a pytest shows `stale` listing a citing page for a changed item. Docs: CLAUDE.md "Harness" (the stale bullet), the ship skill §3.
