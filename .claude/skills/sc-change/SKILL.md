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

1. **The owner is the contributor.** No PR opened, pushed, commented or
   answered without the owner's word for that PR. Thomas reviews human-made
   contributions only (sc.md): small focused changes, short plain PR text
   and commits in his style, no generated prose.
2. **AGENTS.md is the rulebook**, plus what sc.md records beyond it. Check
   added lines for `GameType.`, `!.`, `!)`, `any`, `border-radius`,
   `box-shadow`, `!important`, emoji, semicolons.
3. **Hooks only in SC files**: engine code in `src/client/engine/` and
   `central/Engine*.vue`, gated by `useEngineMode()` / `useUIFeatures`;
   engine display state in the engine store, never on SC's synced `Card`.
4. **Never edit the served checkout while the owner may be playing** (vite
   reloads, an engine game has no resume): work in a worktree under the
   job's tmp (`git worktree add`, `node_modules` symlinked); fast-forward
   `zero-vtes-engine` only when the admin API shows no live game or the
   owner says so.
5. **Push with explicit refspecs** (`git push fork <b>:refs/heads/<b>`):
   `push.default=upstream` is set globally.
6. **The platform stays agnostic of the engine's internals**: the fork reads
   the protocol (`docs/protocol.md`), never engine code. When it has to
   infer something the protocol could say, record the leak in sc.md's
   boundary list and on the engine board, rather than hard-code it.
7. **Guidance, not advantage** (protocol §8 and the owner's policy there):
   the platform may hide what `opportunities` hides and help a player act,
   never remind them of what the engine leaves to the player.

## Procedure

1. A playtest finding or a request lands on the platforms board
   (`docs/boards/platforms.md`). Diagnose from the server's record when a
   game is involved (sc.md "Recipes").
2. Fix on a worktree of `zero-vtes-engine`; checks: `npx vue-tsc --noEmit`,
   `npx eslint`, `npx prettier --check` (SC has no test suite). Visual
   checks against a LOCAL engine server only (test games on
   engine.krcg.org pollute the recorded corpus); agents do not run vite
   against production.
3. Fast-forward `zero-vtes-engine` (rule 4); tell the owner what to look at.
4. Upstream: fold into the `engine-game` re-cut (sc.md recipe) or a
   carve-out branch from `origin/master`; vue-tsc at every commit; show the
   owner the diff and the PR text before anything leaves the machine.
5. Log every PR's state and every reaction from Thomas in sc.md's PR table.
