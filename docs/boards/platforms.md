# Platforms board

Goal: the engine played where players already are, first Succubus Club,
with guidance and never an unnatural advantage (roadmap objective C and
the community beta). Skill: `sc-change`. Docs: `docs/platforms/sc.md`
(Thomas's style, branches, PRs, recipes, traps, the boundary leaks),
`docs/protocol.md` (all a platform reads), `docs/human-play.md` (how the
first fork was built; its "Checks" and "Running the fork" are current).
Platforms read the protocol only: a platform inferring what the protocol
could say is an engine-board line.

A shrinking list (CLAUDE.md "Harness"): one line an ask, its done-condition
and the docs it changes; done is deleted; order is priority; the only
state is `[doing: <session>]`; waiting on a person is a dated follow-up.
Order: what the community beta needs from Succubus Club first, then
Thomas's PRs and the proposal, then the fork's remaining interface items,
then other platforms.

- **Ask the owner whether to chase Thomas, 2026-10-21**: #282 (textures), #283 (precon update), #284 (layout) and #285 (the engine game, with its question whether the chips stay in `CardGO.vue`) wait on his review; the beta's way in waits on #285 (owner, 2026-10-14: if he does not move swiftly enough, our own SC on krcg, or a platform made for the engine, always engine games). Done when the owner has answered and sc.md logs his reaction, or the owner picked the other way in. Docs: sc.md (PRs), the roadmap's milestone.
- **Rejoin after a dropped socket**: `host.ts` fails the game on `onclose`; it should join again with `after` = the last `seq` received (the server resends what followed, in order; checked on a local server 2026-10-14), a few tries before it gives up. Done when a game on a local server whose socket is closed under it resumes at its pending decision. Docs: sc.md traps.
- **Resume after a page refresh**: the seat's game id and token kept for the tab (`sessionStorage`), the game joined again on load without `after` (a fresh `joined` and the pending decision), the table projected from that view; the fork's setup must accept a view from mid-game. Done when a refresh mid-game on a local server shows the same table and decision. Docs: sc.md traps.
- **The engine mode's notice checked**: in #285 and the fork, it says games are recorded two years, anonymous (as `docs/protocol.md` says), and where to send feedback. Done when both say all three. Docs: sc.md.
- **The referendum and action panels inside the seat's board**: vote totals and "Played during the action" are cut off today. Done when both show whole at the fork's smallest supported size. Docs: none.
- **"Report a problem" in engine games**, after the engine board's "Feedback intake": text tied to the game and the decision, and the community channel the notice names. Done when a report sent from the fork shows in `/admin/feedback`. Docs: sc.md.
- **The re-cut caught up**: the fork fixes after `a7515b1` translated into `engine-game` (sc.md recipe), on the owner's go. Done when `engine-game` holds each and sc.md's branch table says so. Docs: sc.md (branches).
- **The demo video**: a storyboard and a fast-track seed (deck pick, glows and staging, an action with a block and a combat, a referendum with all votes one way, the bots' narration, Opportunities, undo, the record), for the owner to record on `engine.krcg.org`. Done when the owner has the storyboard and the seed. Docs: none.
- **The proposal to Thomas**, after the demo video and the engine board's "The repository open to outsiders": the protocol doc, the PR and the video (the PRs are open; the owner told him in person about the AI involvement, 2026-10-14). Done when the owner has the text to send. Docs: sc.md.
- **The opposing declared strike shown before one's own** (`CombatView.sides`). Done when the fork shows it. Docs: none.
- **Ask the owner for the go on the target step's prompt, 2026-10-16**, from `ModeView.target` (Toreador Grand Ball, Priority Contract). Done when the owner has answered, and on a yes the fork prompts from it. Docs: none.
- **Ask the owner to look at the texture fix in a browser, 2026-10-16** (Oblivion, 47th Street Royals; #282). Done when the owner has seen it. Docs: sc.md (PRs).
- **The Opportunities control checked in #285's controls commit**. Done when it is there and works on a local server. Docs: none.
- **The fork's inferences dropped**, each after its engine-board line (ability labels, who uses an ability, a decision's reason, markers and names): the fork reads the protocol's field instead. Done when sc.md's boundary list is empty. Docs: sc.md.
- **The remaining option placements checked**, as the 2026-10-11 interface review planned them: `choose_seat` as central buttons, one per Methuselah; searches (`find_card`, `take_from`, `ash_vampire`) by the zone glowing and its view listing the offered cards, a click taking (chips in the central box for a card no stack holds); per-vampire votes (`cast_votes`) in the vote boxes; and SC's own referendum widget kept or hidden in engine games. Done when each is placed or dropped with the owner. Docs: none.
- **Other platforms**: JOL (VTES Online) and Elysium (https://elysium.cards, https://github.com/dtagent2000/VTES-Elysium): how each runs games, what each needs of the API, what plugging the engine in would take. Done when a page per platform says so and the engine board has its asks. Docs: `docs/platforms/<name>.md`, `docs/index.md`.
