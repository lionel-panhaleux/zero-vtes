# Cards board

Goal: every card in the pool (roadmap objective B), added in a loop where
the owner rules only on readings likely to be wrong. Skill: `add-cards`.
Docs: `docs/engine.md` (§2–5, the reference by mechanism),
`docs/rules-digest.md`, `docs/rules-qa.md`, `data/rulings-v5.csv`,
`data/window-matrix-reviewed.csv`. Works in a round worktree
(`../zero-vtes-rN`). This board holds the card definitions, their readings
and the ledger; a rulebook mechanism no card carries is on the engine board.

Pool (`scripts/coverage.py`, KRCG `vtes.json` of 2026-06-30; 2 crypt and
18 library cards banned, out of scope, owner 2026-10-14): crypt 626 of
1,783 (35.1%), library 828 of 2,346 (35.3%); the Imbued, Powers and
Convictions 0; 49 of 109 sets complete. TWDA decks in the pool 542 of
4,538, kept 145; recent (2025+) 265 of 476 in the pool, kept 94. Demand:
none counted yet (counting starts with the 1.8.0 deploy; the file from
Z-main or the owner, add-cards §1).

A shrinking list (CLAUDE.md "Harness"): one line an ask, its done-condition
and the docs it changes; done is deleted; order is priority; the only
state is `[doing: <session>]`. Order (owner, 2026-10-15: "you're supposed
to add cards"): wrong behaviour in cards already defined first (beta games
meet them), then a round of cards, then one review line before the next
round; the cleanup lines ride with the rounds that meet their cards (the
ship skill's scope rule). Rounds go set by set when the decks' metric
stops paying (owner, 2026-10-15: "fine with switching to a set-by-set mode
when you feel it's best").

- **Owner-review backlog 1/5, Batch 62** (26 readings: 62.2, 62.3, 62.6–62.12, 62.14–62.20, 62.22–62.29, 62.31, 62.32): graded per add-cards §4 (a blind second reader), green and amber recorded, red asked as scenarios. Done when each reading is stated settled in rules-qa or asked. Docs: rules-qa, rules-digest §24.1.
- **Round 16**: the owner's cut from add-cards §1 (`scripts/round_choice.py`: the sets left closest to complete, and the TWDA greedy's next steps from recent kept 94), then §2–§6. Done when the round is on `main`. Docs: as a round (DESIGN slice row and count line, engine.md, rules-digest, a rules-qa batch, the ledger, the window matrix, the board header).
- **The Imbued** (owner, 2026-10-14: in "every card"): the 20 Imbued and their library (14 Powers, 3 Convictions), a round with the mechanism (conviction, the Imbued's actions). Done when the round is on `main`. Docs: engine.md (a new mechanism), as a round.
- **Hard cards** (owner): a sterile baron to close a test loop (Outside the Hourglass and Shalmath in since round 15, Rewind Time since round 14). Done when they are in a round on `main`. Docs: as a round.
- **Owner-review backlog 2/5, Batches 64, 65, 67** (16: 64.1–64.6, 64.9–64.11, 65.1–65.3, 67.2, 67.3, 67.5, 67.7), as 1/5. Done and docs as 1/5.
- **Owner-review backlog 3/5, Batch 68** (14: 68.3–68.9, 68.11–68.17), as 1/5. Done and docs as 1/5.
- **Owner-review backlog 4/5, Batch 69** (14: 69.1–69.11, 69.13–69.15), as 1/5. Done and docs as 1/5.
- **Owner-review backlog 5/5, Batch 70** (7: 70.5, 70.9, 70.13, 70.18, 70.20–70.22) and the implemented-question tails 48.2, 48.3, 48.8, as 1/5. Done and docs as 1/5.
- **Open contradictions that change play**, graded and the red ones asked in a new rules-qa batch: K5 (Yawp Court's after-resolution window), K6 (24.1 under Life Boon's decision point), K7 (an ally's last life as its action's cost), K15 (plays before a diablerie's blood hunt), K16 (Anarch Revolt's condition), K18 (Elen Kamjian at 0 blood), K23 (surplus stealth or intercept), K26 (The Meddling of Semsith ousted), K27 (a locquipment moved with only torpid minions), K29 (Obedience against a queued combat). Done when each K is settled in rules-digest §23 or asked. Docs: rules-digest §23–24, rules-qa.
- **Open contradictions, the rest**, as the above: K14 (Anathema's master card stays), K30 (the prisci block rows' `n/a`), K31 (vote floors), K35 (stealing from oneself), K37 (the burn option's clauses), K44 (the Edge stolen after its holder burns it; the owner leans "no effect"), K25 (Dawn Operation's wording, unverified), K41 (withdrawal's VP, with the engine board's "Tournament withdrawal"), K42 (the allies box: aggravated damage, torpor burns), K43 (a merge burns the incoming copy's cards). Done and docs as the above.
- **Ledger questions with no reading yet**: the Fury's block against later plays (Change of Target, Mirror Walk, Elder Impersonation, Faceless Night, Elder Intervention, KRCG News Radio, Tension in the Ranks, Crypt's Sons), The Erciyes Fragments (a political card played from them, Delaying Tactics), Nergal at a redirected bleed, I am Legion's cancels, a banished Great Beast, Ferraille's two modes each limited. Done when each has a reading, graded, in a rules-qa batch. Docs: rules-qa, rules-digest §24.2.
- **Ledger `blocked:untested` rows to tests** (46: Unleash Hell's Fury 9, The Erciyes Fragments 7, I am Legion 4, Nergal 3, Mirror Walk, Elder Impersonation, Erebus Mask 2 each, 17 cards 1 each, Taking the Skin: Minion's waiting on K15); Vivian VI's row with no status checked. Done when `grep -c 'blocked:untested' data/rulings-v5.csv` is 0. Docs: the ledger.
- **Ledger `blocked:` gap rows** (12), each its mechanism: Dual Form's choice of the copy kept when both go to torpor at once, Lucian, the Perfect's printed text (Sniper Rifle, .44 Magnum), a Fury's block ending the action (Change of Target, Unleash Hell's Fury), Nergal's reduction at resolution (60.7), a banished or crypt-bound Great Beast, Touch of Clarity's wake before the action is declared, the card on The Erciyes Fragments hidden from others. Done when no ledger row is `blocked:`. Docs: the ledger, engine.md per mechanism.
- **Reachable edges, verified first**: `Terms::XBlood` past a surcharge (Terror Frenzy on Lightning Reflexes, one X too many); a title card's terms choosing another Methuselah's vampire while that Methuselah claims the title (the incoming card burned?), and Valerius stealing Prince of a city whose baron his Methuselah controls (the caller's own choice is masked since rules-qa 76.2). Done when each is a test, fixed if wrong, and gone from DESIGN §11.3. Docs: DESIGN §11.3.
- **Reachable edges, mid-play ousts and costs**: Disarm's torpor ousting the active Methuselah inside the play; True Love's Face's pay-to-cancel for a last pool; Life Boon's deferred oust still listed in windows; Meditative Grove's canceled cost missing Regina Blake's announced blood (`settle_canceled`). Done as the above. Docs: DESIGN §11.3.
- **The cheap latent edges fixed now**: `ImmuneToDirectedActions` in `ability_targets`; `Effect::PoolToHostBlood` ousting a non-active seat; a cancelling ability's mark after its terms; directed actions naming several Methuselahs (`ActionCtx::directed_seats`, Shadow Cast). Done when each is fixed with a test or stays in DESIGN §11.3 as waiting for a pool card. Docs: DESIGN §11.3.
- **Tests for cards covered by fuzz or code paths only**: Rapid Change [PRO], Wolf Claws [PRO], Rötschreck's range check, Powerbase: Savannah with Deep Song's target, Saatet-ta's stealth and intercept locks, Carlton Van Wyk's strike and dodge. Done when each has a test and DESIGN §11.3's legacy-additions list is empty. Docs: DESIGN §11.3.
- **Advanced-rules and KRCG-rulings corrections**, listed for `../vtes-advanced-rules` and vtes-rulings (no engine change): K11 (the "during X" template), K12 (§4.3.5 vs §2.4.1), K13 (Thirst), K19 (Annabelle, upstream #11), K28 (a gun's maneuver, §4.3.1 vs §4.10.1), K34 (a dangling reference). Done when the owner has the list to file upstream. Docs: rules-digest §23.
