//! Cards spent for nothing, and hands not built (owner, 2026-10-05: "it
//! tends to throw away wakes, unlocks … because it cannot construct its
//! hand properly"): recorded games (`data/records`, a harvest's
//! `data/server-records`: the humans' seats apart from the bots', by
//! profile and weights) and self-play of an exported network.
//! `cargo run -p zvtes-policy --profile ci --example waste -- <policy.zvp>
//! <games> [--t T] [--rescue B] [--pass B] [--reaction B] [--bleed B]
//! [--idle B] [--seats N] [--twda] [--by-deck] [--cards N]
//! [record.json | records dir]...` (self-play at N seats, 4 by default,
//! the precons or with `--twda` the TWDA decks, at temperature T, with
//! those nudges: `zvtes_policy::Nudges`; none with 0 games, the policy
//! then unread: `waste - 0 data/server-records`). Per group:
//! - rescues of one's own vampire offered (minion-phase decisions) and
//!   taken; bleeds for nothing (`zvtes_policy::empty_bleed`: Aggressive
//!   Corpse's) offered and taken;
//! - wakes (a card whose effect wakes a locked vampire) played, and those
//!   followed by no block from that vampire during the action;
//! - unlocks after an action (Freak Drive's "unlock this vampire") played,
//!   and those followed by neither another action nor a block by that
//!   minion before its controller's next turn;
//! - at each of the seat's own discard phases: the cards in hand, those no
//!   minion it controls could play (wrong clan or discipline), whether a
//!   card was discarded (the discard phase action, cycling one), and
//!   whether that card was a playable one while an unplayable one stayed;
//! - at each of the seat's own influence phases (owner, 2026-10-05: "it
//!   has a tendency to stay idle with all in torpor when it could draw a
//!   crypt card or try to influence one more vampire"), with no ready
//!   minion and with some: those ended with a transfer or a crypt draw
//!   still possible and none made, the pool then, and those ended with a
//!   fully influenced vampire left uncontrolled;
//! - transfers onto a copy of a unique vampire the seat already controls
//!   (`zvtes_policy::stuck_transfer`: not offered out while that copy
//!   stays, no voluntary self-contest), of the influence decisions
//!   offering one (owner, 2026-10-10);
//! - nightmare counters (Week of Nightmares) moved, and those onto one's
//!   own minion (`zvtes_policy::own_nightmare`; owner, 2026-10-10: "never a good move");
//! - cross-table play while no seat is ousted at 4 seats or more (owner,
//!   2026-10-14: damage between cross-table seats "only benefits the
//!   bouncing prey"), against a prey or predator and against a cross-table
//!   seat: block attempts on the seat's bleeds with a stealth card offered,
//!   and those answered with one; combats its minion fought, the cards it
//!   played in them, the blood the opposing minion lost and whether it
//!   went to torpor or burned; the pool its bleeds took from its prey and
//!   from cross-table seats (bounced ones);
//! - Wider View's draw-and-remove (a bot used it until its crypt was
//!   empty): influence decisions offering it and those taking
//!   it, the crypt left then; the vampires removed from the uncontrolled
//!   region (by it or another card), the blood on them, and those removed
//!   while one with less blood stayed;
//! - block windows (owner, 2026-10-15: every legal attempt offered since
//!   E25, the futile ones too, `Engine::block_can_reach`): those offering
//!   an attempt that could reach the stealth and those taken, those
//!   offering a futile one and those taken, and those offering only
//!   futile ones (the windows elided before E25) and those attempted;
//! - combat play (owner, 2026-10-20, of the walls and rushes: "they have
//!   specific actions and reactions to enter combat and they need their
//!   combat to be played efficiently on the right targets"): the seat's own turns
//!   with a rush offered (`zvtes::profile::rush`) and those with one
//!   declared, the rushes' targets (the seat's prey, its predator, another
//!   seat; carrying equipment; a minion that had declared a bleed); its blocks, by the acting seat; every combat
//!   its minion fought, the cards it played, the combat cards it held as
//!   the combat began, those with some held and none played, the blood
//!   the opposing minion lost, whether it or its own went to torpor or
//!   burned;
//! - the impact ledger (owner, 2026-10-25: "do the bot network improve on
//!   getting more actions and more impactful meaningful actions?"): A&B a
//!   round (`zvtes::profile`: its actions declared, its blocks that landed
//!   and its hindrances — a bleed bounced, an action canceled — over its
//!   own turns, one a round, whatever its minions' number), and the
//!   resource R = 2 × pool + the blood on its minions, its own, its
//!   prey's and its predator's change over its own turns; and the hand
//!   (owner: "cycling cards to get a more appropriate hand is even more
//!   important than keeping cards until they're really efficient"): its
//!   slots and dead slots a round (at its discard phases), and A&B and
//!   its prey's R a hand slot a round; with `--cards`, per card: the
//!   rounds it was held (its seat's hand as its turn began), played (and
//!   a round held: its use), cycled, and its prey's R a round held (a
//!   turn's change shared among the cards played in it: holding a card
//!   costs its rounds; a reaction's worth is in A&B, not there).
//!
//! `--by-deck` splits every group by the seat's deck (a pool deck's name,
//! "own list" for one matching none).

use std::collections::BTreeMap;
use std::sync::Arc;
use zvtes::actions::ActionState;
use zvtes::agent::Agent;
use zvtes::cards::{self, AbilitySource, Effect};
use zvtes::combat::CombatNote;
use zvtes::decision::{ActionKind, Choice, DecisionRequest, WindowLabel};
use zvtes::decks::Deck;
use zvtes::draws::Interrupt;
use zvtes::game::{Engine, Step};
use zvtes::ids::{MinionId, SeatId};
use zvtes::library::{CardType, LIBRARY};
use zvtes::notes::Note;
use zvtes_api::record::GameRecord;
use zvtes_env::habits;
use zvtes_host::replay::Replay;
use zvtes_policy::{Net, NetAgent, Nudges};

#[derive(Default, Debug)]
struct Counts {
    /// Influence decisions with a crypt draw offered and 5 pool or more
    /// (`zvtes_policy::idle_influence`), and those ending the phase.
    idle_offered: u64,
    idle_passed: u64,
    idle_drawn: u64,
    empty_bleed_offered: u64,
    /// Transfers decided with one onto a duplicate offered, and those
    /// made onto it.
    dup_offered: u64,
    dup_transfers: u64,
    /// Nightmare counter moves, and those onto one's own minion.
    nightmares: u64,
    nightmares_own: u64,
    empty_bled: u64,
    rescue_offered: u64,
    rescued: u64,
    wakes: u64,
    wakes_wasted: u64,
    unlocks: u64,
    unlocks_wasted: u64,
    discard_phases: u64,
    hand: u64,
    dead: u64,
    phases_with_dead: u64,
    cycled_with_dead: u64,
    cycled: u64,
    /// A playable card discarded while an unplayable one stayed in hand.
    kept_dead: u64,
    /// Cards cycled and cards in hand at discard phases, by first type.
    cycled_types: BTreeMap<String, u64>,
    hand_types: BTreeMap<String, u64>,
    /// Own influence phases, [no ready minion, some]: (phases, ended
    /// with a transfer or crypt draw possible, of those none made, pool
    /// summed over those, ended with a vampire ready to come out).
    influence: [(u64, u64, u64, u64, u64); 2],
    /// Cross-table play, [against a prey or predator, a cross-table seat]
    /// (`relation`; a bleed's pool: [its prey, a cross-table seat]).
    stealth_offered: [u64; 2],
    stealth_played: [u64; 2],
    combats: [u64; 2],
    combat_plays: [u64; 2],
    combat_damage: [u64; 2],
    combat_down: [u64; 2],
    bleeds: u64,
    bled: [i64; 2],
    /// Wider View: offered, used, the crypt summed at those uses, uses
    /// with under 4 crypt cards left.
    wider_offered: u64,
    wider_used: u64,
    wider_crypt: u64,
    wider_low: u64,
    /// Uncontrolled vampires removed, their blood summed, those removed
    /// while one with less blood stayed.
    removed: u64,
    removed_blood: u64,
    removed_dearer: u64,
    /// Block windows: with a reachable attempt offered, one taken; with a
    /// futile one offered, one taken; with only futile ones, one taken.
    reach_offered: u64,
    reach_taken: u64,
    futile_offered: u64,
    futile_taken: u64,
    only_futile: u64,
    only_futile_taken: u64,
    /// Own turns with a rush offered, those with one declared, rushes;
    /// their targets by `standing`, those carrying equipment, those that
    /// declared a bleed earlier in the game.
    rush_turns: u64,
    rush_turns_taken: u64,
    rushes: u64,
    rush_at: [u64; 3],
    rush_equipped: u64,
    rush_bleeder: u64,
    /// Blocks declared, by the acting seat's `standing`.
    blocks: [u64; 3],
    /// Every combat its minion fought: the cards it played, the combat
    /// cards in its hand as it began, those with some and none played, the
    /// blood the opposing minion lost, it and its own minion down.
    fights: u64,
    fight_plays: u64,
    fight_held: u64,
    fight_held_idle: u64,
    fight_damage: u64,
    fight_down: u64,
    fight_lost: u64,
    /// The impact ledger: own turns, the profile's actions, blocks landed
    /// and hindrances, and R's change over those turns [its own, its
    /// prey's, its predator's].
    own_turns: u64,
    actions: u64,
    landed: u64,
    hindrances: u64,
    resource: [i64; 3],
    /// The ledger's means and their errors, seat-game by seat-game (a
    /// game's turns are not independent): A&B, R's three changes.
    ab_ratio: Ratio,
    resource_ratio: [Ratio; 3],
    /// Per card: [rounds held, played, cycled, its prey's R change].
    cards: BTreeMap<u16, [f64; 4]>,
}

/// A ratio of totals over clusters (a seat's game: its count over its own
/// turns), with its two standard errors by linearization.
#[derive(Default, Debug)]
struct Ratio {
    n: f64,
    x: f64,
    t: f64,
    xx: f64,
    xt: f64,
    tt: f64,
}

impl Ratio {
    fn add(&mut self, x: f64, t: f64) {
        self.n += 1.0;
        self.x += x;
        self.t += t;
        self.xx += x * x;
        self.xt += x * t;
        self.tt += t * t;
    }

    /// The mean and two standard errors.
    fn mean(&self) -> (f64, f64) {
        if self.t == 0.0 {
            return (0.0, 0.0);
        }
        let m = self.x / self.t;
        let residuals = (self.xx - 2.0 * m * self.xt + m * m * self.tt).max(0.0);
        let n = self.n.max(2.0);
        (m, 2.0 * (residuals * n / (n - 1.0)).sqrt() / self.t)
    }
}

/// A seat's resource: 2 × its pool + the blood (an ally's life) on its
/// minions in play.
fn resource(e: &Engine, s: SeatId) -> i64 {
    let st = &e.state;
    2 * st.seats[s.index()].pool as i64
        + st.minions_of(s)
            .map(|m| st.minions[m.index()].blood as i64)
            .sum::<i64>()
}

fn is_wider(e: &Engine, o: &Choice) -> bool {
    matches!(*o, Choice::UseAbility { source: AbilitySource::Card(c), ability: 0, .. }
        if LIBRARY[e.state.cards[c.index()].card as usize].name == "Wider View")
}

/// `b` as `a` sees it: 0 its prey or predator, 1 cross-table; `None` once
/// a seat is ousted, under 4 seats, or for `a` itself.
fn relation(e: &Engine, a: SeatId, b: SeatId) -> Option<usize> {
    let st = &e.state;
    let n = st.seats.len();
    if n < 4 || a == b || st.seats.iter().any(|s| s.ousted) {
        return None;
    }
    let d = (b.index() + n - a.index()) % n;
    Some((d != 1 && d != n - 1) as usize)
}

/// `b` as `a` sees it: 0 its prey, 1 its predator, 2 another seat; with
/// two seats left the other is both, counted as prey.
fn standing(e: &Engine, a: SeatId, b: SeatId) -> usize {
    if e.state.prey(a) == b {
        0
    } else if e.state.predator(a) == b {
        1
    } else {
        2
    }
}

/// A combat under way: its minions, their seats, `relation`, the blood
/// each had and the combat cards each side held at the decision before it
/// began, the cards each side played. That decision can be before costs
/// paid at resolution (the Red List hunt's blood: damage dealt, as read
/// here), or inside a combat that ended with no decision since (a second
/// combat right after: the first's last strikes counted again).
struct Fight {
    minions: [MinionId; 2],
    seats: [SeatId; 2],
    rel: Option<usize>,
    blood: [u8; 2],
    plays: [u64; 2],
    held: [u64; 2],
}

fn combat_cards(e: &Engine, seat: SeatId) -> u64 {
    e.state.seats[seat.index()]
        .hand
        .iter()
        .filter(|&&k| LIBRARY[k as usize].types.contains(&CardType::Combat))
        .count() as u64
}

fn is_stealth(e: &Engine, me: SeatId, o: &Choice) -> bool {
    matches!(*o, Choice::Play { card, mode, .. }
        if effects(e, me, card, mode).iter().any(|f| matches!(f, Effect::AddStealth(_))))
}

fn type_of(card: u16) -> String {
    LIBRARY[card as usize]
        .types
        .first()
        .map_or("?".into(), |t| format!("{t:?}"))
}

type ActionKey = Option<(u32, MinionId)>;

/// The decisions of one game, watched: what each play turned out to be.
#[derive(Default)]
struct Watch {
    /// (group, vampire, the action it woke for).
    wakes: Vec<(String, MinionId, ActionKey)>,
    /// (group, minion, its controller, the turn it was unlocked).
    unlocks: Vec<(String, MinionId, SeatId, u32)>,
    /// A transfer or crypt draw made in the current influence phase.
    spent: bool,
    /// Each seat's group: cross-table counts go to the seat they describe.
    groups: Vec<String>,
    combat: Option<Fight>,
    /// Block attempts on a bleed with stealth offered: (action, blocker,
    /// `relation`, stealth played).
    attempts: Vec<(ActionKey, MinionId, usize, bool)>,
    /// The seat bleeding since the last decision, and each seat's prey (0)
    /// or cross-table (1) standing to it and pool then.
    bleeding: Option<(SeatId, Vec<Standing>)>,
    /// Each seat's own turn under way: (turn, a rush offered, one declared).
    rush_turn: Vec<Option<(u32, bool, bool)>>,
    /// At the last decision: each minion's blood, each seat's combat cards.
    blood: Vec<u8>,
    held: Vec<u64>,
    /// The minions that declared a bleed so far.
    bleeders: Vec<MinionId>,
    /// The own turn under way: (turn, seat, its prey, its predator, their
    /// resources as it began).
    turn: Option<(u32, SeatId, SeatId, SeatId, [i64; 3])>,
    /// The cards its seat played in that turn.
    turn_plays: Vec<u16>,
    /// Each seat's own turns in this game.
    seat_turns: Vec<u64>,
    /// Each seat's R changes over its own turns this game [own, prey's,
    /// predator's].
    seat_resource: Vec<[i64; 3]>,
}

/// A seat's prey (0) or cross-table (1) standing to the bleeder, and its pool.
type Standing = (Option<usize>, i32);

fn action_key(e: &Engine) -> ActionKey {
    e.action().map(|a| (e.state.turn, a.acting))
}

fn effects(e: &Engine, me: SeatId, card: u8, mode: u8) -> &'static [Effect] {
    let c = e.playable_card(me, card);
    cards::lookup(c)
        .and_then(|d| d.modes.get(mode as usize))
        .map_or(&[], |m| m.effects)
}

/// Could a minion the seat controls ever play this card (its clan and
/// one of its mode's disciplines)? Master cards always.
fn playable(e: &Engine, me: SeatId, card: u16) -> bool {
    let st = &e.state;
    let c = &LIBRARY[card as usize];
    if c.types.contains(&CardType::Master) {
        return true;
    }
    st.minions_of(me).any(|m| {
        let clan = c.clans.is_empty()
            || st.minions[m.index()]
                .clan()
                .is_some_and(|k| c.clans.contains(&k));
        clan && c.modes.iter().any(|mode| {
            mode.tags.is_empty()
                || if c.combo {
                    mode.tags.iter().all(|t| st.has_discipline(m, t))
                } else {
                    mode.tags.iter().any(|t| st.has_discipline(m, t))
                }
        })
    })
}

impl Watch {
    fn new(groups: Vec<String>) -> Self {
        Watch {
            rush_turn: vec![None; groups.len()],
            seat_turns: vec![0; groups.len()],
            seat_resource: vec![[0; 3]; groups.len()],
            groups,
            ..Watch::default()
        }
    }

    /// A seat's own turn over: its rush counts.
    fn end_turn(&mut self, seat: usize, counts: &mut BTreeMap<String, Counts>) {
        if let Some((_, offered, taken)) = self.rush_turn[seat].take() {
            let c = counts.entry(self.groups[seat].clone()).or_default();
            c.rush_turns += offered as u64;
            c.rush_turns_taken += taken as u64;
        }
    }

    /// The game over: what is still open.
    fn finish(&mut self, e: &mut Engine, counts: &mut BTreeMap<String, Counts>) {
        let notes = e.take_notes().unwrap_or_default();
        self.settle(e, &notes, counts);
        for s in 0..self.groups.len() {
            self.end_turn(s, counts);
        }
        self.close_turn(e, counts);
        use zvtes::profile::Count;
        for (s, p) in e.profiles.iter().enumerate() {
            let c = counts.entry(self.groups[s].clone()).or_default();
            let ab = (p.get(Count::Actions) + p.get(Count::BlocksLanded) + p.get(Count::Hindrances))
                as u64;
            c.actions += p.get(Count::Actions) as u64;
            c.landed += p.get(Count::BlocksLanded) as u64;
            c.hindrances += p.get(Count::Hindrances) as u64;
            let t = self.seat_turns[s] as f64;
            if t > 0.0 {
                c.ab_ratio.add(ab as f64, t);
                for k in 0..3 {
                    c.resource_ratio[k].add(self.seat_resource[s][k] as f64, t);
                }
            }
        }
    }

    /// The ledger's own turn: closed when the game turn changes (R's
    /// change over it to its seat's group), the next one opened.
    fn track_turn(&mut self, e: &Engine, counts: &mut BTreeMap<String, Counts>) {
        if self
            .turn
            .is_some_and(|(t, s, ..)| t == e.state.turn && s == e.state.active)
        {
            return;
        }
        self.close_turn(e, counts);
        let s = e.state.active;
        let (prey, pred) = (e.state.prey(s), e.state.predator(s));
        let r = [resource(e, s), resource(e, prey), resource(e, pred)];
        self.turn = Some((e.state.turn, s, prey, pred, r));
        let c = counts.entry(self.groups[s.index()].clone()).or_default();
        for &k in e.state.seats[s.index()].hand.iter() {
            c.cards.entry(k).or_default()[0] += 1.0;
        }
    }

    fn close_turn(&mut self, e: &Engine, counts: &mut BTreeMap<String, Counts>) {
        if let Some((_, s, prey, pred, r)) = self.turn.take() {
            let c = counts.entry(self.groups[s.index()].clone()).or_default();
            c.own_turns += 1;
            self.seat_turns[s.index()] += 1;
            for (k, x) in [s, prey, pred].into_iter().enumerate() {
                let d = resource(e, x) - r[k];
                c.resource[k] += d;
                self.seat_resource[s.index()][k] += d;
            }
            let share = (resource(e, prey) - r[1]) as f64 / self.turn_plays.len().max(1) as f64;
            for k in self.turn_plays.drain(..) {
                c.cards.entry(k).or_default()[3] += share;
            }
        }
    }

    /// What happened since the last decision: the pool a bleed took, a
    /// combat begun or over (`notes`, drained since).
    fn settle(&mut self, e: &Engine, notes: &[Note], counts: &mut BTreeMap<String, Counts>) {
        let st = &e.state;
        if let Some((x, before)) = self.bleeding.take() {
            let c = counts.entry(self.groups[x.index()].clone()).or_default();
            for (s, &(rel, pool)) in before.iter().enumerate() {
                if let Some(r) = rel {
                    c.bled[r] += (pool - st.seats[s].pool).max(0) as i64;
                }
            }
        }
        // From the combat notes: a combat with no decision in it (every
        // window elided) is seen too.
        for n in notes {
            match n {
                Note::Combat(CombatNote::Begins { minions }) => {
                    let seats = minions.map(|m| st.minions[m.index()].owner);
                    self.combat = Some(Fight {
                        minions: *minions,
                        seats,
                        rel: relation(e, seats[0], seats[1]),
                        blood: minions.map(|m| self.blood.get(m.index()).copied().unwrap_or(0)),
                        plays: [0; 2],
                        held: seats.map(|s| self.held.get(s.index()).copied().unwrap_or(0)),
                    });
                }
                Note::Combat(CombatNote::Ends { .. }) => {
                    if let Some(f) = self.combat.take() {
                        self.fought(e, &f, counts);
                    }
                }
                _ => {}
            }
        }
        self.blood = st.minions.iter().map(|m| m.blood).collect();
        self.held = (0..st.seats.len())
            .map(|s| combat_cards(e, SeatId(s as u8)))
            .collect();
    }

    /// A combat over: each side's counts.
    fn fought(&self, e: &Engine, f: &Fight, counts: &mut BTreeMap<String, Counts>) {
        let st = &e.state;
        let down = |m: MinionId| {
            let m = &st.minions[m.index()];
            (m.in_torpor || !m.in_play) as u64
        };
        for i in 0..2 {
            let them = &st.minions[f.minions[1 - i].index()];
            let c = counts
                .entry(self.groups[f.seats[i].index()].clone())
                .or_default();
            let damage = f.blood[1 - i].saturating_sub(them.blood) as u64;
            c.fights += 1;
            c.fight_plays += f.plays[i];
            c.fight_held += f.held[i];
            c.fight_held_idle += (f.held[i] > 0 && f.plays[i] == 0) as u64;
            c.fight_damage += damage;
            c.fight_down += down(f.minions[1 - i]);
            c.fight_lost += down(f.minions[i]);
            if let Some(rel) = f.rel {
                c.combats[rel] += 1;
                c.combat_plays[rel] += f.plays[i];
                c.combat_damage[rel] += damage;
                c.combat_down[rel] += down(f.minions[1 - i]);
            }
        }
    }

    /// Cross-table play at this decision (`settle` first).
    fn cross(
        &mut self,
        e: &Engine,
        req: &DecisionRequest,
        o: Choice,
        counts: &mut BTreeMap<String, Counts>,
    ) {
        let me = req.seat;
        let key = action_key(e);
        if let Some(f) = &mut self.combat {
            if let (Some(i), Choice::Play { .. }) = (f.seats.iter().position(|&s| s == me), o) {
                f.plays[i] += 1;
            }
        }
        let c = counts.entry(self.groups[me.index()].clone()).or_default();
        if let Some(a) = e.action() {
            if let (ActionKind::Bleed, ActionState::Attempt(b), true) =
                (a.kind, a.state, a.seat == me)
            {
                let stealth = is_stealth(e, me, &o);
                match self.attempts.iter_mut().find(|t| t.0 == key && t.1 == b) {
                    Some(t) => {
                        if stealth && !t.3 {
                            t.3 = true;
                            c.stealth_played[t.2] += 1;
                        }
                    }
                    None => {
                        let them = e.state.minions[b.index()].owner;
                        if let (true, Some(rel)) = (
                            req.options.iter().any(|o| is_stealth(e, me, o)),
                            relation(e, me, them),
                        ) {
                            c.stealth_offered[rel] += 1;
                            c.stealth_played[rel] += stealth as u64;
                            self.attempts.push((key, b, rel, stealth));
                        }
                    }
                }
            }
        }
        let declared = match o {
            Choice::TakeAction {
                minion,
                action: ActionKind::Bleed,
                ..
            } => Some(minion),
            Choice::Play {
                card, mode, actor, ..
            } if effects(e, me, card, mode).contains(&Effect::DeclareAction(ActionKind::Bleed)) => {
                actor
            }
            _ => None,
        };
        if let Some(m) = declared.filter(|m| !self.bleeders.contains(m)) {
            self.bleeders.push(m);
        }
        let declared = declared.is_some();
        let st = &e.state;
        let bleeder = if declared {
            if relation(e, me, st.prey(me)).is_some() {
                c.bleeds += 1;
            }
            Some(me)
        } else {
            e.action()
                .filter(|a| a.kind == ActionKind::Bleed)
                .map(|a| a.seat)
        };
        self.bleeding = bleeder.map(|x| {
            let before = (0..st.seats.len())
                .map(|s| {
                    let s = SeatId(s as u8);
                    let rel = relation(e, x, s).filter(|&r| r == 1 || s == st.prey(x));
                    (rel, st.seats[s.index()].pool)
                })
                .collect();
            (x, before)
        });
    }
    /// Rushes offered and declared on the seat's own turn, their targets;
    /// its blocks.
    fn combat_play(
        &mut self,
        e: &Engine,
        req: &DecisionRequest,
        o: Choice,
        counts: &mut BTreeMap<String, Counts>,
    ) {
        let me = req.seat;
        let st = &e.state;
        let active = st.active.index();
        if self.rush_turn[active].is_some_and(|(t, _, _)| t != st.turn) {
            self.end_turn(active, counts);
        }
        if let Choice::DeclareBlock { .. } = o {
            if let Some(a) = e.action() {
                let c = counts.entry(self.groups[me.index()].clone()).or_default();
                c.blocks[standing(e, me, a.seat)] += 1;
            }
        }
        if st.active != me || req.window != WindowLabel::MinionPhase {
            return;
        }
        let offered = req
            .options
            .iter()
            .any(|o| zvtes::profile::rush(e, me, o).is_some());
        let target = zvtes::profile::rush(e, me, &o);
        let turn = self.rush_turn[active].get_or_insert((st.turn, false, false));
        turn.1 |= offered;
        turn.2 |= target.is_some();
        if let Some(t) = target {
            let c = counts.entry(self.groups[me.index()].clone()).or_default();
            c.rushes += 1;
            c.rush_at[standing(e, me, st.minions[t.index()].owner)] += 1;
            c.rush_equipped += st.cards.iter().any(|k| {
                k.in_play
                    && k.host == Some(t)
                    && LIBRARY[k.card as usize]
                        .types
                        .contains(&CardType::Equipment)
            }) as u64;
            c.rush_bleeder += self.bleeders.contains(&t) as u64;
        }
    }

    /// Before `chosen` is applied: settle what this decision shows, then
    /// record what it plays.
    fn see(
        &mut self,
        e: &Engine,
        req: &DecisionRequest,
        chosen: usize,
        notes: &[Note],
        counts: &mut BTreeMap<String, Counts>,
    ) {
        self.track_turn(e, counts);
        let me = req.seat;
        let key = action_key(e);
        let o = req.options[chosen];
        if let Choice::Play { card, .. } = o {
            let k = e.playable_card(me, card);
            let c = counts.entry(self.groups[me.index()].clone()).or_default();
            c.cards.entry(k).or_default()[1] += 1.0;
            if e.state.active == me {
                self.turn_plays.push(k);
            }
        }
        let group = self.groups[me.index()].clone();
        let group = group.as_str();
        self.settle(e, notes, counts);
        self.cross(e, req, o, counts);
        self.combat_play(e, req, o, counts);
        // A block by a woken vampire uses its wake.
        if let Choice::DeclareBlock { minion } = o {
            self.wakes.retain(|(_, m, k)| !(*m == minion && *k == key));
        }
        // The action a wake was for is over: wasted.
        let (over, kept): (Vec<_>, Vec<_>) = self.wakes.drain(..).partition(|(_, _, k)| *k != key);
        self.wakes = kept;
        for (g, _, _) in over {
            counts.entry(g).or_default().wakes_wasted += 1;
        }
        // Another action or a block by an unlocked minion uses the unlock.
        let acting = match o {
            Choice::TakeAction { minion, .. } | Choice::DeclareBlock { minion } => Some(minion),
            Choice::Play { actor, .. } | Choice::UseAbility { actor, .. }
                if req.window == WindowLabel::MinionPhase =>
            {
                actor
            }
            _ => None,
        };
        if let Some(m) = acting {
            self.unlocks.retain(|(_, u, _, _)| *u != m);
        }
        let turn = e.state.turn;
        // Its controller's next turn: the unlock was not used.
        let (over, kept): (Vec<_>, Vec<_>) = self
            .unlocks
            .drain(..)
            .partition(|(_, _, s, t)| turn > *t && e.state.active == *s);
        self.unlocks = kept;
        for (g, _, _, _) in over {
            counts.entry(g).or_default().unlocks_wasted += 1;
        }
        let c = counts.entry(group.to_string()).or_default();
        let rescue = |o: &Choice| habits::habit(e, me, o) == Some(habits::Habit::Rescue);
        let empty = |o: &Choice| zvtes_policy::empty_bleed(e, o);
        if req.options.iter().any(empty) {
            c.empty_bleed_offered += 1;
            c.empty_bled += empty(&o) as u64;
        }
        let stuck = |o: &Choice| zvtes_policy::stuck_transfer(e, me, o);
        if req.options.iter().any(stuck) {
            c.dup_offered += 1;
            c.dup_transfers += stuck(&o) as u64;
        }
        if let Some(own) = zvtes_policy::own_nightmare(e, me, &o) {
            c.nightmares += 1;
            c.nightmares_own += own as u64;
        }
        if req.options.iter().any(|o| is_wider(e, o)) {
            let crypt = e.state.seats[me.index()].crypt.len() as u64;
            c.wider_offered += 1;
            if is_wider(e, &o) {
                c.wider_used += 1;
                c.wider_crypt += crypt;
                c.wider_low += (crypt < 4) as u64;
            }
        }
        if let (Some(Interrupt::RemoveUncontrolled { seat }), Choice::Slot(i)) = (e.interrupt(), o)
        {
            let slots = &e.state.seats[seat.index()].uncontrolled;
            let blood = slots[i as usize].counters;
            c.removed += 1;
            c.removed_blood += blood as u64;
            c.removed_dearer += slots.iter().any(|s| s.counters < blood) as u64;
        }
        if req.window == WindowLabel::ActionBlock {
            let attempt = |o: &Choice, reach: bool| matches!(*o, Choice::DeclareBlock { minion } if e.block_can_reach(me, minion) == reach);
            for (reach, offered, taken) in [
                (true, &mut c.reach_offered, &mut c.reach_taken),
                (false, &mut c.futile_offered, &mut c.futile_taken),
            ] {
                if req.options.iter().any(|o| attempt(o, reach)) {
                    *offered += 1;
                    *taken += attempt(&o, reach) as u64;
                }
            }
            if e.futile_blocks(req) {
                c.only_futile += 1;
                c.only_futile_taken += matches!(o, Choice::DeclareBlock { .. }) as u64;
            }
        }
        if req.options.iter().any(rescue) {
            c.rescue_offered += 1;
            c.rescued += rescue(&o) as u64;
        }
        if let Choice::Play {
            card,
            mode,
            actor: Some(actor),
            ..
        } = o
        {
            let fx = effects(e, me, card, mode);
            if fx.contains(&Effect::Wake) {
                c.wakes += 1;
                self.wakes.push((group.to_string(), actor, key));
            }
            if fx.contains(&Effect::UnlockActor) {
                c.unlocks += 1;
                self.unlocks.push((group.to_string(), actor, me, turn));
            }
        }
        if zvtes_policy::idle_influence(e, req) {
            c.idle_offered += 1;
            c.idle_passed += (o == Choice::Pass) as u64;
            c.idle_drawn += (o == Choice::DrawCrypt) as u64;
        }
        if req.window == WindowLabel::InfluenceTransfers && e.state.active == me {
            let st = &e.state;
            match o {
                Choice::Transfer { .. } | Choice::DrawCrypt | Choice::InfluenceOut { .. } => {
                    self.spent = true
                }
                Choice::Pass => {
                    let ready = st.minions_of(me).any(|m| st.minions[m.index()].ready());
                    let i = &mut c.influence[ready as usize];
                    let usable = req
                        .options
                        .iter()
                        .any(|o| matches!(o, Choice::Transfer { .. } | Choice::DrawCrypt));
                    i.0 += 1;
                    if usable {
                        i.1 += 1;
                        if !self.spent {
                            i.2 += 1;
                            i.3 += st.seats[me.index()].pool.max(0) as u64;
                        }
                    }
                    i.4 += req
                        .options
                        .iter()
                        .any(|o| matches!(o, Choice::InfluenceOut { .. }))
                        as u64;
                    self.spent = false;
                }
                _ => {}
            }
        }
        if req.window == WindowLabel::DiscardPhase && e.state.active == me {
            let hand = &e.state.seats[me.index()].hand;
            let dead = hand.iter().filter(|&&k| !playable(e, me, k)).count() as u64;
            let cycled = matches!(o, Choice::Discard(_));
            if let Choice::Discard(i) = o {
                if dead > 0 && playable(e, me, hand[i as usize]) {
                    c.kept_dead += 1;
                }
                c.cards.entry(hand[i as usize]).or_default()[2] += 1.0;
                *c.cycled_types.entry(type_of(hand[i as usize])).or_default() += 1;
            }
            for &k in hand {
                *c.hand_types.entry(type_of(k)).or_default() += 1;
            }
            c.discard_phases += 1;
            c.hand += hand.len() as u64;
            c.dead += dead;
            c.cycled += cycled as u64;
            if dead > 0 {
                c.phases_with_dead += 1;
                c.cycled_with_dead += cycled as u64;
            }
        }
    }
}

/// Game records read back: each seat's counts to its group — "humans",
/// or the bots by profile and weights (a harvest mixes several). A record
/// that does not read is skipped, said on stderr.
fn recorded_games(paths: &[String], by_deck: bool, counts: &mut BTreeMap<String, Counts>) {
    for path in paths {
        let record = std::fs::read_to_string(path)
            .map_err(|e| e.to_string())
            .and_then(|t| serde_json::from_str::<GameRecord>(&t).map_err(|e| e.to_string()))
            .and_then(|r| Replay::new(&r).map(|p| (r, p)));
        let (record, replay) = match record {
            Ok(x) => x,
            Err(e) => {
                eprintln!("{path}: skipped ({e})");
                continue;
            }
        };
        let mut e = replay.deal();
        e.notes = Some(Vec::new());
        let human: Vec<bool> = (0..replay.seats())
            .map(|s| replay.humans.contains(&(s as u8)))
            .collect();
        let mut reader = replay.reader();
        let groups: Vec<String> = (0..human.len())
            .map(|s| {
                if human[s] {
                    return "humans".to_string();
                }
                match record.bots.iter().find(|b| b.seat as usize == s) {
                    // As `scripts/games_report.py`'s `bot_label`.
                    Some(b) => {
                        let t = b.temperature.map_or(String::new(), |t| format!("@{t}"));
                        match &b.weights {
                            Some(w) => format!("bots, {}{t} {}", b.profile, &w[..w.len().min(12)]),
                            None => format!("bots, {}{t}", b.profile),
                        }
                    }
                    None => "bots".to_string(),
                }
            })
            .enumerate()
            .map(|(s, g)| match by_deck {
                true => format!("{g}, {}", deck_name(&record.spec.seats[s].deck)),
                false => g,
            })
            .collect();
        let mut watch = Watch::new(groups.clone());
        while let Step::Decision(req) = e.run() {
            // A window the record's engine elided (before rules-qa 73.8):
            // passed, as it was then — no decision of the record's.
            if let Some(i) = replay.elided(&e, &req) {
                e.choose(i);
                continue;
            }
            let i = match reader.next(&e, &req) {
                None => break,
                Some(Ok(i)) => i,
                Some(Err(why)) => {
                    eprintln!("{path}: read to its decision {}: {why}", reader.position());
                    break;
                }
            };
            let notes = e.take_notes().unwrap_or_default();
            watch.see(&e, &req, i, &notes, counts);
            e.choose(i);
        }
        watch.finish(&mut e, counts);
    }
}

/// A record seat's pool deck by name, "own list" when its list is none.
fn deck_name(list: &zvtes_api::spec::DeckList) -> String {
    let Ok(r) = zvtes_host::deck::resolve(0, list) else {
        return "own list".into();
    };
    let sorted = |x: &[(u16, u8)]| {
        let mut v = x.to_vec();
        v.sort();
        v
    };
    let (crypt, library) = (sorted(&r.crypt), sorted(&r.library));
    Deck::all()
        .find(|d| sorted(d.crypt) == crypt && sorted(d.library) == library)
        .map_or("own list".into(), |d| d.name.to_string())
}

/// Self-play: `games` at `seats` seats, the precons or the TWDA decks,
/// at `temperature` with `nudges`.
struct SelfPlay {
    games: u64,
    seats: u64,
    twda: bool,
    temperature: f32,
    nudges: Nudges,
}

fn self_play(path: &str, play: &SelfPlay, by_deck: bool, counts: &mut BTreeMap<String, Counts>) {
    let SelfPlay {
        games,
        seats,
        twda,
        temperature,
        nudges,
    } = *play;
    let net = Arc::new(Net::load(&std::fs::read(path).expect("the policy")).expect("a policy"));
    let decks: Vec<Deck> = if twda {
        (0..zvtes::twda::TWDA.len()).map(Deck::twda).collect()
    } else {
        Deck::all().take(14).collect()
    };
    let group = format!(
        "self-play ({seats} seats{}, t {temperature}, nudges rescue {} pass {} reaction {} bleed {} idle {} draw {} stuck {} nightmare {})",
        if twda { ", TWDA" } else { "" },
        nudges.rescue,
        nudges.pass,
        nudges.reaction,
        nudges.bleed,
        nudges.idle,
        nudges.draw,
        nudges.stuck,
        nudges.nightmare
    );
    for g in 0..games {
        let table: Vec<Deck> = (0..seats)
            .map(|i| decks[((g * 5 + i * 3) % decks.len() as u64) as usize])
            .collect();
        let mut e = Engine::with_decks(9000 + g, &table);
        e.time_limit = 75;
        e.notes = Some(Vec::new());
        let mut agents: Vec<NetAgent> = (0..seats)
            .map(|s| {
                NetAgent::new(net.clone(), g * 10 + s)
                    .temperature(temperature)
                    .nudges(nudges)
            })
            .collect();
        let groups = table
            .iter()
            .map(|d| match by_deck {
                true => format!("{group}, {}", d.name),
                false => group.clone(),
            })
            .collect();
        let mut watch = Watch::new(groups);
        while let Step::Decision(req) = e.run() {
            let i = agents[req.seat.index()].decide(&e, &req);
            let notes = e.take_notes().unwrap_or_default();
            watch.see(&e, &req, i, &notes, counts);
            e.choose(i);
            if e.decisions > 30_000 {
                break;
            }
        }
        watch.finish(&mut e, counts);
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let usage = "usage: waste <policy.zvp> <games> [--t T] [--rescue B] [--pass B] [--reaction B] [--bleed B] [--idle B] [--draw B] [--stuck B] [--nightmare B] [--seats N] [--twda] [--by-deck] [--cards N] [record.json | records dir]... (`- 0`: the records only, no network)";
    let policy = args.next().expect(usage);
    let games: u64 = args.next().and_then(|n| n.parse().ok()).expect(usage);
    let (mut temperature, mut nudges, mut replays, mut seats, mut twda, mut by_deck) =
        (1.0, Nudges::default(), Vec::new(), 4, false, false);
    let mut cards = 0;
    while let Some(a) = args.next() {
        let mut num = || {
            args.next()
                .and_then(|x| x.parse::<f32>().ok())
                .expect(usage)
        };
        match a.as_str() {
            "--t" => temperature = num(),
            "--rescue" => nudges.rescue = num(),
            "--pass" => nudges.pass = num(),
            "--reaction" => nudges.reaction = num(),
            "--bleed" => nudges.bleed = num(),
            "--idle" => nudges.idle = num(),
            "--draw" => nudges.draw = num(),
            "--stuck" => nudges.stuck = num(),
            "--nightmare" => nudges.nightmare = num(),
            "--seats" => seats = num() as u64,
            "--twda" => twda = true,
            "--by-deck" => by_deck = true,
            "--cards" => cards = num() as usize,
            // A directory: its records (a harvest).
            _ => match std::fs::read_dir(&a) {
                Ok(dir) => {
                    let mut files: Vec<String> = dir
                        .filter_map(|f| f.ok().map(|f| f.path()))
                        .filter(|p| p.extension().is_some_and(|x| x == "json"))
                        .map(|p| p.display().to_string())
                        .collect();
                    files.sort();
                    replays.extend(files);
                }
                Err(_) => replays.push(a),
            },
        }
    }
    let mut counts = BTreeMap::new();
    recorded_games(&replays, by_deck, &mut counts);
    if games > 0 {
        let play = SelfPlay {
            games,
            seats,
            twda,
            temperature,
            nudges,
        };
        self_play(&policy, &play, by_deck, &mut counts);
    }
    let pct = |a: u64, b: u64| {
        if b == 0 {
            0.0
        } else {
            100.0 * a as f64 / b as f64
        }
    };
    for (g, c) in &counts {
        println!("{g}:");
        println!(
            "  rescues offered {}, taken {:.1}%",
            c.rescue_offered,
            pct(c.rescued, c.rescue_offered)
        );
        println!(
            "  bleeds for nothing offered {}, taken {:.1}%",
            c.empty_bleed_offered,
            pct(c.empty_bled, c.empty_bleed_offered)
        );
        println!(
            "  transfers decided with a duplicate unique offered {}, onto it {:.1}%; nightmare counters moved {}, onto one's own minion {:.0}%",
            c.dup_offered,
            pct(c.dup_transfers, c.dup_offered),
            c.nightmares,
            pct(c.nightmares_own, c.nightmares)
        );
        println!(
            "  wakes {} ({:.0}% with no block after), unlocks {} ({:.0}% with no action after)",
            c.wakes,
            pct(c.wakes_wasted, c.wakes),
            c.unlocks,
            pct(c.unlocks_wasted, c.unlocks)
        );
        let per = |x: u64| x as f64 / c.discard_phases.max(1) as f64;
        println!(
            "  discard phases {}: hand {:.1}, unplayable {:.2}; holding an unplayable card {:.0}% of them, cycling then {:.0}% (a playable one {:.0}%); cycling overall {:.0}%",
            c.discard_phases,
            per(c.hand),
            per(c.dead),
            pct(c.phases_with_dead, c.discard_phases),
            pct(c.cycled_with_dead, c.phases_with_dead),
            pct(c.kept_dead, c.cycled_with_dead),
            pct(c.cycled, c.discard_phases)
        );
        let hand: u64 = c.hand_types.values().sum();
        let line: Vec<String> = c
            .hand_types
            .iter()
            .map(|(t, &n)| {
                let k = c.cycled_types.get(t).copied().unwrap_or(0);
                format!("{t} {:.0}%/{:.0}%", pct(n, hand), pct(k, c.cycled))
            })
            .collect();
        println!("  types, share of hand / of cycled: {}", line.join(", "));
        println!(
            "  influence with a crypt draw offered and 5+ pool: {}, ended there {:.0}%, drew {:.0}%",
            c.idle_offered,
            pct(c.idle_passed, c.idle_offered),
            pct(c.idle_drawn, c.idle_offered)
        );
        for (name, i) in ["no ready minion", "some ready"].iter().zip(&c.influence) {
            println!(
                "  influence phases, {name}: {}; a transfer or crypt draw possible at the end {}, none made {:.0}% (pool then {:.1}); a vampire left to come out {}",
                i.0,
                i.1,
                pct(i.2, i.1),
                i.3 as f64 / i.2.max(1) as f64,
                i.4
            );
        }
        let per = |a: u64, b: u64| a as f64 / b.max(1) as f64;
        println!(
            "  no seat ousted, adjacent/cross-table: block attempts on its bleeds with stealth offered {}/{}, answered with stealth {:.0}%/{:.0}%; combats {}/{}: cards it played {:.2}/{:.2}, blood the opposing minion lost {:.2}/{:.2}, to torpor or burned {:.0}%/{:.0}%",
            c.stealth_offered[0],
            c.stealth_offered[1],
            pct(c.stealth_played[0], c.stealth_offered[0]),
            pct(c.stealth_played[1], c.stealth_offered[1]),
            c.combats[0],
            c.combats[1],
            per(c.combat_plays[0], c.combats[0]),
            per(c.combat_plays[1], c.combats[1]),
            per(c.combat_damage[0], c.combats[0]),
            per(c.combat_damage[1], c.combats[1]),
            pct(c.combat_down[0], c.combats[0]),
            pct(c.combat_down[1], c.combats[1])
        );
        println!(
            "  bleeds declared, no seat ousted {}: pool taken from its prey {:.2}, from cross-table {:.2} a bleed",
            c.bleeds,
            c.bled[0] as f64 / c.bleeds.max(1) as f64,
            c.bled[1] as f64 / c.bleeds.max(1) as f64
        );
        let turns = c.own_turns.max(1) as f64;
        let (ab, ab_e) = c.ab_ratio.mean();
        let [(own, own_e), (prey, prey_e), (pred, pred_e)] =
            c.resource_ratio.each_ref().map(Ratio::mean);
        println!(
            "  impact: own turns {}; A&B a round {:.2} ±{:.2} (actions {:.2}, blocks landed {:.2}, hindrances {:.2}); R (2 pool + blood) over its own turns: its own {:+.2} ±{:.2}, its prey's {:+.2} ±{:.2}, its predator's {:+.2} ±{:.2}",
            c.own_turns,
            ab,
            ab_e,
            c.actions as f64 / turns,
            c.landed as f64 / turns,
            c.hindrances as f64 / turns,
            own,
            own_e,
            prey,
            prey_e,
            pred,
            pred_e
        );
        let slots = c.hand as f64 / c.discard_phases.max(1) as f64;
        let slot_rounds = (slots * turns).max(1.0);
        println!(
            "  hand: {:.1} slots a round, {:.2} dead; a hand slot a round: A&B {:.3}, its prey's R {:+.3}",
            slots,
            c.dead as f64 / c.discard_phases.max(1) as f64,
            (c.actions + c.landed + c.hindrances) as f64 / slot_rounds,
            c.resource[1] as f64 / slot_rounds
        );
        if cards > 0 {
            let mut rows: Vec<_> = c.cards.iter().collect();
            rows.sort_by(|a, b| b.1[0].total_cmp(&a.1[0]));
            for (&k, s) in rows.into_iter().take(cards) {
                println!(
                    "    {}: held {:.0} rounds, played {:.0} ({:.2} a round held), cycled {:.0}; its prey's R a round held {:+.2}",
                    LIBRARY[k as usize].name,
                    s[0],
                    s[1],
                    s[1] / s[0].max(1.0),
                    s[2],
                    s[3] / s[0].max(1.0)
                );
            }
        }
        println!(
            "  Wider View's draw-and-remove offered {}, used {:.0}% (crypt then {:.1}, under 4 cards {:.0}%); uncontrolled vampires removed {}: blood on them {:.2}, one with less blood kept {:.0}%",
            c.wider_offered,
            pct(c.wider_used, c.wider_offered),
            per(c.wider_crypt, c.wider_used),
            pct(c.wider_low, c.wider_used),
            c.removed,
            per(c.removed_blood, c.removed),
            pct(c.removed_dearer, c.removed)
        );
        println!(
            "  block windows with a reachable attempt {}, attempted {:.0}%; with a futile one {}, taken {:.0}%; only futile ones {}, attempted {:.0}%",
            c.reach_offered,
            pct(c.reach_taken, c.reach_offered),
            c.futile_offered,
            pct(c.futile_taken, c.futile_offered),
            c.only_futile,
            pct(c.only_futile_taken, c.only_futile)
        );
        println!(
            "  own turns with a rush offered {}, one declared {:.0}% ({} rushes); their targets: its prey {:.0}%, its predator {:.0}%, another seat {:.0}%, carrying equipment {:.0}%, a minion that had declared a bleed {:.0}%; blocks {}: of its prey {:.0}%, its predator {:.0}%, another seat {:.0}%",
            c.rush_turns,
            pct(c.rush_turns_taken, c.rush_turns),
            c.rushes,
            pct(c.rush_at[0], c.rushes),
            pct(c.rush_at[1], c.rushes),
            pct(c.rush_at[2], c.rushes),
            pct(c.rush_equipped, c.rushes),
            pct(c.rush_bleeder, c.rushes),
            c.blocks.iter().sum::<u64>(),
            pct(c.blocks[0], c.blocks.iter().sum()),
            pct(c.blocks[1], c.blocks.iter().sum()),
            pct(c.blocks[2], c.blocks.iter().sum())
        );
        println!(
            "  combats {}: cards it played {:.2}, combat cards held as it began {:.2}, some held and none played {:.0}%; blood the opposing minion lost {:.2}, to torpor or burned {:.0}%, its own {:.0}%",
            c.fights,
            per(c.fight_plays, c.fights),
            per(c.fight_held, c.fights),
            pct(c.fight_held_idle, c.fights),
            per(c.fight_damage, c.fights),
            pct(c.fight_down, c.fights),
            pct(c.fight_lost, c.fights)
        );
    }
}
