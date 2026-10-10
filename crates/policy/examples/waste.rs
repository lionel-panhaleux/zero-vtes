//! Cards spent for nothing, and hands not built (owner, 2026-10-05: "it
//! tends to throw away wakes, unlocks … because it cannot construct its
//! hand properly"): the owner's games (`data/replays`, the owner's seats
//! apart from the bots') and self-play of an exported network.
//! `cargo run -p zvtes-policy --profile ci --example waste -- <policy.zvp>
//! <games> [--t T] [--rescue B] [--pass B] [--reaction B] [--bleed B]
//! [--idle B] [replay.json]...`
//! (self-play at temperature T, with those nudges: `zvtes_policy::Nudges`). Per group:
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
//!   fully influenced vampire left uncontrolled.

use std::collections::BTreeMap;
use std::rc::Rc;
use zvtes::agent::Agent;
use zvtes::cards::{self, Effect};
use zvtes::decision::{Choice, DecisionRequest, WindowLabel};
use zvtes::decks::Deck;
use zvtes::game::{Engine, Step};
use zvtes::habits;
use zvtes::ids::{MinionId, SeatId};
use zvtes::library::{CardType, LIBRARY};
use zvtes_policy::{Net, NetAgent, Nudges};
use zvtes_protocol::deck::resolve;
use zvtes_protocol::{Player, Replay};

#[derive(Default, Debug)]
struct Counts {
    /// Influence decisions with a crypt draw offered and 5 pool or more
    /// (`zvtes_policy::idle_influence`), and those ending the phase.
    idle_offered: u64,
    idle_passed: u64,
    idle_drawn: u64,
    empty_bleed_offered: u64,
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
}

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

fn owner_games(paths: &[String], counts: &mut BTreeMap<String, Counts>) {
    for path in paths {
        let replay: Replay =
            serde_json::from_str(&std::fs::read_to_string(path).expect("a replay"))
                .expect("a replay");
        let lists: Vec<_> = replay
            .spec
            .seats
            .iter()
            .map(|s| resolve(&s.deck).expect("a deck"))
            .collect();
        let lists: Vec<_> = lists
            .iter()
            .map(|d| (d.crypt.as_slice(), d.library.as_slice()))
            .collect();
        let mut e = Engine::with_lists(replay.spec.seed, &lists);
        if let Some(t) = replay.spec.time_limit {
            e.time_limit = t;
        }
        let human: Vec<bool> = replay
            .spec
            .seats
            .iter()
            .map(|s| s.player == Player::Human)
            .collect();
        let mut reader = replay.reader();
        let mut watch = Watch::default();
        while let Step::Decision(req) = e.run() {
            let Some(Ok(i)) = reader.next(&e, &req) else {
                break;
            };
            let group = if human[req.seat.index()] {
                "owner"
            } else {
                "bots in the owner's games"
            };
            watch.see(&e, &req, i, group, counts);
            e.choose(i);
        }
    }
}

fn self_play(
    path: &str,
    games: u64,
    temperature: f32,
    nudges: Nudges,
    counts: &mut BTreeMap<String, Counts>,
) {
    let net = Rc::new(Net::load(&std::fs::read(path).expect("the policy")).expect("a policy"));
    let decks: Vec<Deck> = Deck::all().take(14).collect();
    let group = format!(
        "self-play (t {temperature}, nudges rescue {} pass {} reaction {} bleed {} idle {} draw {})",
        nudges.rescue, nudges.pass, nudges.reaction, nudges.bleed, nudges.idle, nudges.draw
    );
    for g in 0..games {
        let seats: Vec<Deck> = (0..4)
            .map(|i| decks[((g * 5 + i * 3) % 14) as usize])
            .collect();
        let mut e = Engine::with_decks(9000 + g, &seats);
        e.time_limit = 75;
        let mut agents: Vec<NetAgent> = (0..4)
            .map(|s| {
                NetAgent::new(net.clone(), g * 10 + s)
                    .temperature(temperature)
                    .nudges(nudges)
            })
            .collect();
        let mut watch = Watch::default();
        while let Step::Decision(req) = e.run() {
            let i = agents[req.seat.index()].decide(&e, &req);
            watch.see(&e, &req, i, &group, counts);
            e.choose(i);
            if e.decisions > 30_000 {
                break;
            }
        }
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let usage = "usage: waste <policy.zvp> <games> [--t T] [--rescue B] [--pass B] [--reaction B] [--bleed B] [--idle B] [--draw B] [replay]...";
    let policy = args.next().expect(usage);
    let games: u64 = args.next().and_then(|n| n.parse().ok()).expect(usage);
    let (mut temperature, mut nudges, mut replays) = (1.0, Nudges::default(), Vec::new());
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
            _ => replays.push(a),
        }
    }
    let mut counts = BTreeMap::new();
    owner_games(&replays, &mut counts);
    self_play(&policy, games, temperature, nudges, &mut counts);
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
    }
}
