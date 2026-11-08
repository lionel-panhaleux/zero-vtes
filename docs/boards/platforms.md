# Platforms board

Goal: the engine played where players already are, first Succubus Club,
with guidance and never an unnatural advantage (roadmap objective C and
the community beta). Skill: `sc-change`. Docs: `docs/platforms/sc.md`
(Thomas's style, branches, PRs, recipes, traps),
`docs/protocol.md` (all a platform reads), `docs/human-play.md` (the
human seat, the fork's interface, "Checks" and "Running the fork").
Platforms read the protocol only: a platform inferring what the protocol
could say is an engine-board line.

A shrinking list (CLAUDE.md "Harness"): one line an ask, its done-condition
and the docs it changes; done is deleted; order is priority; the only
state is `[doing: <session>]`; waiting on a person is a dated ask.
Order (owner, 2026-10-21: agreed with Thomas, the fork deployed on its
own as a separate platform, bots only for now): what the community
beta needs from the fork first, then the fork's remaining
interface items, then other platforms.

- **A "Teach the bot" tab on the deck screen** [doing: Z-succubus]: the owner's idea (2026-10-26): beside the fork's preconstructed decks (`DeckPanel.vue`), the recent TWDA decks the served bot plays worst, weakest first, each with its event, date and the bot's measure, one line saying why ("The bot plays these decks worst: show it how"); a pick is the player's deck as a precon is, the bots' decks unchanged. After the engine board's "The decks the bot needs taught, served". Done when a game with a listed deck plays on sc-bot and the owner has seen the tab. Docs: `docs/platforms/sc.md`.
- **The bots play recent TWDA decks**: the owner (2026-11-08): "we should let the bot play recent (last 6 months) TWDA decks"; the fork's bot seats draw among the precons and the catalog's TWDA decks dated within six months of the game's day (`setup.ts`'s `ENGINE_BOT_DECK` name regex replaced by the catalog's own kind and date), never the player's own deck, each bot named by the deck's title; precons alone while fewer than three recent decks remain (the newest in the catalog is 2026-06-07). After the engine board's "`GET /catalog` says what each deck is". Done when a game on sc-bot deals a recent TWDA deck to a bot and the owner has seen it. Docs: `docs/platforms/sc.md`.
