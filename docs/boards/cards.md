# Cards board

Goal: every card in the pool (roadmap objective B), added in a loop where
the owner rules only on readings likely to be wrong. Skill: `add-cards`.
Docs: `docs/engine.md` (§2–5, the reference by mechanism),
`docs/rules-digest.md`, `docs/rules-qa.md`, `data/rulings-v5.csv`,
`data/window-matrix-reviewed.csv`. This board holds the card definitions,
their readings and the ledger; a rulebook mechanism no card carries is on
the engine board.

Pool (`scripts/coverage.py`, KRCG `vtes.json` and `twda.json` of 2026-11-09; 2
crypt and 18 library cards banned, out of scope, owner 2026-10-14): crypt 1,117
of 1,820 (61.4%), library 1,356 of 2,372 (57.2%); 32 of 51 sets complete (KRCG's
v5 sets: every promo in one "Promo" set, 142 of 159), Heirs to the Blood and its
reprint the last in (round 28). TWDA decks in the pool 1,050 of 4,709, kept 216;
recent (2025+) 463 of 644 in the pool, kept 101. Demand (the server's counts to
2026-10-23): every card asked for in the pool since round 20; 299999 (asked 12
times) is a client's placeholder, no card. VEKN's two-player allowed list all in
the pool (round 20).

A shrinking list (CLAUDE.md "Harness"; the milestone tokens: roadmap
"Milestones"). Order (owner, 2026-10-15: "you're supposed to add cards"): wrong
behaviour in cards already defined first (beta games meet them), then a round of
cards, then one review line before the next round; the cleanup lines ride with
the rounds that meet their cards (the ship skill's scope rule). Rounds go set by
set, the TWDA analysis no longer choosing the cards (owner, 2026-10-30); the
lines below group the incomplete sets, newest first, a set's reprint with it,
promo-only cards last (owner, 2026-11-09: "We aim to add all the cards
ultimately. Let's work set by set. You can make board lines to work by groups
that make sense"). A line's count is the cards it adds after the lines above it
(an older set is mostly reprinted in a newer one); done when `coverage.py
--sets` shows its sets at 100%.

- **Inert clauses that gained a pool tenant** (Z-main's upkeep docs review at e85742f, 2026-11-11: "not in the pool" clauses gone stale as later rounds added their tenants): 1. Abdelsobek ("No mummy is in the pool", vampires.rs; rules-qa 78.2): Saatet-ta (an ally, Label::Mummy) is in, his target `R::TargetVampire` cannot unlock her; 2. Veneficti ("No demon is in the pool: his action is never offered", defs.rs): Herald of Topheth, Infernal Servitor, Charnas the Imp, Nocturn, Impundulu are in; 3. Winterlich ("No changeling in the pool has requirements"): Iara (Malkavian), Draeven Softfoot (Kiasyd); 4. Tupdog ("No Visceratika card is in the pool"): nine VIS cards, its reduction coded but untested; 5. Louis Fortier and Gengis (rules-qa 104.20, "base cards not in the pool"): Louis Fortier G4 (round 24), Gengis G3 (round 25) are in, no `_MERGED` definition; 6. The Fourth Cycle ("No conviction card in V5", defs.rs): three `CardType::Conviction` cards since round 19; 7. Blood Trade burns "all boons": Major Boon is in, a reading to grade; 8. DESIGN §11.3 "Inert clauses, no pool tenant" lists these, and Brunhilde, Danielle Diron, Claudio Severino, Maldavis now have `_MERGED`. Done when 1–7 are each implemented or graded, each with an engine test, their comments and rules-qa items corrected, and §11.3 lists only what stays inert (Masika, Bulscu's [MERGED] text and whatever 1–7 leave). Docs: DESIGN.md, rules-qa (a new batch), rules-digest.
- **Complete Keepers of Tradition and its 2018 reprint**: 166 cards (63 crypt, 103 library). Done when `coverage.py --sets` shows both at 100%. Docs: rules-qa (a new batch), rules-digest, engine.md for a new mechanism, this header's coverage.
- **Complete Lords of the Night**: 172 cards (52 crypt, 120 library). Done when `coverage.py --sets` shows it at 100%. Docs: rules-qa (a new batch), rules-digest, engine.md for a new mechanism, this header's coverage.
- **Complete Third Edition**: 212 cards (74 crypt, 138 library). Done when `coverage.py --sets` shows it at 100%. Docs: rules-qa (a new batch), rules-digest, engine.md for a new mechanism, this header's coverage.
- **Complete Legacies of Blood**: 236 cards (63 crypt, 173 library). Done when `coverage.py --sets` shows it at 100%. Docs: rules-qa (a new batch), rules-digest, engine.md for a new mechanism, this header's coverage.
- **Complete Kindred Most Wanted and Tenth Anniversary**: 269 cards (163 crypt, 106 library; Tenth is mostly vampires). Done when `coverage.py --sets` shows both at 100%. Docs: rules-qa (a new batch), rules-digest, engine.md for a new mechanism, this header's coverage.
- **Complete Black Hand and Camarilla Edition**: 209 cards (97 crypt, 112 library). Done when `coverage.py --sets` shows both at 100%. Docs: rules-qa (a new batch), rules-digest, engine.md for a new mechanism, this header's coverage.
- **Complete Bloodlines and Final Nights**: 234 cards (127 crypt, 107 library). Done when `coverage.py --sets` shows both at 100%. Docs: rules-qa (a new batch), rules-digest, engine.md for a new mechanism, this header's coverage.
- **Complete the 1994–2000 sets and Print on Demand** (Sabbat War, Sabbat, Ancient Hearts, Dark Sovereigns, Vampire: The Eternal Struggle, Jyhad): 208 cards (64 crypt, 144 library; their other cards reprinted above). Done when `coverage.py --sets` shows every one of them at 100%. Docs: rules-qa (a new batch), rules-digest, engine.md for a new mechanism, this header's coverage.
- **Complete the promo-only cards**: 13 library cards, the last of the pool (promos were skipped until the owner's 2026-11-09 "all the cards"). Done when `coverage.py` shows crypt and library at 100% and 51 of 51 sets complete. Docs: rules-qa (a new batch), rules-digest, this header's coverage.

