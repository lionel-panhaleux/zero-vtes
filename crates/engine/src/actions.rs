//! Actions: announcement, the as-announced window, the block loop (rulebook
//! states A/B/C, rules-qa 1.3), redirects, block resolution (combat, or the
//! blocker's diablerie on a leave-torpor), the after-combat window with the
//! Form of Mist continuation, and resolution. The action context lives on
//! the engine so that card plays resolved from the as-played stack can
//! modify it.

use crate::cards::{self, Effect, Hook, Limit, PlayCtx, Scope, Stat, Target, Window};
use crate::decision::{ActionKind, Choice, DecisionRequest, WindowLabel};
use crate::game::{Engine, Flow};
use crate::ids::{MinionId, SeatId};
use crate::library::LIBRARY;
use crate::referendum::RefKind;
use crate::state::{ActionId, CardId, CardInPlay};
use crate::window::OrderedWindow;

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
    PlayCtx {
        seat,
        card,
        mode: 0,
        actor: Some(acting),
        target: Target::Minion(acting),
        x: 0,
        burn: None,
        named: None,
    }
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionState {
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

#[derive(Clone, Debug)]
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
    /// The continuation was taken in the after-combat window.
    pub continued: bool,
    /// Card names played per minion this action (`Limit::PerAction`).
    pub played: Vec<(MinionId, &'static str)>,
    /// A reaction card was played during this action (canceled or not).
    pub reaction_played: bool,
    /// "Vampires cannot block this action" (Daring the Dawn).
    pub vampires_cannot_block: bool,
    /// Aggravated damage the acting vampire takes after resolution, if the
    /// action reaches it (Daring the Dawn).
    pub damage_after: u8,
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
    /// The blocker was unlocked when it blocked (Change of Target leaves it
    /// so: played before the blocker is locked, LSJ 19980224).
    pub blocker_was_unlocked: bool,
    /// The rescue's blood cost, set as it resolves (2, less discounts).
    pub rescue_cost: u8,
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
    /// "If this vampire blocks" riders of reactions played this action,
    /// given again to each block (LSJ 20010814-2).
    pub block_riders: Vec<(MinionId, cards::BlockRider)>,
    /// Minions that cannot attempt to block this action again (an attempt
    /// made to fail: Forced Confessional [DOM]).
    pub barred_blockers: Vec<MinionId>,
    /// Where the Veil Thins [OBL]: an attempt costs 1 blood without
    /// Oblivion.
    pub veil_thins: bool,
    /// Faceless Night [OBF]: failed attempts from now on lock their minion
    /// before resolution.
    pub lock_failed: bool,
    pub failed_blockers: Vec<MinionId>,
    /// Library cards a forced attempt costs, paid once the card forcing it
    /// is replaced (Parijat, ANK 20260317).
    pub library_toll: Vec<(SeatId, u8)>,
    /// Every minion whose block of this action succeeded ("if they did not
    /// block this action").
    pub blocked_by: Vec<MinionId>,
    /// Minions that may burn blood for intercept this action (Eyes of the
    /// Wild).
    pub burn_for_intercept: Vec<MinionId>,
    /// Minions locked after the action unless they blocked it (Eyes of the
    /// Wild).
    pub lock_if_no_block: Vec<MinionId>,
    /// A hunt that gained blood: "successfully hunts" (The Anarch Free
    /// Press; LSJ 20050720-2).
    pub hunt_gained: bool,
    /// A granted action that is a hunt (Week of Nightmares).
    pub is_hunt: bool,
    /// "After action resolution, if that action was successful, unlock the
    /// acting Nosferatu" (Warsaw Station): applied after the resolution and
    /// any referendum, the clan read then (LSJ 20090216), with or without
    /// the card in play (ANK 20250120).
    pub unlock_if_successful: Option<&'static str>,
    /// Minions whose block may still unlock them for blood after its
    /// combat (Truth in Darkness [OBL]), with the blood.
    pub after_block_unlocks: Vec<(MinionId, u8)>,
    /// Bonuses whose condition is read live (`Effect::If`).
    pub conditional: Vec<Conditional>,
    /// The ash card and the hand card an exchange action named (Lenelle).
    pub exchange: Option<(u16, u16)>,
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
    /// The action's stealth, the acting minion's cards in play included
    /// (Unlicensed Taxicab, Codex of the Edenic Groundskeepers), never
    /// below 0.
    pub fn stealth(&self, st: &crate::state::GameState) -> u8 {
        // A granted action's stealth for a clan, read live on the acting
        // minion (Wasserschloss Anif: "Malkavians get +1 stealth during
        // that action").
        let for_clan: i32 = match (self.source, self.ability) {
            (Some(src), Some(ab)) => src
                .mode(st, ab)
                .effects
                .iter()
                .map(|e| match *e {
                    Effect::StealthForClan { clan, n }
                        if st.minions[self.acting.index()].clan() == Some(clan) =>
                    {
                        n as i32
                    }
                    _ => 0,
                })
                .sum(),
            _ => 0,
        };
        (self.base_stealth as i32
            + for_clan
            + self.stealth_mods as i32
            + st.bearer_mod(self.acting, Stat::Stealth, self.kind)
            + st.printed_mod(self.acting, Stat::Stealth, self.printed_ctx(st, None))
            - self.stealth_reduce as i32)
            .max(0) as u8
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
        let n = (st.base_bleed(self.acting) as i32
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
    /// Paragon [PRE]: younger vampires and allies get -1).
    pub fn intercept_of(&self, st: &crate::state::GameState, m: MinionId) -> u8 {
        (Self::intercept_mods(st, m)
            + self.host_intercept(st, m)
            + self.given_intercept(m)
            + self.conditional_bonus(st, cards::Stat::Intercept, Some(m)))
        .max(0) as u8
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
        let on = |host: MinionId| {
            st.layers.iter().filter(move |l| {
                l.scope == Scope::InPlay
                    && st.layer_active(l)
                    && st.cards[l.source.index()].bearer() == Some(host)
            })
        };
        let own: i32 = on(m)
            .filter_map(|l| match l.modifier {
                cards::Modifier::HostInterceptOnBleedAtYou(n) if bleed_at_owner => Some(n as i32),
                _ => None,
            })
            .sum();
        let against: i32 = on(self.acting)
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
        own + st.bearer_mod(m, Stat::Intercept, self.kind) + against + printed
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
                _ => None,
            })
            .sum()
    }

    /// The effective intercept of the current attempt, if any.
    pub fn attempt_intercept(&self, st: &crate::state::GameState) -> u8 {
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
fn printed_stealth(effects: &[Effect]) -> u8 {
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
        self.state.phase_actions = self.state.phase_actions.saturating_add(1);
        let seat = self.state.active;
        let own_target = torpor_target.map(|t| self.state.minions[t.index()].owner == seat);
        let (base_stealth, bleed_target, directed_seat) = match kind {
            ActionKind::Bleed => (0, Some(self.state.prey(seat)), Some(self.state.prey(seat))),
            ActionKind::Hunt
            | ActionKind::LeaveTorpor
            | ActionKind::Employ
            | ActionKind::Recruit
            | ActionKind::EquipFrom => (1, None, None),
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
            ability: None,
            // An action card's target (Entrancement's ally); a granted
            // action's is set by `use_ability`.
            target: match (kind, card) {
                (ActionKind::Card, Some(p)) => p.target,
                _ => Target::None,
            },
            torpor_target,
            bleed_target,
            base_stealth,
            stealth_mods: 0,
            bleed_mods: 0,
            limited_bleed_used: false,
            state: ActionState::AsAnnounced,
            window: OrderedWindow::new(order.clone()),
            order,
            declined: Vec::new(),
            woken: Vec::new(),
            defer_reaction_replacement: false,
            end_on_block: false,
            failed: false,
            outcome: None,
            blocker: None,
            continuation: None,
            continued: false,
            played: Vec::new(),
            reaction_played: false,
            vampires_cannot_block: false,
            damage_after: 0,
            no_unlock: false,
            no_reactions: Vec::new(),
            reaction_surcharge: 0,
            ended_early: false,
            blocker_was_unlocked: false,
            rescue_cost: 2,
            rescue_gain: 0,
            referendum_passed: false,
            margin: 0,
            bleed_reduce: 0,
            card_returned: false,
            card_entered_play: false,
            cost_reduction: 0,
            stealth_reduce: 0,
            // An action card's own "allies cannot block this action"
            // (Slaughtering the Herd [dom]).
            allies_cannot_block: card.is_some_and(|p| {
                cards::lookup(p.card).is_some_and(|d| {
                    d.modes[p.mode as usize]
                        .effects
                        .contains(&Effect::AlliesCannotBlock)
                })
            }),
            strength_bonus: 0,
            aggravated_hands: false,
            giver: None,
            taken: Vec::new(),
            diablerist: None,
            abilities_used: Vec::new(),
            intercept_given: Vec::new(),
            conditional: Vec::new(),
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
            after_block_unlocks: Vec::new(),
            blocked_by: Vec::new(),
            barred_blockers: Vec::new(),
            veil_thins: false,
            lock_failed: false,
            failed_blockers: Vec::new(),
            library_toll: Vec::new(),
            burn_for_intercept: Vec::new(),
            lock_if_no_block: Vec::new(),
            hunt_gained: false,
            is_hunt: false,
            unlock_if_successful: None,
            exchange: None,
            resolved: false,
            bleed_successful: false,
            corruption_fails: 0,
            corruption_unlock: 0,
        });
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
    }

    /// "Each time the attached vampire announces an action, they move 1
    /// blood from themselves to this acting vampire" (Slaughtering the Herd
    /// [DOM]): riders of the cards on the acting minion.
    fn host_announces_riders(&mut self, acting: MinionId) {
        for ci in 0..self.state.cards.len() {
            let c = &self.state.cards[ci];
            if !c.present() || c.host != Some(acting) {
                continue;
            }
            let Some(def) = cards::lookup(c.card) else {
                continue;
            };
            let by = c.by;
            for r in def
                .riders
                .iter()
                .filter(|r| r.hook == Hook::HostAnnouncesAction)
            {
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
    }

    /// Sequencing order during an action: the acting Methuselah, then the
    /// target of a directed action, then the rest clockwise.
    pub(crate) fn action_order_for(&self, seat: SeatId, directed: Option<SeatId>) -> Vec<SeatId> {
        self.action_order(seat, directed)
    }

    fn action_order(&self, seat: SeatId, directed: Option<SeatId>) -> Vec<SeatId> {
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
    fn eligible_blockers(&self, a: &ActionCtx) -> Vec<SeatId> {
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

    /// The Methuselah a directed action is directed at, if any.
    fn directed_target(&self, a: &ActionCtx) -> Option<SeatId> {
        a.directed_seat(&self.state)
    }

    /// Minions of `seat` that may attempt to block: ready and unlocked, or
    /// woken for this action.
    fn block_candidates(&self, a: &ActionCtx, seat: SeatId) -> Vec<MinionId> {
        self.block_candidates_locked(a, seat, false)
    }

    /// Could a non-acting Methuselah reduce the action's stealth to 0 with
    /// a card in hand one of their reacting minions could play?
    fn stealth_strippable(&self, a: &ActionCtx) -> bool {
        let st = &self.state;
        (0..st.seats.len()).map(|i| SeatId(i as u8)).any(|s| {
            s != a.seat
                && !st.seats[s.index()].ousted
                && st.seats[s.index()].hand.iter().any(|&card| {
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
                                            LIBRARY[card as usize].modes
                                                [cards::library_mode(card, mi)]
                                            .tags,
                                        )
                                })
                        })
                    })
                })
        })
    }

    /// Can `m` of `seat` attempt to block the current action, its lock
    /// aside ("unlocks and attempts to block"): the normal prey, predator
    /// and target restrictions apply (ANK 20180623), and a minion that
    /// cannot block cannot (LSJ 20010714-2); one already attempting may
    /// (ANK 20181122-2).
    pub(crate) fn can_attempt_block(&self, seat: SeatId, m: MinionId) -> bool {
        let Some(a) = self.action.as_ref() else {
            return false;
        };
        match a.state {
            ActionState::Attempt(x) => x == m,
            ActionState::Open => {
                !a.failed
                    && self.eligible_blockers(a).contains(&seat)
                    && !a.declined.contains(&seat)
                    // An unlock-and-block card needs the unlock (The
                    // Sleeping Mind [DOM] bars it).
                    && self.block_candidates_locked(a, seat, !a.no_unlock).contains(&m)
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
            let pending = self
                .combat
                .as_ref()
                .is_some_and(|c| c.sides.iter().any(|s| s.pending.total() > 0));
            // Damage still pending resolves first; the combat may end with
            // it (a combatant burned, a Methuselah ousted).
            if !(pending && self.apply_damage()) && self.combat.is_some() {
                self.end_combat_now();
            }
        }
        let v = &mut self.state.minions[m.index()];
        v.owner = to;
        v.temp_from = Some(v.temp_from.unwrap_or(from));
    }

    /// What `m` pays to attempt to block this action, if it can: blood
    /// without Oblivion under Where the Veil Thins (none for an ally: it
    /// cannot), a library card against a wraith or zombie ally while a
    /// Parijat is ready (none from an empty library: ANK 20260317).
    pub(crate) fn block_toll(&self, a: &ActionCtx, m: MinionId) -> Option<(u8, u8)> {
        let st = &self.state;
        let v = &st.minions[m.index()];
        let obl = st.has_discipline(m, "obl");
        let blood = u8::from(a.veil_thins && !obl);
        if blood > 0 && (v.is_ally() || v.blood < blood) {
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
        Some((blood, cards))
    }

    /// Pay `m`'s block toll: the blood now, the library cards now or (a
    /// forced attempt) once its card is replaced.
    pub(crate) fn pay_block_toll(&mut self, m: MinionId, deferred: bool) {
        let a = self.action.as_ref().expect("action context");
        let (blood, cards) = self.block_toll(a, m).expect("an affordable toll");
        let owner = self.state.minions[m.index()].owner;
        self.state.minions[m.index()].blood -= blood;
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
        st.minions_of(seat)
            .filter(|m| {
                let v = &st.minions[m.index()];
                !v.in_torpor
                    && !v.expelled
                    && (ignore_lock || !v.locked || a.woken.contains(m))
                    && !a.barred_blockers.contains(m)
                    && self.block_toll(a, *m).is_some()
                    && !(vampire_acting && st.cannot_block_vampires(*m))
                    && !st.cannot_block(*m)
                    && !st.unblockable(a.acting, a.kind)
                    && !(no_allies && v.is_ally())
                    && !(a.vampires_cannot_block && !v.is_ally())
            })
            .collect()
    }

    /// Intercept reachability (DESIGN §4.3 elision, owner-approved
    /// 2026-09-02, P1 form): an attempt whose intercept can never reach the
    /// action's stealth — nothing in the controller's hand this minion could
    /// play adds enough — is pruned rather than offered. The closure-time
    /// comparison in `action_advance` is the actual semantics.
    fn intercept_reachable(&self, a: &ActionCtx, seat: SeatId, m: MinionId) -> u8 {
        let st = &self.state;
        // A Methuselah other than the acting one who could strip the
        // stealth (Night Terrors), the blocker's own or a third party
        // (rules-qa 1.2): any attempt may still succeed.
        if self.stealth_strippable(a) {
            return u8::MAX;
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
        let mut total = (a.host_intercept(st, m) + a.given_intercept(m) + conditional).max(0) as u8;
        total = total.saturating_add(self.printed_intercept_reach(seat, m));
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
                        _ => 0,
                    })
                    .sum();
                per_card = per_card.max(best);
            }
            total = total.saturating_add(per_card);
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
            for ab in def
                .abilities
                .iter()
                .filter(|ab| ab.windows.contains(&Window::ActionStealthIntercept))
            {
                if ab.limit == Some(Limit::PerAction)
                    && a.abilities_used
                        .contains(&cards::AbilitySource::Card(CardId(ci as u32)).use_key(0))
                {
                    continue;
                }
                if !ab.requires.iter().all(|&r| cards::holds_for_host(r, st, m)) {
                    continue;
                }
                for e in ab.effects {
                    if let Effect::AddIntercept(n) = e {
                        total = total.saturating_add(*n);
                    }
                }
            }
        }
        total = total.saturating_add(self.location_intercept_reach(seat, m));
        (total as i32 + ActionCtx::intercept_mods(st, m)).max(0) as u8
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
                            cards::Requirement::AttemptingSectIn(s) => {
                                v.sect().is_some_and(|x| s.contains(&x))
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
                                .contains(&cards::AbilitySource::Vampire(v).use_key(ai as u8))
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
            ActionState::Blocked(_) => {
                if acting {
                    vec![Window::ActionBlocked]
                } else {
                    vec![]
                }
            }
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
                if self.intercept_reachable(a, seat, m) >= a.stealth(&self.state) {
                    options.push(Choice::DeclareBlock { minion: m });
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
        // attempt needs intercept, as often as it pays.
        if let ActionState::Attempt(m) = a.state {
            let v = &self.state.minions[m.index()];
            if v.owner == seat
                && v.blood > 0
                && a.burn_for_intercept.contains(&m)
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
        options.extend(self.play_options(seat, &windows, None));
        // Abilities of cards in play timed like the cards of these windows
        // (Bowl of Convergence [AUS] on a block attempt).
        for &w in &windows {
            options.extend(self.ability_options(seat, w));
        }
        options.extend(self.ability_options(seat, Window::AnyImpulse));
        let window = match a.state {
            ActionState::AsAnnounced => WindowLabel::ActionAsAnnounced,
            ActionState::Open | ActionState::Attempt(_) => WindowLabel::ActionBlock,
            ActionState::Declined => WindowLabel::ActionBlocksDeclined,
            ActionState::WouldSucceed => WindowLabel::ActionWouldBeSuccessful,
            ActionState::AfterCombat => WindowLabel::ActionAfterCombat,
            ActionState::AfterResolution => WindowLabel::ActionAfterResolution,
            ActionState::Blocked(_) => WindowLabel::ActionBlocked,
        };
        Some(DecisionRequest {
            seat,
            window,
            options,
        })
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
                a.state = ActionState::Attempt(*minion);
                a.window.play();
                self.flow = Flow::Action;
                self.pay_block_toll(*minion, false);
            }
            Choice::BurnBloodForIntercept { minion } => {
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
            Choice::UnlockForBlood { minion } => {
                let i = a
                    .after_block_unlocks
                    .iter()
                    .position(|&(m, _)| m == *minion)
                    .expect("an unlock for blood");
                let (m, n) = a.after_block_unlocks.remove(i);
                a.window.play();
                let barred = a.no_unlock;
                let v = &mut self.state.minions[m.index()];
                v.blood -= n;
                if !barred {
                    v.locked = false;
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
                    self.state.minions[a.acting.index()].locked = false;
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
                if a.intercept_of(&self.state, blocker) >= a.stealth(&self.state) {
                    self.blocked(blocker);
                } else {
                    // The attempt fails: back to A, where the same Methuselah
                    // may attempt again (rulebook B.4, "if one attempt to
                    // block fails, another can be made").
                    if a.lock_failed {
                        a.failed_blockers.push(blocker);
                    }
                    a.state = ActionState::Open;
                    a.fresh_window();
                    self.flow = Flow::Action;
                }
            }
            ActionState::Declined | ActionState::WouldSucceed => {
                let would_succeed = a.state == ActionState::Declined
                    && !a.failed
                    && a.kind == ActionKind::Bleed
                    && a.bleed_amount(&self.state) > 0
                    && a.bleed_target
                        .is_some_and(|t| !self.state.seats[t.index()].ousted);
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
                } else if a.continued {
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
                } else {
                    let acting = a.acting;
                    let riders = a.block_riders.clone();
                    self.start_combat(acting, blocker);
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
        for &(m, rider) in riders {
            let Some(i) = c.side_of_minion(m) else {
                continue;
            };
            match rider {
                cards::BlockRider::Maneuver => c.sides[i].combat_maneuvers += 1,
                cards::BlockRider::Press => c.sides[i].combat_presses += 1,
                cards::BlockRider::PreventFirstRound(n) => c.sides[i].round_prevents += n,
                cards::BlockRider::NoStrikesFirstRound => c.no_strikes_round = Some(1),
                cards::BlockRider::NoEquipment => c.sides[i].no_equipment = true,
                cards::BlockRider::CombatEndsFirstRound => c.sides[i].combat_ends_round = Some(1),
                cards::BlockRider::OpponentNoCombatEndsFirstRound => {
                    c.sides[1 - i].no_combat_ends_round = Some(1)
                }
                cards::BlockRider::StrengthBeforeRange(n) => c.sides[i].strength_option += n,
                cards::BlockRider::UnlockAfterForBlood(n) => unlocks.push((m, n)),
            }
        }
        unlocks
    }

    /// The action reached resolution — successful, fizzled, failed or
    /// blocked (after its combat), or ended before block resolution (Change
    /// of Target, Mirror Walk): open the after-resolution window over the
    /// action's order (elided when nothing can be played). Every action
    /// that reached resolution has one (owner, 2026-09-26: Daring the
    /// Dawn's damage, ANK 20220331); an ended action takes no more cards
    /// in it (RTR 20180719), a failed one does (ANK 20221205).
    pub(crate) fn after_resolution(&mut self) {
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
                self.state.minions[m.index()].locked = false;
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

    /// The combat ended: open the after-combat window over the action's
    /// order (rules-qa 2.1: once, after all combats of the block).
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
        if !self.state.minions[blocker.index()].is_ally() {
            options.push(Choice::Diablerise);
        }
        DecisionRequest {
            seat: self.state.minions[blocker.index()].owner,
            window: WindowLabel::TorporBlock,
            options,
        }
    }

    pub(crate) fn torpor_block_apply(&mut self, choice: &Choice) {
        let a = self.action.as_mut().expect("action context");
        a.outcome = Some(Outcome::Blocked);
        let (blocker, acting) = (a.blocker.expect("blocker"), a.acting);
        match choice {
            Choice::Pass => self.after_resolution(),
            Choice::Diablerise => {
                self.diablerise(blocker, acting);
                self.end_action_into_blood_hunt(blocker);
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
        // Burned from torpor: the cards on it go, no "ready minion" rider.
        self.burn_minion(victim);
        self.state.gain_blood(diablerist, taken);
    }

    /// The action ends and the blood hunt referendum follows immediately;
    /// rotation starts at the diablerist's controller, clockwise (DESIGN
    /// §4.2 — a modeling call, owner-approved 2026-09-02).
    fn end_action_into_blood_hunt(&mut self, diablerist: MinionId) {
        // The blood hunt is part of the action (spec §3.7.8.1): the action
        // ends after it, through its after-resolution window (rules-qa 5.8).
        self.action.as_mut().expect("action context").diablerist = Some(diablerist);
        let seat = self.state.minions[diablerist.index()].owner;
        self.start_referendum(RefKind::BloodHunt { diablerist }, seat, None);
    }

    /// Effects of a modifier / reaction / master resolved during the action.
    pub(crate) fn apply_action_effect(&mut self, effect: Effect, play: &PlayCtx) {
        let mut a = self.action.take().expect("action context");
        let mut burned = None;
        match effect {
            Effect::AddStealth(n) => a.stealth_mods += n,
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
                    self.state.minions[a.acting.index()].locked = false;
                }
            }
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
                let cards::Target::Minion(t) = play.target else {
                    unreachable!("a wake without a minion target");
                };
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
            Effect::CancelableForPool(_) => {}
            Effect::ReduceStealthToZero => {
                let s = a.stealth(&self.state);
                a.stealth_reduce = a.stealth_reduce.saturating_add(s);
            }
            Effect::BurnActorBlood(n) => {
                let m = &mut self.state.minions[play.actor.expect("actor").index()];
                m.blood -= n.min(m.blood);
            }
            Effect::TargetAttemptsBlock(n) => {
                let cards::Target::Minion(t) = play.target else {
                    unreachable!("a block attempt without a minion target");
                };
                self.state.minions[t.index()].locked = false;
                a.intercept_given.push((t, n as i8));
                if a.state == ActionState::Open {
                    a.state = ActionState::Attempt(t);
                    self.action = Some(a);
                    self.pay_block_toll(t, true);
                    return;
                }
            }
            Effect::BurnForInterceptThisAction => {
                a.burn_for_intercept.push(play.actor.expect("actor"))
            }
            Effect::LockIfNoBlock => a.lock_if_no_block.push(play.actor.expect("actor")),
            Effect::AttemptBlock => {
                if a.state == ActionState::Open {
                    let m = play.actor.expect("attempting minion");
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
                self.state.minions[play.actor.expect("actor").index()].locked = false;
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
                a.order = self.action_order_for(a.seat, Some(t));
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
                    {
                        let a = self.action.as_mut().expect("action context");
                        a.outcome = Some(Outcome::Failed);
                    }
                    match kind {
                        ActionKind::Bleed => {
                            self.state.minions[acting.index()].bled_this_turn = true;
                        }
                        ActionKind::Political => {
                            self.state.minions[acting.index()].political_this_turn = true;
                        }
                        _ => {}
                    }
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

    /// Block resolution: the blocker locks; combat, except a blocked
    /// leave-torpor (no combat; the blocker may diablerise).
    fn blocked(&mut self, blocker: MinionId) {
        let was_unlocked = !self.state.minions[blocker.index()].locked;
        let (acting, kind) = {
            let a = self.action.as_mut().expect("action context");
            a.blocker = Some(blocker);
            a.blocker_was_unlocked = was_unlocked;
            (a.acting, a.kind)
        };
        self.state.minions[blocker.index()].locked = true;
        // NRA triggers on resolution, blocked included (rules-qa 1.4).
        self.mark_resolved(acting, kind);
        // "If blocked" riders of the acting minion's cards (Unlicensed
        // Taxicab) apply before any other card or effect (ANK 20220116).
        self.host_blocked_riders(acting);
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
    }

    /// Riders of the cards on the acting minion fired as its action is
    /// blocked, their conditions read on the blocker.
    fn host_blocked_riders(&mut self, acting: MinionId) {
        let seat = self.state.minions[acting.index()].owner;
        let mut burns = Vec::new();
        for (ci, c) in self.state.cards.iter().enumerate() {
            if !c.present() || c.host != Some(acting) {
                continue;
            }
            let Some(def) = cards::lookup(c.card) else {
                continue;
            };
            for r in def.riders.iter().filter(|r| r.hook == Hook::HostBlocked) {
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
                    match e {
                        Effect::BurnSelf => burns.push(CardId(ci as u32)),
                        other => unreachable!("not a host-blocked rider effect: {other:?}"),
                    }
                }
            }
        }
        for id in burns {
            self.burn_card_in_play(id);
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
                            let v = &mut self.state.minions[acting.index()];
                            v.blood -= n.min(v.blood);
                        }
                        other => unreachable!("not a host-blocked rider effect: {other:?}"),
                    }
                }
            }
        }
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
                    Effect::TargetMethuselahDiscards(n) => {
                        self.interrupts
                            .push(crate::draws::Interrupt::DiscardChosen {
                                seat: prey,
                                left: n,
                                done: 0,
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
        for (ci, c) in self.state.cards.iter().enumerate() {
            if !c.present()
                || c.host != Some(acting)
                || c.against != crate::ids::SeatSet::single(prey)
            {
                continue;
            }
            let Some(def) = cards::lookup(c.card) else {
                continue;
            };
            for r in def
                .riders
                .iter()
                .filter(|r| r.hook == Hook::HostAboutToBleedRemembered)
                .filter(|r| {
                    r.requires
                        .iter()
                        .all(|&req| cards::holds_for_host(req, &self.state, acting))
                })
            {
                for e in r.effects {
                    match *e {
                        Effect::BurnSelf => burns.push(CardId(ci as u32)),
                        Effect::BleedMod { n, .. } => bleed += n,
                        other => unreachable!("not an about-to-bleed rider effect: {other:?}"),
                    }
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
        let mut a = self.action.take().expect("action context");
        let seat = a.seat;
        a.outcome = Some(Outcome::Successful);
        match a.kind {
            ActionKind::Bleed => {
                let prey = a.bleed_target.expect("bleed target");
                self.state.minions[a.acting.index()].bled_this_turn = true;
                // The bleed's target was ousted before it resolved (the
                // cost of a cancel, LSJ 20050607): it resolves without
                // effect — no pool, no Edge.
                if self.state.seats[prey.index()].ousted {
                    a.outcome = Some(Outcome::Fizzled);
                    self.action = Some(a);
                    self.after_resolution();
                    return;
                }
                // A bleed granted by a card on the acting minion (Codex of
                // the Edenic Groundskeepers): it fizzles if the card is no
                // longer on the acting minion (RTR 19960221) or its blood
                // cost is unpayable by now (rules-qa 3.6).
                if let Some(src) = a.source {
                    let effects = self.granted_effects(&a);
                    let cost: u8 = effects
                        .iter()
                        .map(|e| match e {
                            Effect::PayBlood(n) => *n,
                            _ => 0,
                        })
                        .sum();
                    // A vampire's own printed bleed has no card to lose.
                    let attached = match src {
                        cards::AbilitySource::Card(c) => {
                            let card = &self.state.cards[c.index()];
                            card.present() && card.host == Some(a.acting)
                        }
                        cards::AbilitySource::Vampire(_) => true,
                    };
                    if !attached || self.state.minions[a.acting.index()].blood < cost {
                        a.outcome = Some(Outcome::Fizzled);
                        self.action = Some(a);
                        self.after_resolution();
                        return;
                    }
                    self.state.minions[a.acting.index()].blood -= cost;
                }
                // A bleed action card's cost (Slaughtering the Herd [dom]):
                // paid now, unpayable by then the action fizzles (spec
                // §1.7.1, rules-qa 3.6).
                if let Some(play) = a.card {
                    let blood = cards::blood_cost(&self.state, play.card, play.x);
                    if self.state.minions[a.acting.index()].blood < blood {
                        a.outcome = Some(Outcome::Fizzled);
                        self.action = Some(a);
                        self.after_resolution();
                        return;
                    }
                    self.state.minions[a.acting.index()].blood -= blood;
                }
                // About to bleed for 1 pool or more: the cards remembering
                // this prey burn for their bonus (Spying Mission [OBF]).
                if a.bleed_amount(&self.state) > 0 {
                    a.bleed_mods += self.about_to_bleed_riders(a.acting, prey);
                }
                let amount = a.bleed_amount(&self.state);
                self.state.seats[prey.index()].pool -= amount as i32;
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
                    }
                }
                if self.state.seats[prey.index()].pool <= 0 {
                    self.state.seats[prey.index()].pool = 0;
                    self.oust(prey);
                }
            }
            ActionKind::Hunt => {
                self.state.minions[a.acting.index()].must_hunt = false;
                let before = self.state.minions[a.acting.index()].blood;
                self.state.gain_blood(a.acting, 1);
                // Successful only if blood was gained (LSJ 20050720-2).
                a.hunt_gained = self.state.minions[a.acting.index()].blood > before;
                if a.hunt_gained {
                    self.successful_hunt_riders(a.acting);
                }
            }
            ActionKind::LeaveTorpor => {
                // Cost is paid at resolution only (a blocked action pays
                // nothing); blood spent during the action can leave it
                // unpayable, in which case the action fizzles (rules-qa
                // 3.6): successful, no effect.
                let m = &mut self.state.minions[a.acting.index()];
                if !m.in_torpor {
                    // Out of torpor already (Warsaw Station): it fizzles,
                    // its cost paid (ANK 20220218).
                    m.blood -= m.blood.min(2);
                    a.outcome = Some(Outcome::Fizzled);
                } else if m.blood >= 2 {
                    m.blood -= 2;
                    m.in_torpor = false;
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
                // The target no longer in torpor: nothing to rescue.
                let tv = &self.state.minions[target.index()];
                if !(tv.present() && tv.in_torpor) {
                    a.outcome = Some(Outcome::Fizzled);
                    self.action = Some(a);
                    self.after_resolution();
                    return;
                }
                let t = self.state.minions[target.index()].blood;
                let (discount, gain) = self.state.rescue_discount(a.acting, target);
                a.rescue_cost = 2u8.saturating_sub(discount);
                a.rescue_gain = gain;
                let payable = acting + t >= a.rescue_cost;
                if !payable {
                    a.outcome = Some(Outcome::Fizzled);
                }
                self.action = Some(a);
                if payable {
                    self.flow = Flow::RescueCost;
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
                    a.outcome = Some(Outcome::Fizzled);
                    self.action = Some(a);
                    self.after_resolution();
                    return;
                }
                self.action = Some(a);
                self.diablerise(diablerist, victim);
                self.end_action_into_blood_hunt(diablerist);
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
                let mut blood = cards::blood_cost(&self.state, play.card, play.x);
                // A reduction "by n blood or pool" takes off the card's cost
                // in whichever it has.
                if pool > 0 {
                    pool = (pool - a.cost_reduction as i32).max(0);
                } else {
                    blood = blood.saturating_sub(a.cost_reduction);
                }
                if self.state.seats[seat.index()].pool < pool
                    || self.state.minions[acting.index()].blood < blood
                {
                    a.outcome = Some(Outcome::Fizzled);
                    self.action = Some(a);
                    self.after_resolution();
                    return;
                }
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
                            self.state.minions[acting.index()].locked = false;
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
                    self.oust_and_pass(seat);
                    return;
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
                let pool = cards::pool_cost(&self.state, seat, play.card, Target::Minion(acting));
                let blood = cards::blood_cost(&self.state, play.card, play.x);
                if self.state.seats[seat.index()].pool < pool
                    || self.state.minions[acting.index()].blood < blood
                    || !self.state.can_hold(acting, play.card, &[])
                {
                    a.outcome = Some(Outcome::Fizzled);
                } else {
                    self.state.seats[seat.index()].pool -= pool;
                    self.state.minions[acting.index()].blood -= blood;
                    self.put_equipment(seat, acting, play);
                    a.card_entered_play = true;
                    if pool > 0 && self.state.seats[seat.index()].pool == 0 {
                        self.action = Some(a);
                        self.oust_and_pass(seat);
                        return;
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
                    a.outcome = Some(Outcome::Fizzled);
                    self.action = Some(a);
                    self.after_resolution();
                    return;
                }
                let mut ousted = false;
                let library = &self.state.seats[seat.index()].library;
                if let Some(k) = library.iter().rposition(|&c| is_equipment(c)) {
                    let card = self.state.seats[seat.index()].library.remove(k);
                    let blood = cards::blood_cost(&self.state, card, 0);
                    if self.equip_requirements_met(acting, card)
                        && self.state.minions[acting.index()].blood >= blood
                    {
                        let pool =
                            cards::pool_cost(&self.state, seat, card, Target::Minion(acting));
                        let s = &mut self.state.seats[seat.index()];
                        s.pool = (s.pool - pool).max(0);
                        ousted = pool > 0 && s.pool == 0;
                        self.state.minions[acting.index()].blood -= blood;
                        self.put_equipment(seat, acting, found_play(seat, acting, card));
                    } else {
                        self.state.seats[seat.index()].ash.push(card);
                    }
                }
                self.rng
                    .shuffle(&mut self.state.seats[seat.index()].library);
                if ousted {
                    self.action = Some(a);
                    self.oust_and_pass(seat);
                    return;
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
                let cost = cards::blood_cost(&self.state, play.card, x);
                let pool = cards::pool_cost(&self.state, seat, play.card, play.target);
                if self.state.minions[a.acting.index()].blood < cost
                    || self.state.seats[seat.index()].pool < pool
                {
                    a.outcome = Some(Outcome::Fizzled);
                    self.action = Some(a);
                    self.after_resolution();
                    return;
                }
                self.state.minions[a.acting.index()].blood -= cost;
                // Its pool cost too (Biothaumaturgic Experiment): paid down
                // to 0, the acting Methuselah is ousted and the turn passes.
                self.state.seats[seat.index()].pool -= pool;
                if pool > 0 && self.state.seats[seat.index()].pool == 0 {
                    self.action = Some(a);
                    self.oust_and_pass(seat);
                    return;
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
                // were read at announcement.
                for e in mode.effects.iter().filter(|_| target_ok).filter(|e| {
                    !matches!(
                        e,
                        Effect::DeclareAction(_)
                            | Effect::AddStealth(_)
                            | Effect::IfBlocks(_)
                            | Effect::EnterCombatWithTarget
                            | Effect::InResultingCombat(_)
                            | Effect::TargetActsInCombat
                            | Effect::Frenzy
                            | Effect::StillYours
                    )
                }) {
                    self.apply_card_effect(*e, &ctx, &mut in_play);
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
                    let Target::Minion(t) = ctx.target else {
                        unreachable!("combat without a minion target");
                    };
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
                        self.combat_riders(&riders);
                        return;
                    }
                }
                a.card_entered_play = in_play.is_some();
                if let Some(id) = in_play {
                    self.enter_library_contest(id);
                }
            }
            ActionKind::BurnCard => {
                // The granted action's pool cost is paid at resolution
                // ("as a Ⓓ action that costs 1 pool"); unpayable by then
                // it fizzles (rules-qa 3.6); paid down to 0, the acting
                // Methuselah is ousted and the turn passes.
                let cost: u8 = self
                    .granted_effects(&a)
                    .iter()
                    .map(|e| match e {
                        Effect::PayPool(n) => *n,
                        _ => 0,
                    })
                    .sum();
                let src = a.source.expect("source").card();
                if self.state.seats[seat.index()].pool < cost as i32 {
                    a.outcome = Some(Outcome::Fizzled);
                } else {
                    self.state.seats[seat.index()].pool -= cost as i32;
                    let host = self.state.cards[src.index()].host;
                    let effects = self.granted_effects(&a);
                    self.burn_card_in_play(src);
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
                        self.oust_and_pass(seat);
                        return;
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
                    a.outcome = Some(Outcome::Fizzled);
                    self.action = Some(a);
                    self.after_resolution();
                    return;
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
                    self.oust_and_pass(seat);
                    return;
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
                                .flatten(),
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
                    a.outcome = Some(Outcome::Fizzled);
                    self.action = Some(a);
                    self.after_resolution();
                    return;
                }
                let (target, exchange) = (a.target, a.exchange);
                self.action = Some(a);
                for e in effects {
                    match *e {
                        Effect::DeclareAction(_)
                        | Effect::AddStealth(_)
                        | Effect::MandatoryAction
                        | Effect::PayBlood(_)
                        | Effect::PayLife(_)
                        | Effect::PayPool(_)
                        | Effect::SameActionAs(_)
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
                            let Target::Minion(t) = target else {
                                unreachable!("blood to a target without a minion");
                            };
                            self.state.gain_blood(t, n);
                        }
                        Effect::BloodToTargetToStart(n) => {
                            let Target::Minion(t) = target else {
                                unreachable!("blood to a target without a minion");
                            };
                            let b = self
                                .state
                                .cap_starting(t, self.state.minions[t.index()].blood + n);
                            self.state.minions[t.index()].blood = b;
                        }
                        // Both still ready (the target was re-checked above).
                        Effect::EnterCombatWithTarget => {
                            let Target::Minion(t) = target else {
                                unreachable!("combat without a minion target");
                            };
                            if self.state.minions[acting.index()].ready() {
                                self.start_combat_at_resolution(acting, t);
                                return;
                            }
                        }
                        // Still one it could hold (re-checked above).
                        Effect::StealTargetCard => {
                            let Target::Card(c) = target else {
                                unreachable!("a steal without a card target");
                            };
                            self.state.cards[c.index()].host = Some(acting);
                        }
                        Effect::BurnTargetCard => {
                            let Target::Card(c) = target else {
                                unreachable!("a burn without a card target");
                            };
                            self.burn_card_in_play(c);
                        }
                        // Whose counter, if several Methuselahs' are on it:
                        // the acting Methuselah's choice.
                        Effect::BurnCorruptionFromTarget => {
                            let Target::Minion(t) = target else {
                                unreachable!("a corruption burn without a minion target");
                            };
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
                            let Target::Minion(t) = target else {
                                unreachable!("a title steal without a minion target");
                            };
                            self.steal_title(acting, t);
                        }
                        Effect::StealBloodFromTarget(n) => {
                            let Target::Minion(t) = target else {
                                unreachable!("a blood steal without a minion target");
                            };
                            let taken = n.min(self.state.minions[t.index()].blood);
                            self.state.minions[t.index()].blood -= taken;
                            self.state.gain_blood(acting, taken);
                            // A hunt stealing no blood is not successful (LSJ
                            // 20050720-2); blood it cannot hold goes to the
                            // bank (LSJ 20010809-1).
                            let hunt = self.action.as_ref().is_some_and(|a| a.is_hunt);
                            if hunt && taken > 0 {
                                self.action.as_mut().expect("action").hunt_gained = true;
                                self.successful_hunt_riders(acting);
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
                        let cost = cards::blood_cost(&self.state, play.card, play.x);
                        (&def.modes[play.mode as usize], cost)
                    }
                    (None, Some(src)) => {
                        let ability = a.ability.expect("granted action's ability");
                        (src.mode(&self.state, ability), 0)
                    }
                    (None, None) => unreachable!("political action without a source"),
                };
                let m = &mut self.state.minions[a.acting.index()];
                m.political_this_turn = true;
                if m.blood < cost {
                    // Unpayable by resolution: fizzles (rules-qa 3.6), no
                    // referendum.
                    a.outcome = Some(Outcome::Fizzled);
                    self.action = Some(a);
                    self.after_resolution();
                    return;
                }
                m.blood -= cost;
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

    /// Riders of a vampire that successfully hunted (Adisa).
    fn successful_hunt_riders(&mut self, m: MinionId) {
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
        let a = self.action.as_ref().expect("action context");
        let seat = a.seat;
        let acting = a.acting;
        if let Some(play) = a.card {
            // Entered play as it resolved: its own flag, never a same-named
            // card already on the minion (a second Raven Spy employed).
            if !a.card_entered_play && !a.card_returned {
                self.state.seats[seat.index()].ash.push(play.card);
            }
            self.state.minions[acting.index()]
                .acted
                .push(ActionId::Card(play.card));
        }
        if let (Some(src), Some(ab)) = (a.source, a.ability) {
            let id = src.action_id(&self.state, ab);
            self.state.minions[acting.index()].acted.push(id);
        }
        // "This minion cannot perform the same action again this turn"
        // (Change of Target): a rulebook action, by kind and target.
        if a.ended_early && a.card.is_none() && a.source.is_none() {
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

    pub(crate) fn rescue_request(&self) -> DecisionRequest {
        let a = self.action.as_ref().expect("action context");
        let st = &self.state;
        let acting = st.minions[a.acting.index()].blood;
        let t = st.minions[a.torpor_target.expect("target").index()].blood;
        let cost = a.rescue_cost;
        let options = (0..=cost)
            .filter(|&k| acting >= k && t >= cost - k)
            .map(|k| Choice::PayRescue { from_acting: k })
            .collect();
        DecisionRequest {
            seat: st.active,
            window: WindowLabel::RescueCost,
            options,
        }
    }

    pub(crate) fn rescue_apply(&mut self, from_acting: u8) {
        let a = self.action.as_ref().expect("action context");
        let (acting, target) = (a.acting, a.torpor_target.expect("target"));
        let (cost, gain) = (a.rescue_cost, a.rescue_gain);
        self.state.minions[acting.index()].blood -= from_acting;
        self.state.minions[target.index()].blood -= cost - from_acting;
        // Rescued: to the ready region, lock state untouched.
        self.state.minions[target.index()].in_torpor = false;
        self.state.gain_blood(target, gain);
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
        let life = body.life;
        let m = MinionId(self.state.minions.len() as u32);
        let id = self.enter_play(CardInPlay {
            card,
            owner: seat,
            controller: seat,
            host: None,
            body: Some(m),
            mode,
            by: None,
            in_play: true,
            contested: false,
            locked: false,
            counters: 0,
            stored: Vec::new(),
            effect_only: false,
            against: Default::default(),
            entered: 0,
            chosen: Vec::new(),
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
        let ctx = PlayCtx {
            seat,
            card,
            mode,
            actor: Some(m),
            target: Target::Minion(m),
            x: 0,
            burn: None,
            named: None,
        };
        self.apply_entry_effects(&ctx, id, false);
        self.enter_card_contest(m);
        m
    }

    /// Put the master card `card` on `m` without playing it (no cost, no
    /// requirement, nothing to cancel), its first mode's effects applying
    /// as it enters play (Childe of the Revolution's Discipline card).
    pub(crate) fn put_on_minion(&mut self, seat: SeatId, card: u16, m: MinionId) {
        let def = cards::lookup(card).expect("a defined card");
        let ctx = PlayCtx {
            seat,
            card,
            mode: 0,
            actor: None,
            target: Target::Minion(m),
            x: 0,
            burn: None,
            named: None,
        };
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
        let id = self.enter_play(CardInPlay {
            card: play.card,
            owner: seat,
            controller: seat,
            host: Some(acting),
            body: None,
            mode: play.mode,
            by: None,
            in_play: true,
            contested: false,
            locked: false,
            counters: life,
            stored: Vec::new(),
            effect_only: false,
            against: Default::default(),
            entered: 0,
            chosen: Vec::new(),
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
                    self.state.minions[ctx.actor.expect("actor").index()].locked = false;
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

    fn put_equipment(&mut self, seat: SeatId, acting: MinionId, play: PlayCtx) {
        let def = cards::lookup(play.card).expect("definition");
        let ctx = PlayCtx {
            target: Target::Minion(acting),
            ..play
        };
        let mut in_play = Some(self.enter_play(CardInPlay {
            card: play.card,
            owner: seat,
            controller: seat,
            host: Some(acting),
            body: None,
            mode: play.mode,
            by: None,
            in_play: true,
            contested: false,
            locked: false,
            counters: 0,
            stored: Vec::new(),
            effect_only: false,
            against: Default::default(),
            entered: 0,
            chosen: Vec::new(),
        }));
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
    fn equip_requirements_met(&self, m: MinionId, card: u16) -> bool {
        let st = &self.state;
        let lib = &LIBRARY[card as usize];
        is_equipment(card)
            && !st.controls_unique_copy(st.minions[m.index()].owner, card)
            && (lib.clans.is_empty()
                || st.minions[m.index()]
                    .clan()
                    .is_some_and(|c| lib.clans.contains(&c)))
            && lib
                .modes
                .first()
                .is_some_and(|mode| cards::has_disciplines(st, m, card, mode.tags))
            && st.can_hold(m, card, &[])
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
        let seat = st.active;
        let mut found: Vec<u16> = st.seats[seat.index()]
            .library
            .iter()
            .copied()
            .filter(|&c| {
                if let Some(filter) = to_hand {
                    return filter.matches(c);
                }
                self.equip_requirements_met(minion, c)
                    && st.seats[seat.index()].pool
                        >= cards::pool_cost(st, seat, c, Target::Minion(minion))
                    && st.minions[minion.index()].blood >= cards::blood_cost(st, c, 0)
            })
            .collect();
        // By card, not by position: the order of the library is hidden.
        found.sort_unstable();
        found.dedup();
        let mut options = vec![Choice::Pass];
        options.extend(found.into_iter().map(Choice::FindCard));
        DecisionRequest {
            seat,
            window: WindowLabel::LibrarySearch,
            options,
        }
    }

    pub(crate) fn search_apply(
        &mut self,
        minion: MinionId,
        to_hand: Option<cards::CardFilter>,
        choice: &Choice,
    ) {
        let seat = self.state.active;
        let mut ousted = false;
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
            Choice::FindCard(card) => {
                let library = &mut self.state.seats[seat.index()].library;
                let k = library
                    .iter()
                    .rposition(|&c| c == card)
                    .expect("found card in the library");
                library.remove(k);
                let pool = cards::pool_cost(&self.state, seat, card, Target::Minion(minion));
                let blood = cards::blood_cost(&self.state, card, 0);
                self.state.seats[seat.index()].pool -= pool;
                self.state.minions[minion.index()].blood -= blood;
                self.put_equipment(seat, minion, found_play(seat, minion, card));
                ousted = pool > 0 && self.state.seats[seat.index()].pool == 0;
            }
            Choice::Pass => {}
            other => unreachable!("illegal search choice {other:?}"),
        }
        self.rng
            .shuffle(&mut self.state.seats[seat.index()].library);
        if ousted {
            self.oust_and_pass(seat);
        } else {
            self.after_resolution();
        }
    }

    /// The cards Lenelle's exchange names: a library card in the ash heap,
    /// by card, then a card from hand, by card (copies are alike).
    pub(crate) fn exchange_request(&self, ash: Option<u16>) -> DecisionRequest {
        let seat = self.state.active;
        let s = &self.state.seats[seat.index()];
        let options = match ash {
            None => {
                let mut ash = s.ash.clone();
                ash.sort_unstable();
                ash.dedup();
                ash.into_iter().map(Choice::FindCard).collect()
            }
            Some(_) => {
                let mut seen = Vec::new();
                let mut options = Vec::new();
                for (i, &c) in s.hand.iter().enumerate() {
                    if !seen.contains(&c) {
                        seen.push(c);
                        options.push(Choice::Discard(i as u8));
                    }
                }
                options
            }
        };
        DecisionRequest {
            seat,
            window: WindowLabel::ActionExchange,
            options,
        }
    }

    pub(crate) fn exchange_apply(&mut self, ash: Option<u16>, choice: &Choice) {
        let seat = self.state.active;
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
        DecisionRequest {
            seat: st.active,
            window: WindowLabel::ActionTerms,
            options,
        }
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
    pub(crate) fn minion_phase_options(&self) -> Vec<Choice> {
        let st = &self.state;
        let s = st.active;
        let ready = st.ready_unlocked(s);
        let starving: Vec<MinionId> = ready
            .iter()
            .copied()
            .filter(|m| {
                let v = &st.minions[m.index()];
                (v.blood == 0 || v.must_hunt) && !v.is_ally()
            })
            .collect();
        // A vampire that does not hunt as normal must take the hunt it can
        // (Week of Nightmares' special hunt); with none, it is not held to
        // hunt.
        let (unusual, starving): (Vec<MinionId>, Vec<MinionId>) =
            starving.into_iter().partition(|&m| st.cannot_hunt(m));
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
                    unusual.contains(&m)
                        && source
                            .mode(st, ability)
                            .effects
                            .contains(&Effect::HuntAction)
                }
                _ => false,
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
            if m.locked || m.recruited {
                continue;
            }
            if m.in_torpor {
                // Leave torpor: the 2-blood cost must be payable; a hostage
                // cannot be moved to the ready region (Carver's Meat
                // Packing and Storage).
                if m.blood >= 2 && m.hostage == 0 {
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
                let cost = 2u8.saturating_sub(st.rescue_discount(id, t).0);
                if m.blood + st.minions[t.index()].blood >= cost
                    && self.may_direct(s, id, Target::Minion(t))
                {
                    options.push(Choice::TakeAction {
                        minion: id,
                        action: ActionKind::Rescue,
                        target: Some(t),
                    });
                }
                if self.may_direct(s, id, Target::Minion(t)) {
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
        self.mandatory_actions(s, options)
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
