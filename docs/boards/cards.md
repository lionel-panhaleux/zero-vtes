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

- [doing: Z-add] **Card readings superseded by Rules Director rulings (owner, 2026-11-11)**: the owner, 2026-11-11: "Ankha's ruling is right and is a Rules Director ruling, it supersedes my rules-qa" (relayed by Z-main from the vtes-advanced-rules session). Corrections, code and tests: 78.28 and 80.44 (taking equipment from another minion is an equip action: Hag's Wrinkles and NRA PAC reach it; LSJ 20060222, ANK 20170326); 58.13's inherent-stealth half (the masked minion's inherent stealth bars Mask of a Thousand Faces, RTR 20041202; the NRA half stands); 59.4 (a hunt under Vampiric Disease: the blood stolen goes to the bank, LSJ 20010809-1); 44.8 (Rötschreck any time in the announce-strike step of that strike, either order, LSJ 19990217); 55.1 (Target Vitals' +2 inherits the strike damage's properties, preventability included, RTR 19970630); 51.5 (Business Pressure gives and takes one pool at a time, RTR 19960530); 27.4 and 33.5 (every card on a minion removed from the game is burned, its owner's included; rulebook "Burn"); 60.21 (after Unleash Hell's Fury's block damages and burns, before-block-resolution modifiers such as Change of Target may still be played, ANK 20220116); 63.2 (Abactor at capacity resolves successfully, its blood hunt called; LSJ 20090411; matching 89.1, 7.2); 74.8 (Tyler McGill's +1 joins his cast even if he already voted). Standing, recorded as confirmed (digest §23 K items, no code): 8.1 and 25.3 (TOM 19951109), 104.3, 88.4, 58.13's NRA half, 18.1 and 42.8 (ANK 20260317 over ANK 20260216), 62.21 and 41.12 (ANK 20180817, LSJ 20100303, LSJ 20100725; G00131, G00132, G00140 the fetched case only). Each item corrected in place ("superseded by <ruling> (owner, 2026-11-11)") in the change with its code. Done when each correction has an engine test through `Engine` and its ledger rows are `test:`-backed (`scripts/rulings_v5.py --check` clean), each standing item has its §23 K entry, and `scripts/digest_index.py --check` is clean. Docs: rules-qa (the items above), rules-digest (their entries, §23 K), data/rulings-v5.csv.
- **Complete Keepers of Tradition and its 2018 reprint**: 166 cards (63 crypt, 103 library). Done when `coverage.py --sets` shows both at 100%. Docs: rules-qa (a new batch), rules-digest, engine.md for a new mechanism, this header's coverage.
- **Complete Lords of the Night**: 172 cards (52 crypt, 120 library). Done when `coverage.py --sets` shows it at 100%. Docs: rules-qa (a new batch), rules-digest, engine.md for a new mechanism, this header's coverage.
- **Complete Third Edition**: 212 cards (74 crypt, 138 library). Done when `coverage.py --sets` shows it at 100%. Docs: rules-qa (a new batch), rules-digest, engine.md for a new mechanism, this header's coverage.
- **Complete Legacies of Blood**: 236 cards (63 crypt, 173 library). Done when `coverage.py --sets` shows it at 100%. Docs: rules-qa (a new batch), rules-digest, engine.md for a new mechanism, this header's coverage.
- **Complete Kindred Most Wanted and Tenth Anniversary**: 269 cards (163 crypt, 106 library; Tenth is mostly vampires). Done when `coverage.py --sets` shows both at 100%. Docs: rules-qa (a new batch), rules-digest, engine.md for a new mechanism, this header's coverage.
- **Complete Black Hand and Camarilla Edition**: 209 cards (97 crypt, 112 library). Done when `coverage.py --sets` shows both at 100%. Docs: rules-qa (a new batch), rules-digest, engine.md for a new mechanism, this header's coverage.
- **Complete Bloodlines and Final Nights**: 234 cards (127 crypt, 107 library). Done when `coverage.py --sets` shows both at 100%. Docs: rules-qa (a new batch), rules-digest, engine.md for a new mechanism, this header's coverage.
- **Complete the 1994–2000 sets and Print on Demand** (Sabbat War, Sabbat, Ancient Hearts, Dark Sovereigns, Vampire: The Eternal Struggle, Jyhad): 208 cards (64 crypt, 144 library; their other cards reprinted above). Done when `coverage.py --sets` shows every one of them at 100%. Docs: rules-qa (a new batch), rules-digest, engine.md for a new mechanism, this header's coverage.
- **Complete the promo-only cards**: 13 library cards, the last of the pool (promos were skipped until the owner's 2026-11-09 "all the cards"). Done when `coverage.py` shows crypt and library at 100% and 51 of 51 sets complete. Docs: rules-qa (a new batch), rules-digest, this header's coverage.

