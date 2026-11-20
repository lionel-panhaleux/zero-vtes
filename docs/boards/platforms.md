# Platforms board

Goal: the engine played where players already are, first Succubus Club,
with guidance and never an unnatural advantage (roadmap objective C and
the community beta). Skill: `sc-change`. Docs: `docs/platforms/sc.md`
(Thomas's style, branches, PRs, recipes, traps),
`docs/protocol.md` (all a platform reads), `docs/human-play.md` (the
human seat, the fork's interface, "Checks" and "Running the fork").
Platforms read the protocol only: a platform inferring what the protocol
could say is an engine-board line.

A shrinking list (CLAUDE.md "Harness"; the milestone tokens: roadmap
"Milestones"). Order (owner, 2026-10-21: agreed with Thomas, the fork deployed
on its own as a separate platform, bots only for now): what the community beta
needs from the fork first, then the fork's remaining interface items, then other
platforms.


- The fork shows protocol 2.28–2.33's view additions it ignores today: a card's `stored` face-down cards (Hukros, hidden to others), a combat side's `second` and `second_strike` (Winged Second), a block's `joint_blocker` (Coterie Tactics). Done when each is drawn on the table or in the combat panel, and logged. Pages: docs/platforms/sc.md.
