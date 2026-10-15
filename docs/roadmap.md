# zero-vtes roadmap

The objectives, how to rank work across lines, the near milestone, and
what the owner has put off. The work itself is on four boards, one per
line of work (CLAUDE.md "Lines of work"): [cards](boards/cards.md),
[engine](boards/engine.md), [training](boards/training.md),
[platforms](boards/platforms.md). Below, "engine: Records complete" names
the engine board's line by its ask.

## Ranking

Each board's header ranks its own lines. Between lines, and for a line
that could go on two boards:

1. What the community beta's players meet, before anything else.
2. A line another line waits on, before the waiting line's own work.
3. An ask of the owner or of a person outside, early: it waits on someone.
4. Then the objectives in order: A (a great opponent) and B (every card)
   are the lines' own; C and the architecture move with the platforms'
   and contributors' demand.

## The first milestone: a community beta

The owner (2026-10-14): a working VTES bot online; bot + Succubus Club is
good enough to beta test with players of the community.

What it still needs:

1. **A way in for testers**: Thomas ships SC with #285 merged, or our own
   SC on krcg, or a platform made for the engine (platforms: Ask the owner
   whether to chase Thomas; the platforms line held until his review,
   owner 2026-10-15).
2. **The best bot served**: p11 finished, measured and served if it beats
   p10 (training: p11 to its end; p11's two questions measured).
3. **Games that survive the server**: a game rebuilt after a restart
   (engine: A game survives a restart).
4. **Complete records** (engine: Records complete).
5. **The loop on testers' games**: the harvest scheduled and its report
   read, promotion to `data/records`, the human win rate as the measure
   (training: The harvest on a schedule; Promotion as a script; The human
   win rate).
6. **Feedback intake**: a "report a problem" from inside the game (the
   protocol's `feedback`, stored with the record at its decision;
   platforms: "Report a problem"), plus one community channel the notice
   names (platforms: The engine mode's notice checked); the `intake`
   skill triages it.
7. **Known rough edges fixed**: the panel overflow (platforms: The
   referendum and action panels); Sniper Rifle and strike-only prevention
   (cards); diablerie's equipment (engine).
8. **What players asked for**: the cards their refused decks named,
   counted (engine: Demand for cards counted), to steer the next rounds
   (cards: Round 13).

Not needed for the beta but in the same order: the repository open to
outsiders (engine), the demo video and the proposal to Thomas (platforms).

## Objectives

### A. A great opponent

The owner: the bot is already convincing; make it a really great
opponent.

Done means: human win rates against the served bot at or below a bar the
owner sets (a table's fair share, 1 in 4 or 5, is the natural one) over a
sample the report marks as large enough; the owner's waste counters
(`waste`) at the owner's own rates; no recurring weakness in the harvest
without a counter; strong on decks it never trained with. Measured on the
"positions where a human beat the bot" set, never trained on.

### B. Every card

The owner: include all the cards, with a seamless integration loop where
the owner almost never intervenes; easily add new cards as they are
released. So a player can use a bot at their table when missing a
player, or play a full bot table to train and test decks.

Scope (owner, 2026-10-14): the Imbued and their cards are in; the banned
cards are out. Size: the cards board's header.

Done means: every card in scope defined with all its text, its ledger
rows `test:` or `n/a:` (no `blocked:`), its readings graded and only red
ones asked (add-cards §4); a new set taken in by the recipe with the owner
answering only red readings; the round choice a script; the coverage
report at 100%; a player's own deck accepted, a bot in a free seat, a full
bot table to test a deck.

### C. The engine every platform uses

The owner: improve the engine–platform integration — the engine so good
every online platform uses it, even without bots; easy to plug in. The
platform must not help players too much: they may forget effects or miss
opportunities; the frontier between good guidance and an unnatural
advantage is thin.

Done means: a platform plugs in from `docs/protocol.md` and the schema
alone, with a runnable minimal client; no platform infers what the
protocol could say (sc.md's leak list empty); all-human engine tables;
the guidance rule written in protocol §8 and every setting and the fork
checked against it; a second platform considered (JOL, Elysium).

### Architecture

The owner: the engine clearly separated, complete and legal, agnostic of
humans and bots; a convenient API, easy for AI agents and human
developers; an open-source definition of the game's interactions,
effects and windows; bots plug in as an option; high performance for the
training loop; a layer between the engine and the network for
training-only optimizations (elision, critic …); platform changes
agnostic of engine and bot; open to external contributions and use.

Done means: the engine crate holds the rules only (observations, habits
and scripted bots in `zvtes-env`), bots behind `host`'s `Bots` with named
profiles; every rulebook mechanism implemented; every card's definition
exported as documented data; every legal play offered (owner,
2026-10-14), a host or the training layer filtering; performance measured
against DESIGN §9; the repository open with a contributor's guide; the
protocol versioned with a changelog and a schema-diff check.

## Deferred by the owner

Put off by the owner (2026-10-14: "a later step", unless dated otherwise),
not on any board; the owner brings each back:

- Guidance vs advantage written down: protocol §8 gains the rule (the host
  may hide what play leaves out of its moment and help a player act; never
  remind of an effect or a trigger the player forgot), every setting
  checked against it, the border cases to the owner; then the fork's
  engine mode audited against it.
- The redaction audit, the multi-human gate: cards named from hand,
  `limbo`, `ActionView.card`, the "waiting on seat X" tell (hide, delay or
  accept).
- Several humans at one table, bots in the free seats, after the audit;
  spectators (public only, or omniscient with a delay); then all-human
  engine tables, and the same in SC.
- Stops and auto-pass preferences per window or card, within the guidance
  rule.
- Training: the early survival shaping of DESIGN §5 (kept, tried or
  dropped) and strength levels players pick (named server profiles), to
  discuss on the training side.
- New-set intake (cards): the recipe for a set KRCG publishes (regenerate,
  list the new ids, window-matrix drafts, a round of them) as a step of
  add-cards §1; off the cards board until KRCG announces a set, its
  done-condition needing a real release (2026-10-15).

## Later tracks

Named, not planned yet:

- **A judge bot**: improve `../vtes-advanced-rules`, the rules docs and
  the skills toward a "judge" bot, tested against players' rules
  questions on Discord and the official judges' answers. First inputs:
  the advanced-rules corrections (cards board) and the exported
  definitions (engine board).
- **Deck evaluation and deck building**: bot simulations to measure a
  deck's strength and weaknesses in a meta, advise, and build strong
  decks (DESIGN "Outlook: deck building": a co-evolved metagame,
  decklist-conditioned pilots, the TWDA as a prior; the precon-tuning
  bridge study first).
- **An MCP server** over these tools, for anyone's agentic AI.
