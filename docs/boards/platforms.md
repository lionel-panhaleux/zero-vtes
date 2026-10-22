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

- **The Imbued in the fork** [doing: Z-play]: protocol 2.7.0 (the cards line's round 19, sent by Z-add 2026-10-22): the `incapacitated` region drawn (`SeatView.incapacitated`, the `incapacitated` event moving a minion between it and `ready`, `Zone::Incapacitated` in card moves), `imbued` minions, the `power` and `burn_imbued` actions, the six new purposes and `looked.zone` read from the protocol; decklists with Imbued crypt cards, Powers and Convictions accepted by the fork's deck import. Done when an Imbued deck plays a game on a local 2.7.0 server with the region shown and nothing inferred. Docs: sc.md (if the fork learns something of SC's deck import).
- **The engine mode's notice checked**: in the fork, it says games are recorded two years, anonymous (as `docs/protocol.md` says), and where to send feedback. Done when it says all three. Docs: sc.md.
- **The referendum and action panels inside the seat's board**: vote totals and "Played during the action" are cut off today. Done when both show whole at the fork's smallest supported size. Docs: none.
- **"Report a problem" in engine games**: the `feedback {text}` command (protocol 1.2.0, live), its text tied to the game and the decision, and the community channel the notice names. Done when a report sent from the fork shows in `/admin/feedback`. Docs: sc.md.
- **Export Replay before the game's end**: from protocol 1.7.0, `GET /games/{id}/record` answers 409 until `over`, and `host.ts` `replay()` returns `res.text()` unchecked. Done when the fork's Export Replay says the record comes at the game's end before it, and saves it after. Docs: none.
- **The demo video**: a storyboard and a fast-track seed (deck pick, glows and staging, an action with a block and a combat, a referendum with all votes one way, the bots' narration, Opportunities, undo, the record), for the owner to record on the fork's public URL. Done when the owner has the storyboard and the seed. Docs: none.
- **The target step's prompt, short** (owner, 2026-10-16: yes, "the Toreador to lock" for Toreador Grand Ball; "the minion to put this card on" for Priority Contract), built from the option's effects (`lock` `on: target`, the second's `role`); the first pick's clan from the protocol, or an engine-board line if it does not say it. Done when the fork prompts so. Docs: none.
- **The fork's inferences dropped**, each after its engine-board line: the fork reads the protocol's field instead. From 2.5.0: `decided`'s `source` and `purpose` (a terms answer with its card, not `TERMS` windows and the English), a bleed's `target` (not the preys tracked), combat choices' `actor` (not `combatOf` through the view), `ability`/`apart` (not the English cut at ": "), `target` seats (not cut at " on "), `Marker.text` and strike `text` (not `MARKER_NAMES`, `STRIKES`), the option keys complete (not names regex-matched in the English, `fromText`); after the engine board's "A played card's copy in the protocol" (plays matched by copy, not id) and "What a pass does, in its data" (`isEffectPass`, `declinesBlock`). Done when the fork reads each of these fields and infers none. Docs: sc.md.
- **The remaining option placements checked**, as the 2026-10-11 interface review planned them: `choose_seat` as central buttons, one per Methuselah; searches (`find_card`, `take_from`, `ash_vampire`) by the zone glowing and its view listing the offered cards, a click taking (chips in the central box for a card no stack holds); per-vampire votes (`cast_votes`) in the vote boxes; and SC's own referendum widget kept or hidden in engine games. Done when each is placed or dropped with the owner. Docs: none.
- **Other platforms**: JOL (VTES Online) and Elysium (https://elysium.cards, https://github.com/dtagent2000/VTES-Elysium): how each runs games, what each needs of the API, what plugging the engine in would take. Done when a page per platform says so and the engine board has its asks. Docs: `docs/platforms/<name>.md`, `docs/index.md`.
