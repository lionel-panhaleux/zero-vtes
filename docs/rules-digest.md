# Rules digest — the settled readings by mechanism

The owner's settled readings of the VTES rules (the owner is a VEKN judge),
organized by mechanism, over `docs/rules-qa.md`. rules-qa is the append-only
log of the rulings, in the order the questions were asked, cited by code as
`rules-qa N.M`; this digest states each settled reading once, where an
implementer looks it up, and cites its sources. It states rules — how the
game works — not how the engine models them (`docs/engine.md` has that).

It is what an implementer reads before designing a card, what a second
reader uses to read a card independently of the code, and the seed of an
"advanced rules for AI" document and of a judge bot.

## 1. How to use

Before designing a card, read §2 (the default readings), then the §3 entries
and the mechanism sections the card touches (a bleed modifier: §8, §9, §10,
§11.1; a combat card: §12; a location with a lock: §5, §16). Each reading
cites its sources; for more, look next at the rules-qa item (its full
wording, the card examples), the card's KRCG rulings (`data/rulings-v5.csv`,
one row per ruling, with its test), and the advanced rules section (the
judge-level synthesis of the ~2,600 KRCG rulings, the source for effect
semantics; this digest points to it rather than restating it). A reading no
source here settles is implemented and recorded as a question in a new
rules-qa batch; once the owner answers, its line lands here too.
Readings implemented but not yet confirmed by the owner are listed in §24.1;
the disagreements found between sources, in §23.

Citation conventions:

- `31.2` (bare, in parentheses or prose): rules-qa item 31.2. Every rules-qa
  item is reachable from the index (§25).
- `§8.3`: a section of this digest.
- `advanced rules §3.2.1`: `../vtes-advanced-rules/docs/advanced-rules.md`;
  its owner adjudications (`.claude/references/owner-rulings.md` there) are
  cited by their advanced rules § (§1.6, §1.8.2, §3.2, §4.5).
- `[LSJ 19980302]`, `ANK 20200905`, `RTR …`, `PIB …`, `TOM …`: KRCG ruling
  references, as in KRCG and the ledger.
- `rulebook "X"`: the 2024 rulebook's section X (the `vtes` skill's
  `references/rules/rulebook.md`).
- "owner (date)": an owner correction or answer, as recorded in rules-qa.

Precedence: the owner's latest word (rules-qa, newest item first) settles a
reading, over a KRCG ruling when they differ (each case listed in §23);
an item revised later is indexed to the section stating the current
reading. A card's own text overrides every default here ("even if …
not yet needed", "cannot be canceled", "requirements and cost apply as
normal").

Card names are as printed; Disciplines in brackets (`[pot]` inferior,
`[POT]` superior); clan names in their V5 forms (Banu Haqim for Assamite,
Ministry for Follower of Set: 70.2, 69.14, 71.11).

## 2. Default readings — apply before asking

The patterns first readings got wrong repeatedly, each corrected by the
owner (rules-qa "Default readings", extended by the later batches). Apply
them to every new card; only a ruling or the card's own text overrides them.
Cited as D1–D33.

**D1. Choices are made at declaration.** Whatever a card or ability names or
chooses — a target, an ally to fetch, a retainer and where it goes, an
amount, who pays — is chosen as it is played, used, or as the action is
announced; never at resolution (31.2, 32.12, 38.1, 41.6, 41.7, 48.8). A
master card's choices are its terms, made as it is played (The Rack's
vampire, Toreador Grand Ball's two Toreadors, Priority Contract's
Assamite); a card in play chooses as its effect is used (Chantry's blood
payer, Anarch Troublemaker's two vampires — a locked one allowed —,
Garibaldi-Meucci Museum's two cards) (28.5). Only a
random or searched result, and what the card says is chosen later ("chosen
at resolution": François Villon's 1 or 2, 62.4; Sakhar's and Dominica's
search, 17.1; Split the Veil's ally as the last counter burns, 32.11), is
known at resolution — and the choices inside the effect, or worded on its
success: Break the Bonds' lock and The Platinum Protocol's counter "if the
bleed is successful" (25.6), Redistribution's and Gird Minions' moves (68.10,
69.4), Third Tradition: Progeny's blood (70.18) (advanced rules §1.1.1,
§3.4.2). Numeric terms are announced with the card (3.5); a referendum's
terms after the action succeeds (3.5, §13.1). At resolution the choices are
re-checked: gone or illegal → the action is successful with no effect (a
fizzle, §10.2).

**D2. A played card is played, however it came.** A card played from a
search, from the ash heap, by Contraband, Piper, Vast Wealth, Spiritual
Guidance, The Erciyes Fragments… is played — not "the normal way", but
played: cost reductions apply, "when played" effects and "cannot play" bans
apply, it counts for "only one … each turn / per game" (36.5, 38.5, 40.10,
41.13, 38.7). Only an effect that says *put in play* does not play the card
(and bypasses requirements and cost); "equip", "recruit", "employ" keep
requirements and cost (advanced rules §1.6.3, §1.8.2). Exceptions are the
reductions whose own text needs the card played normally through an action
(Charisma's "recruit ally actions", 36.5, 62.13, 63.9).

**D3. A cost may be paid even if paying burns or ousts the payer**, as long
as the payer has it: an ally pays its last life and burns, its attempt or
effect then failing — a block attempt so paid fails, a combat it is in ends
(41.4, 42.2, 60.13, ANK 20180928-1) — and a Methuselah pays their last pool
and is ousted (38.5, 33.2, 36.3, 58.4, 71.2). For an ally's last life paid
as its own action's cost, LSJ 20080512 and advanced rules §5.5.3 have the
action still resolve — open (§23, K7). A payment the payer does not
have is not offered. The one exception on record: burning one's last pool
to cancel True Love's Face is not offered (18.2).

**D4. Limits and caps read on the words they qualify**, no further: "not to
exceed their starting life" caps allies only; "once each round" is one use
for one strike or damaging effect; "only one Hive Mind at [san] each turn"
counts [san] plays only; copies of Protected Resources do not lower its cap
(38.8, 41.8, 68.14, 70.8).

**D5. Properties are the card's, never assumed.** Not every vampire is
unique: created vampires (Embrace, Childe of the Revolution, Waters of Duat)
and cards printed "Non-unique." (Valkyrie) are not (41.5, 58.5). A minion
card on a minion is controlled by the minion's controller; a master card on
a minion stays its player's (7.5, 38.4; Anathema's ruling, PIB 20121028, is
the contested exception, §23). A card that is not unique never contests
(44.10).

**D6. Choosing in a referendum's terms is never targeting** (38.6, 35.1).
Choosing a vampire as a master card is played *is* targeting (35.1, §3.5).

**D7. "During X, … can …" is once per X** (per copy, per vampire): "during
your turn" once a turn, "during your minion phase" once in it, "during an
action / a bleed / a referendum" once in it — never repeatable while payable
(6.6, 15.1, 15.2, 42.3, 42.9, 58.7, 60.6, 51.2). "During" always means once
(79.1: Eyes of the Wild, Nergal ADV corrected; Marciana's grant, 79.2); the
word decides: an ability without it is as often as paid (Maris Streck,
79.3). Two "during" clauses of one card are each once in the same span
(Descent into Darkness's blood and return, 79.4). Read "during" in every
ability's wording before choosing its limit; `tests/lint.rs`
(`during_abilities_are_once`) checks the definitions. A "during" phase
window is any time in that phase, inside actions and combats too (26.5). The
advanced rules read no limit into "during" (advanced rules §1.2.1, §1.5.4);
the owner's reading governs (§23).

**D8. "After X, … can …" is once after X** (per copy, per vampire): "after a
successful bleed / an action / a referendum passes" once after it, unless a
cost or condition already limits it (lock this card, must be locked)
(16.1; Gostoso, Forward Momentum, owner 2026-10-09).

**D9. "Use a master phase action to …" is not a "during your master phase"
ability**: usable as often as master phase actions are spent (36.4).

**D10. A mandatory phase effect applies in the phase its card is played
in.** "During your / each Methuselah's X phase, …" on a card entering play
during an X phase is due in that very phase, ordered among the phase's other
mandatory effects by the player — not first in the next X phase. Phase dues
are read live, never snapshotted as the phase begins (33.1, 60.22, 40.5,
57.5, 57.6).

**D11. Card text addresses its controller.** "You", "your" are the card's
controller (32.7, 49.5, 73.7): Open War's pool goes to its controller;
Zizi's "your ready region" is her controller's. Designating another player
needs that addressing; an unaddressed "a vampire", "each Ravnos", "Toreador
get …", "any other controlled Sabbat vampire" is any Methuselah's (49.5,
28.2, 33.3, 53.7, 69.2, 70.2, 77.6). A name with
no region ("while /Ludmijla Rakoczy/ is ready", "unless Brunhilde is ready")
is any Methuselah's copy (73.7, 58.5). In an action card, "you" is the
acting Methuselah (25.5).

**D12. Ⓓ is only a reminder.** Printed or not, an action aimed at another
Methuselah or what they control is directed at them; one aimed at one's own
minion or card is undirected (20.1, 17.4, 17.2; §3.4).

**D13. Conditions are read live, never snapshotted**, even when no pool card
could change them yet (18.4, 13.2, 19.3, 39.5, 32.4): as the effect applies,
each time it applies. During an action, the action's own requirements are
checked continuously: a requirement lapsing mid-action fizzles it; a card
already in play stays when its requirement lapses (advanced rules §1.6.1).
The few fixed amounts are the card's words (Night Terrors' fixed -X, RTR
20030519; Visions of Zapathasura's reduction, 23.5).

**D14. Legal no-op plays stay offered.** A play that does nothing is legal
and offered (cycling a card is a real purpose): a hand strike at long range,
a dodge with no strike to dodge, an unlock of an unlocked vampire, a steal
from an empty vampire, X = 0, a counter burn for votes one cannot cast, a
block attempt that cannot reach the stealth (4.3, 17.4, 25.5, 57.10, 63.15,
68.5, 73.3, 73.4, 73.8). Only what a rule or the card
forbids is barred: prevention with no damage to prevent, stealth or
intercept not needed (3.2, advanced rules §1.6.5, §3.2.1, §4.5.1), a card
naming an object there is none of (advanced rules §1.6.5: Lenelle with an
empty ash heap, 17.1; The Sargon Fragment, 60.17; Bolesław Gutowski with an
empty crypt, 78.20), card limits — and an
action sure to aim at an illegal target (17.4). An ability or granted
action with nothing to act on is still a legal no-op (73.3, 73.4). An engine
exception, not a rule: an ability whose only effect unlocks its own unlocked
minion is not offered (48.7, §22).

**D15. Stealth and intercept only when needed.** Gaining stealth is legal
only while a block attempt is pending with intercept ≥ stealth; intercept
only while stealth exceeds it; surplus plays are illegal (3.2). Advanced
rules §3.2.1 reads need as a threshold, not a cap (once needed, a play may
give more): open (§23, K23). Exceptions are the card's words ("even if …
not yet needed") and standing bonuses of cards in play (Loup, a Repulsion
on the vampire) (3.2, 51.7). Reducing the opposing value is legal even to
no effect (on zero or negative stealth, LSJ 20021211, 48.1), within its own
timing: a third party's stealth strip needs the defender attempting (1.3),
"during the action" reductions wait for the block attempts (66.1),
Gwenllian Arwyn's -1 intercept only while stealth is needed (15.2). A
gift of intercept "to a minion" (London Evening Star, Masika) goes to a
minion attempting to block that needs it (77.20). §9.2.

**D16. An ally playing cards "as a vampire" is one only for the card it so
plays** (73.6): Spectral Servitor, Herald of Topheth, Infernal Servitor are
Independent vampires of their granted capacity for the cards they play; no
other card or rule sees them as vampires (Powerbase: Munich's "Independent
vampires can burn this location" is never theirs). A card saying "this
vampire" gives an ally nothing (60.17, 63.5, 71.14).

**D17. A card hunt is a hunt.** An action a card makes a hunt (Loki's Gift,
Psychophagia, Abactor, Week of Nightmares' Ravnos hunt) is a hunt for every
effect reading one; but "the usual +1 stealth" is the rulebook hunt's own:
a card hunt has the card's stealth (72.1–72.5, 32.13). §3.1.

**D18. A card's type follows how it is played.** A dual Action Modifier /
Reaction card is a modifier when the acting minion plays it, a reaction when
another plays it (52.2, 70.17, 71.24); a card of several types counts as the
type it is played as (62.2: Rapid Change's [PRO] strike is a combat card,
its [pro] stealth is not). §3.8.

**D19. "Targets both" directs the action at both.** An action aimed at two
minions of two Methuselahs is directed at both, and both may block (71.21;
Shadow Cast's set of Methuselahs, 24.5).

**D20. A cost is not an effect.** A card's "burn n blood" effect is not its
cost: cost reductions do not touch it (62.2, 63.5; Graverobbing, Second
Tradition: Domain, Deep Ecology, Stygian Shroud, ANK 20210226), and the card
is not played without that blood beyond its cost as paid, surcharges
included (77.8). A gift is
not a cost (45.3). An action's costs are paid as it resolves, never at
announcement (3.6, 42.1, 54.4, 55.3, 52.10); burning the card that grants an
action as the action is declared is not a cost (54.4). §7.3.

**D21. "Put this card on …" puts the card itself in play**, out of the ash
heap it reached as it resolved — never an effect-only stand-in (47.1, 46.4,
revising 21.1). §16.5.

**D22. Additional strikes come only after the first strike exchange**, both
combatants still ready, one card at a time (47.2, 48.2, overriding 4.4).
§12.9.

**D23. A strike is at close range unless it says otherwise** (21.1, 26.6,
38.4, 62.19, 71.23); its "put this card on the opposing minion" happens only
if it lands (21.1).

**D24. Stealth and intercept have no floor**: they go negative (48.1).

**D25. A printed ability with no "if ready" works in torpor**; one printed
"while ready" does not (13.2, 15.2, 16.4, 58.8, 71.8, 62.3). A torpid
vampire still casts no votes (15.2, 58.8).

**D26. The same action modifier or reaction is played once per action** by a
minion, by name, whatever the level, a canceled play counted, the
referendum included (46.1).

**D27. "Only one … each turn / in a game" counts every Methuselah's play,
a canceled one included** (28.8, 44.10, 52.3, 60.16, 70.10, 71.16; Ashur
Tablets). Each event is once a game across the table (40.1).

**D28. "A minion you control" includes torpid minions and the actor
itself** unless the card says ready (17.1, 57.2, 28.6, 58.12, 50.5). Ready
is not unlocked (50.5): "a ready vampire" may be locked.

**D29. "Cards requiring a Discipline" are cards played using it** (the
played mode), not cards that merely print it (38.4, 42.5, 41.9, 62.2); but
"a card requiring Dominate" for a referendum ability counts any library card
whose disciplines include it (15.2), and "a card requiring a Discipline" for
a steal/burn counts any card printing a requirement (17.4). Read which:
played (a cost, a toll) or printed (a target). §3.12.

**D30. Younger and older are by capacity** (rulebook; 40.11, 41.14, 42.4,
63.4): equal is neither; the capacity in play, Discipline cards included,
unless the card compares a printed one (60.18).

**D31. Text naming the bearer's or player's trait reads the current
holder** (9.1, 12.5, 70.20): a non-Ravnos bearer of Treasured Samadji gets
neither bonus.

**D32. A card text "once each turn" on an in-play ability is once across the
turn, any of the turn's referendums or actions, one use for the card
whichever its mode** (62.32: Ferraille's votes for or against); "once each
combat" over every round of it (14.2); "once each round" once a round
(38.8).

**D33. An effect outlives its source.** What a card or ability started
(a grant "this turn", a lasting bonus) stays when the source leaves play:
"triggered effects do not disappear with their source" (owner, 74.2:
Lucinde, Alastor G7's grant).

## 3. What counts as X

Classification questions, where the owner's corrections cluster. Each entry:
the reading, its sources, the known edge cases. The mechanism sections
below rely on these and do not repeat them.

### 3.1 A hunt

- The rulebook hunt (+1 stealth, undirected, gains blood) and every action a
  card makes a hunt (Loki's Gift, Psychophagia, Abactor, Week of Nightmares'
  Ravnos hunt, Giovanni del Georgio's) are hunts for
  everything reading "hunt", "when hunting", "hunt actions" (72.1, LSJ
  20050720-2): Volo's and Kalyani's "-1 stealth when hunting", Abiku's +1,
  Sunset Strip, Unlicensed Taxicab, Hospital Food, Aaron's Feeding Razor,
  Ariadne's "+1 hunt" (72.1, 72.3, 72.4, 63.12, 63.16, 64.1, RTR 20030519).
- "Does not get the usual +1 stealth when hunting" (Zöe) removes the
  rulebook hunt's own +1; a card hunt has its card's stealth (72.2).
- "Other non-hunt actions cost an additional blood" (Black Annis) spares a
  card hunt (72.5).
- A vampire that must hunt may discharge it with a card hunt (Psychophagia,
  Abactor; 32.13, 63.2); under Week of Nightmares a Ravnos that must hunt
  must take the special hunt (28.8).
- "Did not hunt" (Thirst): a vampire that took a hunt action this turn
  hunted, blocked or not (40.2, LSJ 20050727).
- "Successfully hunts" / "successful resolution" of a hunt: unblocked and
  gained blood (17.3, 63.2, LSJ 20090411, advanced rules §3.7.2); a hunt at
  capacity or by a diseased vampire is successful but gains nothing (63.12,
  59.4) — no Abactor blood hunt then.
- "+n hunt" adds to every hunt of the vampire, non-standard ones too (64.1,
  RTR 20030519); a hunt taking blood from a target takes the bonus from it
  (53.8, RTR 20030519); "cannot gain blood by hunting" covers every hunt
  (59.4).
- Hunting grounds are not hunts (§16.2).

### 3.2 Success, failure, fizzle, cancel (of an action)

- **Blocked**: a block succeeded; the action is not successful (53.6: a
  blocked bleed is "unsuccessful").
- **Successful**: unblocked and resolved, whatever its effect did; a
  **fizzle** is a successful action with no effect — its target gone, a cost
  unpayable at resolution, a requirement lapsed (3.6, 17.3, 17.4, 28.7, 42.6,
  43.1, advanced rules §3.4.1). After-resolution cards keyed to success
  apply (Freak Drive, 3.6; Aline Gädeke, 16.4; Hedonism's queued combat,
  56.1; Infernal Servitor, 70.19; I am Legion's pool, 60.15), and so does
  the action's own "if successful" clause beside the lost effect (Tye
  Cooper's ash card, 78.38).
- **Failed**: ended unsuccessfully by an effect at or before resolution
  (Archon Investigation, Spying Mission [OBF], Obedience, Yawp Court's
  combat; 56.1, 56.2, 54.1). A political action whose referendum fails is
  successful (§13): the failure is the referendum's.
- **Canceled**: an action card canceled as it is played is no action (no
  Waiting Game counter, 71.10, ANK 20210124); an action canceled as
  announced (Direct Intervention) did not resolve and may be retried (1.4);
  an action "canceled before resolution" (The Kiss of Ra) was taken (the
  optional action, Veil the Legions' count, ANK 20210124) but has no
  outcome, no after-resolution window and no NRA mark (64.4).
- **Ended as blocked**: Change of Target, Mirror Walk's [THA], a Fury's
  block — counted as blocked, no combat (23.1, 8.4, 60.21).
- A bleed is successful only for ≥ 1 pool (24.4); a bleed of 0 succeeded as
  an action (Freak Drive playable) but triggers nothing keyed to a
  successful bleed. A referendum "succeeds" when it passes (58.14).
- Every action has an after-resolution window, blocked or failed included,
  except one canceled before resolution or ended as failed by a combat at
  Yawp Court (23.3, 5.8, 54.1, 64.4). §10.

### 3.3 A bleed; "a bleed against your prey"

- A bleed action is the rulebook bleed or an action card / granted action
  that is a bleed; "+1 stealth when bleeding" applies only during bleed
  actions (33.9).
- "Against your prey": the bleed's target as it resolves, after redirects —
  a bleed deflected to another Methuselah is not against your prey (50.3,
  52.4, 24.3); a bleed that ousted the prey still was against them (61.2).
- "The rulebook's limited bleed bonus" is the "(limited)" one; other "+1
  bleed" add on top (42.7, 30.5, 44.3).
- "The current bleed" is the base bleed with printed text, cards borne and
  layers, no action modifier (59.9).

### 3.4 A directed action, and at whom

- An action is directed at the Methuselah whose hand, library, crypt,
  uncontrolled region or controlled card (any card: stealing The Rack) it
  targets (17.2); at the controller of a minion it targets (20.1). An
  action directed at a card of a Methuselah's is directed at them (60.21).
- Aimed at one's own minion or card: undirected, whatever the icon (17.4,
  20.1, 32.5, 38.2, 59.3, 71.13). Actions targeting an ash heap are
  undirected (17.2). Exception: rushes need another Methuselah's minion
  (17.3, 27.2, 52.6).
- "Minions can … as a Ⓓ action" on a card in play (Creeping Sabotage):
  the controller's own minion takes it undirected, another's directed at
  the controller (6.8).
- Rescue and diablerie of another Methuselah's vampire are directed (16.4,
  41.5); a burn action on their card is directed (16.4).
- An action may be directed at several Methuselahs (24.5, 71.21: a second
  target makes both directed); Loki's Gift's bite is directed at the bitten
  vampire's Methuselah (68.17).
- "During directed / undirected actions" reads the action performed,
  whoever performs it (13.2). "Directed actions" for The Guardian's bar: every
  declaration aimed at another Methuselah, their minion or card (41.5).

### 3.5 A target, a choice

- A target is what an action or card is aimed at as announced or played
  (D1). "Choosing is targeting" (LSJ 20031010): a master card's vampire chosen
  at play (Toreador Grand Ball, Priority Contract), the vampire a master
  card is played for (Piper), Archon Investigation's bleeding vampire
  (35.1).
- Choosing in a referendum's terms is never targeting (38.6, 35.1:
  Banishment can choose a Secure Haven vampire).
- Employ, recruit and equip cards target nothing (38.6).
- Targeting a card on a minion does not target the minion (17.4: Evan Klein
  does not flip).
- A choice made as an effect resolves is not a target of the action
  (Gilbert Duane's vampire, 44.5; Propaganda's minion is named at play,
  31.2).
- Esprit de Corps: "would target you (or a card you control)" — an action
  card picking your seat, minion or card, or a bleed against you; not a
  political action (terms are no target, D6), an employ or a recruit (no
  target); the played mode's Discipline (advanced rules §1.8.1); not a card
  played otherwise than from hand (RTR 20001020) (77.12).
- A seat target never names an ousted Methuselah (Curse of Nitocris's
  choice) (78.52).

### 3.6 A vampire; unique; a minion

- A vampire is a crypt card in play, or a created one (Childe of the
  Revolution, Embrace, Waters of Duat, Web of Knives Recruit, the Great
  Beast); created vampires are not unique (41.5, 28.1, 32.6, 69.6, 60.20).
- An ally playing "as a vampire" is a vampire only for the card it plays
  (D16, 73.6, 62.33).
- Unleash Hell's Fury is a minion only for its block attempt (a 9-capacity
  infernal vampire, 60.21).
- A vampire in an uncontrolled region is out of play and no minion (69.6,
  28.3, §14.6); so is one Descent into Darkness puts out of play, with its
  cards (78.46). Dual Form's card is a created non-unique vampire, its
  Disciplines fixed as it resolves, no title or trait copied (78.47).
- "Werewolves you control" are allies of the Werewolf subtype (64.1);
  "animals (allies and retainers)" are those whose type line makes them
  animals (41.13); a "mage" or "demon" ally by its type line (41.8, 62.30,
  62.31).
- A minion card is any library card but a master or an event (60.3).

### 3.7 Played, put in play, moved, equipped

- Played: D2. Put in play: no play, no requirement, no cost (D2; 40.10
  Absimiliard's Army's ghoul, 44.4 Jake Washington's ally).
- Recruited / employed: the normal action, requirements and cost; a card
  another card puts in play is not "recruited" (40.10, 44.4) — it may act the
  turn it enters (§14.7); Pack Alpha's retainer is paid and put in play,
  not employed (22.2).
- Moving equipment (a steal, the equip-from-another-minion action, a
  locquipment move) is not equipping: no requirement checked, nothing stored
  (17.4, 12.2, 38.7, 70.11); nor "an equip action" for Hag's Wrinkles,
  which an equip from hand, Vast Wealth's or an action card equipping from
  the library (Magic of the Smith) is (78.28).
- "Comes into play under your control" from the uncontrolled region =
  influenced out (58.12, 34.7, 57.9).
- "Entered play since …", "entered your ready region during …": a move out
  of the uncontrolled region or a put in play then; a merge keeps the entry
  (70.3, 71.3).
- Summon History: X chosen as it is played, 0 or more (LSJ 20100314); the
  search at resolution, library and/or ash heap ([TEM]: crypt and/or ash
  heap), against the printed cost or capacity; the card is put in play, not
  played — no requirement, its own X cost 0 (PIB 20111101); a vampire comes
  with no blood and may act this turn (LSJ 20100204) (77.3).
- NRA PAC's "an equip action" is Hag's Wrinkles' (78.28), one reading for
  both: an equip from hand, Vast Wealth's, an action card or granted action
  equipping from the library (Magic of the Smith, Michael Harris, Dark Web
  Procurer); the move from another minion is not (80.44).

### 3.8 Action modifier, reaction, combat card

- An action modifier is played by the acting minion (some cards let other
  minions of the acting Methuselah: Cloak the Gathering); a reaction by
  another Methuselah's ready minion, unlocked or woken (1.3; rulebook
  "Reaction": 1.3's "anyone" is any other Methuselah). By its text, the
  acting Methuselah's other vampire may play Touch of Clarity (71.26, LSJ
  20090425).
- A dual card's type follows its player (D18): played by the acting
  minion, a modifier — outside combat, by the acting bearer (70.17); by
  another, a reaction (52.2, 71.24, PIB 20150924-2; 71.25: the vampire's
  own dual card is a modifier).
- "Non-action modifier cards" are every card not played as an action
  modifier (70.17). "A non-combat card" is a card that is not a combat card
  (71.26).
- A "strike card" is a card played in a strike mode, a dodge included
  (42.2); a "card played as a combat card" counts the type it is played as
  (62.2).
- A reflex is played by the vampire the frenzy card is played on, locked or
  not (64.8).
- "Action cards" are all the action card types: action, political action,
  ally, equipment, retainer (60.4).

### 3.9 Controller, owner

- The owner is the player whose deck the card came from; the controller
  plays and chooses for it (advanced rules §6.1).
- A minion card on a minion follows the minion's controller; a master card
  on a minion stays its player's (7.5, 38.4, 31.5).
- A burned card goes to its owner's ash heap, a vampire burned under
  another's control included (7.5, 31.6, 31.5).
- Corruption counters belong to the Methuselah who put them (25.1). A card
  stored on another's card stays its owner's (60.19, 67.1).
- "Owner's uncontrolled region" for a temporarily controlled vampire: the
  region of the Methuselah it would return to (27.7).

### 3.10 Ready, ready region, unlocked, torpor

- Ready = in the ready region (not torpor, not uncontrolled, not burned);
  ready is not unlocked (50.5): a ready vampire may be locked.
- "Not ready" = in torpor or burned (31.3); leaving the ready region by any
  route (torpor, burned, controller ousted) triggers "leaves the ready
  region" effects (59.6, 16.5, 33.7).
- "Your ready region" is the controller's (73.7).
- A combatant no longer ready (burned, going to torpor) plays end-of-round
  combat cards only, unless the card excludes it (Taste of Vitae, Disarm)
  (4.5); an acting minion burned (a blood hunt's diablerist) plays nothing
  more (78.50).

### 3.11 "You", "your", unaddressed text

D11; the per-card readings: 32.7, 49.5, 69.2, 71.7, 68.10, 70.2, 73.7.

### 3.12 Cards and actions "requiring X"

- "Cards requiring a Discipline" for a cost or toll: cards played using it
  (D29; 38.4 Enhanced Coagulant, Libertas; 42.5 Ragnar; 41.9 Roger's
  reduction; 60.5 Lord Tremere).
- "A card requiring Dominate" (referendum abilities): any library card
  whose disciplines include it, either-or and combos too (15.2).
- Card targets: "a card requiring a Discipline" = any library card printing
  a Discipline requirement (17.4 Aniel).
- "An action requiring a Gangrel / Hecata or Oblivion" (Keegan, Anja): an
  action card, a retainer or ally card, or a card in play's granted action
  requiring it — never a rulebook action (41.12).
- "Actions requiring an Anarch" (CrimethInc., An Anarch Manifesto): the
  rulebook action to become Anarch; an action card requiring an Anarch or a
  baron or making a vampire Anarch; a recruit, employ or equip of a card
  requiring an Anarch; an action a card in play grants Anarchs only — not a
  card another card fetches in the action (62.21, 71.14).
- "An ally requiring a Gangrel": printed with the Gangrel clan requirement
  (59.5). Cappadocian Crypt: an action whose card requires Hecata or
  Oblivion at the level played, or granted by a card requiring a Hecata
  (34.6).
- A Sabbat path icon is a requirement, as a clan icon (49.7).
- "If the referendum requires a titled vampire" reads any title requirement
  on the calling card or granted action (63.3).
- Tremere Convocation: every Methuselah's Tremere, +1 stealth on actions
  requiring Blood Sorcery; basic Blood Sorcery plays its combat cards at
  superior; its Ⓓ burn action at -1 stealth for a Malkavian (that clan)
  (80.33).

### 3.13 Younger, older

By capacity (D30; rulebook): 40.11, 41.14, 42.4 (never an ally, never
Mithras himself), 63.4 (equal is not older; against an ally, neither),
60.18 (the burned vampire's capacity in game vs the drawn card's printed
one), 26.8 (younger than the acting vampire), 60.11, 60.13, 71.1.

### 3.14 A cost, an effect, a toll

- A card's cost is its printed blood or pool cost, modified (D20). A "burn
  n blood" in its effect is not (62.2, 63.5).
- An action's cost (rescue's 2 blood, become Anarch's, Daria's and
  Marcellus's +1, Ambrosio's +1 pool, The Red List's 1 blood, Eulogio's 1
  blood) is paid at resolution, offered only if payable as announced (3.6,
  42.1, 62.7, 69.8, 67.5, 55.3, 58.1).
- A block toll (Sergio Bueno, Mithras, Seeds of Terror, Tenebrous Form) is
  paid to attempt; an ally pays in life where the card allows (41.4, 53.2,
  69.12, 42.8).
- Cost alternatives and reductions: §7.3. A gift is not a cost (45.3).
- A burn of the host's "blood or life" is one effect: blood for a vampire,
  life for an ally; as an in-play ability's cost, "burn 1 blood" is a blood
  cost an ally never pays (73.5, advanced rules §5.5.1).
- A payment named as the card is played (the removal of two copies,
  Trochomancy's ash heap) is the play's price: not playable without it
  (59.1, 67.7).

### 3.15 A title; titled

- Titles: printed, granted by a title card, or "n votes (titled)" (a title
  of its own, not unique, never contested, removable: 58.3). Primogen is a
  title (13.2). The +1 vote of Ephor is no title (59.13).
- "Titled" reads the effective title: a contested or yielded title, or an
  off-sect one, does not count (13.2, 59.7, 59.11, 59.19).
- "Non-Anarch titles" (Fee Stake) are every title but baron (32.9).
- Black Hand is a trait, not a title nor a sect: printed, or gained in play
  (Mateusz Kazimirski), kept through a sect change (LSJ 20070322, ANK
  20180807). Seraph is one too, not a title (78.1).
- A contract is any card whose type line says so (Priority Contract,
  Clandestine Contract): The Black Throne pays for each (74.12).
- A justicar requirement met by Karl Schrekt makes him a titled Camarilla
  vampire for that card (59.14).
- A title card replaces a printed title for good (19.2).
- No contest with oneself: a title card (and Zane's referendum) cannot
  choose a vampire whose controller already claims that title (76.2, 76.3).
- Shakar: the Hunt: the contract's combat against another Methuselah's
  minion only (17.3); "every Black Hand vampire is considered chosen" read
  by no pool card beyond its own action (inert); Gerald FitzGerald's Black
  Hand kept through a sect change (LSJ 20070322), his Seraph read by Tattoo
  Signal (78.30) (77.31).

### 3.16 "The same action"

The NRA identity (1.4): card from hand = the card's name, per minion; card
in play = the copy; each action a card in play grants is its own (32.1);
rulebook actions by kind and target for card texts (23.2). §8.5.

### 3.17 Equipment, weapon, gun, vehicle, haven

- Weighted Walking Stick and Zip Gun in play are weapon equipment (22.4,
  64.10); a location is not equipment (17.4: a locquipment is a location).
- "Ranged weapons" are weapons with a ranged strike (64.2); a gun by its
  type line (Zip Gun: Kevlar Vest, 64.10); "non-weapon" excludes melee
  weapons (17.4).
- "Only one vehicle" / "only one haven": never a second, by any route
  (9.2, 17.4, 63.10, 70.20, 51.3).
- Put on (Alastor), not equipped: its layers, inferior, requirements
  ignored; nothing it does "as equipped", no lock (76.15).
- A minion barred from using equipment loses the Disciplines its equipment
  gives meanwhile (76.16); an action an equipment grants its bearer fizzles
  once the equipment left the actor (76.9).

### 3.18 Ash heap cards, card kinds

- "Cards in your ash heap" are all its cards, burned vampires included
  (42.7, 67.7).
- Library card kinds: 60.3 (minion card), 60.4 (action cards), 70.17, 71.26.
- An ally with no card (a token) costs 0 (71.2).

### 3.19 A frenzy card "played on"

On the minion it targets, selects, chooses or affects; a card played on
one's own side of a combat has the opponent as the frenzied one (28.11,
LSJ 20051113, ANK 20220409): Terror Frenzy is played on the opposing
minion. Its strength bonus is a frenzy effect like the rest (70.16).

### 3.20 Damage "successfully inflicted"; strike damage; environmental

- Successfully inflicted = landing after prevention (29.3); Weather
  Control's before-range damage is successfully inflicted and so the round's
  first (29.4).
- Damage from a gun strike is "damage from gun strikes"; gun damage not
  from its strike is not (10.3).
- Environmental and retainer damage is not the opposing minion's strike
  damage (21.3, 62.24, 59.10).
- "Damage X inflict(s)" is X's own damage, a strike or not (Pulled Fangs'
  1 damage), never environmental or a retainer's (74.6, Saulot). Pulled
  Fangs' damage is preventable, not strike damage (74.1).
- Prevention "from the opposing minion's strike" (Rego Motum, Opikun,
  Lodin) covers that strike's damage only, never environmental or effect
  damage beside it (Carrion Crows, Pulled Fangs: 74.1), and is not offered
  without strike damage pending (LSJ 20001114).

### 3.21 The acting minion, the opposing minion

The acting minion is the minion performing the action (a blocked action or
an unblocked rush); the other is the opposing minion, a "blocker" only if it
blocked (1.2). "Considered the acting minion" (Deep Song) moves the acting
side, not "this acting vampire" effects (27.5). A combat with no acting
minion: 1.2. §12.1.

### 3.22 A replacement

Every card the rules replace counts, whatever took it out of hand (played,
canceled, discarded, burned for a ballot, a delayed replacement coming due),
The Barrens' draw-up and Constant Revolution's refill included; a draw-up
after a hand size increase is not (8.3). §19.1.

## 4. Card text and how it reads

Judge-level reading of card text: advanced rules §1.1 (mandatory and
optional effects, "cannot"), §1.2 (wording templates), Appendix A (glossary
of wordings).

### 4.1 Addressing and scope

- "You" / "your" = the card's controller (D11; 32.7, 73.7). Open War's 4
  pool to its controller, not the Methuselah moving the fourth counter
  (32.7). Jerry's and Redistribution's "the Blood Brothers in this circle":
  its controller's (68.10). The Status Perfectus's "a blocking anarch",
  "another ready anarch": the player's (71.7). Maila's "a minion you
  control": any minion acting for her Methuselah, herself included (57.2).
- In an action's modifier or reaction text, "you" is its player; for
  Enchanting Gaze the acting Methuselah (25.5).
- Unaddressed text reaches every Methuselah's cards: Monastery of Shadows'
  +1 stealth to any vampire of capacity ≥ 8 (49.5); Club Illusion's
  Anarch, any Methuselah's, burns its own blood; Anarch Railroad, Fortune
  Teller Shop, Warsaw Station are locked by their controller for another
  Methuselah's acting minion (28.2); The Eldest Are Kholo's votes, Palla
  Grande's bleed (69.2); Victoria Ash's acting Banu Haqim (70.2);
  Terrifying Visage's "a ready vampire" (53.7); Charisma's ready vampire
  (33.3).
- A card does what it says: Vessel's "burn a card named Blood Doll (if
  any)" is any Blood Doll in play (6.7).

### 4.2 Periodicity: during, after, each, use an MPA

The principles are D7 (during), D8 (after), D9 (use an MPA), D32 (once each
turn / combat / round); advanced rules §1.2.1. The limit is per copy, per
vampire. The per-card readings sit with their mechanism; these state only a
limit:

- Once per action: Bowl of Convergence (9.6), Gathii (15.1), Abraham DuSable,
  Oskar Anasov, Gwenllian Arwyn, Larissa Moreira (15.2), Richard de Worde
  and Alicia Cortez "after" (16.1), Marchesa Liliana (42.3), Bone Shambler's
  two abilities (42.9), Camarilla Conclave (51.2), Under Siege (52.8),
  Victoria Ash G7 (70.2).
- Once per referendum: Aisha az-Zahra, Alexa Draper, Yewon Ong, Queen Anne
  (15.2), Fakir al Sidi (59.18).
- Once per phase: Blood Doll (master), Vessel (unlock) (6.6); Shilmulo
  Tarot's move (11.2); Fleshforge Chamber (37.1); Ravnos Cache (36.3); Nonu
  Dis (15.2, ANK 20220617); Mora (15.2); Luciano Carvalho (41.7); Gilbert
  Duane (44.5); Sri Sansa (41.8); Scout Youngwood (58.9); Illusory Resources
  (63.14); Powerbase: Barranquilla's unlock move (62.25); Jerry (68.10);
  The Status Perfectus's move (71.7); Venere Carboni (71.27); Owain Evans
  (62.11); Keith Moody (62.10); Giotto Verducci (71.18).
- Once per turn: Kalinda (15.2); The Medic (58.7); Lord Tremere (60.6,
  once in his minion phase); Alexandra (62.3); Ilonka (16.3); Aline Gädeke,
  whoever's turn (16.4); Alabástrom (16.6); British Museum per minion
  (35.3); Roger de Camden, Tommaso Sforza (41.9, 41.10); Maila (57.2);
  Giovanni del Georgio (59.1); Randall (59.8); Aksinya Daclau, once in each
  Methuselah's turn (59.16); Blackhorse Tanner (62.12); Nergal (60.7);
  Street Cred per vampire (60.11).
- Once per combat: Abaddon's, Saku Pihlajamäki's maneuver or press;
  Flávio Gonçalves', Phaibun's dodge (14.2); Righteous Blade's press per
  Blade (10.4); Gabriel Tremblay (57.3); Lodin (59.10); Hell-for-Leather
  per Anarch (44.1); Omaya (62.17); Nephandus's press (62.24); Ruins of
  Ceoris (62.27); Contagion's [DAI] strike (62.19); Jacob Fermor's press
  (64.1); Renegade Garou's maneuver (63.8); the Great Beast's prevention
  (60.20); Focus the Blood, one play per vampire (63.7).
- Once per round: Rowena (14.2); Valerie Diaz (38.8); Quick Meld's maneuver
  (59.2).
- Independent uses: Carfax Abbey's two picks are two once-per-phase uses,
  the second takeable without the first (7.4); Gregory Winter's two actions
  do not restrict each other (63.16, LSJ 20100909).
- "Use a master phase action to …": as often as MPAs are spent — Open War,
  Twilight Camp (36.4).
- Phase dues: D10.
- Asguresh: once each combat (D7), against a card played as a combat card
  (D18) by the minion opposing him; Thucimia: once each bleed action of hers
  (D7) (74.10).

### 4.3 Live reading and fixed amounts

- D13. Plain printed bonuses are read live while the vampire is in play,
  in torpor too (LSJ 20011022), not while contested (13.2). Vote grants
  from cards in play are read live: a vampire that already cast adds the
  vote to its cast (19.3). Raising the Portcullis is read live as the
  prey's vampires enter or leave torpor (39.5); Tier of Souls' "your prey"
  after an oust (32.4); Aire of Elation, Protection Racket, The Warrens
  (18.4).
- A limited bleed counts toward the limit only while its condition holds
  (Foreshadowing Destruction, 18.4).
- Fixed by the card's words: Night Terrors' -X (RTR 20030519, 18.4);
  Visions of Zapathasura's "reduce … to 0" is a reduction equal to the bleed
  as it resolves, later increases still counting (23.5).
- Fixed when the card says when: Cold War's title "as the terms are set"
  (52.5); Gerald Windham's titles "when votes are counted" (59.19); Justicar
  Retribution's bleed "as the referendum passes" (59.9).
- Read twice where the card is read twice: Malabranca's "if your prey has
  more pool" as announced and as resolved (70.21); Fiendish Tongue's clan as
  the bleed succeeds and again in the discard phase (54.5).

### 4.4 Requirements

- A requirement is checked as the card is played and, for an action,
  continuously until it resolves: lapsing mid-action, it fizzles (advanced
  rules §1.6.1, owner adjudication). A card already in play stays when its
  requirement lapses; a trait gained later reaches only what checks it
  continuously.
- "Only usable by / when …" restricts the play of a played card (actions,
  modifiers, reactions, combat cards) and the conferred ability of a
  permanent, which can still be put in play (advanced rules §1.6, owner
  adjudication; Seal of Veddartha, Agate Talisman LSJ 20030607).
- Text naming the bearer's clan or sect reads the current bearer (D31; 9.1:
  Treasured Samadji, Stolen Police Cruiser's "this Anarch", Shilmulo
  Tarot's "while this Ravnos is ready", 12.5; Zaire River Ferry's "this
  Lasombra", 70.20).
- A Sabbat path icon requires a vampire following that path: for a minion
  card, the one playing it; for a master card, a ready one its Methuselah
  controls — as a clan icon does. Path is read live (Black Chantry
  2025-06-12: one path at a time, benefits only while Sabbat; an
  influence-phase "burn 1 blood to follow a path" for a Sabbat vampire
  following none) (49.7).
- Meeting a requirement one lacks (advanced rules §1.6.4):
  - Abraham Mellon plays cards requiring a Tremere as one, only for the
    cards he plays, played normally (LSJ 20050119); not his Methuselah's
    master cards requiring a ready Tremere (41.1).
  - Lady Scarlett Churchill: her Methuselah's cards requiring a Toreador
    count her as one to be played (Art Museum, Society Hunting Ground,
    Toreador Grand Ball's locked and second Toreador); the card's effects in
    play never do (41.2, ANK 20220816-2).
  - Karl Schrekt meets a justicar requirement on the cards he plays, never a
    master card's (Camarilla Conclave), whatever his sect; a title
    requirement met this way makes him a titled Camarilla vampire from the
    card's point of view (Camarilla's Iron Fist even when no longer
    Camarilla; owner 2026-10-09 over ANK 20250410) (59.14).
  - An ally playing as a vampire: D16.
- Mozambique Allure's "Requires a Laibon or non-titled vampire": a Laibon
  (titled or not) with a crypt card (LSJ 20070411-2), or a non-titled
  non-Laibon; becoming Laibon replaces the sect (69.14).
- A Seal or requirement on a moved card is not rechecked (17.4, 31.4).
- Vlad Tepes meets every sect and title requirement of the cards he plays or
  uses (ANK 20201028), the in-play effect of such a card too (PIB 20150306),
  never a Black Hand requirement (a trait, §3.15); each card reads the sect
  or title it lets him pass as, its referendum terms included (77.2).
- My Kin Against the World: a Laibon acting vampire may choose younger
  Laibon of any clan; Abombwe goes on a Laibon or a vampire with Protean —
  their Laibon clauses are live, Laibon being in the pool (77.34,
  corrected, amending 63.16).

### 4.5 Discipline layouts and modes

- "[dom][pre] X" side by side is a combo: both needed (Iron Glare, Absolute
  Tyranny); separate lines are a choice of one Discipline at one level (Line
  Brawl); "more than one Discipline can be used" allows any non-empty
  subset, each effect applying (Break the Bonds, The Platinum Protocol, LSJ
  20011204-3); Outside the Hourglass: a choice with an "outferior" option
  (25.2).
- "[pot] or [pre]" on one line (Sabbat V5) is either Discipline at that
  level, one mode (52.1): Dominant Personality, Magnetic Authority,
  Relentlessness, Unthinkable Humiliation.
- One mode per set of Disciplines used (Memory Rift, 62.23). With [obf] and
  [pre] together, effects apply in printed order (25.6).
- "Or" in a card's effect is a real choice, each option on its own
  legality (Revelation of the Serpent: the bleed option always, the stealth
  option only when needed, 25.3).
- Several plays of one card: I am Legion's three plays (60.15),
  Groundfighting's maneuver, press and cancel (71.6), Predator's
  Transformation's reflex (64.8). "As above, with …" inherits the mode's
  timing, except that "as above, with +1 stealth" over a mode with no
  printed timing keeps the stealth window alone (Mind of the Wilds
  [ANI][AUS], 64.11); a mode printing no "as above" does not inherit (64.9: Read the
  Winds [ANI][AUS] locks as usual).
- Dabbler counts the distinct Disciplines of every card the vampire played
  during the action (action card, reactions, the action's combat cards),
  each Discipline once whatever its level (34.3).

### 4.6 Bars, immunities, "cannot"

- "Cannot use equipment during the resulting combat" bars all equipment use
  (Form of the Bat: no passive combat bonus without a strike, 18.3);
  Conquer the Beast's vampire for the whole combat (59.17).
- "Cannot maneuver": no maneuver at all, granted ones included, for that
  combat (70.12).
- "It is now unblockable": no minion may attempt to block it for the rest of
  the action (70.13).
- "Do not affect this vampire" (Perfect Clarity): no reaction requiring
  Dominate or Presence may be played during the action (each would affect
  it, LSJ 20010621), except in a referendum's polling step, where such a
  reaction played by another vampire does not affect it (Scalpel Tongue may
  not choose it; Absolute Tyranny's -1 vote skips it); [THA] reaches the
  minion opposing it in any combat of the action; the vampire's own dual
  card is a modifier, not shielded (71.25).
- "Take actions to enter combat with them" bars the vampire's action cards
  entering combat with such a target, read live; combats it does not choose
  are not barred (Reginald Moore, 58.6).
- Depravity bars recruit and employ action cards played by its vampire,
  nothing else (54.8). Approximation of Loyalty [PRE]: no card played as an
  action modifier by the acting Methuselah's minions for the rest of the
  action (70.14).
- Karen Suadela's Gehenna immunity reaches every Gehenna surcharge or bar:
  Torpid Blood, The Slow Withering, Blood Weakens, Wormwood, Thirst,
  Restricted Vitae; read live on her claimed prince title (58.10).
- Stanislava: retainers employed by a minion opposing her in combat lose
  their card text for that combat; still retainers whose life takes damage
  (49.3).
- Immunity is not prevention (38.3, §12.8).

## 5. Timing: impulse, windows, ordering, duration

Advanced rules §2 is the normative source (§2.1 impulse and step windows,
§2.2 as announced, §2.3 after resolution and after combat, §2.4
simultaneous effects, §2.5 duration). The engine's window primitives are in
`docs/engine.md`.

### 5.1 Impulse

- Impulse-reset: any play returns the opportunity to the side or
  Methuselah with priority at the same step; a later play voids prior
  passes; the step closes when all decline in succession (1.2).
- Every phase of the active Methuselah's turn ends on an impulse rotation
  over the other seats: their mandatory effects first (The Coven's
  transfer, LSJ 20100723), then untimed abilities; an effect returns the
  impulse to the active Methuselah; the phase ends once all pass (7.9). The
  unlock phase: 3.7. The minion phase: between two actions the active
  Methuselah holds the impulse first; passing it declares the end of their
  minion phase, then the others may use effects (The Louvre, ANK 20210608)
  but not start an action, except Enkil Cog's; an effect used returns the
  impulse to the active Methuselah, whose phase goes on (51.4, 49.6).
- Untimed abilities ("lock this card" with no printed timing: The Barrens,
  Dreams of the Sphinx, Ennoia's Theater, Maabara, The Erciyes Fragments,
  Zizi; Disarm's "burn 3 blood to burn this card") are usable whenever their
  controller holds the impulse — all own phases, every seat's phase
  rotation, every action and combat window, polling (as a commitment), on
  any Methuselah's turn, in torpor for a printed one — never in an
  as-played window (LSJ 20061207) nor inside a choice that is part of a
  resolution (terms, rider payments, the diablerie choice, the rescue cost
  split, a discard-down) (7.7, 29.10, 44.2, 60.19, 68.16, 71.12). One may
  be used before a forced hunt (The Coven sparing it, 7.7).
- Combat cards with no printed step have one untimed combat window, with a
  deadline derived from their effect: before the step they modify (2.2,
  advanced rules §2.1.4): Claws of the Dead, Wolf Claws, Pack Alpha, Monstrous
  Form's strength (26.7), Bone Spur (64.3, RTR 19960112).
- In combat with no acting minion (Taunt the Caged Beast, Illusions of the
  Kindred, Siren's Lure, the Slave rule), the first opportunity goes to the
  acting Methuselah's minion, else clockwise from the acting Methuselah (1.2).
- A minion outside a combat is not offered its abilities there unless they
  say so (15.2: Kalinda; 59.8: Randall's step for minions out of the
  combat; 41.10: Tommaso Sforza).

### 5.2 Windows

- **As played**: opens after every card play, in an action or not; admits
  only wakes and cancellations — not card abilities (2.4). Wakes need an
  action: an action card just played has opened it, and a wake there
  resolves before the played card's effect; outside an action, cancels
  only (2.4, Rewind Time usable outside actions). True wakes (Wake with
  Evening's Freshness, On the Qui Vive, Forced Awakening, Eyes of Argus
  [AUS], Shadow Sentinel) are; unlock-and-attempt reactions (Dogged
  Pursuit, Eyes of the Wild, Second Tradition's 2nd mode, Sense the Savage
  Way [ANI], One With the Land [PRO], Organized Resistance) and plain
  unlock reactions (Guard Dogs, Rat's Warning, Forced Vigilance [FOR]) are
  not (2.4). A card played not in the normal fashion opens no as-played
  window and cannot be canceled as played (advanced rules §1.8.2).
- **As announced**: holds only effects whose text says so ("only usable as
  the action is announced", "as … announces": Seduction, Warsaw Station,
  Yoruba Shrine, Screamer's fail); an effect usable "during the action"
  (Screamer's and Aranthebes's -1 stealth, Haqim's Law: Retribution's +1
  bleed) waits for the block attempts (66.1, advanced rules §2.2). Action
  cost reductions may be announced there (36.1).
- **Blocks declined → would be successful**: once blocks are declined, a
  bleed of ≥ 1 opens a "would be successful" window over the action's order
  (acting Methuselah first) for cards timed "if a bleed would be
  successful" (Spying Mission), none once the action has failed; after a
  canceled Spying Mission only such cards follow, never a reaction (24.2,
  LSJ 20050607). Every political action has one too (Yawp Court, The Line,
  Ravnos Cache; 54.1). Major Boon played or burned closes the
  blocks-declined window: only "would be successful" plays follow (68.13,
  LSJ 20090205). §10.1.
- **After resolution**: every action has one, blocked, failed or ended
  included (23.3, 5.8; exceptions §3.2); for a political action after the
  referendum; after a diablerie, after the blood hunt (5.8). §10.4.
- **After combat**: opens once, after the last combat arising from the
  block — "after block resolution" plays (Cats' Guidance [ani], Forced
  Vigilance [for]) are after-combat plays, after all combats of the action
  (2.1). §12.12.
- **After a diablerie, before its blood hunt**: Slake the Thirst is offered
  there (68.2, ANK 20201228); an older victim's master Discipline is
  asked before that window (81.1), the library shuffled even when it
  takes nothing or is declined (86.1).
- A card played during polling is a commitment of the rotation: it reopens
  a response opportunity for every other Methuselah before the tally
  (5.3). §13.3.
- Out-of-turn is a card property (Archon Investigation, Sudden Reversal),
  not a window (2.3).
- Strike-card riders: the rider's timing relative to the strike step
  decides the play window — a strike with an optional maneuver rider (Aid
  from Bats, Diversion) is playable at determine range, committing the
  strike early; one with a press or additional strike rider is played at
  strike declaration (Hidden Strength's press is a rider; Flash,
  Indomitability, Resist Earth's Grasp have real "or press" modes); riders
  firing after the card's own play step add no window (2.5). Slam [POT]:
  48.4.

### 5.3 Mandatory effects and their order

- A mandatory effect resolves before its controller passes the impulse;
  its controller orders it among their other effects of the step (6.1,
  advanced rules §1.1.2, §2.4). Unlock-phase dues (Anarch Revolt's burn,
  contest upkeep, Gnaeus Aemilius Augustinus, Contagion's pool, Web of
  Knives Recruit's counter) are ordered freely by the active Methuselah with their other unlock
  effects (6.1, 7.8, 41.11, 62.19, 69.6); the condition is read as the effect
  resolves (6.1).
- Other phases likewise: Frontal Assault's influence-phase burn (28.9), The
  Uncoiling's (40.5), Dragonbound's (57.5) discard-phase burns, Brujah
  Debate's master-phase lock (33.1, 60.22), Puppet Master's "during your
  next minion phase, burn this card to …" — any point of that phase, which
  cannot end before it (27.1, LSJ 20001201).
- Between two mandatory actions (Elen Kamjian's bleed, a mandatory hunt)
  the acting Methuselah chooses the order (17.2).
- "Instant" riders resolve inside what triggered them: Fame's 3 pool
  inside whatever sent the vampire to torpor, mid-combat, before the
  after-combat window, ordered with Tension in the Ranks by the controller
  (6.4).
- After-resolution effects are ordered by their player among their
  after-resolution effects before passing: Daring the Dawn's damage (23.3,
  LSJ 20010715); Forward Momentum's counters with its unlock — after "you
  perform an action" and "after the bleed is unsuccessful" are one window,
  the controller choosing the order (53.6).
- Order is a real choice (advanced rules §2.4.2); effects not ordered:
  advanced rules §2.4.5 (unlocking at the start of the unlock phase, 7.8).

### 5.4 All at once

- What a card does at once is not sequenced: Baltimore Purge's torpor
  (63.13, LSJ 20050111); Constant Revolution's random discards (7.10, LSJ
  20090618); Business Pressure's pool burn and votes once all chose (51.5);
  Marcos Belegrad's burns at the tally (51.1); Golconda's removal with its
  cards (33.5); Heart of Nizchetus's draws (11.1).
- Pool losses in one batch oust together (45.1, 69.10, §21.1).
- A cost paid with several counters is burned all at once (Cave of Apples,
  35.2).

### 5.5 Duration

- Bonus stealth and intercept last the whole action and end with it, a
  reaction card's included (Eyes of Argus): kept across a failed attempt
  into a new one (15.2, advanced rules §3.2.3).
- "Until your next discard phase" ends as that discard phase begins
  (53.4); "until the end of turn" bonuses end then, not in the discard
  phase (8.2, LSJ 20100812); Juliet Parr's until her controller's next unlock
  phase begins (60.2); "until their next master phase" as that master
  phase begins, through the unlock phase before it (Camarilla Vitae Slave,
  76.24); Rage of Apedemak's for the combat (54.8); Make the
  Misere's [pot] for the whole combat (31.7); Coagulated Entity's capacity
  until the action ends (68.6).
- "Once this combat" with no "put this card on": the card goes to the ash
  heap, its effect on the vampire for the combat, gone as it ends if unused
  (Inner Essence, 68.12); Focus the Blood's copies, put on the vampire, stay
  for later combats (63.7).
- Effects outliving their source: a lasting effect applies though its card
  went to the ash heap (21.1); advanced rules §2.5.3. A permanent hand size
  gain stays after its source leaves (Benjamin Rose, 58.15).
- A hand size ending or dropping discards down at once (8.2, 53.4, 54.8,
  13.2). §19.3.

### Open or contested

- 54.1 vs 23.3: an action Yawp Court's combat ends is "a failed action, with
  no after-resolution window", while 23.3 gives a failed action one (ANK
  20221205); read 54.1 as the narrower, card-specific word (§23).

## 6. Turn and phases

### 6.1 Unlock phase

- Unlocking the active Methuselah's cards happens at the start of the
  phase, not ordered; then the active Methuselah does all the unlock-phase
  effects they want, in the order they want — contest upkeep (pay or yield,
  mandatory), mandatory riders, the Edge's pool (optional; passed and not
  taken, lost), abilities — then passes the impulse so others'
  unlock-phase abilities can be used ("during any Methuselah's unlock
  phase": Homunculus); a use returns the impulse to them (3.7, 7.8, 6.1).
- Other Methuselahs' abilities "during each of their unlock phases" or in
  any unlock phase: Life Boon's steal, once per unlock phase of theirs, any
  time among their effects (45.6); Owain Evans, in every unlock phase (his
  controller's too), any vampire of the active Methuselah, offered once the
  active Methuselah's mandatory effects are resolved (62.11); Keith Moody,
  offered to his controller's prey, holding the Edge, as the impulse reaches
  them in his controller's unlock phase; once that phase; not with Keith at
  0 blood; Keith in torpor included (62.10); the burn option, usable in any
  Methuselah's unlock phase (33.8, §16.8).
- "Instead of unlocking as normal" is a substitution (advanced rules
  §5.2.2): Touch of Oblivion's minion burns a counter even if already
  unlocked; one kept from unlocking by another effect (Stolen Police
  Cruiser) burns none that phase; at 0 counters the card burns and the
  minion unlocks normally from the next phase (21.2). Stun, infernal: §14.1.
- "If your prey controls no vampires in torpor … burn this card" (Augury of
  Doom) is read in its controller's prey's unlock phase, instead of the
  burn; played while the prey has none, it waits for that phase (54.2).
- Readiness for an unlock-phase effect is read as it resolves (Gnaeus
  Aemilius Augustinus; the prey chooses as it resolves among their ready
  minions, an ally taking the damage in life; none: nothing) (41.11).
- Unlock-phase moves "once each unlock phase": §4.2. Tabitha Fisk, as Silvian
  Popescu: her controller's unlock phase, once, from an archbishop of
  theirs with blood, she in torpor too (65.3).
- 419 Operation: one of its two unlock-phase moves each phase (6.6); the
  prey's Edge burn is the prey's at any impulse, with no counter on it too
  (ANK 20221011-3); the counters go to whoever is the controller's prey then
  (77.17).
- Feo Ramos: in his controller's unlock phase, each older Sabbat vampire of
  another Methuselah may burn 1 blood to lock him, once each (6.6), he
  locked or in torpor too (D25), Mustafa's shape (78.8).

### 6.2 Master phase

- The master phase is its Methuselah's offer window, punctuated by cancel
  windows and closed by an impulse rotation over the other seats (their
  untimed and mandatory effects, 7.9). Other Methuselahs may only cancel in
  the as-played window and play out-of-turn masters; no wakes without an
  action (1.5).
- Out-of-turn masters (Archon Investigation, Sudden Reversal, Life Boon)
  count toward the player's next master phase; an out-of-turn trifle also
  consumes that next master phase action and gives the extra one only when
  the player reaches their master phase. A player never plays two
  out-of-turn cards between their turns (3.3, 45.3), a second master phase
  action or not (62.5).
- Extra master phase actions: Anson gives a second while he is ready, read
  live — the ordinary action spent first, the second lost if he leaves the
  ready region before it is used; a trifle or another extra adds to them
  (62.5). Rumors of Gehenna's "two master phase actions (instead of one)" is
  the same second action as Anson's: neither another copy nor Anson adds a
  third; the Methuselahs it chooses in its terms (the caller may choose
  themselves) are remembered by the card (70.1).
- "During each Methuselah's master phase" mandatory effects: D10 (Brujah
  Debate, 33.1, 60.22: one lock per copy, ordered among that phase's
  mandatory effects; "the oldest Brujah they control" counts torpid ones).
- Judgment: Camarilla Segregation's burn: any Methuselah, in their own
  master phase, no master phase action spent; a vampire in torpor
  qualifies (32.3).
- Nonu Dis: once per master phase, any time in it once a master card was
  played (as announced, canceled or not); any ready Ministry vampire of his
  controller's, himself included (15.2, ANK 20220617).
- Master cards on minions: §16.6; master card "burn option": §16.8.
- Cybele: +1 master phase action if unlocked as her controller's master
  phase begins, read then, in torpor too (D25); her +2 bleed action costs 1
  blood at resolution and is her one bleed of the turn, the rulebook's
  included (1.4) (78.10).

### 6.3 Minion phase

- The impulse between actions: 51.4 (§5.1). The minion phase ends when its
  Methuselah passes and every other passes; Enkil Cog's action ("during any
  Methuselah's minion phase") is offered when the active Methuselah declares
  the end of their minion phase; if one locks the Cog, that vampire takes
  its action, then the impulse goes back to the active Methuselah, whose
  minion phase continues (49.6). Not offered in its own Methuselah's minion
  phase (50.1); only while its vampire has an action to take: ready,
  unlocked, some action legal (50.2); its "successful bleed against your
  prey" reads the target after resolution (50.3, 61.2).
- "During your minion phase" means any time in it, during another action
  too, its combats included (26.5: Rutor's Hand unlocking one to play Veil
  the Legions; Slaughtering the Herd's burn).
- Mandatory actions are ordered by the acting Methuselah (17.2); a
  mandatory bleed (Elen Kamjian) holds the phase until taken (17.1); §8.7.
- "Not usable if any non-mandatory actions have been performed this turn"
  (Khabar: Glory): any action of the player's minions already announced
  this turn (blocked or fizzled too), except a must-hunt hunt or a printed
  mandatory action (32.10).
- After the normal end of his controller's minion phase, every Tupdog of
  theirs burns, in torpor too, each moving one crypt card to the
  uncontrolled region (an empty crypt: none) (62.14).
- A bleed ousting the active Methuselah (another's minion in their minion
  phase, Enkil Cog): the turn passes with the action (58.20).
- César Holfield: at the end of his controller's minion phase, read then — a
  Gehenna event in play (any Methuselah's) and no hunt by him this turn (a
  card hunt or a blocked one counts, 72.1, 40.2): 1 blood, none at 0; in
  torpor too (D25) (78.11).
- Madness Network: any Methuselah's unlocked Malkavians (the clan only) take
  actions in another's minion phase, as that Methuselah ends it (Enkil Cog's
  moment, 49.6), in torpor too to leave it (RTR 19950509), a hunt with no
  blood (LSJ 20060508); no Mask of a Thousand Faces over such an action (LSJ
  19981007); any unlocked Malkavian, a woken one too, may block its burn
  besides the eligible blockers (LSJ 20041022-2) (78.48).
- Qawiyya el-Ghaduba: 2 blood to unlock once in her controller's minion
  phase (ANK 20190207), any time in it, in torpor too (D25), offered only
  while she is locked (48.7) (80.8).

### 6.4 Influence phase

- Transfers: 4 a turn (fewer on the first turns, rulebook); merging costs 4
  transfers and 1 pool (69.5, §14.9). "Lose all transfers" (Leandro): none in
  the next influence phase, bonus transfers included; the choice is the
  unlocking Methuselah's, both offered (a pool burn may oust) (71.19). The
  Capuchin's +3 transfers are his controller's at the burn, at the start of
  their next influence phase (60.8).
- Influence-phase abilities: Gilbert Duane's (44.5); Antonio d'Erlette's
  search for a mage ally: 4 of the phase's transfers, Antonio ready, his
  library and hand searched at once for a mage ally whose clan requirement
  he meets and whose cost is payable — reduced as LSJ 20090920-2 rules,
  except by a reduction keyed to an action (Charisma, 36.5); library
  shuffled either way; a pick from hand replaced; Veneficti (Baali) is
  never his (62.13). Whether that ally counts as recruited is moot: it
  cannot act that turn anyway (62.13).
- Frontal Assault's influence-phase burn: mandatory, resolved before
  passing the impulse but freely ordered; the prey's ready minions counted
  as it resolves (28.9).
- Twilight Camp: its blood as counters; "after you put a … vampire in play
  from your uncontrolled region" = when influenced out; the new sect lasts;
  "an Anarch" for its master phase action: any Methuselah's (34.7).
- The uncontrolled region: §14.6.
- Len Konopka's +1 transfer read as the influence phase begins, none from
  torpor (80.12). Nosferatu Kingdom: the influence-location family, a
  Nosferatu antitribu only (80.19).

### 6.5 Discard phase

- One discard phase action by default; events take any discard phase
  action, ordinary or not; one event per discard phase (40.1, §20).
- Extra discard phase actions: Sreelekha's if ready as the discard phase
  begins; Ayse Dhanial's uses each spend a discard phase action (twice with
  Sreelekha's), not offered at capacity (15.2); Unmada's, if she is ready as
  the phase begins, hand size read live (62.15). Powerbase: Los Angeles's:
  the player spends whichever action serves best (28.12).
- Mora, the Death Seer: once per discard phase; any library card in the ash
  heap, chosen by name (15.2).
- Roy ("if unlocked during your discard phase, lock him"): applies as the
  discard phase ends, in torpor too (16.5).
- "During another Methuselah's discard phase" (Giotto Verducci): in that
  phase's rotation, after its Methuselah passes; once in each such phase;
  the card replaced at once (71.18). Josef von Bauren: in torpor too; the
  Methuselah discarding draws back up (71.8).
- "During your next discard phase": this turn's when the card is played in
  one's own turn (Fiendish Tongue, 54.5; Mirror Walk's replacement, 8.4;
  Antonio Delgado's unlock, that turn's discard phase after his referendum
  passed, 69.9).
- Mandatory discard-phase effects: The Uncoiling (40.5), Dragonbound
  (57.5), Scourge of the Enochians — played in its controller's discard
  phase, usable then, and if not, the predator takes it as that phase ends,
  as The Coven (57.6).
- Emily Carson: offered only while she is locked (Kalinda's precedent,
  48.7); once each discard phase (76.21).
- Saul Meira: once each discard phase (6.6), 1 blood to unlock another ready
  Black Hand vampire of any Methuselah (D11), Tattoo Signal's reach too
  (78.30) (78.6).
- Engling Fury: 2 blood; [spi] the unlock in this turn's discard phase
  (Feral Hound's); [SPI] the unlock at once (78.25).

### 6.6 End of turn

- "Until the end of turn" effects end at the end of the turn, not during
  the discard phase (8.2, LSJ 20100812); Loup burns at the end of the turn
  unless a Toreador's action succeeded first (51.7).
- "Your next turn" is the one after the turn the effect resolves in
  (Ayelea's control until the end of your next turn, 69.3).

## 7. Playing cards, cancellations, costs

Advanced rules §1.7 (costs and payment), §1.8 (playing and canceling).

### 7.1 Playing a card

- A played card is played however it came (D2, 41.13); a card played from
  another zone "as if from your hand" is played immediately as the
  resolution of the card that plays it, the impulse not going around; its
  requirements and cost as normal; it cannot be forgone: its player's next
  decision is among those cards, none playable → nothing (Spiritual
  Guidance, 67.4).
- A card held by another card until it resolves stays in hand until then:
  Concealed Weapon's weapon is named as the card is played, still in hand
  until it resolves, then paid and shown (63.9, LSJ 20100130); Pack Alpha's
  retainer, named among those the vampire could employ now, is paid and put
  in play if still in hand, replaced as if played from hand (22.2).
- Piper's locked Anarch is the card's actor: it meets the ally's or
  retainer's requirements, pays the blood cost, its Disciplines pick the
  version; Piper stays a master card (Sudden Reversal can cancel it); not
  an action (28.10).
- The Erciyes Fragments: the card on it stays its owner's (the prey's):
  played from it, it is removed to its owner's removed cards; a political
  card Delaying Tactics returns goes to its owner's hand (LSJ 20050322); a
  card entering play from it is its owner's; a master is played with a
  master phase action; the card on the Fragments as they leave play burns
  to its owner's ash heap — its controller ousted, another Methuselah's card
  on it goes to its owner's ash heap (60.19, 67.1).
- "Only one … each turn / per game": D27; "only one Flaming Candle can be
  played or equipped in a game" counts an equip via Contraband, a search or
  Vast Wealth, not a move from another minion (38.7, ANK 20221103).
- The same modifier or reaction once per action: D26 (46.1).

### 7.2 Cancellation

- A canceled card was played: its cost is paid (unless the canceling card
  says otherwise), its limits counted (46.1, 51.8, 71.6, advanced rules
  §1.8.3); a canceled card pays no toll paid as the forcing card resolves
  (Parijat's, 18.1, ANK 20260317, superseding ANK 20260216's Eagle's Sight
  example).
- Burning one's last pool to cancel True Love's Face is not offered (18.2).
- Diana Iadanza cancels any reaction card as it is played during the
  referendum of a political action she performs; its cost is paid (51.8).
- Delaying Tactics cancels a granted referendum (nothing to return to hand,
  6.2); Reinforcements canceled by it is removed from the game, as its text
  says (51.6).
- Victor Donaldson: any cancellation of a Golconda: Inner Peace his
  controller played (its own 2-pool cancel included), Victor in play (in
  torpor too); offered once the canceled card is replaced, so the
  replacement cannot draw it; the library shuffled (59.12).
- Dark Influences: as Direct Intervention, played on another Methuselah's
  turn; its shield is any Dark Influences in play, whoever controls it,
  against a cancel of a minion card of another Methuselah than the
  canceling one (68.9).
- Touch of Clarity's "a non-combat card": a reacting vampire may cancel an
  action card as it is played, before its action is declared (71.26).
- Groundfighting cancels "a combat card … that would restrict this
  anarch's choice of strikes this round" — in the pool, a grapple leaving
  only hand strikes (Immortal Grapple), Drawing Out the Beast against an
  armed anarch (LSJ 20050221) and Sanguine Entrapment's barred dodge or
  combat ends (77.9); the canceled card's cost is paid (71.6).
- Hide the Mind [obf] "a combat card requiring Auspex": any, the player's
  own included (29.11).
- What a cancellation reaches: advanced rules §1.8.4.
- Andrew Stuart: as he plays a card at its [tha] mode, the top card is
  revealed first, before any other cancel (LSJ 20100728); one requiring
  [tha] at any mode (LSJ 20011204-3) cancels the card played — still played,
  its cost paid unless an action card (paid at resolution), replaced
  normally — and Andrew burns 1 blood; a play canceled as it opens is
  settled at once, with no window (Two Wrongs likewise) (77.4).
- Mithras (G3): +2 votes, no title (§3.15); once each action he performs, he
  cancels a reaction card played by a younger vampire — not an ally's, even
  played as a vampire — its cost not paid, a wake included (77.7).
- Rewind Time: [pre] a bleed against you, read live (after a redirect, not
  yours); [tem] any action card as played (ANK 20200813-2), not one played
  otherwise than from hand (RTR 20001020); [TEM] outside an action, never in
  combat (LSJ 20080618), another Methuselah's non-out-of-turn master card,
  its cost not paid, its player +1 master phase action (77.11).
- Death Seeker: cancels a combat card the opposing minion plays, as played,
  its cost unpaid; a canceled strike chosen again (LSJ 20100206); one per
  vampire each round; an end-of-round card counted in that round (ANK
  20180910-2, untested) (80.14).

### 7.3 Costs and reductions

- When paid: a card's cost as it is played (a card) or as the action
  resolves (an action card, an action's own cost: D20, 3.6); a cost paid at
  resolution that became unpayable fizzles the action (3.6). A strike's
  blood as it is chosen, not refunded if dodged (62.16); Preternatural
  Evasion's burn as the strike is chosen, not offered with no blood (63.5,
  ANK 20200703-2).
- What may be paid: D3. A strike card its player cannot pay for is not
  offered (42.2, 58.17).
- Reductions (The Line, The Shard, Ravnos Carnival's and Ravnos Cache's
  counters, Regina Blake, Nergal, Focus the Blood, Inner Essence) are
  announced before the card's terms; a card unaffordable without them is
  offered only with them (36.1, 41.13, 60.7, 63.7). The Line and The Shard
  may be locked as the action is declared or on resolution (just before the
  cost is paid); never by a card canceled as played (ANK 20230620); a
  non-action card's Shard as it resolves. Counters are spent as the cost is
  paid — none by a blocked or fizzled action (36.1).
- The player orders reductions as they wish; payment from an alternate
  source (Carnival's blood counters; Cache's pool counters if the equipment
  costs pool, else blood) comes after (36.2).
- Reductions reach a card put in play or played by another effect (Piper,
  Contraband, a search) unless the reduction's text requires the card to be
  played normally through an action: The Line, The Shard, Ravnos Carnival,
  Ravnos Cache apply; Charisma's "recruit ally actions" does not (36.5,
  41.13). Searched equipment: offered or equipped if payable with the
  reductions it could take; once found, the reductions are offered, then it
  is paid (a pool cost may still oust) and equipped (38.5).
- A reduction "-1 pool" takes nothing off a blood cost (Anne-Marie
  Bourgeois, 41.13); Roger de Camden's is blood only (41.9); Regina Blake's
  blood is announced with the card like a reduction and burned as the cost
  is paid, only if something is left to pay (41.13).
- Nergal's reduction is his choice, of the card he uses it on — announced
  with that card, once each Methuselah's turn, spent only when that cost is
  paid (60.7). Lord Tremere's Blood Sorcery reduction: any card requiring
  [tha] he plays (60.5). Ragnar Nordstrom's: wherever he pays the card's
  blood, combat cards included (42.5). Adana de Sforza's: 1 blood off a card
  played as a combat card, never off a "burn blood" effect (62.2). Inyanga's
  for recruiting an ally requiring a Gangrel (59.5).
- Surcharges: Unleashing the Bestial Soul [DOM]'s applies to the first
  reaction card paid for after it (23.4); Daria's and Marcellus's +1 are
  costs of the action (42.1, 42.6, 62.7, §8.3); Nergal (advanced)'s
  "reaction cards cost +1 blood": any played by another minion while he
  acts, a dual card only as a reaction (71.24, PIB 20150924-2); Scout
  Youngwood's +1 blood on reaction cards, paid with the card's cost (58.9);
  Djeneba's toll on strike cards (42.2); Libertas' surcharge on cards
  played using a Discipline (38.4).
- An X cost: X chosen as the card is played, halved (The Ankara Citadel)
  once chosen (70.17); Veil the Legions' X from 0 to the vampire's blood,
  burned at resolution (23.6).
- Split payments: Contraband's blood part chosen as the card is played
  (with the equipment's name), at most half the cost, pool the rest (29.6);
  The Gate of Acheron's payment one count of random ash heap cards, the
  rest pool (27.6); Cave of Apples' counters (35.2).
- Halving (The Ankara Citadel, Turkey): every card not played as an action
  modifier is halved, a dual card played as a reaction included; a dual
  card the acting bearer plays outside combat is a modifier, not halved
  (70.17).
- Cost alternatives "in pool or blood", the Edge, a Methuselah's ballots:
  §13.
- Whose cost, paid in what: advanced rules §1.7.6; paying what you can:
  §1.7.5.
- Strohmann, Badr: Nergal's shape (38.4) — the mode played; Strohmann's
  surcharge on [aus] reactions while he acts, his combat and referendum
  included (77.24).
- Ghede: his enter-combat action directed at the chosen minion's controller
  (D12); combat cards played using Serpentis (the mode played, 38.4) cost
  him 1 fewer blood, Eye of Hazimel's 1 more included (78.3).
- Walker Grimes: +1 pool on each master card his own controller plays
  targeting him (LSJ 20081124-1), folded with the haven surcharge (78.17).
- Helena (ADV): no blood cost for reaction cards played using Auspex or
  Dominate, any X (LSJ 20051117), a "burn n blood" effect still paid (D20);
  merged: Independent, an anarch token burned (ANK 20180524), immune to
  demons' damage — not inflicted at all (LSJ 20010627) — and +1 level of
  Daimoinon and Obtenebration (78.21).
- Marie Faucigny: non-weapon equipment 1 blood or pool less to equip, a
  locquipment's too (LSJ 20080109) (80.6). Menele (ADV): 1 fewer blood on
  minion cards (60.3) while any Methuselah's Helena is ready, read as paid
  (80.7).

## 8. Actions

Advanced rules §3.1 (announcement and targets), §3.5 (NRA), §3.8 (provided
actions), §3.9 (mandatory actions), §3.10 (changing the acting minion).

### 8.1 Announcement: targets and terms

- Everything the action names is chosen as it is announced (D1); the
  targets are re-checked at resolution: gone, moved, no longer ready or
  younger → successful, no effect, cost paid (17.4, 26.1, 60.10).
- Legality at announcement: an action sure to aim at an illegal target is
  not offered; one whose object comes from a choice or randomness at
  resolution (Vast Wealth's search) may be taken even if it will fail
  (17.4). A play is masked when its terms cannot be met (43.1, LSJ
  20100129).
- Per-card announcement readings:
  - Lenelle: both cards named at announcement (LSJ 20000222); not offered
    with an empty hand or ash heap; a named card gone by resolution:
    successful, blood paid, no exchange (17.1).
  - Feast of the Soul's Secrets [DOM]: the younger target (one's own may be
    in torpor) chosen as the card is played; younger than the acting
    vampire (26.8).
  - Grooming the Protégé: the giving vampire (not locked) and X (1–3) named
    at play; it gives what it still has at resolution (33.4).
  - Minion Tap: the amount announced at play, 0 up to the vampire's blood
    then (an empty vampire a legal target); what is left is taken at
    resolution (38.1, LSJ 20100216).
  - Psychophagia's ally (whose ash heap, then which ally; an ousted
    Methuselah's does not count), Hel-Blá's ally (offered only while her
    Methuselah's ash heap holds an ally requiring Hecata or printing
    Oblivion): named as declared, gone by resolution: no effect (32.12,
    41.6).
  - Giovanni del Georgio: the card named as announced (two copies in his
    prey's ash heap; not offered without such a pair); both removed at
    resolution as its cost — fewer left, it fizzles (59.1).
  - The Sargon Fragment: the card named in the bearer's controller's ash
    heap as the action is announced; not offered with an empty ash heap; an
    ally bearer gets nothing ("this vampire") (60.17).
  - Malgorzata: the vampire receiving the blood is the action's target, the
    one removed named as it is announced; either gone, nothing (60.10).
  - Loki's Gift [PRO]: the vampire chosen as the card is played (68.17).
  - Shroud of Decay [OBL]: playable only with ≥ 7 cards in the prey's ash
    heap, re-checked at resolution (fewer: fizzles) (26.4); burned vampires
    count (42.7).
  - Trochomancy: the removal is the price — not playable unless the target
    Methuselah's ash heap holds the cards (67.7).
  - Line Brawl [cel]'s "another Methuselah" is any other seat; Rewilding may
    target one's own location (undirected), burning one's own 2 pool; Touch
    of Valeren [ACTION]'s "a minion" is any minion, directed when another's
    (26.3).
  - Puppet Master's "a younger vampire" may be one's own (undirected,
    no-op); Deep Song [ANI]'s "a vampire" is another Methuselah's ready
    vampire (27.2).
  - ECTU Operative's "burn a vampire in torpor as a Ⓓ action": any torpid
    vampire, another's (directed) or one's own (undirected); out of torpor
    by resolution: fizzles (38.2).
  - Phantasmagoria's "put this card on a minion": any minion, one's own
    included (undirected) (32.5).
  - Baltimore Purge's Ravnos +1 stealth is read as the action is announced
    (63.13); an action card's conditional stealth is read then and skipped
    at resolution (63.17).

### 8.2 Directed actions

- What is directed: §3.4. Who may block a directed action: its target
  Methuselah; an undirected one: the acting Methuselah's prey and predator
  (6.8, rulebook "Blocking").
- Bars on being targeted: Secure Haven's minion — other Methuselahs'
  actions (card actions, granted ones, rescue, diablerie) cannot target it,
  a referendum's terms can; choosing it at play costs 1 pool more (35.1,
  ANK 20221019-3). Veneficti: not the target of a directed action of an
  ally or of a vampire of capacity 6 or less, read as it is offered (62.30).
  The Guardian's directed actions are barred wherever an action is
  declared: bleeds, rescue or diablerie of another Methuselah's vampire,
  action cards and granted actions aimed at another Methuselah, their
  minion or card (41.5).
- Slaves: Tupdog ("Tremere antitribu slave", rulebook "Slave") takes no
  directed action while his controller controls no ready Tremere
  antitribu, read as it is offered (62.14, advanced rules §5.7.2).
- Intercept "against actions directed at Mithras or a Mithraic Mysteries
  card or member" (Mithraic Cultist): a directed action whose target is
  Mithras, a member, The Mithraeum or a Mithraic Cultist (34.8).
- Unleash Hell's Fury is offered against an action directed at its
  controller, an action at a card of theirs included (60.21).
- Vaticination: directed at every other Methuselah, each hand a target, any
  of them may block; the hand chosen, then the card, replaced unseen (PIB
  20121031) (owner) (80.35). Smash and Grab [ani]: at an ally or a retainer,
  directed at its controller (LSJ 20090324); an ally burned so is burned by
  a Ⓓ action (ANK 20220507) (80.29).

### 8.3 An action's costs

- An action's cost — the action card's cost, the rulebook action's
  (rescue, leave torpor, become Anarch), a granted action's printed cost,
  a surcharge on the action — is offered only if payable as announced and
  paid as it resolves; a blocked action pays nothing; unpayable at
  resolution, the action fizzles (3.6, 42.1, 42.6, 55.3, 58.1, 62.7, 67.5,
  69.8, 52.10).
- Daria's "directed actions cost her +1 blood" is a cost of the action
  itself, not of a card: a plain directed bleed costs her 1 blood, a
  directed action card its cost plus 1; payable with the action's own blood
  cost as announced, paid as it resolves; her undirected actions are
  unaffected (42.1). At resolution the +1 is paid first: unpayable by then,
  the action fizzles; paid, then the card's own cost unpayable, it fizzles
  with the +1 paid (42.6, ANK 20220218).
- Marcellus: an action cost as Daria's, on every Nosferatu's political
  action, card or granted, any Methuselah's; his readiness read at both
  points (62.7).
- Ambrosio Luis Monçada's "actions directed at him cost 1 additional pool":
  the acting Methuselah's pool, paid with the action's cost as it resolves
  (not if blocked); paid down to 0, they are ousted and the action ends
  (69.8).
- War of Ages' "as a +1 stealth political action that costs 1 pool": paid
  as the action resolves (52.10). Eulogio Sánchez de los Reyes's 1 blood:
  the action's cost, at resolution, not if blocked (55.3).
- Not costs: Hunting the Quarry's card burned as the granted action is
  declared (its target the minion it was on; gone, the action fizzles,
  54.4); Heartrender's "remove itself from the game", paid as the action
  succeeds (blocked, it stays in play, 55.4).
- Forced Awakening's blood is burned as the action begins to resolve —
  successful, fizzled or failed; a block and its combat are the action's
  resolution, so the blood burns then too, once (30.2, LSJ 19990421).
- Rescue: 48.8 (§11.6).

### 8.4 An action's stealth

- The rulebook actions carry their printed stealth (hunt +1, become
  Anarch +1: 58.1); political action cards +1 by default; an action granted
  by a card in play has only the stealth its text prints, directed or not
  (6.2, 6.8, 40.11: Rise of the Nephtali's Liaison action has none).
- Veil the Legions [OBF]: X chosen at play (0 to the vampire's blood),
  burned at resolution; the next X actions of the Methuselah's minions this
  turn get +1 stealth, counted as announced (an action canceled before
  resolution still counts, 64.4); "only one at superior each turn" is one in
  all, game-wide, a turn being one player's turn (23.6).
- The Khabar: Community's "+1 stealth when bleeding": during bleed actions
  only (33.9).

### 8.5 No repeat action (NRA)

- A minion cannot perform the same action twice per turn, successful or
  not; the trigger is resolution — a blocked action or a failed
  referendum's action resolved; an action canceled as announced (Direct
  Intervention) did not and may be retried (1.4). An action canceled before
  resolution (The Kiss of Ra) leaves no mark, unless it was blocked then
  continued before (64.4, ANK 20211015).
- Identity: a card from hand = the card's name, per minion; a card in play
  = the copy (two Anarch Revolts = two actions); each action a card in play
  grants is its own (Open War, Haven Uncovered's two; 1.4, 32.1, LSJ
  20100909); a granted referendum's "same political action" ban uses the
  same identity — another copy's referendum stays open (6.2, LSJ 20020201).
- Type cap: one bleed and one political action per minion per turn,
  whatever provides it; other rulebook actions are exempt (a minion may
  hunt twice) (1.4).
- Card text "cannot perform the same action again this turn": for a card
  action, the ordinary identity; for a rulebook action, the same kind at
  the same target (rescuing a different vampire stays open, RTR 19950509)
  (23.2); Change of Target's also covers the rulebook actions NRA exempts
  (1.4).
- The equip-from-own-minion action is NRA-exempt and repeatable, but
  fetching the same equipment again is "the same action" for such texts
  (1.4).
- Evan Klein's tails: the acting minion still locks, the action counts for
  NRA and the bleed and political caps; cost and blocks are skipped;
  as-announced plays remain possible before the flip (16.6).
- Mask of a Thousand Faces: the masker takes the action's NRA marks
  (58.13).

### 8.6 Actions provided by cards in play

- A referendum or action granted by a card in play is that kind of action
  in every respect: it locks, is blockable, counts toward the caps; NRA by
  copy; only its printed stealth; no calling card, so no card vote (6.2,
  46.2); Delaying Tactics cancels it (6.2).
- "Minions can burn this card as a Ⓓ action": 6.8. Powerbase: Madrid's
  burn of all its counters as a Ⓓ action is offered with no counter, a
  legal no-op (73.3).
- Mobile HQ's granted action enters combat with a ready vampire of another
  Methuselah (38.6); a regent's "Sabbat vampires can enter combat with this
  vampire as a Ⓓ action": another Methuselah's Sabbat vampire (52.6).
- The Red List: the master phase action marks another Methuselah's Red
  List minion (one's own is not offered); the action to enter combat with
  it is a +1 stealth Ⓓ rulebook action, once per vampire per turn, its 1
  blood paid at resolution (67.5). Big Game reads Red List on the printed
  card (71.22).
- Society of Leopold's move action targets any other vampire, any
  Methuselah's (directed when another's), the card moving on success
  (59.3). Ephor's action targets any vampire whose title is contested,
  which loses it; its rivals' claims return free in their unlock phase
  (59.13). Maabara's burn is an action directed at its controller (71.12).
- "Use a master phase action to …": D9.

### 8.7 Mandatory actions

- Ordering: 17.2. A mandatory action holds the minion phase until taken;
  other actions wait (Elen Kamjian's bleed while she controls a locked
  minion — any minion, torpor included — and has not bled this turn; with
  0 blood the rulebook hunt takes over; once she bled she declares no
  action but may block and react) (17.1).
- Must hunt: a new vampire of Childe of the Revolution must hunt, even with
  blood, at once when that is its only option (28.1); a card hunt
  discharges it (32.13, 63.2); Week of Nightmares' Ravnos must take the
  special hunt, and with no target takes no action (other minions act)
  (28.8). Advanced rules §3.9 (stuck, recurrence).

### 8.8 Changing the acting minion

- Mask of a Thousand Faces: the masker must be capable of the action's
  kind (not a granted action, a torpor action or one needing a locked
  minion), of its action card (clan, Disciplines, title and sect
  requirements) and of each card the acting minion played on it (clan, one
  of its Disciplines); the masked minion's inherent stealth already used
  does not bar it; [OBF] only when stealth is needed (owner 2026-10-09)
  (58.13). No Mask take-over of a blood-cursed vampire's diablerie (63.1),
  nor once a card on the acting minion applied its effect to the action
  (Aching Beauty's burn, 76.17).
- Deep Song [ANI]'s "considered the acting minion": the target takes the
  acting side in that combat (its Methuselah acts first each step); effects
  on "this acting vampire" (Invigorate [pro], Form of Mist's continuation)
  stay with the vampire that took the action (27.5).
- Tupdog: as a Tremere antitribu of his controller's is blocked, he may lock
  to fight the blocker instead — the acting vampire unlocked, the canceled
  combat's riders gone (62.14, advanced rules §5.7.2).
- Advanced rules §3.10.

## 9. Stealth, intercept, blocks

Advanced rules §3.2 (stealth and intercept), §3.3 (blocking).

### 9.1 Who may block

- Directed: the target Methuselah's ready minions; undirected: the prey's
  and predator's (6.8, §8.2). A rush's target's controller alone (17.3).
- Unlocked or woken (2.4, §14.1); locked minions attempt only with a card
  saying so (forced attempts, 42.8).
- Card readings: Anneke's Methuselah is an eligible blocker against any
  action by a vampire of another Methuselah, for her alone; her other
  restrictions stand — Seduction, her Methuselah's decline, allies acting
  (62.9). Mylan Horseed's "cannot block vampires" leaves his blocks of
  allies (57.10). Allies may block with The Signet of King Saul, its
  capacity read live at the attempt, Discipline masters counting; any
  action of the bearer (59.15). Under Seeds of Terror an ally cannot
  attempt ("burn 1 blood", blood only; Unthinkable Humiliation prints
  "blood or life") (53.2); under Ni Dieu ni Maître [for] an ally attempts
  free (70.12).
- A minion stolen mid-attempt: the attempt fails without barring it; its
  new controller may attempt with it if eligible (27.3).
- A blocker locked during its own attempt fails it (Alexandra, 62.3;
  Marciana, 69.7).
- Aisling Sturbridge: a younger Tremere of any Methuselah is no block
  candidate, read live (80.1). Karsh (ADV) cannot block an infernal vampire,
  read at the attempt (80.5). General Perfidio Díos: 1 blood to unlock and
  attempt to block an action directed at his controller's location, several
  times an action (LSJ 20030605), not bound by "only when needed"; unlocked,
  a legal no-op shown with opportunities on only (owner) (80.4).

### 9.2 Only when needed

- Gaining stealth is legal only while a block attempt is pending with
  intercept ≥ stealth; intercept only while stealth exceeds it; surplus
  plays are illegal; no pre-emptive stealth without an attempt (3.2, 1.6,
  advanced rules §3.2.1). It applies to cards, printed abilities and
  in-play abilities alike: Abraham DuSable, Daria (lock while the acting
  Nosferatu — anyone's — needs stealth), Oskar Anasov, Gwenllian Arwyn
  (15.2); Saatet-ta (44.3); Maila, once each of her Methuselah's turns
  (57.2, ANK 20230122); WMRH Talk Radio, the minion attempting to block,
  whoever controls it (34.1); The Rumor Mill, any ready vampire, once
  (34.2); Monastery of Shadows' lock, by any Methuselah controlling it
  (50.4); Under My Skin's burn, in any later action (60.12); Bowl of
  Convergence (9.6); Shadow Cast's burn (24.5); Nergal (advanced)'s stealth
  burn, once each action (79.1); Mask of a Thousand Faces [OBF] (58.13); Mind of
  the Wilds [ANI][AUS] (64.5).
- Exceptions by text: "even if … not yet needed" (Form of the Cobra
  [PRO], Paths in Two Worlds, Form of Mist's continuation, Creepshow
  Casino; Organized Resistance, Second Tradition) (3.2, 2.4); Predator's
  Transformation [abo]'s +1 stealth as the action is announced, even if not
  needed (64.8); standing bonuses of cards in play (Loup, playable with no
  attempt, LSJ 20011214-2: 3.2, 51.7).
- Bonding [DOM]'s superior mode (+1 stealth and +1 bleed) only when
  stealth is needed; its [dom] bleed bonus alone before any attempt (8.1,
  TOM 19951109). Revelation of the Serpent: 25.3.
- Gathii's reveal is usable before stealth is needed; the +1 stealth then
  not gained (15.1, ANK 20231031). Enchanting Gaze: while a minion with ≥ 1
  of their counters attempts to block, stealth needed or not (25.5).
- Shadow Cast [OBL]'s burn is a stealth effect by its vampire during its
  own action; the burn is allowed when the recorded set of Methuselahs (its
  vampire's controller not counted, LSJ 20100527) is fully included in the
  set the current action is directed at (24.5).
- Reducing the opposing value is legal even to no effect (zero or negative
  stealth: Draba, Night Terrors, LSJ 20021211; advanced rules §3.2.2),
  within the card's own timing: a third party's strip needs the defender
  attempting (1.3); "during the action" reductions wait for the block
  attempts (66.1); Gwenllian Arwyn's only while stealth is needed (15.2).
- Changeling Skin Mask, Marked Territory: the intercept burn only once the
  bearer's block attempt needs it (D15); Marked Territory's during an action
  directed at its controller (77.18); Changeling Skin Mask's "the vampire
  with this equipment may burn it": an ally bearing it cannot (77.19,
  corrected).
- Corpse Minion: as often as paid (TOM 19960109), X burned 1 at a time while
  the intercept is needed — no surplus (K23 not opened) (80.13).

### 9.3 Values

- Not floored at 0: a -1 leaves a 0-intercept blocker at -1, below a
  0-stealth action: the attempt fails unless intercept is gained back (now
  needed); stealth may be negative; "reduce stealth to 0" leaves a negative
  stealth unchanged (48.1, LSJ 20021211).
- Bonuses last the action (15.2, §5.5).
- "Intercept against X" reads the acting minion (13.2: Azucena, Bret
  Stryker; 44.6: Carlton Van Wyk "against vampires"); Lisette Vizquel's +1
  intercept as she attempts to block a diablerie action (63.16); Neighbor
  John's is a plain +1 (62.17).
- Sense the Sin: [aus] against a younger acting vampire; [dai]'s extra +1 if
  the acting minion has anyone's corruption counter; [DAI]'s -1 intercept
  to every minion with any corruption counter, any Methuselah's, read live during the
  action (62.28). The Unmasking: every Methuselah's allies, +1 intercept
  while the acting minion is a vampire (62.29).
- Branimira reads pool when intercept is compared (13.2).
- Into Thin Air: replaced as its controller's unlock phase begins (ANK
  20200129); once this action, 1 blood for -1 intercept on a minion
  attempting to block, an ally or younger than the actor, for the rest of
  the action (78.37); it and Lost in Crowds exclude each other on one action
  both ways, any minion's play (78.37, 78.53).

### 9.4 The block loop

- A block attempt does not fail at a stealth ≥ intercept checkpoint; it
  fails only when all Methuselahs pass in succession. While it lasts the
  defender may stack intercept or pass; the impulse moves on to others who
  may help (KRCG News Radio's intercept; Draba's, Night Terrors' stealth
  removal), then the defender may continue (1.3).
- Failing to block = passing. Declining (passing with no attempt pending)
  is final: later stealth removal by a third party does not let them block
  (1.3). Declines are per Methuselah, not per minion; after a Form of Mist
  continuation earlier declines stand (3.4).
- A blocker short of the stealth attempts with another minion or passes:
  that pass is its decline once the table passed on the attempt still
  under way (KRCG, Night Terrors may still help it; a play voids the
  decline, 1.2); then the impulse goes on from the next Methuselah after
  it, the acting one's coming once blocks are declined, and the acting
  Methuselah's impulse in the open window is only its first and the one
  after a play (82.1). Only that failure: one by an effect (Enchanting
  Gaze, a cancel) returns the impulse to the acting Methuselah, and an
  ended action (The Kiss of Ra) takes no further attempt (82.1); one with
  no play (a lock at the closure, a toll unpaid) reads as an effect's,
  not a decline (82.2).
- Only a change of the action's target (Deflection, Telepathic
  Misdirection) reopens block opportunities; Change of Target does not
  change the target, it ends the action (1.3).
- Third parties act when their cards allow and the impulse reaches them,
  normally after the acting and target Methuselahs both passed; "+X
  intercept" applies to the minion playing the card; granting intercept to
  another's minion (KRCG News Radio) is the exception (1.3).
- No bait or retraction: a third party wanting to strip stealth needs the
  defender to attempt first; once attempting, the defender needs no
  matching intercept to keep the attempt alive (1.3).
- Changing blocker: while its minion's attempt is short of the stealth, a
  Methuselah may attempt with another minion instead — by declaring that
  one's attempt, or with a card whose unlocked minion attempts to block
  (Organized Resistance's second mode); the attempt under way fails then
  (48.3). A forced attempt on the minion already attempting keeps it (48.3,
  52.8, ANK 20181122-2).
- Dawn Operation [for]: the attempting vampire's controller is asked once,
  as the card resolves, whether to cancel the attempt; a canceled attempt
  is withdrawn, not failed — the vampire may attempt again (30.1).
- Forced attempts ("… attempts to block": Eagle's Sight, Second Tradition,
  Organized Resistance, Eyes of the Wild, Dogged Pursuit, Sense the Savage
  Way, One With the Land) by a minion owing a block toll: offered only
  while the toll is payable; if the card's own blood, paid first, leaves it
  unpayable, no attempt is made (the card's other effects stand) (42.8). A
  canceled forced attempt pays no toll (18.1, ANK 20260317).
- Under Siege: offered to each ready Sabbat vampire of its controller that
  could attempt to block, locked or not, the one already attempting
  included; once each action; +1 intercept for the action (52.8).
- Every legal attempt is offered, one that cannot reach the stealth
  included (73.8). Whether it may yet reach it — intercept cards in hand or
  drawable, stealth an ability in play can still take off (Screamer,
  Aranthebes) — is the host's reading, hiding the others with the
  opportunities off (48.5, 66.2, §22).

### 9.5 Block tolls

- A toll is paid to attempt (Sergio Bueno, Mithras, Tenebrous Form, Where
  the Veil Thins, Seeds of Terror, Ni Dieu ni Maître [for], Giotto
  Verducci's, Crocodile's Tongue's cancel). An ally pays in life where the
  toll allows, its last one included — it burns and its attempt fails
  (41.4, owner correction). Mithras's toll applies from torpor too (his
  leave-torpor action) (42.4).
- Tenebrous Form [OBT] and Where the Veil Thins [OBL] in one action: each
  its toll — a minion with neither Discipline burns 2 to attempt; one
  already attempting pays the new card's 1 as it is played, or resigns; a
  second copy of the same card adds its own 1 too (69.12).
- Crocodile's Tongue: "younger" than the acting vampire; the blocking
  minion's cancel is offered while it has the blood or life (an ally paying
  its last life burns); the bar covers every minion of the blocker's
  Methuselah for that action (60.13). Giotto Verducci tolls vampire blockers
  only (71.18).
- Aelswith and Terrifying Visage: the pool the blocker's controller burns
  as the block is declared may oust them; their minion leaves play with
  them, and the action is blocked with no combat (53.1).

### 9.6 When a block succeeds

- "If blocked" riders resolve as the block resolves, before combat or the
  blocker's diablerie choice: Marialena's blood (16.5); Melange [AUS] goes
  on the acting minion as the block succeeds, even if the action then
  continues as if unblocked, its +1 bleed not limited (30.5).
- Change of Target is played before the blocker is locked (LSJ 19980224,
  RTR 19991206): a blocker unlocked when it blocked stays unlocked; the
  action ends as blocked, no combat, no diablerie (23.1). Mirror Walk [THA]
  "end the action before block resolution": counted as blocked (NRA
  identity kept, blocker locked), no combat, nothing more played on it
  (8.4).
- Read the Winds [ani][aus] keeps its vampire unlocked after a successful
  block; [ANI][AUS] prints no "as above": it locks as usual; the usual
  blocking restrictions apply (64.9, ANK 20180623, LSJ 20010714-2).
- Obedience: "about to enter combat with an acting younger vampire" is the
  block — played as the block succeeds, before combat — and also a combat
  the action itself starts (a rush, Deep Song, ANK 20200702), played there
  by a vampire unlocked or woken; the action ends unsuccessfully, no combat
  (55.7). Against an action-begun combat: in the reaction window once
  blocks are declined, by the vampire the action is to enter combat with;
  the action ends failed at resolution (NRA holds, LSJ 20070709); not
  against a combat Hedonism queues, a reacting minion's (The Guardian) or
  the after-resolution window's (56.2).
- Mental Maze: "Followers of Set" is the Ministry, the reacting vampire's
  clan; the acting minion stays locked (no unlock printed, unlike
  Obedience); [OBF]'s blocker is as it was before the block (71.11).
- The Kiss of Ra: against a block attempt by a vampire without Fortitude
  (the Fury included, LSJ 20100210); the blocker burns 2 blood (as much as
  it has), [FOR] then goes to torpor; the action is canceled before
  resolution — nothing more played in it (RTR 20180719), no outcome, no
  after-resolution window, no torpor-block diablerie (LSJ 19970325), no NRA
  mark (unless blocked then continued before, ANK 20211015), still taken;
  the acting minion stays locked (64.4).
- Unleash Hell's Fury: the card itself is a minion for the duration of its
  block attempt (a 9-capacity infernal vampire, +2 intercept, 0 blood); its
  successful block ends the action at once — damage and burn, no combat,
  nothing played on it afterward; never locked (60.21).
- Dogged Pursuit [ANI]: the card goes on the vampire after resolution when
  it did not block, unless it already has one; its +1 intercept serves any
  later attempt of that vampire, burned then (30.4).
- Blocks and torpor actions: a blocked leave-torpor action lets the blocker
  diablerize (rulebook; 16.2 Queen Anne; 63.1).
- Iris Bennett: an older vampire's (capacity, D30) successful block lets its
  controller look at her controller's hand before block resolution, once
  that block (78.14).
- 48.3: the switch of blocker takes effect at once, with no impulse left to
  others for the abandoned attempt.

### Open or contested

- The Fury's block and later plays (Change of Target, Mirror Walk, Elder
  Impersonation, KRCG News Radio, Tension in the Ranks): the ledger rows are
  `blocked` (untested), the reading of 60.21 untested against them.

## 10. Action outcomes

Advanced rules §3.4 (resolution, success and effect), §3.5.3 (ended and
failed), §3.6 (continuing as if unblocked), §2.3 (after resolution).
Vocabulary: §3.2.

### 10.1 The would-be-successful window

- 24.2 (§5.2). Spying Mission [OBF]: "the same Methuselah" = the bleed's
  target as the card resolves, after redirects; copies already on the
  vampire burn only when a later bleed of that Methuselah resolves for ≥ 1,
  after the would-be-successful window; a second one played in that window
  leaves the first in place (24.3, ANK 20200609).
- A bleed whose target is ousted before it resolves (by the cost of
  canceling Spying Mission, LSJ 20050607): the oust is handled first (its
  predator paid), then the bleed fizzles — no pool, no Edge,
  after-resolution cards apply (24.1).
- Yawp Court is used in the political action's would-be-successful window
  (blocks declined, before it resolves): the action card's cost is paid
  first (LSJ 20080201), then the location and the Sabbat vampire lock and
  the combat begins, the acting vampire first; "still ready at the end of
  combat" is read as the combat ends — before a Rötschreck's torpor (LSJ
  20080821), not after a torpor left at once (LSJ 20090325-1); the action
  ending there is a failed action, with no after-resolution window (54.1).
- Major Boon: 68.13 (§5.2).

### 10.2 Resolution and fizzle

- Resolution reads everything again (D13, advanced rules §3.4.2): targets,
  costs, conditions. Gone or illegal → a fizzle: successful, no effect, the
  cost paid (3.6, 17.3, 17.4, 26.1, 42.6, ANK 20220218).
- Out of torpor before resolution: a leave-torpor action fizzles paid (ANK
  20220218, Warsaw Station); a rescue whose vampire left torpor fizzles
  paid (split as normal; a vampire gone from play pays nothing, the rescuer
  pays what it can); a diablerie fizzles with no blood hunt (28.7).
- "Enter combat with a minion" from an action card: target another
  Methuselah's ready minion, re-checked at resolution; Fleetness [CEL]'s
  "locked minion" unlocked by then fizzles the action (ANK 20210124); the
  combat is the resolution, the action succeeds after it (26.1).
- A political action whose acting vampire is no longer ready at resolution
  ends with no referendum (Yawp Court ruling); the card's cost still paid
  (5.9). One whose terms have no legal choice when they are chosen fizzles:
  successful, no referendum, no effect; a rider keyed to the referendum
  failing (Cedrick Calhoun) does not apply (43.1).
- Yoruba Shrine: the referendum fails and polling ends at once (no votes,
  nothing keyed to passing); the political action is not canceled, its card
  spent (35.4).
- A combat begun at resolution is part of it: the action succeeds after
  the combat (17.3, 26.1).
- Gear Up [nec]: the card named at declaration (LSJ 20010627) gone from the
  ash heap as the action resolves — the Anarch still unlocks, nothing is
  fetched; resolve as much as you can (owner, 2026-10-15) (74.3).

### 10.3 Failed, ended, canceled

- A failed action (Archon Investigation, Spying Mission [OBF] failing it at
  resolution, Obedience ending it) loses what keys to success, Hedonism's
  queued combat included, as an action ending early (56.1, LSJ 20090826).
- Ended as blocked (Change of Target, Mirror Walk, a Fury's block): 23.1,
  8.4, 60.21. Canceled before resolution (The Kiss of Ra): 64.4.
- Delaying Tactics / Telepathic Vote Counting's "discard down afterward"
  is immediate, not in the discard phase; the action ends successful (no
  referendum effect), the after-resolution window opens, and the vampire
  has taken its political action this turn (5.5).
- Tangle Atropos' Hand: played on a pending block attempt; the action is
  canceled, not resolved — the same action may be taken again (LSJ 20070709)
  — its card back in hand ([tem]; discard down afterward); [TEM]'s +1
  stealth on the vampire's next action this turn applies as that action is
  declared, a standing bonus not bound by need (77.10).
- Edward Vignes: burns a card printing Dominate from his controller's hand
  to fail an action directed at him, not once a block is attempted or made
  (ANK 20260316, LSJ 20041022); the failed action reached resolution (1.4),
  was not blocked, and keeps its after-resolution window (ANK 20221205); the
  burned card goes to the ash heap and is replaced, as a discard (8.3); in
  torpor too (D25) (78.13).

### 10.4 After resolution

- Every action has an after-resolution window: a blocked action's after
  its combat; an action ended by Change of Target or Mirror Walk (no card
  may be played there, RTR 20180719, but Daring the Dawn's damage is taken
  there); a failed action (ANK 20221205) (23.3, 5.8). Exceptions: §3.2.
- "After action resolution" cards on a bleed open whether it succeeded or
  not, a fizzle included, not after a block (Gifts From Hereafter: the
  vampire's 1 blood paid then, as the ally unlocks, 55.5).
- After-resolution abilities: Aline Gädeke reads the acting minion's sect
  after resolution, usable in torpor, a fizzled action counting (16.4, ANK
  20220218); Alicia Cortez: "directed" as for bleed, rescue or diablerie of
  another's vampire, a burn action on their card; target any younger
  vampire (torpor too) of the Methuselah the action was directed at, a
  locked one included (16.4). Neighborhood Watch Commander: any
  Methuselah's, after the resolution of any Methuselah's blocked or failed
  action whose acting minion is a vampire; ready and unlocked, it locks to
  inflict 1 normal damage out of combat; one Commander each action, any
  controller's (65.2). Infernal Servitor's "after it performs a successful
  action": a fizzle counts; removed as the action ends (70.19). Predator's
  Transformation [ABO]: after the resolution of its directed action that
  burned a minion of its controller's prey out of combat, once per action
  (64.8, ANK 20220507).
- The Guardian's combat begins as he uses the ability in the bleed's
  after-resolution window, the bleeder acting first (ANK 20221102); once it
  ends the after-resolution window opens again for every Methuselah (41.5).
- Big Game "unlocks at the end of the action": as it ends, after all its
  combats, if still ready; a blocked action does not unlock it (71.22).
- Cedrick Calhoun goes to torpor as the action resolves (before the
  after-resolution window) if the referendum failed or was canceled, while
  ready (16.5).
- Go-getter [OBF]: played in the after-resolution window of the blocked
  action (after its combat) by the acting vampire if still ready; the
  action returns to the block step with modifiers and declines standing, as
  Form of Mist's continuation (30.3).
- Freak Drive and other "after resolution" cards: after a successful
  action, a fizzle included (3.6, 24.4, 5.8).
- Epikasta Rigatos: the card bound for the ash heap as her action ends — her
  action's own card, played that action (ANK 20180628), whichever of the
  action card types, blocked too — goes to her library, shuffled, instead
  (76.22).

### 10.5 Continuing as if unblocked

- Form of Mist [PRO]: the continuation happens inside the after-combat
  window (occupants order freely, acting Methuselah first; the blocker may
  unlock before or after it); the block loop resumes when the window
  closes, so an unlocked blocker may block again (the maneuver provided
  again on a second block) (4.1); earlier declines stand (3.4).
- "Continue as if unblocked" needs a blocked action; a blocked rush can use
  one, and the continued action then enters combat with its target at
  resolution (17.3).
- Torrent [CEL]: mandatory and free, applied as the after-combat window
  closes — only for a vampire blocked while acting (not after a rush's
  combat, RTR 19970630); lost if the combat goes on to another round
  (Telepathic Tracking, RTR 20020501); the action's modifiers, reactions
  and declines stand (ANK 20190116); an action card's resolution follows
  (LSJ 20070808-1) (63.6).
- Effects scoped to one block: advanced rules §3.6.3; Melange stays (30.5).

## 11. The kinds of action

Advanced rules §3.7 (bleed, hunt, equip, employ and recruit, referendum,
torpor actions, diablerie). Referendums: §13.

### 11.1 Bleed

- What a bleed is, against whom, success: §3.2, §3.3. One bleed per minion
  per turn, whatever provides it (1.4).
- Amounts: a bleed is successful for ≥ 1 (24.4); the pool actually lost is
  what the bled Methuselah had (Alamut's counters, 34.5); "(limited)" bleed
  bonuses and the others (§3.3, 42.7, 30.5, 44.3, 18.4); "+1 bleed against
  your prey" read live (Tier of Souls, 32.4); Raising the Portcullis, every
  vampire you control, not allies (39.5); Appolonius (advanced): +1 for each
  titled vampire controlled by the target Methuselah, those in torpor
  counted (69.13); Visions of Zapathasura's reduction (23.5). A cap on the
  pool burned is on whoever burns it (Protected Resources, Major Boon's
  payer included; copies do not lower it below 2; "after a minion you
  control successfully bleeds": any minion of its controller's, an ally
  too, every copy burning) (70.8). Army of Rats' "only 1 pool each turn
  with Army of Rats cards": across its controller's copies; the first
  resolved in the unlock phase burns, the others lapse (32.2).
- "If the bleed is successful" effects come after the pool burns, chosen
  then: Break the Bonds [obf]'s lock (optional, any minion of the bled
  Methuselah, locked or torpid included); The Platinum Protocol [pro]'s
  counter (mandatory, any minion that Methuselah controls); with [obf] and
  [pre], printed order (25.6).
- Bleed riders on the target's side: Alonso Petrodon's damage to each
  Anarch vampire of the bled Methuselah (torpor included; an ally is not an
  anarch), inflicted as the bleed resolves with the pool loss, before
  after-resolution effects (57.1); Propaganda [PRE]'s minion, named by the
  card's player as it is played (a ready unlocked minion of the prey), locks
  as the bleed resolves (31.2).
- Abilities before resolution: Larissa Moreira on her own bleed only (15.2,
  ANK 20231124); Victoria Ash (G7) gives an acting Banu Haqim +1 bleed once
  each action, before the bleed resolves, her Methuselah discarding (70.2);
  Marchesa Liliana's +1 is not the limited bonus; her seven cards are her
  Methuselah's picks by name, one at a time as she uses the ability (42.7).
- After a successful bleed: Ilonka looks, then her controller chooses to
  discard at random or not — while ready, after a successful bleed against
  her controller, once each turn; Richard de Worde's use chooses the card
  and burns the blood, the discarded card replaced (16.3). Fever Pitch's
  "you can put this card on this vampire" is an option left open whenever
  the bleed is successful and the vampire has none (53.5). Gostoso, Enkil
  Cog: "a successful bleed against your prey" (61.2, 50.3). The unnamed's 2
  pool: optional, once after each successful bleed (62.16).
- Bleeds and counters: Forward Momentum (53.6, §5.3); Dead Pool's "you can
  add 1 counter": always added; the leaving minion must be a vampire
  (34.4); Illusory Resources' burn of X counters once each bleed against its
  controller (63.14).
- Taxes on bleeding: Donatello Giovanni's "if the chosen vampire attempts
  to bleed before your next unlock phase, they move 1 blood to Donatello" —
  every bleed it announces until then; the chosen vampire any Methuselah's
  but the predator's, his own included (67.3). Aranthebes, The Immortal:
  shuffled into its owner's library; its lock gives -1 stealth to the
  predator's acting minion; the -1 bleed to vampires only (not allies),
  while the card is unlocked (32.8).
- Věnceslava: her prey "burned 1 or more pool" counts any pool her
  controller's prey loses during her action, costs paid included; a
  deflected bleed is not the prey's (52.4).
- Aksinya Daclau: ready and unlocked; once in each Methuselah's turn; the
  master card discarded is the cost, replaced at once; a legal bleed target
  only (59.16).
- Redirecting a bleed (Deflection) changes the target and reopens blocks
  (1.3); the KRCG rulings place it after blocks are declined, before "would
  be successful" plays (LSJ 20061213, LSJ 19980105; advanced rules
  §3.7.1).
- Two Wrongs: the bleed turned to the bleeding minion's controller's prey,
  who may block (a change of target); the next card that would change this
  bleed's target is canceled as it is played, its cost paid (ANK 20260216);
  abilities are not cards and pass it by (76.12).
- Lubomira Hradok: her successful bleed of a Methuselah controlling a ready
  titled vampire (the bled one after redirects; a contested title not
  titled, §3.15), checked as the bleed succeeds: she skips her next normal
  unlock (PIB 20110915-1) (78.16).
- Flurry of Action: a bleed; on success [cel] two cards drawn, as many as
  the library holds (LSJ 20060620), then a discard down to hand size
  ("discard afterward", the pool's wording for it: Learjet, Scrounging, The
  Summoning); [CEL] the vampire unlocks (78.26).
- Burnt Offerings [AUS]: after a bleed by a minion of the reactor's predator
  successfully bled the reactor (a redirected bleed included), as Ilonka's;
  the predator burns 1 pool, an effect (78.29).
- Night Moves: the bleed amount kept, only the pool lost capped at 1 (LSJ
  20030211) (80.23).

### 11.2 Hunt

- What a hunt is, successful hunts, card hunts: §3.1 (72.1–72.5, 40.2,
  32.13, 63.12, 64.1).
- Adisa "successfully hunts" only when the hunt gains blood (17.3, advanced
  rules §3.7.2.1).
- Abactor: +4 hunt is 5 blood with the hunt bonuses on top (Aaron's Feeding
  Razor, Hospital Food); "successful resolution" is an unblocked action that
  gained blood (LSJ 20090411) — at capacity or diseased, no blood hunt; the
  blood hunt is part of the action, before its after-resolution window
  (Freak Drive, CrimethInc. after it; Heidelberg Castle not during it), its
  rotation from the hunting vampire's controller; no diablerie is recorded
  (Carlton Van Wyk, Rebirth); a special hunt, once per minion each turn,
  open to a vampire that must hunt and to a Ravnos under Week of Nightmares
  (Week's own steal does not combine) (63.2).
- Giovanni del Georgio's +2 hunt is 3 blood (59.1). The Hungry Coyote's +1
  hunt: a hunt taking blood from a target takes 1 more from it (RTR
  20030519); Psychophagia, which eats an ally from the ash heap, gets 1 more
  from the bank (53.8). Aaron's Feeding Razor: +1 to every hunt of its
  bearer, mandatory (63.16).
- Hospital Food: any Methuselah's anarch's hunt (49.5); its lock "when an
  anarch announces a hunting action" is its controller's option, on another
  Methuselah's anarch too; a hunt bonus of that action — a blocked hunt gains
  nothing; one at capacity has nothing to add though successful (63.12, LSJ
  20050720-2).
- Week of Nightmares: a Ravnos that must hunt must take the special hunt;
  with no target it is frozen (takes no action; other minions act); at the
  end a Ravnos pays its nightmare blood if it can, burned only if it cannot;
  "only one … per game" counts a play canceled as played (28.8).
- Loki's Gift [PRO]'s bite is directed at the bitten vampire's Methuselah
  when another's (68.17); Volo hunting with it at 0 stealth (72.1).
- Vampiric Disease: "cannot gain blood by hunting" covers every hunt —
  rulebook, granted or card hunts; a hunt stealing blood takes none (59.4).
- Ariadne: her "+1 hunt" adds to every hunt of hers, non-standard ones too
  (RTR 20030519) (64.1).
- KRCG (not in rules-qa): a hunt stealing blood must target a vampire, one
  with no blood allowed (LSJ 19970224); stolen blood the vampire cannot hold
  goes to the bank (LSJ 20010809-1).
- Pulled Fangs: "if they must hunt, they go to torpor instead" — in its
  controller's minion phase, a ready unlocked bearer that must hunt (no
  blood, or made to) goes to torpor at once, read live (74.4).
- Festivo dello Estinto: every Methuselah's Sabbat vampires; a successful
  hunt, card hunts too, tops up from the bank, not the hunt's target (RTR
  20030519); a hunt gaining no blood is not successful (LSJ 20050720-2)
  (77.30).

### 11.3 Equip

- Equip from hand or from another minion (one's own: NRA-exempt and
  repeatable, 1.4); from a minion in torpor too; "only one vehicle": a
  second vehicle never offered (from hand, another minion, or two in one
  equip); a blocked equip action leaves the equipment (9.2).
- Searched equipment (Magic of the Smith, Michael Harris — the search is an
  equip action, TOM 19960130): only equipment the vampire can equip and
  whose blood cost it can pay, with the reductions it could take (a search
  may find nothing, PIB 20121028); a pool cost that ousts is allowed (9.3,
  38.5). Vast Wealth ("the first equipment you find"): no choice; if
  requirements are unmet, it cannot be placed or its blood cost is
  unpayable, it is burned without cost (TOM 19950407); a pool cost that
  ousts is paid (RTR 19941109) (9.3); a unique already in play under its
  Methuselah is burned as unplaceable (12.3).
- Concealed Weapon: the weapon named as the card is played, still in hand
  until it resolves (LSJ 20100130); "cannot cost 3 or more pool" reads its
  cost as modified in play, not the reductions announced with it nor
  another source paying it (Ravnos Cache's counters pay a .44 Magnum, never
  let in an AK-47, LSJ 20080125); "inflict (with a regular strike)" reads
  the card's printed strike, the minion's strength aside (Righteous Blade's
  strength+1 is 2); requirements apply (a Salubri's Righteous Blade); not
  the equip action — the minion may still equip that turn (LSJ 20080815) —
  and no action-keyed reduction (Charisma) (63.9).
- Disguised Weapon [OBF]: played as the strike is chosen, the vampire then
  chooses its strike (the weapon's included); a weapon whose cost cannot be
  paid is not offered (58.17).
- British Museum's unlock: once each turn per minion; the minion must have
  successfully performed the equip action with such a card from hand
  (35.3). Hackerspace: "each Anarch you control" counts Anarch vampires in
  torpor too; an Anarch "comes into play under your control" as it is
  moved out of the uncontrolled region; its counters pay any equip an Anarch
  of its controller's makes, no lock, as Ravnos Cache's are spent (58.12).
- Confiscation: the chosen vampire takes the equipment only if it can hold
  it ("a minion can have only one …"); otherwise nothing happens (51.3).
- Stored cards an equipment takes as equipped: §19.4. The equipment's own
  readings: §16.3.
- Dark Web Procurer: an equip action (TOM 19960130), not the same action as
  equipping from hand (ANK 20180817); a non-unique equipment of 1 or fewer
  pool, a blood cost no pool cost (caps read narrowly) (80.28).

### 11.4 Employ and recruit

- An ally or retainer put in play by another card is not recruited or
  employed (D2, §3.7): it may act the turn it enters (40.10, 44.4); Pack
  Alpha's retainer is paid and put in play, not employed — employ-action
  riders (Feral Hound's unlocks) do not apply (22.2); Piper's actor (28.10).
- Recruit and employ reductions: Inyanga (59.5); Charisma's "recruit ally
  actions" needs the normal action (36.5).
- Third Tradition: Progeny: "cannot perform actions this turn" — as a
  recruited ally; the blood moved over its capacity drains (70.18).
- Fleshforge Chamber: a ghoul put on the Chamber from hand is replaced; one
  played from the Chamber is not (it did not leave the hand); once each
  master phase per copy (37.1).
- KRCG (not in rules-qa): "can act the turn it is recruited" does not let
  an ally act in an opponent's turn (ANK 20200930); an ally may use "lock
  this ally" abilities the turn it enters (ANK 20180517).
- Charnas the Imp: put on any minion, an employ action whatever the target
  (ANK 20230408), directed at another Methuselah's minion's controller (+1
  stealth: the employ's own), on the target only when played (else on the
  playing minion, LSJ 20090824); controlled by its employer's controller; 1
  damage in that controller's unlock phase, out of combat; the employer
  about to be burned, its controller may put Charnas on any other minion in
  play, any Methuselah's — declined, it burns; an oust removes it (ANK
  20180408) (76.8).
- Nosferatu Bestial's unlock-phase employ: once each unlock phase, not an
  action nor an employ action (LSJ 20080815), its own Disciplines pick the
  version (LSJ 20080803), requirements not as an action's (LSJ 20100725), no
  action's cost reduction (LSJ 20090115-1), not played by the minion (ANK
  20201229) (80.39); not offered with no employable retainer in hand
  (80.40).

### 11.5 Steal actions

- Steals of blood, life or cards: the target as it is at resolution, what
  is there taken (17.4). Apolonia Czarnecki: any younger vampire, torpor or
  no blood included; excess over capacity drains off (17.4). Gregory
  Winter's steal at a vampire with no blood is legal and steals nothing
  (RTR 20010711) (63.16). François Villon (G2): how much he steals, 1 or 2,
  is chosen at resolution, among what the target has left; the excess over
  his capacity drains (62.4). Goratrix: a ranged steal of blood or life, not
  damage (62.17). Stealing an ally's last life burns it (32.4, 71.13:
  Gabrielle di Righetti, any Methuselah's ready minion, undirected on her
  own).
- Stolen equipment changes host (controller follows, owner stays), no
  requirement checked; a locquipment is a location: not stealable as
  equipment nor taken by the equip action, but stealable by cards that
  steal locations; a steal that would give a second vehicle is not offered
  (17.4). Darius Shirazi's "non-weapon" excludes melee weapons; Camera
  Phone and Laptop Computer may join the pool (17.4).
- Steals from one's own minion are undirected (17.4) — against the
  rulebook glossary's "you cannot steal a card from yourself" (§23).
- Graverobbing: the stolen vampire keeps its cards and blood; burned later,
  it goes to its owner's ash heap; master cards on it stay their player's,
  minion cards move with it; [DOM]'s move is offered at resolution only if
  the acting vampire has the 2 blood (31.5).
- Dominate Kine [DOM] stealing Living Manse: the new controller chooses
  the new host as the action resolves among their ready minions able to
  hold it (the requirement not rechecked); burned if none can (31.4, 39.4,
  LSJ 19990928).
- Taking control of minions: §17.2.
- Vivienne Géroux's steal: only by a vampire of another Methuselah (80.31).

### 11.6 Leave torpor, rescue

- The cost is paid at resolution; unpayable then, the action fizzles —
  successful, not blocked (Freak Drive playable), no effect (3.6). Out of
  torpor by resolution: 28.7 (§10.2).
- Rescue: who pays (the acting vampire, the rescued one, or a split) is
  chosen as the action is announced; the blood is paid as it resolves; the
  cost is read again at resolution, the split standing: a share its payer
  can no longer meet fizzles the action (48.8).
- Torpid Blood's "actions performed by vampires in torpor": the leave
  torpor action and action cards played by a torpid vampire; +1 blood once
  (40.6).
- Venere Carboni: once in each of his controller's unlock phases (71.27).
- "Cannot perform the same action again": rescuing a different vampire
  stays open (23.2). Directedness: rescue of another Methuselah's vampire
  is directed (16.4).

### 11.7 Diablerie and the blood hunt

- A diablerie is a directed action at the torpid vampire's controller when
  another's (16.4, 41.5); a blocked leave-torpor action lets its blocker
  diablerize (16.2: Queen Anne can unlock after a diablerie as the blocker
  of a leave-torpor action too). Gone from torpor by resolution: fizzles,
  no blood hunt (28.7).
- The blood hunt is inside the action; the after-resolution window comes
  after it (5.8); between the diablerie and its blood hunt Slake the
  Thirst has a window of its own (68.2, ANK 20201228).
- "Burn a political action card for 1 vote" is available in a blood hunt
  too, one per Methuselah per referendum (5.2); a blood hunt is a
  referendum for "any referendum" abilities (15.2) but no political action
  (51.2) and not "called by" the vampire it is called on (41.3, 42.4,
  59.7).
- Blood cursed (Alu, Wise Frog; rulebook "Blood Cursed"): the vampire
  commits no diablerie — no diablerie action, no diablerie of a blocked
  leave-torpor vampire, no Mask take-over of a diablerie; Abactor's blood
  hunt and a rescue stay open to it (63.1).
- Ariadne: no blood hunt is ever called on her (64.1). Abactor's blood hunt
  is no diablerie (63.2).
- Carlton Van Wyk: his burn may target any vampire that committed
  diablerie since his controller's last turn ended, his controller's own or
  one in torpor included (44.6).

### 11.8 Entering combat as an action (rushes)

- Target a ready minion of another Methuselah (not in torpor), locked or not
  unless printed (Nathaniel Bordruff: locked; Adisa: younger); directed at
  its controller, who alone may block. Unblocked: combat at resolution, part
  of the action, acting minion first; the combat has its after-combat
  window, then the after-resolution window. Target no longer qualifying at
  resolution: successful, no effect (17.3). From an action card: 26.1.
- Rushes printed or granted: Theo Bell, Barachiel, Dafina Hanganu,
  Nathaniel Bordruff, Adisa (17.3); Deep Song (27.2, 27.5); Mobile HQ
  (38.6); a regent's (52.6); Tupdog (62.14); Renegade Garou (+1 stealth at
  another Methuselah's minion, 63.8); Herald of Topheth (62.33); The Red
  List (67.5); Ossian's at any vampire, any Methuselah's (64.6).
- Reginald Moore: 58.6. Obedience against a rush's combat: 55.7, 56.2.
- Coagulated Entity: the blood moved as the action resolves, before the
  combat; any chosen Blood Brother unable (no blood, gone) and the action is
  canceled: no combat; the +X capacity lasts until the action ends, the
  excess then drains (68.6).
- Taunt the Caged Beast [ANI]: both vampires chosen as it is played, the
  prey's then the predator's; it targets both — directed at them and their
  controllers, both of whom may block; no combat if either is no longer
  ready as it resolves; the 4 blood up to capacity (71.21, owner
  2026-10-14).
- Powerbase: Luanda: the chosen vampire (torpid too, D28) enters combat as
  its action once that turn while the card stays locked (it unlocks in its
  controller's unlock phase); the card stolen or burned, the choice is gone;
  the target is another Methuselah's minion only (17.3) (77.16).
- Sense Death: [ani] a younger vampire, [val] any minion; the press and
  [VAL]'s maneuver in the action's own combat only (80.21). Rumble: a ready
  minion, a locked one too; +1 hand size in its combat only; the pool burned
  if the actor is not ready at the action's end, after its combats, blocked
  or not (80.27).

### 11.9 Other rulebook actions: become Anarch, merge

- Become Anarch (advanced rules): a rulebook action — a ready untitled
  non-Anarch vampire, +1 stealth, undirected; offered only if its cost is
  payable as announced, the cost paid at resolution (2 blood, 1 if its
  Methuselah controls another ready Anarch, read then). Brunhilde's "costs
  vampires you control 0 blood" reaches her Methuselah's other vampires,
  read at resolution, in torpor too, not while contested (58.1). It is an
  action requiring an Anarch for CrimethInc. (62.21).
- Merging: §14.9.
- Taking control as an action (Puppet Master, Ayelea, Chameleon): §17.2.

## 12. Combat

Advanced rules §4 is the normative source (§4.1 sequence, §4.2 range, §4.3
strikes, §4.4 damage, §4.5 prevention and immunity, §4.6 dodge, §4.7
"combat ends", §4.8 presses, §4.9 end of round and new combats, §4.10
weapons). The round: before range, determine range (maneuvers), choose
strikes, resolve strikes (prevention), additional strikes, press, end of
round (rulebook "Combat").

### 12.1 Structure, sides, who plays

- Impulse in combat: 1.2 (§5.1); the acting minion and the opposing one
  (§3.21); a combat with no acting minion (1.2). Deep Song moves the acting
  side (27.5); Hedonism's vampire is its queued combat's acting side (53.3);
  The Guardian's combat: the bleeder acts first (41.5).
- A combatant no longer ready (burned, going to torpor) may play
  end-of-round combat cards unless the card excludes it (Taste of Vitae,
  Disarm), and no other combat window (4.5).
- A stolen combatant's combat ends after the damage still pending resolves
  (27.3, LSJ 20020618). A combatant's Methuselah ousted ends the combat at
  once (45.4). A new round started (Telepathic Tracking) with a combatant
  gone in the same window: the combat ends instead (29.9).
- Minions out of the combat: not offered abilities there (15.2) unless the
  card says so — Martyr's Resilience (29.1), Randall (59.8), Tommaso Sforza
  (41.10), Octopod, Three's a Crowd, Coordinate Attacks ("another one (you
  control) in combat": its player's combatant, the card played by a minion
  out of the combat) (68.4); Mob Connections, its controller in the press
  step while a minion of theirs is in combat (33.6); Roger de Camden (41.9).
- A minion in combat with Egidia Arrú leaving the ready region (burned or
  torpor, by anything during the combat) costs its controller 1 pool, even
  if Egidia falls in the same event (16.5, advanced rules §6.4.4).
- Harold Tanner: the hand of the controller of the minion in combat with
  him is open to every Methuselah for that combat (71.17).

### 12.2 Before range

- Show of Force's "can gain +N strength": a choice offered before range in
  round 1 of the block's combat (26.2).
- Weather Control's before-range damage is successfully inflicted, so the
  round's first (29.4); Alpha Glint [ANI][FOR] ends the combat in the
  before-range step of any round, Weather Control's damage of that step
  landing first (LSJ 19971110), the range staying at its default (63.4).
- Powerbase: Savannah: the location locked is another unique location its
  controller controls, unlocked; usable in the before-range step of any
  round of a combat whose acting side is its controller's vampire (44.7).
- Tommaso Sforza: before range, a combat whose combatant on his
  Methuselah's side is a wraith or zombie ally, from out of the combat;
  once each turn (41.10). Zip Gun: before range is determined in any round,
  put on the minion (64.10).
- Vampiric Disease's spread happens at close range as the before-strikes
  step opens, between two vampires, either way (59.4).
- Typhonic Beast: before range; +1 strength this combat; [pot][ser] 1 damage
  prevented each round of it, any damage (Horrid Form's); [POT][SER] a press
  this round (78.24).
- Outside the Hourglass: [obf] a dodge without Temporis (25.2); [TEM] before
  range, the vampire's 2 damage, not environmental (PIB 20120426),
  preventable, staged with the other pre-range damage the combatants inflict
  (ANK 20170930), still dealt if the combat ends early (ANK 20180104); one
  superior play per vampire each round; Weather Control's damage that round
  merges with it in a single damage window, unpreventable, added after the
  prevention (owner correction) (78.32).

### 12.3 Range and maneuvers

- A strike card with a maneuver (Slam [POT], Aid from Bats) played in
  determine range gives the maneuver at once and commits the round's
  initial strike (only that strike offered at strike choice); the card goes
  to the ash heap as it resolves; Slam's maneuver only closes from long
  range; not playable at the maneuver while the strike could not be used
  (22.1, LSJ 20021028). Slam [POT] ("as above, with 1 optional maneuver") is
  played only at the maneuver, never as the strike (48.4); Aid from Bats and
  Diversion stay playable as the strike (2.5, 48.4).
- A gun's "1 optional maneuver each combat" (.44 Magnum, AK-47, Assault
  Rifle) commits the round's initial strike to that gun: no other initial
  strike that round, none at all if an effect then bars it (Immortal
  Grapple), no second strike card or weapon maneuver that round; additional
  strikes free; the allowance tracks the gun (10.1, LSJ 20090528-2-2, PIB
  20141026, LSJ 19980302-2). Deer Rifle: two maneuvers each combat per copy,
  the second committing the strike to the same gun again (71.5). Zip Gun's
  maneuver commits the strike (64.10, LSJ 20021028). Lam Into [for] played
  at the maneuver commits the round's strike (the weapon's, if chosen), not
  usable while the strike could not be (57.4).
- Sniper Rifle: offered to the blocker before range in the first round,
  only while no other effect set that round's range, and only if the
  bearer had it when blocking (not equipped afterwards, Concealed Weapon);
  used, it skips determine range (RTR 19970630, ANK 20180720) and commits
  the initial strike to the rifle (10.5).
- Additional maneuvers: Quick Meld [PRO] usable later in the same
  determine-range step (after an opposing maneuver), at either range, once
  this round, 1 blood paid as used (59.2); Dance with the Devil [OBF]/[THA]'s
  extra maneuver only at long range, after the card's own, its blood paid
  as used (54.6); Nose of the Hound [spi]'s "an additional optional
  maneuver" — two in that combat, [SPI] any ready minion (71.20).
- Once-each-combat maneuvers: Abaddon, Saku Pihlajamäki (14.2); Renegade
  Garou (63.8); Make the Misere [cel]'s maneuver is one for the combat
  (31.7); Ruins of Ceoris: one use over the combat, a maneuver (before or as
  range is determined) or a press, a Tremere antitribu bearer only, read
  live (62.27). Roger de Camden's maneuver is given and used as the range is
  determined, for any minion of his Methuselah in combat, himself included;
  once each turn (41.9).
- Mind of the Wilds: the maneuver banked for the first round of the block's
  combat only; the blocker barred from "strike: combat ends" for the rest
  of the action, any round of any combat (64.5, LSJ 20100308).
- "Cannot maneuver" (Ni Dieu ni Maître [ani]): no maneuver at all, granted
  ones included (70.12). Ossian's freeze: a vampire opposing him that played
  a card requiring Auspex in the current action (any action, LSJ 20050201;
  the Discipline the card was played with) uses no maneuver nor strike for
  the rest of the first round — from the combat's start, or from that card
  on mid-round (an Auspex strike played first stands, ANK 20210614); it may
  still play an effect setting the range that is not a maneuver, not a
  Sniper Rifle (a strike commitment, LSJ 20100827), the range set once
  (64.6).
- Thrown Gate: a ranged strike with an optional maneuver, committed at the
  maneuver as Aid from Bats ("an optional maneuver" read as the window
  matrix's maneuver rider); no maneuver without the strike (LSJ 20021028)
  (78.23).
- IR Goggles: a maneuver each combat, any round, any bearer (80.16).

### 12.4 Choosing strikes

- A strike must be chosen, default hand strike; "no strike" is rare: a
  committed strike that cannot be performed, or a card letting the minion
  not strike (4.3). Ineffective strikes stay legal (a hand strike at long
  range, a dodge with no opposing strike) (4.3). A strike naming the kind
  it acts on (Entombment's "the opposing ally", Coma's "the opposing
  vampire") is not chosen against the other kind (76.11).
- A strike's own effect on its striker happens only if the strike resolves:
  dodged or the combat ended first, Molotov Cocktail is not put on, nor
  burned "after use" (76.10).
- Range: close unless the strike says (D23; 21.1). Heroic Might's "strike:
  burn equipment": close range, the equipment named as the strike is
  chosen, a dodge stops it (26.6). Kaymakli Fragment's "an opposing minion
  can strike: steal this equipment" — the strike of the minion in combat
  with its bearer, at close range, dodgeable (71.23). Fast Hands' "steal
  weapon": the weapon named with the strike moves to the striker as it lands
  (a steal: no requirement or cost), not if the striker cannot have it;
  with first strike, a strike with that weapon resolves no more (70.11).
  Canine Horde [ANI]: the opposing minion's equipment named with the strike
  (Weighted Walking Stick included, a location not); with first strike, at
  either range; not offered against a minion with none (RTR 19980928)
  (62.18). Donnybrook [obf]: the equipment or retainer is the opposing
  minion's, named with the strike; resolved with first strike, it burns
  before the opposing strike resolves (a weapon so burned strikes for
  nothing), unless dodged (44.9). Voracious Vermin [ANI]'s additional strike
  (burn weapon): the superior version only if the opposing minion has a
  weapon; not optional once chosen; none if no weapon is left by then; any
  range; a dodge stops it (22.3).
- Strikes with a price: the unnamed's strike blood paid as it is chosen, not
  refunded if dodged; 2R aggravated, effective at any range (62.16);
  Preternatural Evasion [CEL]'s blood burned as the strike is chosen, so
  not offered to a vampire with none nor to an ally ("this vampire"); a
  burn, not the cost (63.5, ANK 20200703-2). Djeneba: "strike cards" are
  cards played in a strike mode, a dodge included; a combat card played
  otherwise is untouched, and so are her own; the minion in combat with her
  pays, whether she acts or blocks; an ally pays in life, its last one too —
  it burns and the combat ends (ANK 20180928-1); a strike card its player
  cannot pay for is not offered (42.2).
- Printed strikes: Contagion [DAI]'s strike, close range, once over the
  combat; its counter is the striker's controller's; the steal optional once
  their counters equal the minion's capacity or cost (35.2), after the
  strike's resolution, damage pending applied first, the combat ending
  (62.19). Nephandus's strike 1R (62.24). Flávio Gonçalves', Phaibun's
  "once each combat, strike: dodge" is chosen as the strike; Immortal
  Grapple bars it; Phaibun's random discard happens as the strike is
  chosen, then replaced; not offered with an empty hand (14.2).
- Anticipation [aus] "use a melee weapon strike": the weapon's own strike,
  riders included (Righteous Blade's press), at +2 damage (weapon gone by
  resolution: no effect); [AUS] also cancels a strike card played at the
  maneuver (Slam) (29.7).
- Strength: "+1 strength" adds to base 1 or to a set strength (Torn
  Signpost); "in combat with X" reads the opposing minion (13.2). Kasim
  Bayar: political card picked as used, then replaced; +2 strength for every
  round of that combat, adding to a set strength (14.2). Gabriel Tremblay's
  discard for +1 strength: untimed within the combat, once each combat, for
  every round left (57.3). Monstrous Form's "+1 strength this round" at any
  combat impulse before strike resolution (26.7). Make the Misere [pot]'s
  +1 for the whole combat (31.7). Street Cred: "younger" than the vampire
  playing it (the blocker too); the damage compared is the strikes'
  successfully inflicted this round; one each turn per vampire (60.11).
  Bone Spur: untimed — before the strike it should affect resolves, after
  both strikes are chosen too (RTR 19960112); [pro] this round's hand
  damage, [PRO] every later round's as well (64.3). An Anarch Manifesto's
  strength to a titled non-Anarch vampire in combat with the bearer, read
  live (71.14).
- Lam Into: each Discipline is the hand strike or a melee weapon of the
  vampire's at that bonus; "do not replace until after combat" is replaced
  as the combat ends, before the after-combat window (57.4).
- Island of Yiaros: X is the votes the vampire holds — title, printed, borne
  and its clan's (New Carthage) — a priscus's ballots none (RTR 19970630);
  for the controller's own striking vampire, after both strikes are chosen;
  "for that strike" is this round's strength (an additional strike that
  round gets it too); Adhiambo's "actions to enter combat with a minion"
  include the Red List hunt (74.9).
- Mantle of the Bestial Majesty: the vampire's hand strikes are aggravated
  until its controller's next unlock phase, as blocker too, its other
  strikes not; [PRO] +1 strength as long (Gear Up's duration) (78.35).

### 12.5 Aims and plays on a chosen strike

- Aims (Target Vitals): the strike is chosen first, then an aim may be
  played on top of it — the striking minion's Methuselah gets one decision
  after its strike choice, an aim or pass, before the strikes resolve; a
  strike committed at the maneuver likewise (62.1, owner 2026-10-11).
- Target Vitals: the aim's +2 damage is added once the strike's damage is
  successfully inflicted (after prevention, so it cannot be prevented), of
  the strike's kind (RTR 19970630); "the opposing minion cannot press this
  round" only when the aim lands (55.1). Under Immortal Grapple it can be
  played on a hand strike; on a committed strike (a .44 Magnum's) it is
  played in the strike phase while that strike can still be used; a grapple
  forbidding it, the committed strike is not usable and Target Vitals
  cannot be used on it (55.8, LSJ 20080409).
- Rötschreck is played once both strikes of a pair are chosen (before they
  resolve), additional pairs included; "an opposing minion attempts to
  inflict aggravated damage" reads the chosen strike (aggravated at its
  source, effective at the range, not aimed at a retainer) (44.8).

### 12.6 Strike resolution, first strike

- First strike: the strike resolves alone; the opposing strike waits for
  its damage to be prevented and applied, and is lost if its minion is no
  longer ready (burned, in torpor) or the round is over; a dodge opposing it
  still resolves at once (it protects); both with first strike resolve
  together (58.2). Wise Frog: first strike in a combat with a vampire
  younger than her, read as the strikes resolve (63.16).
- "Strike: combat ends" resolves first, before any first strike (rulebook
  "Combat Ends"), so no damage lands beside it (63.4); Saulot's Guiding
  Wisdom ending a combat: no strike resolves (advanced rules §4.7.2), the
  end-of-round window still opens (advanced rules §4.9) (19.5). Memory Rift [pre]: "strike:
  combat ends" offered in the first round of the combat its block starts
  (62.23).
- A strike's riders keyed to landing need it to land: "put this card on the
  opposing minion" (21.1); a dodge stops strikes that burn or steal (22.3,
  26.6, 44.9, 71.23).

### 12.7 Damage

- "+1 damage from each strike or damaging effect against" a minion (Dam the
  Heart's River) reaches every damage it takes in that combat: a strike's,
  a retainer's, environmental, an effect's in combat, and after-combat
  damage played in it; never a non-damaging strike; the extra of the base
  damage's kind (84.6). Two plays of an uncapped extra add up (84.8).
- Mixed normal and aggravated damage: when a prevention covers only part,
  the preventer chooses which points go unprevented (one may want torpor
  with no blood lost); non-aggravated-only prevention (Soak) never touches
  aggravated points (4.2, LSJ 20040812-2).
- Strike damage to a retainer waits with the combatants' damage (prevention
  step) and is applied with it; life stolen from a retainer moves at once
  (29.2).
- The preventing player chooses what they prevent (strike vs other damage,
  environmental); Disarm's "successfully inflicted more damage" counts strike
  damage landing after prevention, all the round's strike pairs (29.3).
- Flesh of Marble applies as damage lands, after every other prevention
  (29.4).
- Damage bonuses: Noluthando's +1 on each of his ranged strikes (weapon or
  card) at either range, not hand or melee; on a gun it is gun-strike damage
  (14.2). Target Vitals' add-on: 55.1. Psychic Assault is normal at its
  source — an aggravating effect (Dawn Operation) leaves its damage normal;
  Black Hand read on the striker as it resolves; unpreventable damage is
  still reduced by Nephandus (LSJ 20100519) (62.26).
- Nephandus's reduction is its own step, before prevention, for each strike
  of the opposing minion — hand, melee weapon, ranged, unpreventable
  included — and a strike's damage after combat (Catatonic Fear); not
  environmental damage (Carrion Crows), nor an aim's add-on (62.24).
- FBI Special Affairs Division: the 2 damage dealt as the combat ends,
  before after-combat effects (40.9). Zip Gun: its bearer takes 1
  environmental damage the first time it strikes with it each combat, none
  if the gun is gone first (LSJ 19970801, LSJ 20100310) (64.10).
- Damage out of combat: Neighborhood Watch Commander's 1 normal damage (the
  vampire burns a blood, else goes to torpor) (65.2); Alonso Petrodon's
  (57.1).
- Advanced rules §4.4 (sources, environmental damage, aggravated, one
  block of damage resolution).
- Dark Steel: [obt][pot] a combo (25.2); its environmental damage is dealt
  in normal strike resolution at close range each round of that combat,
  against a dodge too; none after the combat ended or a combatant went to
  torpor by first strike (RTR 19941109, ANK 20200422) (77.21).
- The Dracon: +1 damage on a ranged strike that deals damage, +1 blood or
  life on one that steals, each where the strike does it, at close range
  too; none on a strike dealing none (RTR 19960221); [vic] the played mode
  (77.22).
- Selena: a werewolf's strike damage on her is aggravated (no werewolf in
  the pool deals other damage); Righteous Endeavor's "infernal" is the
  printed trait (77.23).
- Franciscus: burned, not sent to torpor, once more than 3 damage has landed
  on him in one combat — after prevention (29.3), every round and kind
  (78.19).
- Domain of Evernight [tem]: all damage to vampires in the combat its
  blocked action leads to is aggravated, both sides, environmental too, none
  after the combat (Dawn Operation's, LSJ 20020314); [TEM] after a
  successful action, still in it (LSJ 19981028, ANK 20190425); one superior
  play per vampire each turn (78.27).

### 12.8 Prevention and immunity

- Prevention cards are played only with damage to prevent (advanced rules
  §4.5.1; LSJ 20001114). Rolling with the Punches [FOR] covers the opposing
  minion's additional strikes later in the round too; not environmental or
  retainer damage; playable only with strike damage pending (21.3).
- Kevlar Vest: gun or non-gun branch chosen once each combat; the
  prevention may be split across rounds and exchanges (LSJ 20081210; ANK
  20200318, LSJ 20001114); leftover gun points prevent nothing else; gun
  damage not from its strike is not "damage from gun strikes" (10.3).
- Martyr's Resilience ("an unlocked vampire not involved in the combat",
  any Methuselah's, ANK 20181101): after both combatants' prevention, every
  Methuselah in turn clockwise from the acting one may prevent with such a
  vampire, on either combatant or a retainer on one; a combatant's
  controller may also play it in their own prevention; mixed-damage
  allocation is the Martyr's player's (29.1). Randall: out of the combat
  only, in the step for minions out of the combat, any Methuselah's combat,
  locked or in torpor too, once each turn, only while non-aggravated damage
  is pending on the target (59.8).
- Printed prevention: Opikun usable in torpor (LSJ 20011022) and on
  himself, blood burned as used, only while non-aggravated damage from the
  opposing minion's strike is pending on that vampire in its prevention step
  (LSJ 20040812-2), never on other damage (Carrion Crows'); Rowena 1
  non-aggravated once each round, in her prevention step (14.2); Lodin only
  damage from an opposing minion's strike, aggravated first (no allocation
  asked), to Lodin only, once each combat (59.10); Omaya any damage pending
  on her, once over the combat (62.17); the Great Beast any damage, once
  each combat (60.20).
- Beast Meld [ani][pro]: prevents 1 damage, once, at any point of the
  block's combat (any round) (30.6). Unflinching Persistence [FOR]: the later
  prevention is optional, any damage pending on it this round (aggravated
  included), once, lost as the round ends (58.18).
- Immunity is not prevention: Valerie Diaz's ("immune to all
  non-aggravated damage from a strike or damaging effect") is used at the
  prevention step, for the opposing strike's normal damage or the other
  normal damage pending that round taken as one damaging effect;
  "cannot be prevented" does not stop it; "once each round": one use a
  round, for a single strike or damaging effect (38.3, 38.8; 38.8
  supersedes 38.3's "multiple times").
- Carolina Vález: immune — the damage of allies and retainers, any
  Methuselah's, is not inflicted (LSJ 20010627); an ally's weapon damage is
  the ally's (76.19).
- Mythic Form: played before range in the first round, one each combat; +1
  strength and 1 non-aggravated damage prevented each round, any damage,
  never aggravated (LSJ 20040812-2); [PRO] also a maneuver this round and a
  2R aggravated strike once each round (78.33).
- Stunt Cycle: at long range only; its prevention of the opposing strike's
  damage covers what lands (RTR 20041202), none carried over (ANK 20200318);
  an opponent with Celerity may burn 1 blood to prevent the whole strike, no
  cost (ANK 20210226), never an ally's life (§3.14) (corrected: not any
  minion with Celerity) (78.34).

### 12.9 Additional strikes

- Played only after the first strike exchange of the round, both combatants
  still ready — never before strikes nor before the first pair resolves
  (47.2, overriding 4.4; rulebook "Additional Strikes": "announced and
  performed only after the first pair of strikes is completed"); one at a
  time: a combatant with an additional strike still to make gains no other
  until that pair resolves; after each additional pair, a new chance; the
  opponent may still answer in the same window, the acting minion first
  (48.2). The general rule for every card and effect granting additional
  strikes (47.2), Chun Hei's ability likewise.
- The limit: one limited additional strike source per combatant a round.
  AK-47's additional strike comes once the gun's strike resolved, dodged or
  not (not if a "strike: combat ends" resolved first), and is the bearer's
  one limited additional strike that round (none if Blur already used; Blur
  not playable after); in that pair, a gun strike or no strike (10.2). Chun
  Hei's "does not count against the limit": a limited additional strike
  card still fits that round (14.2). Lightning Reflexes [CEL] with X = 0:
  playable; no additional strike, no limited slot consumed (29.8).
- Mandatory additional strikes: Renegade Garou's each round, right after
  the first pair, taking the round's limited slot (63.8, PIB 20121028);
  Octopod's strikes each round after the normal pair (mandatory, limited)
  (68.4).

### 12.10 Dodge

- A dodge is a strike card (42.2); with no opposing strike it is a legal
  no-op (4.3). A dodge opposing a first strike resolves at once (58.2). A
  dodge stops a strike's burn, steal or put-on riders (21.1, 22.3, 26.6,
  44.9, 70.11, 71.23); Righteous Blade's press is kept if the strike is
  dodged (10.4). Advanced rules §4.6.

### 12.11 Presses

- Adrino Manauara's mandatory press: at the press step his controller must
  use it to continue before passing, unless a press to continue already
  stands, in which case it waits for a later round (14.1).
- Righteous Blade's press: had once the strike is chosen — kept if the
  strike is dodged or does not resolve; once per combat per Blade (10.4).
- Press riders on strike cards are played at strike declaration (2.5).
- Trap and Conquer the Beast: put in play (on no minion) by the combatant's
  Methuselah; any card played in that combat by anyone resets the counters,
  a canceled one included; each burns a counter as each press step opens,
  both combatants ready — Trap's press to continue is no combatant's and
  applies first, a second Trap's adds nothing (its counter still burns);
  Conquer's is its vampire's for that round (mandatory at [ani], optional
  at [ANI]), and its vampire cannot use equipment for the whole combat;
  burned as the combat ends (59.17).
- Drawing Out the Beast [ANI]'s damage lands as the press step begins,
  before any press (70.16). Mob Connections: one press to continue (33.6).
- "Cannot press this round" (Target Vitals) only when the aim lands (55.1).
- Once-each-combat presses: Abaddon, Saku (14.2), Jacob Fermor (64.1),
  Nephandus (62.24), Ruins of Ceoris (62.27).

### 12.12 End of round, end of combat, after combat

- The end-of-round window follows the press step in every round, a press
  continuing the combat or not (rulebook 2024 "End of Round"): Taste of
  Vitae after a round a press continued (61.1). It opens after a combat
  ended early too (19.5).
- Hell-for-Leather [obt] ("only usable if this Anarch is ready and the
  opposing minion is not"): in the end-of-round window, the round's damage
  applied — the only point a combat leaves the opposing minion not ready
  (sent to torpor or burned) with combat cards still playable; once per
  combat, per Anarch (44.1, 75.3).
- Against an opposing minion the round burned: a card put on it (Disarm,
  Pulled Fangs) is not offered, no target left (75.1); one reading what the
  round did and targeting elsewhere is (Taste of Vitae, LSJ 20031201-2;
  Street Cred, 75.2), and so is Hell-for-Leather [obt] (44.1).
- Orgy of Blood: X is the ally card's printed pool or blood cost; "each
  vampire you control" in torpor too, up to capacity; played in the
  end-of-round window of the round the ally burned (70.15).
- Of the end-of-round cards after a "combat ends" strike (RTR 20001020),
  Taste of Vitae is offered (for nothing), Disarm never is (it needs damage
  inflicted that round) (63.4).
- Psyche! [CEL]: in the end-of-round window as the combat would end, both
  combatants ready, no combat queued (RTR 20020501) — no "would end"
  replacement after it (Telepathic Tracking, Hunting the Quarry: ANK
  20191219), the end-of-round cards still open before or after it (LSJ
  20021113). The queued combat begins as this one ends, the same two
  minions, still in the action (LSJ 20021213), ending as this one would
  have; this combat's after-combat effects are lost (a continuation as if
  unblocked, Torrent's included; damage or a card after combat), unlocks
  aside, applied as their strikes resolved (LSJ 19980109); cards held "until
  after combat" are replaced before it (LSJ 20001122); no block began it, so
  no block rider nor "when blocking" equipment carries over (Spirit's
  Touch, Sniper Rifle: LSJ 20010813, LSJ 20010814); a hand size the first
  combat lowered is drawn up between them and discarded down again (Raptor,
  LSJ 20030530) (65.1).
- Queued combats: Hedonism [for]/[pre]: a combat is queued (Sabbat V5 rules,
  "Queue a combat") — it starts after resolving the action, still as part
  of its resolution: the action blocked, after the block's combat; the
  action successful, after its effects; only one can be queued; lost if the
  action ends first (LSJ 20090826) or if either vampire is not ready then
  (53.3); a fizzled action still has it, a failed one loses it (56.1). The
  Status Perfectus's second combat begins as the first ends, the acting
  minion first, then back to the after-resolution window (71.7).
- As the combat ends: Morbidity's blood goes back as the combat ends (after
  every round, a press included), then the card burns; the card takes as
  much of the opposing vampire's blood as it has, up to 2 (55.2). FBI's
  damage (40.9). Lam Into's replacement (57.4). Inner Essence's discount
  (68.12) ends; Trap and Conquer the Beast (59.17) burn. Hunting the Beast [AUS]'s Salubri
  is chosen at the end of the combat; "the opposing vampire is not ready" =
  in torpor or burned (31.3).
- "If the opposing vampire is not ready at the end of combat": Pillars
  Fall's "you can put this card on this acting vampire" whenever the acting
  vampire has none, the card entering play from the action (54.3); Ian
  Carfax's unlock only while he is locked, after a combat he was in whose
  opposing minion is not ready (70.22); Abuse of Power's "if only one
  combatant is ready, the controller of the opposing minion burns 1 pool" —
  the opponent of the ready one (54.7). Matasuntha's pool: optional, for her
  controller as the opposing minion leaves the ready region by any route
  (59.6).
- The Path of Lilith: offered as the torpor lands — the opposing vampire
  sent to torpor in combat (not burned), the vampire opposing it
  non-Camarilla and the card's controller's, the card unlocked; its
  controller's unlock phase: each Methuselah controlling a vampire with a
  torture counter (ready or in torpor), its controller included, burns 1
  pool; every torture counter goes once no Path of Lilith is in play (64.7).
- Ossian's life: the blood the opposing vampire used that round to heal
  damage or to prevent destruction (his card's words), at the end of the
  round, after the presses, the round ended early too
  (RTR 20030519), over his starting life (ANK 20171109) (64.6). Predator's
  Transformation [ABO] at the end of the round in which this vampire's
  combat burned a minion of its controller's prey (64.8).
- Anathema: "reduced to 0 in combat" read as each combat step settles: any
  loss to 0 in it (damage, a cost paid in blood); a vampire back above 0 by
  then is not burned; the pool to the Methuselah controlling the opposing
  minion then (70.7).
- After combat: the after-combat window opens once after the last combat of
  the action (2.1); Form of Mist's continuation inside it (4.1).
- Pocket Out of Time: [obt]'s blood is no cost (ANK 20210226); [TEM] after
  each combat of the action, the one just ended included, 1 blood for a new
  combat with the opposing minion, not with a combat queued (RTR 20020501)
  nor either minion not ready (ANK 20220429) (78.40).
- Righteous Aura: in the end-of-round window (Hell-for-Leather's, 44.1);
  burned before its vampire leaves the ready region (ANK 20180511-1,
  untested) (80.20). Fast Reaction: after the combat between the acting
  minion and the player's blocker; a new combat, the acting minion no strike
  in its first round (nor a strike's maneuver, LSJ 20001126); not with a
  combat queued (RTR 20020501); the block combat's continuation lost, as
  Psyche!'s (65.1) (80.36); not after the combat a Fast Reaction queued
  (the owner) (80.37).

### 12.13 Weapons and equipment in combat

- Guns and their maneuver: 10.1, 64.10, 71.5. Weapons in play: Weighted
  Walking Stick counts as weapon equipment while in play (Blood Rage, Soul
  Burn nullify it; Form of the Bat bars it; Voracious Vermin burns it); in
  hand or ash heap it is just a combat card (22.4); Zip Gun in play is
  weapon equipment and a gun, a target for "equipment" effects (64.10).
- Form of the Bat's "cannot use equipment during the resulting combat":
  all equipment use (18.3).
- Enhanced Coagulant: close range (21.1); a strike nullified for weapons
  (Blood Rage) still puts the card on the vampire; once there it is
  controlled by the controller of the vampire it is on; its burn follows
  any successful hunt of the vampire (38.4).
- Anachronism: on any Methuselah's vampire of capacity 7 or more; "ranged
  weapons" are weapons with a ranged strike (LSJ 20060825), Zip Gun
  included, the Ivory Bow excepted; those on it burn as it enters; it cannot
  equip one (from hand, by an action or another card), move one to it, nor
  use one, nor play Zip Gun (64.2).
- Retainers in combat: strike damage to them (29.2); Stanislava's (49.3).
- Advanced rules §4.10 (using a weapon without striking, weapons leaving).

### 12.14 Frenzy cards

- 28.11 (§3.19). Tranquility Shield vs Terror Frenzy (played on the
  opposing minion): the shielded vampire's opponent cannot play either
  version; a shield played after cancels both versions' effects on the
  shielded vampire (it may maneuver to close, press to continue, use
  equipment; its combat cards cost no more) (29.5); Drawing Out the Beast's
  strength is a frenzy effect it cancels (70.16). Meditative Grove cancels
  by the same reading (28.11).
- Predator's Transformation: the reflex by the vampire the frenzy card is
  played on, locked or not, its cost still paid (ANK 20260216); [abo] +1
  stealth as the action is announced, even if not needed; [ABO] once per
  action (64.8; owner 2026-10-14 confirmed the locked reflex).
- Drawing Out the Beast against an armed anarch restricts its strikes
  (Groundfighting, 71.6); not played against an ally (70.16).

### 12.15 Combat-wide effects and hand size

- Hand size in combat: Rage of Apedemak's +1 lasts the combat, the cards
  then over discarded at once (54.8); a combat-lowered hand size between
  two combats (65.1).
- Infernal Pursuit [CEL]: from its own replacement on, each replacement in
  the combat draws one extra card per copy played, then the hand is
  discarded down; none past the library (LSJ 20060620) or once the combat
  is over; several cards replaced at once (Blood Weakens' held
  replacements; the two cards discarded to cancel Target Vitals) are
  replaced one at a time, each followed by its extra cards and as many
  discarded (ANK 20180925-1: "discarding each time X cards" — even below
  hand size) (63.11).
- Focus the Blood: the card itself goes on the vampire with 1 blood from the
  bank; one play each combat per vampire, copies from earlier combats
  stacking; each copy, burned as its host pays a card played as a combat
  card, takes 1 blood off it, in any later combat too, announced with the
  card (63.7).
- A card granting several strikes (Blur) bars further additional strike
  cards until all of them are made (48.2).

### Open or contested

- Advanced rules §4.3.5 reads a limited additional strike card as playable
  "before or after" the strike it adds to, against 47.2 (§23).
- 29.8 (X = 0 consumes no limited slot) has no ruling; LSJ 20030224 (a
  canceled limited effect does not use the limit) supports it by analogy.

## 13. Referendums and votes

Advanced rules §3.7.5 (referendum procedure), §3.7.6 (votes and ballots).
Vocabulary: rules-qa calls a Methuselah's own votes (the Edge, a burned
political card, a card granting the Methuselah votes) "ballots" (1.1, 41.3,
42.4, 49.2, 51.1); in the rulebook and the advanced rules, ballots are the
prisci's alone (52.9, 70.6) — read "a Methuselah's ballots" in rules-qa as
"a Methuselah's own votes" (§23).

### 13.1 Calling a referendum; terms

- A political action is an action (§8); the referendum is held after it
  succeeds (blocks declined, the would-be-successful window passed: 54.1).
  Numeric and other terms are chosen and announced as the referendum's
  terms after the action succeeds, before votes (3.5: Kine Resources
  Contested's allocation); choosing in terms is never targeting (38.6, D6).
- Terms with no legal choice when chosen: the action fizzles (43.1); the
  play is masked when the terms cannot be met (43.1, LSJ 20100129).
- Per-card terms:
  - Parity Shift shortfall: when the chosen Methuselah has less than 3 pool,
    the caller re-allocates what there is, each recipient capped at its term
    amount (5.6, LSJ 20010606).
  - Revolutionary Council: X may be 0; the acting baron may be chosen if
    unlocked (via CrimethInc.) (39.1); its points go to Methuselahs (caller
    included) and to any location or equipment in play, any Methuselah's; a
    card may take more than 1 point, the extra wasted (39.2).
  - Archon's "choose a Camarilla vampire": any Methuselah's, in torpor
    included (39.3).
  - Disputed Territory: the location may be any Methuselah's, the chosen
    Methuselah may be the caller; a unique location with another copy in play is
    contested (out of play) and cannot be chosen (39.4).
  - Dmitra Ilyanova: the allocation of the 4 blood is the terms of the
    referendum she calls — fixed when the terms are set (picks of ready
    vampires, any Methuselah's, the same one again as often as wished);
    blood over capacity lost to the bank (58.16).
  - My Kin Against the World: X announced with the card; the terms choose
    exactly X younger vampires of the acting vampire's clan (any
    Methuselah's, in torpor too), fewer if fewer qualify; "only one … each
    turn" counts it played (60.16).
  - Free States Rant: X read as the terms are chosen; "ready vampires"
    anyone's, the caller's included; the blood burned in torpor too as it
    passes (71.15). Cold War's title read as the terms are set (52.5).
  - Rumors of Gehenna's chosen Methuselahs (the caller may choose
    themselves) (70.1).
- Reins of Power, Ancient Influence: the Methuselahs choose their ready
  vampire in turn, clockwise from the acting Methuselah, in the open; the
  choice is optional — with none, Ancient Influence gains 0 (the 5 pool still
  burned); for Reins of Power, a predator choosing none makes their prey burn
  0 (49.1); ready, locked or not; the chosen capacity read as the effect
  applies (50.5). Political Stranglehold counts every vampire of capacity ≥
  8 its Methuselah controls, in torpor too (50.5).
- A granted referendum (Anarch Revolt): 6.2 (§8.6); a passed "burn this
  card" referendum burns the copy whose action was taken (to its owner's ash
  heap); if that copy already left play, nothing (6.3).
- Benjamin Rose's referendum is not conditioned on his title; each passed
  one adds 1 hand size, kept after he leaves play (58.15). Juliet Parr: the
  referendum of her own political action (not a blood hunt); each passed
  adds 2 hand size, until her controller's next unlock phase begins (60.2).
- Cryptic Rider: "only usable on a successful referendum" — after the
  resolution of a political action whose referendum passed (Voter
  Captivation's timing), not after a blood hunt; "the next referendum a
  vampire you control calls this turn" — the next political action of any
  of the player's vampires (69.11). Día de los Muertos: "the first referendum
  a Sabbat vampire you control calls on this turn" is the first called after
  it is played this turn; it passes with no polling step, by a margin of 0
  (LSJ 19980107); "only one … in a game" counts every Methuselah's (52.3).
- Public Vilification: the chosen vampire another Methuselah's, younger than
  the acting vampire, ready; the 3 blood allocated among other vampires (the
  acting one included) as the terms; short of blood as it passes, the caller
  picks which receive (LSJ 20010606) (77.15).

### 13.2 Vote sources

- A source is a vampire or a Methuselah. A vampire source is atomic: all its
  votes cast at once, in one commitment, all the same way; then spent for
  that referendum. Sources are independent commitments: a Methuselah may
  cast one source on one rotation and hold others for later rotations.
  Methuselah-sourced votes (the Edge, each discarded or burned card, each
  card granting Methuselah votes) are each independent and may go either
  way. Effects modifying a vampire's votes fold into its source (1.1).
- Votes gained after casting (Bewitching Oration) join the vampire's cast on
  the same side, no new commitment; before casting they fold into the source
  (5.1). Vote grants from cards in play are read live (19.3: Elysium: The
  Palace of Versailles, Power Structure, Toreador Justicar's "+1 vote in
  this referendum" for every ready Toreador, an Anarch's included).
- The calling political action card gives the acting Methuselah 1 vote,
  either way, as an independent ballot, consuming their one
  political-action-card vote (5.4); so they cannot burn another political
  action card for a vote (46.2). The burn ("burn a political action card for
  1 vote", one per Methuselah per referendum) is the other Methuselahs', and
  anyone's in a referendum with no calling card (a blood hunt, 5.2; a
  referendum a card in play grants, 6.2) (46.2).
- Location votes: "you can lock this card" on a location is its controller's
  to use, in any referendum (Power Structure, Elysium, Ventrue
  Headquarters) (46.3); Ventrue Headquarters' "lock … to get +3 votes" are
  the Methuselah's own votes, all on one side as the card locks, like the
  Edge (19.4). Powerbase: Madrid locked in a polling step gives its votes to
  any titled Sabbat vampire, any Methuselah's — that vampire's controller
  casts them (52.7).
- Oxford University, England: X from 1 up to the player's pool, named as it
  locks; paying the last pool ousts them and they cast nothing (33.2, as
  Ferraille, LSJ 20081105). Business Pressure: as each Methuselah, in
  impulse order from its player, chooses once how much pool to burn — so how
  many votes — and on which side, then passes; once everyone has chosen,
  the votes are counted and the pool burned at once, the last pool ousting;
  not paid vote by vote; [PRE]'s +3 votes a pool are its player's only
  (51.5, owner over RTR 19960530, §23).
- Propaganda of the Deed: the +2 votes are its controller's own, cast for or
  against, in any political action's polling step (71.4). Alamut, Dead
  Pool: a vampire abstaining can still burn counters for votes it cannot
  cast, a legal no-op (73.4).
- The prisci block: each ready priscus casts one ballot, even with no vote
  of its own; the side with more ballots gets the block's 3 votes, a tie
  gives none (52.9). Eat the Rich: "each priscus gets -1 ballot" — its
  ballots in all lessened by 1 (Gratiano, a priscus with "+1 ballot", keeps
  one); "each non-priscus titled vampire gets -1 vote" — off its total
  votes in this referendum, never below 0; the Edge cast is worth 2 votes;
  "a ready titled vampire" read as it passes (70.6). KRCG: vote reductions
  do not touch the block's 3 votes (ANK 20180307-1).
- Title votes: by title, read live; contested, yielded or off-sect titles
  give none (§18.4). Fee Stake: "non-Anarch titles are worth -1 vote" =
  every title but baron, minimum 0; its +1 vote to its Anarch only when it
  calls the referendum (32.9).
- Vote penalties reaching votes already cast: Camarilla Conclave's -1 vote
  to each non-Camarilla vampire, a vampire's votes stopping at 0 (51.2, ANK
  20251125); Absolute Tyranny [POT][PRE]'s -1 vote to each vampire not
  following the Path of Power and the Inner Voice, likewise; played by the
  acting vampire as an action modifier, by another as a reaction (52.2).
  KRCG: a vampire's net votes may go negative and simply subtract nothing
  (ANK 20251125, LSJ 20050221-2) — "stop at 0" differs when votes come
  back (§23).
- Tyler McGill: the +1 vote counts in votes he casts after a Methuselah
  burned the Edge for a vote in that referendum; votes already cast are not
  revised; "Nosferatu" is the clan only (74.8).
- Bruce de Guy's bar, used as he announces a political action, keeps
  Camarilla vampires from casting votes and ballots in its referendum, not
  their Methuselahs' votes; Maldavis gives +1 vote to any Methuselah's
  primogen in a referendum he calls (a granted political action too), read
  live; Tabriz Assembly's burn referendum is any vampire's, directed at its
  controller when another's, its +1 vote its controller's (77.26).
- Fiorenza Savona: +1 on her controller's Edge and on each political card
  burned for a vote, in torpor too (80.10). Karsh (ADV): +3 votes in blood
  hunt referendums, untitled (80.5). Fee Stake: Perth: Baron of Perth, no
  clan's calling vote; any vampire calls its burn referendum, non-Anarch
  titles -1 in it (32.9), the Priscus sub-referendum untouched (80.15).

### 13.3 Polling

- A card played during polling is a commitment of the rotation: it reopens a
  response opportunity for every other Methuselah before the tally (5.3).
- Before votes: as a political referendum's polling step opens, the other
  Methuselahs get a chance to play the cards usable "before any votes and
  ballots are cast" before the caller casts — Confusion of the Eye [OBF],
  Fakir al Sidi, Closed Session — a pre-vote rotation from the caller's
  left, the caller last, those cards (and abilities) alone, no vote; once it
  closes, the polling proper rotates from the caller; not for a blood hunt,
  nor a referendum passing automatically (64.12, owner 2026-10-11).
  Confusion of the Eye [OBF]: in a political action's polling step only,
  before any vote; the acting vampire casts nothing; "if the referendum
  requires a titled vampire" reads any title requirement on the calling card
  or granted action (63.3). Fakir al Sidi: before any vote, locked or in
  torpor too, once per referendum; the referendum fails (not canceled)
  (59.18). [obf] Confusion of the Eye against the bleed of an ally or a
  vampire younger than the reacting one (63.3).
- Referendum abilities (Aisha az-Zahra, Alexa Draper, Yewon Ong, Queen
  Anne): once per referendum, usable while locked (advanced rules §1.2.1,
  ANK 20220705); those with no condition also in torpor, though a torpid
  vampire casts no votes; "any referendum" includes blood hunts; Aisha
  counts every ready Anarch her controller controls (herself, locked ones)
  as it resolves; "a card requiring Dominate" = any library card whose
  disciplines include it (15.2). Queen Anne: "a referendum called by Anne" =
  her political action's referendum, not a blood hunt; target any younger
  Camarilla vampire of any Methuselah, torpor included, not one already
  abstaining (15.2). Danielle Diron's vote ability is offered in torpor too
  (58.8).
- Arishat: "a vampire" any vampire, any Methuselah's, herself included, one
  yet to vote as well (it then casts none) (58.8). "Cancel the votes and
  ballots of a vampire" (Demonstration) is that vampire made to abstain, as
  Quentin's text says it: its votes cast and to come, not its Methuselah's
  (77.14). T.J.: during the polling
  step of a referendum called by a vampire whose title, read then, is a
  Camarilla one (contested or off-sect excluded; a blood hunt is called by
  none); lock and 2 blood are the cost (unlocked, 2 blood, in torpor too);
  the political action card is not returned (59.7). Gerald Windham: the
  ready titled vampires of other Methuselahs, contested or off-sect titles
  excluded, "n votes (titled)" included; read when votes are counted;
  nothing while Gerald has a title (59.19).
- Reactions in polling: Diana Iadanza cancels any reaction card played
  during the referendum of a political action she performs (51.8); Perfect
  Clarity in the polling step (71.25); the same reaction once per action,
  the referendum included (46.1).
- Yoruba Shrine fails the referendum at once (35.4). Delaying Tactics:
  5.5, 6.2, 51.6.
- Mustafa: an older Toreador of another Methuselah, in torpor too (D25),
  once each polling step (D7), moves 1 blood to him and chooses the side as
  it uses the ability; a cast of his already made is cast again on that side
  (its side's printed votes and toll; unpayable, it stands), the votes
  gained since kept; the last use decides (LSJ 20090304-2); an abstaining
  Mustafa is not compelled (advanced rules §3.7.6.3) (76.5).
- Kateline Nadasdy: "non-acting" is not the political action's acting
  vampire; an abstention after the vampire cast cancels its votes and a
  priscus's ballots (LSJ 20100311, LSJ 19971001); usable locked or in
  torpor, once each referendum (76.6).
- Wrong and Crosswise [DEM]: +4 against, the vote forced against, not by a
  vampire voting in favour (LSJ 20031115, LSJ 20031119); [dem] a bleed at
  the reactor -2 (80.22). Malkavian Rider Clause: the next referendum its
  player's vampire calls passes with no polling step, whenever it comes (LSJ
  19980107); nothing if this one fails (80.34).

### 13.4 Tally and costs of voting

- Alexander Silverson: each vampire casting votes against a referendum his
  political action called burns 1 blood (himself included; one with no
  blood cannot vote against); a Methuselah's own votes (the Edge, a
  political action card's vote) cost nothing; a blood hunt called on him is
  not called by him (41.3). Mithras: the same for younger vampires (lower
  capacity, read live — never an ally, never Mithras himself) (42.4).
  Hartmut Stover's burn ("once results are tallied") is not a cost to cast:
  a vampire with no blood may vote against his referendum and burns
  nothing; Hartmut burns if he votes against his own referendum (49.2).
  Marcos Belegrad: each Methuselah casting votes against his referendum
  burns 1 pool once, however many sources, all at once at the tally, before
  the referendum's effects (51.1).
- Jason "Son" Newberry: +2 only on votes against a blood hunt; with no title
  votes he still casts 2 against one. Styles Margs: +2 in favor stand as the
  blood hunt is called on him, a vote no one casts (16.5).
- Orlando Oriundus: while he is ready, the votes his controller's vampires
  cast count double at the tally; the Methuselah's own votes (the Edge, a
  card's vote) do not; "other multipliers may not be played on your
  vampires" is inert, no other multiplier in the pool (77.5).
- De Sade: Alexander Silverson's toll (41.3), on a vampire's first cast
  against only (ANK 20221019); a cast turned by another's effect (Mustafa)
  pays no toll — Silverson's and Mithras's tolls read the same (77.25).

### 13.5 After the referendum

- The after-resolution window opens after the referendum (5.8). "Whenever a
  referendum succeeds" (SchreckNET) is any referendum passing in an action,
  a blood hunt included, any Methuselah's; the hand is open to its user
  until that action ends (58.14). Condemnation: Mute: [chi] locks any younger
  vampire, any Methuselah's; [dai] goes on any ready vampire; [DAI]'s blood
  burns after any referendum passes, a blood hunt included (60.14).
- Lutz von Hohenzollern: any Methuselah's political action whose referendum
  passes, after all its effects; nothing if his controller was ousted (or
  pending an oust) by them; "your prey" read then; his own pool burn ousts
  through the normal oust (62.6).
- Justicar Retribution: "all vampires" are every vampire in play, any
  Methuselah's, the caller's included, those in torpor too; the current
  bleed is the base, with printed text, cards borne and layers, against its
  controller's prey, no action modifier; read as the referendum passes;
  burned (59.9, owner correcting "ready vampires").
- Poison Pill: the pool its player lost to the referendum's effect (a gain
  offsetting none, ANK 20180813) is lost by the acting Methuselah in the
  same batch, ousting at once (69.10).
- Kindred Segregation: each Methuselah in turn, clockwise from the caller,
  keeps allies one at a time until they pass; a keep may cost a last pool
  (it ousts); the kept allies' costs are paid, then every other ally burns;
  an ally with no card (a token) costs 0 (71.2).
- Antonio Delgado: "he can unlock during the discard phase" — that turn's
  discard phase, after his referendum passed (69.9).
- Cedrick Calhoun (16.5) and a fizzled political action (43.1): §10.
- Donald Cargill: 1 blood after any referendum his controller's predator
  called passes, after all its effects, still if it ousted that predator
  (LSJ 20090115-2: the acting seat's place kept once ousted mid-action); in
  torpor too (D25) (78.12).

### Open or contested

- Vote floors: rules-qa's "stop at 0" (51.2, 52.2, 70.6) and ANK 20251125's
  net negative votes (§23).
- The pre-vote rotation (64.12) runs from the caller's left, the caller
  last — an exception to the acting Methuselah's usual first impulse
  (rulebook "Sequencing"), settled by the owner.

## 14. Minions and their states

Advanced rules §5 (capacity and merging, locking, torpor, burning, allies,
retainers, traits, titles, trait changes).

### 14.1 Lock, unlock, stun, wakes

- Locking the locked and unlocking the unlocked are legal no-ops (advanced
  rules §5.2.4; 25.5, 57.10, 62.17): Revelation of the Serpent's unlock needs
  no locked vampire; only "lock to" / "unlock to" costs cannot be paid that
  way; under The Sleeping Mind [DOM] it is usable with no effect (25.5).
  Anja Giovanni may unlock an acting Hecata already unlocked (41.12).
  Nightmares upon Nightmares reaches every minion the Methuselah controls,
  torpid or locked included (a locked one may be "locked", ANK 20210322-2);
  the discard is per minion, one card each (40.3).
- Stun (V5 Sabbat terms, 2025-06-12): lock the minion and put a stun counter
  on it; a minion with stun counters does not unlock as normal at the start
  of its controller's unlock phase, and during that phase burns all stun
  counters it had at the start of the turn (several stuns = one skipped
  unlock); Mind Numb stuns (31.1).
- Infernal: does not unlock as normal; its controller may burn 1 pool to
  unlock it, offered in its controller's unlock phase whenever it is locked
  there (again if something locks it again that phase), ordered among that
  phase's effects; paying the last pool ousts (58.4: Arishat; 62.33: Herald
  of Topheth, an infernal ally).
- Cards entering play locked: Rutor's Hand enters play locked and unlocks in
  its controller's unlock phase (26.5).
- Self-unlocks limit themselves: Sybren van Oosten's and Queen Anne's
  unlocks need the vampire locked (16.1). An ability whose only effect
  unlocks its own unlocked minion is not offered (48.7, an engine choice,
  §22).
- Vampiric Disease: every genuine unlock of a locked vampire burns (the
  unlock phase and any card or effect unlocking it) (59.4).
- Wakes: 2.4 (§5.2). A wake lets a locked minion block and react for the
  action (rulebook "Wake").
- Lord Tremere: once in his minion phase — the lock of an unlocked vampire
  or the unlock of a locked one, in torpor too; any Methuselah's ready
  Tremere or Tremere antitribu (60.5, 60.6). Alexandra: any Methuselah's
  other ready Toreador, locked if unlocked, unlocked if locked; once in each
  of her controller's turns, at any impulse but an as-played window, from
  torpor too (62.3). Victoria Ash: any Methuselah's younger ready vampire, a
  locked one a no-op lock (62.17). Scout Youngwood's lock targets a ready
  ally or younger vampire of her prey's (not in torpor), once each unlock
  phase, usable from torpor (58.9). Mylan Horseed's unlock can target an
  unlocked vampire (57.10). Zizi "can lock to unlock another Blood Brother
  of the same circle": no printed timing — any impulse, Viri ready (68.16).
- Abdelsobek: any vampire his controller controls (or mummy, none in the
  pool), himself and one in torpor included (D28); an unlocked one offered,
  a legal no-op (17.4); once a turn, whatever the target, the same printed
  action (1.4) (78.2).
- Shalmath: unlocks once in each Methuselah's turn (ANK 20180719-4), in
  torpor too (D25); only a card's Celerity mode is barred to him (LSJ
  20050322-2), equipment and abilities not (78.9).
- Tattoo Signal: requires a Seraph; put on the acting Seraph, one a minion;
  locked by its ready Seraph to unlock another Black Hand vampire of any
  Methuselah (D11); its burn option open while its controller has no ready
  Seraph (LSJ 20091203, LSJ 20070410, 33.8) (78.30).

### 14.2 Torpor

- A torpid vampire is controlled, not ready (§3.10): its printed bonuses
  and abilities without "if ready" work (D25, 13.2); it casts no votes
  (15.2); it is a source for "burns 1 blood from a vampire they control"
  (Smiling Jack: never an empty one, LSJ 19971212; one vampire may pay
  several counters, 7.6). Torpid vampires count for "you control" and for
  Recalled to the Founder's 3, and may be the one burned (40.8).
- A contested vampire in torpor stays in torpor through the contest (face
  down, out of play: no rescue or diablerie target); if it wins it returns to
  torpor (3.1).
- Fame ("after this vampire goes to torpor, burn 3 pool") resolves at once
  inside whatever sent the vampire to torpor (6.4).
- Damage on a vampire in torpor: he is wounded (rulebook "Wounded"), so he
  mends normal damage with blood, unmended points doing nothing more, and
  each aggravated point burns 1 blood or him (Daring the Dawn after the
  combat that sent him there). Pulled Fangs' damage lands on a vampire its
  round sent to torpor (74.13).
- Renewed Vigor: [obe] at any Methuselah's torpid vampire, [OBE] at any
  other vampire, torpid included; a target already at full life or capacity
  stays a legal no-op; [for]'s unlock-phase burn only for a bearer in torpor
  (63.15).
- Torpor actions: §11.6, §11.7.
- Decapitate [POT]: before the torpor lands, after the preventions
  (`Window::CombatGoingToTorpor`); not by a vampire itself going down; an
  effect stopping the torpor played first bars it (RTR 20010711) (owner);
  [pot]'s +2 takes the base damage's properties (80.43).

### 14.3 Burning minions

- A burned vampire goes to its owner's ash heap, under someone else's
  control too (31.6); its cards: §16.4, §17.1. Advanced rules §5.4 (who
  burned it, triggers, the "would be burned" window).
- Removed from the game is not burned: Permanent Vacation's ally removed
  from the game leaves its card out of the game (not in the ash heap); the
  cards on it are burned, and nothing keyed to a burn fires (50.6). Golconda:
  Inner Peace: capacity read at resolution (Discipline master cards
  included); the vampire is removed from the game, cards on it removed with
  it (another Methuselah's burned) (33.5).
- "Burns n blood or is burned" (Hermana Hambrienta's 2 blood, Robert
  Carter's employer's 1): the controller's choice, as Society of Leopold's
  wording; the Hermanas only as influenced out, after the excess drained
  (LSJ 20051126) (68.3). Society of Leopold: put on any Methuselah's vampire,
  in torpor too; the choice free while the vampire has the blood, forced
  (burned) without it; its host's controller's unlock phase (59.3). Fear of
  Mekhet: the 5 blood all or nothing — a vampire with fewer than 5 is burned
  without paying any; a vampire in torpor still pays; moving the card does
  not lock its new holder, which may be another justicar or Inner Circle
  member of the same Methuselah (torpid included, ANK 20210109) (49.4).
- Burn triggers: Dead Pool (34.4); Cursed Abattoir's "after a zombie you
  control is burned, you can add 1 counter" whenever one of its
  controller's zombie allies burns (in or out of combat, from damage or a
  card), optional; no zombie retainer in the pool (55.6); The Black Throne
  is offered as the contracted minion is about to leave the ready region —
  including by its controller's oust — after the contract's own "burn this
  card to gain 3 pool" (33.7).
- The Capuchin: burned by any means, he goes to his owner's uncontrolled
  region as a new card with no counter (60.8).
- Antediluvian Awakening: the vampire burned may be in torpor; burning it
  burns the card; chosen as the unlock rider resolves; the card gone by
  then, nothing is due (70.9).
- An ally at 0 life is always burned (73.5, 71.10).
- Taking the Skin: Minion: the reflex cancels a frenzy card played on the
  vampire (still played, its cost paid: ANK 20190104, ANK 20260216); the
  skin when the vampire burns a minion by its action or in combat (LSJ
  20090922), not a referendum's or a side effect's burn (RTR 19960124, ANK
  20220916); one skin a minion; its extra bleed past the once-a-turn cap
  (1.4), any bleed; burned in its controller's discard phase; its play
  before a diablerie's blood hunt (RTR 19991206) waits on K15 (78.39).

### 14.4 Blood, capacity

- Blood above capacity drains to the bank at once: when a Discipline card
  leaves a vampire (burned, player ousted) (28.4); as Wormwood enters play
  and as its counters go down (40.7, LSJ 20100902); after any move over
  capacity (17.4, 57.7, 58.16, 62.4, 70.18). A vampire never holds more than
  its capacity in play; Seraphina's blood stops at capacity (17.1); moves
  of pool to vampires go up to capacity (Gird Minions: a vampire, then an
  amount up to its capacity, again until Pass; torpid ones too; moving the
  last pool counter ousts, 69.4).
- An uncontrolled vampire may hold any amount of blood, over capacity
  (transfers and effects: Wasserschloss Anif, Powerbase: Montreal, Family
  Gathering); the excess drains when it is influenced out (28.3).
  Wasserschloss Anif's "a Tremere you control" is not limited to ready: a
  Tremere in torpor can move its blood (28.6).
- Heidelberg Castle: each use moves from one vampire to the other (blood,
  then any of the equipment and retainers it could have), not both ways; any
  amount, the excess to the bank (57.7, ANK 20170804).
- Capacity: Discipline cards and other increases count (60.18, 33.5,
  59.15); temporary capacity lasts its effect (Coagulated Entity, 68.6);
  "cost" of a vampire is its capacity (71.9, 35.2).
- "Gains blood" (Slake the Thirst): each effect's gain is a gain; a gain
  stays answerable in the window of the first real decision after it; a
  strike's gain as the strikes end; several gains: the one answered chosen
  by its amount (LSJ 20100203); also between a diablerie and its blood hunt
  (68.2).

### 14.5 Allies

- Life, not blood: "burn n blood or life" on a host and a vampire's "burns 1
  blood" are one effect — blood for a vampire, life for an ally; an ally at
  zero life is always burned; as an in-play ability's cost, "burn 1 blood"
  stays a blood cost an ally never pays (73.5, advanced rules §5.5.1). An
  ally pays tolls in life where the toll allows (41.4, 42.2, 60.13); Seeds of
  Terror's blood-only toll bars it (53.2).
- Playing as a vampire: D16 (73.6). Herald of Topheth plays [dai], [pot]
  and [pre] cards as a capacity-5 Independent vampire; infernal (it does not
  unlock as normal, 1 pool to unlock); its rush at another Methuselah's
  minion; its life back to 5 at most, 1 each of its controller's unlock
  phases (62.33).
- Life moves: Sri Sansa, one move each unlock phase, either way, with a mage
  ally (Nephandus, Veneficti); "not to exceed their starting life" caps the
  ally only — blood moved to Sri above her capacity drains off; no move to
  an ally at its starting life; an ally giving its last life burns (41.8,
  62.31). Over starting life where the card says: Nephandus's torpor burn
  +1 life (62.24), Gregory Winter's torpor burn raises his life over his
  starting life and his unlock-phase life burn can burn him (63.16),
  Ossian's (64.6). Renegade Garou's unlock-phase life gain from the bank at
  2 or fewer life (63.8).
- Allies are not vampires, anarchs or Camarilla for "vampire" texts (57.1,
  39.5, 32.8, 63.4, 71.14); "werewolves you control" are allies of the
  Werewolf subtype — Jacob Fermor's +1 bleed while Jacob is ready (in torpor:
  none), read live (64.1).
- Allies entering play: §11.4. Infernal Servitor: "if you control no
  infernal vampires" — one in torpor counts; read as each step settles;
  "after it performs a successful action" — a fizzled action counts;
  removed as the action ends (70.19). Emerald Legionnaire: "if you control
  fewer copies of Emerald Legionnaire than ready unique Harbinger of Skulls"
  — the ready Harbingers of Skulls you control (crypt vampires), the ally
  ready itself (ANK 20180511-1) (67.2). Jake Washington: "a ready vampire
  with no blood" any Methuselah's; the amount (1–4) chosen as he is burned
  (44.4).
- Unleash Hell's Fury as a minion: 60.21.
- Dr. Morrow: any ally, his controller's too; more counters add nothing (LSJ
  20070306); the -1 stealth is his text's, standing: it applies while he is
  in play (torpor included), gone with him; the counters stay; Black Hand a
  trait (LSJ 20070322) (76.7).
- Ponticulus: +1 intercept on actions at his controller; locks to give any
  Methuselah's Tremere attempting to block the intercept it needs; no
  action, no equipment, still blocks (80.25). Vivienne Géroux: an Anarch
  vampire of capacity 1 for her basic Celerity/Fortitude cards only (ANK
  20230417); her controller's vampires -1 stealth on political actions;
  given away by a Ⓓ action (80.30).

### 14.6 The uncontrolled region, banishment

- A card in the uncontrolled region is out of play and no vampire (no
  influence, no effect on cards in play reaches it) (69.6); blood on it is
  counters (28.3, 34.7).
- Banishment: the vampire goes to its controller's uncontrolled region with
  its blood as counters; cards on it leave play but stay face down on it and
  come back (face up, unlocked) when it is re-influenced (5.7); a banished
  vampire keeps everything, counters of any kind included (25.1). Pit of
  Contemplation's "owner's uncontrolled region": a temporarily controlled
  vampire goes to the region of the Methuselah it would return to; the move
  is Banishment's (27.7).
- Entering from the uncontrolled region: "comes into play" / "put in play
  from your uncontrolled region" = influenced out (58.12, 34.7). Anarch
  Convert: his ability applies as he is moved out (the pool's only way in);
  not offered with no other non-titled vampire to make Anarch; gaining 1
  pool or drawing from the crypt chosen after the vampire is chosen; moved
  out beside a copy in play, he must use it (57.9). Flames of Insurrection:
  the offer goes to the Anarch's controller whoever controls the card (ANK
  20180627-2), optional, the vampire in their uncontrolled region their pick
  (69.15). Mary Anne Blaire's action adds the blood to any vampire in her
  controller's uncontrolled region (59.11). Leandro: the other blood stays
  for the uncontrolled region (71.19).
- Crypt draws to the region: Mesu Bedshet's "if it is younger" — younger
  than the acting vampire; "draw it" to the uncontrolled region, with or
  without the 2 blood; another clan's card shuffled back with the whole
  crypt (71.1). Tupdog's burn moves one crypt card there (62.14). New Blood
  draws a crypt card as he enters (68.1). Petra Resonance: a crypt emptied
  since it was played reveals nothing (71.9).
- Kaymakli Nightmares: a created vampire in an uncontrolled region
  (banished) is burned, its counters still to the pool and not counted for
  the draw; a Web of Knives Recruit in training is no crypt card and stays;
  "only one … per game": anyone's, a canceled play counting (70.10).
- Web of Knives Recruit: the counter burns as a mandatory unlock-phase
  effect ordered among the others; with three or more in training, the two
  are the player's picks; the new vampire enters ready, unlocked, its card
  the vampire (as Childe of the Revolution's) (69.6).
- Paulo de Castille: "after Paulo enters play during your influence phase" —
  as he is moved out of the uncontrolled region in that phase, before any
  contest (as Erlik's distribution) (74.5).
- Kindred Intelligence: the crypt's top card to the uncontrolled region, not
  with an empty crypt, read at resolution too (80.18). Dr. Solomon Grey: 1
  pool as he enters play, before any contest (LSJ 20080526) (80.3).

### 14.7 Created and new vampires

- Childe of the Revolution's new vampire enters ready, unlocked, with no
  blood, capacity 1 plus any Discipline card on it; it may act that turn and
  must hunt (even with blood; at once when that is its only option); the
  library is shuffled after the search regardless (28.1). Abombwe put on by
  Childe of the Revolution's search, its requirement ignored (LSJ
  20051116-2) (63.16).
- Embrace-like cards with no sect printed (Waters of Duat: "becomes a …
  Follower of Set"): the new vampire takes the acting vampire's (sire's)
  sect (32.6, rulebook "Sect").
- Created vampires are not unique (41.5). Soul Gem of Etrius: "younger"
  compares the burned vampire's capacity in game to the drawn crypt card's
  printed capacity; the bearer's controller draws; the new vampire enters
  unlocked (it may act this turn); a drawn copy of a vampire its Methuselah
  controls burns and the Gem draws again; an empty crypt: the Gem burns
  (60.18). Call the Great Beast: the 11 counters checked as it is played and
  after each counter action; no play or counter action while its Methuselah
  controls a Great Beast (LSJ 20101130, read for every capacity); the Great
  Beast enters unlocked, three distinct Disciplines chosen from the
  rulebook's list; its prevention once each combat (60.20).
- A master card becoming an ally (Jake Washington) is not recruited: it may
  act the turn it enters (44.4).
- João Bilé: "actions to put vampires in play" are those the Sterile trait
  bars (advanced rules §5.7.6): the actions whose card puts a vampire in
  play (The Embrace, Childe of the Revolution and their kin) (76.18).
- Nosferatu Bestial: a 2-capacity unique Nosferatu of the acting Nosferatu's
  sect, basic Animalism, no blood; hunts only, no votes or ballots, no
  equipment (80.39); it may act the turn it enters (owner) (80.38). Dual
  Form's copy of a clanless vampire (the Great Beast) is clanless (80.45).

### 14.8 Retainers

- Strike damage and stolen life (29.2); Stanislava's text-loss (49.3);
  Luciano Carvalho's move: the animal retainer and the vampire it moves to
  both chosen as the ability is used, any other vampire his Methuselah
  controls able to hold it, himself and torpid ones included; once each
  discard phase (41.7). Pack Alpha's retainer (22.2).
- Advanced rules §5.6 (the retainer is not the bearer; using a weapon is not
  a strike; retainer damage is environmental).

### 14.9 Merging and versions

- Merging (rulebook "Advanced"): in the influence phase for 4 transfers and
  1 pool, the vampire in play ready or in torpor, not already merged.
  Merged: the advanced card on top — its capacity, Disciplines and sect (an
  anarch token gone with another sect, advanced rules §5.1.2) —, its
  [MERGED] title first, else its printed title, else the base card's; the
  base card's text whole (Appolonius's optional press and +1 bleed beside
  the advanced card's). Burned, both cards go to the ash heap. Legacy
  finding no base version is still successful (69.5).
- Group versions are one vampire for contests, each with its own text:
  Theo Bell G2 and G6 (no merge, the G2 advanced card not in the pool; his
  ready prince or justicar is another vampire of his controller's, read
  live) (60.1); François Villon G2 and G6 (62.4).
- A merged vampire keeps its entry into play (71.3, LSJ 20030519-2; 70.3).

### 14.10 Traits, sects, clans

- Sect changes: Gilbert Duane's vampire made Anarch is chosen as the ability
  resolves, among the non-titled vampires his Methuselah controls, in
  torpor included; once per influence phase (44.5); Waiting Game's anarch
  with no blood becomes Camarilla, its title kept (71.10); Mozambique Allure
  (69.14); Twilight Camp's sect lasts (34.7); merged sect (69.5); a created
  vampire's sect (32.6). Advanced rules §5.9 (actual change, temporary
  override).
- Paths: 49.7 (§4.4). Circles (Blood Brothers): a vampire with no circle is
  in a circle of one (LSJ 20020609); New Blood: "choose any circle" — a
  circle printed in the crypt (Mexico City, Torrance, Kiev, Chicago,
  Budapest) or his own circle of one, as he enters play; "the unique circle
  of one of any vampire" (ANK 20211113) read where a card reads it (68.1,
  superseding 67.6).
- Infernal: §14.1. Blood cursed: 63.1. Slave: 62.14. Black Hand read on the
  striker (62.26). Red List: 67.5, 71.22; "Red List" on Ossian a bare trait
  (64.6). Flight: generated crypt data and a Discipline level, read
  wherever gained — As the Crow, Dive Bomb, Earthshock (84.37, superseding
  16.6, 62.14). A slave a card creates carries the slave's text (84.9).
- Clans with their V5 names (§1).
- Text naming a trait of the current bearer: D31.
- Derange: either clan plays it (ANK 20190203); on a younger vampire, torpid
  too; its clan and sect set as played, kept as it moves (LSJ 20070707),
  over the underlying ones and a sect change (LSJ 20100811, ANK 20190619),
  gone with the card (RTR 20201130); a title off its new sect suspended (RTR
  20201130, LSJ 20060904); its move to another vampire of any age, never a
  Malkavian or Malkavian antitribu; one blood unlocks in its unlock phase, a
  disease's or an infernal's unlock still one (ANK 20221210-2) (78.44).

### 14.11 Counters on minions

- Corruption counters belong to the Methuselah who put them; they stay
  through torpor and control changes and survive banishment; lost when the
  minion is burned or removed with its ousted controller; an ousted
  Methuselah's counters on others' minions stay (no longer "yours", but
  Aniel can burn them) (25.1). Aniel's counter burn: on a minion with
  several Methuselahs' counters, the controller chooses whose, at
  resolution; no counter left then: fizzles; the counter and card halves
  are one printed action for NRA (25.4). Enchanting Gaze (25.5).
- Cave of Apples "their capacity or cost": a vampire's capacity, an ally's
  printed pool or blood cost; only the player's own counters count, burned
  all at once (35.2); Contagion's counter (62.19).
- Vampiric Disease's counter is the vampire's state, kept through torpor, a
  change of control and banishment; played on any Methuselah's vampire, in
  torpor too (on a diseased one, a legal no-op) (59.4). Torture counters
  (64.7).
- Diabolic Lure [dai]: after its infernal vampire's block, before block
  resolution; 4 pact counters on the acting minion, kept below the threshold
  and added across Lures; at its capacity or an ally's printed cost, pool or
  blood (a token's 0), burned to make it infernal; [DAI] the pool burned
  whether or not (LSJ 20100205-2, ANK 20221007) (corrected: a blood-cost
  ally's cost counts) (78.45).

## 15. Vampires' printed text; other card readings

The principles of printed vampire text; most vampires' readings sit with
the mechanism they use (search the vampire's name). §15.2 holds the card
readings (vampires, allies, library cards) no mechanism section states.

### 15.1 Principles

- Every vampire ends with all its printed text; a vampire may land with part
  of its text, the rest quoted as incomplete (13.1).
- Plain printed bonuses are read live while the vampire is in play, in
  torpor too (LSJ 20011022), not while contested (13.2); "while X is ready"
  (André the Manipulator, Carmelita Neillson, Weirich Waldburg) is lost in
  torpor; an over-size hand is then discarded down at once (13.2, 8.2).
  Hesha Ruhadze counts uncontested unique equipment on him; Neserian counts
  uncontested locations, hunting grounds included (13.2).
- Printed abilities follow the periodicity templates (§4.2), the
  only-when-needed rule (§9.2) and D25 (torpor). A printed passive reaching
  other vampires (Brunhilde) is read live, in torpor too, not while
  contested (58.1).
- A vampire outside a combat is not offered its abilities there (15.2).
- Vampires meeting requirements they lack: §4.4.
- Printed actions: §8, §11 (Sakhar, Dominica, Seraphina, Lenelle, Elen
  Kamjian: 17.1; rushes: 17.3; steals: 17.4).
- A printed title, "n votes (titled)": §18.4.

### 15.2 Card readings not stated elsewhere

- Alabástrom: once per turn, any other ready Gargoyle (16.6); Flight read
  since 84.37.
- Valkyrie: "unless /Brunhilde/ is ready" — any ready Brunhilde in play, any
  Methuselah's; "Non-unique." — copies neither contest nor self-contest
  (several may be influenced) (58.5).
- The Medic: "during your turn" — once each of his Methuselah's turns; any
  Anarch they control, in torpor too, its blood capped at capacity (58.7,
  owner correcting "as often as he has the Edge").
- Mistress Fanchon: "a minion card" is any library card but a master or an
  event (60.3). The Beast: "action cards" are all the action card types —
  action, political action, ally, equipment and retainer cards (60.4).
- Veneficti: +1 stealth; not the target of a directed action of an ally or
  of a vampire of capacity 6 or less (read as offered); he steals a demon
  ally (Herald of Topheth, added for it); his burn option per his printed
  clan, Baali — any of the pool's Baali is infernal (62.30).
- Marciana Giovanni: "during an action, you can lock her" — any action, her
  Methuselah's or another's; the non-Sabbat vampire chosen any
  Methuselah's, herself included (LSJ 20100527); locking her during her own
  attempt fails it (69.7); the chosen vampire burns once (79.2).
- Maris Streck: no "during": her +1 intercept to any blocking minion as
  often as she pays, locked too (79.3).
- Ambrosio Luis Monçada's tax on directed actions: 69.8 (§8.3).
- Zizi: "if /Viri/ is in your ready region"; Viri: "while /Zizi/ is in your
  ready region" — the controller's ready region; Rurik Rakoczy's "while
  /Ludmijla Rakoczy/ is ready" and Valkyrie's name no region: any
  Methuselah's (73.7).
- Petra Resonance: a vampire's cost is its capacity; the steal takes what
  the lowest has, up to 3 (an oust at 0) (71.9).
- Blood Brothers: Unwholesome Bond [SAN] for New Blood — one chosen
  uncontrolled Blood Brother with no printed circle (LSJ 20050927, ANK
  20211113), a mode of its own; [san]'s X counts any Methuselah's ready
  unlocked Blood Brothers of the circle at resolution (68.7). Sanguine
  Instruction: the target chosen first, then the Discipline — one the actor
  has at superior and the target not; a target with none of them is not
  offered; any Methuselah's vampire of the acting vampire's clan (68.8).
  Gestalt: all or nothing at resolution (a chosen Blood
  Brother unable to burn: no effect); [SAN]: the reacting vampire plays cards
  with the chosen ones' Discipline levels until the action ends (68.15).
  Shell Game: X = 0 a legal no-op; "you may unlock one of those" — offered
  after a successful action, one (two at [SAN]) of the locked ones, its
  player's pick (68.5). Hive Mind [san]'s "another ready Blood Brother of
  the same circle": any Methuselah's (68.14). Jerry and Redistribution:
  68.10 (§16.7).
- Vampires that print no text: Cameron, Hester Reed (63.16); Gunnar,
  Lillian, Robert Price (64.1); Delilah Easton, Isabel de Leon (65.3).
  Tatiana Romanov: +1 bleed (65.3).
- Adelino Da Silva: "an older vampire" is one of higher capacity than his
  (41.14).
- Repulsion: while a vampire bears one, none of its levels is playable by
  him; [pre]'s maneuver only to long range, once in the block's combat, any
  round. Palatial Estate: nothing on an ally host; in torpor too. Gear Up
  [nec]: not offered with an empty ash heap (D14). Divine Sign: the chosen
  minion is the action's target (D1), any minion. Dive into Madness: a third
  [DEM] not offered (74.11).
- Game of Malkav: each pick secret from every other seat until the last
  (kept outside the state; a viewer is told a number was chosen, not which);
  revealed, the gains and burns at once (76.14).
- Thadius Zho: any vampire, a torpid one too (Apolonia's precedent); a
  0-blood target a legal no-op (76.20).
- Winterlich: no changeling ally in the pool, his text inert (DESIGN §11.3)
  (77.32).
- Read as printed, by the default readings and their KRCG rulings: Aimery,
  Alex Camille (threshold, open K23), Bulscu (an action at another's
  equipment or location is directed, D12), Carna (torpor too), Claudio
  Severino, Dan Murdock, Dr. Julius Sutphen (Mithras G6's toll), Gold Pan
  Dan (once each turn after combat), Karif al Numair (any Methuselah's,
  toggled), Lazverinus, Martin Franckel (the bled Methuselah, read live),
  Mélusine d'Anjou (a younger vampire's pending attempt), Nehemiah (his
  [cel] modes only barred, LSJ 20050322-2), Paul Cordwood (stealth or
  intercept when needed), Recruitment Exercise, Spoils of War, Suhailah
  (read live), Taste of Death; twelve more whose text is their sect and
  title (77.33).
- Onaedo: +1 stealth during her diablerie actions while any Methuselah's
  Otieno is ready (D11, 73.7); no Otieno in the pool, so inert (DESIGN
  §11.3) (78.5).
- Earl, Old Neddacka, Bloodfeud, Denette Stensen, Frank Litzpar, Jephta
  Hester, Lana Butcher, Jackson Asher, Bituin: their sect, title (Bituin's
  Baron of Manila) and traits generated, nothing else to define (78.22).
- Printed stats of Blythe Candeleria, Kite, Rodolfo, Richard Tauber, Yazid
  Tamari (ADV: a standing bonus, not 79.1's once); Round 16's text-only
  vampires (80.2). Okulos: once in each Methuselah's turn, on any impulse,
  in torpor too (80.11).
- Round 17's cards, each read in its rules-qa item: Cold Aura not offered
  once the range is set (84.1); Waxen Poetica burned when dodged, kept when
  combat ends first (84.2); Botched Move against any played card (84.3);
  Alvusia's strength to any slave Gargoyle in any action, its bleed to one
  bleeding (84.4); Angelo's circle for the
  Blood Brother playing (84.5); Gianna (84.7); Raw Recruit, its Gargoyle a
  slave, its second action read live (84.9); Victor Pelletier (84.10);
  Filchware's Pawn Shop (84.11); Baldesar (84.12); Public Enemy and
  Retribution, "since your last turn" (84.13); Kestrelle Hayes, any target
  (84.14); Leadership Vacuum (84.15); Legion's fledgling (84.16);
  Spontaneous Power (84.17); Call the Wild Hunt (84.18); Increased Strength
  (84.19); Promise of 1528 (84.20); Provision of the Silsila (84.21); Hand
  Contract and Shakar (84.22); Guide and Mentor (84.23); Port Authority and
  Prophecies of Gehenna (84.24); the two Paths (84.25); Ruins of Villers
  Abbey (84.26); the combat cards (84.27); locations and costs (84.28);
  bleed modifiers (84.29); Black Sunrise, As the Crow, Dive Bomb (84.30);
  Crematorium, Morgue Hunting Ground, Giulia, Infamous Insurgent (84.31);
  printed vampire stats (84.32), unlocks (84.33), combat and actions
  (84.34); Tremere slaves (84.35); inert and text-only (84.36); Hesha's
  groups (84.38); Coordinate Attacks' filter (84.39); found while testing
  (84.40).
- Round 18's cards, each read in its rules-qa item: Helicopter locked
  however equipped, taken from another minion too (85.1); Julius burned once
  in torpor, its triggers fired (85.2); Antoinette DuChamp (85.3); Chandler,
  Christine, Navar (85.4); Eurayle (85.5); Fergus (85.6); Greensleeves inert
  (85.7); Smudge (85.8); text-only (85.9); The Admonitions' control passing
  (85.10); Communal Haven: Cathedral (85.11); Conflagration (85.12);
  Creation Rites (85.13); the Crusades (85.14); Dodge (85.15); Eternal
  Vigilance (85.16); Hatch the Viper (85.17); Helicopter's unlock (85.18);
  Hidden Lurker (85.19); Improvised Flamethrower's backfire (85.20);
  Leverage (85.21); Nosferatu Justicar (85.22); NSA Trio (85.23);
  Sacrificial Lamb (85.24); The Secret Library of Alexandria (85.25); Shanty
  Town Hunting Ground (85.26); Storage Annex (85.27); directed at another's
  minion (85.28); "your next discard phase" (85.29); Hatch put in play
  (85.30); found while testing (85.31, 85.33, 85.34).

## 16. Cards in play

Advanced rules §1.3.1 (locquipments), §1.12 (cards and counters on other
cards), §1.10 (burn and removal), §6.3 (control of a card in play).

### 16.1 Locations

- A location's text not saying "you control" lets any Methuselah's minion
  qualify (28.2, D11). "You can lock this card" is its controller's to use
  (46.3); an unaddressed bonus reaches anyone's minions (49.5).
- Locquipment (Living Manse) is a location in play: not stealable as
  equipment nor taken by the equip action, but stealable by cards that steal
  locations (17.4); moved in an ambiguous fashion, it goes on any ready
  minion of the new controller able to hold it; none: burned (39.4, LSJ
  19990928; 31.4).
- Carfax Abbey: 7.4. Gurchon Hall: "the two oldest ready vampires you
  control" — oldest by capacity; a tie for either place its controller's
  pick; one already fed by a hunting ground this turn gains nothing and
  still counts as one of the two (69.1).
- Maabara: locked at any impulse, several cards may be on it; the burn is an
  action directed at its controller; cards on it go to the ash heap with it
  (71.12). The Erciyes Fragments: 60.19 (§7.1).
- Powerbase: Barranquilla: the ready Sabbat vampire chosen as it is played
  (none: not playable), its capacity read as it resolves; the unlock move
  once each unlock phase; the burn: any Methuselah's vampire, +1 stealth
  while the acting vampire is titled, read live; burned once no counter is
  left (62.25). Powerbase: Savannah (44.7). Powerbase: Madrid (52.7, 73.3).
  Powerbase: Munich (73.6). Powerbase: Los Angeles (28.12).
- Zaire River Ferry: "this Lasombra" — the bearer must be Lasombra for
  either ability (9.1); "a vampire can have only one haven": the only haven
  in the pool, and unique (70.20).
- Monastery of Shadows (49.5, 50.4), Yawp Court (54.1), Heidelberg Castle
  (57.7), Wasserschloss Anif (28.6), Twilight Camp (34.7), Cappadocian Crypt
  (34.6), Alamut (34.5), Secure Haven (35.1), Cave of Apples (35.2),
  Fortschritt Library (71.16).
- Arcanum Chapterhouse, Alexandria: its controller's predator and prey each
  burn 1 pool per Hunting Ground they control in their own unlock phase,
  counted then; with two Methuselahs the one rival once (6.8); any minion
  may burn it, its controller's undirected (78.41).
- Gran Madre di Dio, Italy: equipment off play and as played, a location in
  play; a minion its controller's predator or prey puts in play out of the
  unlock phase locked, a vampire younger than its bearer 1 blood (owner)
  (80.41).

### 16.2 Hunting grounds

- "Gain blood from only one hunting ground each turn" is per vampire: two
  grounds may feed two vampires; the vampire is chosen as the ability
  resolves (6.5). A pick on a vampire at capacity is legal (no effect) and
  counts as its one hunting ground this turn (7.2). Corporate Hunting Ground
  as the others (62.20). Once per phase per copy (6.6).
- Lady Scarlett does not take a hunting ground's blood for a Toreador
  (41.2). Neserian counts hunting grounds as locations (13.2).
- Institution Hunting Ground: the hunting-ground family (6.5) (80.17).

### 16.3 Equipment

- Bearer's traits read on the current bearer (D31; 9.1, 12.5).
- "When a minion equips with it, lock it" (Helicopter): the card comes
  locked however it is equipped, a search or a take from another minion
  included; put on rather than equipped (Alastor), not (85.1).
- Shilmulo Tarot: on equip it takes the top 2 cards (fewer if fewer there);
  its unlock-phase move is once per unlock phase per Tarot, whoever bears
  it; unique (11.2); stores its 2 cards when equipped from hand or by a
  search (LSJ 20090506); taken from another minion (moved, not played) it
  stores none (12.2); a non-Ravnos bearer: its Methuselah draws nothing
  from it, the unlock-phase move still applies (12.5).
- Black Market Cache: 0 to 3 non-unique equipment chosen one at a time as it
  resolves; "no cards on it, burn it" checked whenever empty — at once after
  a search finding none, and as its last card is drawn (11.3); replaced
  before its search, so its replacement is never one of the cards put on it
  (12.4).
- Learjet: "a card this minion played as an acting minion" is any card the
  bearer plays while acting in the action, its combats' cards and a
  canceled card included; the extra draw offered after the normal
  replacement is drawn (LSJ 20050224-4), and with a delayed replacement when
  that one comes due (PIB 20110718); a vehicle (63.10); 73.1, 73.2 (§19.2).
- The Signet of King Saul (59.15); Anachronism (64.2); Enhanced Coagulant
  (38.4); weapons (§12.13); Kaymakli Fragment (71.23); Monocle of Clarity:
  its question changes no game state, the equip action alone (68.11).
- Unique equipment: §18.1. Equipping: §11.3.
- Eye of Hazimel: the rush fizzles once the Eye left the actor (RTR
  19960221); superior Potence and Chimerstry and the combat-card discount
  are a vampire's only (D16), Ghede's added; not moved or stolen while its
  bearer is ready (Heidelberg Castle, Redistribution, Confiscation, an equip
  from it); burned, 5 unpreventable environmental damage on its bearer; in
  its controller's unlock phase, 1 pool or the Eye (78.42).
- Seal of Veddartha: equipped by any vampire, its bleed only above capacity
  5 (LSJ 20030607); a counter each unlock phase up to 4, whatever the
  Disciplines (LSJ 20070320-2), each a level (dom, dom, for, for); all
  cleared as another minion takes it (80.32).

### 16.4 Cards on minions

- A card on a minion leaves play with it: burned with it, removed from the
  game with it (owner's cards; other Methuselahs' cards burned to their
  ash heaps) (27.4, 33.5) — an ally Permanent Vacation removes has all its
  cards burned, nothing keyed to a burn firing (50.6); face down with it in banishment (5.7);
  moved with a stolen minion (minion cards), staying their player's (master
  cards) (31.5). Advanced rules §1.12.1.
- Who controls it: §3.9 (7.5, 38.4).
- Readings of put-on cards: Charisma "put this card on a ready vampire":
  any Methuselah's (33.3); Phantasmagoria: any minion (32.5); Terrifying
  Visage: any ready Cathari vampire, its path icon needing a ready Cathari
  vampire of its player's (53.7); Ephor: any Methuselah's Ventrue of
  capacity ≥ 8 (read at play; it stays if the capacity drops, but the action
  then needs it); its +1 vote is no title (59.13); Society of Leopold (59.3);
  Vampiric Disease (59.4); Contagion: the card itself goes on the vampire,
  remembering its level; each other Methuselah burns 1 pool as a mandatory
  effect of their unlock phase while the vampire is ready (read as it
  resolves) (62.19); Legendary Vampire: "entered your ready region during
  your last influence phase" — entered play between the end of the player's
  last minion phase and the end of their last influence phase (moved out of
  the uncontrolled region, or put in play then) (71.3); Under My Skin
  (60.12); Condemnation: Mute [dai] (60.14).
- Moving a card between minions: Fear of Mekhet (49.4); Society of Leopold's
  move action (59.3); Luciano Carvalho (41.7).
- Corporal Reservoir: on any Methuselah's Black Hand vampire (D11), the card
  its player's (7.5), used by the vampire's controller (ANK 20210309-3):
  locked to prevent 1 damage in combat or to gain a blood, in torpor too
  (D25); it unlocks only for a blood in its vampire's unlock phase (78.36).

### 16.5 "Put this card on …"

- The card itself enters play, out of the ash heap it went to as it
  resolved — never an effect-only stand-in; the general rule for every card,
  present and future, whose text puts the card itself on a minion or in
  play (47.1, generalizing 46.4, revising 21.1). Touch of Oblivion [obl] and
  Sculpt the Flesh (on the opposing minion, if the strike lands), Disarm (on
  the opposing vampire), Pass Through Shadow [OBL] (after combat), Dogged
  Pursuit [ANI] (if the vampire does not block), Melange [AUS] (on the
  acting minion, if the vampire blocks), Fever Pitch (53.5), Pillars Fall
  (54.3), Focus the Blood (63.7), Contagion (62.19), Inner Essence (68.12).
  Burned — counters run out, a cost paid with it — it goes back to the ash
  heap; its effects unchanged (47.1). Pass Through Shadow's burn for +1
  stealth sends it back there (46.4).
- A card with an effect lingering after it resolved, not put in play, goes
  to the ash heap as it resolves; the lingering effect applies regardless
  (21.1, as revised: only action cards wait in limbo until successful or
  blocked).
- Patagia: on the acting Nosferatu, one each; a maneuver each combat; flight
  once each combat, its own vampire's (ANK 20221021), read since 84.37
  (80.24). Praxis Seizure: York: on the calling vampire if still in play
  (80.26).

### 16.6 Master cards on minions

- A master card on a minion stays controlled by the player who played it:
  it contests that player's copies; it is removed when that player is
  ousted (7.5); on a stolen vampire it stays its player's (31.5). A card on a
  removed vampire is burned to its owner's ash heap (7.5). Anathema's ruling
  (PIB 20121028: the card stays when its player is ousted) disagrees (§23).
- A master card's offer to the vampire it is on goes to that vampire's
  controller (ANK 20210309-3; 69.15's Flames of Insurrection, the offer to
  the Anarch's controller).

### 16.7 Counters on cards; moving them

- Counters spent as a cost are spent as the cost is paid (36.1); several
  burned at once (35.2). Dreams of the Sphinx's third lock: its effect
  lands, then the card burns at 3 counters (7.7).
- Split the Veil's ally is chosen when the last counter burns, among wraith
  or zombie allies in the player's ash heap; none there: the card burns
  anyway (32.11).
- Camarilla Threat: each copy in play adds its pool; paid as the card is
  discarded, a last pool ousting its Methuselah (70.5).
- Jerry ("move any amount of blood from the Blood Brothers in the Chicago
  Circle to any other Blood Brothers in this circle") and Redistribution
  ("among the Blood Brothers in this circle"): its controller's Blood
  Brothers only; one move at a time — x blood between two of them (the
  source, the destination, then the amount), once for a given source and
  destination, or (Redistribution) any number of equipment, one at a time —
  each move a separate part of the effect: Slake the Thirst answers one move;
  Jerry once each master phase (68.10, owner 2026-10-12).
- Trap and Conquer the Beast (59.17), Waiting Game (71.10), Augury of Doom
  (54.2), Powerbase: Barranquilla (62.25), Illusory Resources (63.14),
  Wormwood (40.7), Twilight Camp (34.7).

### 16.8 The burn option

- The burn option (rulebook, advanced): usable in any Methuselah's unlock
  phase; "a minion who meets the requirements" = a ready controlled minion
  (not in torpor) of the card's clan if named, with one of its Disciplines if
  any (pool: Sight Beyond Sight) (33.8). Veneficti's per his printed clan
  (62.30).
- Not in rules-qa: the rulebook's one such discard per unlock phase and its
  "is not a legal target" clause; LSJ 20100203-2 (not usable while one meets
  the requirement) (§24).
- The burn option reads a card's printed trait requirement (a Seraph, an
  infernal vampire, an unmerged advanced vampire, 33.8): Legacy, Unleash
  Hell's Fury, Veneficti and Tattoo Signal are burnable only with a minion
  meeting it (78.31).

## 17. Control and ownership

Advanced rules §6.1 (owner and controller), §6.2 (taking control of
minions), §6.3 (of a card in play), §6.4 (leaving and re-entering play).

### 17.1 Owner, controller, where cards go

- §3.9. Burned cards to their owner's ash heap (7.5, 31.5, 31.6); a
  vampire burned under someone else's control (temporary included, Puppet
  Master [DOM]) goes to its owner's ash heap (31.6).
- Cards stored on another Methuselah's card stay their owner's (60.19); the
  holder's controller ousted, another Methuselah's card stored on it goes to
  its owner's ash heap — as another's card on a removed minion does — not
  out of every zone (67.1).
- Advanced rules §6.1.2: control decides what an oust removes.

### 17.2 Taking control of minions

- Puppet Master [DOM]: its "during your next minion phase, burn this card
  to … take control" is mandatory, any point of that phase, which cannot end
  before it is done (27.1); its target may be one's own (undirected, no-op)
  (27.2). Ayelea: "give you control of a Tremere antitribu until the end of
  your next turn as a Ⓓ action" — another Methuselah's, as it is (locked, in
  torpor); "your next turn" is the one after the turn the action resolves
  in: the vampire returns at its end, as Puppet Master's (69.3). Chameleon:
  "who entered play since your last minion phase" — since the end of the
  player's previous minion phase, entries of the current minion phase
  counting; X is the target's capacity, paid in blood; the target ready or
  in torpor (70.3).
- Stealing mid-action: a stolen blocker's attempt fails without barring it;
  a stolen combatant's combat ends after the damage still pending resolves
  (27.3, LSJ 20020618).
- Return: a temporarily controlled minion that cannot return (its Methuselah
  ousted) is removed from the game (ANK 20181110); its owner's cards on it
  go with it; other Methuselahs' cards on it are burned to their ash heaps
  (27.4). Advanced rules §6.4.1 was fixed to agree with §6.2.4: removed when
  it should return to an ousted Methuselah; only a return that can no longer
  occur (Parmenides) leaves it with the current controller (28.13).
- Taking control of a unique vampire whose copy one already controls: by
  choice (Puppet Master's take) not offered; forced, the incoming copy burns
  (advanced rules §1.13.1) (58.19).
- Graverobbing's steal: 31.5 (§11.5).
- Form of Corruption: "anew" — the prey did not hold the Edge before; the
  counter optional; the steal in its controller's master phase, no master
  phase action, a vampire of the current prey's, in torpor too, with no more
  blood than the counters (0 counters, a 0-blood vampire: literal); for good
  (76.13).
- Juniper: an ally of her controller's predator or prey, chosen as the
  action is announced, directed at its controller, re-checked at resolution
  (D13); a permanent steal, as Xipe Totec's (78.4).
- Malkavian Dementia: a ready Malkavian of another Methuselah until its
  taker's next unlock phase, back as it begins, before any unlock (ANK
  20200629-2), whatever its clan by then (ANK 20221208-2), as it is (LSJ
  20010119); removed if the Methuselah it returns to was ousted (ANK
  20181110), its taker's cards on it burned (owner, 2026-09-06) (78.43).

### 17.3 Taking control of cards in play

- Requirements are not checked (17.4, 31.4; advanced rules §6.3.1); the
  card's placement: a locquipment on a ready minion of the new controller
  able to hold it, else burned (31.4, 39.4); equipment changes host,
  controller follows, owner stays (17.4).
- Disputed Territory's location (39.4), Dominate Kine [DOM] (31.4),
  Confiscation (51.3), Fast Hands (70.11).

## 18. Contests, uniqueness, titles

Advanced rules §1.13 (contests), §5.8 (titles).

### 18.1 Uniqueness, self-contest

- No self-contest: a unique card cannot be played while a copy is in play
  under the same Methuselah — including while that copy is contested (out of
  play, but still contesting incoming copies). It also bars a search from
  placing such a unique (7.1, 11.2, 12.3). A title card put on its caller
  (Praxis Seizure: York, the Crusades) is not called while its player
  claims that title (85.32). Anarch Convert moved out beside a
  copy in play must use his ability (no voluntary self-contest, 57.9).
- Unique cards on record: Shilmulo Tarot (11.2), Ravnos Cache (36.3),
  Camarilla Conclave (51.2), Enkil Cog (50.1), Zaire River Ferry (70.20). Not
  unique: Ashur Tablets — copies never contest, every copy in play counts;
  "only one … each turn" counts any Methuselah's play, a canceled one
  included; the third copy is checked as a copy enters play (44.10, owner
  correction). Created vampires and "Non-unique." vampires (41.5, 58.5).
- Jimmy Dunn: a second Jimmy entering play (influenced, or by any effect)
  burns every other Jimmy in play, any Methuselah's, before any contest
  (60.9).

### 18.2 Contested cards

- A contested card's continuous effects are suspended, not ended (a
  contested Guardian Angel gives no intercept or prevention); they resume if
  the contest resolves in its favor (7.3). A contested vampire's bonuses do
  not apply (13.2, 58.1); a contested torpid vampire stays in torpor (3.1). A
  contested location is out of play and cannot be chosen (39.4). A
  contested Elder Library's hand size ends (8.2).
- The cards on a contested vampire are out of play with it, as a set-aside
  vampire's (advanced rules §1.13.3, §6.4.1): no rider, layer or ability of
  theirs applies (Baltimore Purge, Laptop Computer); a unique one among them
  leaves its own contest until they return, unlocked, with the vampire;
  yielded, it burns with them, none of their burn riders firing (Soul Gem
  of Etrius) (83.1).
- Contest upkeep in the unlock phase is mandatory, ordered freely (7.8).
- Descent into Darkness's vampire out of play: a contested vampire cannot
  act, so never plays it contested (owner); its title's contest and its
  cards' left until it returns (LSJ 20080816), a rival left alone
  uncontested; a copy entering meanwhile is no rival (advanced rules
  §1.13.3), Jimmy Dunn's burn not reaching it; back, it and its cards enter
  their contests again, its remembered title too; still its controller's for
  "controls a copy" (LSJ 20040526); not unlocked in the unlock phase, its
  blood not drained, its cards out of play (owner, confirmed) (78.54).

### 18.3 Title contests

- Two vampires under one Methuselah do contest a title (the same-controller
  exemption covers cards only) (3.1).
- Saulot's Guiding Wisdom ("Unique archetype. Title."): the unique card
  contests first (paid in pool), so the title does not; while contested its
  vampire has no title from it (face down, neither copy's title claimed). A
  non-unique title card (Toreador Justicar) contests as its title, paid in
  blood (19.1).
- Rafael de Corazon: each title rival of his yields in its controller's
  unlock phase (no pay option); Rafael himself pays or yields as usual
  (62.8). Ephor's action strips a contested title (59.13).

### 18.4 Titles

- What a title is, "titled": §3.15. A title card replaces a printed title for
  good: a prince given Toreador Justicar loses Cape Town for the rest of the
  game; once the card is gone he is untitled (19.2).
- Valerius's steal: the prince loses the title (its card, if any, is
  burned); Valerius gets the title with no card, losing any title he held; a
  baron of the same city would contest it (19.6).
- Patsy: the Edge no longer its caller's controller's by resolution, nothing
  happens (no title removed, no pool burned); the title of a card
  representing it burns the card (58.11); Inside Dirt's action likewise
  (77.13). Osebo Kholo's move is a standing condition, the oldest ready
  untitled Laibon Osebo taking the card as the game settles, a tie picked by
  its controller (77.1). "n votes (titled)" is a title of
  its own, removable like any (58.3).
- Karl Schrekt meets a justicar requirement (59.14, §4.4).
- Advanced rules §5.8.2 (off-sect and off-clan titles), §5.8.3 (how
  requirements read titles).
- Calebros: the steal is offered to another Methuselah's older Camarilla
  vampire (older: capacity, strictly), read again at resolution against the
  actor; directed at his controller; Calebros cannot block it, his
  controller's other minions can; the stealer's own title is replaced (one
  title); a torpid Calebros may be the target (76.4).
- Tegyrius, Vizier: his "2 votes (titled)" a generated title, kept through a
  sect change (LSJ 20010714); +2 votes while a ready Victoria Ash is in his
  controller's ready region; no votes in torpor (D25) (78.7).
- Praxis Seizure: York: the unique Camarilla title Prince of York (80.26).

## 19. Replacement, hand, draws, searches, stored cards

Advanced rules §1.9 (replacement), §6.8 (hand, draw and discard).

### 19.1 Replacement

- What counts as a replacement: §3.22 (8.3). With an empty library no card is
  replaced (8.3, 9.4). A card leaving the hand is replaced at once — a cost
  discard (Aksinya Daclau's master card, 59.16; Giotto Verducci's card,
  71.18), Richard de Worde's discard (16.3) — unless a card delays it
  (below).
- Delayed replacements: Mirror Walk's "do not replace until your next
  discard phase" played in one's own minion phase is replaced as that turn's
  discard phase begins (8.4); Liquidation likewise; it needs 7 library cards
  as it is played (62.22); The Unmasking as its controller's next turn's
  discard phase begins (62.29); Lam Into until after combat, replaced as the
  combat ends (57.4); Read the Winds after the action (64.9). Blood Weakens:
  the cards held are those played during the action; an action card played
  using a Discipline is not replaced as played either — it waits for the
  action's end (LSJ 20080809, Gear Up); "until a Methuselah's hand is empty":
  when the next held card is played, an empty hand draws it and those held
  before (40.4). The Meddling of Semsith: "does not replace cards during any
  Methuselah's minion phase" — the replacements are held and drawn as that
  minion phase ends (or the turn ends early), counting against the hand size
  meanwhile (ANK 20180318); the card leaving play mid-phase does not release
  them before the phase's end (70.4).
- Replacement order around searches and draws: Black Market Cache replaced
  before its search (12.4); Victor Donaldson offered once the canceled card is
  replaced (59.12); Fleshforge Chamber (37.1); Pack Alpha's retainer replaced
  as if played from hand (22.2).
- Skidmark: every card his controller plays while he acts, the action card
  announcing it and his modifiers included, is replaced at the action's end,
  after its combats and after-resolution window (Bum's Rush's delay, LSJ
  20001120); a card with its own delay keeps it (78.18).

### 19.2 Extra draws and draw alternatives

- Draw alternatives (Shilmulo Tarot, Black Market Cache): each card drawn
  from the library (replacement, draw-up, Heart of Nizchetus) may instead be
  taken from the stored cards, a choice per card; with Visit from the
  Capuchin, one treatment per replacement (ANK 20200905); an empty library
  means no draw, so no stored card either (9.4). With the Capuchin and a
  stocked draw alternative, each replacement is the Capuchin counter or a
  stored card — not the library top (12.1, ANK 20200905). Visit from the
  Capuchin: 8.3.
- Heart of Nizchetus: the number (up to 3) picked before drawing, capped by
  the library's size; the cards drawn at once (each a draw-alternative
  choice), no discard-down in between; then that same number goes to the
  bottom (11.1), in the order the player chooses (9.5).
- Blackhorse Tanner: any library draw of his controller's — a replacement, a
  draw-up, a card's draw —, each card drawn offered until the use is spent,
  once in each Methuselah's turn, from torpor too; the card goes under the
  library and the new card drawn is not offered again; not offered with no
  other card in the library (62.12).
- Learjet's extra draw comes with every delayed replacement of a card the
  bearer played as the acting minion, whatever releases it (PIB 20110718
  generalized): at the next unlock phase (Old Friends), at the next discard
  phase (Mirror Walk), as Blood Weakens' held replacements are drawn when the
  hand empties, at the end of the action; a replacement The Meddling of
  Semsith holds again brings its extra draw at the minion phase's end with
  it; the draw is keyed to the replacement, so it is offered even if Learjet
  or its bearer left play meanwhile (73.1). The action card its bearer plays
  to take an action brings no extra draw: "replacing the action card is not
  done during the action, but before" (73.2, owner 2026-10-14).
- Infernal Pursuit (63.11, §12.15). Constant Revolution (LSJ 20090618): the
  random cards go all at once ("no replace" = not one at a time, capped by
  hand size, the rest from pool), then the hand is drawn back up (7.10).
- Katherine Stoddard: once during her controller's turn, ready (locked
  allowed): a card printing Auspex discarded from hand, then a draw up to
  hand size (78.15).

### 19.3 Hand size

- When a hand-size bonus ends (Dreams of the Sphinx's +2 at end of turn —
  not during the discard phase, LSJ 20100812; an Elder Library becoming
  contested), the hand is discarded down at once (8.2, rulebook "Drawing
  Cards"); a draw-up after a hand-size increase is not a replacement (8.3).
  Fotini Katsikaris's "+1 hand size until your next discard phase" ends as
  that phase begins, the cards over discarded at once (53.4). Ennoia's
  Theater: the hand-size lock untimed, any Methuselah's turn, "this turn";
  one lock a turn for either use (44.2). Benjamin Rose (58.15), Juliet Parr
  (60.2), Unmada (62.15), Rage of Apedemak (54.8).
- Discards: Delaying Tactics' discard-down is immediate (5.5); Josef von
  Bauren's Methuselah draws back up (71.8).
- A hand over size waits for no pending choice but a draw's own (rulebook
  "Drawing Cards"): Flurry of Action's draw is discarded down before Curse
  of Nitocris's pick (78.51).

### 19.4 Searches and stored cards

- A search finds what qualifies, may find nothing; the library (or crypt) is
  shuffled after, found or not (17.1, 28.1, 59.12, 62.13, 71.16). Sakhar:
  any equipment, equippable or not; Dominica: a master card labeled
  archetype; revealed to hand or none found; hand size not checked then
  (17.1). Fortschritt Library: "only one … per game" anyone's (71.16).
- Searches that equip: 9.3, 38.5 (§11.3); they do not offer a unique already
  in play under the Methuselah (12.3).
- Stored cards: Shilmulo Tarot (11.2, 12.2, 12.5), Black Market Cache (11.3,
  12.4), Maabara (71.12), The Erciyes Fragments (60.19).
- Open hands: SchreckNET until that action ends (58.14); Harold Tanner for
  the combat (71.17).
- Light Intensifying Goggles: an aim card the bearer used, canceled too,
  stored if its player says so, asked each time (`may_store_aim`) (owner);
  played by the bearer only, as if from hand, then removed from the game; no
  "do not replace" (LSJ 20081126); burned with the goggles (LSJ 20021128)
  (80.42). Nefertiti (ADV): her search finds nothing (no Aabbt Kindred in
  the pool), the crypt shuffled; not with an empty crypt (78.20) (80.9).

## 20. Events and Gehenna

Rulebook "Event Cards"; advanced rules §6.8.2.

- An event takes any discard phase action, ordinary or not (an ordinary one
  spent first keeps Powerbase: Los Angeles's open); one event per discard
  phase whatever the actions left; each event once a game across the table
  (a second copy anywhere is never offered) (40.1).
- Thirst's "did not hunt": §3.1 (40.2, LSJ 20050727). Nightmares upon
  Nightmares (40.3). Blood Weakens (40.4). The Uncoiling's random burn: due in
  its controller's discard phase, mandatory, orderable against the phase's
  other effects (40.5). Torpid Blood (40.6). Wormwood (40.7). Recalled to the
  Founder (40.8). FBI Special Affairs Division (40.9). Absimiliard's Army's
  ghoul is put in play (not recruited) and may act the turn it enters; the
  card stays its controller's (removed from the game from their library)
  (40.10). Rise of the Nephtali: the Liaison action has no stealth of its
  own; "younger" = lower capacity (40.11).
- Dragonbound (57.5), Scourge of the Enochians (57.6), The Meddling of
  Semsith (70.4), Antediluvian Awakening (70.9), Kaymakli Nightmares
  (70.10), Waiting Game: a counter as a non-Anarch vampire's action is
  announced, any Methuselah's — an action card canceled as it is played is no
  action (ANK 20210124); "during your unlock phase" its controller's; "each
  anarch", "each ally" every Methuselah's, each anarch's controller choosing;
  an ally at 0 life burns (71.10).
- Karen Suadela's Gehenna immunity (58.10, §4.6). Lilith's Blessing is
  banned (since 2013) and not in the pool (57.8).

## 21. Pool, ousting, victory points

Advanced rules §6.5 (pool, the Edge and ousting).

### 21.1 Ousting

- A Methuselah at 0 pool or below is ousted; a cost may oust its payer (D3).
  Paying one's last pool to vote ousts and casts nothing (33.2).
- Life Boon's decision point: when a Methuselah's pool reaches 0 or below,
  the oust waits while a Methuselah holding Life Boon may save them: each
  holder in turn, clockwise from the active Methuselah, never the active one
  (out-of-turn), nor one ousted by the same effect; one pass over the
  holders; those saved leave the batch and the rest are ousted together, as
  one simultaneous oust; a Methuselah may save any Methuselah of the batch
  (45.1). Life Boon's pool is chosen as it is played (its terms): at least
  what keeps the Methuselah in — what they owe below 0, plus 1 — and at most
  the Boon player's pool less 1: giving never ousts the Boon player (45.2).
  The pool moves as the Boon resolves (a gift, not a cost: no reduction);
  canceled as played, the out-of-turn master phase action is spent and the
  next holder may still save them (45.3).
- While the oust waits, the effect that ran the pool out goes on as if the
  Methuselah were still in the game (the rest of a referendum's effects, the
  action it is part of); if nobody saves them, the oust applies then — on
  their own turn, the turn passes then; a combatant's Methuselah ousted ends
  the combat at once (45.4). The pool a Methuselah is below 0 by is kept
  until the oust applies: what they owe is what a Boon must make good (45.8,
  RTR 19970630).
- A bleed whose target is ousted before it resolves: the oust first, then
  the bleed fizzles (24.1, §10.1) — read against 45.4 in §23.
- A bleed ousting the active Methuselah: the turn passes with the action
  (58.20). Simultaneous pool losses in one batch: Poison Pill (69.10).
- The Black Throne by its controller's oust (33.7). An ousted Methuselah's
  cards: master cards on others' minions removed (7.5); temporarily
  controlled minions removed (27.4, 28.13); corruption counters stay (25.1);
  stored cards to their owners (67.1). Aelswith and Terrifying Visage's
  blocker oust (53.1).
- Burning one's last pool to cancel True Love's Face is not offered (18.2).

### 21.2 Victory points, game end

- A simultaneous oust of every remaining Methuselah is legal. Every
  Methuselah in play at that moment scores the VP for their ousted prey,
  ousted themselves or not; the 6 pool goes only to a non-ousted predator.
  Game win = ≥ 2 VP and strictly more than any other, ousted or not; draws
  are legal (two at 2 VP; four or five all at 1 VP) (6.9; the tournament
  rules' win condition).
- Life Boon's VP: the Boon player gains every VP the Methuselah saved gains
  — a prey's oust, being the last one standing, the half point at the time
  limit (tournament rules) — the oldest Boon first, each until it has given 1
  VP (then burned); "until you are being ousted": the Boon player gains
  nothing from the oust that takes them out — neither the VP for ousting
  them nor the last-one-standing VP, both the saved Methuselah's (45.5). A
  Methuselah saved by a Boon still scores for a prey ousted by the same
  effect (the VP to the Boon player, LSJ 20021029) and gains the 6 pool
  (45.7).
- "If you would get the Edge, it is burned instead" (Leverage): a
  successful bleed then leaves no one the Edge (85.21).
- "Your prey" after an oust is the new prey (32.4, 62.6); "a bleed against
  your prey" that ousted them still was (61.2).
- Lutz von Hohenzollern (62.6), Life Boon's steal (45.6).

### 21.3 The Edge

- The Edge's pool is a normal unlock effect, optional, usable any time in
  the unlock phase (7.7, 7.8). A bleed of 0 or a fizzled bleed takes no Edge
  (24.4, 24.1). Its vote is a Methuselah's own vote (1.1, 19.4); Eat the
  Rich doubles it (70.6). Patsy (58.11), Keith Moody (62.10).
- Curse of Nitocris: put in play under any chosen Methuselah, its player
  included (D11); its controller burns 1 pool each unlock phase; each time a
  Methuselah who did not hold the Edge gets it (LSJ 19971006), its
  controller passes it to any Methuselah, itself included (78.49).

## 22. Engine modeling (see docs/engine.md)

rules-qa items, or parts of them, that record how the engine models play
rather than a rule; listed so the index reaches them. The rule half of a
mixed item is stated in its section.

- Offer pruning, not rules (each repeat or offer is legal): a failed
  attempt is not offered again while nothing changed, nor swapped back
  after a change of blocker (48.6: "an engine reading, not a rule"); an
  ability whose only effect unlocks its own unlocked minion is not offered
  (48.7, an exception to D14 — advanced rules §5.2.4 holds the unlock
  legal). A block attempt is offered whether or not it can reach the
  stealth (73.8); the host hides one that cannot with the opportunities
  off — an intercept card in hand or drawable (48.5), stealth an ability in
  play can still take off (66.2), counting as reach.
- Vocabulary of the window primitives: block attempt pending vs blocked
  before resolution, the as-played cancel sub-window (2.3); strike riders and
  DESIGN §4.4 (2.5); the window-matrix generator's readings of "when …
  attempts to block", "when this vampire burns a minion", "with an optional
  maneuver each combat", type tags read in a fixed order (64.11).
- Enforcement fixes: "once each turn" on an in-play ability enforced across
  the turn (62.32; the rule is D32); bugs seen while testing (63.17: Baltimore
  Purge's conditional stealth, Focus the Blood's discount, Renewed Vigor's
  overflow); a latent bug surfaced by a fixture reshuffle (67.1, the rule
  stated in §17.1).
- Optional offers an agent may take unasked, never worse than declining:
  Fever Pitch's put-on (53.5), Cursed Abattoir's counter (55.6).
- Scoping: Lilith's Blessing banned and removed (57.8); Monocle of Clarity's
  question changes no state (68.11); New Blood's circle first left unmodeled
  (67.6, superseded by 68.1); bare traits with no card reading them yet
  (Red List on Ossian, 64.6; trophies, 67.5; Flight until 84.37); "no tenant"
  clauses (Abraham Mellon's minion clause, 41.1; Elysium, 44.7; a justicar
  requirement in play, 59.14; no mage ally, 41.8, superseded by 62.31; no
  demon or zombie retainer, 62.30, 55.6; no ammo card, 64.10; the G2 advanced
  Theo Bell, 60.1); implementation order of a vampire's text (13.1).

## 23. Contradictions between sources

Each disagreement found between rules-qa, the KRCG rulings (ledger), the
advanced rules and the rulebook, with the reading the digest follows.
"Settled" = the owner ruled knowingly; "open" = a question for a new
rules-qa batch.

### 23.1 The owner over a KRCG ruling (settled)

- **K1** Karl Schrekt: 59.14 (owner, 2026-10-09) — met as a justicar he is a
  titled Camarilla vampire for the card, so he calls Camarilla's Iron Fist
  when no longer Camarilla — over ANK 20250410 ("when he changes sects he
  cannot play votes that require a titled Camarilla vampire"); the ledger
  rows are `n/a: superseded`. Advanced rules §5.8.3 agrees with the owner.
- **K2** CrimethInc.: 62.21 (owner, 2026-10-11) — a recruit, employ or equip
  of a card requiring an Anarch is an action requiring one — over ANK
  20180817, LSJ 20100303, LSJ 20100725 ("requirements apply but do not count
  as requirements for the action") and advanced rules §3.7.4.1, §1.6.3. They
  agree only on a card another card fetches in the action.
- **K3** Business Pressure: 51.5 (owner, 2026-10-07) — each Methuselah
  chooses once, in impulse order, "not paid vote by vote" — against the
  "give and take during the resolution" of RTR 19960530 (and LSJ 20030602)
  that advanced rules §3.7.5.3 reads as one pool at a time, each waiting to
  see; the ledger row for RTR 19960530 is test-backed by the owner's
  one-choice reading, its give-and-take sentence unaddressed.
- **K4** Parijat's toll on a canceled card: 18.1 follows ANK 20260317, which
  supersedes ANK 20260216 (Eagle's Sight example); ledger rows `n/a`.
- **K45** Camarilla Vitae Slave: 76.24 (owner, 2026-10-16) — the chosen
  Discipline lasts until its controller's next master phase, as printed —
  over LSJ 19970224 ("until his controller's next unlock phase"); the
  ledger row is test-backed by the owner's reading, its "burned or stolen"
  half tested, its timing overridden.

### 23.2 Within rules-qa

- **K5** 23.3 (every action has an after-resolution window, a failed one
  included, ANK 20221205; advanced rules §3.5.3 agrees) vs 54.1 (Yawp Court's
  combat ends a failed action "with no after-resolution window") vs 64.4
  (The Kiss of Ra's canceled action has none, consistent with ANK
  20220411). Digest: 23.3 the rule, 64.4 the exception for actions canceled
  before resolution, 54.1 card-specific — **open**: is Yawp Court's an action
  that ends rather than fails?
- **K6** 45.4 (the oust waits; the effect that ran the pool out goes on as
  if the Methuselah were still seated) vs 24.1 (a bleed whose target is
  ousted before it resolves: the oust first, then the bleed fizzles) and
  advanced rules §1.7.1, §6.5.2 ("handle the oust before the canceled
  card"). 24.1 is an oust by another effect mid-action (a cancel cost), 45.4
  the effect that drained the pool — **open** whether 24.1 still holds under
  Life Boon's decision point.
- **K7** Default readings' "an ally pays its last life … its attempt/effect
  then fails" (41.4, 42.2) may be too broad: an ally's last life paid for its own
  action's cost leaves the action resolving (LSJ 20080512, Rotting
  Behemoth); spent on a modifier the action fails, on a strike the combat
  ends first (ANK 20180928-1; advanced rules §5.5.3, §6.5.2). D3 keeps
  rules-qa's wording and flags the action-cost case — **open**.
- **K8** Default readings' "choices are made at declaration … only a random
  or searched result is known at resolution" is overstated: choices inside
  the effect or worded on success are made at resolution (advanced rules
  §1.1.1, §3.4.2, PIB 20150418; rules-qa 25.6, 62.4, 68.10, 69.4, 70.18). D1
  states both.
- **K9** "Legal no-ops stay offered" (17.4, 25.5) vs 48.7 (a self-unlock of
  an unlocked minion not offered): an engine choice, §22; advanced rules
  §5.2.4 holds such unlocks legal.
- **K10** Superseded within rules-qa (the digest states the later): 4.4 →
  47.2; 21.1 → 46.4 → 47.1; 33.1 → 60.22; 38.3 → 38.8; 31.4 → 39.4; 67.6 →
  68.1; 41.8 → 62.31; 48.5, 66.2 → 73.8 (the engine's offer; the host's
  filter keeps their reading); 44.11 removed (replaced by Batch 45); 28.13 records
  advanced rules §6.4.1 fixed to agree with §6.2.4.

### 23.3 rules-qa vs the advanced rules

- **K11** "During X" is once per X (6.6, 15.2, 42.3, 58.7; the rulebook's
  wording template "During X, do Y": one Y per X; 79.1) vs advanced rules
  §1.2.1 (no periodicity clause, no limit — its example, Maris Streck, has
  no "during" and agrees, 79.3) and §1.5.4 (abilities
  repeatable within an action unless the text limits them: Corpse Minion,
  TOM 19960109, LSJ 20020814). The advanced rules lack the template; a judge
  reading them alone rules differently — **open** for the advanced rules (a
  pointer or a new template there).
- **K12** Additional strikes only after the first pair (47.2, owner,
  2026-10-02; rulebook "Additional Strikes") vs advanced rules §4.3.5 ("the
  card granting a limited additional strike may be played before or after
  it, e.g. Quickness", LSJ 20001206, ANK 20211124). The ruling reads "before
  or after" the unlimited Quickness strike, as advanced rules §2.4.1 says;
  §4.3.5 looks like a misreading.
- **K13** Thirst's "did not hunt": 40.2 (any hunt action, blocked or not,
  LSJ 20050727 — "the hunt need not be successful") vs advanced rules
  §3.7.2.1 (Thirst still hits a vampire that gained no blood), citing the
  same ruling against its text.
- **K14** A master card on another's minion: 7.5, 31.5 (stays its player's,
  removed when that player is ousted; rulebook "Master Cards", advanced rules
  §6.6.3, §6.4.3) vs advanced rules §1.12.2 and PIB 20121028 (a card on a
  minion is controlled by the minion's controller, so Anathema stays when its
  player is ousted; a test-backed ledger row). Digest: 7.5, Anathema the
  ruled exception — **open** whether it is one.
- **K15** Diablerie: 5.8 (the after-resolution window after the blood hunt)
  vs advanced rules §2.3.4, §3.7.8.1 (ANK 20201228: after-resolution effects
  fit between the diablerie and the blood hunt); 68.2 gives Slake the
  Thirst a window there — **open** whether Freak Drive-type cards may be
  played before the blood hunt.
- **K16** Anarch Revolt's burn condition: 6.1 (read when the effect
  resolves, ordered freely) vs LSJ 20080106 and advanced rules §5.2.2
  (checked at the end of the unlock phase, even if the Methuselah had a
  ready Anarch earlier). 6.1 lets the burn resolve early while an Anarch is
  present — **open**.
- **K17** Withdrawing a block attempt: 48.3 (owner, 2026-10-03: change of
  blocker, the attempt under way failing) and 30.1 (Dawn Operation's
  canceled attempt withdrawn, the vampire may attempt again) vs advanced
  rules §3.3.2 (a declared attempt cannot be withdrawn, the only exception
  resigning under a blood cost, 69.12 — generalizing LSJ 20081202 on
  Faceless Night: "a minion attempting to block before it is played cannot
  back out") and the rulebook's
  "Detailed course of an action" (no other minion attempts until the current
  attempt is resolved). Owner rules; the digest marks them so.
- **K18** Elen Kamjian at 0 blood: 17.1 (the rulebook hunt takes over) vs
  advanced rules §3.9 (a minion with two different mandatory actions — must
  hunt, must bleed — is stuck; its footnote's LSJ 20001127 is, in the
  ledger, a Rötschreck ruling: the reference is unverified). The ledger's
  ANK 20211009-2 on Elen: another bleed (Flurry of Action) does not
  discharge her forced bleed; having bled already she is stuck (as 17.1) —
  **open** for the 0-blood case.
- **K19** Grants to a class of minions read live (19.3, 39.5, 69.2) vs
  advanced rules §2.5.2, which reads only the members in play as the effect
  resolves (Annabelle Triabell, citing LSJ 20030811, not in the ledger); the
  advanced rules' own notes say the database's Annabelle ruling inverts LSJ
  20030811 (upstream vtes-rulings #11) — **open** at the source.
- **K20** "Ballots": rules-qa calls a Methuselah's own votes ballots (1.1,
  41.3, 42.4, 49.2, 51.1); the rulebook and advanced rules §3.7.6.1 keep
  "ballot" for the prisci (52.9, 70.6). Read as votes (§13); a card reading
  "ballots" (Eat the Rich) means the prisci's.
- **K21** Dreams of the Sphinx's +2 hand size: 8.2 (ends at end of turn, not
  in the discard phase, LSJ 20100812) vs advanced rules §6.8.2 (expires in
  the discard phase, orderable with its effects) and LSJ 20020904-2 (as
  everyone passes at the end of the discard phase, after the discard phase
  actions). Minor; the timings meet at the discard phase's end.
- **K22** Fear of Mekhet's 5 blood all or nothing (49.4) vs advanced rules
  §1.7.5 (pay what you can, the effect fails: Sword of Nuln) — the card's
  wording ("burns 5 blood or is burned") decides.
- **K23** 3.2's "surplus plays are illegal" vs advanced rules §3.2.1 (need is
  a threshold, not a cap: once needed, a play may give more, Angelica). D15
  and §9.2 follow 3.2 — **open**.
- **K24** 1.3's "anyone may play reactions with an unlocked/awoken minion"
  vs the rulebook and advanced rules §2.1.2 (other Methuselahs' minions);
  52.2's "by another as a reaction" is another Methuselah's; Touch of
  Clarity by the acting Methuselah's other vampire is its text (71.26, LSJ
  20090425). §3.8.
- **K25** Dawn Operation: 62.26 (Psychic Assault stays normal) vs advanced
  rules §4.4.3 (Dawn Operation makes environmental and retainer damage
  aggravated too) — likely the cards' wording; unverified.
- **K26** The Meddling of Semsith leaving play: 70.4 (replacements not
  released before the phase's end) vs LSJ 20021008 and advanced rules §6.5.3
  (its effect ends when its player is ousted, the cards drawn at once) — the
  oust case is **open**.
- **K27** Where a moved locquipment goes: 31.4, 39.4 (a ready minion of the
  new controller) vs RTR 19960112 and advanced rules §6.3.2 (any minion,
  burned only if there is none) — **open** (torpid minions).
- **K28** A gun's maneuver commits the strike (10.1, 64.10, 71.5; advanced
  rules §4.3.1) vs advanced rules §4.10.1 ("never commits … may still strike
  with that gun"): the advanced rules contradict themselves; rules-qa
  follows §4.3.1.

### 23.4 rules-qa vs the KRCG ledger

- **K29** Obedience: 56.2 (not against a combat Hedonism queues) vs LSJ
  19991025 (an unlocked older vampire may avoid a combat queued after a
  preceding one, the opponent still the acting vampire); the ledger's `n/a`
  reason ("no V5 card queues a combat with the acting vampire after a
  combat") no longer holds: Hedonism queues one after the block's combat,
  its vampire the acting side (53.3), and Psyche! queues the same two
  minions again (65.1) — **open**.
- **K30** The prisci block: ANK 20180307-1 (vote reductions do not touch
  the prisci block's votes) is `n/a` on the Fee Stake rows ("no Priscus in
  V5") and on Free States Rant ("no Priscus sub-referendum"), against 52.9's
  block and 70.6's prisci; it is in no rules-qa item and bears on 32.9,
  51.2, 52.2, 70.6 — **open** (the rows' statuses).
- **K31** Vote floors: 51.2, 52.2, 70.6 ("a vampire's votes stop at 0",
  citing ANK 20251125) vs ANK 20251125 and LSJ 20050221-2 (net negative
  votes, subtracting nothing). They differ when votes come back (a 0-vote
  vampire at -1 then +1: 0 under the ruling, 1 under the floor) — **open**.
- **K32** A card played by another card: D2 / 41.13 (still played) vs ANK
  20201229 (Concealed Weapon's weapon "is not played by the minion": Louhi
  cannot cancel it) and RTR 20001020 (Dark Influences, Disguised Weapon,
  Sudden Reversal cancel only cards played from the hand in the normal
  fashion: not Piper's ally, not Disguised Weapon's weapon); reconciled by
  advanced rules §1.8.2 (played, but with no as-played window, so never
  canceled as played); a card played "as if from hand" can be canceled as
  played (LSJ 20060106, Approximation of Loyalty).
- **K33** Empty crypt or library draws: LSJ 20060620 (draw as much as you
  can, even from an empty deck) vs RTR 20000501 and LSJ 20070411-2 (a card
  needing a crypt card cannot be played with an empty crypt, Mozambique
  Allure); the ledger applies the specific rulings to Mozambique Allure;
  KRCG left Recruitment Exercise with LSJ 20060620 alone (77.35); related
  60.18, 62.14, 71.9.
- **K34** ANK 20210309-3 (a master card on a vampire offering it an effect:
  the vampire's controller decides, Chanjelin Ward) refines 7.5 and 32.7;
  69.15 follows it; the reference dangles upstream (vtes-rulings).

### 23.5 rules-qa vs the rulebook

- **K35** Steals from oneself: 17.4 (Darius Shirazi, Saankaláxt stealing
  from their own minion, undirected) and 27.2 (Puppet Master on one's own
  vampire, a no-op) vs the rulebook glossary "Steal (a card)": "you cannot
  steal a card from yourself" — **open**.
- **K36** Saulot's Guiding Wisdom: 19.1 (the unique card contests first,
  paid in pool) vs the glossary "Title" (unique title contests are paid in
  blood) — an exception for a unique title card; settled by 19.1.
- **K37** The burn option: 33.8 omits the rulebook's one discard per unlock
  phase and its "not a legal target" clause, and LSJ 20100203-2 (not usable
  while one meets the requirement) — **open** (§16.8).
- **K38** Lightning Reflexes at X = 0 consuming no limited slot (29.8) has
  no ruling; the glossary "Limited" allows one card or effect per round; LSJ
  20030224 (a canceled limited effect does not trigger the limit) supports
  29.8 by analogy.
- **K39** The pre-vote rotation: 64.12 (from the caller's left, the caller
  last) inverts the rulebook's acting-Methuselah-first impulse (rulebook
  "Sequencing", polling step) — the owner's ruling; an explicit exception.
- **K40** Card actions and the basic actions: 6.2, 72.2 (a granted or card
  action has only its printed stealth; "the usual +1" is the rulebook
  hunt's) vs the rulebook's "Action Card (or Card in Play)": a card's special
  version of a basic action follows the basic action's rules. The owner
  confirmed rules-qa's reading (72.2).
- **K41** Game end: 6.9's win condition (≥ 2 VP, strictly more than any
  other) and 45.5's half point at the time limit are the tournament rules';
  the rulebook says only that most VP wins, a tie having no winner;
  withdrawal (1 VP in the rulebook, half a VP in the tournament rules) is in no
  rules-qa item.
- **K42** Allies "as a vampire": 73.6 (the granted capacity) vs the
  rulebook allies box's default capacity 1 when none is printed — no
  conflict for the pool's cards, which print one; the box's other points
  (aggravated damage costs life as normal; an effect sending the ally to
  torpor burns it, LSJ 20060409) are in no rules-qa item.
- **K43** Merging: 69.5's title order is a reading; the rulebook also burns
  the cards and counters on the incoming copy, which 69.5 does not state.

### 23.6 Open at the source

- **K44** Stealing the Edge after its holder burns it mid-action (advanced
  rules `docs/_work/review.md`): RTR 19980623 (Tereza Rostas: "cannot be
  thwarted by burning the Edge") vs TOM 19951214-2 (no steal of an
  uncontrolled Edge); the owner leans to "the steal resolves with no effect"
  (2026-07-22). Reachable with Hand Intervention, Sabbat Threat.

## 24. Open readings

### 24.1 Implemented readings awaiting the owner

Readings recorded in rules-qa as implemented, with no owner confirmation on
record (the batch header or the item says none); the digest states them as
read, for the owner to confirm or correct:

- 69.1–69.11, 69.13–69.15 and 70.5, 70.13, 70.18, 70.20–70.22 graded green
  (two blind reads agree; 70.9 answered by the owner).
- 74.7–74.12 graded green or amber (both reads agree, or a precedent;
  74.1–74.6 and 74.13 answered by the owner).
- 75.1 (75.2 confirmed, 75.3 settled by 44.1).
- 76.1–76.23, 76.25 graded green or amber (76.24 answered by the owner).
- 77.1–77.27, 77.29–77.35 graded green, amber or corrected (77.28 answered
  by the owner: the counters you burned).
- 80.1–80.3, 80.5–80.34, 80.36, 80.39, 80.40, 80.44, 80.45 graded green,
  amber or corrected (80.9; 80.4, 80.35, 80.37, 80.38, 80.41–80.43
  answered by the owner).
- 84.5–84.40 graded green or amber (84.1–84.4 answered by the owner;
  84.5's active Angelo a gap: his own circle of one).
- 85.3–85.34 graded green or amber (85.1 and 85.2 confirmed by the owner).
- 49.6's several-holders question is moot (50.1: the Cog is unique).

### 24.2 Questions for a new rules-qa batch

From §23: K5 (Yawp Court's after-resolution window), K6 (24.1 under the
oust's decision point), K7 (an ally's last life as its action's cost), K11
(the "during" template, for the advanced rules),
K14 (Anathema), K15 (after-resolution plays before a diablerie's blood
hunt), K16 (Anarch Revolt's condition), K18 (Elen Kamjian), K19 (live
grants), K26 (The Meddling of Semsith ousted), K27 (locquipment on a torpid
minion), K29 (Obedience vs a queued combat), K30 (the prisci block's 3
votes), K31 (vote floors), K35 (stealing from oneself), K23 (surplus
stealth or intercept), K37 (the burn option's clauses). Also from the
ledger, not yet a reading anywhere:

- The Fury's block (60.21) against later plays: the `blocked:untested` rows
  for Change of Target, Mirror Walk, Elder Impersonation, Faceless Night,
  Elder Intervention, KRCG News Radio, Tension in the Ranks, Crypt's Sons.
- The Erciyes Fragments: a political card played from them and Delaying
  Tactics; the card on them not shown to other players (`blocked`).
- Nergal's reduction at a redirected bleed's resolution; I am Legion's
  cancel of a do-not-replace, limited or as-if-from-hand card; a banished
  Great Beast (`blocked`).

### 24.3 Principles from the rulings and the advanced rules not in rules-qa

KRCG and advanced-rules principles an implementer must know, stated by
neither rules-qa nor the sections above; each is the source's reading, not
an owner ruling:

- A canceled card's "do not replace until" clause is canceled too; the card
  is replaced normally (LSJ 20080630, LSJ 20011023; advanced rules §1.9.3).
  No replacement draw in the as-played window (LSJ 20061207).
- Burning a card for a vote is not playing it (ANK 20210309-2; advanced
  rules §1.11.1). A canceled limited effect does not use the limit (LSJ
  20030224).
- A card put in play ignoring requirements enters at its basic version, X =
  0, and a minion so put may act that turn (LSJ 20100204, LSJ 20040518-2;
  advanced rules §1.6.3); on-equip clauses do not fire then.
- An effect letting a minion play cards requiring a Discipline does not give
  the Discipline, never stacks into superior, and meets one requirement
  only (LSJ 19980303, ANK 20180111, LSJ 20020218, LSJ 20011217). Meeting a
  title requirement for a political action gives none of that title's votes
  (ANK 20231221). "Requires a baron" counts as requiring an Anarch (LSJ
  20080603); a negative requirement ("non-Camarilla") does not (ANK
  20250320).
- X can be 0, never negative (LSJ 20030502); cost arithmetic: advanced
  rules §1.7.2.
- Equip, employ or recruit outside an action is not that action: the
  vampire may still take it normally; action-keyed reductions do not apply
  (LSJ 20080815, LSJ 20090115-1; 63.9's Concealed Weapon).
- An action targeting an ash heap cannot be played with no target in any
  ash heap (ANK 20200420-3); an equipment-granted action fizzles if the
  equipment is no longer on the actor at resolution (RTR 19960221); the
  acting minion leaving the ready region before block resolution ends the
  action, the blocker not locked (rulebook "Resolve the Action").
- Fail effects cannot be used once a block is attempted or succeeded (ANK
  20260316, LSJ 20041022); a failed action reached resolution (NRA) and is
  not blocked (ANK 20211015, LSJ 20070219). After-resolution plays are inside
  the action: "during an action" includes the after-resolution window (ANK
  20171124).
- Minus stealth and minus intercept may be played any time in the action,
  no attempt needed (LSJ 20020612). A minion that paid blood to attempt can
  resign (LSJ 20031121-2, 69.12). Blocking after waking is optional (TOM
  19951129). Several minions may each play their copy of a modifier in one
  action (ANK 20200515); a locked vampire can play a modifier on its own
  action (RTR 19941109).
- Once set, range cannot be reset (RTR 19970630); an earlier range-setting
  effect has priority (PIB 20120214). A minion that cannot strike cannot use
  strike maneuvers (LSJ 20001126). "Cannot be dodged" leaves the dodge
  playable, to no effect (LSJ 20030902-2). Added damage inherits the base
  damage's properties (RTR 19970630). Environmental damage goes through a
  dodge (RTR 19941109); retainer damage is environmental (ANK 20180612).
  "Prevents n" with fewer pending prevents all (RTR 20041202); leftover
  prevention carries over only if the card says "can" (ANK 20200318).
  Press-step effects do not happen if combat ends early (LSJ 20010312).
  Damage after combat is not combat damage: no combat card prevents it
  (LSJ 20100204-2).
- Blood hunts and automatically passing referendums take no polling-step
  effects (PIB 20150105, LSJ 19980107). Gaining votes "against" forces that
  side (LSJ 20031115).
- A temporarily controlled minion returns from torpor, staying in torpor
  (LSJ 20010119). Crypt cards never go to the library nor library cards to
  the crypt (LSJ 20041201); a created vampire that would go to the crypt is
  burned (LSJ 20041201-2), is no vampire in the ash heap (LSJ 20020115) and
  stays one banished (RTR 19990712). A vampire that burns and returns is a
  new vampire (LSJ 20100213). Cards on a card leaving play burn (LSJ
  20021128). Black Hand is a trait, kept through a sect change (LSJ
  20070322).
- "Once each turn" is once in each player's turn (ANK 20180719-4); "since
  your last turn" runs from the end of your last turn to now (LSJ
  20020429); a card replaced "next turn" is replaced before unlocking (ANK
  20200629-2).
- Advanced rules pointers: a "cannot" naming cards bars playing them even to
  cycle, one naming an outcome leaves the play legal (§1.1.3, LSJ 20050628);
  referents are re-read at resolution (§1.1.4); a limit travels with the
  copy, a canceled use spends none (§1.2.1); "reduced to X" is an event
  (§1.2.2); card-play costs and bans do not reach abilities (§1.5.1); a lock
  or torpor bars no ability unless the text says so (§1.5.2); a card's cost
  and its gain are one step (§1.7.1); cancellation reaches downward only
  (§1.8.4); an oust removes rather than burns (§1.10.1); hosted cards burn
  with the host except into torpor, the uncontrolled region or a contest
  (§1.12.1); entry effects resolve before a contest (§1.13.2); "X" sets,
  "+X" adds (§1.15); effects are atomic (§2.1.3); values a card writes are
  fixed at play (§2.5.2); an action card voided by a standing "no effect"
  bar is no action (§3.4.1); continuing needs the action card still in the
  ash heap (§3.6.1); a provided action is not an action card (§3.8.2); a
  minion burned in combat is burned by its opponent (§5.4.1).

## 25. Index of rules-qa items

Every rules-qa item, by batch, with the digest sections citing it: the
section stating it and those cross-referring to it (§2 the default
readings, §22 engine modeling, §23 the contradictions, §24 the open
readings). 853 items, Batches 1–86 (44.11 was removed, replaced by Batch
45). The index is derived from the citations: a bare `N.M` cites rules-qa
item N.M in the section it appears in. A new rules-qa item gets its line in
the section stating it, then its entry here: `python3
scripts/digest_index.py` writes the entries, `--check` fails on a stale
index or an item cited nowhere.

- **1** — 1.1 §13,§13.2,§21.3,§23.3; 1.2 §3.21,§5.1,§9.4,§12.1; 1.3
  §2,§3.8,§9.2,§9.4,§11.1,§23.3; 1.4
  §3.2,§3.16,§6.2,§8.5,§10.3,§11.1,§11.3,§14.1,§14.3; 1.5 §6.2; 1.6 §9.2
- **2** — 2.1 §5.2,§12.12; 2.2 §5.1; 2.3 §5.2,§22; 2.4 §5.2,§9.1,§9.2,§14.1;
  2.5 §5.2,§12.3,§12.11,§22
- **3** — 3.1 §14.2,§18.2,§18.3; 3.2 §2,§9.2,§23.3; 3.3 §6.2; 3.4
  §9.4,§10.5; 3.5 §2,§13.1; 3.6 §2,§3.2,§3.14,§7.3,§8.3,§10.2,§10.4,§11.6;
  3.7 §5.1,§6.1
- **4** — 4.1 §10.5,§12.12; 4.2 §12.7; 4.3 §2,§12.4,§12.10; 4.4
  §2,§12.9,§23.2; 4.5 §3.10,§12.1
- **5** — 5.1 §13.2; 5.2 §11.7,§13.2; 5.3 §5.2,§13.3; 5.4 §13.2; 5.5
  §10.3,§13.3,§19.3; 5.6 §13.1; 5.7 §14.6,§16.4; 5.8
  §3.2,§5.2,§10.4,§11.7,§13.5,§23.3; 5.9 §10.2
- **6** — 6.1 §5.3,§6.1,§23.3; 6.2
  §7.2,§8.4,§8.5,§8.6,§13.1,§13.2,§13.3,§23.5; 6.3 §13.1; 6.4 §5.3,§14.2;
  6.5 §16.2; 6.6 §2,§4.2,§6.1,§6.5,§16.2,§23.3; 6.7 §4.1; 6.8
  §3.4,§8.2,§8.4,§8.6,§9.1,§16.1; 6.9 §21.2,§23.5
- **7** — 7.1 §18.1; 7.2 §16.2; 7.3 §18.2; 7.4 §4.2,§16.1; 7.5
  §2,§3.9,§16.4,§16.6,§17.1,§21.1,§23.3,§23.4; 7.6 §14.2; 7.7
  §5.1,§16.7,§21.3; 7.8 §5.3,§6.1,§18.2,§21.3; 7.9 §5.1,§6.2; 7.10
  §5.4,§19.2
- **8** — 8.1 §9.2; 8.2 §5.5,§6.6,§15.1,§18.2,§19.3,§23.3; 8.3
  §3.22,§10.3,§19.1,§19.2,§19.3; 8.4 §3.2,§6.5,§9.6,§10.3,§19.1
- **9** — 9.1 §2,§4.4,§16.1,§16.3; 9.2 §3.17,§11.3; 9.3 §11.3,§19.4; 9.4
  §19.1,§19.2; 9.5 §19.2; 9.6 §4.2,§9.2
- **10** — 10.1 §12.3,§12.13,§23.3; 10.2 §12.9; 10.3 §3.20,§12.8; 10.4
  §4.2,§12.10,§12.11; 10.5 §12.3
- **11** — 11.1 §5.4,§19.2; 11.2 §4.2,§16.3,§18.1,§19.4; 11.3 §16.3,§19.4
- **12** — 12.1 §19.2; 12.2 §3.7,§16.3,§19.4; 12.3 §11.3,§18.1,§19.4; 12.4
  §16.3,§19.1,§19.4; 12.5 §2,§4.4,§16.3,§19.4
- **13** — 13.1 §15.1,§22; 13.2
  §2,§3.4,§3.15,§4.3,§5.5,§9.3,§12.4,§14.2,§15.1,§16.2,§18.2
- **14** — 14.1 §12.11; 14.2 §2,§4.2,§12.3,§12.4,§12.7,§12.8,§12.9,§12.11
- **15** — 15.1 §2,§4.2,§9.2; 15.2
  §2,§3.12,§4.2,§5.1,§5.5,§6.2,§6.5,§9.2,§9.3,§11.1,§11.7,§12.1,§13.3,§14.2,§15.1,§23.3
- **16** — 16.1 §2,§4.2,§14.1; 16.2 §9.6,§11.7; 16.3 §4.2,§11.1,§19.1; 16.4
  §2,§3.2,§3.4,§4.2,§10.4,§11.6,§11.7; 16.5
  §3.10,§6.5,§9.6,§10.4,§12.1,§13.4,§13.5; 16.6 §4.2,§8.5,§14.10,§15.2
- **17** — 17.1 §2,§6.3,§8.1,§8.7,§14.4,§15.1,§19.4,§23.3; 17.2
  §2,§3.4,§5.3,§6.3,§8.7; 17.3
  §3.1,§3.2,§3.4,§3.15,§9.1,§10.2,§10.5,§11.2,§11.8,§15.1; 17.4
  §2,§3.2,§3.4,§3.5,§3.7,§3.12,§3.17,§4.4,§8.1,§10.2,§11.5,§14.1,§14.4,§15.1,§16.1,§17.3,§23.2,§23.5
- **18** — 18.1 §7.2,§9.4,§23.1; 18.2 §2,§7.2,§21.1; 18.3 §4.6,§12.13; 18.4
  §2,§4.3,§11.1
- **19** — 19.1 §18.3,§23.5; 19.2 §3.15,§18.4; 19.3 §2,§4.3,§13.2,§23.3;
  19.4 §13.2,§21.3; 19.5 §12.6,§12.12; 19.6 §18.4
- **20** — 20.1 §2,§3.4
- **21** — 21.1 §2,§5.5,§12.4,§12.6,§12.10,§12.13,§16.5,§23.2; 21.2 §6.1;
  21.3 §3.20,§12.8
- **22** — 22.1 §12.3; 22.2 §3.7,§7.1,§11.4,§14.8,§19.1; 22.3
  §12.4,§12.6,§12.10; 22.4 §3.17,§12.13
- **23** — 23.1 §3.2,§9.6,§10.3; 23.2 §3.16,§8.5,§11.6; 23.3
  §3.2,§5.2,§5.3,§5.5,§10.4,§23.2; 23.4 §7.3; 23.5 §2,§4.3,§11.1; 23.6
  §7.3,§8.4
- **24** — 24.1 §10.1,§21.1,§21.3,§23.2,§24.2; 24.2 §5.2,§10.1; 24.3
  §3.3,§10.1; 24.4 §3.2,§10.4,§11.1,§21.3; 24.5 §2,§3.4,§9.2
- **25** — 25.1 §3.9,§14.6,§14.11,§21.1; 25.2 §4.5,§12.2,§12.7; 25.3
  §4.5,§9.2; 25.4 §14.11; 25.5 §2,§4.1,§9.2,§14.1,§14.11,§23.2; 25.6
  §2,§4.5,§11.1,§23.2
- **26** — 26.1 §8.1,§10.2,§11.8; 26.2 §12.2; 26.3 §8.1; 26.4 §8.1; 26.5
  §2,§6.3,§14.1; 26.6 §2,§12.4,§12.6,§12.10; 26.7 §5.1,§12.4; 26.8
  §3.13,§8.1
- **27** — 27.1 §5.3,§17.2; 27.2 §3.4,§8.1,§11.8,§17.2,§23.5; 27.3
  §9.1,§12.1,§17.2; 27.4 §16.4,§17.2,§21.1; 27.5 §3.21,§8.8,§11.8,§12.1;
  27.6 §7.3; 27.7 §3.9,§14.6
- **28** — 28.1 §3.6,§8.7,§14.7,§19.4; 28.2 §2,§4.1,§16.1; 28.3
  §3.6,§14.4,§14.6; 28.4 §14.4; 28.5 §2; 28.6 §2,§14.4,§16.1; 28.7
  §3.2,§10.2,§11.6,§11.7; 28.8 §2,§3.1,§8.7,§11.2; 28.9 §5.3,§6.4; 28.10
  §7.1,§11.4; 28.11 §3.19,§12.14; 28.12 §6.5,§16.1; 28.13 §17.2,§21.1,§23.2
- **29** — 29.1 §12.1,§12.8; 29.2 §12.7,§12.13,§14.8; 29.3 §3.20,§12.7; 29.4
  §3.20,§12.2,§12.7; 29.5 §12.14; 29.6 §7.3; 29.7 §12.4; 29.8
  §12.9,§12.15,§23.5; 29.9 §12.1; 29.10 §5.1; 29.11 §7.2
- **30** — 30.1 §9.4,§23.3; 30.2 §8.3; 30.3 §10.4; 30.4 §9.6; 30.5
  §3.3,§9.6,§10.5,§11.1; 30.6 §12.8
- **31** — 31.1 §14.1; 31.2 §1,§2,§3.5,§11.1; 31.3 §3.10,§12.12; 31.4
  §4.4,§11.5,§16.1,§17.3,§23.2,§23.3; 31.5
  §3.9,§11.5,§16.4,§16.6,§17.1,§17.2,§23.3; 31.6 §3.9,§14.3,§17.1; 31.7
  §5.5,§12.3,§12.4
- **32** — 32.1 §3.16,§8.5; 32.2 §11.1; 32.3 §6.2; 32.4
  §2,§4.3,§11.1,§11.5,§21.2; 32.5 §3.4,§8.1,§16.4; 32.6 §3.6,§14.7,§14.10;
  32.7 §2,§3.11,§4.1,§23.4; 32.8 §11.1,§14.5; 32.9 §3.15,§13.2,§23.4; 32.10
  §6.3; 32.11 §2,§16.7; 32.12 §2,§8.1; 32.13 §2,§3.1,§8.7,§11.2
- **33** — 33.1 §2,§5.3,§6.2,§23.2; 33.2 §2,§13.2,§21.1; 33.3 §2,§4.1,§16.4;
  33.4 §8.1; 33.5 §5.4,§14.3,§14.4,§16.4; 33.6 §12.1,§12.11; 33.7
  §3.10,§14.3,§21.1; 33.8 §6.1,§14.1,§16.8,§23.5; 33.9 §3.3,§8.4
- **34** — 34.1 §9.2; 34.2 §9.2; 34.3 §4.5; 34.4 §11.1,§14.3; 34.5
  §11.1,§16.1; 34.6 §3.12,§16.1; 34.7 §3.7,§6.4,§14.6,§14.10,§16.1,§16.7;
  34.8 §8.2
- **35** — 35.1 §2,§3.5,§8.2,§16.1; 35.2
  §5.4,§7.3,§12.4,§14.4,§14.11,§16.1,§16.7; 35.3 §4.2,§11.3; 35.4
  §10.2,§13.3
- **36** — 36.1 §5.2,§7.3,§16.7; 36.2 §7.3; 36.3 §2,§4.2,§18.1; 36.4
  §2,§4.2; 36.5 §2,§6.4,§7.3,§11.4
- **37** — 37.1 §4.2,§11.4,§19.1
- **38** — 38.1 §2,§8.1; 38.2 §3.4,§8.1; 38.3 §4.6,§12.8,§23.2; 38.4
  §2,§3.9,§3.12,§7.3,§12.13,§16.3,§16.4; 38.5 §2,§7.3,§11.3,§19.4; 38.6
  §2,§3.5,§8.6,§11.8,§13.1; 38.7 §2,§3.7,§7.1; 38.8 §2,§4.2,§12.8,§23.2
- **39** — 39.1 §13.1; 39.2 §13.1; 39.3 §13.1; 39.4
  §11.5,§13.1,§16.1,§17.3,§18.2,§23.2,§23.3; 39.5 §2,§4.3,§11.1,§14.5,§23.3
- **40** — 40.1 §2,§6.5,§20; 40.2 §3.1,§6.3,§11.2,§20,§23.3; 40.3 §14.1,§20;
  40.4 §19.1,§20; 40.5 §2,§5.3,§6.5,§20; 40.6 §11.6,§20; 40.7
  §14.4,§16.7,§20; 40.8 §14.2,§20; 40.9 §12.7,§12.12,§20; 40.10
  §2,§3.7,§11.4,§20; 40.11 §2,§3.13,§8.4,§20
- **41** — 41.1 §4.4,§22; 41.2 §4.4,§16.2; 41.3 §11.7,§13,§13.4,§23.3; 41.4
  §2,§3.14,§9.5,§14.5,§23.2; 41.5
  §2,§3.4,§3.6,§8.2,§10.4,§11.7,§12.1,§14.7,§18.1; 41.6 §2,§8.1; 41.7
  §2,§4.2,§14.8,§16.4; 41.8 §2,§3.6,§4.2,§14.5,§22,§23.2; 41.9
  §2,§3.12,§4.2,§7.3,§12.1,§12.3; 41.10 §4.2,§5.1,§12.1,§12.2; 41.11
  §5.3,§6.1; 41.12 §3.12,§14.1; 41.13 §2,§3.6,§7.1,§7.3,§23.4; 41.14
  §2,§3.13,§15.2
- **42** — 42.1 §2,§3.14,§7.3,§8.3; 42.2
  §2,§3.8,§7.3,§12.4,§12.10,§14.5,§23.2; 42.3 §2,§4.2,§23.3; 42.4
  §2,§3.13,§9.5,§11.7,§13,§13.4,§23.3; 42.5 §2,§3.12,§7.3; 42.6
  §3.2,§7.3,§8.3,§10.2; 42.7 §3.3,§3.18,§8.1,§11.1; 42.8 §3.14,§9.1,§9.4;
  42.9 §2,§4.2
- **43** — 43.1 §3.2,§8.1,§10.2,§13.1,§13.5
- **44** — 44.1 §4.2,§12.12,§24.1; 44.2 §5.1,§19.3; 44.3 §3.3,§9.2,§11.1;
  44.4 §3.7,§11.4,§14.5,§14.7; 44.5 §3.5,§4.2,§6.4,§14.10; 44.6 §9.3,§11.7;
  44.7 §12.2,§16.1,§22; 44.8 §12.5; 44.9 §12.4,§12.6,§12.10; 44.10 §2,§18.1
- **45** — 45.1 §5.4,§21.1; 45.2 §21.1; 45.3 §2,§3.14,§6.2,§21.1; 45.4
  §12.1,§21.1,§23.2; 45.5 §21.2,§23.5; 45.6 §6.1,§21.2; 45.7 §21.2; 45.8
  §21.1
- **46** — 46.1 §2,§7.1,§7.2,§13.3; 46.2 §8.6,§13.2; 46.3 §13.2,§16.1; 46.4
  §2,§16.5,§23.2
- **47** — 47.1 §2,§16.5,§23.2; 47.2 §2,§12.9,§12.15,§23.2,§23.3
- **48** — 48.1 §2,§9.3; 48.2 §2,§12.9,§12.15; 48.3 §9.4,§9.6,§23.3; 48.4
  §5.2,§12.3; 48.5 §9.4,§22,§23.2; 48.6 §22; 48.7
  §2,§6.3,§6.5,§14.1,§22,§23.2; 48.8 §2,§8.3,§11.6
- **49** — 49.1 §13.1; 49.2 §13,§13.4,§23.3; 49.3 §4.6,§12.13,§14.8; 49.4
  §14.3,§16.4,§23.3; 49.5 §2,§3.11,§4.1,§11.2,§16.1; 49.6 §5.1,§6.3,§24.1;
  49.7 §3.12,§4.4,§14.10
- **50** — 50.1 §6.3,§18.1,§24.1; 50.2 §6.3; 50.3 §3.3,§6.3,§11.1; 50.4
  §9.2,§16.1; 50.5 §2,§3.10,§13.1; 50.6 §14.3,§16.4
- **51** — 51.1 §5.4,§13,§13.4,§23.3; 51.2
  §2,§4.2,§11.7,§13.2,§13.5,§18.1,§23.4; 51.3 §3.17,§11.3,§17.3; 51.4
  §5.1,§6.3; 51.5 §5.4,§13.2,§23.1; 51.6 §7.2,§13.3; 51.7 §2,§6.6,§9.2; 51.8
  §7.2,§13.3
- **52** — 52.1 §4.5; 52.2 §2,§3.8,§13.2,§13.5,§23.3,§23.4; 52.3 §2,§13.1;
  52.4 §3.3,§11.1; 52.5 §4.3,§13.1; 52.6 §3.4,§8.6,§11.8; 52.7 §13.2,§16.1;
  52.8 §4.2,§9.4; 52.9 §13,§13.2,§23.3,§23.4; 52.10 §2,§8.3
- **53** — 53.1 §9.5,§21.1; 53.2 §3.14,§9.1,§14.5; 53.3 §12.1,§12.12,§23.4;
  53.4 §5.5,§19.3; 53.5 §11.1,§16.5,§22; 53.6 §3.2,§5.3,§11.1; 53.7
  §2,§4.1,§16.4; 53.8 §3.1,§11.2
- **54** — 54.1 §3.2,§5.2,§5.5,§10.1,§13.1,§16.1,§23.2; 54.2 §6.1,§16.7;
  54.3 §12.12,§16.5; 54.4 §2,§8.3; 54.5 §4.3,§6.5; 54.6 §12.3; 54.7 §12.12;
  54.8 §4.6,§5.5,§12.15,§19.3
- **55** — 55.1 §12.5,§12.7,§12.11; 55.2 §12.12; 55.3 §2,§3.14,§8.3; 55.4
  §8.3; 55.5 §10.4; 55.6 §14.3,§22; 55.7 §9.6,§11.8; 55.8 §12.5
- **56** — 56.1 §3.2,§10.3,§12.12; 56.2 §3.2,§9.6,§11.8,§23.4
- **57** — 57.1 §11.1,§12.7,§14.5; 57.2 §2,§4.1,§4.2,§9.2; 57.3 §4.2,§12.4;
  57.4 §12.3,§12.4,§12.12,§19.1; 57.5 §2,§5.3,§6.5,§20; 57.6 §2,§6.5,§20;
  57.7 §14.4,§16.1; 57.8 §20,§22; 57.9 §3.7,§14.6,§18.1; 57.10 §2,§9.1,§14.1
- **58** — 58.1 §3.14,§8.3,§8.4,§11.9,§15.1,§18.2; 58.2 §12.6,§12.10; 58.3
  §3.15,§18.4; 58.4 §2,§14.1; 58.5 §2,§15.2,§18.1; 58.6 §4.6,§11.8; 58.7
  §2,§4.2,§15.2,§23.3; 58.8 §2,§13.3; 58.9 §4.2,§7.3,§14.1; 58.10 §4.6,§20;
  58.11 §18.4,§21.3; 58.12 §2,§3.7,§11.3,§14.6; 58.13 §8.5,§8.8,§9.2; 58.14
  §3.2,§13.5,§19.4; 58.15 §5.5,§13.1,§19.3; 58.16 §13.1,§14.4; 58.17
  §7.3,§11.3; 58.18 §12.8; 58.19 §17.2; 58.20 §6.3,§21.1
- **59** — 59.1 §3.14,§4.2,§8.1,§11.2; 59.2 §4.2,§12.3; 59.3
  §3.4,§8.6,§14.3,§16.4; 59.4 §3.1,§11.2,§12.2,§14.1,§14.11,§16.4; 59.5
  §3.12,§7.3,§11.4; 59.6 §3.10,§12.12; 59.7 §3.15,§11.7,§13.3; 59.8
  §4.2,§5.1,§12.1,§12.8; 59.9 §3.3,§4.3,§13.5; 59.10 §3.20,§4.2,§12.8; 59.11
  §3.15,§14.6; 59.12 §7.2,§19.1,§19.4; 59.13 §3.15,§8.6,§16.4,§18.3; 59.14
  §3.15,§4.4,§18.4,§22,§23.1; 59.15 §9.1,§14.4,§16.3; 59.16
  §4.2,§11.1,§19.1; 59.17 §4.6,§12.11,§12.12,§16.7; 59.18 §4.2,§13.3; 59.19
  §3.15,§4.3,§13.3
- **60** — 60.1 §14.9,§22; 60.2 §5.5,§13.1,§19.3; 60.3
  §3.6,§3.18,§7.3,§15.2; 60.4 §3.8,§3.18,§15.2; 60.5 §3.12,§7.3,§14.1; 60.6
  §2,§4.2,§14.1; 60.7 §4.2,§7.3; 60.8 §6.4,§14.3; 60.9 §18.1; 60.10 §8.1;
  60.11 §3.13,§4.2,§12.4; 60.12 §9.2,§16.4; 60.13 §2,§3.13,§9.5,§14.5; 60.14
  §13.5,§16.4; 60.15 §3.2,§4.5; 60.16 §2,§13.1; 60.17 §2,§8.1; 60.18
  §2,§3.13,§14.4,§14.7,§23.4; 60.19 §3.9,§5.1,§7.1,§16.1,§17.1,§19.4; 60.20
  §3.6,§4.2,§12.8,§14.7; 60.21 §3.2,§3.4,§3.6,§8.2,§9.6,§10.3,§14.5,§24.2;
  60.22 §2,§5.3,§6.2,§23.2
- **61** — 61.1 §12.12; 61.2 §3.3,§6.3,§11.1,§21.2
- **62** — 62.1 §12.5; 62.2 §2,§3.8,§3.14,§7.3; 62.3 §2,§4.2,§9.1,§14.1;
  62.4 §2,§11.5,§14.4,§14.9,§23.2; 62.5 §6.2; 62.6 §13.5,§21.2; 62.7
  §3.14,§7.3,§8.3; 62.8 §18.3; 62.9 §9.1; 62.10 §4.2,§6.1,§21.3; 62.11
  §4.2,§6.1; 62.12 §4.2,§19.2; 62.13 §2,§6.4,§19.4; 62.14
  §6.3,§8.2,§8.8,§11.8,§14.6,§14.10,§23.4; 62.15 §6.5,§19.3; 62.16
  §7.3,§11.1,§12.4; 62.17 §4.2,§9.3,§11.5,§12.8,§14.1; 62.18 §12.4; 62.19
  §2,§4.2,§5.3,§12.4,§14.11,§16.4,§16.5; 62.20 §16.2; 62.21
  §3.12,§11.9,§23.1; 62.22 §19.1; 62.23 §4.5,§12.6; 62.24
  §3.20,§4.2,§12.4,§12.7,§12.11,§14.5; 62.25 §4.2,§16.1,§16.7; 62.26
  §12.7,§14.10,§23.3; 62.27 §4.2,§12.3,§12.11; 62.28 §9.3; 62.29 §9.3,§19.1;
  62.30 §3.6,§8.2,§15.2,§16.8,§22; 62.31 §3.6,§14.5,§22,§23.2; 62.32 §2,§22;
  62.33 §3.6,§11.8,§14.1,§14.5
- **63** — 63.1 §8.8,§9.6,§11.7,§14.10; 63.2 §3.1,§8.7,§11.2,§11.7; 63.3
  §3.12,§13.3; 63.4 §2,§3.13,§12.2,§12.6,§12.12,§14.5; 63.5
  §2,§3.14,§7.3,§12.4; 63.6 §10.5; 63.7 §4.2,§5.5,§7.3,§12.15,§16.5; 63.8
  §4.2,§11.8,§12.3,§12.9,§14.5; 63.9 §2,§7.1,§11.3,§24.3; 63.10 §3.17,§16.3;
  63.11 §12.15,§19.2; 63.12 §3.1,§11.2; 63.13 §5.4,§8.1; 63.14
  §4.2,§11.1,§16.7; 63.15 §2,§14.2; 63.16
  §3.1,§4.2,§4.4,§9.3,§11.2,§11.5,§12.6,§14.5,§14.7,§15.2; 63.17 §8.1,§22
- **64** — 64.1 §3.1,§3.6,§4.2,§11.2,§11.7,§12.11,§14.5,§15.2; 64.2
  §3.17,§12.13,§16.3; 64.3 §5.1,§12.4; 64.4 §3.2,§8.4,§8.5,§9.6,§10.3,§23.2;
  64.5 §9.2,§12.3; 64.6 §11.8,§12.3,§12.12,§14.5,§14.10,§22; 64.7
  §12.12,§14.11; 64.8 §3.8,§4.5,§9.2,§10.4,§12.12,§12.14; 64.9
  §4.5,§9.6,§19.1; 64.10 §3.17,§12.2,§12.3,§12.7,§12.13,§22,§23.3; 64.11
  §4.5,§22; 64.12 §13.3,§13.5,§23.5
- **65** — 65.1 §12.12,§12.15,§23.4; 65.2 §10.4,§12.7; 65.3 §6.1,§15.2
- **66** — 66.1 §2,§5.2,§9.2; 66.2 §9.4,§22,§23.2
- **67** — 67.1 §3.9,§7.1,§17.1,§21.1,§22; 67.2 §14.5; 67.3 §11.1; 67.4
  §7.1; 67.5 §3.14,§8.3,§8.6,§11.8,§14.10,§22; 67.6 §14.10,§22,§23.2; 67.7
  §3.14,§3.18,§8.1
- **68** — 68.1 §14.6,§14.10,§22,§23.2; 68.2 §5.2,§11.7,§14.4,§23.3; 68.3
  §14.3; 68.4 §12.1,§12.9; 68.5 §2,§15.2; 68.6 §5.5,§11.8,§14.4; 68.7 §15.2;
  68.8 §15.2; 68.9 §7.2; 68.10 §2,§3.11,§4.1,§4.2,§15.2,§16.7,§23.2; 68.11
  §16.3,§22; 68.12 §5.5,§12.12,§16.5; 68.13 §5.2,§10.1; 68.14 §2,§15.2;
  68.15 §15.2; 68.16 §5.1,§14.1; 68.17 §3.4,§8.1,§11.2
- **69** — 69.1 §16.1,§24.1; 69.2 §2,§3.11,§4.1,§23.3; 69.3 §6.6,§17.2; 69.4
  §2,§14.4,§23.2; 69.5 §6.4,§14.9,§14.10,§23.5; 69.6 §3.6,§5.3,§14.6; 69.7
  §9.1,§15.2; 69.8 §3.14,§8.3,§15.2; 69.9 §6.5,§13.5; 69.10
  §5.4,§13.5,§21.1; 69.11 §13.1,§24.1; 69.12 §3.14,§9.5,§23.3,§24.3; 69.13
  §11.1,§24.1; 69.14 §1,§4.4,§14.10; 69.15 §14.6,§16.6,§23.4,§24.1
- **70** — 70.1 §6.2,§13.1; 70.2 §1,§2,§3.11,§4.1,§4.2,§11.1; 70.3
  §3.7,§14.9,§17.2; 70.4 §19.1,§20,§23.3; 70.5 §16.7,§24.1; 70.6
  §13,§13.2,§13.5,§21.3,§23.3,§23.4; 70.7 §12.12; 70.8 §2,§11.1; 70.9
  §14.3,§20,§24.1; 70.10 §2,§14.6,§20; 70.11 §3.7,§12.4,§12.10,§17.3; 70.12
  §4.6,§9.1,§12.3; 70.13 §4.6,§24.1; 70.14 §4.6; 70.15 §12.12; 70.16
  §3.19,§12.11,§12.14; 70.17 §2,§3.8,§3.18,§7.3; 70.18
  §2,§11.4,§14.4,§23.2,§24.1; 70.19 §3.2,§10.4,§14.5; 70.20
  §2,§3.17,§4.4,§16.1,§18.1,§24.1; 70.21 §4.3; 70.22 §12.12,§24.1
- **71** — 71.1 §3.13,§14.6; 71.2 §2,§3.18,§13.5; 71.3 §3.7,§14.9,§16.4;
  71.4 §13.2; 71.5 §12.3,§12.13,§23.3; 71.6 §4.5,§7.2,§12.14; 71.7
  §3.11,§4.1,§4.2,§12.12; 71.8 §2,§6.5,§19.3; 71.9 §14.4,§14.6,§15.2,§23.4;
  71.10 §3.2,§14.3,§14.10,§16.7,§20; 71.11 §1,§9.6; 71.12
  §5.1,§8.6,§16.1,§19.4; 71.13 §3.4,§11.5; 71.14 §2,§3.12,§12.4,§14.5; 71.15
  §13.1; 71.16 §2,§16.1,§19.4; 71.17 §12.1,§19.4; 71.18
  §4.2,§6.5,§9.5,§19.1; 71.19 §6.4,§14.6; 71.20 §12.3; 71.21 §2,§3.4,§11.8;
  71.22 §8.6,§10.4,§14.10; 71.23 §2,§12.4,§12.6,§12.10,§16.3; 71.24
  §2,§3.8,§7.3; 71.25 §3.8,§4.6,§13.3; 71.26 §3.8,§3.18,§7.2,§23.3; 71.27
  §4.2,§11.6
- **72** — 72.1 §2,§3.1,§6.3,§11.2; 72.2 §3.1,§23.5; 72.3 §3.1; 72.4 §3.1;
  72.5 §2,§3.1,§11.2
- **73** — 73.1 §16.3,§19.2; 73.2 §16.3,§19.2; 73.3 §2,§8.6,§16.1; 73.4
  §2,§13.2; 73.5 §3.14,§14.3,§14.5; 73.6 §2,§3.6,§14.5,§16.1,§23.5; 73.7
  §2,§3.10,§3.11,§4.1,§15.2; 73.8 §2,§9.4,§22,§23.2
- **74** — 74.1 §3.20,§24.1; 74.2 §2; 74.3 §10.2; 74.4 §11.2; 74.5 §14.6;
  74.6 §3.20,§24.1; 74.7 §24.1; 74.8 §13.2; 74.9 §12.4; 74.10 §4.2; 74.11
  §15.2; 74.12 §3.15,§24.1; 74.13 §14.2,§24.1
- **75** — 75.1 §12.12,§24.1; 75.2 §12.12,§24.1; 75.3 §12.12,§24.1
- **76** — 76.1 §24.1; 76.2 §3.15; 76.3 §3.15; 76.4 §18.4; 76.5 §13.3; 76.6
  §13.3; 76.7 §14.5; 76.8 §11.4; 76.9 §3.17; 76.10 §12.4; 76.11 §12.4; 76.12
  §11.1; 76.13 §17.2; 76.14 §15.2; 76.15 §3.17; 76.16 §3.17; 76.17 §8.8;
  76.18 §14.7; 76.19 §12.8; 76.20 §15.2; 76.21 §6.5; 76.22 §10.4; 76.23
  §24.1; 76.24 §5.5,§23.1,§24.1; 76.25 §24.1
- **77** — 77.1 §18.4,§24.1; 77.2 §4.4; 77.3 §3.7; 77.4 §7.2; 77.5 §13.4;
  77.6 §2; 77.7 §7.2; 77.8 §2; 77.9 §7.2; 77.10 §10.3; 77.11 §7.2; 77.12
  §3.5; 77.13 §18.4; 77.14 §13.3; 77.15 §13.1; 77.16 §11.8; 77.17 §6.1;
  77.18 §9.2; 77.19 §9.2; 77.20 §2; 77.21 §12.7; 77.22 §12.7; 77.23 §12.7;
  77.24 §7.3; 77.25 §13.4; 77.26 §13.2; 77.27 §24.1; 77.28 §24.1; 77.29
  §24.1; 77.30 §11.2; 77.31 §3.15; 77.32 §15.2; 77.33 §15.2; 77.34 §4.4;
  77.35 §23.4,§24.1
- **78** — 78.1 §3.15; 78.2 §14.1; 78.3 §7.3; 78.4 §17.2; 78.5 §15.2; 78.6
  §6.5; 78.7 §18.4; 78.8 §6.1; 78.9 §14.1; 78.10 §6.2; 78.11 §6.3; 78.12
  §13.5; 78.13 §10.3; 78.14 §9.6; 78.15 §19.2; 78.16 §11.1; 78.17 §7.3;
  78.18 §19.1; 78.19 §12.7; 78.20 §2,§19.4; 78.21 §7.3; 78.22 §15.2; 78.23
  §12.3; 78.24 §12.2; 78.25 §6.5; 78.26 §11.1; 78.27 §12.7; 78.28 §3.7;
  78.29 §11.1; 78.30 §3.15,§6.5,§14.1; 78.31 §16.8; 78.32 §12.2; 78.33
  §12.8; 78.34 §12.8; 78.35 §12.4; 78.36 §16.4; 78.37 §9.3; 78.38 §3.2;
  78.39 §14.3; 78.40 §12.12; 78.41 §16.1; 78.42 §16.3; 78.43 §17.2; 78.44
  §14.10; 78.45 §14.11; 78.46 §3.6; 78.47 §3.6; 78.48 §6.3; 78.49 §21.3;
  78.50 §3.10; 78.51 §19.3; 78.52 §3.5; 78.53 §9.3; 78.54 §18.2
- **79** — 79.1 §2,§9.2,§15.2,§23.3; 79.2 §2,§15.2; 79.3 §2,§15.2,§23.3;
  79.4 §2
- **80** — 80.1 §9.1,§24.1; 80.2 §15.2; 80.3 §14.6,§24.1; 80.4 §9.1,§24.1;
  80.5 §9.1,§13.2,§24.1; 80.6 §7.3; 80.7 §7.3; 80.8 §6.3; 80.9 §19.4,§24.1;
  80.10 §13.2; 80.11 §15.2; 80.12 §6.4; 80.13 §9.2; 80.14 §7.2; 80.15 §13.2;
  80.16 §12.3; 80.17 §16.2; 80.18 §14.6; 80.19 §6.4; 80.20 §12.12; 80.21
  §11.8; 80.22 §13.3; 80.23 §11.1; 80.24 §16.5; 80.25 §14.5; 80.26
  §16.5,§18.4; 80.27 §11.8; 80.28 §11.3; 80.29 §8.2; 80.30 §14.5; 80.31
  §11.5; 80.32 §16.3; 80.33 §3.12; 80.34 §13.3,§24.1; 80.35 §8.2,§24.1;
  80.36 §12.12,§24.1; 80.37 §12.12,§24.1; 80.38 §14.7,§24.1; 80.39
  §11.4,§14.7,§24.1; 80.40 §11.4,§24.1; 80.41 §16.1,§24.1; 80.42 §19.4;
  80.43 §14.2,§24.1; 80.44 §3.7,§24.1; 80.45 §14.7,§24.1
- **81** — 81.1 §5.2
- **82** — 82.1 §9.4; 82.2 §9.4
- **83** — 83.1 §18.2
- **84** — 84.1 §15.2,§24.1; 84.2 §15.2; 84.3 §15.2; 84.4 §15.2,§24.1; 84.5
  §15.2,§24.1; 84.6 §12.7; 84.7 §15.2; 84.8 §12.7; 84.9 §14.10,§15.2; 84.10
  §15.2; 84.11 §15.2; 84.12 §15.2; 84.13 §15.2; 84.14 §15.2; 84.15 §15.2;
  84.16 §15.2; 84.17 §15.2; 84.18 §15.2; 84.19 §15.2; 84.20 §15.2; 84.21
  §15.2; 84.22 §15.2; 84.23 §15.2; 84.24 §15.2; 84.25 §15.2; 84.26 §15.2;
  84.27 §15.2; 84.28 §15.2; 84.29 §15.2; 84.30 §15.2; 84.31 §15.2; 84.32
  §15.2; 84.33 §15.2; 84.34 §15.2; 84.35 §15.2; 84.36 §15.2; 84.37
  §14.10,§15.2,§16.5,§22; 84.38 §15.2; 84.39 §15.2; 84.40 §15.2,§24.1
- **85** — 85.1 §15.2,§16.3,§24.1; 85.2 §15.2,§24.1; 85.3 §15.2,§24.1; 85.4
  §15.2; 85.5 §15.2; 85.6 §15.2; 85.7 §15.2; 85.8 §15.2; 85.9 §15.2; 85.10
  §15.2; 85.11 §15.2; 85.12 §15.2; 85.13 §15.2; 85.14 §15.2; 85.15 §15.2;
  85.16 §15.2; 85.17 §15.2; 85.18 §15.2; 85.19 §15.2; 85.20 §15.2; 85.21
  §15.2,§21.2; 85.22 §15.2; 85.23 §15.2; 85.24 §15.2; 85.25 §15.2; 85.26
  §15.2; 85.27 §15.2; 85.28 §15.2; 85.29 §15.2; 85.30 §15.2; 85.31 §15.2;
  85.32 §18.1; 85.33 §15.2; 85.34 §15.2,§24.1
- **86** — 86.1 §5.2
