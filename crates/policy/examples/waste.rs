//! Cards spent for nothing, and hands not built (owner, 2026-10-05: "it
//! tends to throw away wakes, unlocks … because it cannot construct its
//! hand properly"): recorded games (`data/records`, a harvest's
//! `data/server-records`: the humans' seats apart from the bots', by
//! profile and weights) and self-play of an exported network.
//! `cargo run -p zvtes-policy --profile ci --example waste -- <policy.zvp>
//! <games> [--t T] [--rescue B] [--pass B] [--reaction B] [--bleed B]
//! [--idle B] [--seats N] [--twda] [record.json | records dir]...`
//! (self-play at N seats, 4 by default, the precons or with `--twda` the
//! TWDA decks, at temperature T, with those nudges:
//! `zvtes_policy::Nudges`; none with 0 games, the policy then unread:
//! `waste - 0 data/server-records`). Per group:
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
//!   futile ones (the windows elided before E25) and those attempted.

use std::collections::BTreeMap;
use std::sync::Arc;
use zvtes::actions::ActionState;
use zvtes::agent::Agent;
use zvtes::cards::{self, AbilitySource, Effect};
use zvtes::decision::{ActionKind, Choice, DecisionRequest, WindowLabel};
use zvtes::decks::Deck;
use zvtes::draws::Interrupt;
use zvtes::game::{Engine, Step};
use zvtes::habits;
use zvtes::ids::{MinionId, SeatId};
use zvtes::library::{CardType, LIBRARY};
use zvtes_api::record::GameRecord;
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
}

fn is_wider(e: &Engine, o: &Choice) -> bool {
    matches!(*o, Choice::UseAbility { source: AbilitySource::Card(c), ability: 0, .. }
        if c != zvtes::state::ASH_PLAY
            && LIBRARY[e.state.cards[c.index()].card as usize].name == "Wider View")
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

/// A combat under way: (turn, its minions), their seats, `relation`, the
/// blood each had when first seen, the cards each side played.
struct Fight {
    id: (u32, MinionId, MinionId),
    seats: [SeatId; 2],
    rel: usize,
    blood: [u8; 2],
    plays: [u64; 2],
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
            groups,
            ..Watch::default()
        }
    }

    /// What happened since the last decision: the pool a bleed took, a
    /// combat over.
    fn settle(&mut self, e: &Engine, counts: &mut BTreeMap<String, Counts>) {
        let st = &e.state;
        if let Some((x, before)) = self.bleeding.take() {
            let c = counts.entry(self.groups[x.index()].clone()).or_default();
            for (s, &(rel, pool)) in before.iter().enumerate() {
                if let Some(r) = rel {
                    c.bled[r] += (pool - st.seats[s].pool).max(0) as i64;
                }
            }
        }
        let now = e
            .combat()
            .map(|c| (st.turn, c.sides[0].minion, c.sides[1].minion));
        if self.combat.as_ref().is_some_and(|f| Some(f.id) != now) {
            let f = self.combat.take().unwrap();
            let minions = [f.id.1, f.id.2];
            for i in 0..2 {
                let them = &st.minions[minions[1 - i].index()];
                let c = counts
                    .entry(self.groups[f.seats[i].index()].clone())
                    .or_default();
                c.combats[f.rel] += 1;
                c.combat_plays[f.rel] += f.plays[i];
                c.combat_damage[f.rel] += f.blood[1 - i].saturating_sub(them.blood) as u64;
                c.combat_down[f.rel] += (them.in_torpor || !them.in_play) as u64;
            }
        }
        if let (None, Some(id), Some(c)) = (&self.combat, now, e.combat()) {
            let seats = [c.sides[0].seat, c.sides[1].seat];
            if let Some(rel) = relation(e, seats[0], seats[1]) {
                self.combat = Some(Fight {
                    id,
                    seats,
                    rel,
                    blood: [id.1, id.2].map(|m| st.minions[m.index()].blood),
                    plays: [0; 2],
                });
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
                action: ActionKind::Bleed,
                ..
            } => true,
            Choice::Play { card, mode, .. } => {
                effects(e, me, card, mode).contains(&Effect::DeclareAction(ActionKind::Bleed))
            }
            _ => false,
        };
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
    /// Before `chosen` is applied: settle what this decision shows, then
    /// record what it plays.
    fn see(
        &mut self,
        e: &Engine,
        req: &DecisionRequest,
        chosen: usize,
        group: &str,
        counts: &mut BTreeMap<String, Counts>,
    ) {
        let me = req.seat;
        let key = action_key(e);
        let o = req.options[chosen];
        self.settle(e, counts);
        self.cross(e, req, o, counts);
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
fn recorded_games(paths: &[String], counts: &mut BTreeMap<String, Counts>) {
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
        e.notes = None;
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
            watch.see(&e, &req, i, &groups[req.seat.index()], counts);
            e.choose(i);
        }
        watch.settle(&e, counts);
    }
}

fn self_play(
    path: &str,
    games: u64,
    seats: u64,
    twda: bool,
    temperature: f32,
    nudges: Nudges,
    counts: &mut BTreeMap<String, Counts>,
) {
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
        let mut agents: Vec<NetAgent> = (0..seats)
            .map(|s| {
                NetAgent::new(net.clone(), g * 10 + s)
                    .temperature(temperature)
                    .nudges(nudges)
            })
            .collect();
        let mut watch = Watch::new(vec![group.clone(); seats as usize]);
        while let Step::Decision(req) = e.run() {
            let i = agents[req.seat.index()].decide(&e, &req);
            watch.see(&e, &req, i, &group, counts);
            e.choose(i);
            if e.decisions > 30_000 {
                break;
            }
        }
        watch.settle(&e, counts);
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let usage = "usage: waste <policy.zvp> <games> [--t T] [--rescue B] [--pass B] [--reaction B] [--bleed B] [--idle B] [--draw B] [--stuck B] [--nightmare B] [--seats N] [--twda] [record.json | records dir]... (`- 0`: the records only, no network)";
    let policy = args.next().expect(usage);
    let games: u64 = args.next().and_then(|n| n.parse().ok()).expect(usage);
    let (mut temperature, mut nudges, mut replays, mut seats, mut twda) =
        (1.0, Nudges::default(), Vec::new(), 4, false);
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
    recorded_games(&replays, &mut counts);
    if games > 0 {
        self_play(
            &policy,
            games,
            seats,
            twda,
            temperature,
            nudges,
            &mut counts,
        );
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
    }
}
