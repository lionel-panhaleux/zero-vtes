# Cards board

Goal: every card in the pool (roadmap objective B), added in a loop where
the owner rules only on readings likely to be wrong. Skill: `add-cards`.
Docs: `docs/engine.md` (§2–5, the reference by mechanism),
`docs/rules-digest.md`, `docs/rules-qa.md`, `data/rulings-v5.csv`,
`data/window-matrix-reviewed.csv`. Works in a round worktree
(`../zero-vtes-rN`). This board holds the card definitions, their readings
and the ledger; a rulebook mechanism no card carries is on the engine board.

Pool (KRCG `vtes.json` of 2026-06-30): crypt 483 of 1,785 in the engine
(1,765 vampires + 20 Imbued; 297 `VampireDef`s carry the printed text
beyond generated traits), library 739 of 2,364; 2 crypt and 18 library
cards banned. 440 TWDA decks fully in the pool, 114 kept (slice 16w).
C4 makes these numbers a report.

Status: `todo` · `doing` · `blocked:<what>` · `later` · `done`. Order: wrong
behaviour in cards already defined first (beta games meet them), then
what makes the loop run without the owner (B), then the owner-review
backlog a batch at a time, then the long tail.

| # | Item | Why / source | Status |
|---|---|---|---|
| C1 | Round 12 (`round12`, `../zero-vtes-r12`) through add-cards §1–6: the owner's cut, tables regenerated, readings graded, tests, fuzz fixtures, merged on the owner's word; its numbers in the round log below | add-cards; DESIGN §11.3 owner suggestions (more `ADDED_CARDS`, the TWDA unlocks) | doing |
| C2 | Sniper Rifle in the slave's combat: clear `CombatCtx::held_at_block` when Tupdog takes a blocked Tremere antitribu's combat (`start_combat(slave, blocker)`); its ledger row (ANK 20200115, `n/a: no slave rule in V5`, stale since 16n) to a test | findings L14 | todo |
| C3 | Strike-only prevention: `Effect::PreventStrike(n)` / `R::StrikeDamagePending` for Rego Motum, a non-aggravated strike-only variant for Opikun; tests with Carrion Crows' damage beside the strike (blood 3, not 4) and no strike damage (not playable) | findings L15 | todo |
| C4 | Coverage report: a script over `vtes.json` and the generated tables — cards defined vs the full pool by set and type, TWDA decks in the pool and kept, the ids players asked for (E5); run each round, its numbers in this header | objective B; harvest (cards session, 2026-10-14): coverage report | todo |
| C5 | Round choice as a script: add-cards §1's greedy simulation (today a monkeypatched `g.table_ids` each round) printing the steps table (bundle → kept, recent kept, cost per deck), the ceiling check and the demand from E5; the owner only picks the cut | objective B (the owner almost never intervenes); add-cards §1 | todo |
| C6 | New-set intake: when KRCG publishes a set, the recipe (regenerate, list the new ids, window-matrix drafts, a round of them) as a step of add-cards §1, tried on the next release | objective B (new cards as they are released) | todo |
| C7 | Owner-review backlog 1/5, Batch 62 (26 readings: 62.2, 62.3, 62.6–62.12, 62.14–62.20, 62.22–62.29, 62.31, 62.32): graded per add-cards §4 (a blind second reader), green and amber recorded, red asked as scenarios | rules-digest §24.1 | todo |
| C8 | Owner-review backlog 2/5, Batches 64, 65, 67 (16: 64.1–64.6, 64.9–64.11, 65.1–65.3, 67.2, 67.3, 67.5, 67.7), as C7 | rules-digest §24.1 | todo |
| C9 | Owner-review backlog 3/5, Batch 68 (14: 68.3–68.9, 68.11–68.17), as C7 | rules-digest §24.1 | todo |
| C10 | Owner-review backlog 4/5, Batch 69 (14: 69.1–69.11, 69.13–69.15), as C7 | rules-digest §24.1 | todo |
| C11 | Owner-review backlog 5/5, Batch 70 (7: 70.5, 70.9, 70.13, 70.18, 70.20–70.22) and the implemented-question tails 48.2, 48.3, 48.8, as C7 | rules-digest §24.1 | todo |
| C12 | Open contradictions that change play, graded and the red ones asked in a new rules-qa batch: K5 (Yawp Court's after-resolution window), K6 (24.1 under Life Boon's decision point), K7 (an ally's last life as its action's cost), K15 (plays before a diablerie's blood hunt), K16 (Anarch Revolt's condition), K18 (Elen Kamjian at 0 blood), K23 (surplus stealth or intercept), K26 (The Meddling of Semsith ousted), K27 (a locquipment moved with only torpid minions), K29 (Obedience against a queued combat) | rules-digest §23, §24.2 | todo |
| C13 | Open contradictions, the rest, as C12: K14 (Anathema's master card stays), K30 (the prisci block rows' `n/a`), K31 (vote floors), K35 (stealing from oneself), K37 (the burn option's clauses), K44 (the Edge stolen after its holder burns it; the owner leans "no effect"), K25 (Dawn Operation's wording, unverified), K41 (withdrawal's VP, with E27), K42 (the allies box: aggravated damage, torpor burns), K43 (a merge burns the incoming copy's cards) | rules-digest §23, §24.2 | todo |
| C14 | Ledger questions with no reading yet: the Fury's block against later plays (Change of Target, Mirror Walk, Elder Impersonation, Faceless Night, Elder Intervention, KRCG News Radio, Tension in the Ranks, Crypt's Sons), The Erciyes Fragments (a political card played from them, Delaying Tactics), Nergal at a redirected bleed, I am Legion's cancels, a banished Great Beast, Ferraille's two modes each limited | rules-digest §24.2 | todo |
| C15 | Ledger `blocked:untested` rows to tests (44: Unleash Hell's Fury 9, The Erciyes Fragments 7, I am Legion 4, Nergal 3, Street Cred, Mirror Walk, Elder Impersonation, Erebus Mask 2 each, 11 cards 1 each); Vivian VI's row with no status checked | `data/rulings-v5.csv` (54 `blocked:` rows) | todo |
| C16 | Ledger `blocked:` gap rows (10), each its mechanism: Lucian, the Perfect's printed text (Sniper Rifle, .44 Magnum), a Fury's block ending the action (Change of Target, Unleash Hell's Fury), Nergal's reduction at resolution (60.7), a banished or crypt-bound Great Beast, Touch of Clarity's wake before the action is declared, the card on The Erciyes Fragments hidden from others | `data/rulings-v5.csv` | todo |
| C17 | Reachable edges, verified first: `Terms::XBlood` past a surcharge (Terror Frenzy on Lightning Reflexes, one X too many); a unique title's self-contest by terms (Toreador Justicar, Valerius stealing Prince) | DESIGN §11.3 known edges (both "(verify)") | todo |
| C18 | Reachable edges, mid-play ousts and costs: Disarm's torpor ousting the active Methuselah inside the play; True Love's Face's pay-to-cancel for a last pool; Life Boon's deferred oust still listed in windows; Meditative Grove's canceled cost missing Regina Blake's announced blood (`settle_canceled`) | DESIGN §11.3 known edges, deferred oust | todo |
| C19 | Latent edges no pool card reaches yet (fixed when one does, or now if cheap): `ImmuneToDirectedActions` in `ability_targets`; `Effect::PoolToHostBlood` ousting a non-active seat; a cancelling ability's mark after its terms; directed actions naming several Methuselahs (`ActionCtx::directed_seats`, Shadow Cast) | DESIGN §11.3 known edges | todo |
| C20 | Tests for cards covered by fuzz or code paths only: Rapid Change [PRO], Wolf Claws [PRO], Rötschreck's range check, Powerbase: Savannah with Deep Song's target, Saatet-ta's stealth and intercept locks, Carlton Van Wyk's strike and dodge | DESIGN §11.3 known edges, legacy additions | todo |
| C21 | Hard cards (owner): Outside the Hourglass and Rewind Time with Shalmath (the next step of the TWDA ranking; the Lasombra and Ravnos G2 vampires of that note came in with round 9, 16u); a sterile baron to close a test loop | DESIGN §11.3 owner suggestions; CLAUDE.md owner rules | todo |
| C22 | Inert clauses, live once a round adds their tenant: Blood Trade's boons, The Fourth Cycle's conviction cards, self-damaging weapons, Black Forest Base's changeling, Brunhilde's and Danielle Diron's [MERGED] text, Veneficti's demon, Tupdog's Visceratika; My Kin Against the World's Laibon clause checked now (Laibon in the pool since 16s) | DESIGN §11.3 inert clauses | blocked:no pool tenant |
| C23 | The Imbued in "every card" (owner, 2026-10-14: the Imbued yes, the banned cards no): the 20 Imbued and their library (14 Powers, 3 Convictions), a round with the mechanism (conviction, the Imbued's actions) | objective B; pool count | todo |
| C24 | Advanced-rules and KRCG-rulings corrections, listed for `../vtes-advanced-rules` and vtes-rulings (no engine change): K11 (the "during X" template), K12 (§4.3.5 vs §2.4.1), K13 (Thirst), K19 (Annabelle, upstream #11), K28 (a gun's maneuver, §4.3.1 vs §4.10.1), K34 (a dangling reference) | rules-digest §23; the judge track (roadmap) | todo |

## Round log

add-cards §7: one row a round.

| Round | Cards | Readings | Red asked | Corrected | Missed by grading |
|---|---|---|---|---|---|
| 12 | | | | | |
