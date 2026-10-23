# Platforms board

Goal: the engine played where players already are, first Succubus Club,
with guidance and never an unnatural advantage (roadmap objective C and
the community beta). Skill: `sc-change`. Docs: `docs/platforms/sc.md`
(Thomas's style, branches, PRs, recipes, traps),
`docs/protocol.md` (all a platform reads), `docs/human-play.md` (how the
first fork was built; its "Checks" and "Running the fork" are current).
Platforms read the protocol only: a platform inferring what the protocol
could say is an engine-board line.

A shrinking list (CLAUDE.md "Harness"): one line an ask, its done-condition
and the docs it changes; done is deleted; order is priority; the only
state is `[doing: <session>]`; waiting on a person is a dated follow-up.
Order (owner, 2026-10-21: agreed with Thomas, the fork deployed on its
own as a separate platform, bots only for now): what the community
beta needs from the fork first, then the fork's remaining
interface items, then other platforms.

- **The demo video**: a storyboard and a fast-track seed (deck pick, glows and staging, an action with a block and a combat, a referendum with all votes one way, the bots' narration, Opportunities, undo, the record), for the owner to record on the fork's public URL. Done when the owner has the storyboard and the seed. Docs: none.
- **The target step's prompt, short** (owner, 2026-10-16: yes, "the Toreador to lock" for Toreador Grand Ball; "the minion to put this card on" for Priority Contract), built from the option's effects (`lock` `on: target`, the second's `role`); the first pick's clan from the protocol, or an engine-board line if it does not say it. Done when the fork prompts so. Docs: none.
- **The fork's inferences dropped**, each after its engine-board line: the fork reads the protocol's field instead. From 2.5.0: `decided`'s `source` and `purpose` (a terms answer with its card, not `TERMS` windows and the English), a bleed's `target` (not the preys tracked), combat choices' `actor` (not `combatOf` through the view), `ability`/`apart` (not the English cut at ": "), `target` seats (not cut at " on "), `Marker.text` and strike `text` (not `MARKER_NAMES`, `STRIKES`), the option keys complete (not names regex-matched in the English, `fromText`); after the engine board's "A played card's copy in the protocol" (plays matched by copy, not id) and "What a pass does, in its data" (`isEffectPass`, `declinesBlock`). Done when the fork reads each of these fields and infers none. Docs: sc.md.
- **The remaining option placements checked**, as the 2026-10-11 interface review planned them: `choose_seat` as central buttons, one per Methuselah; searches (`find_card`, `take_from`, `ash_vampire`) by the zone glowing and its view listing the offered cards, a click taking (chips in the central box for a card no stack holds); per-vampire votes (`cast_votes`) in the vote boxes; and SC's own referendum widget kept or hidden in engine games. Done when each is placed or dropped with the owner. Docs: none.
- **Other platforms**: JOL (VTES Online) and Elysium (https://elysium.cards, https://github.com/dtagent2000/VTES-Elysium): how each runs games, what each needs of the API, what plugging the engine in would take. Done when a page per platform says so and the engine board has its asks. Docs: `docs/platforms/<name>.md`, `docs/index.md`.
