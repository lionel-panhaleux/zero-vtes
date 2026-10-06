# Scoping: Sabbat V5 precons, 25th and 30th Anniversary decks (2026-10-05)

Plan, owner decisions recorded below. Status: slice 1, Reign of Stanislava, implemented
2026-10-06 (DESIGN §11 P4 · 16d; rules-qa Batches 49–50); The Endless Dance and Sabbat V5
remain. Per-card verdicts, with the precedent card or primitive for each:
`scoping-sabbat-anniversary-cards.md`.

Sources: KRCG `vtes.json` / `twda.json` (2026-10-05); Reign of Stanislava
list: https://www.blackchantry.com/2019/06/04/contents-and-card-changes-in-25th-anniversary-revealed/ ;
Endless Dance list: https://www.blackchantry.com/products/vampire-the-eternal-struggle-fifth-edition/30th-anniversary/
(same as https://vtesone.wordpress.com/expansions/black-chantry-expansions/30th-anniversary-set/);
both validated as "KRCG set minus the 20 listed extras"; Sabbat rules:
https://www.blackchantry.com/2025/06/12/new-sabbat-rules-terms-and-errata/

## Decks

| Deck | Product | Crypt / library | New uniques (crypt + library) |
|---|---|---|---|
| Path of Caine | Sabbat V5 (2025-10-26) | 12 / 77 | 30 (12 + 18) |
| Path of Cathari | Sabbat V5 | 12 / 77 | 23 (12 + 11) |
| Path of Death and the Soul | Sabbat V5 | 12 / 77 | 26 (12 + 14) |
| Path of Power and the Inner Voice | Sabbat V5 | 12 / 77 | 31 (12 + 19) |
| Reign of Stanislava | 25th Anniversary (2019) | 12 / 88 | 15 (5 + 10) |
| The Endless Dance | 30th Anniversary (2024) | 12 / 88 | 19 (7 + 12) |

- Union: **137 new cards** (60 crypt, 77 library). Shared: Under Siege
  (3 Sabbat decks), Unholy Sacrament (2), Zillah's Valley, Ancient
  Influence, Political Stranglehold, Reins of Power (both anniversaries).
- Sabbat V5 decks carry KRCG `precon` tags: adding "Sabbat V5" to
  `FULL_PRECON_SETS` appends them after the 14 (sorted by set), no index moves.
- Anniversary decks have no tag; each deck **equals the KRCG set minus its 20
  extras** (verified for both). A generator can derive them from a 20-id
  exclusion list per set — still generated, `--check`able.
- TWDA: fully-in-pool decks go **81 → 99** (before the version cap). After slice 1 the
  capped list stays at 13: "Stanislava 25" is a version of the anniversary deck (counted
  as a first version, like the precons), and "Do A Thing v2" replaces Gangrel "Imortal"
  as the newer version of that deck. The
  Stanislava cards drive it (Forced March, Monastery of Shadows ×9 decks each;
  Enkil Cog, Instantaneous Transformation, the G2/G3 Gangrel ×7).
- No new clan ("Gangrel antitribu" already in the pool) and one new Discipline
  code, `abo` (Xaviar), already in `discipline_key`: obs vocabulary stable.
- Roster keying: a seat draws a clan, then a deck (`Deck::clan` = most crypt
  copies). The Sabbat precons are singleton multi-clan crypts (Caine: Nosferatu
  5 / Tzimisce 4 / Banu Haqim 3; Power: Ventrue 4 / Lasombra 4 / Brujah 4, a
  tie). Owner: drawn by path (decision 4).

## Verdicts (from the four classification passes)

| Batch | existing | extension | new |
|---|---|---|---|
| Crypt (60; 20 without text, 4 title only) | 40 | 16 | 4 |
| Sabbat library A (29) | 1 | 22 | 6 |
| Sabbat library B (30) | 11 | 15 | 4 |
| Anniversary library (18) | 1 | 13 | 4 |

"existing" cites a pool card with the same primitives; most of the work is
small widenings (new `printed_holds` conditions, `RefEffect`s, requirements).

## Cross-cutting layers

1. **Index placement** — Sabbat V5 is V5-era, but adding it to `V5_SETS`
   sorts 137 cards in and moves every index (goldens, decklist tables, trained
   vocabularies). Recommendation: append, as `ADDED_CARDS` does (or a
   set-keyed appended list).
2. **Sabbat titles** — bishop, archbishop (city), priscus, cardinal, regent:
   `crypt::Title`, `title_votes` (bishop 1, archbishop 2, cardinal 3, regent
   4, priscus: Prisci block), `title_key` (archbishop = `City`: the rulebook
   has prince, archbishop and baron of one city contest — Ingrid, Prince of
   Geneva, contests Rexton, Archbishop of Geneva),
   `TitleClass`, the `gen_crypt.py` TITLE regex (exits today). Also the
   Camarilla **Inner Circle** (Stanislava; completes Closed Session). ~14 cards.
3. **Paths** — KRCG `path` on every Sabbat vampire and ~39 new library cards;
   nothing models it. Black Chantry 2025-06-12: four paths; each new Sabbat
   vampire follows the path printed in its text; during your influence phase a
   Sabbat vampire following no path can burn 1 blood to follow one; one path at
   a time (following a new one replaces the old); benefits only while Sabbat;
   no effect on clan requirements. So path is **mutable vampire state** (a new
   influence-phase decision), not only printed data. The rule says nothing of
   the library icon. Text requirements on ~6 cards (Absolute Tyranny, Burial
   Site, Forward Momentum, Privileged Position, Terrifying Visage, Sakura).
   **Open: what the library path icon does** (see Q1) — it gates up to 39 cards.
4. **Advanced vampires** — Xaviar (G3 ADV) alone, base card absent: an `adv`
   flag suffices; merge rules not needed for these decks. The contest identity
   stays the name: an advanced vampire contests other copies of the same
   vampire, advanced or not (rulebook "Advanced Vampires"); only its text is
   keyed apart (`VampireDef::adv`). Done in slice 1 (`gen_crypt.py` emits `adv`).

## New mechanisms (beyond widenings)

| Mechanism | Cards | Size |
|---|---|---|
| Out-of-turn action (lock to act in any minion phase; retry after cancel) | Enkil Cog (7 TWDA) | L |
| Aim cards (strike-companion window, conditional +damage, no-press, discard-cancel) | Target Vitals | L |
| Combat queuing ("queue a combat", defined 2025-06-12: after the current combat, after block resolution, or after the action's effects) + about-to-enter-combat window | Obedience, Hedonism | M–L |
| Pre-referendum combat window | Yawp Court | M |
| Referendum auto-pass (unblocks Amici Noctis / Voter Captivation rows) | Día de los Muertos | M |
| Per-Methuselah vampire choice at resolution | Ancient Influence, Reins of Power | M |
| Multi-seat pool-for-votes round | Business Pressure | M |
| Post-tally burn for against-voters | Hartmut Stover, Marcos Belegrad | M |
| Look at / reorder top N of library (hidden info) | Eulogio | M |
| Retainer ability suppression in combat | Stanislava | M |
| Starting-life bonus for zombies recruited/employed | Ashur-uballit | M |
| End-of-combat trigger for a card in play | Morbidity | M |
| "Combat about to end" window after `CombatWouldEnd` (golden churn) | Hunting the Quarry | S–M |
| After-combat delayed effects (generalize `PutOnActorAfterCombat`) | Catatonic Fear, Kiss of Cathari, Abuse of Power, Pillars Fall | S–M |
| Unlock-rider scoped to host's controller; move card between titled vampires | Fear of Mekhet | M |
| Ash-heap pick to library; card removed from game whatever happens | Reinforcements | M |
| State-based self-burn; per-torpid-vampire pool burn | Augury of Doom | S–M |
| Diablerie moves a title card to the diablerist | Regent | S–M |
| Per-vampire per-turn card-play limit (plays.rs ignores `PerTurn` for plays) | Forced March, Instantaneous Transformation | S |
| End-of-turn hook; "clan member's successful action" hook | Loup | S |
| `AttemptBlock` for a minion already attempting (ANK 20181122-2) | Sentry Signal, Under Siege | S |
| Non-active seats' impulse between actions (verify) | The Louvre, Paris | S–M |

## Suggested order

1. **Reign of Stanislava** (15 cards): Inner Circle + archbishop titles, ADV
   flag, Enkil Cog (the one L), the play-limit widening. Largest TWDA yield.
2. **The Endless Dance** (19 cards; 15 once 1 is done: 4 library cards shared): mostly
   referendum widenings, Business Pressure, Loup hooks.
3. **Sabbat V5** (105 cards), after the title and path layers; per deck: Power and the Inner Voice (political, titles),
   Cathari, Caine, Death and the Soul (zombie allies, Morbidity, Target Vitals).

Per card throughout: window-matrix rows, ledger rows for every KRCG ruling,
full vampire text (rules-qa 13.1).

## Owner decisions (2026-10-05)

1. **Path icon = play requirement.** A library card with a path icon requires
   a vampire following that path: the acting / playing vampire for a minion
   card; for a master card, a ready vampire of that path you control. Every
   iconed card depends on the path layer, which gives `Requirement`s for both
   shapes, like the clan icon. Path is read live (followed path, Sabbat only,
   rules 2025-06-12).
2. **Append** the new cards after the existing tables (the `ADDED_CARDS` way);
   no V5 index moves.
3. **Order**: Reign of Stanislava, then The Endless Dance, then Sabbat V5.
4. **Roster by path**: the Sabbat V5 decks are drawn by path, a category apart
   from clans (a seat draws a clan or a path, then a deck of it). Stanislava
   counts as a Gangrel deck and may share a table with the V5 Gangrel precon
   (the crypt-share redraw is what separates decks, not the clan).
5. **Anniversary decklists need a review before training**: not every card in
   the bundle belongs to the deck. Status: both lists confirmed against Black
   Chantry (Stanislava: the 2019-06-04 announcement; Endless Dance: the
   30th Anniversary product page, identical to the VTES ONE blog list), and each
   equals the KRCG set minus its 20 listed extras. The 20-id exclusion lists
   are the reviewed part; the generator `--check` keeps them honest.
6. **No Fall of London / Shadows of Berlin decks in training** (not real
   decks). Checked: KRCG tags no precon in FoL, SoB or Echoes of Gehenna, and
   the generators read decklists only from `FULL_PRECON_SETS` /
   `MINI_PRECON_SETS` (V5 sets, New Blood). Their cards reach training only
   through TWDA decks that use them. Keep it so when the new sets are added.

7. **Reign of Stanislava readings settled** (2026-10-06): rules-qa Batch 49
   (49.1 Reins / Ancient Influence choices, 49.2 Hartmut, 49.3 Stanislava,
   49.4 Fear of Mekhet, 49.5 Monastery of Shadows, 49.6 Enkil Cog, 49.7 path
   icon).
   - **Enkil Cog (49.6)**: offered only when the active Methuselah declares
     the end of their minion phase; a Cog action sends the impulse back and
     the minion phase continues. Model it as a decision on the minion-phase
     end, not as a between-actions impulse for every seat.
   - **Mark Decker**: model his werewolf bonus (`OpponentSubtype(Werewolf)` in
     `printed_holds`); Double Deuce is already a werewolf ally in the pool, so
     it is testable now. Renegade Garou (101602, Gangrel werewolf ally: limited
     additional strike, mandatory per PIB 20121028; optional maneuver; +1
     stealth enter-combat action; regains life at 2 or fewer) may be added
     when werewolves come in.
   - **Xaviar**: model the ADV flag so the base card can join later. Owner
     allows adding the base Xaviar (201477, G3 Gangrel, Independent 2 votes,
     prevent 1 damage each combat, +2 strength for 2 blood before range): it
     would make the merge rules (and ANK 20180524: changes sect as merged,
     burns an anarch token) testable. Optional at the end of the slice.

## Open readings (candidate rules-qa batch)

Readings no ruling settles, for the later slices (Endless Dance, Sabbat V5; the
Stanislava ones are Batch 49), to file in a new batch when each card lands:
   Reinforcements vs Delaying Tactics; Camarilla Conclave penalty timing and
   cast votes; Marcos Belegrad per Methuselah or per source; Fotini "until your
   next discard phase" vs end of turn; Ashur-uballit live or fixed life;
   Věnceslava "burned" includes paid costs; Heartrender removal timing; Forward
   Momentum on a blocked / 0 bleed; Augury of Doom state-based burn timing; Rage
   of Apedemak hand size on end; Morbidity blood destination; Louvre on a locked
   Toreador; Eulogio optional look/reorder.

## Footnote: next legacy additions

Cards one short of completing a TWDA deck once these 137 are in (decks
completed): The Unmasking 9, The Bitter and Sweet Story 7, Scourge of the
Enochians 7, Maila (G6) 5, The Status Perfectus 4, Adhocracy 3, Gift of
Proteus 3, The Erciyes Fragments 3, Eat the Rich 3, Psyche! 3, Genevieve (G2) 3.
