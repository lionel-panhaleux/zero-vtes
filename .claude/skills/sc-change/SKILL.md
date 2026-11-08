---
name: sc-change
description: >
  Change the Succubus Club (SC) fork, served on its own at sc-bot.krcg.org
  (bots only; contributions to Thomas Wajs's upstream SC deferred, owner
  2026-10-21): playtest fixes, its builds, answers to reviews. Use for any
  work in ../succubus-club, any SC pull request, or any question about how
  SC is structured or how Thomas works.
---

# Changing Succubus Club

SC is Thomas's project, not ours. Every change respects his time, his
architecture and his rules; the fork never leaks into upstream unasked.
Read `docs/platforms/sc.md` first (his style, the integration's shape,
branches, PRs, recipes, traps) and update it as you learn.

## Rules

1. **The owner stands behind every contribution, and AI involvement stays
   disclosed.** Thomas reviews human-made contributions (sc.md): what goes
   upstream is small, focused, reviewed by the owner, its text written or
   approved by the owner, short and plain, commits in his style. The
   `Co-Authored-By` trailers stay: never strip attribution to pass as
   human-made. No PR opened, pushed, commented or answered without the
   owner's word for that PR.
2. **AGENTS.md is the rulebook**, plus what sc.md records beyond it. Check
   added lines for `GameType.`, `!.`, `!)`, `any`, `border-radius`,
   `box-shadow`, `!important` (avoided), emoji, semicolons.
3. **Hooks only in SC files**: engine code in `src/client/engine/` and
   `Engine*.vue` components, in `central/` or beside the SC components they
   belong with (sc.md "The integration's shape"); gated by
   `useEngineMode()` / `useUIFeatures` in the re-cut (the fork tests
   `GameType.Engine` inline); engine display state in the engine store,
   never on SC's synced `Card`.
4. **Trunk-based on `zero-vtes-engine`** (owner, 2026-11-08: no worktree
   by default): work and commit in the fork's checkout on
   `zero-vtes-engine`; no local server serves it (the owner plays on
   sc-bot.krcg.org), so an edit loses nobody's game. A redeploy waits for
   no game live (Z-main checks it, `scripts/fetch_games.py`).
5. **Push with explicit refspecs** (`git push fork <b>:refs/heads/<b>`):
   `push.default=upstream` is set globally.
6. **The platform stays agnostic of the engine's internals**: the fork reads
   the protocol (`docs/protocol.md`), never engine code. When it has to
   infer something the protocol could say, send the leak to Z-engine
   for an engine-board line (CLAUDE.md "Between lines"), rather than
   hard-code it.
7. **Guidance, not advantage** (the owner's policy; protocol §8 is to
   gain it, roadmap "Deferred by the owner"):
   the platform may hide what `opportunities` hides and help a player act,
   never remind them of what the engine leaves to the player. Explaining
   why options are missing (the must-hunt notice) is not a reminder.

## Procedure

1. A playtest finding or a request goes through `intake` onto the
   platforms board (`docs/boards/platforms.md`), unless the change at
   hand takes it (a finding on what is being shipped is fixed in it). Diagnose from the server's record when a
   game is involved (sc.md "Recipes").
2. Fix on `zero-vtes-engine` (rule 4); checks: `npx vue-tsc --noEmit`,
   `npx eslint`, `npx prettier --check` (SC has no test suite). Agents
   don't start vite as a check (AGENTS.md); the owner plays on
   sc-bot.krcg.org (sc.md "Where the owner plays"). A visual check, when
   the owner asks for one, runs against a LOCAL engine server (test games
   on engine.krcg.org pollute the recorded corpus).
3. Commit; push `zero-vtes-engine` (rule 5) and ask Z-main for the
   sc-bot redeploy at that commit; tell the
   owner what to look at once it is live.
4. Upstream (deferred by the owner, 2026-10-21: roadmap "Deferred by
   the owner"; only once it is brought back): fold into the
   `engine-game` re-cut (sc.md recipe) or a carve-out branch from
   `origin/dev` (every PR targets `dev`); vue-tsc at every commit; show the owner the diff and the PR text before
   anything leaves the machine.
5. Keep sc.md's PR table at the open PRs' current state; what a reaction
   from Thomas teaches goes into its "How he reviews".
