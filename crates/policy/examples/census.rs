//! Which decisions the network meets and what it picks (owner,
//! 2026-10-08: options a human may forget but never should — candidates
//! for a default by rule, as the politics): self-play of an exported
//! network; per window and set of option kinds offered, the decisions
//! and the share each kind was picked. `cargo run -p zvtes-policy
//! --profile ci --example census -- <policy.zvp> <games> [filter]`: with a
//! filter, every row whose window or kinds contain it, abilities named by
//! their card.

use std::collections::BTreeMap;
use std::rc::Rc;
use zvtes::agent::Agent;
use zvtes::cards::AbilitySource;
use zvtes::decision::Choice;
use zvtes::decks::Deck;
use zvtes::game::{Engine, Step};
use zvtes::library::LIBRARY;
use zvtes_policy::{Net, NetAgent};

fn kind(o: &Choice) -> String {
    let s = format!("{o:?}");
    s.split([' ', '(', '{']).next().unwrap_or("").to_string()
}

/// `kind`, an ability with its source's name.
fn named(e: &Engine, o: &Choice) -> String {
    match *o {
        Choice::UseAbility {
            source, ability, ..
        } => {
            let st = &e.state;
            let name = match source {
                AbilitySource::Card(c) => LIBRARY[st.cards[c.index()].card as usize].name,
                AbilitySource::Vampire(m) => st.minions[m.index()].name(),
            };
            format!("UseAbility:{name}#{ability}")
        }
        _ => kind(o),
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let usage = "usage: census <policy.zvp> <games>";
    let path = args.next().expect(usage);
    let games: u64 = args.next().and_then(|n| n.parse().ok()).expect(usage);
    let filter = args.next();
    let net = Rc::new(Net::load(&std::fs::read(path).expect("the policy")).expect("a policy"));
    let decks: Vec<Deck> = Deck::all().take(14).collect();
    // (window, kinds offered) -> (decisions, picks by kind)
    let mut seen: BTreeMap<(String, String), (u64, BTreeMap<String, u64>)> = BTreeMap::new();
    let mut total = 0u64;
    for g in 0..games {
        let seats: Vec<Deck> = (0..4)
            .map(|i| decks[((g * 5 + i * 3) % 14) as usize])
            .collect();
        let mut e = Engine::with_decks(7000 + g, &seats);
        e.time_limit = 75;
        let mut agents: Vec<NetAgent> = (0..4)
            .map(|s| NetAgent::new(net.clone(), g * 10 + s))
            .collect();
        while let Step::Decision(req) = e.run() {
            let i = agents[req.seat.index()].decide(&e, &req);
            let name = |o: &Choice| {
                if filter.is_some() {
                    named(&e, o)
                } else {
                    kind(o)
                }
            };
            let mut kinds: Vec<String> = req.options.iter().map(name).collect();
            kinds.sort();
            kinds.dedup();
            let entry = seen
                .entry((format!("{:?}", req.window), kinds.join(",")))
                .or_default();
            entry.0 += 1;
            *entry.1.entry(name(&req.options[i])).or_default() += 1;
            total += 1;
            e.choose(i);
            if e.decisions > 30_000 {
                break;
            }
        }
    }
    let mut rows: Vec<_> = seen.into_iter().collect();
    rows.sort_by_key(|(_, (n, _))| std::cmp::Reverse(*n));
    println!("{total} network decisions");
    let shown = rows.iter().filter(|((w, k), _)| {
        filter
            .as_ref()
            .is_none_or(|f| w.contains(f.as_str()) || k.contains(f.as_str()))
    });
    for ((w, k), (n, picks)) in shown.take(60) {
        let p: Vec<String> = picks
            .iter()
            .map(|(c, m)| format!("{c} {:.0}%", 100.0 * *m as f64 / *n as f64))
            .collect();
        println!(
            "{:5.1}% {n:6} {w:20} [{k}] -> {}",
            100.0 * *n as f64 / total as f64,
            p.join(", ")
        );
    }
}
