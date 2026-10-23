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

- **The remaining option placements checked**, as the 2026-10-11 interface review planned them: `choose_seat` as central buttons, one per Methuselah; searches (`find_card`, `take_from`, `ash_vampire`) by the zone glowing and its view listing the offered cards, a click taking (chips in the central box for a card no stack holds); per-vampire votes (`cast_votes`) in the vote boxes; and SC's own referendum widget kept or hidden in engine games. Done when each is placed or dropped with the owner. Docs: none.
