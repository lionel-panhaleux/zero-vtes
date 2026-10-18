//! Chores: options a player may forget but never should (owner,
//! 2026-10-08: "some things are only 'options' in the sense a human is
//! allowed to forget them, but it's never good to do so"). When one is
//! offered, a network seat does not decline it: passing is masked, the
//! network still choosing among the rest (which chore first, which
//! vampire). Agent behaviour like the default politics (`zvtes::politics`),
//! not an engine mask (rules-qa 17.4 untouched): every option stays legal.
//!
//! - The Edge's pool in the unlock phase ("you may gain 1 pool"; declining
//!   it to keep a Parity Shift playable is the super-rare exception the
//!   owner accepted).
//! - A hunting ground's blood to a ready vampire with room for it (free:
//!   no lock, no cost).
//! - The optional effects the rulings call never worse to use, as the
//!   scripted bots take them: Fever Pitch put on its vampire (rules-qa
//!   53.5), Cursed Abattoir's counter (55.6). Forward Momentum's counter
//!   is no option (the engine adds it; only its order with the unlock is
//!   chosen, 53.6).
//!
//! Self-play (`examples/census.rs`, `p7-politics@1800`, 32 games): the
//! network took the Edge's pool 97% of the time, but declined the hunting
//! grounds 26–33%. Reads only public state.

use zvtes::cards::{AbilitySource, Effect, Target};
use zvtes::decision::{Choice, DecisionRequest, WindowLabel};
use zvtes::draws::Interrupt;
use zvtes::game::Engine;

/// `o` is a chore for `req`'s seat.
pub fn is_chore(e: &Engine, req: &DecisionRequest, o: &Choice) -> bool {
    let st = &e.state;
    if req.window == WindowLabel::ResolutionChoice
        && matches!(
            e.interrupt(),
            Some(
                Interrupt::MayPutOn { .. }
                    | Interrupt::MayAddCounter { .. }
                    | Interrupt::MayStoreAim { .. }
            )
        )
    {
        return *o != Choice::Pass;
    }
    match *o {
        Choice::BurnEdge => true,
        Choice::UseAbility {
            source: source @ AbilitySource::Card(_),
            ability,
            target: Target::Minion(m),
            ..
        } => {
            let room = st
                .capacity_of(m)
                .is_some_and(|c| st.minions[m.index()].blood < c);
            room && source
                .mode(st, ability)
                .effects
                .iter()
                .any(|e| matches!(e, Effect::HuntingGroundBlood(_)))
        }
        _ => false,
    }
}

/// The options left once the chores offered are not declined: all but
/// `Pass` when a chore is offered (`None`: no opinion).
pub fn mask(e: &Engine, req: &DecisionRequest) -> Option<Vec<bool>> {
    if !req.options.iter().any(|o| is_chore(e, req, o)) {
        return None;
    }
    let mask: Vec<bool> = req.options.iter().map(|o| *o != Choice::Pass).collect();
    mask.iter().any(|&b| !b).then_some(mask)
}

#[cfg(test)]
mod tests {
    use super::*;
    use zvtes::agent::{Agent, RandomLegal};
    use zvtes::decks::Deck;
    use zvtes::game::Step;

    /// On random games: a mask only ever withholds `Pass`, and only when a
    /// chore is offered; both chores are met; it reads the legal view.
    #[test]
    fn chores_withhold_only_passing() {
        let (mut edge, mut ground) = (0, 0);
        for seed in 0..60u64 {
            let decks: Vec<Deck> = (0..4)
                .map(|s| Deck::precon(((seed * 3 + s * 5) % 14) as usize))
                .collect();
            let mut e = Engine::with_decks(seed, &decks);
            e.time_limit = 60;
            let mut agents: Vec<RandomLegal> =
                (0..4).map(|s| RandomLegal::new(seed * 10 + s)).collect();
            while let Step::Decision(req) = e.run() {
                if let Some(m) = mask(&e, &req) {
                    for (o, &kept) in req.options.iter().zip(&m) {
                        assert_eq!(kept, *o != Choice::Pass);
                    }
                    edge += req.options.contains(&Choice::BurnEdge) as u32;
                    ground += req
                        .options
                        .iter()
                        .any(|o| is_chore(&e, &req, o) && *o != Choice::BurnEdge)
                        as u32;
                    let mut view = e.clone();
                    view.state = e.state.legal_view(req.seat);
                    assert_eq!(mask(&view, &req), Some(m));
                }
                let i = agents[req.seat.index()].decide(&e, &req);
                e.choose(i);
            }
        }
        assert!(
            edge > 0 && ground > 0,
            "edge {edge}, hunting grounds {ground}"
        );
    }
}
