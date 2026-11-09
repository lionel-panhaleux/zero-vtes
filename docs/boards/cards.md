# Cards board

Goal: every card in the pool (roadmap objective B), added in a loop where
the owner rules only on readings likely to be wrong. Skill: `add-cards`.
Docs: `docs/engine.md` (§2–5, the reference by mechanism),
`docs/rules-digest.md`, `docs/rules-qa.md`, `data/rulings-v5.csv`,
`data/window-matrix-reviewed.csv`. This board holds the card definitions,
their readings and the ledger; a rulebook mechanism no card carries is on
the engine board.

Pool (`scripts/coverage.py`, KRCG `vtes.json` and `twda.json` of 2026-11-08; 2
crypt and 18 library cards banned, out of scope, owner 2026-10-14): crypt 1,055
of 1,820 (58.0%), library 1,240 of 2,372 (52.3%); the Imbued, Powers and
Convictions all in (round 19), Nights of Reckoning complete (round 21),
Anthology and Anarch Unbound complete (round 22, set by set), The Unaligned
complete (round 23), Lost Kindred, Twilight Rebellion and Sword of Caine
complete (round 24), Ebony Kingdom and Blood Shadowed Court complete (round 25),
Gehenna and Anarchs complete (round 26), Secrets of New York and Noites
Brasileiras in, Fifth Edition complete (round 27); 30 of 51 sets complete
(KRCG's v5 sets: every promo in one "Promo" set, 142 of 159). TWDA decks in the
pool 982 of 4,709, kept 198; recent (2025+) 451 of 644 in the pool, kept 102
(the sets' versions displace a few kept decks). Demand (the server's counts to
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

- **Complete Heirs to the Blood and its 2018 reprint**: 178 cards (62 crypt, 116 library). Done when `coverage.py --sets` shows both at 100%. Docs: rules-qa (a new batch), rules-digest, engine.md for a new mechanism, this header's coverage.
- **Complete Keepers of Tradition and its 2018 reprint**: 166 cards (63 crypt, 103 library). Done when `coverage.py --sets` shows both at 100%. Docs: rules-qa (a new batch), rules-digest, engine.md for a new mechanism, this header's coverage.
- **Complete Lords of the Night**: 172 cards (52 crypt, 120 library). Done when `coverage.py --sets` shows it at 100%. Docs: rules-qa (a new batch), rules-digest, engine.md for a new mechanism, this header's coverage.
- **Complete Third Edition**: 212 cards (74 crypt, 138 library). Done when `coverage.py --sets` shows it at 100%. Docs: rules-qa (a new batch), rules-digest, engine.md for a new mechanism, this header's coverage.
- **Complete Legacies of Blood**: 236 cards (63 crypt, 173 library). Done when `coverage.py --sets` shows it at 100%. Docs: rules-qa (a new batch), rules-digest, engine.md for a new mechanism, this header's coverage.
- **Complete Kindred Most Wanted and Tenth Anniversary**: 269 cards (163 crypt, 106 library; Tenth is mostly vampires). Done when `coverage.py --sets` shows both at 100%. Docs: rules-qa (a new batch), rules-digest, engine.md for a new mechanism, this header's coverage.
- **Complete Black Hand and Camarilla Edition**: 209 cards (97 crypt, 112 library). Done when `coverage.py --sets` shows both at 100%. Docs: rules-qa (a new batch), rules-digest, engine.md for a new mechanism, this header's coverage.
- **Complete Bloodlines and Final Nights**: 234 cards (127 crypt, 107 library). Done when `coverage.py --sets` shows both at 100%. Docs: rules-qa (a new batch), rules-digest, engine.md for a new mechanism, this header's coverage.
- **Complete the 1994–2000 sets and Print on Demand** (Sabbat War, Sabbat, Ancient Hearts, Dark Sovereigns, Vampire: The Eternal Struggle, Jyhad): 208 cards (64 crypt, 144 library; their other cards reprinted above). Done when `coverage.py --sets` shows every one of them at 100%. Docs: rules-qa (a new batch), rules-digest, engine.md for a new mechanism, this header's coverage.
- **Complete the promo-only cards**: 13 library cards, the last of the pool (promos were skipped until the owner's 2026-11-09 "all the cards"). Done when `coverage.py` shows crypt and library at 100% and 51 of 51 sets complete. Docs: rules-qa (a new batch), rules-digest, this header's coverage.

