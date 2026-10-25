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

- **Reachable edges, mid-play ousts and costs**: Disarm's torpor ousting the active Methuselah inside the play; True Love's Face's pay-to-cancel for a last pool; Life Boon's deferred oust still listed in windows; Meditative Grove's canceled cost missing Regina Blake's announced blood (`settle_canceled`). Done when each is a test, fixed if wrong, and gone from DESIGN §11.3. Docs: DESIGN §11.3.
- **The cheap latent edges fixed now**: `ImmuneToDirectedActions` in `ability_targets`; `Effect::PoolToHostBlood` ousting a non-active seat; a cancelling ability's mark after its terms; directed actions naming several Methuselahs (`ActionCtx::directed_seats`, Shadow Cast). Done when each is fixed with a test or stays in DESIGN §11.3 as waiting for a pool card. Docs: DESIGN §11.3.
- **Tests for cards covered by fuzz or code paths only**: Rapid Change [PRO], Wolf Claws [PRO], Rötschreck's range check, Powerbase: Savannah with Deep Song's target, Saatet-ta's stealth and intercept locks, Carlton Van Wyk's strike and dodge. Done when each has a test and DESIGN §11.3's legacy-additions list is empty. Docs: DESIGN §11.3.
