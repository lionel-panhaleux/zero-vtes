# Cards board

Goal: every card in the pool (roadmap objective B), added in a loop where
the owner rules only on readings likely to be wrong. Skill: `add-cards`.
Docs: `docs/engine.md` (§2–5, the reference by mechanism),
`docs/rules-digest.md`, `docs/rules-qa.md`, `data/rulings-v5.csv`,
`data/window-matrix-reviewed.csv`. Works in a round worktree
(`../zero-vtes-rN`). This board holds the card definitions, their readings
and the ledger; a rulebook mechanism no card carries is on the engine board.

Pool (`scripts/coverage.py`, KRCG `vtes.json` of 2026-06-30; 2 crypt and
18 library cards banned, out of scope, owner 2026-10-14): crypt 731 of
1,783 (41.0%), library 954 of 2,346 (40.7%); the Imbued, Powers and
Convictions all in (round 19); 76 of 109 sets complete. TWDA decks in the
pool 644 of 4,538, kept 166; recent (2025+) 302 of 476 in the pool, kept
102 (round 20's larger-event versions displaced four 2025 ones). Demand
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

- **Open contradictions, the rest**, graded and the red ones asked in a new rules-qa batch: K14 (Anathema's master card stays), K30 (the prisci block rows' `n/a`), K31 (vote floors), K35 (stealing from oneself), K37 (the burn option's clauses), K44 (the Edge stolen after its holder burns it; the owner leans "no effect"), K25 (Dawn Operation's wording, unverified), K41 (withdrawal's VP, with the engine board's "Tournament withdrawal"), K42 (the allies box: aggravated damage, torpor burns), K43 (a merge burns the incoming copy's cards). Done when each K is settled in rules-digest §23 or asked. Docs: rules-digest §23–24, rules-qa.
- **Ledger questions with no reading yet**: the Fury's block against later plays (Change of Target, Mirror Walk, Elder Impersonation, Faceless Night, Elder Intervention, KRCG News Radio, Tension in the Ranks, Crypt's Sons), The Erciyes Fragments (a political card played from them, Delaying Tactics), Nergal at a redirected bleed, I am Legion's cancels, a banished Great Beast. Done when each has a reading, graded, in a rules-qa batch. Docs: rules-qa, rules-digest §24.2.
- **Ledger `blocked:untested` rows to tests** (95: Unleash Hell's Fury 9, The Erciyes Fragments 8, I am Legion 4, Psyche!, Telepathic Tracking, Nergal 3 each, Govern the Unaligned, Disarm and 9 more 2 each, 53 cards 1 each — 44 of them the `n/a` rows whose card has since entered the pool, Ambush's nine among them); Vivian VI's row with no status checked. Done when `grep -c 'blocked:untested' data/rulings-v5.csv` is 0. Docs: the ledger.
- **Ledger `blocked:` gap rows** (17), each its mechanism: a hand card's owner (another's card taken from a stolen Storage Annex, rules-qa 85.27), an active Angelo or New Blood choosing a circle of one (rules-qa 84.5), Dual Form's choice of the copy kept when both go to torpor at once, Lucian, the Perfect's printed text (.44 Magnum), a Fury's block ending the action (Change of Target, Unleash Hell's Fury), Nergal's reduction at resolution (60.7), a banished or crypt-bound Great Beast, Touch of Clarity's wake before the action is declared, the card on The Erciyes Fragments hidden from others. Done when no ledger row is `blocked:`. Docs: the ledger, engine.md per mechanism.
- **Reachable edges, verified first**: `Terms::XBlood` past a surcharge (Terror Frenzy on Lightning Reflexes, one X too many); a title card's terms choosing another Methuselah's vampire while that Methuselah claims the title (the incoming card burned?), and Valerius stealing Prince of a city whose baron his Methuselah controls (the caller's own choice is masked since rules-qa 76.2). Done when each is a test, fixed if wrong, and gone from DESIGN §11.3. Docs: DESIGN §11.3.
- **Reachable edges, mid-play ousts and costs**: Disarm's torpor ousting the active Methuselah inside the play; True Love's Face's pay-to-cancel for a last pool; Life Boon's deferred oust still listed in windows; Meditative Grove's canceled cost missing Regina Blake's announced blood (`settle_canceled`). Done as the above. Docs: DESIGN §11.3.
- **The cheap latent edges fixed now**: `ImmuneToDirectedActions` in `ability_targets`; `Effect::PoolToHostBlood` ousting a non-active seat; a cancelling ability's mark after its terms; directed actions naming several Methuselahs (`ActionCtx::directed_seats`, Shadow Cast). Done when each is fixed with a test or stays in DESIGN §11.3 as waiting for a pool card. Docs: DESIGN §11.3.
- **Tests for cards covered by fuzz or code paths only**: Rapid Change [PRO], Wolf Claws [PRO], Rötschreck's range check, Powerbase: Savannah with Deep Song's target, Saatet-ta's stealth and intercept locks, Carlton Van Wyk's strike and dodge. Done when each has a test and DESIGN §11.3's legacy-additions list is empty. Docs: DESIGN §11.3.
- **Advanced-rules and KRCG-rulings corrections**, listed for `../vtes-advanced-rules` and vtes-rulings (no engine change): K11 (the "during X" template), K12 (§4.3.5 vs §2.4.1), K13 (Thirst), K19 (Annabelle, upstream #11), K28 (a gun's maneuver, §4.3.1 vs §4.10.1), K34 (a dangling reference). Done when the owner has the list to file upstream. Docs: rules-digest §23.
