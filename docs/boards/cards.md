# Cards board

Goal: every card in the pool (roadmap objective B), added in a loop where
the owner rules only on readings likely to be wrong. Skill: `add-cards`.
Docs: `docs/engine.md` (§2–5, the reference by mechanism),
`docs/rules-digest.md`, `docs/rules-qa.md`, `data/rulings-v5.csv`,
`data/window-matrix-reviewed.csv`. This board holds the card definitions,
their readings and the ledger; a rulebook mechanism no card carries is on
the engine board.

Pool (`scripts/coverage.py`, KRCG `vtes.json` of 2026-06-30; 2 crypt and
18 library cards banned, out of scope, owner 2026-10-14): crypt 1,018 of
1,783 (57.1%), library 1,214 of 2,346 (51.7%); the Imbued, Powers and
Convictions all in (round 19), Nights of Reckoning complete (round 21),
Anthology and Anarch Unbound complete (round 22, set by set), The Unaligned
complete (round 23), Lost Kindred, Twilight Rebellion and Sword of Caine
complete (round 24), Ebony Kingdom and Blood Shadowed Court complete (round
25), Gehenna and Anarchs complete (round 26); 87 of 109 sets complete. TWDA
decks in the pool 878 of 4,538, kept 191; recent (2025+) 347 of 476 in the
pool, kept 90 (the sets' versions displace a few kept decks). Demand
(the server's counts to 2026-10-23): every card asked for in the pool
since round 20; 299999 (asked 12 times) is a client's placeholder, no card.
VEKN's two-player allowed list all in the pool (round 20).

A shrinking list (CLAUDE.md "Harness"; the milestone tokens: roadmap
"Milestones"). Order (owner, 2026-10-15: "you're supposed to add cards"): wrong
behaviour in cards already defined first (beta games meet them), then a round of
cards, then one review line before the next round; the cleanup lines ride with
the rounds that meet their cards (the ship skill's scope rule). Rounds go set by
set, the owner picking the sets; the TWDA analysis no longer chooses the cards,
and promos are skipped (owner, 2026-10-30).

- **Refresh the TWDA tables from the v5 twda.json** [M:recent-decks]: the owner
  (2026-11-08): "we should let the bot play recent (last 6 months) TWDA decks";
  KRCG's root twda.json (v5 schema) holds decks to 2026-10-25, 144 after
  2026-06-13 (5 of the 26 since 2026-05-08 kept today, the rest needing cards
  outside the pool). Regenerate `twda.rs` (`scripts/gen_twda.py`) from it, and
  rename "Path of Death" to "Path of Death and the Soul" (owner, 2026-11-08:
  `SABBAT_NAMES` in `scripts/window_matrix.py` emptied; the harvest maps the old
  name with Z-train before it commits); the fuzz goldens re-blessed (the other
  lines told before, docs/hazards.md "The shared tree"). Done when the catalog
  holds TWDA decks dated after 2026-06-13, the generator, ledger and
  window-matrix `--check`s pass and the goldens are re-blessed. Docs:
  `docs/boards/cards.md` (the header's TWDA counts, and its set count: v5 merges
  every promo into one "Promo" set, so the header says which measure it quotes,
  from `scripts/coverage.py`'s v5 output).
- **Add Secrets of New York and Noites Brasileiras (round 27)** [M:new-sets] (63
  cards, 2026-10-31; owner, 2026-11-08). Done when `OUT_BUNDLES`
  (`scripts/window_matrix.py`) is empty, every card is defined and the add-cards
  checks pass. Docs: the add-cards set (`docs/engine.md`, rules-qa Batch 112,
  `docs/rules-digest.md`, `data/rulings-v5.csv`,
  `data/window-matrix-reviewed.csv`).
