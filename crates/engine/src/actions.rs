//! Actions: announcement, the as-announced window, the block loop (rulebook
//! states A/B/C, rules-qa 1.3), redirects, block resolution (combat, or the
//! blocker's diablerie on a leave-torpor), the after-combat window with the
//! Form of Mist continuation, and resolution. The action context lives on
//! the engine so that card plays resolved from the as-played stack can
//! modify it.

use crate::cards::{self, Effect, Hook, Limit, PlayCtx, Requirement, Scope, Stat, Target, Window};
use crate::decision::{ActionKind, Choice, DecisionRequest, WindowLabel};
use crate::game::{Engine, Flow};
use crate::ids::{MinionId, SeatId};
use crate::library::LIBRARY;
use crate::referendum::RefKind;
use crate::state::{ActionId, CardId, CardInPlay};
use crate::window::OrderedWindow;

/// Does the card's first mode carry requirement `r`?
fn lib_requires(card: u16, r: cards::Requirement) -> bool {
    cards::lookup(card).is_some_and(|d| d.modes.first().is_some_and(|m| m.requires.contains(&r)))
}

/// An equipment card the engine can put in play (it has a definition).
fn is_equipment(card: u16) -> bool {
    LIBRARY[card as usize]
        .types
        .contains(&crate::library::CardType::Equipment)
        && cards::lookup(card).is_some()
}

/// The play context of an equipment found in the library and equipped on
/// `acting` (its only mode).
fn found_play(seat: SeatId, acting: MinionId, card: u16) -> PlayCtx {
    PlayCtx::new(seat, card, 0, Some(acting), Target::Minion(acting), 0)
}

/// Rulebook "Resolve Any Block Attempts": A (no attempt), B (an attempt is
/// ongoing), C (blocks declined by all); plus the as-announced window that
/// precedes them (spec §2.2).
/// How an action ended (owner, 2026-09-04): blocked; failed (Archon
/// Investigation: "not successful"); successful; or *fizzled* — successful
/// (not blocked, so after-resolution effects such as Freak Drive apply) but
/// without effect, because its cost could not be paid at resolution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Blocked,
    Failed,
    Successful,
    Fizzled,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ActionState {
    #[default]
    AsAnnounced,
    Open,
    Attempt(MinionId),
    Declined,
    /// A bleed of 1 or more whose blocks were declined, before it resolves:
    /// the cards played "if a bleed would be successful" (Spying Mission
    /// [OBF]), after every other modifier and reaction; only cards of that
    /// timing follow one (LSJ 19980105, LSJ 20110502), a canceled one
    /// included (LSJ 20050607), and none once the action has failed.
    WouldSucceed,
    /// Blocked, before block resolution: effects the acting Methuselah uses
    /// "if blocked" (Crypt's Sons continuing as if unblocked); then the
    /// combat (or, on a leave-torpor, the diablerie offer).
    Blocked(MinionId),
    /// After the block's combat(s) (rules-qa 2.1): unlock reactions of the
    /// blocker, the acting minion's continuation, then the action ends
    /// blocked or re-enters the block loop.
    AfterCombat,
    /// After resolution of a successful action (for a political action,
    /// after its referendum: spec §3.7.5.1): Voter Captivation, Freak
    /// Drive.
    AfterResolution,
}

/// A bonus for the rest of the action under a condition read live on a
/// minion, whenever the stat is read ("with an additional +1 intercept if
/// the acting minion is titled", Protection Racket: a change of sect or
/// title during the action gains or loses it, ANK 20190416).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Conditional {
    /// Bleed (the action's) or Intercept (of `to`).
    pub stat: cards::Stat,
    /// The minion given the intercept.
    pub to: MinionId,
    /// The minion the condition is read on.
    pub on: MinionId,
    /// The seat that played the card (whose "you" the condition reads).
    pub seat: SeatId,
    pub requires: &'static [cards::Requirement],
    pub n: i8,
    /// A limited bleed increase: it counts against the limit only while its
    /// condition holds (Foreshadowing Destruction, RTR 20191031).
    pub limited: bool,
}

#[derive(Clone, Debug, Default)]
pub struct ActionCtx {
    pub seat: SeatId,
    pub acting: MinionId,
    pub kind: ActionKind,
    /// The action card, for card actions (employ …).
    pub card: Option<PlayCtx>,
    /// The card in play or vampire granting the action, for granted
    /// actions (Anarch Revolt's referendum, a vampire's printed action).
    pub source: Option<cards::AbilitySource>,
    /// Ability index on `source`.
    pub ability: Option<u8>,
    /// The action's announced target, for a granted action that names one
    /// (a minion to enter combat with): directed at its controller unless
    /// that is the acting Methuselah (spec §3.1.5).
    pub target: Target,
    pub torpor_target: Option<MinionId>,
    /// The vampire that diablerised in this action: the acting minion, or
    /// the blocker of a leave-torpor action (rules-qa 16.2).
    pub diablerist: Option<MinionId>,
    /// The Methuselah being bled (changes on a redirect).
    pub bleed_target: Option<SeatId>,
    pub base_stealth: u8,
    pub stealth_mods: u8,
    /// Stealth taken off the acting minion (Screamer).
    pub stealth_reduce: u8,
    /// "Allies cannot block this action" (Visions of Gehenna).
    pub allies_cannot_block: bool,
    /// The acting minion's strength this action (Invigorate [ani]).
    pub strength_bonus: u8,
    /// The acting minion's hand strikes are aggravated this action
    /// (Invigorate [pro]).
    pub aggravated_hands: bool,
    pub bleed_mods: u8,
    /// The rulebook's one "(limited)" bleed increase per action, used by an
    /// unconditional increase (conditional ones: `limited_used`).
    pub limited_bleed_used: bool,
    pub state: ActionState,
    pub window: OrderedWindow,
    pub order: Vec<SeatId>,
    /// Methuselahs who passed with no attempt pending: final (rules-qa 1.3).
    pub declined: Vec<SeatId>,
    /// Minions woken for this action.
    pub woken: Vec<MinionId>,
    /// Consign to Oblivion [OBL]: reactions replaced at the end of the action.
    pub defer_reaction_replacement: bool,
    /// Mirror Walk [THA]: if blocked, the blocker locks and the action
    /// ends before block resolution.
    pub end_on_block: bool,
    /// Archon Investigation: the action is not successful.
    pub failed: bool,
    /// Set when the action ends.
    pub outcome: Option<Outcome>,
    /// The minion whose block resolved (for "who has blocked").
    pub blocker: Option<MinionId>,
    /// Form of Mist [PRO]: after combat, burn `blood` to continue with
    /// +`stealth` as if unblocked. Lost if the combat continues.
    pub continuation: Option<(u8, u8)>,
    /// Torrent [CEL]: the action continues as if unblocked as the
    /// after-combat window closes, mandatory and free (rules-qa 63.6).
    pub continue_free: bool,
    /// The continuation was taken in the after-combat window.
    pub continued: bool,
    /// Card names played per minion this action (`Limit::PerAction`).
    pub played: Vec<(MinionId, &'static str)>,
    /// A reaction card was played during this action (canceled or not).
    pub reaction_played: bool,
    /// "Vampires cannot block this action" (Daring the Dawn).
    pub vampires_cannot_block: bool,
    /// "Titled vampires cannot block this action" (Propaganda).
    pub titled_cannot_block: bool,
    /// Aggravated damage the acting vampire takes after resolution, if the
    /// action reaches it (Daring the Dawn).
    pub damage_after: u8,
    /// Pool its Methuselah gains after resolution if it is successful (I
    /// am Legion).
    pub pool_if_successful: u8,
    /// "During this action, minions cannot unlock" (The Sleeping Mind
    /// [DOM]).
    pub no_unlock: bool,
    /// Minions that cannot play reaction cards this action (Unleashing the
    /// Bestial Soul).
    pub no_reactions: Vec<MinionId>,
    /// The next reaction card played this action costs this much more
    /// blood or life (Unleashing the Bestial Soul [DOM]).
    pub reaction_surcharge: u8,
    /// The blocked action ends before block resolution (Change of Target):
    /// no other card is played, no combat.
    pub ended_early: bool,
    /// Ended without resolution (The Kiss of Ra): `Effect::CancelAction`.
    pub canceled: bool,
    /// Controllers of the minions this directed action burned, outside
    /// combat (Predator's Transformation [ABO]).
    pub burned_owners: Vec<SeatId>,
    /// The blocker was unlocked when it blocked (Change of Target leaves it
    /// so: played before the blocker is locked, LSJ 19980224).
    pub blocker_was_unlocked: bool,
    /// The rescue's blood cost, set as it resolves (2, less discounts).
    pub rescue_cost: u8,
    /// The part of the rescue's cost the acting vampire pays, chosen as the
    /// action is announced (rules-qa 48.8); the rescued vampire the rest.
    pub rescue_split: Option<u8>,
    /// Blood the rescued vampire gains if this rescue succeeds (Saulot's
    /// Healing Touch).
    pub rescue_gain: u8,
    /// The political action's referendum passed, by this margin.
    pub referendum_passed: bool,
    pub margin: u32,
    /// Bleed reductions (Party Out Of Bounds [obf], Protected District).
    pub bleed_reduce: u8,
    /// The action card was returned to hand (Delaying Tactics): not
    /// discarded at the end of the action.
    pub card_returned: bool,
    /// The action card entered play at resolution (not discarded after).
    pub card_entered_play: bool,
    /// Taken off the action card's cost at resolution (Szlachta
    /// Assistant: "by 2 blood or pool").
    pub cost_reduction: u8,
    /// Blood-only reductions of the action card's cost (The Line).
    pub blood_reduction: u8,
    /// Counters of cards in play paying the action card's cost, spent as it
    /// is paid (Ravnos Carnival, Ravnos Cache).
    pub counter_payments: Vec<(CardId, u8)>,
    /// The vampire whose blood pays 1 of the action card's cost as it is
    /// paid (Regina Blake).
    pub blood_payer: Option<MinionId>,
    /// An equip action from another minion: that minion, and the equipment
    /// announced (rulebook "Equip").
    pub giver: Option<MinionId>,
    pub taken: Vec<CardId>,
    /// In-play abilities used this action, by copy ("during an action":
    /// once per action, rules-qa 9.6).
    pub abilities_used: Vec<(cards::AbilitySource, u8)>,
    /// Intercept gained by a minion, from any source, for the rest of the
    /// action: a failed attempt keeps it for a new one (rules-qa 15.2).
    pub intercept_given: Vec<(MinionId, i8)>,
    /// Minions that do not lock for their block (Read the Winds [ani]).
    pub no_lock_blockers: Vec<MinionId>,
    /// Minions barred from "strike: combat ends" for the rest of the
    /// action (Mind of the Wilds' blocker).
    pub no_combat_ends: Vec<MinionId>,
    /// "If this vampire blocks" riders of reactions played this action,
    /// given again to each block (LSJ 20010814-2).
    pub block_riders: Vec<(MinionId, cards::BlockRider)>,
    /// Minions that cannot attempt to block this action again (an attempt
    /// made to fail: Forced Confessional [DOM]).
    pub barred_blockers: Vec<MinionId>,
    /// Methuselahs none of whose minions may attempt to block the action
    /// again (Crocodile's Tongue).
    pub barred_seats: Vec<SeatId>,
    /// Where the Veil Thins [OBL]: an attempt costs 1 blood without
    /// Oblivion.
    pub veil_thins: bool,
    /// Blood (or an ally's life) every minion burns to attempt to block
    /// this action (Unthinkable Humiliation).
    pub block_toll_extra: u8,
    /// The acting Methuselah's prey burned pool during this action
    /// (Věnceslava): any pool they lost, costs paid included (rules-qa 52.4).
    pub prey_burned: bool,
    /// Blood only every minion burns to attempt to block this action
    /// (Seeds of Terror): an ally cannot pay it.
    pub block_toll_blood: u8,
    /// A combat queued by a play (Hedonism [for]/[pre]): (its vampire, the
    /// minion that attempted to block) — begun after block resolution (the
    /// block's combat) or after a successful action's effects, still part
    /// of its resolution; dropped if the action ends first (advanced rules
    /// 4.9.3; rules-qa 53.3).
    pub queued_combat: Option<(MinionId, MinionId)>,
    /// Forward Momentum's counter after the bleed (+1 successful, -1
    /// otherwise), pending in the after-resolution window — the same window
    /// as its unlock, in the controller's order (owner, 2026-10-08:
    /// rules-qa 53.6): a gain taken by the unlock first, a loss after it;
    /// whatever is left applies as the action ends.
    pub momentum: Vec<(CardId, bool)>,
    /// The queued combat began after the action's effects: its end goes on
    /// to the after-resolution window (rules-qa 53.3).
    pub queued_at_resolution: bool,
    /// Cards put on their vampire after resolution if the bleed was
    /// successful (Fever Pitch [FOR]/[PRE]).
    pub put_if_bleed_successful: Vec<crate::combat::Held>,
    /// Enthrall [OBF]/[PRE]: after a successful bleed, its vampire may burn
    /// this blood to draw a crypt card (`Choice::DrawCrypt`).
    pub crypt_for_blood: Option<(MinionId, u8)>,
    /// Intercept given minions of a sect for this action ("Anarchs get -1
    /// intercept during this action", Fiendish Tongue).
    pub sect_intercept: Vec<(crate::crypt::Sect, i8)>,
    /// Ensnare a Beast [ANI]/[CEL]: (its vampire, n) — if it blocks, the
    /// acting minion's strike cards cost +n in the resulting combat.
    pub strike_toll_if_blocks: Vec<(MinionId, u8)>,
    /// The minion the granting card was on as it was burned to declare
    /// the action (Hunting the Quarry: "burn this card to attempt to enter
    /// combat with the attached minion").
    pub burned_source_host: Option<MinionId>,
    /// The political action's cost paid before its resolution (Yawp Court:
    /// "the action cost, if any, is paid before Yawp Court gets locked",
    /// LSJ 20080201).
    pub cost_paid_early: bool,
    /// Gifts From Hereafter [DOM]/[OBL]: (its vampire, n) — after
    /// resolution, that vampire may burn n blood to unlock the acting ally.
    pub unlock_paid_by: Vec<(MinionId, u8)>,
    /// Faceless Night [OBF]: failed attempts from now on lock their minion
    /// before resolution.
    pub lock_failed: bool,
    pub failed_blockers: Vec<MinionId>,
    /// Each failed attempt's situation (`attempt_print`): the same minion
    /// attempts again only once it changed (rules-qa 48.6).
    pub failed_attempts: Vec<(MinionId, u64)>,
    /// Library cards a forced attempt costs, paid once the card forcing it
    /// is replaced (Parijat, ANK 20260317).
    pub library_toll: Vec<(SeatId, u8)>,
    /// Every minion whose block of this action succeeded ("if they did not
    /// block this action").
    pub blocked_by: Vec<MinionId>,
    /// Minions that may burn blood for intercept this action (Eyes of the
    /// Wild).
    pub burn_for_intercept: Vec<MinionId>,
    /// Vampires that "can burn 1 blood to get +1 intercept during that
    /// action", once (The Rumor Mill, Tabloid Newspaper).
    pub burn_for_intercept_once: Vec<MinionId>,
    /// WMRH Talk Radio's intercept: the minion given it, the blocks before
    /// (not counted, ANK 20230319), and the Methuselah burning 1 pool after
    /// resolution if it did not block.
    pub pool_if_no_block: Vec<(MinionId, usize, SeatId)>,
    /// The combatants of the last combat of the action (Monster).
    pub last_combat: Option<[MinionId; 2]>,
    /// The Disciplines each minion used to play cards during the action,
    /// canceled cards included (Dabbler; ANK 20220411).
    pub disciplines_played: Vec<(MinionId, &'static str)>,
    /// Minions locked after the action unless they blocked it (Eyes of the
    /// Wild).
    pub lock_if_no_block: Vec<MinionId>,
    /// A hunt that gained blood: "successfully hunts" (The Anarch Free
    /// Press; LSJ 20050720-2).
    pub hunt_gained: bool,
    /// A granted action that is a hunt (Week of Nightmares).
    pub is_hunt: bool,
    /// A mandatory action (a must-hunt vampire's hunt, a printed mandatory
    /// action): not one Khabar: Glory counts.
    pub mandatory: bool,
    /// "After action resolution, if that action was successful, unlock the
    /// acting Nosferatu" (Warsaw Station): applied after the resolution and
    /// any referendum, the clan read then (LSJ 20090216), with or without
    /// the card in play (ANK 20250120).
    pub unlock_if_successful: Option<&'static str>,
    /// Minions whose block may still unlock them for blood after its
    /// combat (Truth in Darkness [OBL]), with the blood.
    pub after_block_unlocks: Vec<(MinionId, u8)>,
    /// The acting minion's unlock for blood after a successful action
    /// (Paths in Two Worlds [OBL]).
    pub after_resolution_unlocks: Vec<(MinionId, u8)>,
    /// "If this action is blocked, all damage inflicted on vampires during
    /// the resulting combat is aggravated" (Dawn Operation).
    pub dawn_operation: bool,
    /// The slave that took the blocked action's combat instead (rulebook
    /// "Slave"): it fights the blocker, the acting vampire unlocked; the
    /// canceled combat's riders are dead (advanced rules §5.7.2).
    pub slave: Option<MinionId>,
    /// Cards put on their minion after resolution if it did not block
    /// (Dogged Pursuit [ANI]).
    pub put_if_no_block: Vec<crate::combat::Held>,
    /// Cards put on the acting minion if their minion blocks (Melange
    /// [AUS]).
    pub put_on_acting_if_blocks: Vec<crate::combat::Held>,
    /// Blood burned before resolution by minions that did not block after
    /// the card (Forced Awakening): the minion, the blocks already made,
    /// the blood.
    pub burn_if_no_block: Vec<(MinionId, usize, u8)>,
    /// Bonuses whose condition is read live (`Effect::If`).
    pub conditional: Vec<Conditional>,
    /// The ash card and the hand card an exchange action named (Lenelle).
    pub exchange: Option<(u16, u16)>,
    /// The ally in the ash heap named as the action was declared (Hel-Blá).
    pub ash_named: Option<u16>,
    /// The vampire of the uncontrolled region named as the action was
    /// declared, by slot and card (Malgorzata's removal).
    pub uncontrolled_named: Option<(u8, u16)>,
    /// The action resolved into a combat (entering combat as a Ⓓ action):
    /// its after-combat window leads to after resolution (rules-qa 17.3).
    pub resolved: bool,
    /// The bleed was successful: it resolved for 1 pool or more (rulebook
    /// "Bleed"; a bleed reduced to 0 is not, advanced rules §3.7.1.2).
    pub bleed_successful: bool,
    /// Enchanting Gaze [PRE][PRO] uses left: the acting Methuselah can burn
    /// 1 of their corruption counters from the minion attempting to block
    /// to fail its attempt for good.
    pub corruption_fails: u8,
    /// Revelation of the Serpent [PRE][PRO]: after a successful bleed, the
    /// acting vampire can unlock by burning this many of its Methuselah's
    /// corruption counters from a minion of the bled Methuselah (0: none).
    pub corruption_unlock: u8,
}

impl ActionCtx {
    /// What an attempt by `m` would face, in public terms: the stealth,
    /// `m`'s intercept, blood and lock, the cards in play, every hand's size
    /// and its Methuselah's library and ash heap sizes (a card played or
    /// drawn changes them; the hand itself is hidden, and what the engine
    /// keeps must not depend on it: `tests/elision.rs`). The same again
    /// after a failed attempt, a new one would fail alike (rules-qa 48.6).
    pub fn attempt_print(&self, st: &crate::state::GameState, m: MinionId) -> u64 {
        use std::hash::{Hash, Hasher};
        let seat = &st.seats[st.minions[m.index()].owner.index()];
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (
            self.stealth(st),
            self.intercept_of(st, m),
            st.minions[m.index()].blood,
            st.minions[m.index()].locked,
            st.cards.iter().filter(|c| c.present()).count(),
            st.seats.iter().map(|s| s.hand.len()).collect::<Vec<_>>(),
            seat.library.len(),
            seat.ash.len(),
        )
            .hash(&mut h);
        h.finish()
    }

    /// Did `m` already fail an attempt in this very situation?
    pub fn failed_alike(&self, st: &crate::state::GameState, m: MinionId) -> bool {
        let print = self.attempt_print(st, m);
        self.failed_attempts.contains(&(m, print))
    }

    /// The action's stealth, the acting minion's cards in play included
    /// (Unlicensed Taxicab, Codex of the Edenic Groundskeepers). Not
    /// floored: stealth may be negative (LSJ 20021211, "a minion with zero
    /// or negative stealth"; rules-qa 48.1).
    pub fn stealth(&self, st: &crate::state::GameState) -> i32 {
        // A granted action's stealth for a clan, read live on the acting
        // minion (Wasserschloss Anif: "Malkavians get +1 stealth during
        // that action").
        // An action card's too (Baltimore Purge: "if this vampire is Ravnos,
        // this is a +1 stealth action"); a titled vampire's (Powerbase:
        // Barranquilla).
        let mode = match (self.card, self.source, self.ability) {
            (Some(p), _, _) => cards::lookup(p.card).and_then(|d| d.modes.get(p.mode as usize)),
            (None, Some(src), Some(ab)) => Some(src.mode(st, ab)),
            _ => None,
        };
        let for_clan: i32 = mode.map_or(0, |m| {
            m.effects
                .iter()
                .map(|e| match *e {
                    Effect::StealthForClan { clan, n }
                        if st.minions[self.acting.index()].clan() == Some(clan) =>
                    {
                        n as i32
                    }
                    Effect::StealthForTitled(n)
                        if st.title_of(self.acting) != crate::crypt::Title::None =>
                    {
                        n as i32
                    }
                    _ => 0,
                })
                .sum()
        });
        self.base_stealth as i32
            + for_clan
            + self.stealth_mods as i32
            + st.bearer_mod(self.acting, Stat::Stealth, self.kind)
            + st.printed_mod(self.acting, Stat::Stealth, self.printed_ctx(st, None))
            - self.stealth_reduce as i32
    }

    /// The bleed amount, the acting minion's cards in play included
    /// (Treasured Samadji), after reductions.
    pub fn bleed_amount(&self, st: &crate::state::GameState) -> u8 {
        // The granting card's own bleed condition, live (Codex of the Edenic
        // Groundskeepers: "+3 bleed if the target Methuselah controls no
        // ready unlocked minions").
        let granted: i32 = match (self.source, self.ability, self.bleed_target) {
            (Some(src), Some(ab), Some(t)) => src
                .mode(st, ab)
                .effects
                .iter()
                .map(|e| match e {
                    Effect::BleedIfTargetHasNoReadyUnlocked(n)
                        if st.ready_unlocked(t).is_empty() =>
                    {
                        *n as i32
                    }
                    // The printed "+1 bleed" of the granted bleed (Elen Kamjian).
                    Effect::BleedMod { n, .. } => *n as i32,
                    _ => 0,
                })
                .sum(),
            _ => 0,
        };
        // A bleed action card's own "+1 bleed" (Entrancement [pre]).
        let card_bleed: i32 = match (self.kind, self.card) {
            (ActionKind::Bleed, Some(p)) => cards::lookup(p.card).map_or(0, |d| {
                d.modes[p.mode as usize]
                    .effects
                    .iter()
                    .map(|e| match e {
                        Effect::BleedMod { n, .. } => *n as i32,
                        _ => 0,
                    })
                    .sum()
            }),
            _ => 0,
        };
        let granted = granted + card_bleed;
        // "No other equipment can increase the bleed amount of this
        // action" (Camera Phone).
        let alone = match (self.source, self.ability) {
            (Some(src), Some(ab)) => src
                .mode(st, ab)
                .effects
                .contains(&Effect::NoOtherEquipmentBleed),
            _ => false,
        };
        let bearer = if alone {
            st.bearer_mod_without_equipment_increase(self.acting, Stat::Bleed, self.kind)
        } else {
            st.bearer_mod(self.acting, Stat::Bleed, self.kind)
        };
        let against = self
            .bleed_target
            .map_or(0, |t| st.bleed_against_mod(self.acting, t));
        let n = (st.base_bleed(self.acting) as i32
            + against
            + self.bleed_mods as i32
            + self.conditional_bonus(st, cards::Stat::Bleed, None)
            + bearer
            + st.printed_mod(self.acting, Stat::Bleed, self.printed_ctx(st, None))
            + granted)
            .max(0) as u8;
        n.saturating_sub(self.bleed_reduce)
    }

    /// Effective intercept of minion `m`'s attempt: the accumulated
    /// intercept folded through the action's intercept layers (Perfect
    /// Paragon [PRE]: younger vampires and allies get -1). Not floored: a
    /// 0-intercept blocker given -1 (Form of the Bat [pro]) cannot block
    /// a 0-stealth action (rules-qa 48.1).
    pub fn intercept_of(&self, st: &crate::state::GameState, m: MinionId) -> i32 {
        // Unleash Hell's Fury: "+2 intercept during that attempt".
        let fury = if matches!(
            st.minions[m.index()].kind,
            crate::state::MinionKind::Fury { .. }
        ) {
            2
        } else {
            0
        };
        fury + Self::intercept_mods(st, m)
            + self.host_intercept(st, m)
            + self.given_intercept(m)
            + self
                .sect_intercept
                .iter()
                .filter(|&&(sect, _)| st.sect_of(m) == Some(sect))
                .map(|&(_, n)| n as i32)
                .sum::<i32>()
            + self.conditional_bonus(st, cards::Stat::Intercept, Some(m))
    }

    /// Is the one limited bleed increase used: by an unconditional one, or
    /// a conditional one whose condition holds now (a bonus lost to a
    /// change of target no longer counts, ANK 20211019-2)?
    pub fn limited_used(&self, st: &crate::state::GameState) -> bool {
        self.limited_bleed_used
            || self
                .conditional
                .iter()
                .any(|c| c.limited && self.condition_holds(st, c.seat, c.on, c.requires))
    }

    /// Do `requires` hold of minion `on` now, for `seat`'s card?
    pub fn condition_holds(
        &self,
        st: &crate::state::GameState,
        seat: SeatId,
        on: MinionId,
        requires: &[cards::Requirement],
    ) -> bool {
        requires
            .iter()
            .all(|&r| cards::holds_for_minion(r, st, seat, None, Some(self.acting), on, Some(self)))
    }

    /// The conditional bonuses to `stat` (of minion `to`, if given) whose
    /// condition holds now.
    pub fn conditional_bonus(
        &self,
        st: &crate::state::GameState,
        stat: cards::Stat,
        to: Option<MinionId>,
    ) -> i32 {
        self.conditional
            .iter()
            .filter(|c| c.stat == stat && to.is_none_or(|t| c.to == t))
            .filter(|c| self.condition_holds(st, c.seat, c.on, c.requires))
            .map(|c| c.n as i32)
            .sum()
    }

    /// Intercept given to `m` for the rest of the action.
    pub fn given_intercept(&self, m: MinionId) -> i32 {
        self.intercept_given
            .iter()
            .filter(|(g, _)| *g == m)
            .map(|(_, n)| *n as i32)
            .sum()
    }

    /// Intercept from cards in play keyed to the action: on `m` (Guardian
    /// Angel: +1 during bleed actions directed at its controller; Sport
    /// Bike), and on the acting minion against `m` (Stolen Police Cruiser:
    /// younger vampires get -1 intercept against its bearer).
    pub fn host_intercept(&self, st: &crate::state::GameState, m: MinionId) -> i32 {
        let owner = st.minions[m.index()].owner;
        let bleed_at_owner = self.kind == ActionKind::Bleed && self.bleed_target == Some(owner);
        let younger = st.minions[m.index()].is_ally() || st.younger_than(m, self.acting);
        // Directed at /Mithras/ or a Mithraic Mysteries card or member
        // (Mithraic Cultist).
        let at_mithraic = match self.target {
            Target::Minion(t) => st.is_mithraic(t),
            Target::Card(c) => {
                let name = crate::library::LIBRARY[st.cards[c.index()].card as usize].name;
                name == "Mithraeum, London, The" || name == "Mithraic Cultist"
            }
            _ => false,
        } && self.directed_seat(st).is_some();
        let own: i32 = st
            .borne(m)
            .filter_map(|l| match l.modifier {
                cards::Modifier::HostInterceptOnBleedAtYou(n) if bleed_at_owner => Some(n as i32),
                cards::Modifier::HostInterceptDirectedAtYou(n)
                    if self.directed_seat(st) == Some(owner) =>
                {
                    Some(n as i32)
                }
                cards::Modifier::HostInterceptVsMithraic(n) if at_mithraic => Some(n as i32),
                cards::Modifier::HostInterceptVsVampires(n)
                    if st.minions[self.acting.index()].is_vampire() =>
                {
                    Some(n as i32)
                }
                _ => None,
            })
            .sum();
        let against: i32 = st
            .borne(self.acting)
            .filter_map(|l| match l.modifier {
                cards::Modifier::YoungerInterceptAgainstBearer { n, who }
                    if younger && who.is(st, self.acting) =>
                {
                    Some(n as i32)
                }
                _ => None,
            })
            .sum();
        let printed = st.printed_mod(m, Stat::Intercept, self.printed_ctx(st, Some(self.acting)));
        // The Unmasking: allies against a vampire, a Gehenna-immune one
        // included (LSJ 20081120-2).
        let unmasking: i32 =
            if st.minions[m.index()].is_ally() && st.minions[self.acting.index()].is_vampire() {
                st.in_play_layers()
                    .filter_map(|l| match l.modifier {
                        cards::Modifier::AlliesInterceptVsVampires(n) => Some(n as i32),
                        _ => None,
                    })
                    .sum()
            } else {
                0
            };
        own + st.bearer_mod(m, Stat::Intercept, self.kind) + against + printed + unmasking
    }

    /// "A successful action requiring an Anarch or making this vampire
    /// Anarch" (CrimethInc.): the rulebook action to become Anarch (rules-qa
    /// 58.1); an action card requiring an Anarch or a baron (LSJ 20080603)
    /// or making a vampire Anarch, a recruit, employ or equip of a card
    /// requiring one included (owner, rules-qa 62.21) — not a card another
    /// card fetches or plays as part of the action, whose requirements are
    /// not the action's (LSJ 20100725, ANK 20180817); an action a card in
    /// play grants Anarchs only (Open War).
    pub fn requires_anarch(&self, st: &crate::state::GameState) -> bool {
        use crate::library::Label;
        if self.kind == ActionKind::BecomeAnarch {
            return true;
        }
        if let Some(p) = self.card {
            let labels = LIBRARY[p.card as usize].labels;
            return labels.contains(&Label::RequiresAnarch) || labels.contains(&Label::MakesAnarch);
        }
        if let (Some(src), Some(ab)) = (self.source, self.ability) {
            return src.mode(st, ab).requires.iter().any(|r| {
                matches!(r, cards::Requirement::ActorSectIn(s) if s.contains(&crate::crypt::Sect::Anarch))
            });
        }
        false
    }

    /// The minion the action targets, if any (a torpid vampire rescued or
    /// diablerised, a granted action's minion target).
    pub fn target_minion(&self) -> Option<MinionId> {
        match (self.kind, self.target) {
            (ActionKind::Rescue | ActionKind::Diablerie, _) => self.torpor_target,
            (_, Target::Minion(m)) => Some(m),
            _ => None,
        }
    }

    /// The minion the action's resolution begins a combat with ("enter
    /// combat with a minion", a card's or a granted action's), if any.
    pub fn combat_target(&self, st: &crate::state::GameState) -> Option<MinionId> {
        let (effects, target) = match (&self.card, self.source, self.ability) {
            (Some(p), _, _) => (
                cards::lookup(p.card).map_or(&[][..], |d| d.modes[p.mode as usize].effects),
                p.target,
            ),
            (None, Some(src), Some(ab)) => (src.mode(st, ab).effects, self.target),
            _ => return None,
        };
        match target {
            Target::Minion(m) if effects.contains(&Effect::EnterCombatWithTarget) => Some(m),
            _ => None,
        }
    }

    /// The Methuselah a directed action is directed at, if any.
    pub fn directed_seat(&self, st: &crate::state::GameState) -> Option<SeatId> {
        match self.kind {
            ActionKind::Bleed => self.bleed_target,
            ActionKind::Rescue | ActionKind::Diablerie => {
                let t = self.torpor_target.expect("torpor target");
                let owner = st.minions[t.index()].owner;
                (owner != self.seat).then_some(owner)
            }
            ActionKind::BurnCard | ActionKind::StealCard => {
                let controller = st.card_controller(self.source.expect("source").card());
                (controller != self.seat).then_some(controller)
            }
            ActionKind::Ability | ActionKind::Card => match self.target {
                Target::Minion(m) => {
                    let controller = st.minions[m.index()].owner;
                    (controller != self.seat).then_some(controller)
                }
                Target::Seat(s) => (s != self.seat).then_some(s),
                Target::Card(c) => {
                    let controller = st.card_controller(c);
                    (controller != self.seat).then_some(controller)
                }
                Target::None | Target::Uncontrolled(_) => None,
            },
            _ => None,
        }
    }

    /// The Methuselahs the action is directed at (at most one yet: every
    /// directed action here names a single Methuselah).
    pub fn directed_seats(&self, st: &crate::state::GameState) -> crate::ids::SeatSet {
        self.directed_seat(st)
            .map_or(crate::ids::SeatSet::EMPTY, crate::ids::SeatSet::single)
    }

    /// What a printed modifier reads of this action (`GameState::printed_mod`).
    pub fn printed_ctx(
        &self,
        st: &crate::state::GameState,
        opponent: Option<MinionId>,
    ) -> crate::state::PrintedCtx {
        crate::state::PrintedCtx {
            kind: Some(self.kind),
            directed: Some(self.directed_seat(st).is_some()),
            opponent,
            card: None,
            target: self.directed_seat(st),
            reacting: false,
        }
    }

    /// Sum of the action's intercept layers applying to `m`.
    pub fn intercept_mods(st: &crate::state::GameState, m: MinionId) -> i32 {
        st.layers
            .iter()
            .filter(|l| st.layer_active(l))
            .filter_map(|l| match l.modifier {
                cards::Modifier::InterceptMod { younger_than, n }
                    if st.ally_or_younger(m, younger_than) =>
                {
                    Some(n as i32)
                }
                cards::Modifier::CorruptedInterceptMod(n)
                    if !st.minions[m.index()].corruption.is_empty() =>
                {
                    Some(n as i32)
                }
                _ => None,
            })
            .sum()
    }

    /// The effective intercept of the current attempt, if any.
    pub fn attempt_intercept(&self, st: &crate::state::GameState) -> i32 {
        match self.state {
            ActionState::Attempt(m) => self.intercept_of(st, m),
            _ => 0,
        }
    }

    /// The minion the action is directed at, if any: the vampire in
    /// torpor rescued or diablerised, or a granted action's minion target.
    /// A card on a minion targets that card only, not its host (rulebook
    /// "Target").
    pub fn directed_minion(&self) -> Option<MinionId> {
        match (self.torpor_target, self.target) {
            (Some(t), _) | (None, Target::Minion(t)) => Some(t),
            _ => None,
        }
    }

    /// The pending block attempt fails, and its minion cannot attempt to
    /// block this action again (Forced Confessional, Enchanting Gaze).
    fn attempt_fails(&mut self) {
        if let ActionState::Attempt(m) = self.state {
            self.barred_blockers.push(m);
            if self.lock_failed {
                self.failed_blockers.push(m);
            }
            self.state = ActionState::Open;
            self.fresh_window();
        }
    }

    fn fresh_window(&mut self) {
        self.window = OrderedWindow::new(self.order.clone());
    }
}

/// The stealth an action card, a granted action or an equipment prints ("+1
/// stealth action"); the rulebook's +1 applies to political action cards and
/// the undirected rulebook actions (rules-qa 6.2).
pub(crate) fn printed_stealth(effects: &[Effect]) -> u8 {
    effects
        .iter()
        .map(|e| match e {
            Effect::AddStealth(n) => *n,
            _ => 0,
        })
        .sum()
}

impl Engine {
    /// Announce an action (rulebook "Detailed action flow"): lock the acting
    /// minion, compute base stealth and the block-eligible seats, open the
    /// as-announced window. Card actions arrive with their play context.
    pub(crate) fn declare_action(
        &mut self,
        minion: MinionId,
        kind: ActionKind,
        torpor_target: Option<MinionId>,
        card: Option<PlayCtx>,
        source: Option<cards::AbilitySource>,
    ) {
        self.state.minions[minion.index()].locked = true;
        self.state.action_serial += 1;
        if kind == ActionKind::Hunt {
            self.state.minions[minion.index()].hunted = true;
        }
        self.state.phase_actions = self.state.phase_actions.saturating_add(1);
        // The acting minion's controller: the active Methuselah, or another
        // one's vampire taking an action in their phase (Enkil Cog).
        let seat = self.state.minions[minion.index()].owner;
        let own_target = torpor_target.map(|t| self.state.minions[t.index()].owner == seat);
        let (base_stealth, bleed_target, directed_seat) = match kind {
            ActionKind::Bleed => (0, Some(self.state.prey(seat)), Some(self.state.prey(seat))),
            ActionKind::Hunt
            | ActionKind::LeaveTorpor
            | ActionKind::Employ
            | ActionKind::Recruit
            | ActionKind::EquipFrom
            | ActionKind::BecomeAnarch => (1, None, None),
            // Granted by a card in play: its printed stealth only (rules-qa
            // 6.2), added by `use_ability`.
            ActionKind::EquipFound => (0, None, None),
            ActionKind::Equip => {
                // +1 stealth, and what the equipment prints for the action
                // to equip with it (Unlicensed Taxicab).
                let play = card.expect("equipment card");
                let def = cards::lookup(play.card).expect("definition");
                (
                    1 + printed_stealth(def.modes[play.mode as usize].effects),
                    None,
                    None,
                )
            }
            ActionKind::Political => match card {
                Some(_) => (1, None, None),
                None => (0, None, None),
            },
            ActionKind::Card => {
                // Directed at its target's controller, unless that is the
                // acting Methuselah (Ⓓ is a reminder; rules-qa 17.4).
                let play = card.expect("action card");
                let def = cards::lookup(play.card).expect("definition");
                let controller = match play.target {
                    Target::Minion(m) => Some(self.state.minions[m.index()].owner),
                    Target::Card(c) => Some(self.state.card_controller(c)),
                    Target::Seat(s) => Some(s),
                    _ => None,
                };
                (
                    printed_stealth(def.modes[play.mode as usize].effects),
                    None,
                    controller.filter(|&c| c != seat),
                )
            }
            ActionKind::BurnCard | ActionKind::StealCard => {
                // Directed at the card's controller; one's own card makes
                // it undirected. No default stealth for an action granted
                // by a card in play — only what its text prints (rules-qa
                // 6.2).
                let controller = self.state.card_controller(source.expect("source").card());
                (0, None, (controller != seat).then_some(controller))
            }
            // A granted action with its effects at resolution: its printed
            // stealth only, added by `use_ability`, which sets the target
            // (and so the directed seat) before the as-announced window.
            ActionKind::Ability => (0, None, None),
            ActionKind::Rescue | ActionKind::Diablerie => {
                let t = torpor_target.expect("torpor target");
                if own_target == Some(true) {
                    (1, None, None)
                } else {
                    (0, None, Some(self.state.minions[t.index()].owner))
                }
            }
        };
        let order = self.action_order(seat, directed_seat);
        self.action = Some(ActionCtx {
            seat,
            acting: minion,
            kind,
            card,
            source,
            // An action card's target (Entrancement's ally); a granted
            // action's is set by `use_ability`.
            target: match (kind, card) {
                (ActionKind::Card, Some(p)) => p.target,
                _ => Target::None,
            },
            torpor_target,
            bleed_target,
            base_stealth,
            state: ActionState::AsAnnounced,
            window: OrderedWindow::new(order.clone()),
            order,
            titled_cannot_block: card.is_some_and(|p| {
                cards::lookup(p.card).is_some_and(|d| {
                    d.modes[p.mode as usize]
                        .effects
                        .contains(&Effect::TitledCannotBlock)
                })
            }),
            rescue_cost: 2,
            // An action card's own "allies cannot block this action"
            // (Slaughtering the Herd [dom]).
            allies_cannot_block: card.is_some_and(|p| {
                cards::lookup(p.card).is_some_and(|d| {
                    d.modes[p.mode as usize]
                        .effects
                        .contains(&Effect::AlliesCannotBlock)
                })
            }),
            // "If this action is blocked" riders of the action card (Show
            // of Force), the acting minion's in the block's combat.
            block_riders: card
                .and_then(|p| cards::lookup(p.card).map(|d| d.modes[p.mode as usize].effects))
                .unwrap_or(&[])
                .iter()
                .filter_map(|e| match e {
                    Effect::IfBlocks(r) => Some((minion, *r)),
                    _ => None,
                })
                .collect(),
            // The action card's own Disciplines, played by the acting
            // minion.
            disciplines_played: card
                .map(|p| {
                    crate::library::LIBRARY[p.card as usize].modes
                        [cards::library_mode(p.card, p.mode as usize)]
                    .tags
                    .iter()
                    .map(|t| (minion, cards::discipline_key(t)))
                    .collect()
                })
                .unwrap_or_default(),
            // An action card's hunt (Psychophagia).
            is_hunt: card.is_some_and(|p| {
                cards::lookup(p.card).is_some_and(|d| {
                    d.modes[p.mode as usize]
                        .effects
                        .contains(&Effect::HuntAction)
                })
            }),
            mandatory: self.state.minions[minion.index()].must_hunt
                && (kind == ActionKind::Hunt
                    || card.is_some_and(|p| {
                        cards::lookup(p.card).is_some_and(|d| {
                            d.modes[p.mode as usize]
                                .effects
                                .contains(&Effect::HuntAction)
                        })
                    })),
            // "Anarchs get -1 intercept during this action" (Fiendish
            // Tongue).
            sect_intercept: card
                .and_then(|p| cards::lookup(p.card).map(|d| d.modes[p.mode as usize].effects))
                .unwrap_or(&[])
                .iter()
                .filter_map(|e| match *e {
                    Effect::SectInterceptThisAction(sect, n) => Some((sect, n)),
                    _ => None,
                })
                .collect(),
            ..Default::default()
        });
        // The reductions announced with the card: a lock is made now —
        // once the card is no longer cancelable as played (The Line: not
        // locked if the card is canceled, ANK 20230620).
        if let Some(play) = card {
            self.apply_announced_discounts(&play);
        }
        // Veil the Legions [OBF]: one of the next X actions this turn gets
        // +1 stealth, counted as it is announced (a later cancel does not
        // give it back: LSJ 20070526).
        let seat_state = &mut self.state.seats[seat.index()];
        if seat_state.veil_left > 0 {
            seat_state.veil_left -= 1;
            self.action.as_mut().expect("action").stealth_mods += 1;
        }
        self.flow = Flow::Action;
        self.host_announces_riders(minion);
        // Who pays the rescue's blood: chosen as it is announced, paid as
        // it resolves (rules-qa 48.8, 3.6).
        if kind == ActionKind::Rescue {
            self.flow = Flow::RescueCost;
        }
    }

    /// "Each time the attached vampire announces an action, they move 1
    /// blood from themselves to this acting vampire" (Slaughtering the Herd
    /// [DOM]): riders of the cards on the acting minion.
    fn host_announces_riders(&mut self, acting: MinionId) {
        for (id, r) in self.state.card_riders(Hook::HostAnnouncesAction) {
            let c = &self.state.cards[id.index()];
            if !c.present() || c.host != Some(acting) {
                continue;
            }
            let by = c.by;

            for e in r.effects {
                match *e {
                    Effect::HostBloodToBy(n) => {
                        let Some(to) = by.filter(|b| self.state.minions[b.index()].present())
                        else {
                            continue;
                        };
                        let moved = n.min(self.state.minions[acting.index()].blood);
                        self.state.minions[acting.index()].blood -= moved;
                        self.state.gain_blood(to, moved);
                    }
                    other => unreachable!("not an announce rider effect: {other:?}"),
                }
            }
        }
    }

    /// Sequencing order during an action: the acting Methuselah, then the
    /// target of a directed action, then the rest clockwise.
    pub(crate) fn action_order(&self, seat: SeatId, directed: Option<SeatId>) -> Vec<SeatId> {
        let mut order = vec![seat];
        if let Some(t) = directed {
            if t != seat {
                order.push(t);
            }
        }
        for s in self.state.clockwise_from(seat) {
            if !order.contains(&s) {
                order.push(s);
            }
        }
        order
    }

    /// Seats entitled to attempt a block (rulebook "Who May Attempt to
    /// Block"): the target of a directed action; prey and predator of an
    /// undirected one.
    /// A Methuselah controlling a vampire that "may attempt to block a
    /// vampire controlled by another Methuselah, ignoring the normal prey,
    /// predator or target restrictions" (Anneke) is one too, for that
    /// vampire only (`block_candidates_locked`).
    fn eligible_blockers(&self, a: &ActionCtx) -> Vec<SeatId> {
        let mut v = self.normal_blockers(a);
        if self.state.minions[a.acting.index()].is_vampire() {
            for i in 0..self.state.seats.len() {
                let s = SeatId(i as u8);
                if s != a.seat
                    && !v.contains(&s)
                    && self.state.minions_of(s).any(|m| self.blocks_anyone(m))
                {
                    v.push(s);
                }
            }
        }
        v
    }

    /// The seats the rulebook lets block: the target of a directed action;
    /// prey and predator of an undirected one.
    fn normal_blockers(&self, a: &ActionCtx) -> Vec<SeatId> {
        match self.directed_target(a) {
            Some(t) => vec![t],
            None => {
                let mut v = vec![self.state.prey(a.seat), self.state.predator(a.seat)];
                v.dedup();
                v.retain(|s| *s != a.seat);
                v
            }
        }
    }

    /// Does `m` print "may attempt to block … ignoring the normal prey,
    /// predator or target restrictions" (Anneke)?
    fn blocks_anyone(&self, m: MinionId) -> bool {
        self.state.printed_mod(
            m,
            cards::Stat::BlocksAnyVampire,
            crate::state::PrintedCtx::default(),
        ) > 0
    }

    /// The Methuselah a directed action is directed at, if any.
    fn directed_target(&self, a: &ActionCtx) -> Option<SeatId> {
        a.directed_seat(&self.state)
    }

    /// Minions of `seat` that may attempt to block: ready and unlocked, or
    /// woken for this action.
    fn block_candidates(&self, a: &ActionCtx, seat: SeatId) -> Vec<MinionId> {
        self.block_candidates_locked(a, seat, false)
            .into_iter()
            .filter(|&m| self.may_block_from(a, seat, m))
            .collect()
    }

    /// `m` of an eligible `seat` may block by the who-may-block rule: any
    /// of a seat the rulebook lets block; of another, a vampire printing
    /// "may attempt to block … ignoring the normal prey, predator or target
    /// restrictions" (Anneke) — her other restrictions standing (RTR
    /// 20020501).
    fn may_block_from(&self, a: &ActionCtx, seat: SeatId, m: MinionId) -> bool {
        self.normal_blockers(a).contains(&seat)
            || self.state.minions[a.acting.index()].is_vampire() && self.blocks_anyone(m)
    }

    /// Could a Methuselah other than the acting one reduce the action's
    /// stealth to 0 with a card one of their reacting minions could play
    /// (Night Terrors), as `seat` weighs an attempt? Their own hand is
    /// read; another's is hidden from them, so for another only whether
    /// they hold a card and one of their minions could play such a card of
    /// the pool: the attempts offered must not depend on a hand the
    /// decider cannot see (DESIGN §4.3; `tests/elision.rs` found this).
    fn stealth_strippable(&self, a: &ActionCtx, seat: SeatId) -> bool {
        static STRIPS: std::sync::OnceLock<Vec<u16>> = std::sync::OnceLock::new();
        let strips = STRIPS.get_or_init(|| {
            (0..LIBRARY.len() as u16)
                .filter(|&c| {
                    cards::lookup(c).is_some_and(|d| {
                        d.modes
                            .iter()
                            .any(|m| m.effects.contains(&Effect::ReduceStealthToZero))
                    })
                })
                .collect()
        });
        let st = &self.state;
        let playable = |s: SeatId, card: u16| {
            cards::lookup(card).is_some_and(|def| {
                def.modes.iter().enumerate().any(|(mi, mode)| {
                    mode.effects.contains(&Effect::ReduceStealthToZero)
                        && st.minions_of(s).any(|r| {
                            let v = &st.minions[r.index()];
                            let clans = LIBRARY[card as usize].clans;
                            v.ready()
                                && (!v.locked || a.woken.contains(&r))
                                && (clans.is_empty()
                                    || v.clan().is_some_and(|c| clans.contains(&c)))
                                && cards::has_disciplines(
                                    st,
                                    r,
                                    card,
                                    LIBRARY[card as usize].modes[cards::library_mode(card, mi)]
                                        .tags,
                                )
                        })
                })
            })
        };
        (0..st.seats.len()).map(|i| SeatId(i as u8)).any(|s| {
            let hand = &st.seats[s.index()].hand;
            s != a.seat
                && !st.seats[s.index()].ousted
                && if s == seat {
                    hand.iter().any(|&card| playable(s, card))
                } else {
                    !hand.is_empty() && strips.iter().any(|&card| playable(s, card))
                }
        })
    }

    /// Can `m` of `seat` attempt to block the current action, its lock
    /// aside ("unlocks and attempts to block"): the normal prey, predator
    /// and target restrictions apply (ANK 20180623), and a minion that
    /// cannot block cannot (LSJ 20010714-2), nor one that cannot pay the
    /// attempt's toll; one already attempting may (ANK 20181122-2).
    pub(crate) fn can_attempt_block(&self, seat: SeatId, m: MinionId) -> bool {
        let Some(a) = self.action.as_ref() else {
            return false;
        };
        // Another of the same Methuselah's minions during its attempt: a
        // change of blocker (rules-qa 48.3).
        let switching = match a.state {
            ActionState::Attempt(x) => {
                x != m
                    && self.state.minions[x.index()].owner == seat
                    && a.stealth(&self.state) > a.intercept_of(&self.state, x)
            }
            _ => false,
        };
        match a.state {
            ActionState::Attempt(x) if !switching => x == m,
            ActionState::Open | ActionState::Attempt(_) => {
                !a.failed
                    && self.eligible_blockers(a).contains(&seat)
                    && !a.declined.contains(&seat)
                    // An unlock-and-block card needs the unlock (The
                    // Sleeping Mind [DOM] bars it).
                    && self.block_candidates_locked(a, seat, !a.no_unlock).contains(&m)
                    && self.may_block_from(a, seat, m)
                    // The attempt's toll payable (Sergio Bueno, Mithras).
                    && self.block_toll(a, m).is_some()
            }
            _ => false,
        }
    }

    /// Could `m`, of `seat`, attempt to block the action now, whoever may
    /// normally block it (Eagle's Sight [AUS])? Not the acting
    /// Methuselah's own minion; not a Methuselah who declined; a block
    /// candidate, unlocked (no unlock here). One attempt: back in state A
    /// after it fails, its Methuselah is no more eligible than before
    /// (LSJ 20030227).
    pub(crate) fn can_attempt_block_anyone(&self, seat: SeatId, m: MinionId) -> bool {
        let Some(a) = self.action.as_ref() else {
            return false;
        };
        a.state == ActionState::Open
            && !a.failed
            && seat != a.seat
            && !a.declined.contains(&seat)
            && self.block_candidates_locked(a, seat, false).contains(&m)
            && self.block_toll(a, m).is_some()
    }

    /// `to` takes control of `m` until the end of their turn (Puppet Master
    /// [DOM]). A block attempt it was making fails, not barred: its new
    /// controller may attempt again if eligible (LSJ 20080326, PIB
    /// 20150501); a combat it is in ends, damage still pending resolving
    /// first (LSJ 20020618). The cards on it go with it.
    pub(crate) fn take_control(&mut self, m: MinionId, to: SeatId) {
        let from = self.state.minions[m.index()].owner;
        if from == to {
            return;
        }
        if self
            .action
            .as_ref()
            .is_some_and(|a| a.state == ActionState::Attempt(m))
        {
            self.resign_attempt();
        }
        if self
            .combat
            .as_ref()
            .is_some_and(|c| c.side_of_minion(m).is_some())
        {
            let pending = self.combat.as_ref().is_some_and(|c| {
                c.sides
                    .iter()
                    .any(|s| s.pending.total() > 0 || s.unpreventable > 0)
            });
            // Damage still pending resolves first; the combat may end with
            // it (a combatant burned, a Methuselah ousted).
            if !(pending && self.apply_damage()) && self.combat.is_some() {
                self.end_combat_now();
            }
        }
        // A forced self-contest: the incoming copy burns (advanced rules
        // 1.13.1).
        let name = self.state.minions[m.index()].name();
        let unique = self.state.minions[m.index()]
            .vampire()
            .is_some_and(|c| c.unique);
        let dup = unique
            && self
                .state
                .minions
                .iter()
                .enumerate()
                .any(|(j, o)| j != m.index() && o.owner == to && o.in_play && o.name() == name);
        if dup {
            self.burn_minion(m);
            return;
        }
        let v = &mut self.state.minions[m.index()];
        v.owner = to;
        v.temp_from = Some(v.temp_from.unwrap_or(from));
    }

    /// `seat`'s corruption counters on `t` equal or exceed "their capacity
    /// or cost" (Cave of Apples, Contagion): a vampire's capacity, an
    /// ally's printed cost (rules-qa 35.2).
    pub(crate) fn corruption_steal_reached(&self, seat: SeatId, t: MinionId) -> bool {
        let v = &self.state.minions[t.index()];
        let need = match v.ally_id() {
            Some(id) => {
                let card = self.state.cards[id.index()].card;
                let lib = &LIBRARY[card as usize];
                match (lib.pool_cost, lib.blood_cost) {
                    (crate::library::Cost::Fixed(n), _) | (_, crate::library::Cost::Fixed(n)) => n,
                    _ => 0,
                }
            }
            None => self.state.capacity_of(t).unwrap_or(u8::MAX),
        };
        v.corruption_of(seat) >= need
    }

    /// What `m` pays to attempt to block this action, if it can: blood
    /// without Oblivion under Where the Veil Thins (none for an ally: it
    /// cannot), a library card against a wraith or zombie ally while a
    /// Parijat is ready (none from an empty library: ANK 20260317), blood
    /// or life printed on the acting vampire (Sergio Bueno).
    pub(crate) fn block_toll(&self, a: &ActionCtx, m: MinionId) -> Option<(u8, u8)> {
        let st = &self.state;
        let v = &st.minions[m.index()];
        let obl = st.has_discipline(m, "obl");
        let veil = u8::from(a.veil_thins && !obl);
        // A printed toll on the acting vampire (Sergio Bueno), in torpor too
        // (LSJ 20011022): an ally pays it in life, its last one too — it
        // burns and the attempt fails (rules-qa 41.4).
        let printed = st
            .printed_mod(
                m,
                cards::Stat::BlockToll,
                crate::state::PrintedCtx {
                    opponent: Some(a.acting),
                    ..Default::default()
                },
            )
            .max(0) as u8;
        let blood = veil + printed + a.block_toll_extra + a.block_toll_blood;
        if (veil > 0 || a.block_toll_blood > 0) && v.is_ally() || v.blood < blood {
            return None;
        }
        let wraith_or_zombie = st.minions[a.acting.index()]
            .kind
            .ally_card()
            .is_some_and(|c| {
                let labels = LIBRARY[c as usize].labels;
                labels.contains(&crate::library::Label::Wraith)
                    || labels.contains(&crate::library::Label::Zombie)
            });
        let parijat = wraith_or_zombie
            && (0..st.minions.len()).map(|i| MinionId(i as u32)).any(|p| {
                st.minions[p.index()].ready()
                    && st.printed_mod(
                        p,
                        cards::Stat::WraithZombieBlockToll,
                        crate::state::PrintedCtx::default(),
                    ) > 0
            });
        let cards = u8::from(parijat);
        if st.seats[v.owner.index()].library.len() < cards as usize {
            return None;
        }
        // Archon: burned as far as it goes, the attempt allowed anyway.
        let archon = if v.is_vampire() {
            st.blockers_of_burn(a.acting)
        } else {
            0
        };
        Some((blood + archon, cards))
    }

    /// Pay `m`'s block toll: the blood now, the library cards now or (a
    /// forced attempt) once its card is replaced.
    pub(crate) fn pay_block_toll(&mut self, m: MinionId, deferred: bool) {
        let a = self.action.as_ref().expect("action context");
        let (blood, cards) = self.block_toll(a, m).expect("an affordable toll");
        let owner = self.state.minions[m.index()].owner;
        let v = &mut self.state.minions[m.index()];
        v.blood = v.blood.saturating_sub(blood);
        // An ally paying its last life burns: its attempt fails (rules-qa
        // 41.4).
        if blood > 0 && v.is_ally() && v.blood == 0 {
            self.burn_minion(m);
            if self.action.as_ref().map(|a| a.state) == Some(ActionState::Attempt(m)) {
                self.resign_attempt();
            }
            return;
        }
        if deferred {
            if cards > 0 {
                self.action
                    .as_mut()
                    .expect("action")
                    .library_toll
                    .push((owner, cards));
            }
        } else {
            self.burn_library_top(owner, cards);
        }
    }

    /// The attempting minion resigns its attempt: back to A, where it may
    /// attempt again (LSJ 20031121-2).
    /// A card action's cost after its reductions: the "blood or pool" ones
    /// off whichever the card costs, the blood-only ones, then counters
    /// (Ravnos Carnival: blood; Ravnos Cache: either) — with what each
    /// counter source covers.
    pub(crate) fn reduced_cost_with(
        &self,
        a: &ActionCtx,
        pool: i32,
        blood: u8,
    ) -> (i32, u8, Vec<(CardId, u8)>) {
        let (mut pool, mut blood) = (pool, blood);
        crate::plays::reduce(&mut pool, &mut blood, a.cost_reduction);
        blood = blood.saturating_sub(a.blood_reduction);
        let mut used = Vec::new();
        for &(c, x) in &a.counter_payments {
            let avail = x.min(self.state.cards[c.index()].counters);
            let n = match self.state.cost_source_of(c) {
                Some(cards::CostSource::Carnival(_)) => avail.min(blood),
                Some(cards::CostSource::Cache | cards::CostSource::Hackerspace) if pool > 0 => {
                    avail.min(pool as u8)
                }
                Some(cards::CostSource::Cache | cards::CostSource::Hackerspace) => avail.min(blood),
                _ => 0,
            };
            match self.state.cost_source_of(c) {
                Some(cards::CostSource::Cache | cards::CostSource::Hackerspace) if pool > 0 => {
                    pool -= n as i32
                }
                _ => blood -= n,
            }
            used.push((c, n));
        }
        (pool, blood, used)
    }

    /// A vampire's blood paying 1 of a `(pool, blood)` cost (Regina Blake),
    /// if it still has one and something is left to pay: the cost less it,
    /// and the payer, whose blood the caller burns as the cost is paid.
    pub(crate) fn blood_payment(
        &self,
        payer: Option<MinionId>,
        pool: &mut i32,
        blood: &mut u8,
    ) -> Option<MinionId> {
        let r = payer.filter(|r| {
            let v = &self.state.minions[r.index()];
            v.present() && v.blood > 0
        })?;
        if *pool > 0 {
            *pool -= 1;
        } else if *blood > 0 {
            *blood -= 1;
        } else {
            return None;
        }
        Some(r)
    }

    /// Spend the counters paying a cost (`reduced_cost_with`'s usage); a
    /// Ravnos Carnival with none left is burned.
    pub(crate) fn spend_counters(&mut self, used: &[(CardId, u8)]) {
        for &(c, n) in used {
            let card = &mut self.state.cards[c.index()];
            card.counters -= n.min(card.counters);
            if card.counters == 0
                && matches!(
                    self.state.cost_source_of(c),
                    Some(cards::CostSource::Carnival(_))
                )
            {
                self.burn_card_in_play(c);
            }
        }
    }

    /// The cost reductions announced with an action card: The Line and The
    /// Shard locked now (still unlocked, the Edge still held), the
    /// counters recorded to be spent as the cost is paid.
    pub(crate) fn apply_announced_discounts(&mut self, play: &PlayCtx) {
        let (either, blood, counters) = self.take_discounts(play);
        let a = self.action.as_mut().expect("action");
        a.cost_reduction += either;
        a.blood_reduction += blood;
        a.counter_payments.extend(counters);
        a.blood_payer = play.blood_payer;
    }

    /// The reductions picked for `ctx`'s card, taken: The Line, The Shard
    /// (the Edge still held) and Ravnos Cache locked, still unlocked. What
    /// they take off — "blood or pool", blood only — and the counters to
    /// spend as the cost is paid.
    pub(crate) fn take_discounts(&mut self, ctx: &PlayCtx) -> (u8, u8, Vec<(CardId, u8)>) {
        let (mut either, mut blood, mut counters) = (0, 0, Vec::new());
        let mut burned = Vec::new();
        for (c, n) in ctx.discounts.into_iter().flatten() {
            let card = &self.state.cards[c.index()];
            if !card.present() {
                continue;
            }
            let lock = match self.state.cost_source_of(c) {
                Some(cards::CostSource::Line) if !card.locked => {
                    blood += n;
                    true
                }
                Some(cards::CostSource::Shard)
                    if !card.locked && self.state.seats[ctx.seat.index()].has_edge =>
                {
                    either += n;
                    true
                }
                Some(cards::CostSource::Cache) if !card.locked => {
                    counters.push((c, n));
                    true
                }
                Some(cards::CostSource::Carnival(_) | cards::CostSource::Hackerspace) => {
                    counters.push((c, n));
                    false
                }
                // Focus the Blood: burned as the cost is paid.
                Some(cards::CostSource::FocusBlood) => {
                    blood += n;
                    burned.push(c);
                    false
                }
                _ => false,
            };
            self.state.cards[c.index()].locked |= lock;
        }
        for c in burned {
            self.burn_card_in_play(c);
        }
        (either, blood, counters)
    }

    pub(crate) fn resign_attempt(&mut self) {
        let a = self.action.as_mut().expect("action context");
        a.state = ActionState::Open;
        a.fresh_window();
    }

    /// Burn the top `n` cards of `s`'s library.
    pub(crate) fn burn_library_top(&mut self, s: SeatId, n: u8) {
        for _ in 0..n {
            if let Some(c) = self.state.seats[s.index()].library.pop() {
                self.state.seats[s.index()].ash.push(c);
            }
        }
    }

    /// As `block_candidates`, locked minions included if `ignore_lock`.
    fn block_candidates_locked(
        &self,
        a: &ActionCtx,
        seat: SeatId,
        ignore_lock: bool,
    ) -> Vec<MinionId> {
        let st = &self.state;
        let vampire_acting = !st.minions[a.acting.index()].is_ally();
        // "Allies cannot block this action / this Anarch" (Visions of
        // Gehenna, Invigorate [dom], Libertas).
        let no_allies = a.allies_cannot_block || st.allies_cannot_block(a.acting);
        // "Vampires with capacity 3 or less cannot block Rexton".
        let small = st
            .printed_mod(
                a.acting,
                cards::Stat::SmallVampiresCannotBlock,
                crate::state::PrintedCtx::default(),
            )
            .max(0);
        st.minions_of(seat)
            .chain(self.fury_tokens(a, seat))
            .filter(|m| {
                let v = &st.minions[m.index()];
                !v.in_torpor
                    && !v.expelled
                    && (ignore_lock || !v.locked || a.woken.contains(m))
                    && !a.barred_blockers.contains(m)
                    && !a.barred_seats.contains(&v.owner)
                    && self.block_toll(a, *m).is_some()
                    && !(vampire_acting && st.cannot_block_vampires(*m))
                    && !st.shuns(*m, a.acting)
                    && !(v.is_vampire()
                        && st
                            .big_cannot_block(a.acting)
                            .is_some_and(|n| st.capacity_of(*m).is_some_and(|c| c >= n)))
                    && !st.cannot_block(*m)
                    && !st.unblockable(a.acting, a.kind)
                    && !(no_allies && v.is_ally())
                    && !(small > 0
                        && v.is_vampire()
                        && st.capacity_of(*m).is_some_and(|c| (c as i32) <= small))
                    && !(a.vampires_cannot_block && !v.is_ally())
                    && !(a.titled_cannot_block
                        && !v.is_ally()
                        && st.title_of(*m) != crate::crypt::Title::None)
            })
            .collect()
    }

    /// Unleash Hell's Fury of `seat`'s, in play, against an action directed
    /// at them: its token, out of play until it attempts (the same minion
    /// for every attempt, LSJ 20100210).
    fn fury_tokens(&self, a: &ActionCtx, seat: SeatId) -> Vec<MinionId> {
        let st = &self.state;
        if a.directed_seat(st) != Some(seat) {
            return Vec::new();
        }
        (0..st.minions.len())
            .map(|i| MinionId(i as u32))
            .filter(|&m| {
                let v = &st.minions[m.index()];
                matches!(v.kind, crate::state::MinionKind::Fury { id, .. }
                    if !v.in_play && st.cards[id.index()].present()
                        && st.card_controller(id) == seat)
            })
            .collect()
    }

    /// Intercept reachability (DESIGN §4.3 elision, owner-approved
    /// 2026-09-02, P1 form): an attempt whose intercept can never reach the
    /// action's stealth — nothing in the controller's hand this minion could
    /// play adds enough, and no intercept card to play whose replacement
    /// draw could bring more (rules-qa 48.5) — is pruned rather than
    /// offered. The closure-time
    /// comparison in `action_advance` is the actual semantics.
    fn intercept_reachable(&self, a: &ActionCtx, seat: SeatId, m: MinionId) -> i32 {
        let st = &self.state;
        // A Methuselah other than the acting one who could strip the
        // stealth (Night Terrors), the blocker's own or a third party
        // (rules-qa 1.2): any attempt may still succeed.
        if self.stealth_strippable(a, seat) {
            return i32::MAX;
        }
        // Intercept gained earlier in the action counts (rules-qa 15.2).
        // Conditional intercept counts whether or not its condition holds
        // now: it may by closure (optimistic, never pruning a reachable
        // attempt).
        let conditional: i32 = a
            .conditional
            .iter()
            .filter(|c| c.stat == cards::Stat::Intercept && c.to == m)
            .map(|c| c.n as i32)
            .sum();
        // What the attempt has already (may be negative: Form of the Bat
        // [pro]), then what it could still gain.
        let base = a.host_intercept(st, m) + a.given_intercept(m) + conditional;
        let mut total = self.printed_intercept_reach(seat, m);
        for &card in &st.seats[seat.index()].hand {
            let Some(def) = cards::lookup(card) else {
                continue;
            };
            let mut per_card = 0u8;
            for (mi, mode) in def.modes.iter().enumerate() {
                let tags = LIBRARY[card as usize].modes[cards::library_mode(card, mi)].tags;
                if !cards::has_disciplines(st, m, card, tags) {
                    continue;
                }
                if mode.requires.contains(&cards::Requirement::DirectedAtYou)
                    && a.directed_seat(st) != Some(seat)
                {
                    continue;
                }
                // One play per card: the best mode of each card adds, its
                // conditional intercept counted (optimistic).
                let best = mode
                    .effects
                    .iter()
                    .map(|e| match e {
                        Effect::AddIntercept(n)
                        | Effect::If {
                            then: Effect::AddIntercept(n),
                            ..
                        } => *n,
                        // Given to the attempting minion (Organized
                        // Resistance: a baron's +1 to an attempting Anarch,
                        // itself included).
                        Effect::AttemptingIntercept(n) if *n > 0 => *n as u8,
                        _ => 0,
                    })
                    .sum();
                per_card = per_card.max(best);
            }
            total = total.saturating_add(per_card);
            // Each intercept card played is replaced from the library, and
            // the draw may be another (rules-qa 48.5): with one to play and
            // a library to draw from, any stealth may yet be met.
            if per_card > 0 && !st.seats[seat.index()].library.is_empty() {
                return i32::MAX;
            }
        }
        // Unused intercept abilities of the cards on `m` (Bowl of
        // Convergence [AUS]).
        for (ci, c) in st.cards.iter().enumerate() {
            if !c.present() || c.host != Some(m) {
                continue;
            }
            let Some(def) = cards::lookup(c.card) else {
                continue;
            };
            for (ai, ab) in def
                .abilities
                .iter()
                .enumerate()
                .filter(|(_, ab)| ab.windows.contains(&Window::ActionStealthIntercept))
            {
                if ab.limit == Some(Limit::PerAction)
                    && a.abilities_used.contains(
                        &cards::AbilitySource::Card(CardId(ci as u32)).action_key(ai as u8),
                    )
                {
                    continue;
                }
                if !ab.requires.iter().all(|&r| cards::holds_for_host(r, st, m)) {
                    continue;
                }
                for e in ab.effects {
                    match *e {
                        Effect::AddIntercept(n) => total = total.saturating_add(n),
                        // Dogged Pursuit's burn for +1 intercept.
                        Effect::AttemptingIntercept(n) if n > 0 => {
                            total = total.saturating_add(n as u8)
                        }
                        _ => {}
                    }
                }
            }
        }
        total = total.saturating_add(self.location_intercept_reach(seat, m));
        base + total as i32 + ActionCtx::intercept_mods(st, m)
    }

    /// An upper bound on the intercept unlocked cards in play (not on `m`)
    /// can give `m`'s attempt: its own Methuselah's (Market Square, Channel
    /// 10) or another's, paying for it (KRCG News Radio) — third-party help
    /// counted, never pruning an attempt it could make succeed (DESIGN
    /// §11, rules-qa 1.3).
    fn location_intercept_reach(&self, seat: SeatId, m: MinionId) -> u8 {
        let st = &self.state;
        let v = &st.minions[m.index()];
        let mut total = 0u8;
        for (ci, c) in st.cards.iter().enumerate() {
            let id = CardId(ci as u32);
            if !c.present() || c.locked || c.host == Some(m) {
                continue;
            }
            let Some(def) = cards::lookup(c.card) else {
                continue;
            };
            let controller = st.card_controller(id);
            if st.seats[controller.index()].ousted {
                continue;
            }
            let best = def
                .abilities
                .iter()
                .filter(|ab| {
                    ab.actor == cards::Actor::Controller
                        && ab.windows.contains(&Window::ActionStealthIntercept)
                        && ab.requires.iter().all(|&r| match r {
                            cards::Requirement::YourAttemptNeedsIntercept => controller == seat,
                            cards::Requirement::OthersAttemptNeedsIntercept => {
                                controller != seat
                                    && st.seats[controller.index()].pool
                                        > ab.effects
                                            .iter()
                                            .map(|e| match e {
                                                Effect::PayPool(n) => *n as i32,
                                                _ => 0,
                                            })
                                            .sum::<i32>()
                            }
                            cards::Requirement::AttemptingClan(k) => v.clan() == Some(k),
                            cards::Requirement::AttemptingCapacityAtLeast(n) => {
                                st.capacity_of(m).is_some_and(|c| c >= n)
                            }
                            cards::Requirement::AttemptingSectIn(s) => {
                                st.sect_of(m).is_some_and(|x| s.contains(&x))
                            }
                            cards::Requirement::NotFirstActionOfPhase => st.phase_actions >= 2,
                            _ => true,
                        })
                })
                .map(|ab| {
                    ab.effects
                        .iter()
                        .map(|e| match e {
                            Effect::AttemptingIntercept(n) if *n > 0 => *n as u8,
                            _ => 0,
                        })
                        .sum::<u8>()
                })
                // The Rumor Mill: its controller's vampire with blood may be
                // chosen to buy 1 intercept.
                .chain(def.abilities.iter().filter_map(|ab| {
                    (ab.effects.contains(&Effect::TargetBurnsBloodForIntercept)
                        && controller == seat
                        && v.blood > 0
                        && v.is_vampire())
                    .then_some(1)
                }))
                .max()
                .unwrap_or(0);
            total = total.saturating_add(best);
        }
        total
    }

    /// An upper bound on the intercept `m` can get from abilities printed on
    /// `seat`'s vampires: a use per blood burned (Oskar Anasov) or card
    /// discarded (Abraham DuSable, on his own attempt), once per action for
    /// "during an action" (rules-qa 15.2).
    fn printed_intercept_reach(&self, seat: SeatId, m: MinionId) -> u8 {
        let st = &self.state;
        let mut total = 0u8;
        for v in st.minions_of(seat) {
            let Some(def) = cards::vampire_of(&st.minions[v.index()]) else {
                continue;
            };
            for (ai, ab) in def
                .abilities
                .iter()
                .enumerate()
                .filter(|(_, ab)| ab.windows.contains(&Window::ActionStealthIntercept))
            {
                let per_use: u8 = ab
                    .effects
                    .iter()
                    .map(|e| match e {
                        Effect::AttemptingIntercept(n) if *n > 0 => *n as u8,
                        Effect::AddIntercept(n) if v == m => *n,
                        _ => 0,
                    })
                    .sum();
                if per_use == 0 {
                    continue;
                }
                let uses = match ab.terms {
                    Some(cards::Terms::Discard { n, of }) => {
                        (st.hand_picks(seat, of).len() / n.max(1) as usize).min(u8::MAX as usize)
                            as u8
                    }
                    _ if ab
                        .effects
                        .iter()
                        .any(|e| matches!(e, Effect::HostBurnBlood(_))) =>
                    {
                        st.minions[v.index()].blood
                    }
                    _ => 1,
                };
                let uses = match ab.limit {
                    Some(Limit::PerAction) => {
                        let used = self.action.as_ref().is_some_and(|a| {
                            a.abilities_used
                                .contains(&cards::AbilitySource::Vampire(v).action_key(ai as u8))
                        });
                        if used {
                            0
                        } else {
                            uses.min(1)
                        }
                    }
                    _ => uses,
                };
                total = total.saturating_add(per_use.saturating_mul(uses));
            }
        }
        total
    }

    /// Windows open for `seat` in the action's current state.
    fn action_windows(a: &ActionCtx, seat: SeatId) -> Vec<Window> {
        // Ended by Change of Target or Mirror Walk: nothing more is played
        // (LSJ 20010803-1, RTR 20180719), though its after-resolution
        // window still opens for the effects due there (Daring the Dawn).
        if a.ended_early || (a.end_on_block && a.outcome == Some(Outcome::Blocked)) {
            return Vec::new();
        }
        let acting = seat == a.seat;
        match a.state {
            ActionState::AsAnnounced => {
                if acting {
                    vec![Window::ActionAsAnnounced]
                } else {
                    vec![Window::ReactionAsAnnounced]
                }
            }
            ActionState::Open | ActionState::Attempt(_) => {
                let attempt = matches!(a.state, ActionState::Attempt(_));
                if acting && attempt {
                    vec![
                        Window::ActionDuring,
                        Window::ActionStealthIntercept,
                        Window::ActionBlockAttempt,
                    ]
                } else if acting {
                    vec![Window::ActionDuring, Window::ActionStealthIntercept]
                } else {
                    vec![
                        Window::ReactionDuringAction,
                        Window::ActionStealthIntercept,
                        Window::Wake,
                        Window::ReactionUnlock,
                        Window::ReactionUnlockAttempt,
                    ]
                }
            }
            ActionState::Declined => {
                if acting {
                    vec![Window::ActionDuring]
                } else {
                    vec![
                        Window::ReactionDuringAction,
                        Window::ActionBlocksDeclined,
                        Window::Wake,
                        Window::ReactionUnlock,
                    ]
                }
            }
            ActionState::WouldSucceed if !a.failed => vec![Window::ActionWouldBeSuccessful],
            ActionState::WouldSucceed => vec![],
            ActionState::AfterCombat => vec![Window::ActionAfterCombat],
            ActionState::AfterResolution => vec![Window::ActionAfterResolution],
            // The acting Methuselah's "if blocked" cards (Change of Target)
            // and the blocker's "about to enter combat" ones (Obedience).
            // Continued as if unblocked (Crypt's Sons): no combat to react
            // to (ANK 20210131).
            ActionState::Blocked(_) if a.continued => vec![],
            ActionState::Blocked(_) => vec![Window::ActionBlocked],
        }
    }

    pub(crate) fn action_request(&self) -> Option<DecisionRequest> {
        let a = self.action.as_ref().expect("action context");
        if self.acting_gone(a) {
            return None;
        }
        let seat = a.window.current()?;
        let mut options = vec![Choice::Pass];
        // A failed action cannot be blocked any more (it ends as the
        // blocks close).
        if a.state == ActionState::Open
            && !a.failed
            && self.eligible_blockers(a).contains(&seat)
            && !a.declined.contains(&seat)
        {
            for m in self.block_candidates(a, seat) {
                if self.intercept_reachable(a, seat, m) >= a.stealth(&self.state)
                    && !a.failed_alike(&self.state, m)
                {
                    options.push(Choice::DeclareBlock { minion: m });
                }
            }
        }
        // A change of blocker: during its own minion's attempt, left short
        // of the stealth, its Methuselah may attempt with another instead —
        // the attempt under way fails then (rules-qa 48.3).
        if let ActionState::Attempt(x) = a.state {
            if self.state.minions[x.index()].owner == seat
                && a.stealth(&self.state) > a.intercept_of(&self.state, x)
                && !a.failed
                && !a.declined.contains(&seat)
                && self.eligible_blockers(a).contains(&seat)
            {
                for m in self.block_candidates(a, seat) {
                    if m != x
                        && self.intercept_reachable(a, seat, m) >= a.stealth(&self.state)
                        && !a.failed_alike(&self.state, m)
                    {
                        options.push(Choice::DeclareBlock { minion: m });
                    }
                }
            }
        }
        // Only after a block's combat: an unblocked action has nothing to
        // continue (rules-qa 17.3).
        if a.state == ActionState::AfterCombat && seat == a.seat && !a.resolved {
            if let Some((blood, _)) = a.continuation {
                let m = &self.state.minions[a.acting.index()];
                // A directed action whose target Methuselah left the game
                // mid-combat (DESIGN §10.8) has nothing to continue against.
                let target_gone = self
                    .directed_target(a)
                    .is_some_and(|t| self.state.seats[t.index()].ousted);
                if m.ready() && m.blood >= blood && !target_gone {
                    options.push(Choice::ContinueAction);
                }
            }
        }
        // Eyes of the Wild: burn 1 blood for +1 intercept, while its
        // attempt needs intercept, as often as it pays; The Rumor Mill's
        // chosen vampire, once.
        if let ActionState::Attempt(m) = a.state {
            let v = &self.state.minions[m.index()];
            if v.owner == seat
                && v.blood > 0
                && (a.burn_for_intercept.contains(&m) || a.burn_for_intercept_once.contains(&m))
                && a.stealth(&self.state) > a.intercept_of(&self.state, m)
            {
                options.push(Choice::BurnBloodForIntercept { minion: m });
            }
        }
        // Enchanting Gaze [PRE][PRO]: the acting Methuselah's counter on the
        // minion attempting to block.
        if let ActionState::Attempt(m) = a.state {
            if seat == a.seat
                && a.corruption_fails > 0
                && self.state.minions[m.index()].corruption_of(seat) > 0
            {
                options.push(Choice::BurnCorruptionToFail { minion: m });
            }
        }
        // Revelation of the Serpent [PRE][PRO]: after a successful bleed, the
        // acting vampire unlocks for counters on a minion of the bled
        // Methuselah — none left if it was ousted (ANK 20220210). An
        // unlocked vampire can unlock, to no effect (owner, 2026-09-26:
        // rules-qa 25.5), as can one under "cannot unlock".
        if a.state == ActionState::AfterResolution
            && seat == a.seat
            && a.corruption_unlock > 0
            && a.bleed_successful
            && self.state.minions[a.acting.index()].present()
        {
            let prey = a.bleed_target.expect("bleed target");
            for m in self.state.minions_of(prey) {
                if self.state.minions[m.index()].corruption_of(seat) >= a.corruption_unlock {
                    options.push(Choice::BurnCorruptionToUnlock { minion: m });
                }
            }
        }
        // Truth in Darkness [OBL]: the blocker's unlock for blood, after
        // block resolution.
        // Paths in Two Worlds [OBL]: after a successful action.
        if a.state == ActionState::AfterResolution
            && matches!(a.outcome, Some(Outcome::Successful | Outcome::Fizzled))
        {
            for &(m, n) in &a.after_resolution_unlocks {
                let v = &self.state.minions[m.index()];
                if v.owner == seat && v.locked && v.ready() && v.blood >= n {
                    options.push(Choice::UnlockForBlood { minion: m });
                }
            }
        }
        // Gifts From Hereafter [DOM]/[OBL]: its vampire's blood for the
        // acting ally's unlock (rules-qa 55.5).
        if a.state == ActionState::AfterResolution
            && matches!(a.outcome, Some(Outcome::Successful | Outcome::Fizzled))
        {
            let acting = &self.state.minions[a.acting.index()];
            if acting.present() && acting.locked {
                for &(payer, n) in &a.unlock_paid_by {
                    let p = &self.state.minions[payer.index()];
                    if p.owner == seat && p.present() && p.blood >= n {
                        options.push(Choice::UnlockForBlood { minion: a.acting });
                        break;
                    }
                }
            }
        }
        // Enthrall [OBF]/[PRE]: "this vampire can burn 1 blood to draw 1
        // card from your crypt" — from an empty one too, drawing nothing
        // (LSJ 20060620).
        if a.state == ActionState::AfterResolution && a.bleed_successful && seat == a.seat {
            if let Some((m, n)) = a.crypt_for_blood {
                let v = &self.state.minions[m.index()];
                if v.present() && v.blood >= n {
                    options.push(Choice::DrawCrypt);
                }
            }
        }
        if a.state == ActionState::AfterCombat {
            for &(m, n) in &a.after_block_unlocks {
                let v = &self.state.minions[m.index()];
                if v.owner == seat && v.locked && v.ready() && v.blood >= n {
                    options.push(Choice::UnlockForBlood { minion: m });
                }
            }
        }
        // Damage after resolution (Daring the Dawn): the acting Methuselah
        // orders it among the after-resolution effects, and resolves it
        // before passing (owner, 2026-09-26: rules-qa 23.3).
        let damage_pending = a.state == ActionState::AfterResolution
            && seat == a.seat
            && a.damage_after > 0
            && a.outcome.is_some();
        if damage_pending {
            options.retain(|o| *o != Choice::Pass);
            options.push(Choice::TakeDamageAfterResolution);
        }
        let windows = Self::action_windows(a, seat);
        options.extend(self.impulse_options(seat, &windows));
        let window = match a.state {
            ActionState::AsAnnounced => WindowLabel::ActionAsAnnounced,
            ActionState::Open | ActionState::Attempt(_) => WindowLabel::ActionBlock,
            ActionState::Declined => WindowLabel::ActionBlocksDeclined,
            ActionState::WouldSucceed => WindowLabel::ActionWouldBeSuccessful,
            ActionState::AfterCombat => WindowLabel::ActionAfterCombat,
            ActionState::AfterResolution => WindowLabel::ActionAfterResolution,
            ActionState::Blocked(_) => WindowLabel::ActionBlocked,
        };
        Some(DecisionRequest::new(seat, window, options))
    }

    pub(crate) fn action_apply(&mut self, choice: &Choice) {
        let eligible = {
            let a = self.action.as_ref().expect("action context");
            self.eligible_blockers(a)
        };
        let a = self.action.as_mut().expect("action context");
        let seat = a.window.current().expect("impulse holder");
        match choice {
            Choice::Pass => {
                if a.state == ActionState::Open
                    && eligible.contains(&seat)
                    && !a.declined.contains(&seat)
                {
                    // rules-qa 1.3: passing with no attempt pending is a final
                    // decline (rulebook A.3).
                    a.declined.push(seat);
                }
                a.window.pass();
                self.flow = Flow::Action;
            }
            Choice::DeclareBlock { minion } => {
                // Another blocker instead (rules-qa 48.3): the attempt under
                // way fails, as at its closure: not offered again alike
                // (rules-qa 48.6).
                if let ActionState::Attempt(x) = a.state {
                    if a.lock_failed {
                        a.failed_blockers.push(x);
                    }
                    let print = a.attempt_print(&self.state, x);
                    a.failed_attempts.push((x, print));
                }
                a.state = ActionState::Attempt(*minion);
                a.window.play();
                self.flow = Flow::Action;
                // Unleash Hell's Fury: a vampire for its attempt.
                if matches!(
                    self.state.minions[minion.index()].kind,
                    crate::state::MinionKind::Fury { .. }
                ) {
                    self.state.minions[minion.index()].in_play = true;
                }
                self.pay_block_toll(*minion, false);
            }
            Choice::BurnBloodForIntercept { minion } => {
                if !a.burn_for_intercept.contains(minion) {
                    a.burn_for_intercept_once.retain(|m| m != minion);
                }
                a.window.play();
                a.intercept_given.push((*minion, 1));
                self.state.minions[minion.index()].blood -= 1;
                self.flow = Flow::Action;
            }
            Choice::TakeDamageAfterResolution => {
                let (m, n) = (a.acting, std::mem::take(&mut a.damage_after));
                a.window.play();
                if self.state.minions[m.index()].present() {
                    self.damage_out_of_combat(m, n, true);
                }
                self.flow = Flow::Action;
            }
            Choice::DrawCrypt => {
                let (m, n) = a.crypt_for_blood.take().expect("a crypt draw for blood");
                a.window.play();
                self.state.burn_blood(m, n);
                let seat = &mut self.state.seats[seat.index()];
                if let Some(card) = seat.crypt.pop() {
                    seat.uncontrolled.push(crate::state::UncontrolledSlot {
                        card,
                        counters: 0,
                        banished: None,
                    });
                }
                self.flow = Flow::Action;
            }
            Choice::UnlockForBlood { minion }
                if *minion == a.acting
                    && a.state == ActionState::AfterResolution
                    && !a.unlock_paid_by.is_empty()
                    && !a
                        .after_resolution_unlocks
                        .iter()
                        .any(|&(m, _)| m == *minion) =>
            {
                let (payer, n) = a.unlock_paid_by.remove(0);
                a.unlock_paid_by.clear();
                a.window.play();
                let barred = a.no_unlock;
                self.state.minions[payer.index()].blood -= n;
                if !barred {
                    self.state.unlock(*minion);
                }
                self.flow = Flow::Action;
            }
            Choice::UnlockForBlood { minion } => {
                let list = if a.state == ActionState::AfterResolution {
                    &mut a.after_resolution_unlocks
                } else {
                    &mut a.after_block_unlocks
                };
                let i = list
                    .iter()
                    .position(|&(m, _)| m == *minion)
                    .expect("an unlock for blood");
                let (m, n) = list.remove(i);
                a.window.play();
                let barred = a.no_unlock;
                self.state.minions[m.index()].blood -= n;
                if !barred {
                    self.state.unlock(m);
                }
                self.flow = Flow::Action;
            }
            Choice::BurnCorruptionToFail { minion } => {
                a.corruption_fails -= 1;
                self.state.minions[minion.index()].burn_corruption(seat, 1);
                a.attempt_fails();
                self.flow = Flow::Action;
            }
            Choice::BurnCorruptionToUnlock { minion } => {
                let n = std::mem::take(&mut a.corruption_unlock);
                a.window.play();
                self.state.minions[minion.index()].burn_corruption(seat, n);
                if !a.no_unlock {
                    self.state.unlock(a.acting);
                }
                self.flow = Flow::Action;
            }
            Choice::ContinueAction => {
                // Form of Mist [PRO]: the rider's blood is a burn, not a card
                // cost (rulings); +1 stealth "even if not yet needed" is a
                // plain bonus here, no play to gate.
                let (blood, stealth) = a.continuation.take().expect("continuation");
                a.stealth_mods += stealth;
                a.continued = true;
                a.window.play();
                self.state.minions[a.acting.index()].blood -= blood;
                self.flow = Flow::Action;
            }
            Choice::Play {
                card,
                mode,
                actor,
                target,
            } => {
                a.window.play();
                self.flow = Flow::Action;
                self.announce_play(seat, *card, *mode, *actor, *target, Flow::Action);
            }
            Choice::UseAbility {
                source,
                ability,
                actor,
                target,
            } => {
                // An untimed ability (The Barrens) is an effect: the impulse
                // returns to the acting Methuselah.
                a.window.play();
                self.flow = Flow::Action;
                self.use_ability(*source, *ability, *actor, *target, Flow::Action);
            }
            other => unreachable!("illegal action choice {other:?}"),
        }
    }

    /// The action window closed on consecutive passes: state transition
    /// (rulebook A.4, B.4, C.3; spec §2.2 for the as-announced window).
    /// The acting minion left play before the action resolved (an ally
    /// paying its last life for a modifier, ANK 20180928-1): nothing more
    /// can happen in it.
    fn acting_gone(&self, a: &ActionCtx) -> bool {
        !self.state.minions[a.acting.index()].in_play
            && matches!(
                a.state,
                ActionState::AsAnnounced
                    | ActionState::Open
                    | ActionState::Attempt(_)
                    | ActionState::Declined
                    | ActionState::WouldSucceed
                    | ActionState::Blocked(_)
            )
    }

    pub(crate) fn action_advance(&mut self) {
        // Its acting minion gone, the action fails.
        if self.acting_gone(self.action.as_ref().expect("action context")) {
            self.action.as_mut().expect("action context").outcome = Some(Outcome::Failed);
            self.end_action();
            return;
        }
        if self.action.as_ref().expect("action context").state == ActionState::AsAnnounced
            && self.announced_at_target_riders()
        {
            return;
        }
        // A combat queued in it (Hedonism) begins once the block's own
        // combat is over — block resolution — or the combat the
        // resolution began (owner, 2026-10-08: rules-qa 53.3).
        if self.action.as_ref().expect("action context").state == ActionState::AfterCombat
            && self.begin_queued_combat()
        {
            return;
        }
        let a = self.action.as_mut().expect("action context");
        debug_assert!(a.window.is_closed());
        match a.state {
            ActionState::AsAnnounced => {
                a.state = ActionState::Open;
                a.fresh_window();
                self.flow = Flow::Action;
            }
            ActionState::Open => {
                a.state = ActionState::Declined;
                a.fresh_window();
                self.flow = Flow::Action;
            }
            ActionState::Attempt(blocker) => {
                // Locked during its attempt (Visions of Zapathasura
                // [OBF][PRE]), a minion that did not wake fails it (LSJ
                // 20010702, RTR 19941006).
                // One that left play during it (47th Street Royals burned
                // for its bleed reduction) fails it too.
                let locked_out = !self.state.minions[blocker.index()].present()
                    || self.state.minions[blocker.index()].locked && !a.woken.contains(&blocker);
                if !locked_out && a.intercept_of(&self.state, blocker) >= a.stealth(&self.state) {
                    self.blocked(blocker);
                } else {
                    // The attempt fails: back to A, where the same Methuselah
                    // may attempt again (rulebook B.4, "if one attempt to
                    // block fails, another can be made").
                    if a.lock_failed {
                        a.failed_blockers.push(blocker);
                    }
                    let print = a.attempt_print(&self.state, blocker);
                    a.failed_attempts.push((blocker, print));
                    a.state = ActionState::Open;
                    a.fresh_window();
                    self.flow = Flow::Action;
                }
            }
            // Obedience against the combat the resolution would begin: the
            // action ends unsuccessfully before it resolves, no combat
            // (owner, 2026-10-08: rules-qa 55.7, the window 56.2); it
            // reached resolution, so NRA holds (LSJ 20070709).
            // Canceled before resolution (The Kiss of Ra): no outcome, no
            // NRA mark, no after-resolution window.
            ActionState::Declined if a.canceled => {
                self.end_action();
            }
            ActionState::Declined if a.ended_early => {
                a.outcome = Some(Outcome::Failed);
                let (acting, kind) = (a.acting, a.kind);
                self.mark_resolved(acting, kind);
                self.after_resolution();
            }
            ActionState::Declined | ActionState::WouldSucceed => {
                // A bleed for 1 or more, or a political action before its
                // referendum (Yawp Court: "if a political action is
                // successful, before the referendum").
                let would_succeed = a.state == ActionState::Declined
                    && !a.failed
                    && (a.kind == ActionKind::Bleed
                        && a.bleed_amount(&self.state) > 0
                        && a.bleed_target
                            .is_some_and(|t| !self.state.seats[t.index()].ousted)
                        || a.kind == ActionKind::Political);
                if would_succeed {
                    a.state = ActionState::WouldSucceed;
                    a.fresh_window();
                    self.flow = Flow::Action;
                } else if a.failed {
                    // Failed at resolution (Archon Investigation, Spying
                    // Mission [OBF]): it reached resolution, so NRA holds
                    // (ANK 20211015, RTR 20180719).
                    a.outcome = Some(Outcome::Failed);
                    let (acting, kind) = (a.acting, a.kind);
                    self.mark_resolved(acting, kind);
                    self.after_resolution();
                } else {
                    self.succeed();
                }
            }
            ActionState::AfterCombat => {
                if a.resolved {
                    // A combat the resolution began: the action succeeded.
                    self.after_resolution();
                } else if a.continued || std::mem::take(&mut a.continue_free) {
                    // Continue as if unblocked (rules-qa 3.4): back to A with
                    // the action's context kept — modifiers, reactions and
                    // the declines stand; only the attempt state resets.
                    a.continued = false;
                    a.state = ActionState::Open;
                    a.fresh_window();
                    self.flow = Flow::Action;
                } else {
                    a.outcome = Some(Outcome::Blocked);
                    self.after_resolution();
                }
            }
            // Go-getter [OBF]: a blocked action continued as if unblocked.
            ActionState::AfterResolution if a.continued => {
                a.continued = false;
                a.outcome = None;
                a.state = ActionState::Open;
                a.fresh_window();
                self.flow = Flow::Action;
            }
            ActionState::AfterResolution => self.end_action(),
            // Change of Target: the action ends, unsuccessfully, before block
            // resolution — no combat, no diablerie (LSJ 20050105-2); the
            // blocker is not locked, the card being played before it is
            // (LSJ 19980224, RTR 19991206).
            ActionState::Blocked(blocker) if a.ended_early => {
                if a.blocker_was_unlocked {
                    self.state.minions[blocker.index()].locked = false;
                }
                a.outcome = Some(Outcome::Blocked);
                self.after_resolution();
            }
            ActionState::Blocked(blocker) => {
                if a.continued {
                    // Continued as if unblocked before block resolution:
                    // back to A, declines standing (ANK 20190116).
                    a.continued = false;
                    a.state = ActionState::Open;
                    a.fresh_window();
                    self.flow = Flow::Action;
                } else if a.kind == ActionKind::LeaveTorpor {
                    self.flow = Flow::TorporBlock;
                } else if let Some(slave) = a.slave {
                    let acting = a.acting;
                    self.state.minions[acting.index()].locked = false;
                    self.start_combat(slave, blocker);
                } else {
                    let acting = a.acting;
                    let riders = a.block_riders.clone();
                    let dawn = a.dawn_operation;
                    // Ensnare a Beast [ANI]/[CEL]: its vampire blocked.
                    let toll: u8 = a
                        .strike_toll_if_blocks
                        .iter()
                        .filter(|&&(m, _)| m == blocker)
                        .map(|&(_, n)| n)
                        .sum();
                    self.start_combat(acting, blocker);
                    let c = self.combat.as_mut().expect("combat");
                    c.all_aggravated = dawn;
                    c.sides[0].strike_card_toll += toll;
                    let unlocks = self.combat_riders(&riders);
                    let a = self.action.as_mut().expect("action context");
                    a.after_block_unlocks = unlocks;
                }
            }
        }
    }

    /// Riders on the combatants of the combat just begun (a block's, or one
    /// an action's resolution began); the unlocks for blood they grant
    /// after it.
    fn combat_riders(&mut self, riders: &[(MinionId, cards::BlockRider)]) -> Vec<(MinionId, u8)> {
        let c = self.combat.as_mut().expect("combat");
        let mut unlocks = Vec::new();
        let mut barred = Vec::new();
        for &(m, rider) in riders {
            let Some(i) = c.side_of_minion(m) else {
                continue;
            };
            match rider {
                cards::BlockRider::Maneuver => c.sides[i].combat_maneuvers += 1,
                cards::BlockRider::ManeuverFirstRound => c.sides[i].maneuvers += 1,
                cards::BlockRider::OpponentNoCombatEndsThisAction => {
                    c.sides[1 - i].no_combat_ends_combat = true;
                    barred.push(c.sides[1 - i].minion);
                }
                cards::BlockRider::Press => c.sides[i].combat_presses += 1,
                cards::BlockRider::PreventFirstRound(n) => c.sides[i].round_prevents += n,
                cards::BlockRider::PreventInCombat(n) => c.sides[i].combat_prevents += n,
                cards::BlockRider::NoStrikesFirstRound => c.no_strikes_round = Some(1),
                cards::BlockRider::NoEquipment => c.sides[i].no_equipment = true,
                cards::BlockRider::CombatEndsFirstRound => c.sides[i].combat_ends_round = Some(1),
                cards::BlockRider::OpponentNoCombatEndsFirstRound => {
                    c.sides[1 - i].no_combat_ends_round = Some(1)
                }
                cards::BlockRider::StrengthBeforeRange(n) => c.sides[i].strength_option += n,
                cards::BlockRider::UnlockAfterForBlood(n) => unlocks.push((m, n)),
                cards::BlockRider::Strength(n) => c.sides[i].strength_mods += n,
                cards::BlockRider::UncontrolledBloodIfWon { clan, n } => {
                    c.won_uncontrolled_blood = Some((m, clan, n))
                }
            }
        }
        if let Some(a) = self.action.as_mut() {
            a.no_combat_ends.extend(barred);
        }
        unlocks
    }

    /// The action fizzles at resolution: on to after resolution.
    fn fizzle(&mut self, mut a: ActionCtx) {
        a.outcome = Some(Outcome::Fizzled);
        self.action = Some(a);
        self.after_resolution();
    }

    /// The action reached resolution — successful, fizzled, failed or
    /// blocked (after its combat), or ended before block resolution (Change
    /// of Target, Mirror Walk): open the after-resolution window over the
    /// action's order (elided when nothing can be played). Every action
    /// that reached resolution has one (owner, 2026-09-26: Daring the
    /// Dawn's damage, ANK 20220331); an ended action takes no more cards
    /// in it (RTR 20180719), a failed one does (ANK 20221205).
    pub(crate) fn after_resolution(&mut self) {
        // A combat queued before resolution (Hedonism), the action
        // successful: it begins after the action's effects, still part of
        // its resolution ("Queue a combat", Sabbat V5 rules; owner,
        // 2026-10-08: rules-qa 53.3); the action resumes here after it.
        // An action failing or ending early loses it (LSJ 20090826); a
        // fizzled one is successful (rules-qa 56.1).
        let successful = self
            .action
            .as_ref()
            .is_some_and(|a| matches!(a.outcome, Some(Outcome::Successful | Outcome::Fizzled)));
        if successful && self.begin_queued_combat() {
            self.action
                .as_mut()
                .expect("action context")
                .queued_at_resolution = true;
            return;
        }
        if let Some(a) = self.action.as_mut() {
            a.queued_combat = None;
            // I am Legion: after resolution, the action successful.
            let n = std::mem::take(&mut a.pool_if_successful);
            if successful && n > 0 && !self.state.seats[a.seat.index()].ousted {
                self.state.seats[a.seat.index()].pool += n as i32;
            }
        }
        self.bleed_ends_riders();
        // WMRH Talk Radio: the minion given intercept did not block since.
        if let Some(a) = self.action.as_mut() {
            let due = std::mem::take(&mut a.pool_if_no_block);
            let losses: Vec<SeatId> = due
                .iter()
                .filter(|&&(m, since, _)| {
                    !a.blocked_by[since.min(a.blocked_by.len())..].contains(&m)
                })
                .map(|&(_, _, s)| s)
                .collect();
            let mut loss = vec![0; self.state.seats.len()];
            for s in losses {
                loss[s.index()] += 1;
            }
            if loss.iter().any(|&n| n > 0) {
                self.burn_pool_batch(&loss);
            }
        }
        // Warsaw Station: a successful action (a fizzled one is successful,
        // with no effect).
        if let Some(a) = self.action.as_ref() {
            let m = a.acting;
            let v = &self.state.minions[m.index()];
            if a.unlock_if_successful.is_some_and(|c| v.clan() == Some(c))
                && matches!(a.outcome, Some(Outcome::Successful | Outcome::Fizzled))
                && v.present()
                && !a.no_unlock
            {
                self.state.unlock(m);
            }
        }
        let st = &self.state;
        let a = self.action.as_mut().expect("action context");
        a.state = ActionState::AfterResolution;
        // The resolution may have ousted seats of the order.
        a.order.retain(|s| !st.seats[s.index()].ousted);
        a.fresh_window();
        self.flow = Flow::Action;
    }

    /// A combat begun in the after-resolution window is over: the window
    /// opens again, every Methuselah may play anew.
    pub(crate) fn back_to_after_resolution(&mut self) {
        let st = &self.state;
        let a = self.action.as_mut().expect("action context");
        a.order.retain(|s| !st.seats[s.index()].ousted);
        a.fresh_window();
        self.flow = Flow::Action;
    }

    /// The combat ended: open the after-combat window over the action's
    /// order (rules-qa 2.1: once, after all combats of the block).
    /// The combat queued in the action (Hedonism) begins, if both its
    /// minions are still ready — a vampire in torpor enters no combat
    /// (advanced rules 4.9.3): true if it did. The action's step resumes
    /// as it ends (`resume_after_queued_combat`).
    pub(crate) fn begin_queued_combat(&mut self) -> bool {
        let Some((x, y)) = self.action.as_mut().and_then(|a| a.queued_combat.take()) else {
            return false;
        };
        let ready = |m: MinionId| self.state.minions[m.index()].ready();
        if !ready(x) || !ready(y) {
            return false;
        }
        self.start_combat(x, y);
        self.combat.as_mut().expect("combat").queued = true;
        true
    }

    /// A bleed of the controller's minion is over, before its
    /// after-resolution window (Forward Momentum: +1 counter if it was
    /// successful, else -1 — a blocked bleed included, rules-qa 53.6),
    /// pending in that window (`ActionCtx::momentum`).
    fn bleed_ends_riders(&mut self) {
        let Some(a) = self.action.as_ref() else {
            return;
        };
        if a.kind != ActionKind::Bleed || a.state == ActionState::AfterResolution {
            return;
        }
        let (seat, acting, successful) = (a.seat, a.acting, a.bleed_successful);
        for (id, r) in self.state.card_riders(Hook::OwnBleedEnds) {
            let c = &self.state.cards[id.index()];
            if !c.present() || self.state.card_controller(id) != seat {
                continue;
            }
            let holds = r.requires.iter().all(|&req| {
                cards::holds(
                    req,
                    &self.state,
                    seat,
                    Some(acting),
                    None,
                    self.action.as_ref(),
                    None,
                    None,
                    false,
                )
            });
            if !holds {
                continue;
            }
            for e in r.effects {
                match *e {
                    Effect::BleedOutcomeCounters => {
                        let a = self.action.as_mut().expect("action context");
                        a.momentum.push((id, successful));
                    }
                    other => unreachable!("not a bleed-ends rider effect: {other:?}"),
                }
            }
        }
    }

    /// A political action card's cost paid now, before its resolution
    /// (Yawp Court, LSJ 20080201): false if unpayable — left to fizzle at
    /// resolution.
    pub(crate) fn pay_political_cost_now(&mut self) -> bool {
        let a = self.action.as_ref().expect("action context");
        if a.cost_paid_early {
            return true;
        }
        let Some(play) = a.card else {
            return true;
        };
        let cost = self.card_blood(a.acting, play, play.x);
        let (_, cost, used) = self.reduced_cost_with(a, 0, cost);
        let acting = a.acting;
        if self.state.minions[acting.index()].blood < cost {
            return false;
        }
        self.state.minions[acting.index()].blood -= cost;
        self.spend_counters(&used);
        self.action
            .as_mut()
            .expect("action context")
            .cost_paid_early = true;
        true
    }

    /// Yawp Court's combat over: the acting vampire still ready at its end
    /// (`ready_at_end`, not out of a torpor it went to, LSJ 20090325-1), the
    /// Sabbat vampire takes 2 environmental damage (RTR 19970630); the
    /// acting vampire not ready now — a Rötschreck's torpor after the combat
    /// included (LSJ 20080821) — the action ends with no referendum (LSJ
    /// 20060902), else it goes on to its referendum (rules-qa 54.1).
    pub(crate) fn after_yawp_combat(&mut self, sabbat: MinionId, ready_at_end: bool) {
        let acting = self.action.as_ref().expect("action context").acting;
        if ready_at_end && self.state.minions[sabbat.index()].present() {
            self.damage_out_of_combat(sabbat, 2, false);
        }
        let Some(a) = self.action.as_mut() else {
            return;
        };
        if !ready_at_end || !self.state.minions[acting.index()].ready() {
            a.outcome = Some(Outcome::Failed);
            self.end_action();
            return;
        }
        self.resume_after_queued_combat();
    }

    /// A queued combat over: the action's step goes on where it was, or
    /// its resolution, the combat begun after its effects.
    pub(crate) fn resume_after_queued_combat(&mut self) {
        let a = self.action.as_mut().expect("action context");
        if std::mem::take(&mut a.queued_at_resolution) {
            self.after_resolution();
            return;
        }
        a.fresh_window();
        self.flow = Flow::Action;
    }

    pub(crate) fn after_combat(&mut self) {
        let a = self.action.as_mut().expect("action context");
        a.state = ActionState::AfterCombat;
        a.fresh_window();
        self.flow = Flow::Action;
    }

    /// A blocked leave-torpor (rulebook "Leave Torpor"): no combat; the
    /// blocking vampire may diablerise the acting vampire, else — or if the
    /// blocker is an ally — the action simply fails (no cost paid).
    pub(crate) fn torpor_block_request(&self) -> DecisionRequest {
        let a = self.action.as_ref().expect("action context");
        let blocker = a.blocker.expect("blocker");
        let mut options = vec![Choice::Pass];
        if !self.state.minions[blocker.index()].is_ally() && !self.state.blood_cursed(blocker) {
            options.push(Choice::Diablerise);
        }
        DecisionRequest::new(
            self.state.minions[blocker.index()].owner,
            WindowLabel::TorporBlock,
            options,
        )
    }

    pub(crate) fn torpor_block_apply(&mut self, choice: &Choice) {
        let a = self.action.as_mut().expect("action context");
        a.outcome = Some(Outcome::Blocked);
        let (blocker, acting) = (a.blocker.expect("blocker"), a.acting);
        match choice {
            Choice::Pass => self.after_resolution(),
            Choice::Diablerise => {
                self.diablerise(blocker, acting);
                self.end_action_into_blood_hunt(blocker, acting);
            }
            other => unreachable!("illegal torpor-block choice {other:?}"),
        }
    }

    /// Rulebook "Diablerie" steps 1–3 as a single unit: blood to the
    /// diablerist (excess drains), victim burned. Equipment and the
    /// advanced rule 4 (a Discipline for an elder victim) need cards —
    /// later.
    fn diablerise(&mut self, diablerist: MinionId, victim: MinionId) {
        let taken = self.state.minions[victim.index()].blood;
        // Regent: to a Sabbat diablerist, before the victim burns (and the
        // blood hunt is called).
        if self.state.sect_of(diablerist) == Some(crate::crypt::Sect::Sabbat) {
            for ci in 0..self.state.cards.len() {
                let c = &self.state.cards[ci];
                let moves = c.in_play
                    && c.host == Some(victim)
                    && cards::lookup(c.card).is_some_and(|d| {
                        d.modes[c.mode as usize]
                            .effects
                            .contains(&Effect::MovesToSabbatDiablerist)
                    });
                if moves {
                    let id = CardId(ci as u32);
                    self.state.cards[ci].host = Some(diablerist);
                    if let Some(t) = cards::card_title(self.state.cards[ci].card) {
                        self.gain_title(diablerist, t, Some(id));
                    }
                }
            }
        }
        // Burned from torpor: the cards on it go, no "ready minion" rider.
        self.burn_minion(victim);
        self.state.gain_blood(diablerist, taken);
        // Blood Weakens, The Slow Withering: ignored until a Gehenna event
        // is played; their replacement is drawn.
        self.state.minions[diablerist.index()].diablerie_shield = true;
        // "Has committed diablerie since your last turn" (Carlton Van Wyk).
        self.state.minions[diablerist.index()].diablerie_turn = Some(self.state.turn);
        self.release_held(|h| &mut h.diablerie);
    }

    /// The action ends and the blood hunt referendum follows immediately;
    /// rotation starts at the diablerist's controller, clockwise (DESIGN
    /// §4.2 — a modeling call, owner-approved 2026-09-02).
    fn end_action_into_blood_hunt(&mut self, diablerist: MinionId, victim: MinionId) {
        // The blood hunt is part of the action (spec §3.7.8.1): the action
        // ends after it, through its after-resolution window (rules-qa 5.8).
        self.action.as_mut().expect("action context").diablerist = Some(diablerist);
        // "Blood hunts cannot be called on the attached vampire" (Archon).
        // Fueled by Heart's Blood: not on a vampire with capacity above its
        // counters who diablerized a younger one.
        let fueled = self
            .state
            .global(cards::Modifier::NoBloodHuntAboveCounters)
            .is_some_and(|c| {
                self.state.capacity_of(diablerist).unwrap_or(0)
                    > self.state.cards[c.index()].counters
                    && self.state.younger_than(victim, diablerist)
                    // Not Karen (LSJ 20100121).
                    && !self.state.gehenna_immune(diablerist)
            });
        if self.state.blood_hunt_barred(diablerist) || fueled {
            self.after_resolution();
            return;
        }
        let seat = self.state.minions[diablerist.index()].owner;
        self.start_referendum(RefKind::BloodHunt { diablerist }, seat, None);
    }

    /// Effects of a modifier / reaction / master resolved during the action.
    pub(crate) fn apply_action_effect(&mut self, effect: Effect, play: &PlayCtx) {
        let mut a = self.action.take().expect("action context");
        let mut burned = None;
        match effect {
            Effect::TakeOverAction => {
                let new = play.actor.expect("the masking vampire");
                self.state.unlock(a.acting);
                self.state.minions[new.index()].locked = true;
                a.acting = new;
            }
            Effect::AddStealth(n) => a.stealth_mods += n,
            Effect::NoLockForBlock => {
                a.no_lock_blockers
                    .push(play.actor.expect("the blocking minion"));
            }
            // For the rest of the action (rules-qa 15.2).
            Effect::AddIntercept(n) => {
                // The minion given the intercept: the card's actor, else the
                // minion attempting to block.
                let m = play
                    .actor
                    .or(match a.state {
                        ActionState::Attempt(m) => Some(m),
                        _ => None,
                    })
                    .expect("intercepting minion");
                a.intercept_given.push((m, n as i8));
            }
            Effect::BleedMod { n, limited } => {
                a.bleed_mods += n;
                if limited {
                    a.limited_bleed_used = true;
                }
            }
            Effect::BleedModX { limited } => {
                a.bleed_mods += play.x;
                if limited {
                    a.limited_bleed_used = true;
                }
            }
            Effect::ReduceBleed(n) => a.bleed_reduce += n,
            Effect::AlliesCannotBlock => a.allies_cannot_block = true,
            Effect::VampiresCannotBlock => a.vampires_cannot_block = true,
            Effect::DamageAfterResolution(n) => a.damage_after += n,
            Effect::PoolIfSuccessful(n) => a.pool_if_successful += n,
            Effect::TargetCannotBlock => {
                if let cards::Target::Minion(t) = play.target {
                    a.barred_blockers.push(t);
                }
            }
            Effect::NoUnlockThisAction => a.no_unlock = true,
            Effect::TargetNoReactions => {
                if let cards::Target::Minion(t) = play.target {
                    a.no_reactions.push(t);
                }
            }
            Effect::NextReactionCosts(n) => a.reaction_surcharge += n,
            // The bleed amount as it resolves; later increases still count.
            Effect::ReduceBleedToZero => {
                self.action = Some(a);
                let n = self
                    .action
                    .as_ref()
                    .expect("action")
                    .bleed_amount(&self.state);
                a = self.action.take().expect("action");
                a.bleed_reduce += n;
            }
            Effect::EndBlockedAction => {
                a.ended_early = true;
                if !a.no_unlock {
                    self.state.unlock(a.acting);
                }
            }
            Effect::EndBlockedActionLocked => a.ended_early = true,
            Effect::StealthForNextActions => {
                let m = play.actor.expect("actor");
                let x = play.x.min(self.state.minions[m.index()].blood);
                self.state.minions[m.index()].blood -= x;
                self.state.seats[play.seat.index()].veil_left += x;
                self.state.veil_superior = true;
            }
            Effect::StrengthThisAction(n) => a.strength_bonus += n,
            Effect::AggravatedHandsThisAction => a.aggravated_hands = true,
            // No capacity: allies only.
            Effect::AlliesIntercept(n) => self.state.layers.push(cards::Layer {
                scope: Scope::Action,
                modifier: cards::Modifier::InterceptMod { younger_than: 0, n },
                source: crate::state::CardId(u32::MAX),
            }),
            Effect::BlockToll(n) => a.block_toll_extra += n,
            Effect::BlockTollBlood(n) => a.block_toll_blood += n,
            Effect::UnlockActingForActorBlood(n) => {
                a.unlock_paid_by.push((play.actor.expect("actor"), n));
            }
            Effect::SectInterceptThisAction(sect, n) => a.sect_intercept.push((sect, n)),
            Effect::StrikeTollIfBlocks(n) => {
                a.strike_toll_if_blocks
                    .push((play.actor.expect("actor"), n));
            }
            // Hedonism: the attempting minion locked, a combat queued with
            // it — before its attempt fails.
            Effect::LockAttempting => {
                if let ActionState::Attempt(m) = a.state {
                    self.state.minions[m.index()].locked = true;
                }
            }
            Effect::AttemptingBurnsBlood(n) => {
                if let ActionState::Attempt(m) = a.state {
                    let v = &mut self.state.minions[m.index()];
                    v.blood = v.blood.saturating_sub(n);
                }
            }
            Effect::AttemptingToTorpor => {
                if let ActionState::Attempt(m) = a.state {
                    if self.state.minions[m.index()].ready() {
                        self.minions_to_torpor(&[m]);
                    }
                }
            }
            Effect::CancelAction => {
                a.attempt_fails();
                a.ended_early = true;
                a.canceled = true;
            }
            Effect::QueueCombatWithAttempting => {
                if let ActionState::Attempt(m) = a.state {
                    a.queued_combat = Some((play.actor.expect("actor"), m));
                }
            }
            Effect::PutOnActorIfBleedSuccessful => {
                let h = crate::combat::Held::from_play(play, play.actor.expect("actor"), 0);
                a.put_if_bleed_successful.push(h);
            }
            // "Minions get -1 intercept" (Unthinkable Humiliation [POT]):
            // every minion, this action.
            // Hospital Food: "the anarch gains an additional blood" — a
            // hunt bonus of this action (rules-qa 63.12).
            Effect::HuntBonusThisAction(n) => {
                self.state.layers.push(cards::Layer {
                    scope: Scope::Action,
                    modifier: cards::Modifier::ActionHunt(n),
                    source: crate::state::CardId(u32::MAX),
                });
            }
            Effect::CorruptedIntercept(n) => {
                self.state.layers.push(cards::Layer {
                    scope: Scope::Action,
                    modifier: cards::Modifier::CorruptedInterceptMod(n),
                    source: crate::state::CardId(u32::MAX),
                });
            }
            Effect::MinionsIntercept(n) => {
                self.state.layers.push(cards::Layer {
                    scope: Scope::Action,
                    modifier: cards::Modifier::InterceptMod {
                        younger_than: u8::MAX,
                        n,
                    },
                    source: crate::state::CardId(u32::MAX),
                });
            }
            Effect::YoungerIntercept(n) => {
                // An ally actor has no capacity: allies only.
                let younger_than = self
                    .state
                    .capacity_of(play.actor.expect("actor"))
                    .unwrap_or(0);
                self.state.layers.push(cards::Layer {
                    scope: Scope::Action,
                    modifier: cards::Modifier::InterceptMod { younger_than, n },
                    source: crate::state::CardId(u32::MAX),
                });
            }
            Effect::DistributeMargin {
                clan,
                pool,
                titled_pool,
            } => {
                let titled = self.state.title_of(a.acting) != crate::crypt::Title::None;
                if a.margin > 0 {
                    self.interrupts.push(crate::draws::Interrupt::MarginPool {
                        seat: play.seat,
                        left: a.margin.min(u8::MAX as u32) as u8,
                        cap: if titled { titled_pool } else { pool },
                        clan,
                    });
                }
            }
            Effect::GainBloodPerMargin => {
                // Voter Captivation: 1 blood per vote of the margin; X of
                // them to the pool instead (a passed referendum with no
                // polling step passes by 0: PIB 20150105).
                let m = play.actor.expect("actor");
                let to_pool = play.x.min(a.margin as u8);
                self.state.gain_blood(m, a.margin as u8 - to_pool);
                self.state.seats[play.seat.index()].pool += to_pool as i32;
            }
            Effect::Wake => a.woken.push(play.actor.expect("waking minion")),
            Effect::WakeTarget => {
                let t = play.target.minion();
                a.woken.push(t);
            }
            Effect::IfBlocks(rider) => a.block_riders.push((play.actor.expect("actor"), rider)),
            // The minion attempting to block (Form of the Bat [pro]: "the
            // blocking minion gets -1 intercept"), for the rest of the
            // action.
            Effect::AttemptingIntercept(n) => {
                if let ActionState::Attempt(m) = a.state {
                    a.intercept_given.push((m, n));
                }
            }
            Effect::AttemptFails => a.attempt_fails(),
            Effect::AttemptFailsBarSeat => {
                if let ActionState::Attempt(m) = a.state {
                    a.barred_seats.push(self.state.minions[m.index()].owner);
                }
                a.attempt_fails();
            }
            Effect::FailBlockForCorruption => a.corruption_fails += 1,
            Effect::UnlockForCorruption(n) => a.corruption_unlock = n,
            Effect::VeilThins => {
                a.veil_thins = true;
                // The minion attempting now pays, or resigns its attempt.
                if let ActionState::Attempt(m) = a.state {
                    let seat = self.state.minions[m.index()].owner;
                    self.interrupts
                        .push(crate::draws::Interrupt::BlockToll { seat, minion: m });
                }
            }
            Effect::LockFailedBlockers => a.lock_failed = true,
            Effect::CancelableForPool(_) | Effect::CancelableForBlockerBlood(_) => {}
            // Negative stealth is left unchanged (LSJ 20021211).
            Effect::ReduceStealthToZero => {
                let s = a.stealth(&self.state).max(0) as u8;
                a.stealth_reduce = a.stealth_reduce.saturating_add(s);
            }
            Effect::BurnActorBlood(n) => {
                self.state.burn_blood(play.actor.expect("actor"), n);
            }
            Effect::TargetAttemptsBlock(n) => {
                let t = play.target.minion();
                self.state.unlock(t);
                a.intercept_given.push((t, n as i8));
                // Its toll unpayable by now (the card's own blood paid
                // first): no attempt (rules-qa 42.8). During another of
                // its Methuselah's minions' attempt, a change of blocker:
                // that attempt fails (rules-qa 48.3).
                let switch = match a.state {
                    ActionState::Attempt(x) => {
                        x != t
                            && self.state.minions[x.index()].owner == play.seat
                            && a.stealth(&self.state) > a.intercept_of(&self.state, x)
                    }
                    _ => false,
                };
                if (a.state == ActionState::Open || switch) && self.block_toll(&a, t).is_some() {
                    if let ActionState::Attempt(x) = a.state {
                        if a.lock_failed {
                            a.failed_blockers.push(x);
                        }
                        let print = a.attempt_print(&self.state, x);
                        a.failed_attempts.push((x, print));
                    }
                    a.state = ActionState::Attempt(t);
                    self.action = Some(a);
                    self.pay_block_toll(t, true);
                    return;
                }
            }
            Effect::BurnForInterceptThisAction => {
                a.burn_for_intercept.push(play.actor.expect("actor"))
            }
            Effect::TargetBurnsBloodForIntercept => {
                if let Target::Minion(t) = play.target {
                    a.burn_for_intercept_once.push(t);
                }
            }
            Effect::PoolIfAttemptingDoesNotBlock => {
                if let ActionState::Attempt(m) = a.state {
                    a.pool_if_no_block.push((m, a.blocked_by.len(), play.seat));
                }
            }
            Effect::LockIfNoBlock => a.lock_if_no_block.push(play.actor.expect("actor")),
            Effect::DawnOperation { cancel } => {
                a.dawn_operation = true;
                // "If a vampire is currently attempting to block, they can
                // cancel their block attempt."
                if let (true, ActionState::Attempt(m)) = (cancel, a.state) {
                    if self.state.minions[m.index()].is_vampire() {
                        let seat = self.state.minions[m.index()].owner;
                        self.interrupts
                            .push(crate::draws::Interrupt::CancelAttempt { seat, minion: m });
                    }
                }
            }
            Effect::PutOnActorIfNoBlock | Effect::PutOnActingIfBlocks => {
                let h = crate::combat::Held::from_play(play, play.actor.expect("actor"), 0);
                if effect == Effect::PutOnActorIfNoBlock {
                    a.put_if_no_block.push(h);
                } else {
                    a.put_on_acting_if_blocks.push(h);
                }
            }
            Effect::BurnIfNoBlock(n) => {
                let since = a.blocked_by.len();
                a.burn_if_no_block
                    .push((play.actor.expect("actor"), since, n));
            }
            Effect::ContinueAfterResolution => a.continued = true,
            Effect::UnlockForBloodIfSuccessful(n) => {
                a.after_resolution_unlocks
                    .push((play.actor.expect("actor"), n));
            }
            Effect::AttemptBlock => {
                let m = play.actor.expect("attempting minion");
                if a.state == ActionState::Open && self.block_toll(&a, m).is_some() {
                    a.state = ActionState::Attempt(m);
                    self.action = Some(a);
                    self.pay_block_toll(m, true);
                    return;
                }
            }
            Effect::AllySkipsNextUnlock => {
                let m = &mut self.state.minions[play.actor.expect("actor").index()];
                if m.is_ally() {
                    m.skip_unlock = true;
                }
            }
            Effect::LockActor => {
                self.state.minions[play.actor.expect("actor").index()].locked = true;
            }
            Effect::UnlockActor if a.no_unlock => {}
            Effect::UnlockActor => {
                self.state.unlock(play.actor.expect("actor"));
            }
            Effect::RedirectBleed => {
                let cards::Target::Seat(t) = play.target else {
                    unreachable!("redirect without a seat target");
                };
                // A change of target reopens block attempts for the new
                // target (rulebook A.3 / C.2; rules-qa 1.3): back to A with
                // passes voided.
                a.bleed_target = Some(t);
                a.declined.clear();
                a.state = ActionState::Open;
                a.order = self.action_order(a.seat, Some(t));
                a.fresh_window();
            }
            Effect::BurnActingMinion => burned = Some(a.acting),
            Effect::ActionFails => a.failed = true,
            Effect::ActionLayer(modifier) => {
                self.state.layers.push(cards::Layer {
                    scope: Scope::Action,
                    modifier,
                    source: crate::state::CardId(u32::MAX),
                });
            }
            Effect::DeferReactionReplacement => a.defer_reaction_replacement = true,
            // A marker of the card placed later (Melange [AUS]).
            Effect::StillYours => {}
            Effect::EndActionIfBlocked => a.end_on_block = true,
            other => unreachable!("not an action effect: {other:?}"),
        }
        self.action = Some(a);
        if let Some(m) = burned {
            // Its riders (Tension in the Ranks) may oust the acting
            // Methuselah: the play's resolution passes the turn.
            self.burn_minion(m);
        }
    }

    /// Riders printed on the vampire an action targets, fired as the
    /// as-announced window closes (Evan Klein's coin flip). Returns true
    /// when the action ended.
    fn announced_at_target_riders(&mut self) -> bool {
        let a = self.action.as_ref().expect("action context");
        let (kind, acting) = (a.kind, a.acting);
        let Some(target) = a.directed_minion() else {
            return false;
        };
        let v = &self.state.minions[target.index()];
        let owner = v.owner;
        let Some(def) = cards::vampire_of(v).filter(|_| v.present()) else {
            return false;
        };
        let effects: Vec<Effect> = def
            .riders
            .iter()
            .filter(|r| {
                r.hook == Hook::ActionAnnouncedAtTarget
                    && r.requires.iter().all(|&req| {
                        cards::holds(
                            req,
                            &self.state,
                            owner,
                            Some(target),
                            None,
                            self.action.as_ref(),
                            None,
                            None,
                            false,
                        )
                    })
            })
            .flat_map(|r| r.effects.iter().copied())
            .collect();
        for e in effects {
            match e {
                // Tails: the action fails at once — no block, no cost — the
                // acting minion locked and the action counted for NRA
                // (rules-qa 16.6).
                Effect::CoinFlipActionFails if self.rng.below(2) == 0 => {
                    self.action.as_mut().expect("action context").outcome = Some(Outcome::Failed);
                    self.mark_resolved(acting, kind);
                    self.after_resolution();
                    return true;
                }
                Effect::CoinFlipActionFails => {}
                other => unreachable!("not an announce rider effect: {other:?}"),
            }
        }
        false
    }

    /// The action reached resolution: the rulebook's once-a-turn bleed and
    /// political action (NRA, rules-qa 1.4).
    fn mark_resolved(&mut self, acting: MinionId, kind: ActionKind) {
        let m = &mut self.state.minions[acting.index()];
        match kind {
            ActionKind::Bleed => m.bled_this_turn = true,
            ActionKind::Political => m.political_this_turn = true,
            _ => {}
        }
    }

    /// Forced Awakening: those that did not block after playing it burn
    /// their blood as the action begins to resolve (LSJ 19990421) — at its
    /// resolution, or as another minion's block, which is the action's
    /// resolution (owner, 2026-09-26: rules-qa 30.2) — once; blocks before
    /// the card do not count (ANK 20230319).
    fn awakening_burns(&mut self, blocker: Option<MinionId>) {
        let a = self.action.as_mut().expect("action context");
        let entries = std::mem::take(&mut a.burn_if_no_block);
        let burns: Vec<(MinionId, u8)> = entries
            .iter()
            .filter(|&&(m, since, _)| !a.blocked_by[since..].contains(&m) && blocker != Some(m))
            .map(|&(m, _, n)| (m, n))
            .collect();
        for (m, n) in burns {
            let v = &mut self.state.minions[m.index()];
            if v.present() {
                v.blood -= n.min(v.blood);
            }
        }
    }

    /// Block resolution: the blocker locks; combat, except a blocked
    /// leave-torpor (no combat; the blocker may diablerise).
    fn blocked(&mut self, blocker: MinionId) {
        // Unleash Hell's Fury: the acting minion takes 1 unpreventable
        // environmental aggravated damage, the card burns — before any
        // other effect, even if the action is ended (ANK 20210627, LSJ
        // 20100301); not locked, no combat.
        if let crate::state::MinionKind::Fury { id, .. } = self.state.minions[blocker.index()].kind
        {
            let (acting, kind) = {
                let a = self.action.as_mut().expect("action context");
                a.blocker = Some(blocker);
                (a.acting, a.kind)
            };
            self.mark_resolved(acting, kind);
            self.state.minions[blocker.index()].in_play = false;
            self.burn_card_in_play(id);
            self.damage_out_of_combat(acting, 1, true);
            if let Some(a) = self.action.as_mut() {
                a.outcome = Some(Outcome::Blocked);
                self.after_resolution();
            }
            return;
        }
        self.awakening_burns(Some(blocker));
        let was_unlocked = !self.state.minions[blocker.index()].locked;
        let (acting, kind, no_lock) = {
            let a = self.action.as_mut().expect("action context");
            a.blocker = Some(blocker);
            a.blocker_was_unlocked = was_unlocked;
            (a.acting, a.kind, a.no_lock_blockers.contains(&blocker))
        };
        // "This vampire does not lock for the block" (Read the Winds).
        if !no_lock {
            self.state.minions[blocker.index()].locked = true;
        }
        // NRA triggers on resolution, blocked included (rules-qa 1.4).
        self.mark_resolved(acting, kind);
        // "If blocked" riders of the acting minion's cards (Unlicensed
        // Taxicab) apply before any other card or effect (ANK 20220116).
        self.host_blocked_riders(acting, blocker);
        // Its controller ousted by such a rider (Terrifying Visage): the
        // blocker left play, the action is blocked with no combat (rules-qa
        // 53.1).
        if self.action.is_none() {
            return;
        }
        if !self.state.minions[blocker.index()].present() {
            self.action.as_mut().expect("action context").outcome = Some(Outcome::Blocked);
            self.after_resolution();
            return;
        }
        self.host_blocks_riders(acting, blocker);
        // Mirror Walk [THA]: the action ends before block resolution — no
        // combat, no diablerie opportunity on a blocked leave-torpor (PIB
        // 20121216), nothing more played on it (RTR 20180719).
        if self.action.as_ref().is_some_and(|a| a.end_on_block) {
            self.action.as_mut().expect("action context").outcome = Some(Outcome::Blocked);
            self.after_resolution();
            return;
        }
        // Before block resolution: the acting Methuselah's "if blocked"
        // effects (elided when there are none).
        let a = self.action.as_mut().expect("action context");
        a.state = ActionState::Blocked(blocker);
        a.blocked_by.push(blocker);
        a.fresh_window();
        self.flow = Flow::Action;
        // "If this vampire blocks, put this card on the acting minion; you
        // still control this card" (Melange [AUS]).
        let a = self.action.as_mut().expect("action context");
        let placed: Vec<crate::combat::Held> = a
            .put_on_acting_if_blocks
            .iter()
            .copied()
            .filter(|h| h.minion == blocker)
            .collect();
        a.put_on_acting_if_blocks.retain(|h| h.minion != blocker);
        for h in placed {
            self.place_held(h, acting);
        }
    }

    /// Riders of the cards on the acting minion fired as its action is
    /// blocked, their conditions read on the blocker.
    fn host_blocked_riders(&mut self, acting: MinionId, blocker: MinionId) {
        let seat = self.state.minions[acting.index()].owner;
        let mut burns = Vec::new();
        let mut bleeds: Vec<u8> = Vec::new();
        // "The blocking minion's controller burns n pool" (Terrifying
        // Visage, Aelswith).
        let mut toll = 0;
        for (id, r) in self.state.card_riders(Hook::HostBlocked) {
            let c = &self.state.cards[id.index()];
            if !c.present() || c.host != Some(acting) {
                continue;
            }

            let holds = r.requires.iter().all(|&req| {
                cards::holds(
                    req,
                    &self.state,
                    seat,
                    Some(acting),
                    None,
                    self.action.as_ref(),
                    None,
                    None,
                    false,
                )
            });
            if !holds {
                continue;
            }
            // Its version (Phantasmagoria [PRE]).
            if !r.requires.iter().all(|&req| match req {
                Requirement::SuperiorVersion => c.mode > 0,
                _ => true,
            }) {
                continue;
            }
            for e in r.effects {
                match *e {
                    Effect::BurnSelf => burns.push(id),
                    // "They burn 1 blood or life before block
                    // resolution" (Phantasmagoria [PRE]).
                    Effect::HostBurnBlood(n) => bleeds.push(n),
                    Effect::BlockerControllerBurnsPool(n) => toll += n,
                    other => unreachable!("not a host-blocked rider effect: {other:?}"),
                }
            }
        }
        for id in burns {
            self.burn_card_in_play(id);
        }
        for n in bleeds {
            let v = &mut self.state.minions[acting.index()];
            v.blood -= n.min(v.blood);
            if v.is_ally() && v.blood == 0 {
                self.burn_minion(acting);
            }
        }
        // Printed on the acting vampire (Marialena).
        if let Some(def) = cards::vampire_of(&self.state.minions[acting.index()]) {
            for r in def.riders.iter().filter(|r| r.hook == Hook::HostBlocked) {
                if !r
                    .requires
                    .iter()
                    .all(|&req| cards::holds_for_host(req, &self.state, acting))
                {
                    continue;
                }
                for e in r.effects {
                    match *e {
                        Effect::HostBurnBlood(n) => {
                            self.state.burn_blood(acting, n);
                        }
                        Effect::BlockerControllerBurnsPool(n) => toll += n,
                        other => unreachable!("not a host-blocked rider effect: {other:?}"),
                    }
                }
            }
        }
        self.blocker_controller_burns(blocker, toll);
    }

    /// The pool burned by the blocker's controller as the action is blocked
    /// (`host_blocked_riders`), the last ousting them: their minion leaves
    /// play with them and no combat follows (rules-qa 53.1).
    fn blocker_controller_burns(&mut self, blocker: MinionId, n: u8) {
        if n == 0 {
            return;
        }
        let mut loss = vec![0; self.state.seats.len()];
        loss[self.state.minions[blocker.index()].owner.index()] = n as i32;
        self.burn_pool_batch(&loss);
    }

    /// A bleed action card's "if the bleed is successful" effects (Break
    /// the Bonds, The Platinum Protocol), in printed order.
    fn bleed_successful_effects(&mut self, play: &PlayCtx, prey: SeatId) {
        let Some(def) = cards::lookup(play.card) else {
            return;
        };
        for e in def.modes[play.mode as usize].effects {
            let Effect::IfBleedSuccessful(then) = e else {
                continue;
            };
            for &e in *then {
                let purpose = match e {
                    Effect::LockMinionOfTarget => crate::draws::MinionPurpose::Lock,
                    Effect::CorruptMinionOfTarget => crate::draws::MinionPurpose::Corrupt,
                    // The vampire named at declaration, if still a younger
                    // one of the player's (Feast of the Soul's Secrets
                    // [DOM]; ANK 20240628-2).
                    Effect::BloodToTarget(n) => {
                        if let Target::Minion(t) = play.target {
                            let v = &self.state.minions[t.index()];
                            if v.present() && v.owner == play.seat {
                                self.state.gain_blood(t, n);
                            }
                        }
                        continue;
                    }
                    Effect::GainPool(n) => {
                        self.state.seats[play.seat.index()].pool += n as i32;
                        continue;
                    }
                    Effect::TargetMethuselahDiscards(n) => {
                        self.interrupts
                            .push(crate::draws::Interrupt::DiscardChosen {
                                seat: prey,
                                left: n,
                                done: 0,
                                combat_only: false,
                            });
                        continue;
                    }
                    Effect::BloodToUncontrolledTarget(n) => {
                        // The vampire named at declaration, if still there.
                        if let Target::Uncontrolled(i) = play.target {
                            if (i as usize) < self.state.seats[play.seat.index()].uncontrolled.len()
                            {
                                self.state.add_uncontrolled_blood(play.seat, i as usize, n);
                            }
                        }
                        continue;
                    }
                    other => unreachable!("not a successful-bleed effect: {other:?}"),
                };
                self.interrupts.push(crate::draws::Interrupt::MinionOf {
                    seat: play.seat,
                    of: prey,
                    purpose,
                });
            }
        }
    }

    /// Riders of the cards on the acting minion remembering the Methuselah
    /// it is about to bleed successfully (Spying Mission [OBF]: "burn this
    /// card and this vampire gets +2 bleed"): all at once (ANK 20200609),
    /// after every modifier and reaction (LSJ 19980105). Returns the bleed
    /// they add.
    fn about_to_bleed_riders(&mut self, acting: MinionId, prey: SeatId) -> u8 {
        let mut burns = Vec::new();
        let mut bleed = 0;
        for (id, r) in self.state.card_riders(Hook::HostAboutToBleedRemembered) {
            let c = &self.state.cards[id.index()];
            if !c.present()
                || c.host != Some(acting)
                || c.against != crate::ids::SeatSet::single(prey)
                || !r
                    .requires
                    .iter()
                    .all(|&req| cards::holds_for_host(req, &self.state, acting))
            {
                continue;
            }
            for e in r.effects {
                match *e {
                    Effect::BurnSelf => burns.push(id),
                    Effect::BleedMod { n, .. } => bleed += n,
                    other => unreachable!("not an about-to-bleed rider effect: {other:?}"),
                }
            }
        }
        for id in burns {
            self.burn_card_in_play(id);
        }
        bleed
    }

    /// Riders of the cards on the blocker fired as it blocks, before block
    /// resolution, their conditions read on the acting minion (Rebel).
    fn host_blocks_riders(&mut self, acting: MinionId, blocker: MinionId) {
        let seat = self.state.minions[blocker.index()].owner;
        let mut gains = Vec::new();
        for (ci, c) in self.state.cards.iter().enumerate() {
            if !c.present() || c.host != Some(blocker) {
                continue;
            }
            let Some(def) = cards::lookup(c.card) else {
                continue;
            };
            for (ri, r) in def.riders.iter().enumerate() {
                if r.hook != Hook::HostBlocks {
                    continue;
                }
                // Rider uses share the turn's ability records, past the
                // card's abilities.
                let key = cards::AbilitySource::Card(CardId(ci as u32)).use_key(128 + ri as u8);
                if r.requires.contains(&cards::Requirement::OncePerTurn)
                    && self.state.turn_uses.contains(&key)
                {
                    continue;
                }
                let holds = r.requires.iter().all(|&req| {
                    cards::holds(
                        req,
                        &self.state,
                        seat,
                        Some(acting),
                        None,
                        self.action.as_ref(),
                        None,
                        None,
                        false,
                    )
                });
                if !holds {
                    continue;
                }
                gains.push((key, r.effects));
            }
        }
        for (key, effects) in gains {
            self.state.turn_uses.push(key);
            for e in effects {
                match *e {
                    Effect::AddHostBlood(n) => {
                        self.state.gain_blood(blocker, n);
                    }
                    other => unreachable!("not a host-blocks rider effect: {other:?}"),
                }
            }
        }
    }

    /// The effects of the ability granting the current action, if any.
    fn granted_effects(&self, a: &ActionCtx) -> &'static [Effect] {
        match (a.source, a.ability) {
            (Some(src), Some(ab)) => src.mode(&self.state, ab).effects,
            _ => &[],
        }
    }

    /// Blood `m`'s printed text adds to its directed actions (Daria), the
    /// action read by `ctx`.
    pub(crate) fn action_toll(&self, m: MinionId, ctx: crate::state::PrintedCtx) -> u8 {
        self.state
            .printed_mod(m, cards::Stat::ActionCost, ctx)
            .max(0) as u8
    }

    /// The blood `o`, an action `m` announces, costs it before any toll:
    /// an action card's (surcharges and printed reductions in, less the
    /// reductions it could announce), a granted action's printed blood;
    /// none for a rulebook action (a rescue's may be the rescued one's).
    fn announced_blood(&self, seat: SeatId, m: MinionId, o: &Choice) -> u8 {
        match *o {
            Choice::Play {
                card, mode, target, ..
            } => {
                let Some(&(lib, _)) = self.virtual_hand(seat).get(card as usize) else {
                    return 0;
                };
                let Some(def) = cards::lookup(lib) else {
                    return 0;
                };
                let x =
                    cards::terms_x(&self.state, seat, lib, &def.modes[mode as usize]).unwrap_or(0);
                let ctx = PlayCtx::new(seat, lib, mode, Some(m), target, x);
                self.card_blood(m, ctx, x)
                    .saturating_sub(self.cost_potential(&ctx))
            }
            Choice::UseAbility {
                source, ability, ..
            } => source
                .mode(&self.state, ability)
                .effects
                .iter()
                .map(|e| match e {
                    Effect::PayBlood(n) => *n,
                    _ => 0,
                })
                .sum(),
            _ => 0,
        }
    }

    /// A granted action's printed pool and blood costs.
    fn granted_cost(&self, a: &ActionCtx) -> (u8, u8) {
        self.granted_effects(a)
            .iter()
            .fold((0, 0), |(pool, blood), e| match e {
                Effect::PayPool(n) => (pool + n, blood),
                Effect::PayBlood(n) => (pool, blood + n),
                _ => (pool, blood),
            })
    }

    fn succeed(&mut self) {
        // Faceless Night [OBF]: the failed blockers lock before resolution.
        let failed = self
            .action
            .as_ref()
            .expect("action context")
            .failed_blockers
            .clone();
        for m in failed {
            if self.state.minions[m.index()].present() {
                self.state.minions[m.index()].locked = true;
            }
        }
        self.awakening_burns(None);
        let mut a = self.action.take().expect("action context");
        let seat = a.seat;
        a.outcome = Some(Outcome::Successful);
        // "Directed actions cost her +1 blood" (Daria; Marcellus's
        // political surcharge): the action's cost, paid first as it
        // resolves; unpayable by then, the action fizzles (rules-qa 3.6,
        // 42.1, 42.6).
        let toll = self.action_toll(a.acting, a.printed_ctx(&self.state, None));
        if toll > 0 {
            if self.state.minions[a.acting.index()].blood < toll {
                // It reached resolution: NRA holds (rules-qa 1.4).
                self.mark_resolved(a.acting, a.kind);
                return self.fizzle(a);
            }
            self.state.minions[a.acting.index()].blood -= toll;
        }
        match a.kind {
            ActionKind::Bleed => {
                let prey = a.bleed_target.expect("bleed target");
                self.state.minions[a.acting.index()].bled_this_turn = true;
                // The bleed's target was ousted before it resolved (the
                // cost of a cancel, LSJ 20050607): it resolves without
                // effect — no pool, no Edge.
                if self.state.seats[prey.index()].ousted {
                    return self.fizzle(a);
                }
                // A bleed granted by a card on the acting minion (Codex of
                // the Edenic Groundskeepers): it fizzles if the card is no
                // longer on the acting minion (RTR 19960221) or its blood
                // cost is unpayable by now (rules-qa 3.6).
                if let Some(src) = a.source {
                    let (_, cost) = self.granted_cost(&a);
                    // A vampire's own printed bleed has no card to lose.
                    let attached = match src {
                        cards::AbilitySource::Card(c) => {
                            let card = &self.state.cards[c.index()];
                            card.present() && card.host == Some(a.acting)
                        }
                        cards::AbilitySource::Vampire(_) => true,
                    };
                    if !attached || self.state.minions[a.acting.index()].blood < cost {
                        return self.fizzle(a);
                    }
                    self.state.minions[a.acting.index()].blood -= cost;
                }
                // A bleed action card's cost (Slaughtering the Herd [dom]):
                // paid now, unpayable by then the action fizzles (spec
                // §1.7.1, rules-qa 3.6).
                if let Some(play) = a.card {
                    let (_, blood, used) =
                        self.reduced_cost_with(&a, 0, self.card_blood(a.acting, play, play.x));
                    if self.state.minions[a.acting.index()].blood < blood {
                        return self.fizzle(a);
                    }
                    self.spend_counters(&used);
                    self.state.minions[a.acting.index()].blood -= blood;
                }
                // About to bleed for 1 pool or more: the cards remembering
                // this prey burn for their bonus (Spying Mission [OBF]).
                if a.bleed_amount(&self.state) > 0 {
                    a.bleed_mods += self.about_to_bleed_riders(a.acting, prey);
                }
                let amount = a.bleed_amount(&self.state);
                let lost = amount.min(self.state.seats[prey.index()].pool.max(0) as u8);
                self.state.seats[prey.index()].pool -= amount as i32;
                // The context is out of `self.action` here: Věnceslava's
                // memory is set on it directly — the bleed's target the
                // acting Methuselah's prey, not a deflected one.
                if amount > 0 && prey == self.state.prey(seat) && prey != seat {
                    a.prey_burned = true;
                }
                if lost > 0 {
                    self.own_bleed_riders(seat, a.acting, prey, lost);
                }
                // A successful bleed, for 1 pool or more: take the Edge
                // (rulebook "Bleed").
                if amount > 0 {
                    a.bleed_successful = true;
                    for s in &mut self.state.seats {
                        s.has_edge = false;
                    }
                    self.state.seats[seat.index()].has_edge = true;
                    // The action card's "if the bleed is successful" effects,
                    // their minions chosen now — none left of an ousted prey.
                    if let Some(play) = a.card {
                        self.bleed_successful_effects(&play, prey);
                        // Enthrall [OBF]/[PRE]: its crypt draw for blood,
                        // offered after resolution.
                        let draw = cards::lookup(play.card).and_then(|d| {
                            d.modes[play.mode as usize]
                                .effects
                                .iter()
                                .find_map(|e| match e {
                                    Effect::CryptForBloodIfBleedSuccessful(n) => Some(*n),
                                    _ => None,
                                })
                        });
                        if let Some(n) = draw {
                            a.crypt_for_blood = Some((a.acting, n));
                        }
                        // Fiendish Tongue: a Tzimisce's unlock in the
                        // coming discard phase.
                        let unlock = cards::lookup(play.card).and_then(|d| {
                            d.modes[play.mode as usize]
                                .effects
                                .iter()
                                .find_map(|e| match e {
                                    Effect::TzimisceDiscardUnlockIfBleedSuccessful(n) => Some(*n),
                                    _ => None,
                                })
                        });
                        let m = &mut self.state.minions[a.acting.index()];
                        if let Some(n) = unlock.filter(|_| m.clan() == Some("Tzimisce")) {
                            m.discard_unlock_for_blood = n;
                        }
                    }
                }
                // Below 0 kept until the oust: a Life Boon must make it good
                // (RTR 19970630).
                if self.state.seats[prey.index()].pool <= 0 && prey == self.state.active {
                    // The active Methuselah bled out by another's minion
                    // (Enkil Cog's action in their phase): ousted, the turn
                    // passes with the action.
                    self.action = Some(a);
                    if self.oust_and_pass(prey) {
                        return;
                    }
                    a = self.action.take().expect("action");
                } else if self.state.seats[prey.index()].pool <= 0 {
                    self.oust(prey);
                } else if a.card.is_some_and(|p| {
                    cards::lookup(p.card).is_some_and(|d| {
                        d.modes[p.mode as usize]
                            .effects
                            .contains(&Effect::TargetLocksReadyUnlocked)
                    })
                }) {
                    // Propaganda [PRE]: "the target Methuselah locks a ready
                    // unlocked minion they control" — the one named as the
                    // card was played (owner, 2026-09-26: rules-qa 31.2),
                    // whatever the bleed's amount.
                    if let Some(Target::Minion(t)) = a.card.map(|p| p.target) {
                        let v = &mut self.state.minions[t.index()];
                        if v.present() {
                            v.locked = true;
                        }
                    }
                }
            }
            ActionKind::Hunt => {
                self.state.minions[a.acting.index()].must_hunt = false;
                let before = self.state.minions[a.acting.index()].blood;
                let hunt = 1 + self.state.hunt_bonus(a.acting);
                self.state.hunt_blood(a.acting, hunt);
                // Successful only if blood was gained (LSJ 20050720-2).
                a.hunt_gained = self.state.minions[a.acting.index()].blood > before;
                if a.hunt_gained {
                    self.successful_hunt_riders(a.acting);
                }
            }
            ActionKind::BecomeAnarch => {
                // The cost at resolution (a blocked action pays nothing);
                // unpayable by then, or no longer an untitled non-Anarch
                // vampire, it fizzles (rules-qa 3.6).
                let acting = a.acting;
                let cost = self.state.become_anarch_cost(acting);
                if self.state.minions[acting.index()].blood < cost
                    || !self.state.may_become_anarch(acting)
                {
                    a.outcome = Some(Outcome::Fizzled);
                } else {
                    self.state.minions[acting.index()].blood -= cost;
                    self.make_anarch(acting);
                }
            }
            ActionKind::LeaveTorpor => {
                // Cost is paid at resolution only (a blocked action pays
                // nothing); blood spent during the action can leave it
                // unpayable, in which case the action fizzles (rules-qa
                // 3.6): successful, no effect.
                let acting = a.acting;
                // Torpid Blood: +1.
                let cost = 2 + self.state.torpid_surcharge(None, acting);
                let m = &mut self.state.minions[acting.index()];
                if !m.in_torpor {
                    // Out of torpor already (Warsaw Station): it fizzles,
                    // its cost paid (ANK 20220218).
                    m.blood -= m.blood.min(cost);
                    a.outcome = Some(Outcome::Fizzled);
                } else if m.blood >= cost {
                    m.blood -= cost;
                    self.leave_torpor(acting);
                } else {
                    a.outcome = Some(Outcome::Fizzled);
                }
            }
            ActionKind::Rescue => {
                // The 2-blood cost is paid at resolution; blood spent during
                // the action (an as-announced modifier) can leave it
                // unpayable, in which case the action fizzles (rules-qa
                // 3.6): successful, no effect.
                let acting = self.state.minions[a.acting.index()].blood;
                let target = a.torpor_target.expect("target");
                // The target no longer in play: the rescuer pays what it can
                // and the action fizzles (ANK 20220218: a fizzled action's
                // cost is paid; owner, 2026-09-26: rules-qa 28.7). Out of
                // torpor but in play, it pays as normal and fizzles too
                // (`rescue_apply`).
                if !self.state.minions[target.index()].present() {
                    let v = &mut self.state.minions[a.acting.index()];
                    v.blood -= v.blood.min(2);
                    return self.fizzle(a);
                }
                let t = self.state.minions[target.index()].blood;
                a.rescue_cost = self.rescue_cost_now(a.acting, target);
                a.rescue_gain = self.state.rescue_discount(a.acting, target).1;
                let payable = acting + t >= a.rescue_cost;
                if !payable {
                    a.outcome = Some(Outcome::Fizzled);
                }
                // The split chosen as it was announced; a share its payer
                // can no longer meet falls to the other (rules-qa 48.8).
                let declared = a.rescue_split.unwrap_or(a.rescue_cost);
                let from_acting = declared
                    .min(a.rescue_cost)
                    .min(acting)
                    .max(a.rescue_cost.saturating_sub(t));
                self.action = Some(a);
                if payable {
                    self.rescue_apply(from_acting);
                } else {
                    self.end_action();
                }
                return;
            }
            ActionKind::Diablerie => {
                let victim = a.torpor_target.expect("diablerie target");
                let diablerist = a.acting;
                // Out of torpor before it resolves (Warsaw Station): the
                // action is successful with no effect — no diablerie (ANK
                // 20220218, 20181107).
                let v = &self.state.minions[victim.index()];
                if !(v.present() && v.in_torpor) {
                    return self.fizzle(a);
                }
                self.action = Some(a);
                self.diablerise(diablerist, victim);
                self.end_action_into_blood_hunt(diablerist, victim);
                return;
            }
            ActionKind::Employ | ActionKind::Recruit => {
                // Cost is paid at resolution (spec §1.7.1); unpayable by
                // then, the action fizzles (rules-qa 3.6) and the card is
                // discarded; paid down to 0 pool, the Methuselah is ousted.
                let play = a.card.expect("action card");
                let acting = a.acting;
                let mut pool =
                    cards::pool_cost(&self.state, seat, play.card, Target::Minion(acting));
                let mut blood = cards::blood_cost(&self.state, play.card, play.x)
                    + self.card_surcharge(acting, play.card, play.mode);
                // The recruiting minion's printed reductions (Kuyén, Máddji).
                self.state
                    .printed_cost_off(acting, play.card, play.mode)
                    .apply(&mut pool, &mut blood);
                let once = self.once_off_amount(&play, blood);
                blood -= once;
                // "Recruit ally actions cost this vampire -1 blood or pool"
                // (Charisma): the action's only, not a recruit outside an
                // action (LSJ 20090115-1).
                if a.kind == ActionKind::Recruit {
                    let d = self.state.recruit_discount(acting);
                    crate::plays::reduce(&mut pool, &mut blood, d);
                }
                // The reductions: "by n blood or pool" off the card's cost in
                // whichever it has; blood-only ones; counters.
                let (pool, blood, used) = self.reduced_cost_with(&a, pool, blood);
                if self.state.seats[seat.index()].pool < pool
                    || self.state.minions[acting.index()].blood < blood
                {
                    return self.fizzle(a);
                }
                // Paid: Nergal's reduction spent (unpayable, it is not, LSJ
                // 20091021-2).
                self.spend_once_cost_off(&play, once);
                self.spend_counters(&used);
                self.state.seats[seat.index()].pool -= pool;
                self.state.minions[acting.index()].blood -= blood;
                match a.kind {
                    ActionKind::Recruit => {
                        self.recruit_ally(seat, play.card, play.mode);
                        // "Unlock this vampire if this is their first
                        // successful recruit ally action this turn"
                        // (Spectral Servitor).
                        let first = self.state.minions[acting.index()].recruits_this_turn == 0;
                        let unlocks = cards::lookup(play.card).is_some_and(|d| {
                            d.modes[play.mode as usize]
                                .effects
                                .contains(&Effect::UnlockRecruiterOnFirst)
                        });
                        if first && unlocks && !a.no_unlock {
                            self.state.unlock(acting);
                        }
                        self.state.minions[acting.index()].recruits_this_turn += 1;
                    }
                    _ => self.put_retainer(seat, acting, play, false),
                }
                // An ally paying its last life "as a vampire": the action
                // resolved fully, then it burns (LSJ 20080512).
                if blood > 0
                    && self.state.minions[acting.index()].is_ally()
                    && self.state.minions[acting.index()].blood == 0
                {
                    self.burn_minion(acting);
                }
                a.card_entered_play = true;
                if pool > 0 && self.state.seats[seat.index()].pool == 0 {
                    self.action = Some(a);
                    if self.oust_and_pass(seat) {
                        return;
                    }
                    a = self.action.take().expect("action");
                }
            }
            ActionKind::Equip => {
                // Rulebook "Equip": the card goes on the acting minion, its
                // cost paid now (spec §1.7.1). Unpayable by then, or no
                // longer holdable ("a minion can have only one vehicle"),
                // the action fizzles (rules-qa 3.6) and the card is
                // discarded; paid down to 0 pool, the Methuselah is ousted.
                let play = a.card.expect("equipment card");
                let acting = a.acting;
                let mut pool =
                    cards::pool_cost(&self.state, seat, play.card, Target::Minion(acting));
                let mut blood = cards::blood_cost(&self.state, play.card, play.x)
                    + self.card_surcharge(acting, play.card, play.mode);
                // "-1 blood or pool" (British Museum): off whichever the
                // card costs; then the reductions announced for the action.
                let discount = self.state.equip_discount(acting, play.card);
                crate::plays::reduce(&mut pool, &mut blood, discount);
                // The equipping minion's printed ones (Anne-Marie Bourgeois).
                self.state
                    .printed_cost_off(acting, play.card, play.mode)
                    .apply(&mut pool, &mut blood);
                let once = self.once_off_amount(&play, blood);
                blood -= once;
                let (mut pool, mut blood, used) = self.reduced_cost_with(&a, pool, blood);
                let paid_by = self.blood_payment(a.blood_payer, &mut pool, &mut blood);
                if self.state.seats[seat.index()].pool < pool
                    || self.state.minions[acting.index()].blood < blood
                    || !self.state.can_hold(acting, play.card, &[])
                {
                    a.outcome = Some(Outcome::Fizzled);
                } else {
                    self.spend_once_cost_off(&play, once);
                    self.spend_counters(&used);
                    if let Some(r) = paid_by {
                        self.state.minions[r.index()].blood -= 1;
                    }
                    self.state.seats[seat.index()].pool -= pool;
                    self.state.minions[acting.index()].blood -= blood;
                    self.put_equipment(seat, acting, play);
                    a.card_entered_play = true;
                    if pool > 0 && self.state.seats[seat.index()].pool == 0 {
                        self.action = Some(a);
                        if self.oust_and_pass(seat) {
                            return;
                        }
                        a = self.action.take().expect("action");
                    }
                }
            }
            ActionKind::EquipFound => {
                // Vast Wealth: fizzles if the card left the acting minion
                // (as the Codex's bleed). Working down from the top, the
                // first equipment is equipped as normal — or burned without
                // cost if it cannot be placed or its blood paid (TOM
                // 19950407, rules-qa 9.3), its pool paid even if that ousts
                // (RTR 19941109); the library is shuffled afterward.
                let src = a.source.expect("source").card();
                let acting = a.acting;
                let c = &self.state.cards[src.index()];
                if !(c.present() && c.host == Some(acting)) {
                    return self.fizzle(a);
                }
                // Payable counting the reductions it may take, offered
                // once found (rules-qa 38.5).
                let library = &self.state.seats[seat.index()].library;
                let found = library.iter().rposition(|&c| is_equipment(c));
                let found = found.map(|k| self.state.seats[seat.index()].library.remove(k));
                self.rng
                    .shuffle(&mut self.state.seats[seat.index()].library);
                if let Some(card) = found {
                    let ctx = found_play(seat, acting, card);
                    if self.equip_requirements_met(acting, card)
                        && self.payable_with_potential(&ctx, true)
                    {
                        self.action = Some(a);
                        if !self.cost_choices(&ctx) {
                            self.equip_found(ctx);
                        } else {
                            self.flow = Flow::CostTerms {
                                ctx,
                                resume: Box::new(Flow::GameOver),
                                pending: None,
                                then: crate::plays::CostThen::EquipFound,
                            };
                        }
                        return;
                    }
                    self.state.seats[seat.index()].ash.push(card);
                }
            }
            ActionKind::EquipFrom => {
                // The equipment still on the minion it was announced from
                // moves to the acting minion; one it could no longer hold
                // stays where it is.
                let acting = a.acting;
                for &id in &a.taken {
                    let c = &self.state.cards[id.index()];
                    if c.present() && c.host == a.giver && self.state.can_hold(acting, c.card, &[])
                    {
                        self.state.cards[id.index()].host = Some(acting);
                    }
                }
            }
            ActionKind::Card => {
                // The card's effects resolve now; its cost (X read now,
                // spec §1.7.1) unpayable by resolution fizzles (rules-qa
                // 3.6).
                let play = a.card.expect("action card");
                let def = cards::lookup(play.card).expect("definition");
                let mode = &def.modes[play.mode as usize];
                let x = cards::terms_x(&self.state, seat, play.card, mode).unwrap_or(play.x);
                let mut pool = cards::pool_cost(&self.state, seat, play.card, play.target);
                let mut blood = cards::blood_cost(&self.state, play.card, x)
                    + self.card_surcharge(a.acting, play.card, play.mode);
                self.state
                    .printed_cost_off(a.acting, play.card, play.mode)
                    .apply(&mut pool, &mut blood);
                let once = self.once_off_amount(&play, blood);
                blood -= once;
                let (pool, cost, used) = self.reduced_cost_with(&a, pool, blood);
                if self.state.minions[a.acting.index()].blood < cost
                    || self.state.seats[seat.index()].pool < pool
                {
                    return self.fizzle(a);
                }
                self.spend_once_cost_off(&play, once);
                self.spend_counters(&used);
                self.state.minions[a.acting.index()].blood -= cost;
                // Its pool cost too (Biothaumaturgic Experiment): paid down
                // to 0, the acting Methuselah is ousted and the turn passes.
                self.state.seats[seat.index()].pool -= pool;
                if pool > 0 && self.state.seats[seat.index()].pool == 0 {
                    self.action = Some(a);
                    if self.oust_and_pass(seat) {
                        return;
                    }
                    a = self.action.take().expect("action");
                }
                if mode.effects.contains(&Effect::SearchEquip) {
                    // Magic of the Smith: the equipment is chosen now that
                    // the action succeeded (spec §1.14.1, §3.4).
                    let acting = a.acting;
                    self.action = Some(a);
                    self.flow = Flow::LibrarySearch {
                        minion: acting,
                        to_hand: None,
                    };
                    return;
                }
                let ctx = PlayCtx { x, ..play };
                let mut in_play = None;
                // A target no longer legal (burned, taken back, no longer
                // the predator's): the action fizzles — successful, no
                // effect, paid (LSJ 20020725, PIB 20121031).
                let target_ok = match (mode.target, ctx.target) {
                    (Some(spec), Target::Minion(m)) => {
                        self.state.minions[m.index()].present()
                            && spec.filter.iter().all(|&r| {
                                cards::holds_for_minion(
                                    r,
                                    &self.state,
                                    seat,
                                    None,
                                    Some(a.acting),
                                    m,
                                    None,
                                )
                            })
                    }
                    (Some(spec), Target::Card(c)) => {
                        self.card_target_qualifies(seat, spec, Some(a.acting), c)
                    }
                    (Some(spec), Target::Uncontrolled(i)) => {
                        self.uncontrolled_target_qualifies(seat, spec, Some(a.acting), i as usize)
                    }
                    (Some(spec), Target::Seat(t)) => spec
                        .filter
                        .iter()
                        .all(|&r| cards::holds_for_seat(r, &self.state, seat, Some(&a), t)),
                    _ => true,
                };
                if !target_ok {
                    a.outcome = Some(Outcome::Fizzled);
                }
                // Declaration-time effects (the kind, the printed stealth)
                // were read at announcement; the layers of a card put on its
                // vampire after the combat wait for it (Pillars Fall).
                let later = mode
                    .effects
                    .contains(&Effect::PutOnActorIfOpposingNotReadyAfterCombat);
                for e in mode.effects.iter().filter(|_| target_ok).filter(|e| {
                    !(later && matches!(e, Effect::Layer(_)))
                        && !matches!(
                            e,
                            Effect::DeclareAction(_)
                                | Effect::AddStealth(_)
                                | Effect::StealthForClan { .. }
                                | Effect::StealthForTitled(_)
                                | Effect::IfBlocks(_)
                                | Effect::EnterCombatWithTarget
                                | Effect::InResultingCombat(_)
                                | Effect::TargetActsInCombat
                                | Effect::Frenzy
                                | Effect::StillYours
                                | Effect::TitledCannotBlock
                                | Effect::UnlockActor
                                | Effect::HuntAction
                                | Effect::HuntAt(_)
                                | Effect::BloodHuntIfHuntGained
                        )
                }) {
                    self.apply_card_effect(*e, &ctx, &mut in_play);
                }
                // "Hunt at +n hunt" (Abactor): 1 + n blood, hunt bonuses
                // included (rules-qa 59.1); none gained (at capacity, a
                // disease) is no successful hunt.
                if let Some(n) = mode.effects.iter().find_map(|e| match e {
                    Effect::HuntAt(n) => Some(*n),
                    _ => None,
                }) {
                    let acting = a.acting;
                    let before = self.state.minions[acting.index()].blood;
                    let n = 1 + n + self.state.hunt_bonus(acting);
                    self.state.hunt_blood(acting, n);
                    a.hunt_gained = self.state.minions[acting.index()].blood > before;
                    if a.hunt_gained {
                        self.action = Some(a);
                        self.successful_hunt_riders(acting);
                        a = self.action.take().expect("action context");
                    }
                }
                // The effects ousted the acting Methuselah (Rewilding on
                // their own location): the turn passes with the action.
                if self.state.seats[seat.index()].ousted {
                    self.action = Some(a);
                    if self.state.living() > 1 {
                        self.next_turn();
                    }
                    return;
                }
                // "Enter combat with a minion": both still ready, the
                // combat the resolution; the action succeeds after it
                // (rules-qa 17.3).
                if target_ok && mode.effects.contains(&Effect::EnterCombatWithTarget) {
                    let t = ctx.target.minion();
                    if self.state.minions[a.acting.index()].ready() {
                        let acting = a.acting;
                        let riders: Vec<(MinionId, cards::BlockRider)> = mode
                            .effects
                            .iter()
                            .filter_map(|e| match e {
                                Effect::InResultingCombat(r) => Some((acting, *r)),
                                _ => None,
                            })
                            .collect();
                        self.action = Some(a);
                        if mode.effects.contains(&Effect::TargetActsInCombat) {
                            // Deep Song [ANI]: locked as the combat begins
                            // (ANK 20200702), and "considered the acting
                            // minion during that combat".
                            self.state.minions[t.index()].locked = true;
                            self.start_combat_at_resolution(t, acting);
                        } else {
                            self.start_combat_at_resolution(acting, t);
                        }
                        // At the end of that combat: Abuse of Power's pool
                        // burn, Pillars Fall put on the acting vampire.
                        let c = self.combat.as_mut().expect("combat");
                        for e in mode.effects {
                            match *e {
                                Effect::OpposingBurnsPoolIfOneReadyAfterCombat(n) => {
                                    c.one_ready_burn = Some(n);
                                }
                                Effect::PutOnActorIfOpposingNotReadyAfterCombat => {
                                    c.put_if_opposing_down =
                                        Some(crate::combat::Held::from_play(&ctx, acting, 0));
                                }
                                _ => {}
                            }
                        }
                        self.combat_riders(&riders);
                        return;
                    }
                }
                // A hunt action card is the hunt a vampire must take.
                if a.is_hunt {
                    self.state.minions[a.acting.index()].must_hunt = false;
                }
                // "Unlock them" (Khabar: Glory): after the card's effects.
                if target_ok
                    && mode.effects.contains(&Effect::UnlockActor)
                    && !a.no_unlock
                    && self.state.minions[a.acting.index()].present()
                {
                    self.state.unlock(a.acting);
                }
                a.card_entered_play = in_play.is_some();
                if let Some(id) = in_play {
                    self.enter_library_contest(id);
                }
                // Abactor: "a referendum is called to call a blood hunt on
                // this vampire (independent of the action, just as if they
                // had committed diablerie)" — after a hunt that gained blood
                // (LSJ 20090411), part of the action's resolution (LSJ
                // 20100112); no diablerie (LSJ 20090724).
                if mode.effects.contains(&Effect::BloodHuntIfHuntGained)
                    && a.hunt_gained
                    && self.state.minions[a.acting.index()].present()
                    && !self.state.blood_hunt_barred(a.acting)
                {
                    let acting = a.acting;
                    self.action = Some(a);
                    self.start_referendum(RefKind::BloodHunt { diablerist: acting }, seat, None);
                    return;
                }
            }
            ActionKind::BurnCard => {
                // The granted action's pool cost is paid at resolution
                // ("as a Ⓓ action that costs 1 pool"); unpayable by then
                // it fizzles (rules-qa 3.6); paid down to 0, the acting
                // Methuselah is ousted and the turn passes.
                // "As a Ⓓ action that costs 1 blood" (Powerbase: Munich).
                let (cost, blood) = self.granted_cost(&a);
                let src = a.source.expect("source").card();
                if self.state.seats[seat.index()].pool < cost as i32
                    || self.state.minions[a.acting.index()].blood < blood
                {
                    a.outcome = Some(Outcome::Fizzled);
                } else {
                    self.state.minions[a.acting.index()].blood -= blood;
                    self.state.seats[seat.index()].pool -= cost as i32;
                    let host = self.state.cards[src.index()].host;
                    let effects = self.granted_effects(&a);
                    if effects.contains(&Effect::ShuffleSourceIntoLibrary) {
                        self.shuffle_into_library(src);
                    } else {
                        self.burn_card_in_play(src);
                    }
                    for e in effects {
                        if let (Effect::LockHostSkipNextUnlock(who), Some(h)) = (e, host) {
                            // Stolen Police Cruiser: "this Anarch is locked
                            // and does not unlock as normal during their
                            // next unlock phase".
                            if self.state.minions[h.index()].present() && who.is(&self.state, h) {
                                let v = &mut self.state.minions[h.index()];
                                v.locked = true;
                                v.skip_unlock = true;
                            }
                        }
                    }
                    if self.state.seats[seat.index()].pool == 0 {
                        self.action = Some(a);
                        if self.oust_and_pass(seat) {
                            return;
                        }
                        a = self.action.take().expect("action");
                    }
                }
            }
            // Still in play, the card is the acting Methuselah's.
            ActionKind::StealCard => {
                let src = a.source.expect("source").card();
                if self.state.cards[src.index()].present() {
                    self.give_card_control(src, seat);
                } else {
                    a.outcome = Some(Outcome::Fizzled);
                }
            }
            ActionKind::Ability => {
                // The printed cost is paid at resolution; unpayable by then,
                // the action fizzles (rules-qa 3.6). Paid, a target that no
                // longer qualifies leaves it successful with no effect
                // (spec §3.4.1, Nathaniel Bordruff ANK 20210124).
                let effects = self.granted_effects(&a);
                let (blood, pool) = effects.iter().fold((0u8, 0u8), |(b, p), e| match e {
                    // An ally's action "that costs 1 life" (Rotting
                    // Behemoth): its life is its blood.
                    Effect::PayBlood(n) | Effect::PayLife(n) => (b + n, p),
                    Effect::PayPool(n) => (b, p + n),
                    _ => (b, p),
                });
                let acting = a.acting;
                if self.state.minions[acting.index()].blood < blood
                    || self.state.seats[seat.index()].pool < pool as i32
                {
                    return self.fizzle(a);
                }
                self.state.minions[acting.index()].blood -= blood;
                // An ally paying its last life burns: the action still
                // resolves, a combat it would enter ends at once (LSJ
                // 20080512).
                if blood > 0
                    && self.state.minions[acting.index()].is_ally()
                    && self.state.minions[acting.index()].blood == 0
                {
                    self.burn_minion(acting);
                }
                self.state.seats[seat.index()].pool -= pool as i32;
                if pool > 0 && self.state.seats[seat.index()].pool == 0 {
                    self.action = Some(a);
                    if self.oust_and_pass(seat) {
                        return;
                    }
                    a = self.action.take().expect("action");
                }
                let src = a.source.expect("source");
                let spec = src.mode(&self.state, a.ability.expect("ability")).target;
                let qualifies = match (a.target, spec) {
                    // "Enter combat with this vampire": the card still on it
                    // (Haven Uncovered, Priority Contract).
                    (Target::Minion(t), Some(spec))
                        if spec.filter.contains(&cards::Requirement::TargetIsHost) =>
                    {
                        let host = match src {
                            cards::AbilitySource::Card(c) => self.state.cards[c.index()]
                                .present()
                                .then_some(self.state.cards[c.index()].host)
                                .flatten()
                                .or(a.burned_source_host),
                            cards::AbilitySource::Vampire(_) => None,
                        };
                        self.minion_target_qualifies(seat, spec, host, t)
                    }
                    (Target::Minion(t), Some(spec)) => {
                        self.minion_target_qualifies(seat, spec, Some(acting), t)
                    }
                    (Target::Card(c), Some(spec)) => {
                        self.card_target_qualifies(seat, spec, Some(acting), c)
                    }
                    _ => true,
                };
                if !qualifies {
                    return self.fizzle(a);
                }
                let (target, exchange) = (a.target, a.exchange);
                self.action = Some(a);
                for e in effects {
                    match *e {
                        Effect::DeclareAction(_)
                        | Effect::AddStealth(_)
                        | Effect::BurnSourceOnDeclare
                        | Effect::MandatoryAction
                        | Effect::PayBlood(_)
                        | Effect::PayLife(_)
                        | Effect::PayPool(_)
                        | Effect::SameActionAs(_)
                        | Effect::LockSelf
                        | Effect::HuntAction => {}
                        // "Add 1 counter to this card" (The Gate of
                        // Acheron): the card granting the action, still in
                        // play.
                        Effect::AddCounter(n) => {
                            let c = src.card();
                            if self.state.cards[c.index()].present() {
                                self.state.cards[c.index()].counters += n;
                            }
                        }
                        Effect::GreatBeastAtEleven => {
                            let c = src.card();
                            self.great_beast_at_eleven(c);
                        }
                        Effect::RemoveAllSourceCounters => {
                            let c = src.card();
                            if self.state.cards[c.index()].present() {
                                self.state.cards[c.index()].counters = 0;
                            }
                        }
                        Effect::SearchToHand(filter) => {
                            self.flow = Flow::LibrarySearch {
                                minion: acting,
                                to_hand: Some(filter),
                            };
                            return;
                        }
                        // The named cards, if both are still where they were.
                        Effect::ExchangeHandForAsh => {
                            let (ash, held) = exchange.expect("named cards");
                            let s = &mut self.state.seats[seat.index()];
                            let from_ash = s.ash.iter().position(|&c| c == ash);
                            let from_hand = s.hand.iter().position(|&c| c == held);
                            if let (Some(i), Some(j)) = (from_ash, from_hand) {
                                s.ash.remove(i);
                                s.hand.remove(j);
                                s.ash.push(held);
                                s.hand.push(ash);
                            }
                        }
                        Effect::BloodToTarget(n) => {
                            let t = target.minion();
                            self.state.gain_blood(t, n);
                        }
                        Effect::BloodToTargetToStart(n) => {
                            let t = target.minion();
                            let b = self.state.cap_starting(
                                t,
                                self.state.minions[t.index()].blood.saturating_add(n),
                            );
                            self.state.minions[t.index()].blood = b;
                        }
                        // The ally named as the action was declared, if still
                        // there; gone: no effect (as Psychophagia's, rules-qa
                        // 32.12).
                        Effect::AllyFromAshLocked(_) => {
                            let named = self.action.as_ref().and_then(|a| a.ash_named);
                            if let Some(card) =
                                named.filter(|c| self.state.seats[seat.index()].ash.contains(c))
                            {
                                self.raise_ash_ally(seat, card, true);
                            }
                        }
                        // Both still ready (the target was re-checked above).
                        Effect::EnterCombatWithTarget => {
                            let t = target.minion();
                            if self.state.minions[acting.index()].ready() {
                                self.start_combat_at_resolution(acting, t);
                                return;
                            }
                        }
                        // Still one it could hold (re-checked above).
                        Effect::StealTargetCard => {
                            let c = target.card();
                            self.state.cards[c.index()].host = Some(acting);
                        }
                        // Still contested (re-checked above).
                        Effect::TargetYieldsTitle => {
                            let t = target.minion();
                            self.yield_title(t);
                        }
                        // The card still in play, the target still a
                        // vampire in play (re-checked above).
                        Effect::MoveSourceToTarget => {
                            let c = src.card();
                            if self.state.cards[c.index()].present() {
                                self.state.cards[c.index()].host = Some(target.minion());
                            }
                        }
                        Effect::BurnTargetCard => {
                            let c = target.card();
                            self.burn_card_in_play(c);
                        }
                        // Rise of the Nephtali: no contest with a younger
                        // vampire — the younger one yields at once.
                        Effect::GainLiaison => {
                            let t = crate::crypt::Title::Independent {
                                name: "Liaison",
                                votes: 4,
                            };
                            let holder = (0..self.state.minions.len())
                                .map(|i| MinionId(i as u32))
                                .find(|&h| {
                                    h != acting
                                        && self.state.minions[h.index()].in_play
                                        && self.state.claimed_title(h) == t
                                });
                            let gains = match holder {
                                Some(h) if self.state.younger_than(h, acting) => {
                                    self.state.minions[h.index()].granted_title = None;
                                    true
                                }
                                Some(_) => false,
                                None => true,
                            };
                            if gains && self.state.minions[acting.index()].in_play {
                                self.gain_title(acting, t, None);
                                self.release_held(|h| &mut h.liaison);
                            }
                        }
                        // Still in torpor (re-checked above): ECTU
                        // Operative's burn.
                        Effect::BurnTargetMinion => {
                            let t = target.minion();
                            self.burn_minion(t);
                        }
                        // Veneficti's demon ally (none in the pool).
                        Effect::StealTargetAlly => {
                            let t = target.minion();
                            self.state.minions[t.index()].owner = seat;
                            self.enter_card_contest(t);
                        }
                        // Nephandus: "to gain 1 life", over his starting
                        // life too (ANK 20171109).
                        Effect::AddHostBlood(n) => {
                            self.state.minions[acting.index()].blood += n;
                        }
                        // The Sargon Fragment: any copy of the named card
                        // (LSJ 20050228), the Fragment still on the acting
                        // vampire — else no effect (RTR 19960221).
                        Effect::AshCardToHandNamed => {
                            let named = self.action.as_ref().and_then(|a| a.ash_named);
                            let c = src.card();
                            let on = self.state.cards[c.index()].present()
                                && self.state.cards[c.index()].host == Some(acting);
                            let s = &mut self.state.seats[seat.index()];
                            if let (true, Some(i)) =
                                (on, named.and_then(|n| s.ash.iter().position(|&x| x == n)))
                            {
                                let card = s.ash.remove(i);
                                s.hand.push(card);
                            }
                        }
                        // Malgorzata: both vampires still where they were
                        // named.
                        Effect::RemoveUncontrolledForBlood(n) => {
                            let named = self.action.as_ref().and_then(|a| a.uncontrolled_named);
                            let slots = &self.state.seats[seat.index()].uncontrolled;
                            if let (Some((i, card)), Target::Uncontrolled(t)) = (named, target) {
                                let (i, t) = (i as usize, t as usize);
                                if i < slots.len() && t < slots.len() && slots[i].card == card {
                                    let s = &mut self.state.seats[seat.index()];
                                    let gone = s.uncontrolled.remove(i);
                                    s.removed_crypt.push(gone.card);
                                    let t = if t > i { t - 1 } else { t };
                                    self.state.add_uncontrolled_blood(seat, t, n);
                                }
                            }
                        }
                        // Mary Anne Blaire: the vampire named, if still
                        // there.
                        Effect::BloodToUncontrolledTarget(n) => {
                            if let Target::Uncontrolled(i) = target {
                                if (i as usize) < self.state.seats[seat.index()].uncontrolled.len()
                                {
                                    self.state.add_uncontrolled_blood(seat, i as usize, n);
                                }
                            }
                        }
                        Effect::PreyAshPairHunt(bonus) => {
                            let prey = self.state.prey(seat);
                            let named = self.action.as_ref().and_then(|a| a.ash_named);
                            let ps = &mut self.state.seats[prey.index()];
                            let pair =
                                named.filter(|&c| ps.ash.iter().filter(|&&x| x == c).count() >= 2);
                            let Some(card) = pair else {
                                let a = self.action.take().expect("action context");
                                return self.fizzle(a);
                            };
                            for _ in 0..2 {
                                let i = ps.ash.iter().position(|&x| x == card).expect("copy");
                                ps.ash.remove(i);
                                ps.removed.push(card);
                            }
                            let before = self.state.minions[acting.index()].blood;
                            let n = 1 + bonus + self.state.hunt_bonus(acting);
                            self.state.hunt_blood(acting, n);
                            let gained = self.state.minions[acting.index()].blood > before;
                            if let Some(a) = self.action.as_mut() {
                                a.hunt_gained = gained;
                            }
                            if gained {
                                self.successful_hunt_riders(acting);
                            }
                        }
                        // Still ready (re-checked above): Mylan Horseed's
                        // unlock.
                        Effect::UnlockTarget => {
                            let t = target.minion();
                            self.effect_unlock(t);
                        }
                        // Victoria Ash: a locked target stays locked (a
                        // no-op, rules-qa 57.10).
                        Effect::LockTarget => {
                            let t = target.minion();
                            self.state.minions[t.index()].locked = true;
                        }
                        // Heartrender: the target burned, the ally out of
                        // the game (rules-qa 55.4).
                        Effect::RemoveActorToBurnTarget => {
                            match target {
                                Target::Minion(t) => self.burn_minion(t),
                                Target::Card(c) => self.burn_card_in_play(c),
                                _ => {}
                            }
                            if self.state.minions[acting.index()].in_play {
                                self.remove_ally_from_game(acting);
                            }
                        }
                        // Michael Harris: the equipment is chosen now that
                        // the action succeeded (as Magic of the Smith).
                        // Eulogio: the top cards reordered, then unlocked.
                        Effect::ReorderTopThenUnlock(n) => {
                            self.effect_unlock(acting);
                            let total =
                                (n as usize).min(self.state.seats[seat.index()].library.len());
                            if total > 1 {
                                self.interrupts.push(crate::draws::Interrupt::Reorder {
                                    seat,
                                    total: total as u8,
                                    placed: 0,
                                });
                            }
                        }
                        Effect::SearchEquip => {
                            self.flow = Flow::LibrarySearch {
                                minion: acting,
                                to_hand: None,
                            };
                            return;
                        }
                        // Whose counter, if several Methuselahs' are on it:
                        // the acting Methuselah's choice.
                        Effect::BurnCorruptionFromTarget => {
                            let t = target.minion();
                            let v = &mut self.state.minions[t.index()];
                            if let [(owner, _)] = v.corruption[..] {
                                v.burn_corruption(owner, 1);
                            } else {
                                self.interrupts
                                    .push(crate::draws::Interrupt::CorruptionOwner {
                                        seat,
                                        minion: t,
                                    });
                            }
                        }
                        // Still a prince (re-checked above).
                        Effect::StealTargetTitle => {
                            let t = target.minion();
                            self.steal_title(acting, t);
                        }
                        Effect::StealBloodFromTarget(n) => {
                            let t = target.minion();
                            // A hunt bonus is more blood from the target
                            // (The Hungry Coyote; RTR 20030519).
                            let hunt = self.action.as_ref().is_some_and(|a| a.is_hunt);
                            let n = n + if hunt {
                                self.state.hunt_bonus(acting)
                            } else {
                                0
                            };
                            // Diseased: no blood to gain, none taken.
                            let n = if hunt && self.state.minions[acting.index()].disease {
                                0
                            } else {
                                n
                            };
                            let taken = n.min(self.state.minions[t.index()].blood);
                            self.state.minions[t.index()].blood -= taken;
                            self.state.gain_blood(acting, taken);
                            // A hunt stealing no blood is not successful (LSJ
                            // 20050720-2); blood it cannot hold goes to the
                            // bank (LSJ 20010809-1).
                            if hunt && taken > 0 {
                                self.action.as_mut().expect("action").hunt_gained = true;
                                self.successful_hunt_riders(acting);
                            }
                        }
                        // François Villon (G2): how much, chosen now.
                        Effect::StealBloodUpTo(n) => {
                            let t = target.minion();
                            let max = n.min(self.state.minions[t.index()].blood);
                            if max <= 1 {
                                self.state.minions[t.index()].blood -= max;
                                self.state.gain_blood(acting, max);
                            } else {
                                self.interrupts.push(crate::draws::Interrupt::StealUpTo {
                                    seat,
                                    from: t,
                                    to: acting,
                                    max,
                                });
                            }
                        }
                        // Cave of Apples: a corruption counter; enough of the
                        // player's on it, they may burn them to steal it.
                        Effect::CorruptTargetMayStealAtCapacity => {
                            let t = target.minion();
                            self.state.minions[t.index()].add_corruption(seat, 1);
                            if self.corruption_steal_reached(seat, t) {
                                self.interrupts
                                    .push(crate::draws::Interrupt::StealForCorruption {
                                        seat,
                                        minion: t,
                                    });
                            }
                        }
                        other => unreachable!("not a granted action's effect: {other:?}"),
                    }
                }
                self.after_resolution();
                return;
            }
            ActionKind::Political => {
                // Rulebook "The Referendum": the cost is paid and the
                // referendum called — only if the acting minion is still
                // ready (spec §3.7.5.1); NRA on resolution (rules-qa 1.4).
                // The referendum and cost come from the political action
                // card, or from the card in play granting the action (no
                // card-play cost, spec §1.5).
                let (mode, cost) = match (a.card, a.source) {
                    (Some(play), _) => {
                        let def = cards::lookup(play.card).expect("definition");
                        (
                            &def.modes[play.mode as usize],
                            self.card_blood(a.acting, play, play.x),
                        )
                    }
                    (None, Some(src)) => {
                        let ability = a.ability.expect("granted action's ability");
                        (src.mode(&self.state, ability), 0)
                    }
                    (None, None) => unreachable!("political action without a source"),
                };
                let (_, cost, used) = self.reduced_cost_with(&a, 0, cost);
                let m = &mut self.state.minions[a.acting.index()];
                m.political_this_turn = true;
                if !a.cost_paid_early {
                    if m.blood < cost {
                        // Unpayable by resolution: fizzles (rules-qa 3.6),
                        // no referendum.
                        return self.fizzle(a);
                    }
                    m.blood -= cost;
                    self.spend_counters(&used);
                }
                let referendum = mode
                    .effects
                    .iter()
                    .find_map(|e| match e {
                        Effect::CallReferendum(r) => Some(*r),
                        _ => None,
                    })
                    .expect("political action card calls a referendum");
                let ready = self.state.minions[a.acting.index()].ready();
                self.action = Some(a);
                if ready {
                    self.start_referendum(RefKind::Political, seat, Some(referendum));
                } else {
                    self.after_resolution();
                }
                return;
            }
        }
        self.action = Some(a);
        self.after_resolution();
    }

    /// Riders of a vampire that successfully hunted (Adisa), and of the
    /// cards that became another on it ("after this vampire successfully
    /// hunts, burn this card", Enhanced Coagulant — not while it is still
    /// equipment).
    pub(crate) fn successful_hunt_riders(&mut self, m: MinionId) {
        // "Do not replace until a vampire successfully hunts" (Restricted
        // Vitae).
        self.release_held(|h| &mut h.hunt);
        let on: Vec<CardId> = (0..self.state.cards.len())
            .map(|i| CardId(i as u32))
            .filter(|&id| {
                let c = &self.state.cards[id.index()];
                c.present()
                    && c.host == Some(m)
                    && cards::became(c.card, c.mode)
                    && cards::lookup(c.card).is_some_and(|d| {
                        d.riders.iter().any(|r| {
                            r.hook == Hook::SuccessfulHunt && r.effects == [Effect::BurnSelf]
                        })
                    })
            })
            .collect();
        for id in on {
            self.burn_card_in_play(id);
        }
        let Some(def) = cards::vampire_of(&self.state.minions[m.index()]) else {
            return;
        };
        for r in def.riders.iter().filter(|r| r.hook == Hook::SuccessfulHunt) {
            for e in r.effects {
                match *e {
                    Effect::SkipNextUnlock => self.state.minions[m.index()].skip_unlock = true,
                    other => unreachable!("not a hunt rider effect: {other:?}"),
                }
            }
        }
    }

    /// Bookkeeping at the end of an action, successful or not: action-scoped
    /// layers pop, deferred replacements are drawn, the action card is
    /// discarded if it did not enter play, NRA identity is recorded.
    fn finish_action(&mut self) {
        // The hands looked at during the action (SchreckNET).
        self.state.peeks.clear();
        // Forward Momentum's pending counters.
        let momentum = std::mem::take(&mut self.action.as_mut().expect("action context").momentum);
        for (id, gain) in momentum {
            let c = &mut self.state.cards[id.index()];
            if gain {
                c.counters += 1;
            } else {
                c.counters = c.counters.saturating_sub(1);
            }
        }
        // Eyes of the Wild: locked after the action unless it blocked it.
        let a = self.action.as_ref().expect("action context");
        let lock: Vec<MinionId> = a
            .lock_if_no_block
            .iter()
            .copied()
            .filter(|m| !a.blocked_by.contains(m))
            .collect();
        for m in lock {
            if self.state.minions[m.index()].present() {
                self.state.minions[m.index()].locked = true;
            }
        }
        // Dogged Pursuit [ANI]: put on them after resolution if they did not
        // block ("a vampire can have only one").
        let a = self.action.as_ref().expect("action context");
        let put: Vec<crate::combat::Held> = a
            .put_if_no_block
            .iter()
            .copied()
            .filter(|h| !a.blocked_by.contains(&h.minion))
            .collect();
        for h in put {
            let name = LIBRARY[h.card as usize].name;
            if self.state.minions[h.minion.index()].present()
                && !self.state.holds_a(h.minion, cards::Holding::Named(name))
            {
                self.place_held(h, h.minion);
            }
        }
        // Fever Pitch [FOR]/[PRE]: "you can put this card on this vampire"
        // after a successful bleed, one per vampire — its player's choice
        // (owner, 2026-10-08: rules-qa 53.5).
        let a = self.action.as_ref().expect("action context");
        let put: Vec<crate::combat::Held> = if a.bleed_successful {
            a.put_if_bleed_successful.clone()
        } else {
            Vec::new()
        };
        for h in put {
            if self.may_put_held(h) {
                self.interrupts.push(crate::draws::Interrupt::MayPutOn {
                    seat: h.seat,
                    held: h,
                });
            }
        }
        let a = self.action.as_ref().expect("action context");
        let seat = a.seat;
        let acting = a.acting;
        if let Some(play) = a.card {
            // Entered play as it resolved: its own flag, never a same-named
            // card already on the minion (a second Raven Spy employed).
            if !a.card_entered_play
                && !a.card_returned
                && self.played_from_removing_store(play.from)
            {
                let owner = self.played_card_owner(seat, play.from);
                self.state.seats[owner.index()].removed.push(play.card);
            } else if cards::removed_once_played(play.card, play.mode) {
                self.state.seats[seat.index()].removed.push(play.card);
            } else if !a.card_entered_play && !a.card_returned {
                self.state.seats[seat.index()].ash.push(play.card);
            }
            // Canceled before resolution, the same action may be taken
            // again — not one blocked and continued before (ANK 20211015).
            if !a.canceled || !a.blocked_by.is_empty() {
                self.state.minions[acting.index()]
                    .acted
                    .push(ActionId::Card(play.card));
            }
        }
        if let (Some(src), Some(ab)) = (a.source, a.ability) {
            if !a.canceled || !a.blocked_by.is_empty() {
                let id = src.action_id(&self.state, ab);
                self.state.minions[acting.index()].acted.push(id);
            }
        }
        if !a.mandatory {
            self.state.optional_action = true;
        }
        // "This minion cannot perform the same action again this turn"
        // (Change of Target): a rulebook action, by kind and target.
        if a.ended_early && !a.canceled && a.card.is_none() && a.source.is_none() {
            let id = ActionId::Rulebook(a.kind, a.torpor_target.or(a.giver));
            self.state.minions[acting.index()].acted.push(id);
        }
        // Damage after action resolution (Daring the Dawn) not yet taken in
        // the after-resolution window: an action ending without passing
        // through it (a torpor-block diablerie's blood hunt ousting the
        // acting Methuselah) still reached resolution (ANK 20220331).
        let damage = a.damage_after;
        if damage > 0 && a.outcome.is_some() && self.state.minions[acting.index()].present() {
            self.damage_out_of_combat(acting, damage, true);
        }
        // "After a Toreador performs a successful action" (Loup): burned.
        let a = self.action.as_ref().expect("action context");
        let successful = matches!(a.outcome, Some(Outcome::Successful | Outcome::Fizzled));
        if let Some(clan) = self.state.minions[acting.index()]
            .clan()
            .filter(|_| successful)
        {
            for (id, _) in self.state.card_riders(Hook::ClanActionSuccessful(clan)) {
                if self.state.cards[id.index()].present() {
                    self.burn_card_in_play(id);
                }
            }
        }
        self.state.layers.retain(|l| l.scope != Scope::Action);
        self.release_action_holds();
    }

    /// Unpreventable damage outside combat (Daring the Dawn): blood burned,
    /// torpor, or burned outright (rulebook "Damage Resolution").
    pub(crate) fn damage_out_of_combat(&mut self, m: MinionId, n: u8, aggravated: bool) {
        let mut d = crate::combat::Damage::default();
        if aggravated {
            d.aggravated = n;
        } else {
            d.normal = n;
        }
        let v = &self.state.minions[m.index()];
        let r = if v.is_ally() {
            crate::combat::resolve_life_damage(v.blood, d)
        } else {
            crate::combat::resolve_damage(v.blood, d)
        };
        self.state.minions[m.index()].blood -= r.blood_burned;
        if r.burned {
            self.burn_minion(m);
        } else if r.wounded && !self.state.minions[m.index()].in_torpor {
            self.minions_to_torpor(&[m]);
        }
    }

    pub(crate) fn end_action(&mut self) {
        self.finish_action();
        self.last_outcome = self.action.as_ref().and_then(|a| a.outcome);
        self.action = None;
        // Back to the acting seat's minion phase, unless the action ended the
        // game (an oust that left one Methuselah).
        if self.state.living() > 1 {
            self.flow = Flow::MinionPhase;
        } else {
            self.flow = Flow::GameOver;
        }
    }

    /// The rescue's blood cost as things stand: 2, the torpid surcharge,
    /// less discounts.
    fn rescue_cost_now(&self, acting: MinionId, target: MinionId) -> u8 {
        let (discount, _) = self.state.rescue_discount(acting, target);
        (2 + self.state.torpid_surcharge(Some(acting), target)).saturating_sub(discount)
    }

    /// As the rescue is announced: the part of its cost the acting vampire
    /// pays (rules-qa 48.8) — the splits the blood allows now, or every
    /// split if none does (the action would fizzle unless blood comes).
    pub(crate) fn rescue_request(&self) -> DecisionRequest {
        let a = self.action.as_ref().expect("action context");
        let st = &self.state;
        let target = a.torpor_target.expect("target");
        let acting = st.minions[a.acting.index()].blood;
        let t = st.minions[target.index()].blood;
        let cost = self.rescue_cost_now(a.acting, target);
        let mut splits: Vec<u8> = (0..=cost)
            .filter(|&k| acting >= k && t >= cost - k)
            .collect();
        if splits.is_empty() {
            splits = (0..=cost).collect();
        }
        let options = splits
            .into_iter()
            .map(|k| Choice::PayRescue { from_acting: k })
            .collect();
        DecisionRequest::new(self.acting_seat(), WindowLabel::RescueCost, options)
    }

    /// The split chosen as the rescue is announced: the action goes on.
    pub(crate) fn rescue_declare(&mut self, from_acting: u8) {
        self.action.as_mut().expect("action context").rescue_split = Some(from_acting);
        self.flow = Flow::Action;
    }

    pub(crate) fn rescue_apply(&mut self, from_acting: u8) {
        let a = self.action.as_ref().expect("action context");
        let (acting, target) = (a.acting, a.torpor_target.expect("target"));
        let (cost, gain) = (a.rescue_cost, a.rescue_gain);
        self.state.minions[acting.index()].blood -= from_acting;
        self.state.minions[target.index()].blood -= cost - from_acting;
        if self.state.minions[target.index()].in_torpor {
            // Rescued: to the ready region, lock state untouched.
            self.leave_torpor(target);
            self.state.gain_blood(target, gain);
        } else {
            // Out of torpor already (Warsaw Station): paid, it fizzles
            // (rules-qa 28.7).
            self.action.as_mut().expect("action context").outcome = Some(Outcome::Fizzled);
        }
        self.after_resolution();
    }

    /// Put an equipment card from outside play on `acting`, its cost paid:
    /// its mode's own effects apply, and a unique enters contest.
    /// An ally enters play in `seat`'s ready region, unlocked, with its
    /// printed life from the blood bank (rulebook "Ally cards"): a minion
    /// and its card in play, linked. A recruited ally cannot act this turn
    /// (rulebook "Recruit Ally"). A unique ally another Methuselah has is
    /// contested (rulebook "Contested Cards").
    pub(crate) fn recruit_ally(&mut self, seat: SeatId, card: u16, mode: u8) -> MinionId {
        // The version's type line: printed, or its superior mode's ("[ANI]
        // As above, but this ally has 2 life and 1 strength").
        let body = cards::lookup(card)
            .and_then(|d| {
                d.modes[mode as usize].effects.iter().find_map(|e| match e {
                    Effect::Version(b) => Some(*b),
                    _ => None,
                })
            })
            .or(LIBRARY[card as usize].body)
            .expect("an ally's type line");
        // "Zombies you recruit get +1 starting life" (Ashur-uballit).
        let mut body = body;
        if LIBRARY[card as usize]
            .labels
            .contains(&crate::library::Label::Zombie)
        {
            body.life += self.state.zombie_life_bonus(seat);
        }
        let life = body.life;
        let m = MinionId(self.state.minions.len() as u32);
        let id = self.enter_play(CardInPlay {
            body: Some(m),
            ..CardInPlay::new(card, seat, None, mode)
        });
        self.state.minions.push(crate::state::Minion {
            recruited: true,
            ..crate::state::Minion::new(
                seat,
                crate::state::MinionKind::Ally { card, id, body },
                life,
            )
        });
        // Its text in play: layers on the ally itself (`CardInPlay::bearer`).
        let ctx = PlayCtx::new(seat, card, mode, Some(m), Target::Minion(m), 0);
        self.apply_entry_effects(&ctx, id, false);
        self.enter_card_contest(m);
        m
    }

    /// Put the master card `card` on `m` without playing it (no cost, no
    /// requirement, nothing to cancel), its first mode's effects applying
    /// as it enters play (Childe of the Revolution's Discipline card).
    pub(crate) fn put_on_minion(&mut self, seat: SeatId, card: u16, m: MinionId) {
        let def = cards::lookup(card).expect("a defined card");
        let ctx = PlayCtx::new(seat, card, 0, None, Target::Minion(m), 0);
        let mut in_play = None;
        for &e in def.modes[0].effects {
            self.apply_card_effect(e, &ctx, &mut in_play);
        }
    }

    /// A retainer enters play on the acting minion with the life its
    /// version has (rulebook "Employ Retainer"), its layers giving the
    /// employer what its text says.
    /// `outside_action`: employed by an effect, not an employ action (Pack
    /// Alpha): its "if the action to employ this retainer is successful"
    /// riders have no action to follow (Feral Hound).
    pub(crate) fn put_retainer(
        &mut self,
        seat: SeatId,
        acting: MinionId,
        play: PlayCtx,
        outside_action: bool,
    ) {
        let def = cards::lookup(play.card).expect("definition");
        let life = def.modes[play.mode as usize]
            .effects
            .iter()
            .find_map(|e| match e {
                Effect::EmployRetainer { life } => Some(*life),
                _ => None,
            })
            .expect("a retainer's life");
        // "Zombies … you employ get +1 starting life" (Ashur-uballit).
        let life = if LIBRARY[play.card as usize]
            .labels
            .contains(&crate::library::Label::Zombie)
        {
            life + self.state.zombie_life_bonus(seat)
        } else {
            life
        };
        let id = self.enter_play(CardInPlay {
            counters: life,
            ..CardInPlay::new(play.card, seat, Some(acting), play.mode)
        });
        let ctx = PlayCtx {
            target: Target::Minion(acting),
            ..play
        };
        self.apply_entry_effects(&ctx, id, outside_action);
        self.enter_library_contest(id);
    }

    /// The effects of a minion card entering play from its mode (layers),
    /// the action's own announcement effects aside.
    fn apply_entry_effects(&mut self, ctx: &PlayCtx, id: CardId, outside_action: bool) {
        let Some(def) = cards::lookup(ctx.card) else {
            return;
        };
        let mut in_play = Some(id);
        for e in def.modes[ctx.mode as usize].effects {
            match *e {
                Effect::DeclareAction(_)
                | Effect::AddStealth(_)
                | Effect::EmployRetainer { .. }
                | Effect::Version(_) => {}
                // No employ action to have succeeded.
                Effect::UnlockActor | Effect::UnlockActorAtDiscard if outside_action => {}
                // The acting minion, at the action's resolution (Feral
                // Hound).
                Effect::UnlockActor => {
                    self.state.unlock(ctx.actor.expect("actor"));
                }
                Effect::ActsWhenRecruited => {
                    self.state.minions[ctx.actor.expect("the entering ally").index()].recruited =
                        false;
                }
                Effect::UnlockRecruiterOnFirst => {}
                Effect::CannotGainLife => {
                    self.state.minions[ctx.actor.expect("the entering ally").index()]
                        .no_life_gain = true;
                }
                Effect::EntryRemoveFromAshOrBurn => {
                    self.interrupts
                        .push(crate::draws::Interrupt::EntryRemoveOrBurn {
                            seat: ctx.seat,
                            ally: ctx.actor.expect("the entering ally"),
                        });
                }
                Effect::EntryBurnAllyOrRetainer => {
                    self.interrupts.push(crate::draws::Interrupt::EntryBurn {
                        seat: ctx.seat,
                        ally: ctx.actor.expect("the entering ally"),
                    });
                }
                Effect::UnlockActorAtDiscard => {
                    self.state.minions[ctx.actor.expect("actor").index()].unlock_at_discard = true;
                }
                e => self.apply_card_effect(e, ctx, &mut in_play),
            }
        }
    }

    pub(crate) fn put_equipment(&mut self, seat: SeatId, acting: MinionId, play: PlayCtx) {
        let def = cards::lookup(play.card).expect("definition");
        // "Only one … can be played or equipped in a game" (Flaming
        // Candle): found in the library and equipped counts too.
        if lib_requires(play.card, cards::Requirement::NotPlayedThisGame)
            && !self.state.once_played.contains(&play.card)
        {
            self.state.once_played.push(play.card);
        }
        let ctx = PlayCtx {
            target: Target::Minion(acting),
            ..play
        };
        let mut in_play =
            Some(self.enter_play(CardInPlay::new(play.card, seat, Some(acting), play.mode)));
        for e in def.modes[play.mode as usize]
            .effects
            .iter()
            .filter(|e| !matches!(e, Effect::DeclareAction(_) | Effect::AddStealth(_)))
        {
            self.apply_card_effect(*e, &ctx, &mut in_play);
        }
        if let Some(id) = in_play {
            self.enter_library_contest(id);
        }
    }

    /// Could minion `m` equip with library card `card` as a card play would
    /// — clan and discipline requirements, "a minion can have only one", no
    /// unique copy already in play under its controller (rules-qa 7.1)?
    /// Cards without a definition cannot be put in play by the engine yet.
    pub(crate) fn equip_requirements_met(&self, m: MinionId, card: u16) -> bool {
        let st = &self.state;
        let lib = &LIBRARY[card as usize];
        is_equipment(card)
            && !st.controls_unique_copy(st.minions[m.index()].owner, card)
            && (lib.clans.is_empty()
                || st.minions[m.index()]
                    .clan()
                    .is_some_and(|c| lib.clans.contains(&c)))
            && lib.path.is_none_or(|p| st.follows(m, p))
            && lib
                .modes
                .first()
                .is_some_and(|mode| cards::has_disciplines(st, m, card, mode.tags))
            && st.can_hold(m, card, &[])
            // "Only one … can be played or equipped in a game" (Flaming
            // Candle).
            && !(lib_requires(card, cards::Requirement::NotPlayedThisGame)
                && st.once_played.contains(&card))
    }

    /// The search of a successful Magic of the Smith: every equipment in
    /// the library the vampire can equip and pay for (a pool cost down to 0
    /// included; no unpayable blood cost, rules-qa 9.3), by card, or none.
    /// A search for a card to move to hand offers every card of its kind.
    pub(crate) fn search_request(
        &self,
        minion: MinionId,
        to_hand: Option<cards::CardFilter>,
    ) -> DecisionRequest {
        let st = &self.state;
        let seat = self.acting_seat();
        // By card, not by position: the order of the library is hidden.
        let found = crate::state::distinct(&st.seats[seat.index()].library, |c| {
            if let Some(filter) = to_hand {
                return filter.matches(c);
            }
            // Payable with the reductions it could take once chosen
            // (rules-qa 38.5).
            self.equip_requirements_met(minion, c)
                && self.payable_with_potential(&found_play(seat, minion, c), false)
        });
        let mut options = vec![Choice::Pass];
        options.extend(found.into_iter().map(Choice::FindCard));
        DecisionRequest::new(seat, WindowLabel::LibrarySearch, options)
    }

    pub(crate) fn search_apply(
        &mut self,
        minion: MinionId,
        to_hand: Option<cards::CardFilter>,
        choice: &Choice,
    ) {
        let seat = self.acting_seat();
        match *choice {
            Choice::FindCard(card) if to_hand.is_some() => {
                let s = &mut self.state.seats[seat.index()];
                let k = s
                    .library
                    .iter()
                    .rposition(|&c| c == card)
                    .expect("found card in the library");
                s.library.remove(k);
                s.hand.push(card);
            }
            // Out of the library (shuffled afterward), then its cost: the
            // reductions announced for it first, if any apply (rules-qa
            // 38.5).
            Choice::FindCard(card) => {
                let library = &mut self.state.seats[seat.index()].library;
                let k = library
                    .iter()
                    .rposition(|&c| c == card)
                    .expect("found card in the library");
                library.remove(k);
                self.rng
                    .shuffle(&mut self.state.seats[seat.index()].library);
                let ctx = found_play(seat, minion, card);
                if !self.cost_choices(&ctx) {
                    self.equip_found(ctx);
                } else {
                    self.flow = Flow::CostTerms {
                        ctx,
                        resume: Box::new(Flow::GameOver),
                        pending: None,
                        then: crate::plays::CostThen::EquipFound,
                    };
                }
                return;
            }
            Choice::Pass => {}
            other => unreachable!("illegal search choice {other:?}"),
        }
        self.rng
            .shuffle(&mut self.state.seats[seat.index()].library);
        self.after_resolution();
    }

    /// A searched or found equipment (Magic of the Smith, Michael Harris,
    /// Vast Wealth) paid and equipped: British Museum's reduction, then the
    /// announced ones (The Shard, The Line, Ravnos Cache); paid down to 0
    /// pool, the Methuselah is ousted and the turn passes.
    pub(crate) fn equip_found(&mut self, ctx: PlayCtx) {
        let (seat, card) = (ctx.seat, ctx.card);
        let minion = ctx.actor.expect("equipping minion");
        let mut pool = cards::pool_cost(&self.state, seat, card, Target::Minion(minion));
        let mut blood = cards::blood_cost(&self.state, card, 0);
        let discount = self.state.equip_discount(minion, card);
        crate::plays::reduce(&mut pool, &mut blood, discount);
        let (pool, blood) = self.pay_named_discounts(&ctx, pool, blood);
        let s = &mut self.state.seats[seat.index()];
        s.pool = (s.pool - pool).max(0);
        self.state.burn_blood(minion, blood);
        self.put_equipment(seat, minion, ctx);
        if !(pool > 0 && self.state.seats[seat.index()].pool == 0 && self.oust_and_pass(seat)) {
            self.after_resolution();
        }
    }

    /// The cards Lenelle's exchange names: a library card in the ash heap,
    /// by card, then a card from hand, by card (copies are alike).
    pub(crate) fn exchange_request(&self, ash: Option<u16>) -> DecisionRequest {
        let seat = self.acting_seat();
        let s = &self.state.seats[seat.index()];
        let options = match ash {
            None => crate::state::distinct(&s.ash, |_| true)
                .into_iter()
                .map(Choice::FindCard)
                .collect(),
            Some(_) => self.discard_choices(seat),
        };
        DecisionRequest::new(seat, WindowLabel::ActionExchange, options)
    }

    pub(crate) fn exchange_apply(&mut self, ash: Option<u16>, choice: &Choice) {
        let seat = self.acting_seat();
        match (ash, *choice) {
            (None, Choice::FindCard(card)) => self.flow = Flow::Exchange { ash: Some(card) },
            (Some(card), Choice::Discard(i)) => {
                let held = self.state.seats[seat.index()].hand[i as usize];
                self.action.as_mut().expect("action").exchange = Some((card, held));
                self.flow = Flow::Action;
            }
            (_, other) => unreachable!("illegal exchange choice {other:?}"),
        }
    }

    /// The equipment an equip action takes from `giver`, announced one at
    /// a time with the action (rulebook "Equip": several at once); `Pass`
    /// once at least one is taken.
    pub(crate) fn action_terms_request(
        &self,
        minion: MinionId,
        giver: MinionId,
        picked: &[CardId],
    ) -> DecisionRequest {
        let st = &self.state;
        let with: Vec<u16> = picked.iter().map(|p| st.cards[p.index()].card).collect();
        let mut options = Vec::new();
        if !picked.is_empty() {
            options.push(Choice::Pass);
        }
        for id in st.equipment_on(giver) {
            if !picked.contains(&id) && st.can_hold(minion, st.cards[id.index()].card, &with) {
                options.push(Choice::TakeEquipment { card: id.0 });
            }
        }
        DecisionRequest::new(
            st.minions[minion.index()].owner,
            WindowLabel::ActionTerms,
            options,
        )
    }

    pub(crate) fn action_terms_apply(
        &mut self,
        minion: MinionId,
        giver: MinionId,
        mut picked: Vec<CardId>,
        choice: &Choice,
    ) {
        match choice {
            Choice::TakeEquipment { card } => {
                picked.push(CardId(*card));
                self.flow = Flow::ActionTerms {
                    minion,
                    giver,
                    picked,
                };
            }
            Choice::Pass => {
                self.declare_action(minion, ActionKind::EquipFrom, None, None, None);
                // Each equipment's own bonus for the action to equip with it
                // applies (Unlicensed Taxicab, LSJ 20060222).
                let stealth: u8 = picked
                    .iter()
                    .filter_map(|p| cards::lookup(self.state.cards[p.index()].card))
                    .flat_map(|def| def.modes.iter().take(1))
                    .map(|mode| printed_stealth(mode.effects))
                    .sum();
                let a = self.action.as_mut().expect("action");
                a.base_stealth += stealth;
                a.giver = Some(giver);
                a.taken = picked;
            }
            other => unreachable!("illegal action terms choice {other:?}"),
        }
    }

    /// Rulebook "Minion Phase": a ready unlocked vampire with no blood must
    /// hunt, and no non-mandatory action may be taken while any such hunt is
    /// pending — so the phase cannot even end. An ally performs no hunt,
    /// rescue or diablerie ("any ready vampire"), and one recruited this
    /// turn takes no action at all (rulebook "Recruit Ally").
    /// `seat` burned pool: during an action, the acting Methuselah's prey
    /// burning some is remembered (Věnceslava).
    pub(crate) fn note_pool_burned(&mut self, seat: SeatId) {
        let prey = self.action.as_ref().map(|a| self.state.prey(a.seat));
        if let (Some(a), Some(p)) = (self.action.as_mut(), prey) {
            if p == seat && a.seat != seat {
                a.prey_burned = true;
            }
        }
    }

    /// The Methuselah of the action under way, else the active one (an
    /// action is the acting minion's controller's: Enkil Cog's in another
    /// Methuselah's phase).
    pub(crate) fn acting_seat(&self) -> SeatId {
        self.action.as_ref().map_or(self.state.active, |a| a.seat)
    }

    pub(crate) fn minion_phase_options(&self) -> Vec<Choice> {
        self.action_options(self.state.active, None)
    }

    /// The actions `s` may take now: in their minion phase, with any ready
    /// unlocked minion; with `only`, that one alone — Enkil Cog's action in
    /// another Methuselah's phase, which may also be let go (Pass).
    pub(crate) fn action_options(&self, s: SeatId, only: Option<MinionId>) -> Vec<Choice> {
        let options = self.seat_action_options(s, only);
        let Some(m) = only else {
            return options;
        };
        std::iter::once(Choice::Pass)
            .chain(options.into_iter().filter(|o| {
                matches!(*o, Choice::TakeAction { minion, .. } if minion == m)
                    || matches!(*o, Choice::Play { actor: Some(a), .. }
                        | Choice::UseAbility { actor: Some(a), .. } if a == m)
            }))
            .collect()
    }

    fn seat_action_options(&self, s: SeatId, only: Option<MinionId>) -> Vec<Choice> {
        let st = &self.state;
        let mut ready = st.ready_unlocked(s);
        ready.retain(|&m| only.is_none_or(|o| o == m));
        let starving: Vec<MinionId> = ready
            .iter()
            .copied()
            .filter(|m| {
                let v = &st.minions[m.index()];
                (v.blood == 0 || v.must_hunt) && !v.is_ally()
            })
            .collect();
        // A vampire that does not hunt as normal must take the hunt it can
        // (Week of Nightmares' special hunt); with none, it is frozen: no
        // action at all (owner, 2026-09-26: rules-qa 28.8). So is one a
        // card forbade the same hunt this turn (Change of Target): stuck,
        // it cannot act (LSJ 19980112).
        let (unusual, starving): (Vec<MinionId>, Vec<MinionId>) =
            starving.into_iter().partition(|&m| {
                st.cannot_hunt(m)
                    || st.minions[m.index()]
                        .acted
                        .contains(&ActionId::Rulebook(ActionKind::Hunt, None))
            });
        let special: Vec<Choice> = self
            .ability_options(s, Window::ActionDeclare)
            .into_iter()
            .filter(|o| match *o {
                Choice::UseAbility {
                    source,
                    ability,
                    actor: Some(m),
                    ..
                } => {
                    // Or any vampire that must hunt (Restricted Vitae's
                    // special hunt).
                    (unusual.contains(&m) || starving.contains(&m))
                        && source
                            .mode(st, ability)
                            .effects
                            .contains(&Effect::HuntAction)
                }
                _ => false,
            })
            .collect();
        // A hunt action card is a hunt too, for any vampire that must hunt
        // (Psychophagia; owner, 2026-09-26: rules-qa 32.13).
        let hunt_cards: Vec<Choice> = self
            .play_options(s, &[Window::ActionDeclare], None)
            .into_iter()
            .filter(|o| match *o {
                Choice::Play {
                    card,
                    mode,
                    actor: Some(m),
                    ..
                } => {
                    (unusual.contains(&m) || starving.contains(&m))
                        && cards::lookup(self.virtual_hand(s)[card as usize].0).is_some_and(|d| {
                            d.modes[mode as usize].effects.contains(&Effect::HuntAction)
                        })
                }
                _ => false,
            })
            .collect();
        let special: Vec<Choice> = special.into_iter().chain(hunt_cards).collect();
        let frozen: Vec<MinionId> = unusual
            .iter()
            .copied()
            .filter(|&m| {
                !special.iter().any(|o| {
                    matches!(*o, Choice::UseAbility { actor: Some(a), .. }
                        | Choice::Play { actor: Some(a), .. } if a == m)
                })
            })
            .collect();
        if !starving.is_empty() || !special.is_empty() {
            // The forced hunt — after any untimed ability the Methuselah
            // wants first (The Coven's blood spares the hunt; owner,
            // 2026-09-06).
            // Beside it, any other mandatory action: the acting Methuselah
            // chooses their order (rules-qa 17.2).
            let mut options: Vec<Choice> = starving
                .into_iter()
                .map(|minion| Choice::TakeAction {
                    minion,
                    action: ActionKind::Hunt,
                    target: None,
                })
                .collect();
            options.extend(special);
            let (forced, _) =
                self.mandatory_split(s, &self.ability_options(s, Window::ActionDeclare));
            options.extend(forced);
            options.extend(self.ability_options(s, Window::AnyImpulse));
            return options;
        }
        let mut options = vec![Choice::Pass];
        let torpid = st.in_torpor();
        // A rulebook action a card forbade repeating this turn (Change of
        // Target).
        let forbidden = |minion: MinionId, kind: ActionKind, target: Option<MinionId>| {
            st.minions[minion.index()]
                .acted
                .contains(&ActionId::Rulebook(kind, target))
        };
        for id in st.minions_of(s) {
            let m = &st.minions[id.index()];
            if m.locked || m.recruited || only.is_some_and(|o| o != id) {
                continue;
            }
            if m.in_torpor {
                // Leave torpor: the 2-blood cost must be payable; a hostage
                // cannot be moved to the ready region (Carver's Meat
                // Packing and Storage).
                if m.blood >= 2 + st.torpid_surcharge(None, id) && m.hostage == 0 {
                    options.push(Choice::TakeAction {
                        minion: id,
                        action: ActionKind::LeaveTorpor,
                        target: None,
                    });
                }
                continue;
            }
            if !m.bled_this_turn {
                options.push(Choice::TakeAction {
                    minion: id,
                    action: ActionKind::Bleed,
                    target: None,
                });
            }
            let vampire = !m.is_ally();
            // Become Anarch: its cost payable as it is announced.
            if vampire
                && st.may_become_anarch(id)
                && m.blood >= st.become_anarch_cost(id)
                && !forbidden(id, ActionKind::BecomeAnarch, None)
            {
                options.push(Choice::TakeAction {
                    minion: id,
                    action: ActionKind::BecomeAnarch,
                    target: None,
                });
            }
            // "Ravnos … do not hunt as normal" (Week of Nightmares).
            if vampire && !st.cannot_hunt(id) {
                options.push(Choice::TakeAction {
                    minion: id,
                    action: ActionKind::Hunt,
                    target: None,
                });
            }
            // Equip from another minion the Methuselah controls, ready or
            // in torpor (rules-qa 9.2), with something this one could hold.
            for g in st.minions_of(s).filter(|&g| g != id) {
                if st
                    .equipment_on(g)
                    .iter()
                    .any(|e| st.can_hold(id, st.cards[e.index()].card, &[]))
                {
                    options.push(Choice::TakeAction {
                        minion: id,
                        action: ActionKind::EquipFrom,
                        target: Some(g),
                    });
                }
            }
            // Hostages can be neither rescued nor diablerized.
            for &t in torpid
                .iter()
                .filter(|&&t| vampire && st.minions[t.index()].hostage == 0)
            {
                // Rescue: 2 blood (less discounts) payable by acting,
                // rescued, or split.
                let cost = (2 + st.torpid_surcharge(Some(id), t))
                    .saturating_sub(st.rescue_discount(id, t).0);
                if m.blood + st.minions[t.index()].blood >= cost
                    && self.may_direct(s, id, Target::Minion(t))
                {
                    options.push(Choice::TakeAction {
                        minion: id,
                        action: ActionKind::Rescue,
                        target: Some(t),
                    });
                }
                if self.may_direct(s, id, Target::Minion(t)) && !st.blood_cursed(id) {
                    options.push(Choice::TakeAction {
                        minion: id,
                        action: ActionKind::Diablerie,
                        target: Some(t),
                    });
                }
            }
        }
        // Card actions from hand (employ …), declared by a ready unlocked
        // minion; NRA by card name per minion per turn (rules-qa 1.4).
        let mut declared = self.play_options(
            s,
            &[
                Window::ActionDeclare,
                Window::ActionDeclarePolitical,
                Window::ActionDeclareEquip,
                Window::ActionDeclareRecruit,
                Window::ActionDeclareEmploy,
            ],
            None,
        );
        // Actions granted by cards in play (Anarch Revolt's referendum, "burn
        // this card as a Ⓓ action").
        declared.extend(self.ability_options(s, Window::ActionDeclare));
        declared.extend(self.ability_options(s, Window::ActionDeclarePolitical));
        declared.retain(|o| {
            let actor = match *o {
                Choice::UseAbility { actor, .. } | Choice::Play { actor, .. } => actor,
                _ => None,
            };
            !actor.is_some_and(|a| st.minions[a.index()].recruited)
        });
        options.retain(|o| match *o {
            Choice::TakeAction {
                minion,
                action,
                target,
            } => !forbidden(minion, action, target),
            _ => true,
        });
        options.extend(declared);
        options.extend(self.ability_options(s, Window::AnyImpulse));
        // "During your next minion phase, burn this card to … take control"
        // (Puppet Master [DOM]): at any point of it (LSJ 20001201), but the
        // phase cannot end before.
        let due = options.iter().any(|o| match *o {
            Choice::UseAbility {
                source: cards::AbilitySource::Card(c),
                ability,
                ..
            } => cards::AbilitySource::Card(c)
                .mode(st, ability)
                .effects
                .contains(&Effect::TakeControlOfHost),
            _ => false,
        });
        if due {
            options.retain(|o| *o != Choice::Pass);
        }
        options.retain(|o| {
            let actor = match *o {
                Choice::TakeAction { minion, .. } => Some(minion),
                Choice::UseAbility { actor, .. } | Choice::Play { actor, .. } => actor,
                _ => None,
            };
            !actor.is_some_and(|a| frozen.contains(&a))
        });
        // "Cannot perform directed actions" (The Guardian).
        let barred: Vec<MinionId> = st
            .minions_of(s)
            .filter(|&m| {
                st.printed_mod(
                    m,
                    cards::Stat::NoDirectedActions,
                    crate::state::PrintedCtx::default(),
                ) > 0
            })
            .collect();
        if !barred.is_empty() {
            options.retain(|o| {
                !self
                    .directed_actor(s, o)
                    .is_some_and(|m| barred.contains(&m))
            });
        }
        // "Directed actions cost her +1 blood" (Daria; Marcellus's
        // political surcharge): payable with the action's own blood as it
        // is announced (rules-qa 42.1).
        options.retain(|o| {
            let Some((m, kind)) = self.option_action(s, o) else {
                return true;
            };
            let ctx = crate::state::PrintedCtx {
                kind: Some(kind),
                directed: Some(self.directed_actor(s, o).is_some()),
                ..Default::default()
            };
            let toll = self.action_toll(m, ctx);
            toll == 0 || st.minions[m.index()].blood >= toll + self.announced_blood(s, m, o)
        });
        self.mandatory_actions(s, options)
    }

    /// The minion `o` has perform an action and the action's kind, if it
    /// is one: a rulebook action, an action card, a granted action.
    fn option_action(&self, seat: SeatId, o: &Choice) -> Option<(MinionId, ActionKind)> {
        let declared = |effects: &[Effect]| {
            effects.iter().find_map(|e| match e {
                Effect::DeclareAction(k) => Some(*k),
                _ => None,
            })
        };
        match *o {
            Choice::TakeAction { minion, action, .. } => Some((minion, action)),
            Choice::Play {
                card,
                mode,
                actor: Some(m),
                ..
            } => {
                let lib = self.virtual_hand(seat).get(card as usize)?.0;
                Some((
                    m,
                    declared(cards::lookup(lib)?.modes[mode as usize].effects)?,
                ))
            }
            Choice::UseAbility {
                source,
                ability,
                actor: Some(m),
                ..
            } => Some((m, declared(source.mode(&self.state, ability).effects)?)),
            _ => None,
        }
    }

    /// The minion `o` has perform a directed action, if it does: a bleed; a
    /// rescue or diablerie of another Methuselah's vampire; an action card
    /// or a granted action aimed at another Methuselah, their minion or
    /// card, or a card in play's "burn / steal this card" of another's
    /// (rules-qa 17.2, 17.4). A political action is undirected.
    fn directed_actor(&self, seat: SeatId, o: &Choice) -> Option<MinionId> {
        let st = &self.state;
        let at_other = |t: Target| match t {
            Target::Minion(m) => st.minions[m.index()].owner != seat,
            Target::Card(c) => st.card_controller(c) != seat,
            Target::Seat(s) => s != seat,
            Target::None | Target::Uncontrolled(_) => false,
        };
        let declared = |effects: &[Effect]| {
            effects.iter().find_map(|e| match e {
                Effect::DeclareAction(k) => Some(*k),
                _ => None,
            })
        };
        match *o {
            Choice::TakeAction {
                minion,
                action: ActionKind::Bleed,
                ..
            } => Some(minion),
            Choice::TakeAction {
                minion,
                action: ActionKind::Rescue | ActionKind::Diablerie,
                target: Some(t),
            } => (st.minions[t.index()].owner != seat).then_some(minion),
            Choice::Play {
                card,
                mode,
                actor: Some(m),
                target,
            } => {
                let lib = self.virtual_hand(seat).get(card as usize)?.0;
                let kind = declared(cards::lookup(lib)?.modes[mode as usize].effects)?;
                (kind == ActionKind::Bleed || kind != ActionKind::Political && at_other(target))
                    .then_some(m)
            }
            Choice::UseAbility {
                source,
                ability,
                actor: Some(m),
                target,
            } => {
                let kind = declared(source.mode(st, ability).effects)?;
                let directed = match (kind, source) {
                    (ActionKind::Bleed, _) => true,
                    (ActionKind::Political, _) => false,
                    (
                        ActionKind::BurnCard | ActionKind::StealCard,
                        cards::AbilitySource::Card(c),
                    ) => st.card_controller(c) != seat,
                    _ => at_other(target),
                };
                directed.then_some(m)
            }
            _ => None,
        }
    }

    /// Mandatory printed actions (spec §3.9, Elen Kamjian): while one is
    /// attached and not yet taken this turn, it is the only action on
    /// offer (untimed abilities aside) and the phase cannot end; a vampire
    /// that cannot take it (it bled already) is stuck and takes no action.
    fn mandatory_actions(&self, s: SeatId, mut options: Vec<Choice>) -> Vec<Choice> {
        let (mut forced, stuck) = self.mandatory_split(s, &options);
        if !forced.is_empty() {
            forced.extend(self.ability_options(s, Window::AnyImpulse));
            return forced;
        }
        options.retain(|o| {
            let actor = match *o {
                Choice::TakeAction { minion, .. } => Some(minion),
                Choice::UseAbility { actor, .. } | Choice::Play { actor, .. } => actor,
                _ => None,
            };
            !actor.is_some_and(|a| stuck.contains(&a))
        });
        options
    }

    /// The mandatory printed actions attached among `options`, and the
    /// vampires stuck under one they cannot take. A vampire with no blood
    /// must hunt instead ("unless she must hunt", Elen Kamjian).
    fn mandatory_split(&self, s: SeatId, options: &[Choice]) -> (Vec<Choice>, Vec<MinionId>) {
        let st = &self.state;
        let mut forced = Vec::new();
        let mut stuck = Vec::new();
        for m in st.ready_unlocked(s) {
            if st.minions[m.index()].blood == 0 {
                continue;
            }
            let Some(def) = cards::vampire_of(&st.minions[m.index()]) else {
                continue;
            };
            for (ai, ability) in def.abilities.iter().enumerate() {
                let ai = ai as u8;
                if !ability.effects.contains(&Effect::MandatoryAction)
                    || st.minions[m.index()]
                        .acted
                        .iter()
                        .any(|a| a.same(ActionId::Printed(m, ai)))
                    || !ability
                        .requires
                        .iter()
                        .all(|&r| cards::holds(r, st, s, Some(m), None, None, None, None, false))
                {
                    continue;
                }
                let offered: Vec<Choice> = options
                    .iter()
                    .copied()
                    .filter(|o| {
                        matches!(o, Choice::UseAbility {
                            source: cards::AbilitySource::Vampire(v),
                            ability,
                            ..
                        } if *v == m && *ability == ai)
                    })
                    .collect();
                if offered.is_empty() {
                    stuck.push(m);
                }
                forced.extend(offered);
            }
        }
        (forced, stuck)
    }
}
