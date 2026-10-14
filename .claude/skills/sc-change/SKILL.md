---
name: sc-change
description: >
  Change the Succubus Club (SC) fork or prepare a contribution to Thomas
  Wajs's upstream SC: playtest fixes on the owner's play branch, re-cuts of
  the upstream branch, small carve-out PRs, answers to reviews. Use for any
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
   `central/Engine*.vue`, gated by `useEngineMode()` / `useUIFeatures`;
   engine display state in the engine store, never on SC's synced `Card`.
4. **Never edit the served checkout while the owner may be playing** (vite
   reloads, an engine game has no resume): work in a worktree under the
   job's tmp (`git worktree add`, `node_modules` symlinked); fast-forward
   `zero-vtes-engine` only when no game is live (check with
   `scripts/fetch_games.py`, which reads the admin token itself; never read
   the token) or the owner says so; when in doubt, ask the owner.
5. **Push with explicit refspecs** (`git push fork <b>:refs/heads/<b>`):
   `push.default=upstream` is set globally.
6. **The platform stays agnostic of the engine's internals**: the fork reads
   the protocol (`docs/protocol.md`), never engine code. When it has to
   infer something the protocol could say, record the leak in sc.md's
   boundary list and on the engine board, rather than hard-code it.
7. **Guidance, not advantage** (protocol §8 and the owner's policy there):
   the platform may hide what `opportunities` hides and help a player act,
   never remind them of what the engine leaves to the player. Explaining
   why options are missing (the must-hunt notice) is not a reminder.

## Procedure

1. A playtest finding or a request lands on the platforms board
   (`docs/boards/platforms.md`). Diagnose from the server's record when a
   game is involved (sc.md "Recipes").
2. Fix on a worktree of `zero-vtes-engine`; checks: `npx vue-tsc --noEmit`,
   `npx eslint`, `npx prettier --check` (SC has no test suite). Agents
   don't start vite as a check (AGENTS.md); the owner's own dev server on
   :5188 is the owner's loop, restarted only on their word. A visual check,
   when the owner asks for one, runs against a LOCAL engine server (test
   games on engine.krcg.org pollute the recorded corpus).
3. Fast-forward `zero-vtes-engine` (rule 4); tell the owner what to look at.
4. Upstream: fold into the `engine-game` re-cut (sc.md recipe) or a
   carve-out branch from `origin/master`; vue-tsc at every commit; show the
   owner the diff and the PR text before anything leaves the machine.
5. Log every PR's state and every reaction from Thomas in sc.md's PR table.
