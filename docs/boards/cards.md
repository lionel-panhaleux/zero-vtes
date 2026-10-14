# Cards board

Goal: every card in the pool (roadmap objective B), added in a loop where
the owner rules only on readings likely to be wrong. Skill: `add-cards`.
Docs: `docs/engine.md` (§2–5, the reference by mechanism),
`docs/rules-digest.md`, `docs/rules-qa.md`, `data/rulings-v5.csv`,
`data/window-matrix-reviewed.csv`. Works in a round worktree
(`../zero-vtes-rN`). This board holds the card definitions, their readings
and the ledger; a rulebook mechanism no card carries is on the engine board.

Pool (KRCG `vtes.json` of 2026-06-30): crypt 483 of 1,785 in the engine
(1,765 vampires + 20 Imbued), library 739 of 2,364; 2 crypt and 18 library
cards banned and out of scope (owner, 2026-10-14). The coverage report
line makes these numbers a script's.

A shrinking list (CLAUDE.md "Harness"): one line an ask, its done-condition
and the docs it changes; done is deleted; order is priority; the only
state is `[doing: <session>]`. Order: wrong behaviour in cards already
defined first (beta games meet them), then what makes the loop run without
the owner (objective B), then the owner-review backlog a batch at a time,
then the long tail.

- **Round 12** [doing: Z-add]: `round12` in `../zero-vtes-r12` through add-cards §1–6 (the owner's cut, tables regenerated, readings graded, tests, fuzz fixtures), rebased on the engine board's "Every legal block attempt offered" and merged on the owner's word. Done when it is on `main` with its `Grading:` trailer. Docs: rules-qa Batch 74, rules-digest, DESIGN §11 (slice row, count line), engine.md for a new mechanism.
- **Sniper Rifle in the slave's combat**: clear `CombatCtx::held_at_block` when Tupdog takes a blocked Tremere antitribu's combat (`start_combat(slave, blocker)`). Done when a test plays it, and the ledger row (ANK 20200115, `n/a: no slave rule in V5`, stale since 16n) is `test:`. Docs: the ledger.
- **Strike-only prevention**: `Effect::PreventStrike(n)` / `R::StrikeDamagePending` for Rego Motum, a non-aggravated strike-only variant for Opikun; today both prevent non-strike damage. Done when tests show Carrion Crows' damage beside the strike unprevented (blood 3, not 4) and the card unplayable with no strike damage. Docs: engine.md (prevention), rules-digest if a reading moves.
- **Coverage report**: a script over `vtes.json` and the generated tables printing cards defined vs the full pool by set and type, TWDA decks in the pool and kept, and the ids players asked for (the engine board's "Demand for cards counted"). Done when add-cards §1 runs it and this header quotes its output. Docs: add-cards §1, this header.
- **Round choice as a script**: add-cards §1's greedy simulation (today a monkeypatched `g.table_ids` each round) printing the steps table (bundle → kept, recent kept, cost per deck), the ceiling check and the players' demand; the owner only picks the cut. Done when add-cards §1 calls the script instead of describing the patch. Docs: add-cards §1.
- **New-set intake**: when KRCG publishes a set, the recipe (regenerate, list the new ids, window-matrix drafts, a round of them) as a step of add-cards §1. Done when the step is in add-cards §1 and has been tried on a release. Docs: add-cards §1.
- **Owner-review backlog 1/5, Batch 62** (26 readings: 62.2, 62.3, 62.6–62.12, 62.14–62.20, 62.22–62.29, 62.31, 62.32): graded per add-cards §4 (a blind second reader), green and amber recorded, red asked as scenarios. Done when each reading is stated settled in rules-qa or asked. Docs: rules-qa, rules-digest §24.1.
- **Owner-review backlog 2/5, Batches 64, 65, 67** (16: 64.1–64.6, 64.9–64.11, 65.1–65.3, 67.2, 67.3, 67.5, 67.7), as 1/5. Done and docs as 1/5.
- **Owner-review backlog 3/5, Batch 68** (14: 68.3–68.9, 68.11–68.17), as 1/5. Done and docs as 1/5.
- **Owner-review backlog 4/5, Batch 69** (14: 69.1–69.11, 69.13–69.15), as 1/5. Done and docs as 1/5.
- **Owner-review backlog 5/5, Batch 70** (7: 70.5, 70.9, 70.13, 70.18, 70.20–70.22) and the implemented-question tails 48.2, 48.3, 48.8, as 1/5. Done and docs as 1/5.
- **Open contradictions that change play**, graded and the red ones asked in a new rules-qa batch: K5 (Yawp Court's after-resolution window), K6 (24.1 under Life Boon's decision point), K7 (an ally's last life as its action's cost), K15 (plays before a diablerie's blood hunt), K16 (Anarch Revolt's condition), K18 (Elen Kamjian at 0 blood), K23 (surplus stealth or intercept), K26 (The Meddling of Semsith ousted), K27 (a locquipment moved with only torpid minions), K29 (Obedience against a queued combat). Done when each K is settled in rules-digest §23 or asked. Docs: rules-digest §23–24, rules-qa.
- **Open contradictions, the rest**, as the above: K14 (Anathema's master card stays), K30 (the prisci block rows' `n/a`), K31 (vote floors), K35 (stealing from oneself), K37 (the burn option's clauses), K44 (the Edge stolen after its holder burns it; the owner leans "no effect"), K25 (Dawn Operation's wording, unverified), K41 (withdrawal's VP, with the engine board's "Tournament withdrawal"), K42 (the allies box: aggravated damage, torpor burns), K43 (a merge burns the incoming copy's cards). Done and docs as the above.
- **Ledger questions with no reading yet**: the Fury's block against later plays (Change of Target, Mirror Walk, Elder Impersonation, Faceless Night, Elder Intervention, KRCG News Radio, Tension in the Ranks, Crypt's Sons), The Erciyes Fragments (a political card played from them, Delaying Tactics), Nergal at a redirected bleed, I am Legion's cancels, a banished Great Beast, Ferraille's two modes each limited. Done when each has a reading, graded, in a rules-qa batch. Docs: rules-qa, rules-digest §24.2.
- **Ledger `blocked:untested` rows to tests** (44: Unleash Hell's Fury 9, The Erciyes Fragments 7, I am Legion 4, Nergal 3, Street Cred, Mirror Walk, Elder Impersonation, Erebus Mask 2 each, 11 cards 1 each); Vivian VI's row with no status checked. Done when `grep -c 'blocked:untested' data/rulings-v5.csv` is 0. Docs: the ledger.
- **Ledger `blocked:` gap rows** (10), each its mechanism: Lucian, the Perfect's printed text (Sniper Rifle, .44 Magnum), a Fury's block ending the action (Change of Target, Unleash Hell's Fury), Nergal's reduction at resolution (60.7), a banished or crypt-bound Great Beast, Touch of Clarity's wake before the action is declared, the card on The Erciyes Fragments hidden from others. Done when no ledger row is `blocked:`. Docs: the ledger, engine.md per mechanism.
- **Reachable edges, verified first**: `Terms::XBlood` past a surcharge (Terror Frenzy on Lightning Reflexes, one X too many); a unique title's self-contest by terms (Toreador Justicar, Valerius stealing Prince). Done when each is a test, fixed if wrong, and gone from DESIGN §11.3. Docs: DESIGN §11.3.
- **Reachable edges, mid-play ousts and costs**: Disarm's torpor ousting the active Methuselah inside the play; True Love's Face's pay-to-cancel for a last pool; Life Boon's deferred oust still listed in windows; Meditative Grove's canceled cost missing Regina Blake's announced blood (`settle_canceled`). Done as the above. Docs: DESIGN §11.3.
- **The cheap latent edges fixed now**: `ImmuneToDirectedActions` in `ability_targets`; `Effect::PoolToHostBlood` ousting a non-active seat; a cancelling ability's mark after its terms; directed actions naming several Methuselahs (`ActionCtx::directed_seats`, Shadow Cast). Done when each is fixed with a test or stays in DESIGN §11.3 as waiting for a pool card. Docs: DESIGN §11.3.
- **Tests for cards covered by fuzz or code paths only**: Rapid Change [PRO], Wolf Claws [PRO], Rötschreck's range check, Powerbase: Savannah with Deep Song's target, Saatet-ta's stealth and intercept locks, Carlton Van Wyk's strike and dodge. Done when each has a test and DESIGN §11.3's legacy-additions list is empty. Docs: DESIGN §11.3.
- **My Kin Against the World's Laibon clause checked**, Laibon being in the pool since 16s. Done when a test plays the clause. Docs: DESIGN §11.3 (inert clauses).
- **Hard cards** (owner): Outside the Hourglass and Rewind Time with Shalmath; a sterile baron to close a test loop. Done when they are in a round on `main`. Docs: as a round.
- **The Imbued** (owner, 2026-10-14: in "every card"): the 20 Imbued and their library (14 Powers, 3 Convictions), a round with the mechanism (conviction, the Imbued's actions). Done when the round is on `main`. Docs: engine.md (a new mechanism), as a round.
- **Advanced-rules and KRCG-rulings corrections**, listed for `../vtes-advanced-rules` and vtes-rulings (no engine change): K11 (the "during X" template), K12 (§4.3.5 vs §2.4.1), K13 (Thirst), K19 (Annabelle, upstream #11), K28 (a gun's maneuver, §4.3.1 vs §4.10.1), K34 (a dangling reference). Done when the owner has the list to file upstream. Docs: rules-digest §23.
