# Cards board

Goal: every card in the pool (roadmap objective B), added in a loop where
the owner rules only on readings likely to be wrong. Skill: `add-cards`.
Docs: `docs/engine.md` (§2–5, the reference by mechanism),
`docs/rules-digest.md`, `docs/rules-qa.md`, `data/rulings-v5.csv`,
`data/window-matrix-reviewed.csv`. Works in a round worktree
(`../zero-vtes-rN`). This board holds the card definitions, their readings
and the ledger; a rulebook mechanism no card carries is on the engine board.

Pool (`scripts/coverage.py`, KRCG `vtes.json` of 2026-06-30; 2 crypt and
18 library cards banned, out of scope, owner 2026-10-14): crypt 785 of
1,783 (44.0%), library 997 of 2,346 (42.5%); the Imbued, Powers and
Convictions all in (round 19), Nights of Reckoning complete (round 21),
Anthology and Anarch Unbound complete (round 22, set by set), The Unaligned
complete (round 23); 80 of 109 sets complete. TWDA decks in the pool 713
of 4,538, kept 169; recent (2025+) 326 of 476 in the pool, kept 101 (two
sets' versions displace a few kept decks). Demand
(the server's counts to 2026-10-23): every card asked for in the pool
since round 20; 299999 (asked 12 times) is a client's placeholder, no card.
VEKN's two-player allowed list all in the pool (round 20).

A shrinking list (CLAUDE.md "Harness"): one line an ask, its done-condition
and the docs it changes; done is deleted; order is priority; the only
state is `[doing: <session>]`. Order (owner, 2026-10-15: "you're supposed
to add cards"): wrong behaviour in cards already defined first (beta games
meet them), then a round of cards, then one review line before the next
round; the cleanup lines ride with the rounds that meet their cards (the
ship skill's scope rule). Rounds go set by set when the decks' metric
stops paying (owner, 2026-10-15: "fine with switching to a set-by-set mode
when you feel it's best").

