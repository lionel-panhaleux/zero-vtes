---
name: intake
description: >
  The ingress loop of zero-vtes: challenge an incoming request, idea or
  discovery before it reaches a board (docs/boards/*.md), then place it as
  one line with a done-condition and its doc-impact, or fix it now, or
  write it into a wiki page instead. Use for anything met on the way (an
  engine bug, a choice offered twice, a rough edge, a playtest note, a
  harvest report's weakness, feedback, a peer's request, an upkeep
  proposal) and when the owner asks for new work.
---

# Intake

No work reaches a board unchallenged, and nothing found is left
untracked: an issue met on the way is fixed on the spot when small, or
becomes a board line here (owner, 2026-10-13). A board is a shrinking
list; refusing a line at the door is cheaper than evicting it later.

## For each candidate

1. **Small and on the way?** Fix it now in the current unit of work (it
   shares the abstraction), with its test. Not a line.
2. **Conflict.** Does it contradict a standing decision (DESIGN.md's
   resolved questions and §10, rules-qa, CLAUDE.md's owner rules,
   `docs/protocol.md`) or duplicate a line (`grep -n` the four boards and
   the roadmap's "Deferred by the owner")? Say so. Changing a decision is
   valid work: the line names the page it edits, and a ruling or an owner
   rule changes only on the owner's word. A duplicate merges into the
   existing line.
3. **Completability.** State "done when …", machine-checkable where
   possible (a test passes, a `--check` is clean, a grep is empty, a field
   is in the schema). No done-condition means it is knowledge: write it
   into the page it belongs to (`docs/index.md` lists them) and stop. A
   standing rule ("factored only when several more come") is a page's
   sentence, not a line.
4. **Scope.** One line per piece that reads alone, split on real
   boundaries (the line that does it, the crate, the contract) so an agent
   holds each in context; never split to defer. Work for another line is
   that line's board, ordered after what it waits on ("after the engine
   board's 'Feedback intake'").
5. **Doc-impact.** Name the pages the work will change, or "none" when
   nothing standing changes.
6. **Waiting is a date.** Work waiting on a person is a dated ask ("Ask
   the owner whether to chase Thomas, 2026-10-21"); never a waiting state.
   Chasing someone outside is outward-facing: the line asks the owner.
7. **Place it** on the board of the line that does it (CLAUDE.md "Lines
   of work"; the harness itself is the engine board's), at the position
   the board's "Order:" and the roadmap's "Ranking" give it, and say so in
   the turn's report so the owner sees what came in. A session writes its
   own board only: a line for another board goes as a message to that
   line's session, one crossing lines to Z-main (CLAUDE.md "Sessions").

## The line

```
- **<the ask, a verb phrase>**[ [doing: <session>]]: <what and why, the
  owner's words when they are the source, where the context is>. Done when
  <condition>. Docs: <pages | none>.
```

One line however long; the bold ask is how other lines and the roadmap
refer to it (no ids). No status, no "later", no done section: the only
state is `[doing: <session>]`, done is deleted. Context too long for a
line belongs in a page the line points at, or in the task's own notes
(the session's job tmp), which die with the task.

## Sources

The owner's asks; discoveries during a task; peers' messages; playtests
and the server's records; the harvest report (training skill); the
players' feedback (`GET https://engine.krcg.org/admin/feedback?since=`
with the admin token from `~/.config/zvtes/admin_token`, never printed:
each report's game, engine and decision, its record replayed with
`--example replay` at that decision); the egress reviewer's out-of-scope
notes; the upkeep's lint and ratchet proposals. Each goes through the
steps above; a source's raw notes are not kept once triaged.
