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
Order: held by the owner (2026-10-15) until Thomas gives his blessing
(his review of #285): no platforms line is shipped meanwhile. Then
what the community beta needs from Succubus Club first, then
Thomas's PRs and the proposal, then the fork's remaining interface items,
then other platforms.

- **The engine mode's notice checked**: in #285 and the fork, it says games are recorded two years, anonymous (as `docs/protocol.md` says), and where to send feedback. Done when both say all three. Docs: sc.md.
- **The referendum and action panels inside the seat's board**: vote totals and "Played during the action" are cut off today. Done when both show whole at the fork's smallest supported size. Docs: none.
- **"Report a problem" in engine games**: the `feedback {text}` command (protocol 1.2.0, live), its text tied to the game and the decision, and the community channel the notice names. Done when a report sent from the fork shows in `/admin/feedback`. Docs: sc.md.
- **Export Replay before the game's end**: from protocol 1.7.0, `GET /games/{id}/record` answers 409 until `over`, and `host.ts` `replay()` returns `res.text()` unchecked. Done when the fork's Export Replay says the record comes at the game's end before it, and saves it after. Docs: none.
- **The fork's log told a card leaving play rightly**: `log.ts` writes every `leaves_play` without `to` as "burned": wrong for `contested` (since protocol 2.3.0 also every card on a contested minion), for `effect: true` (an effect ending) and for a card out of play with its minion (Descent into Darkness); `crates/text` `event.rs` phrases each. Taken with the fork's `api.ts` brought to the server's protocol, its new effect kinds labelled (from 2.3.0: `taint`, `burn_life` show as raw ids meanwhile; the engine board's "Terms told in their unit and for what" adds detail and effect kinds). Done when the fork logs "is contested", "'s effect ends" and "leaves play" for these, as the text crate does. Docs: none.
- **The re-cut caught up**: the fork fixes after `a7515b1` translated into `engine-game` (sc.md recipe), on the owner's go. Done when `engine-game` holds each and sc.md's branch table says so. Docs: sc.md (branches).
- **The demo video**: a storyboard and a fast-track seed (deck pick, glows and staging, an action with a block and a combat, a referendum with all votes one way, the bots' narration, Opportunities, undo, the record), for the owner to record on `engine.krcg.org`. Done when the owner has the storyboard and the seed. Docs: none.
- **The proposal to Thomas**, after the demo video and the engine board's "The repository open to outsiders": the protocol doc, the PR and the video (the PRs are open; the owner told him in person about the AI involvement, 2026-10-14). Done when the owner has the text to send. Docs: sc.md.
- **The target step's prompt, short** (owner, 2026-10-16: yes, "the Toreador to lock" for Toreador Grand Ball; "the minion to put this card on" for Priority Contract), built from the option's effects (`lock` `on: target`, the second's `role`); the first pick's clan from the protocol, or an engine-board line if it does not say it. Done when the fork prompts so. Docs: none.
- **The Opportunities control checked in #285's controls commit**. Done when it is there and works on a local server. Docs: none.
- **The fork's inferences dropped**, each after its engine-board line (ability labels, who uses an ability, a decision's reason, markers and names, an action's start and end: `action_starts` from protocol 2.5.0 for "Played during the action", not `view.action`): the fork reads the protocol's field instead. Done when the fork reads each of these fields and infers none. Docs: sc.md.
- **The remaining option placements checked**, as the 2026-10-11 interface review planned them: `choose_seat` as central buttons, one per Methuselah; searches (`find_card`, `take_from`, `ash_vampire`) by the zone glowing and its view listing the offered cards, a click taking (chips in the central box for a card no stack holds); per-vampire votes (`cast_votes`) in the vote boxes; and SC's own referendum widget kept or hidden in engine games. Done when each is placed or dropped with the owner. Docs: none.
- **Other platforms**: JOL (VTES Online) and Elysium (https://elysium.cards, https://github.com/dtagent2000/VTES-Elysium): how each runs games, what each needs of the API, what plugging the engine in would take. Done when a page per platform says so and the engine board has its asks. Docs: `docs/platforms/<name>.md`, `docs/index.md`.
