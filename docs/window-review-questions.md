# Window-matrix review — owner arbitration queue (rules Q&A Batch 2)

**Status (2026-09-02): arbitrated.** Owner rulings recorded in
`docs/rules-qa.md` Batch 2 and applied to `data/window-matrix-reviewed.csv`
(0 question rows left). Per-entry outcomes are marked "→ ruled" below; the
convention notes at the end still need feeding back into the generator.

Per-card questions raised by the hand review of `data/window-matrix-draft.csv`
(output: `data/window-matrix-reviewed.csv`). Each entry: card, text quote, draft
vs proposed classification, and the precise question. Entries marked **Q** need
an owner ruling; entries marked *note* record non-obvious readings applied
(owner may veto). Grouped by theme.

Vocabulary changes introduced by the review (owner may rename):

- `REACTION_DURING_ACTION` — generic reaction window during an action (replaces
  the draft's `REACTION_UNCLASSIFIED` default where the card is simply a
  reaction with no narrower filter).
- `REACTION_BOUNCE` — Deflection-class bleed redirects: playable while the
  controller is being bled, must precede the would-be-successful window
  (advanced-rules §2.1.4); replaces the draft's `ACTION_BLOCKS_DECLINED`.

---
## Bounces and "after blocks are declined"

The parent brief assumed the draft's `ACTION_BLOCKS_DECLINED` was wrong for
bounces — it is not: V5 wording prints "Only usable if a minion is bleeding
you, after blocks are declined" on Deflection and Bait and Switch. The window
name stands; bounces additionally get the new `bounce` flag (their redirect
reopens block opportunities per rules-qa 1.3). `REACTION_BOUNCE` from the
header is withdrawn.

1. *note* — **Archon Investigation**: "Out-of-turn. Only usable if a minion is
   bleeding you and the bleed amount is 4 or more, after blocks are declined."
   Draft: `OUT_OF_TURN`. Proposed: window `ACTION_BLOCKS_DECLINED`, flag
   `out_of_turn` (out-of-turn is master-phase *accounting*, not a play window).

    → ruled 2.3: accepted (`out_of_turn` flag; timing is card-text dependent).

## New window names introduced (owner may veto/rename)

2. *note* — **Cats' Guidance** [ani]: "Only usable by a locked vampire who has
   blocked, after block resolution. Unlock this vampire." Draft:
   `ACTION_STEALTH_INTERCEPT` (regex hit on superior's intercept). Proposed:
   `ACTION_AFTER_COMBAT;ACTION_STEALTH_INTERCEPT` — advanced-rules §2.3.3
   groups it with Freak Drive/Hay Ride as after-combat plays. **Q**: confirm
   the [ani] mode's window is the after-combat window (not a point between
   block resolution and combat start).

    → ruled 2.1: after-combat window, after *all* combats of the action.

3. *note* — **Change of Target**: "Only usable if this minion is blocked,
   before block resolution." Proposed new window `ACTION_BLOCKED` (modifier
   plays legal only while a block is pending resolution) — distinct legality
   filter from the generic `ACTION_DURING`.

    → ruled 2.3: accepted.

4. *note* — **Claws of the Dead** [pro] ("hand strikes aggravated this round",
   no printed timing): proposed new window `COMBAT_UNTIMED` — combat effects
   with no printed step, playable at any combat impulse, deadline derived from
   the effect (§2.1.4: must precede the step they modify). **Q**: confirm this
   class exists as one window rather than per-card step assignments.

    → ruled 2.2: accepted as one window with effect-derived deadline.

5. *note* — **Deep Song** [ANI]: "The target vampire is considered the acting
   minion during that combat" — acting-designation override (rules-qa 1.2 /
   advanced-rules §3.10 territory); stress-set material for the combat machine.

## Further new window names (owner may veto/rename)

6. *note* — **Spying Mission** [OBF]: "Only usable if a bleed would be
   successful. Instead, …" — new window `ACTION_WOULD_BE_SUCCESSFUL`
   (advanced-rules §3.4.3: the last window before resolution; bounces must
   precede it).

7. *note* — **Telepathic Tracking** [AUS]: "Only usable if both combatants are
   still ready and combat would end. Instead, start a new round." — new window
   `COMBAT_WOULD_END` (the would-end replacement hook, distinct from
   `COMBAT_END_OF_ROUND` plays like Taste of Vitae/Disarm).

8. *note* — **Hide the Mind / Anticipation [AUS] / Sudden Reversal**: cancels
   played "as it is played" get window `CARD_AS_PLAYED` (the as-played cancel
   sub-window, §1.8.1). Sudden Reversal: `OUT_OF_TURN` demoted to the
   `out_of_turn` flag, same treatment as Archon Investigation (entry 1).

    → ruled 2.3: accepted.

9. *note* — **block-loop windows split**: cards "only usable if a minion
   attempts to block" (Forced Confessional, Stygian Shroud, True Love's Face
   [OBF][PRE], Form of the Bat [pro]) get `ACTION_BLOCK_ATTEMPT` (attempt
   pending); cards "only usable if blocked, before block resolution" (Change
   of Target) get `ACTION_BLOCKED` (the successful-block-to-combat gap,
   advanced-rules §4.1.2). These are distinct legality filters per rules-qa
   1.3 semantics.

    → ruled 2.3: accepted.

10. *note* — **WAKE broadened**: classified as `WAKE` every reaction whose
    filter is "usable by a locked minion" — true wakes (WWEF, On the Qui Vive,
    Forced Awakening, Eyes of Argus [AUS], Shadow Sentinel) *and*
    unlock-and-attempt/unlock reactions (Dogged Pursuit, Eyes of the Wild,
    Guard Dogs, Rat's Warning, Second Tradition [2nd mode], Sense the Savage
    Way [ANI], One With the Land [PRO], Organized Resistance, Forced Vigilance
    [FOR]). **Q**: keep one window for "locked-minion reaction plays", or
    split wake-grants from unlock-effects?

    → ruled 2.4: **split**. True wakes → `WAKE;CARD_AS_PLAYED` (wakes are
    playable in the as-played window, e.g. before Consign to Oblivion's cost
    increase resolves). "Unlocks and attempts to block" → new
    `REACTION_UNLOCK_ATTEMPT` (block loop only); plain "Unlock this vampire"
    (Guard Dogs, Rat's Warning, Forced Vigilance [FOR]) → new `REACTION_UNLOCK`.
    Neither is playable in the as-played window. *note* (owner may rename the
    two new names): Organized Resistance mode 1 — a *locked* baron grants +1
    intercept without unlocking — is neither wake nor unlock; classified
    `ACTION_STEALTH_INTERCEPT` with new flag `usable_locked`.

## Question cards (review=question in the CSV)

11. **Q** — **Cats' Guidance [ani] / Forced Vigilance [for]** ("Only usable by
    a locked vampire who has blocked, after block resolution. Unlock…"):
    proposed `ACTION_AFTER_COMBAT` per advanced-rules §2.3.3 grouping them
    with Freak Drive/Hay Ride. Confirm the play point is the after-combat
    window, not a point between block resolution and combat start.

    → ruled 2.1: after-combat window, opening after all combats of the action.

12. **Q** — **`COMBAT_UNTIMED` class** — Claws of the Dead [pro], Wolf Claws
    [pro] ("hand strikes aggravated this round"), Pack Alpha [ANI] (burn
    retainer to attach): combat cards with no printed step. Proposed: one
    window "any combat impulse, deadline derived from effect" rather than
    per-card step assignment. Confirm, and confirm the aggravated-hands plays
    must simply precede the strike resolution they affect.

    → ruled 2.2: confirmed; aggravated-hands plays must precede strike resolution.

## Flags added by the review

- `in_play_action` (55 cards) — card in play grants a minion action (Ⓓ
  actions, referendum-to-burn, granted hunt/equip variants). Drives the P2
  action-mask surface. MPA/DPA-consuming abilities (Parthenon, Uncoiling,
  Tension in the Ranks) deliberately excluded — phase economy, not minion
  actions.
- `trifle` (10 cards) — master-phase-action accounting (rulebook trifle).
- `bounce` (5 cards: Deflection, Telepathic Misdirection, Redirection, My
  Enemy's Enemy, Bait and Switch) — bleed redirect; reopens blocks (rules-qa
  1.3).
- `draw_replacement` (Black Market Cache, Shilmulo Tarot, Visit from the
  Capuchin) — intercepts replacement draws.
- `alt_hand_source` (Fleshforge Chamber) — play cards from a location as if
  from hand.
- `usable_locked` (Organized Resistance) — reaction play whose "unlocked"
  requirement is waived without a wake or unlock (rules-qa 2.4).
- `token_entity` extended to cards *becoming vampires* (Childe of the
  Revolution, Waters of Duat) — the generator's regex only caught allies.

## Convention notes (feed back into scripts/window_matrix.py)

- Equipment/ally/retainer/master rows classify the *play* window only; in-play
  granted abilities (strike/maneuver/prevention from weapons, lock-to-X
  locations) are the effects framework's concern, except granted *actions*
  (flagged `in_play_action`).
- Strike-card riders — **owner-corrected (2026-08-04)**: the rider's timing
  relative to the strike step decides the play window. A strike with an
  optional *maneuver* rider is playable at the maneuver (determine range)
  window, committing the strike early (DESIGN §4.4); a strike with a *press*
  or *additional strike* rider is played at strike declaration, since those
  steps come after. Applied: Aid from Bats, Slam, Diversion regain
  `COMBAT_DETERMINE_RANGE` (+ new flag `maneuver_rider`); Hidden Strength
  loses `COMBAT_PRESS` (its press is a rider, not a mode — unlike Flash,
  Indomitability, Resist Earth's Grasp, whose "or press"/"Press, or" wordings
  are real press-window modes and keep the window). Same principle for
  non-strike cards: riders firing after the card's own play step add no play
  window.
- Wake regex split (rules-qa 2.4): `wakes?\b` → `WAKE` + `CARD_AS_PLAYED`;
  `unlocks? and attempts? to block` → `REACTION_UNLOCK_ATTEMPT`; bare
  `unlock this vampire` on a reaction → `REACTION_UNLOCK`, *except* when the
  mode says `after block resolution` (Cats' Guidance [ani], Forced Vigilance
  [for]) → `ACTION_AFTER_COMBAT`; the generator cannot split modes, so Forced
  Vigilance (`ACTION_AFTER_COMBAT;REACTION_UNLOCK`) will not converge without
  per-mode classification. Only wakes and `cancel_as_played` cards may carry
  `CARD_AS_PLAYED`.
- "Prevent"/"votes" regex false positives: granted prevention abilities and
  vote-mention cards are not prevention-window/referendum plays (Amici Noctis,
  Voter Captivation are after-resolution).
- Oddballs worth stress-set slots: Owl Companion / Revelations (open-hand
  information effects), Spectral Servitor / Vozhd of Juiz de Fora (allies
  playing discipline cards "as a vampire"), Screamer (burn as an
  as-announced interrupt), Piper (recruit outside an action).
