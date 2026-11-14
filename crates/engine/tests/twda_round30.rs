//! Round 30 (`ADDED_BATCHES`): Lords of the Night completed. Windows with a
//! single option are elided, so the scripts only stop where a real decision
//! exists.

use zvtes::cards::{AbilitySource, Target};
use zvtes::combat::Range;
use zvtes::crypt::{Sect, Title, CRYPT};
use zvtes::decision::{ActionKind, Choice, WindowLabel};
use zvtes::game::Engine;
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::{find_library, CardId, CardInPlay};

mod common;
use common::*;

// ---- Helpers ----------------------------------------------------------------

/// `n` seats at seat 0's minion phase, turn 4, 10 pool each.
fn game(n: usize) -> Engine {
    let mut e = Engine::bare(n);
    for s in &mut e.state.seats {
        s.pool = 10;
    }
    e.state.turn = 4;
    e
}

/// The `nth` untitled vampire matching `f` with no printed-text
/// definition.
#[track_caller]
fn textless(f: impl Fn(&zvtes::crypt::CryptCard) -> bool, nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            f(c) && c.title == Title::None
                && !c.imbued
                && zvtes::cards::vampire(*i as u16).is_none()
        })
        .nth(nth)
        .map(|(i, _)| i as u16)
        .expect("a textless vampire")
}

/// The `nth` textless vampire of `clan` with `disc` (any level) and a
/// capacity of 4 or more.
#[track_caller]
fn of_clan(clan: &str, disc: &str, nth: usize) -> u16 {
    textless(
        |c| {
            c.clan == clan
                && c.capacity >= 4
                && c.disciplines
                    .iter()
                    .any(|d| d.eq_ignore_ascii_case(disc) || disc.is_empty())
        },
        nth,
    )
}

/// A card already in play for `seat`, on `host` if given, with its first
/// mode's layers (no play): its id.
fn in_play(e: &mut Engine, seat: u8, name: &str, host: Option<MinionId>) -> CardId {
    in_play_mode(e, seat, name, host, 0)
}

/// As `in_play`, in mode `mode`, with that mode's layers.
fn in_play_mode(e: &mut Engine, seat: u8, name: &str, host: Option<MinionId>, mode: u8) -> CardId {
    let card = find_library(name);
    e.state
        .cards
        .push(CardInPlay::new(card, SeatId(seat), host, mode));
    let id = CardId(e.state.cards.len() as u32 - 1);
    for effect in zvtes::cards::lookup(card).expect("definition").modes[mode as usize].effects {
        if let zvtes::cards::Effect::Layer(modifier) = effect {
            e.state.layers.push(zvtes::cards::Layer {
                scope: zvtes::cards::Scope::InPlay,
                modifier: *modifier,
                source: id,
            });
        }
    }
    id
}

/// The card `name` entered play (itself, not a stand-in) on `host`, out of
/// every ash heap: its id.
#[track_caller]
fn placed_on(e: &Engine, name: &str, host: Option<MinionId>) -> CardId {
    let card = find_library(name);
    assert!(
        e.state.seats.iter().all(|s| !s.ash.contains(&card)),
        "{name} in an ash heap"
    );
    let i = e
        .state
        .cards
        .iter()
        .position(|c| c.card == card && c.in_play && !c.effect_only && c.host == host)
        .unwrap_or_else(|| panic!("{name} not in play on {host:?}"));
    CardId(i as u32)
}

/// The card `card` of the hand played by `actor` in `mode` on `target`.
fn play_on(card: u8, mode: u8, actor: MinionId, target: Target) -> Choice {
    Choice::Play {
        card,
        mode,
        actor: Some(actor),
        target,
    }
}

/// The ability `ability` of the card in play `source`, used by `actor`.
fn use_on(source: CardId, ability: u8, actor: Option<MinionId>, target: Target) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Card(source),
        ability,
        actor,
        target,
    }
}

fn ash_has(e: &Engine, seat: usize, name: &str) -> bool {
    e.state.seats[seat].ash.contains(&find_library(name))
}

/// The prey's minions offered a block in the current request.
fn offers_block(req: &zvtes::decision::DecisionRequest, m: MinionId) -> bool {
    req.options.contains(&block(m))
}

// ---- Torpor, blood and unlocks -------------------------------------------------

/// Resume the Coil [nec]: "Only usable by a vampire in torpor. Leave
/// torpor." No blood paid.
/// [ANK 20181017] Not the default action: no blood paid to leave torpor.
#[test]
fn resume_the_coil_leaves_torpor_for_free() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("nec", 0), 1, false);
    e.state.minions[a.index()].in_torpor = true;
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Resume the Coil"]);
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].in_torpor);
    assert_eq!(blood(&e, a), 1, "nothing paid");
    assert!(ash_has(&e, 0, "Resume the Coil"));
}

/// Resume the Coil [nec] blocked: "there is no combat and if the blocking
/// minion can commit diablerie, they can diablerize this acting vampire."
#[test]
fn resume_the_coil_blocked_offers_the_diablerie() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("nec", 0), 1, false);
    e.state.minions[a.index()].in_torpor = true;
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), vampire_with("aus", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Resume the Coil"]);
    hand(&mut e, 1, &["Spirit's Touch"]);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play_on(0, 0, b, Target::None));
    let req = until(&mut e, SeatId(1), WindowLabel::TorporBlock);
    assert_eq!(req.window, WindowLabel::TorporBlock);
    assert!(req.options.contains(&Choice::Diablerise));
    assert!(e.combat().is_none(), "no combat");
}

/// Resume the Coil [NEC]: "Rescue a vampire from torpor", for free, another
/// Methuselah's too.
/// [ANK 20181017] No blood paid to rescue from torpor.
#[test]
fn resume_the_coil_superior_rescues_for_free() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("NEC", 0), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 1, false);
    e.state.minions[t.index()].in_torpor = true;
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Resume the Coil"]);
    pick_when(&mut e, play_on(0, 1, a, Target::Minion(t)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[t.index()].in_torpor);
    assert_eq!((blood(&e, a), blood(&e, t)), (2, 1));
}

/// Resume the Coil, out of torpor before it resolves: either mode fizzles.
/// [nec]: its vampire raised as the block window opens; [NEC]: its target,
/// a Nosferatu, by Warsaw Station.
/// [ANK 20220218] Only usable in torpor: leaving torpor before resolution
/// fizzles it. [ANK 20210124] [LSJ 20070411] [LSJ 20090514] The target no
/// longer in torpor: it fizzles.
#[test]
fn resume_the_coil_fizzles_once_its_vampire_left_torpor() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("nec", 0), 2, true);
    e.add_minion(SeatId(0), plain("Brujah", 2), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Resume the Coil"]);
    pick(&mut e, play(0, 0, a));
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::ActionBlock);
    e.state.minions[a.index()].in_torpor = false;
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Fizzled));

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("NEC", 0), 2, false);
    let t = e.add_minion(SeatId(0), plain("Nosferatu", 0), 1, true);
    e.add_minion(SeatId(0), plain("Brujah", 2), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let w = in_play(&mut e, 0, "Warsaw Station", None);
    hand(&mut e, 0, &["Resume the Coil"]);
    pick(&mut e, play_on(0, 1, a, Target::Minion(t)));
    pick_when(&mut e, use_on(w, 1, None, Target::Minion(t)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Fizzled));
    assert!(!e.state.minions[t.index()].in_torpor);
}

/// Sheepdog: "This vampire gains 4 blood. Put this card on this vampire.
/// This vampire does not unlock as normal. Burn this card during your
/// unlock phase." The next unlock skipped, the card burned in it; the one
/// after unlocks.
#[test]
fn sheepdog_feeds_four_and_holds_the_next_unlock() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity == 7, 0), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Sheepdog"]);
    pick_when(&mut e, play(0, 0, a));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 5);
    let dog = placed_on(&e, "Sheepdog", Some(a));
    e.choose(index_of(&req, Choice::Pass));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[a.index()].locked, "did not unlock");
    assert!(!e.state.cards[dog.index()].in_play, "burned");
    assert!(ash_has(&e, 0, "Sheepdog"));
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        !e.state.minions[a.index()].locked,
        "unlocks as normal again"
    );
}

/// Sheepdog on an infernal vampire: its controller may still pay a pool
/// to unlock it.
/// [LSJ 20050114] "Does not unlock as normal" is redundant with infernal:
/// the infernal pool unlock stays.
#[test]
fn sheepdog_leaves_an_infernal_vampire_its_paid_unlock() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity == 7, 0), 1, false);
    e.state.minions[a.index()].made_infernal = true;
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Sheepdog"]);
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    placed_on(&e, "Sheepdog", Some(a));
    assert!(offered_before(
        &mut e,
        Choice::UnlockInfernal { minion: a },
        SeatId(0),
        WindowLabel::MinionPhase
    ));
}

/// Whispers from the Dead [nec]: the ash heap card named as the action is
/// declared, in hand on success, 1 blood paid; a full hand discards down.
/// [LSJ 20010627] The card announced when declaring the action.
#[test]
fn whispers_from_the_dead_fetches_the_card_named_as_declared() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("nec", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Whispers from the Dead"]);
    fill_hand(&mut e, 0);
    e.state.seats[0].library = vec![find_library("Form of Mist"); 5].into();
    let (conceal, deflection) = (find_library("Conceal"), find_library("Deflection"));
    e.state.seats[0].ash = vec![conceal, deflection].into();
    pick(&mut e, play(0, 0, a));
    let req = request(&mut e);
    assert!(req.options.contains(&Choice::FindCard(conceal)));
    e.choose(index_of(&req, Choice::FindCard(deflection)));
    let req = until(&mut e, SeatId(0), WindowLabel::DiscardDown);
    assert!(e.state.seats[0].hand.contains(&deflection), "{req:?}");
    assert!(e.state.seats[0].ash.contains(&conceal));
    assert_eq!(blood(&e, a), 3, "1 blood");
}

/// Whispers from the Dead [NEC]: "this is a +1 stealth action": a block
/// needing 1 intercept is futile.
#[test]
fn whispers_from_the_dead_superior_has_stealth() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("NEC", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Whispers from the Dead"]);
    let conceal = find_library("Conceal");
    e.state.seats[0].ash = vec![conceal].into();
    pick(&mut e, play(0, 1, a));
    never_reaching_until(&mut e, SeatId(1), WindowLabel::MinionPhase, b);
    assert!(e.state.seats[0].hand.contains(&conceal));
}

// ---- Locks and steals ---------------------------------------------------------

/// Revelation of Ecstasy [ser] on a locked minion of the prey: success,
/// still locked.
/// [ANK 20170303] Can target a locked minion.
#[test]
fn revelation_of_ecstasy_locks_a_prey_minion_even_locked() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), vampire_with("ser", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let u = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[t.index()].locked = true;
    let other = e.add_minion(SeatId(2), plain("Ventrue", 2), 4, false);
    hand(&mut e, 0, &["Revelation of Ecstasy"]);
    let req = request(&mut e);
    assert!(req.options.contains(&play_on(0, 0, a, Target::Minion(t))));
    assert!(req.options.contains(&play_on(0, 0, a, Target::Minion(u))));
    assert!(
        !req.options
            .contains(&play_on(0, 0, a, Target::Minion(other))),
        "the prey's only"
    );
    pick(&mut e, play_on(0, 0, a, Target::Minion(t)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Successful));
    assert!(ash_has(&e, 0, "Revelation of Ecstasy"));
}

/// Revelation of Ecstasy [SER]: a second corruption counter on a capacity-2
/// prey vampire of another clan and sect: the steal offered, taken for
/// good.
/// [TOM 19960226-1] The card's requirements do not apply to the minion
/// stolen.
#[test]
fn revelation_of_ecstasy_superior_steals_at_capacity_without_requirements() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("SER", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(
        SeatId(1),
        textless(|c| c.capacity == 2 && c.clan != "Ministry", 0),
        2,
        false,
    );
    e.state.minions[t.index()].add_corruption(SeatId(0), 1);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Revelation of Ecstasy"]);
    pick(&mut e, play_on(0, 1, a, Target::Minion(t)));
    let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert_eq!(req.options, vec![Choice::Pass, Choice::ChooseMinion(t)]);
    e.choose(index_of(&req, Choice::ChooseMinion(t)));
    request(&mut e);
    assert_eq!(e.state.minions[t.index()].owner, SeatId(0));
    assert!(e.state.minions[t.index()].locked);
}

/// The `nth` textless vampire with every Discipline of `discs` (case read:
/// "AUS" superior), capacity 4 or more.
#[track_caller]
fn with_discs(discs: &[&str], nth: usize) -> u16 {
    textless(
        |c| c.capacity >= 4 && discs.iter().all(|d| c.disciplines.contains(d)),
        nth,
    )
}

/// Nightmare Curse [chi]: "Unlock this vampire and lock a ready minion":
/// a locked one too, the prey's.
#[test]
fn nightmare_curse_unlocks_the_actor_and_locks_a_locked_target_too() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("chi", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[t.index()].locked = true;
    let torpid = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, true);
    hand(&mut e, 0, &["Nightmare Curse"]);
    let req = request(&mut e);
    assert!(
        !req.options
            .contains(&play_on(0, 0, a, Target::Minion(torpid))),
        "ready"
    );
    pick(&mut e, play_on(0, 0, a, Target::Minion(t)));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].locked, "{req:?}");
    assert!(e.state.minions[t.index()].locked);
    assert!(ash_has(&e, 0, "Nightmare Curse"));
}

/// Nightmare Curse [aus][chi]: the card on the prey's vampire, locked; in
/// its controller's unlock phase it stays locked, and that Methuselah may
/// burn 1 pool to unlock it.
#[test]
fn nightmare_curse_holds_its_minion_for_a_pool() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_discs(&["aus", "chi"], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Nightmare Curse"]);
    pick(&mut e, play_on(0, 1, a, Target::Minion(t)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let curse = placed_on(&e, "Nightmare Curse", Some(t));
    assert!(e.state.minions[t.index()].locked);
    let mut used = false;
    loop {
        let req = request(&mut e);
        if (req.seat, req.window) == (SeatId(1), WindowLabel::MinionPhase) {
            break;
        }
        if let Some(&u) = uses(&req, curse.0).first() {
            assert_eq!(req.seat, SeatId(1), "the attached minion's controller");
            assert!(e.state.minions[t.index()].locked, "not unlocked as normal");
            pick(&mut e, u);
            used = true;
            continue;
        }
        e.choose(fallback(&req));
    }
    assert!(used);
    assert!(!e.state.minions[t.index()].locked);
    assert_eq!(e.state.seats[1].pool, 9);
}

/// Nightmare Curse, two copies on one minion: each copy's pool unlock is
/// its own, the minion unlocked once both are paid; beside an [AUS][CHI]
/// copy, the [aus][chi] copy's pool unlocks nothing.
/// [Advanced rules 1.15] "Copies attached to one minion are never merged:
/// two Kindred Society Games on a vampire cost 2 blood to unlock."
#[test]
fn nightmare_curse_two_copies_each_need_their_own_pool() {
    let run = |second_mode: u8| {
        let mut e = game(2);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let one = in_play_mode(&mut e, 0, "Nightmare Curse", Some(t), 1);
        let two = in_play_mode(&mut e, 0, "Nightmare Curse", Some(t), second_mode);
        e.state.minions[t.index()].locked = true;
        let mut paid = Vec::new();
        loop {
            let req = request(&mut e);
            if (req.seat, req.window) == (SeatId(1), WindowLabel::MinionPhase) {
                break;
            }
            if let Some(&u) = uses(&req, one.0).first().or(uses(&req, two.0).first()) {
                assert!(
                    e.state.minions[t.index()].locked,
                    "{paid:?} paid: still locked"
                );
                if let Choice::UseAbility { source, .. } = u {
                    paid.push(source);
                }
                pick(&mut e, u);
                continue;
            }
            e.choose(fallback(&req));
        }
        (e, t, paid)
    };
    let (e, t, paid) = run(1);
    assert_eq!(paid.len(), 2);
    assert!(!e.state.minions[t.index()].locked, "both paid");
    assert_eq!(e.state.seats[1].pool, 8);
    let (e, t, paid) = run(2);
    assert_eq!(paid.len(), 1, "the [aus][chi] copy only");
    assert!(
        e.state.minions[t.index()].locked,
        "the [AUS][CHI] copy holds it"
    );
    assert_eq!(e.state.seats[1].pool, 9);
}

/// Nightmare Curse [AUS][CHI]: no pool unlock; the minion stays locked.
#[test]
fn nightmare_curse_superior_offers_no_pool_unlock() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_discs(&["AUS", "CHI"], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Nightmare Curse"]);
    pick(&mut e, play_on(0, 2, a, Target::Minion(t)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let curse = placed_on(&e, "Nightmare Curse", Some(t));
    never_offered_where(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), .. } if *c == curse),
    );
    assert!(e.state.minions[t.index()].locked);
}

/// Nightmare Curse: "Burn this card after this acting vampire leaves the
/// ready region": its vampire burned, the card burns, its minion still
/// locked.
#[test]
fn nightmare_curse_burns_when_its_vampire_leaves_the_ready_region() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_discs(&["AUS", "CHI"], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Nightmare Curse"]);
    pick(&mut e, play_on(0, 2, a, Target::Minion(t)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let curse = placed_on(&e, "Nightmare Curse", Some(t));
    e.burn(a);
    assert!(!e.state.cards[curse.index()].in_play, "burned");
    assert!(ash_has(&e, 0, "Nightmare Curse"));
    assert!(e.state.minions[t.index()].locked);
}

/// Temptation: put on the prey's 2-blood vampire; a counter each master
/// phase (no master phase action spent); at 2 counters, in the minion
/// phase, the steal: counters burned, the vampire unlocked and seat 0's
/// until the end of the turn, then back, the card still on it.
/// [LSJ 20001201] At any point of the minion phase: after a first action.
#[test]
fn temptation_counts_up_and_steals_until_the_end_of_the_turn() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("ser", 0), 4, false);
    let a2 = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 2, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Temptation"]);
    pick(&mut e, play_on(0, 0, a, Target::Minion(t)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let card = placed_on(&e, "Temptation", Some(t));
    assert_eq!(
        e.state.card_controller(card),
        SeatId(0),
        "you still control"
    );
    let steal = use_on(card, 1, None, Target::None);
    for turn in 0..2 {
        let req = until(&mut e, SeatId(0), WindowLabel::MasterPhase);
        let mpa = e.state.seats[0].mpa;
        e.choose(index_of(&req, use_on(card, 0, None, Target::None)));
        assert_eq!(e.state.seats[0].mpa, mpa, "no master phase action");
        assert_eq!(e.state.cards[card.index()].counters, turn + 1);
        let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        if turn == 0 {
            assert!(!req.options.contains(&steal), "1 counter, 2 blood");
            e.choose(index_of(&req, Choice::Pass));
        }
    }
    pick(&mut e, action(a2, ActionKind::Hunt));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.choose(index_of(&req, steal));
    let v = &e.state.minions[t.index()];
    assert_eq!((v.owner, v.locked), (SeatId(0), false));
    assert_eq!(e.state.cards[card.index()].counters, 0);
    let req = request(&mut e);
    assert!(req.options.contains(&bleed(t)), "acts for its taker");
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[t.index()].owner, SeatId(1));
    placed_on(&e, "Temptation", Some(t));
}

/// Temptation with no counters steals a vampire with no blood.
/// [LSJ 20010828-2] 0 counters against 0 blood: the steal allowed.
#[test]
fn temptation_with_no_counters_steals_an_empty_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("ser", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 0, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Temptation"]);
    pick(&mut e, play_on(0, 0, a, Target::Minion(t)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let card = placed_on(&e, "Temptation", Some(t));
    e.state.minions[t.index()].locked = true;
    let req = request(&mut e);
    e.choose(index_of(&req, use_on(card, 1, None, Target::None)));
    let v = &e.state.minions[t.index()];
    assert_eq!((v.owner, v.locked), (SeatId(0), false));
}

/// Temptation [SER]: "this card comes into play with 1 counter".
#[test]
fn temptation_superior_enters_with_a_counter() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("SER", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 1, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Temptation"]);
    pick(&mut e, play_on(0, 1, a, Target::Minion(t)));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let card = placed_on(&e, "Temptation", Some(t));
    assert_eq!(e.state.cards[card.index()].counters, 1);
    assert!(req.options.contains(&use_on(card, 1, None, Target::None)));
    assert!(req.options.contains(&Choice::Pass), "\"you can\": not due");
}

/// Temptation [SER] on one's own locked vampire with 1 blood:
/// undirected (no cross-table block offered), its steal used to unlock it.
/// [LSJ 20010828] [ANK 20180917] On one's own vampire: undirected; its
/// effect can unlock the vampire.
#[test]
fn temptation_on_ones_own_vampire_is_undirected_and_unlocks_it() {
    let mut e = game(4);
    let a = e.add_minion(SeatId(0), vampire_with("SER", 0), 4, false);
    let own = e.add_minion(SeatId(0), plain("Brujah", 1), 1, false);
    e.state.minions[own.index()].locked = true;
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let cross = e.add_minion(SeatId(2), vampire_with("aus", 1), 4, false);
    e.add_minion(SeatId(3), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Temptation"]);
    hand(&mut e, 2, &["Spirit's Touch"]);
    pick(&mut e, play_on(0, 1, a, Target::Minion(own)));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(cross));
    let card = placed_on(&e, "Temptation", Some(own));
    pick(&mut e, use_on(card, 1, None, Target::None));
    let v = &e.state.minions[own.index()];
    assert_eq!((v.owner, v.locked), (SeatId(0), false));
}

/// Two Temptations on one vampire count apart: one at the vampire's blood
/// steals; the other keeps its counters.
/// [LSJ 20010227] Multiple Temptations on one minion are handled
/// separately.
#[test]
fn temptation_copies_count_their_counters_apart() {
    let mut e = game(2);
    e.add_minion(SeatId(0), vampire_with("ser", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 2, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let one = in_play(&mut e, 0, "Temptation", Some(t));
    let two = in_play(&mut e, 0, "Temptation", Some(t));
    e.state.cards[one.index()].counters = 1;
    e.state.cards[two.index()].counters = 2;
    let req = request(&mut e);
    assert!(!req.options.contains(&use_on(one, 1, None, Target::None)));
    e.choose(index_of(&req, use_on(two, 1, None, Target::None)));
    assert_eq!(e.state.minions[t.index()].owner, SeatId(0));
    assert_eq!(e.state.cards[one.index()].counters, 1);
    assert_eq!(e.state.cards[two.index()].counters, 0);
}

/// Temptation used on a blocker: the block fails, the bleed succeeds; on
/// a combatant: the combat ends.
/// [LSJ 20080326] A minion stolen while attempting to block: the attempt
/// fails. [LSJ 20020618] Stolen in combat: combat ends, pending damage
/// first.
#[test]
fn temptation_steals_a_blocker_or_a_combatant() {
    for in_combat in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("ser", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 2, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let card = in_play(&mut e, 0, "Temptation", Some(t));
        e.state.cards[card.index()].counters = 2;
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(t));
        if in_combat {
            until(&mut e, SeatId(0), WindowLabel::CombatStrike);
            pick(&mut e, Choice::HandStrike);
            pick_when(&mut e, use_on(card, 1, None, Target::None));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
            assert!(e.combat().is_none(), "combat ended");
        } else {
            pick_when(&mut e, use_on(card, 1, None, Target::None));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(e.state.seats[1].pool, 9, "the bleed succeeded");
        }
        assert_eq!(e.state.minions[t.index()].owner, SeatId(0));
    }
}

/// Temptation used on the target of seat 0's own action (Revelation of
/// Ecstasy at the prey's minion): no longer the prey's, the action fizzles.
/// [LSJ 20020725] The action continues with the same target; illegal now,
/// it fizzles.
#[test]
fn temptation_stealing_the_target_fizzles_the_action() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("ser", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 2, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let card = in_play(&mut e, 0, "Temptation", Some(t));
    e.state.cards[card.index()].counters = 2;
    hand(&mut e, 0, &["Revelation of Ecstasy"]);
    pick(&mut e, play_on(0, 0, a, Target::Minion(t)));
    pick_when(&mut e, use_on(card, 1, None, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Fizzled));
    assert_eq!(e.state.minions[t.index()].owner, SeatId(0));
}

/// Temptation's control, temporary: its Methuselah ousted meanwhile, the
/// vampire is removed at the end of the turn.
/// [ANK 20181110] [LSJ 20081105] [LSJ 20010119] Temporary control: removed
/// when it should return to an ousted Methuselah.
#[test]
fn temptation_temporary_control_and_ousts() {
    let mut e = game(3);
    e.add_minion(SeatId(0), vampire_with("ser", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 2, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 2), 4, false);
    let card = in_play(&mut e, 0, "Temptation", Some(t));
    e.state.cards[card.index()].counters = 2;
    pick(&mut e, use_on(card, 1, None, Target::None));
    e.state.seats[1].pool = 1;
    pick_when(&mut e, bleed(t));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[1].ousted);
    assert!(
        e.state.minions[t.index()].in_play,
        "until the end of the turn"
    );
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    assert!(!e.state.minions[t.index()].in_play, "removed, not returned");
    // In torpor at the end of the turn: it returns, in torpor.
    let mut e = game(2);
    e.add_minion(SeatId(0), vampire_with("ser", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 2, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let card = in_play(&mut e, 0, "Temptation", Some(t));
    e.state.cards[card.index()].counters = 2;
    pick(&mut e, use_on(card, 1, None, Target::None));
    e.state.minions[t.index()].in_torpor = true;
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let v = &e.state.minions[t.index()];
    assert!(v.in_play && v.in_torpor);
    assert_eq!(v.owner, SeatId(1));
}

// ---- Hunts ----------------------------------------------------------------------

/// Soul Feasting [nec]: a hunt (+1 blood), or a wraith ally of one's own
/// burned to hunt at +3 (4 blood), undirected.
#[test]
fn soul_feasting_hunts_or_eats_a_wraith_for_three_more() {
    for eat in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(
            SeatId(0),
            textless(|c| c.capacity >= 6 && c.disciplines.contains(&"nec"), 0),
            1,
            false,
        );
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let tye = e.add_ally(SeatId(0), find_library("Tye Cooper"));
        hand(&mut e, 0, &["Soul Feasting"]);
        if eat {
            pick(&mut e, play_on(0, 1, a, Target::Minion(tye)));
        } else {
            pick(&mut e, play(0, 0, a));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, a), if eat { 5 } else { 2 });
        assert_eq!(e.state.minions[tye.index()].in_play, !eat);
    }
}

/// Soul Feasting at the prey's wraith retainer (Nar-Sheptha): directed
/// at its controller, burned, +3 hunt.
#[test]
fn soul_feasting_on_anothers_wraith_retainer_is_directed() {
    let mut e = game(4);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| c.capacity >= 6 && c.disciplines.contains(&"nec"), 0),
        1,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let h = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    e.add_minion(SeatId(3), plain("Ventrue", 2), 4, false);
    let nar = in_play(&mut e, 1, "Nar-Sheptha", Some(h));
    e.state.cards[nar.index()].counters = 1;
    hand(&mut e, 0, &["Soul Feasting"]);
    pick(&mut e, play_on(0, 2, a, Target::Card(nar)));
    request(&mut e);
    let directed = e.action().expect("action").directed_seat(&e.state);
    assert_eq!(directed, Some(SeatId(1)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[nar.index()].in_play);
    assert_eq!(blood(&e, a), 5);
}

/// Soul Feasting [NEC]: "an additional +1 hunt" on either hunt.
#[test]
fn soul_feasting_superior_adds_one_more() {
    for eat in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(
            SeatId(0),
            textless(|c| c.capacity >= 6 && c.disciplines.contains(&"NEC"), 0),
            1,
            false,
        );
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let tye = e.add_ally(SeatId(0), find_library("Tye Cooper"));
        hand(&mut e, 0, &["Soul Feasting"]);
        if eat {
            pick(&mut e, play_on(0, 4, a, Target::Minion(tye)));
        } else {
            pick(&mut e, play(0, 3, a));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, a), if eat { 6 } else { 3 });
    }
}

/// Soul Feasting, a special hunt: once each turn per minion.
/// [RBK action-card-or-card-in-play] "A minion can only perform this
/// special hunt once per turn."
#[test]
fn soul_feasting_is_performed_once_each_turn() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("nec", 0), 1, false);
    let other = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Soul Feasting", "Soul Feasting"]);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.state.minions[a.index()].locked = false;
    pick(&mut e, action(other, ActionKind::Hunt));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(req.options.contains(&bleed(a)), "unlocked");
    assert!(!req.options.contains(&play(0, 0, a)), "once each turn");
}

// ---- Pool and locations ----------------------------------------------------------

/// Haqim's Law: Leadership: each Methuselah controlling one of the oldest
/// ready Banu Haqim gains 2 pool, once: two cap-6 (one locked) give seat 1
/// 2; a torpid cap-6 of seat 2's is not ready; seat 0's younger nothing.
#[test]
fn haqims_law_leadership_pays_each_controller_of_the_oldest_once() {
    let bh = |cap: u8, nth| textless(|c| c.clan == "Banu Haqim" && c.capacity == cap, nth);
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), bh(4, 0), 4, false);
    e.add_minion(SeatId(0), bh(5, 0), 4, false);
    e.add_minion(SeatId(1), bh(6, 0), 4, false);
    let locked = e.add_minion(SeatId(1), bh(6, 1), 4, false);
    e.state.minions[locked.index()].locked = true;
    // No third textless one: Hafsa's text is inert with no block or bleed.
    e.add_minion(SeatId(2), find_card("Hafsa, The Watcher"), 4, true);
    e.add_minion(SeatId(2), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Haqim's Law: Leadership"]);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e), vec![10, 12, 10]);
}

/// Poison the Well of Life [qui]: a cross-table hunting ground burned,
/// the action directed at its controller, whose minion may block.
#[test]
fn poison_the_well_burns_a_hunting_ground_directed_at_its_controller() {
    let mut e = game(4);
    let a = e.add_minion(SeatId(0), vampire_with("qui", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let cross = e.add_minion(SeatId(2), vampire_with("aus", 1), 4, false);
    e.add_minion(SeatId(3), plain("Ventrue", 2), 4, false);
    let park = in_play(&mut e, 2, "Park Hunting Ground", None);
    let elysium = in_play(&mut e, 2, "Elysian Fields", None);
    hand(&mut e, 0, &["Poison the Well of Life"]);
    hand(&mut e, 2, &["Spirit's Touch"]);
    let req = request(&mut e);
    assert!(
        !req.options
            .contains(&play_on(0, 0, a, Target::Card(elysium))),
        "not a hunting ground"
    );
    pick(&mut e, play_on(0, 0, a, Target::Card(park)));
    request(&mut e);
    assert_eq!(
        e.action().expect("action").directed_seat(&e.state),
        Some(SeatId(2))
    );
    let mut offered = false;
    loop {
        let req = request(&mut e);
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        offered |= offers_block(&req, cross);
        e.choose(fallback(&req));
    }
    assert!(offered);
    assert!(!e.state.cards[park.index()].in_play);
    assert!(e.state.cards[elysium.index()].in_play);
}

/// Poison the Well of Life [QUI]: every other Methuselah's hunting ground
/// burned, one's own kept; directed at each Methuselah controlling one, so a
/// seat neither prey nor predator may block, and one with none may not.
/// [LSJ 20080809-1] [RTR 20081119] Wave of Insanity's "all allies": directed
/// at all Methuselahs controlling one.
#[test]
fn poison_the_well_superior_burns_every_other_hunting_ground() {
    let mut e = game(4);
    let a = e.add_minion(SeatId(0), vampire_with("QUI", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let cross = e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    let none = e.add_minion(SeatId(3), plain("Ventrue", 2), 4, false);
    let own = in_play(&mut e, 0, "Park Hunting Ground", None);
    let one = in_play(&mut e, 1, "Zoo Hunting Ground", None);
    let two = in_play(&mut e, 2, "Uptown Hunting Ground", None);
    hand(&mut e, 0, &["Poison the Well of Life"]);
    hand(&mut e, 2, &["Spirit's Touch"]);
    pick(&mut e, play(0, 1, a));
    let mut offered = false;
    loop {
        let req = request(&mut e);
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        offered |= offers_block(&req, cross);
        assert!(
            !offers_block(&req, none),
            "no hunting ground: not directed at"
        );
        e.choose(fallback(&req));
    }
    assert!(offered, "the cross-table Methuselah may block");
    assert!(e.state.cards[own.index()].in_play);
    assert!(!e.state.cards[one.index()].in_play);
    assert!(!e.state.cards[two.index()].in_play);
}

// ---- Bleeds ----------------------------------------------------------------------

/// Truth of Blood blocked: the blocker's controller discards 2 at random
/// before block resolution (5 cards as the acting minion may still play
/// Change of Target), replaced once the action ends (7 again).
/// [ANK 20220116] [ANK 20211207] [ANK 20210627] [RTR 19991206] [LSJ
/// 20091125] "If blocked" effects applied first, before any other card.
/// [RTR 19991206] Still applied when Change of Target ends the action.
#[test]
fn truth_of_blood_blocked_costs_the_blocker_two_cards_until_the_action_ends() {
    for change in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), with_discs(&["aus", "qui"], 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        fill_hand(&mut e, 1);
        e.state.seats[1].library = vec![find_library("Form of Mist"); 6].into();
        hand(&mut e, 0, &["Truth of Blood", "Change of Target"]);
        pick(&mut e, play(0, 0, a));
        pick_when(&mut e, block(b));
        let req = expect(&mut e, SeatId(0), WindowLabel::ActionBlocked);
        assert_eq!(e.state.seats[1].hand.len(), 5, "before block resolution");
        assert_eq!(e.state.seats[1].ash.len(), 2);
        if change {
            e.choose(index_of(&req, play(0, 0, a)));
        } else {
            e.choose(index_of(&req, Choice::Pass));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[1].hand.len(), 7, "replaced after the action");
        assert!(e.combat().is_none());
    }
}

// ---- Ash heap vampires ----------------------------------------------------------

/// The new minion seat `s` controls of crypt card `card`, in play.
fn minion_of(e: &Engine, s: u8, card: u16) -> MinionId {
    let i = e
        .state
        .minions
        .iter()
        .position(|m| {
            m.in_play
                && m.owner == SeatId(s)
                && matches!(m.kind, zvtes::state::MinionKind::Vampire(c) if c == card)
        })
        .expect("in play");
    MinionId(i as u32)
}

/// Possession [nec]: the vampire named as the action is declared, moved from
/// the ash heap to the uncontrolled region; 2 blood paid.
#[test]
fn possession_moves_an_ash_vampire_to_the_uncontrolled_region() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("nec", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let (x, y) = (plain("Toreador", 0), plain("Toreador", 1));
    e.state.seats[0].ash_crypt = vec![x, y].into();
    hand(&mut e, 0, &["Possession"]);
    pick(&mut e, play(0, 0, a));
    let req = request(&mut e);
    assert!(req.options.contains(&Choice::AshVampire(x)), "{req:?}");
    e.choose(index_of(&req, Choice::AshVampire(y)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[0].uncontrolled.iter().any(|u| u.card == y));
    assert!(e.state.seats[0].ash_crypt.contains(&x));
    assert!(!e.state.seats[0].ash_crypt.contains(&y));
    assert_eq!(blood(&e, a), 2);
}

/// Possession [NEC]: a vampire burned with a lasting bonus on it comes back
/// to the ready region with 1 blood, nothing it had carried over, and may
/// act this turn.
/// [LSJ 20040726] Continuous effects applied before are lost in the ash
/// heap.
#[test]
fn possession_superior_readies_it_with_one_blood_and_nothing_carried() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("NEC", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let card = plain("Toreador", 0);
    let old = e.add_minion(SeatId(0), card, 4, false);
    e.state.minions[old.index()].timed.push((
        zvtes::cards::Lasting::Stealth(1),
        zvtes::state::Expiry::Never,
        SeatId(0),
    ));
    e.state.minions[old.index()].capacity_less = 1;
    e.burn(old);
    assert!(e.state.seats[0].ash_crypt.contains(&card));
    hand(&mut e, 0, &["Possession"]);
    pick(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let m = minion_of(&e, 0, card);
    assert_ne!(m, old, "a new minion");
    let v = &e.state.minions[m.index()];
    assert_eq!(v.blood, 1);
    assert!(v.timed.is_empty() && !v.locked && !v.in_torpor);
    assert_eq!(e.state.capacity_of(m), Some(CRYPT[card as usize].capacity));
    let req = request(&mut e);
    assert!(req.options.contains(&bleed(m)), "acts this turn");
}

/// Possession finds no vampire in an ash heap holding only a created
/// vampire's card (Childe of the Revolution, Call the Great Beast): not
/// offered.
/// [LSJ 20020115] A vampire-creating action card is not a vampire in the
/// ash heap: Possession cannot target it.
#[test]
fn possession_cannot_name_a_created_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("nec", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0].ash = vec![
        find_library("Childe of the Revolution"),
        find_library("Call the Great Beast"),
    ]
    .into();
    hand(&mut e, 0, &["Possession"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, a)));
    assert!(!req.options.contains(&play(0, 1, a)));
}

// ---- Set's Curse ------------------------------------------------------------------

/// The ally Set's Curse became, if any.
fn curse_ally(e: &Engine) -> Option<MinionId> {
    let card = find_library("Set's Curse");
    e.state
        .minions
        .iter()
        .position(|m| {
            m.in_play
                && matches!(m.kind, zvtes::state::MinionKind::Ally { card: c, .. } if c == card)
        })
        .map(|i| MinionId(i as u32))
}

/// A Set's Curse ally already in play for `seat`, as its mode puts it.
fn curse_ally_of(e: &mut Engine, seat: u8) -> MinionId {
    let card = find_library("Set's Curse");
    let m = MinionId(e.state.minions.len() as u32);
    let id = CardId(e.state.cards.len() as u32);
    e.state.cards.push(CardInPlay {
        body: Some(m),
        ..CardInPlay::new(card, SeatId(seat), None, 0)
    });
    let body = zvtes::library::Body {
        life: 3,
        strength: 2,
        bleed: 0,
    };
    e.state.minions.push(zvtes::state::Minion::new(
        SeatId(seat),
        zvtes::state::MinionKind::Ally { card, id, body },
        3,
    ));
    m
}

/// Set's Curse [ser] at the prey's ally: the ally burned, the card in play
/// as seat 0's animal ally (3 life, 2 strength, 0 bleed), able to act this
/// turn; it plays no action card and takes no equipment.
#[test]
fn sets_curse_burns_an_ally_and_becomes_an_animal_ally() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("ser", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let tasha = e.add_ally(SeatId(1), find_library("Tasha Morgan"));
    hand(&mut e, 0, &["Set's Curse", "Bum's Rush"]);
    pick(&mut e, play_on(0, 0, a, Target::Minion(tasha)));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[tasha.index()].in_play);
    let m = curse_ally(&e).expect("Set's Curse in play");
    assert!(!ash_has(&e, 0, "Set's Curse"));
    let v = &e.state.minions[m.index()];
    assert_eq!((v.owner, v.blood, v.locked), (SeatId(0), 3, false));
    let body = e.state.ally_body(m).expect("an ally");
    assert_eq!((body.strength, body.bleed), (2, 0));
    assert!(e.state.has_subtype(m, zvtes::library::Label::Animal));
    assert!(e.state.no_equipment_or_retainers(m));
    assert!(
        req.options.contains(&bleed(m)),
        "not recruited: acts this turn"
    );
    assert!(
        !req.options
            .iter()
            .any(|o| matches!(o, Choice::Play { actor: Some(x), .. } if *x == m)),
        "no action card"
    );
}

/// Set's Curse [ser], its retainer half: the prey's retainer burned.
#[test]
fn sets_curse_burns_a_retainer() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("ser", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let h = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let nar = in_play(&mut e, 1, "Nar-Sheptha", Some(h));
    e.state.cards[nar.index()].counters = 1;
    hand(&mut e, 0, &["Set's Curse"]);
    pick(&mut e, play_on(0, 1, a, Target::Card(nar)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[nar.index()].in_play);
    assert!(curse_ally(&e).is_some());
}

/// Set's Curse [SER]: a vampire of capacity 2 or less burned; a capacity-3
/// one is no target.
#[test]
fn sets_curse_superior_burns_a_small_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("SER", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let small_card = textless(|c| c.capacity == 2, 0);
    let small = e.add_minion(SeatId(1), small_card, 2, false);
    let three = e.add_minion(SeatId(1), textless(|c| c.capacity == 3, 0), 3, false);
    hand(&mut e, 0, &["Set's Curse"]);
    let req = request(&mut e);
    assert!(!req
        .options
        .contains(&play_on(0, 2, a, Target::Minion(three))));
    e.choose(index_of(&req, play_on(0, 2, a, Target::Minion(small))));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[small.index()].in_play);
    assert!(e.state.seats[1].ash_crypt.contains(&small_card));
    assert!(curse_ally(&e).is_some());
}

/// Set's Curse at Ilomba: its host burns a blood instead, Ilomba stays, and
/// Set's Curse is not put in play.
/// [ANK 20190422] A target that cannot be burned (Ilomba): not put into
/// play.
#[test]
fn sets_curse_on_ilomba_burns_nothing_and_stays_out() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("ser", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let h = e.add_minion(SeatId(1), plain("Ventrue", 0), 2, false);
    let ilomba = in_play(&mut e, 1, "Ilomba", Some(h));
    e.state.cards[ilomba.index()].counters = 1;
    hand(&mut e, 0, &["Set's Curse"]);
    pick(&mut e, play_on(0, 1, a, Target::Card(ilomba)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.cards[ilomba.index()].in_play, "Ilomba stays");
    assert_eq!(blood(&e, h), 1, "its host paid");
    assert!(curse_ally(&e).is_none());
    assert!(ash_has(&e, 0, "Set's Curse"));
}

/// Set's Curse at Saatet-ta: she is burned and shuffled into her owner's
/// library, and Set's Curse still enters play.
/// [ANK 20200813] A target shuffled into the library after it is burned:
/// both effects apply.
#[test]
fn sets_curse_on_a_shuffled_ally_still_enters_play() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("ser", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let saatet = e.add_ally(SeatId(1), find_library("Saatet-ta"));
    fill_hand(&mut e, 1);
    hand(&mut e, 0, &["Set's Curse"]);
    pick(&mut e, play_on(0, 0, a, Target::Minion(saatet)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[saatet.index()].in_play);
    assert!(e.state.seats[1]
        .library
        .contains(&find_library("Saatet-ta")));
    assert!(curse_ally(&e).is_some());
}

/// Revelation of Ecstasy [SER] at a Set's Curse ally: one counter steals
/// it (an ally costing 0).
/// [LSJ 20080804] Once in play, Set's Curse is an ally with a cost of 0.
#[test]
fn revelation_of_ecstasy_steals_a_sets_curse_ally_at_once() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("SER", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let curse = curse_ally_of(&mut e, 1);
    assert_eq!(e.state.ally_cost(curse), 0);
    hand(&mut e, 0, &["Revelation of Ecstasy"]);
    pick(&mut e, play_on(0, 1, a, Target::Minion(curse)));
    let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    e.choose(index_of(&req, Choice::ChooseMinion(curse)));
    request(&mut e);
    assert_eq!(e.state.minions[curse.index()].owner, SeatId(0));
}

/// Revelation of Ecstasy [SER] at seat 1's Spell of Life mummy made from a
/// capacity-3 vampire: "their capacity or cost" is its crypt card's
/// capacity (a crypt card's cost, rules-qa 71.9; 121.131): the third
/// counter steals it.
#[test]
fn revelation_of_ecstasy_steals_a_mummy_at_its_capacity() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Ministry", "SER", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: textless(|c| c.capacity == 3, 0),
            copy: Default::default(),
            counters: 0,
            banished: None,
        });
    let spells: Vec<CardId> = (0..5)
        .map(|_| in_play(&mut e, 0, "Spell of Life", None))
        .collect();
    pick(&mut e, spell_burn(spells[0]));
    pick(&mut e, Choice::Slot(0));
    let mummy = MinionId(
        e.state
            .minions
            .iter()
            .position(|v| matches!(v.kind, zvtes::state::MinionKind::Represented { .. }))
            .expect("a mummy") as u32,
    );
    e.state.minions[mummy.index()].owner = SeatId(1);
    assert_eq!(e.state.ally_cost(mummy), 3);
    e.state.minions[mummy.index()].add_corruption(SeatId(0), 2);
    hand(&mut e, 0, &["Revelation of Ecstasy"]);
    pick(&mut e, play_on(0, 1, a, Target::Minion(mummy)));
    let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert_eq!(req.options, vec![Choice::Pass, Choice::ChooseMinion(mummy)]);
    e.choose(index_of(&req, Choice::ChooseMinion(mummy)));
    request(&mut e);
    assert_eq!(e.state.minions[mummy.index()].owner, SeatId(0), "stolen");
}

// ---- Retain the Quick Blood ----------------------------------------------------------

/// Retain the Quick Blood: put on its vampire; the blood it spends on a
/// [cel] action card's cost goes onto the card; a [dom] card's to the bank.
#[test]
fn retain_the_quick_blood_stores_celerity_and_quietus_costs() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 6, 0), 6, false);
    e.state.minions[a.index()].gained_levels = vec!["cel", "qui", "dom"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(
        &mut e,
        0,
        &["Retain the Quick Blood", "Fleetness", "Puppet Master"],
    );
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let card = placed_on(&e, "Retain the Quick Blood", Some(a));
    e.state.minions[a.index()].locked = false;
    pick(&mut e, Choice::Pass);
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.choose(index_of(&req, play(0, 0, a)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 5, "Fleetness' blood");
    assert_eq!(e.state.cards[card.index()].counters, 1, "on the card");
    pick(&mut e, Choice::Pass);
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let b = e.state.cards[card.index()].counters;
    e.choose(index_of(&req, play(0, 0, a)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.cards[card.index()].counters,
        b,
        "a [dom] card's to the bank"
    );
}

/// Retain the Quick Blood in its controller's unlock phase: [cel][qui]
/// moves 1 blood to its vampire, once; [CEL][QUI] up to 2.
#[test]
fn retain_the_quick_blood_returns_blood_in_the_unlock_phase() {
    for (mode, moved) in [(0u8, 1u8), (1, 2)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 6, 0), 1, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let card = in_play_mode(&mut e, 0, "Retain the Quick Blood", Some(a), mode);
        e.state.cards[card.index()].counters = 3;
        pick(&mut e, Choice::Pass);
        let mut uses = 0;
        loop {
            let req = request(&mut e);
            if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
                break;
            }
            if let Some(&u) = uses_of(&req, card).first() {
                uses += 1;
                pick(&mut e, u);
                continue;
            }
            e.choose(fallback(&req));
        }
        assert_eq!(uses, 1, "once each unlock phase");
        assert_eq!(blood(&e, a), 1 + moved);
        assert_eq!(e.state.cards[card.index()].counters, 3 - moved);
    }
}

/// Two Retain the Quick Blood on one vampire: its controller picks the one
/// keeping each payment (rules-qa 121.127): a Fleetness' blood on one, the
/// next turn's on the other; both return it in the next unlock phase.
#[test]
fn retain_the_quick_blood_two_copies_take_the_payments_chosen() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 6, 0), 6, false);
    e.state.minions[a.index()].gained_levels = vec!["cel"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let first = in_play(&mut e, 0, "Retain the Quick Blood", Some(a));
    let second = in_play(&mut e, 0, "Retain the Quick Blood", Some(a));
    for keeper in [first, second] {
        hand(&mut e, 0, &["Fleetness"]);
        pick(&mut e, play(0, 0, a));
        let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
        assert_eq!(
            req.options,
            vec![Choice::ChooseCard(first.0), Choice::ChooseCard(second.0)]
        );
        e.choose(index_of(&req, Choice::ChooseCard(keeper.0)));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.cards[keeper.index()].counters, 1);
        pick(&mut e, Choice::Pass);
        if keeper == first {
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        }
    }
    assert_eq!(blood(&e, a), 6 - 2);
    let mut used = Vec::new();
    loop {
        let req = request(&mut e);
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        if let Some((c, u)) = [first, second]
            .into_iter()
            .find_map(|c| uses_of(&req, c).first().map(|&u| (c, u)))
        {
            used.push(c);
            pick(&mut e, u);
            continue;
        }
        e.choose(fallback(&req));
    }
    used.sort_by_key(|c| c.0);
    assert_eq!(used, vec![first, second], "each returns its blood");
    assert_eq!(blood(&e, a), 6);
}

/// The abilities offered of the card in play `card`.
fn uses_of(req: &zvtes::decision::DecisionRequest, card: CardId) -> Vec<Choice> {
    uses(req, card.0)
}

// ---- Precision ----------------------------------------------------------------------

/// Precision [cel]: the name chosen as the action is declared, among every
/// library card name; the prey discards one copy of two, replaced.
#[test]
fn precision_makes_the_prey_discard_the_named_card() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("cel", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 1, &["Conceal", "Conceal", "Deflection"]);
    fill_hand(&mut e, 1);
    e.state.seats[1].library = vec![find_library("Form of Mist"); 3].into();
    hand(&mut e, 0, &["Precision"]);
    pick(&mut e, play_on(0, 0, a, Target::Seat(SeatId(1))));
    let req = request(&mut e);
    let names: std::collections::HashSet<&str> =
        zvtes::library::LIBRARY.iter().map(|c| c.name).collect();
    assert_eq!(req.options.len(), names.len(), "every library card name");
    e.choose(index_of(&req, Choice::FindCard(find_library("Conceal"))));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let s1 = &e.state.seats[1];
    assert_eq!(
        s1.hand
            .iter()
            .filter(|&&c| c == find_library("Conceal"))
            .count(),
        1
    );
    assert!(s1.ash.contains(&find_library("Conceal")));
    assert_eq!(s1.hand.len(), 7, "replaced");
    assert_eq!(s1.library.len(), 2);
}

/// Precision [CEL]: after a discard, the actor may burn 1 blood to burn 1
/// of the prey's pool; with no copy discarded, nothing is offered.
#[test]
fn precision_superior_burns_a_pool_for_a_blood_after_a_discard() {
    for held in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("CEL", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 1, &[if held { "Conceal" } else { "Deflection" }]);
        hand(&mut e, 0, &["Precision"]);
        pick(&mut e, play_on(0, 1, a, Target::Seat(SeatId(1))));
        pick(&mut e, Choice::FindCard(find_library("Conceal")));
        let offered = offered_before(
            &mut e,
            Choice::ResolvePrinted { minion: a },
            SeatId(0),
            WindowLabel::MinionPhase,
        );
        assert_eq!(offered, held);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        if held {
            assert_eq!((blood(&e, a), e.state.seats[1].pool), (3, 9));
        } else {
            assert_eq!((blood(&e, a), e.state.seats[1].pool), (4, 10));
        }
    }
}

// ---- Shell Break --------------------------------------------------------------------

/// The `nth` textless Giovanni with Necromancy, capacity 4 or more.
fn giovanni(nth: usize) -> u16 {
    textless(
        |c| {
            c.clan == "Giovanni"
                && c.capacity >= 4
                && c.disciplines.iter().any(|d| d.eq_ignore_ascii_case("nec"))
        },
        nth,
    )
}

/// Shell Break on the prey's unique vampire; burned: the card moves
/// to its Giovanni, seat 0's wraith retainer with 2 life, its employer +1
/// bleed and +1 intercept (never the first bearer's); a wraith retainer
/// for every card (Soul Feasting may eat it).
#[test]
fn shell_break_moves_to_its_giovanni_as_a_wraith_retainer() {
    use zvtes::cards::{CardFilter, Stat};
    use zvtes::library::{CardType, Label};
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), giovanni(0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let tasha = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Shell Break", "Soul Feasting"]);
    pick(&mut e, play_on(0, 0, g, Target::Minion(tasha)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let shell = placed_on(&e, "Shell Break", Some(tasha));
    assert_eq!(
        e.state.bearer_mod(tasha, Stat::Bleed, ActionKind::Bleed),
        0,
        "not yet"
    );
    assert!(!e
        .state
        .card_in_play_matches(shell, CardFilter::Type(CardType::Retainer)));
    e.burn(tasha);
    assert!(!e.state.minions[tasha.index()].in_play);
    let c = &e.state.cards[shell.index()];
    assert!(c.in_play && c.host == Some(g));
    assert_eq!((e.state.card_controller(shell), c.counters), (SeatId(0), 2));
    assert!(e.state.card_in_play_matches(
        shell,
        CardFilter::Labeled(CardType::Retainer, Label::Wraith)
    ));
    assert_eq!(e.state.bearer_mod(g, Stat::Bleed, ActionKind::Bleed), 1);
    assert_eq!(e.state.bearer_mod(g, Stat::Intercept, ActionKind::Bleed), 1);
    assert!(!ash_has(&e, 1, "Shell Break") && !ash_has(&e, 0, "Shell Break"));
    e.state.minions[g.index()].locked = false;
    let other = e.add_minion(SeatId(0), plain("Brujah", 2), 4, false);
    pick(&mut e, action(other, ActionKind::Hunt));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        req.options.contains(&play_on(0, 2, g, Target::Card(shell))),
        "a wraith retainer"
    );
}

/// A Shell Break retainer: a strike aimed at it (Target Retainer) deals no
/// non-aggravated damage; an aggravated one takes its life.
/// [LSJ 20010627] [LSJ 20010629] [LSJ 20090304-1] Immune: the damage is not
/// inflicted, even unpreventable or out of combat.
#[test]
fn shell_break_retainer_is_immune_to_non_aggravated_damage() {
    for aggravated in [false, true] {
        let mut e = game(2);
        let g = e.add_minion(SeatId(0), giovanni(0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let a = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let victim = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let shell = shell_retainer(&mut e, g, victim);
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        e.state.minions[g.index()].locked = false;
        if aggravated {
            e.state.minions[a.index()].timed.push((
                zvtes::cards::Lasting::AggravatedHands,
                zvtes::state::Expiry::EndOfTurn,
                SeatId(1),
            ));
        }
        hand(&mut e, 1, &["Target Retainer"]);
        let before = blood(&e, g);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(g));
        pick_when(&mut e, play_on(0, 0, a, Target::Card(shell)));
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(
            e.state.cards[shell.index()].counters,
            if aggravated { 1 } else { 2 }
        );
        assert_eq!(blood(&e, g), before, "the strike went to the retainer");
    }
}

/// Giovanni `g` plays Shell Break on `victim`, which then burns: the card a
/// wraith retainer on `g` (`shell_break_moves_to_its_giovanni_as_a_wraith_retainer`).
/// From `g`'s controller's minion phase back to it.
fn shell_retainer(e: &mut Engine, g: MinionId, victim: MinionId) -> CardId {
    let seat = e.state.minions[g.index()].owner;
    hand(e, seat.index() as u8, &["Shell Break"]);
    pick(e, play_on(0, 0, g, Target::Minion(victim)));
    until(e, seat, WindowLabel::MinionPhase);
    let shell = placed_on(e, "Shell Break", Some(victim));
    e.burn(victim);
    assert_eq!(e.state.cards[shell.index()].host, Some(g));
    shell
}

/// A Shell Break retainer is "a retainer" for every reader, not only a
/// strike aimed at one: Can't Take it with You ("burns 1 pool for each
/// equipment, location or retainer card he or she controls") counts it.
#[test]
fn shell_break_retainer_counts_for_cant_take_it_with_you() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), giovanni(0), 4, false);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let victim = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    shell_retainer(&mut e, g, victim);
    let before = pool(&e);
    hand(&mut e, 0, &["Can't Take it with You"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    close_polling(&mut e);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e), vec![before[0], before[1] + 1]);
}

/// Shell Break's target: "a ready unique non-wraith non-zombie minion who
/// does not already have a Shell Break": not a non-unique ally, a wraith
/// (Tye Cooper), a minion holding one, a torpid vampire.
#[test]
fn shell_break_needs_a_unique_non_wraith_minion_without_one() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), giovanni(0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let held = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let torpid = e.add_minion(SeatId(1), plain("Ventrue", 2), 4, true);
    in_play(&mut e, 1, "Shell Break", Some(held));
    let tye = e.add_ally(SeatId(1), find_library("Tye Cooper"));
    let hunter_card = find_library("City Star Taxi");
    assert!(!zvtes::library::LIBRARY[hunter_card as usize]
        .labels
        .contains(&zvtes::library::Label::Unique));
    let hunter = e.add_ally(SeatId(1), hunter_card);
    hand(&mut e, 0, &["Shell Break"]);
    let req = request(&mut e);
    let at = |m| play_on(0, 0, g, Target::Minion(m));
    assert!(req.options.contains(&at(v)));
    for m in [held, torpid, tye, hunter] {
        assert!(!req.options.contains(&at(m)), "{m:?}");
    }
}

/// Shell Break's Giovanni gone first: the bearer burned, the card burns
/// with it.
#[test]
fn shell_break_burns_with_its_bearer_once_its_giovanni_is_gone() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), giovanni(0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Shell Break"]);
    pick(&mut e, play_on(0, 0, g, Target::Minion(v)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let shell = placed_on(&e, "Shell Break", Some(v));
    e.burn(g);
    e.burn(v);
    assert!(!e.state.cards[shell.index()].in_play);
    assert!(ash_has(&e, 0, "Shell Break"));
}

// ---- Khazar's Diary ---------------------------------------------------------------------

/// A Khazar's Diary in play for `seat` with `n` counters.
fn diary(e: &mut Engine, seat: u8, n: u8) -> CardId {
    let d = in_play(e, seat, "Khazar's Diary (Endless Night)", None);
    e.state.cards[d.index()].counters = n;
    d
}

/// The represented minion of `card` (CRYPT if `crypt`) in play.
fn represented(e: &Engine, card: u16, crypt: bool) -> Option<MinionId> {
    e.state
        .minions
        .iter()
        .position(|m| {
            m.in_play
                && matches!(m.kind, zvtes::state::MinionKind::Represented { card: c, crypt: k, .. }
                    if c == card && k == crypt)
        })
        .map(|i| MinionId(i as u32))
}

/// Khazar's Diary: a unique minion burned, anyone's, adds a counter; a
/// non-unique one none. At 7, every Methuselah's Giovanni has superior
/// Necromancy, no other clan.
#[test]
fn khazars_diary_counts_unique_burns_and_grants_necromancy_at_seven() {
    let mut e = game(3);
    let g = e.add_minion(SeatId(0), giovanni(0), 4, false);
    let other = e.add_minion(
        SeatId(0),
        textless(
            |c| c.clan != "Giovanni" && c.disciplines.contains(&"nec"),
            0,
        ),
        4,
        false,
    );
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let u = e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 2), 4, false);
    let taxi = e.add_ally(SeatId(2), find_library("City Star Taxi"));
    let d = diary(&mut e, 1, 6);
    assert_eq!(e.state.discipline_level(g, "nec"), 1);
    e.burn(taxi);
    assert_eq!(e.state.cards[d.index()].counters, 6, "not unique");
    e.burn(u);
    assert_eq!(e.state.cards[d.index()].counters, 7);
    assert_eq!(e.state.discipline_level(g, "nec"), 2);
    assert_eq!(e.state.discipline_level(other, "nec"), 1);
}

/// Khazar's Diary at 7: seat 0's Giovanni raises the vampire card of seat
/// 2's ash heap, an undirected action; the wraith is seat 0's ally, 2 life,
/// a wraith, +1 stealth, unlocked and acting this turn.
/// [LSJ 20080426] The wraith can act the turn it is put in play. [LSJ
/// 20010627] [ANK 20180202] Undirected, even at another's ash heap.
#[test]
fn khazars_diary_raises_a_wraith_that_acts_at_once() {
    let mut e = game(3);
    let g = e.add_minion(SeatId(0), giovanni(0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    let x = plain("Toreador", 0);
    e.state.seats[2].ash_crypt = vec![x].into();
    let d = diary(&mut e, 0, 7);
    pick(&mut e, granted(d.0, g));
    request(&mut e);
    assert_eq!(e.action().expect("action").directed_seat(&e.state), None);
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let w = represented(&e, x, true).expect("the wraith");
    assert!(!e.state.seats[2].ash_crypt.contains(&x));
    let v = &e.state.minions[w.index()];
    assert_eq!((v.owner, v.blood, v.locked), (SeatId(0), 2, false));
    assert!(v.is_ally());
    assert!(e.state.has_subtype(w, zvtes::library::Label::Wraith));
    assert_eq!(
        e.state.ally_body(w).map(|b| (b.strength, b.bleed)),
        Some((0, 1))
    );
    assert_eq!(e.state.lasting(w).0, 1, "+1 stealth");
    assert!(req.options.contains(&bleed(w)), "acts this turn");
}

/// The Diary's wraith burned: its card back to its owner's ash heap, not
/// burned from play (Pressing Flesh cannot take it as a burned ally).
/// [PIB 20130128-2] [LSJ 20100325-2] Back to the owner's ash heap, not an
/// ally burned from play. [LSJ 20071014] Representing something else, it
/// is no ally burned from play for Compel the Spirit either.
#[test]
fn khazars_diary_wraith_returns_to_its_owners_ash_heap_not_burned_from_play() {
    let mut e = game(3);
    let g = e.add_minion(SeatId(0), giovanni(0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    let tasha = find_library("City Star Taxi");
    e.state.seats[2].ash = vec![tasha].into();
    let d = diary(&mut e, 0, 7);
    pick(&mut e, granted(d.0, g));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let w = represented(&e, tasha, false).expect("the wraith");
    assert_eq!(e.state.minions[w.index()].name(), "", "nameless");
    e.burn(w);
    assert!(e.state.seats[2].ash.contains(&tasha));
    assert!(e.state.seats[0].ash.iter().all(|&c| c != tasha));
    assert!(e.state.seats[2].burned_allies.0.is_empty());
    assert!(!zvtes::cards::holds(
        zvtes::cards::Requirement::BurnedAllyInAnyAsh,
        &e.state,
        SeatId(0),
        None,
        None,
        None,
        None,
        None,
        false,
    ));
}

/// The Diary's action is not offered with no minion card in any ash heap.
/// [ANK 20200420-3] Cannot be played with no target in any ash heap.
#[test]
fn khazars_diary_action_needs_a_minion_in_some_ash_heap() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), giovanni(0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[1].ash = vec![find_library("Conceal")].into();
    let d = diary(&mut e, 0, 7);
    let req = request(&mut e);
    assert!(!req.options.contains(&granted(d.0, g)));
}

/// The Diary's wraith is nameless: raised from a Tye Cooper card while
/// another Tye Cooper is in play, neither is contested (red reading).
#[test]
fn khazars_diary_wraith_contests_nothing() {
    let mut e = game(3);
    let g = e.add_minion(SeatId(0), giovanni(0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let tye = e.add_ally(SeatId(1), find_library("Tye Cooper"));
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    let card = find_library("Tye Cooper");
    e.state.seats[2].ash = vec![card].into();
    let d = diary(&mut e, 0, 7);
    pick(&mut e, granted(d.0, g));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let w = represented(&e, card, false).expect("the wraith");
    assert!(!e.state.minions[w.index()].contested);
    assert!(!e.state.minions[tye.index()].contested);
}

/// Khazar's Diary's second half at another Methuselah's Diary: a counter
/// added, the action directed at its controller.
#[test]
fn khazars_diary_adds_a_counter_to_anothers_diary_directed() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), giovanni(0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let d = diary(&mut e, 1, 2);
    hand(&mut e, 0, &["Khazar's Diary (Endless Night)"]);
    pick(&mut e, play_on(0, 1, g, Target::Card(d)));
    request(&mut e);
    assert_eq!(
        e.action().expect("action").directed_seat(&e.state),
        Some(SeatId(1))
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[d.index()].counters, 3);
}

// ---- Spell of Life --------------------------------------------------------------------

/// The Spell of Life copies' ability (ability 0) of `card`.
fn spell_burn(card: CardId) -> Choice {
    use_on(card, 0, None, Target::None)
}

/// Spell of Life: "Only one Spell of Life can be played each turn."
#[test]
fn spell_of_life_is_played_once_each_turn() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Ministry", "", 0), 4, false);
    let b = e.add_minion(SeatId(0), of_clan("Ministry", "", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Spell of Life", "Spell of Life"]);
    pick(&mut e, play(0, 0, a));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(req.options.contains(&bleed(b)));
    assert!(!req.options.contains(&play(0, 0, b)), "once each turn");
    placed_on(&e, "Spell of Life", None);
}

/// Spell of Life: five copies in play, three seat 0's and two seat 1's,
/// burned by seat 0; two of seat 0's uncontrolled vampires (capacity 4 and
/// 7) in play as seat 0's mummies: allies with life equal to capacity, 2
/// strength, 1 bleed, able to enter combat with seat 1's minion this turn.
/// [LSJ 20090428] The copies burned need not be one's own.
#[test]
fn spell_of_life_burns_five_copies_anyones_into_mummies() {
    let mut e = game(2);
    e.add_minion(SeatId(0), of_clan("Ministry", "", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let foe = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let (four, seven) = (
        textless(|c| c.capacity == 4, 3),
        textless(|c| c.capacity == 7, 3),
    );
    for c in [four, seven] {
        e.state.seats[0]
            .uncontrolled
            .push(zvtes::state::UncontrolledSlot {
                card: c,
                copy: Default::default(),
                counters: 2,
                banished: None,
            });
    }
    let mine: Vec<CardId> = (0..3)
        .map(|_| in_play(&mut e, 0, "Spell of Life", None))
        .collect();
    for _ in 0..2 {
        in_play(&mut e, 1, "Spell of Life", None);
    }
    pick(&mut e, spell_burn(mine[0]));
    for _ in 0..2 {
        let req = request(&mut e);
        assert_eq!(req.options[0], Choice::Pass);
        e.choose(1);
    }
    let req = request(&mut e);
    let spell = find_library("Spell of Life");
    assert!(e.state.cards.iter().all(|c| c.card != spell || !c.in_play));
    assert_eq!(
        e.state.seats[0].ash.iter().filter(|&&c| c == spell).count(),
        3
    );
    assert_eq!(
        e.state.seats[1].ash.iter().filter(|&&c| c == spell).count(),
        2
    );
    for (c, life) in [(four, 4), (seven, 7)] {
        let m = represented(&e, c, true).expect("a mummy");
        let v = &e.state.minions[m.index()];
        assert_eq!((v.owner, v.blood, v.locked), (SeatId(0), life, false));
        assert!(e.state.has_subtype(m, zvtes::library::Label::Mummy));
        assert_eq!(
            e.state.ally_body(m).map(|b| (b.strength, b.bleed)),
            Some((2, 1))
        );
        assert!(
            req.options.iter().any(|o| matches!(o,
                Choice::UseAbility { source: AbilitySource::Vampire(x), target: Target::Minion(t), .. }
                    if *x == m && *t == foe)),
            "its rush: {req:?}"
        );
    }
}

/// Spell of Life, six copies in play: its user picks the five burned, any
/// Methuselah's, here keeping the copy used; then the mummies.
/// [LSJ 20090428] The copies you burn need not be ones you control.
#[test]
fn spell_of_life_chooses_five_copies_of_more() {
    let mut e = game(2);
    e.add_minion(SeatId(0), of_clan("Ministry", "", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: textless(|c| c.capacity == 4, 3),
            copy: Default::default(),
            counters: 0,
            banished: None,
        });
    let mine: Vec<CardId> = (0..3)
        .map(|_| in_play(&mut e, 0, "Spell of Life", None))
        .collect();
    let theirs: Vec<CardId> = (0..3)
        .map(|_| in_play(&mut e, 1, "Spell of Life", None))
        .collect();
    pick(&mut e, spell_burn(mine[0]));
    let req = expect(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    let every: Vec<Choice> = mine
        .iter()
        .chain(&theirs)
        .map(|c| Choice::ChooseCard(c.0))
        .collect();
    assert_eq!(req.options, every);
    for c in theirs.iter().chain(&mine[1..]) {
        pick(&mut e, Choice::ChooseCard(c.0));
    }
    let req = expect(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert_eq!(req.options[0], Choice::Pass, "the mummies next");
    assert!(e.state.cards[mine[0].index()].in_play, "the copy used kept");
    for c in theirs.iter().chain(&mine[1..]) {
        assert!(!e.state.cards[c.index()].in_play);
    }
    let spell = find_library("Spell of Life");
    assert_eq!(
        e.state.seats[1].ash.iter().filter(|&&c| c == spell).count(),
        3
    );
}

/// Spell of Life's mummies are unique by their card's name: one of X
/// contests seat 1's X; one of a non-unique vampire would not contest it.
/// [LSJ 20070928-2] [LSJ 20071001-2] Unique by card name, the name in play,
/// contesting unique minions of that name (the Nefertiti half reads the
/// name in play, `Minion::name`). [LSJ 20070928-2] A non-unique vampire is
/// not contested.
#[test]
fn spell_of_life_mummies_are_unique_by_name() {
    let mut e = game(2);
    e.add_minion(SeatId(0), of_clan("Ministry", "", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let x = plain("Ventrue", 0);
    let theirs = e.add_minion(SeatId(1), x, 4, false);
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: x,
            copy: Default::default(),
            counters: 0,
            banished: None,
        });
    let mine: Vec<CardId> = (0..5)
        .map(|_| in_play(&mut e, 0, "Spell of Life", None))
        .collect();
    pick(&mut e, spell_burn(mine[0]));
    pick(&mut e, Choice::Slot(0));
    let m = MinionId(
        e.state
            .minions
            .iter()
            .position(|v| matches!(v.kind, zvtes::state::MinionKind::Represented { card, .. } if card == x))
            .expect("a mummy") as u32,
    );
    assert_eq!(e.state.minions[m.index()].name(), CRYPT[x as usize].name);
    assert!(
        e.state.minions[m.index()].contested && e.state.minions[theirs.index()].contested,
        "both contested"
    );
    if let Some(nu) = CRYPT.iter().position(|c| !c.unique) {
        let nu = nu as u16;
        let mut e = game(2);
        e.add_minion(SeatId(0), of_clan("Ministry", "", 0), 4, false);
        let them = e.add_minion(SeatId(1), nu, 1, false);
        e.state.seats[0]
            .uncontrolled
            .push(zvtes::state::UncontrolledSlot {
                card: nu,
                copy: Default::default(),
                counters: 0,
                banished: None,
            });
        let mine: Vec<CardId> = (0..5)
            .map(|_| in_play(&mut e, 0, "Spell of Life", None))
            .collect();
        pick(&mut e, spell_burn(mine[0]));
        pick(&mut e, Choice::Slot(0));
        let m = represented(&e, nu, true).expect("a mummy");
        assert!(!e.state.minions[m.index()].contested);
        assert!(!e.state.minions[them.index()].contested);
    }
}

/// Spell of Life's mummy under Wormwood at 3: its life is its printed
/// capacity, 7.
/// [LSJ 20100716] The mummies are not affected by Wormwood.
#[test]
fn spell_of_life_mummies_ignore_wormwood() {
    let mut e = game(2);
    e.add_minion(SeatId(0), of_clan("Ministry", "", 0), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 3, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 3, false);
    let w = in_play(&mut e, 1, "Wormwood", None);
    e.state.cards[w.index()].counters = 3;
    let seven = textless(|c| c.capacity == 7, 3);
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: seven,
            copy: Default::default(),
            counters: 0,
            banished: None,
        });
    let mine: Vec<CardId> = (0..5)
        .map(|_| in_play(&mut e, 0, "Spell of Life", None))
        .collect();
    pick(&mut e, spell_burn(mine[0]));
    pick(&mut e, Choice::Slot(0));
    let m = represented(&e, seven, true).expect("a mummy");
    assert_eq!(blood(&e, m), 7);
}

/// A mummy burned: its crypt card in its owner's ash heap as a vampire,
/// which Possession names.
/// [LSJ 20071014] In the ash heap it counts as a vampire: Possession can be
/// used.
#[test]
fn possession_raises_a_burned_mummy() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("nec", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let x = textless(|c| c.capacity == 4, 3);
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: x,
            copy: Default::default(),
            counters: 0,
            banished: None,
        });
    let mine: Vec<CardId> = (0..5)
        .map(|_| in_play(&mut e, 0, "Spell of Life", None))
        .collect();
    pick(&mut e, spell_burn(mine[0]));
    pick(&mut e, Choice::Slot(0));
    let m = represented(&e, x, true).expect("a mummy");
    e.burn(m);
    assert!(e.state.seats[0].ash_crypt.contains(&x));
    assert!(
        e.state.seats[0].burned_from_play.0.is_empty(),
        "not burned from play"
    );
    hand(&mut e, 0, &["Possession"]);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[0].uncontrolled.iter().any(|u| u.card == x));
}

// ---- action_a: cards on a vampire ------------------------------------------

/// Blithe Acceptance [obf]: on the acting vampire, itself; a second is not
/// played by a vampire with one (rules-qa 99.1).
#[test]
fn blithe_acceptance_goes_on_its_vampire_once() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obf", 0), 2, false);
    e.add_minion(SeatId(0), vampire_with("obf", 1), 2, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Blithe Acceptance"]);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "Blithe Acceptance", Some(a));
    assert_eq!(blood(&e, a), 1, "1 blood");
    // A vampire bearing one is not offered another.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obf", 0), 4, false);
    let o = e.add_minion(SeatId(0), vampire_with("obf", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 0, "Blithe Acceptance", Some(a));
    hand(&mut e, 0, &["Blithe Acceptance"]);
    let req = request(&mut e);
    assert!(!plays(&req).contains(&play(0, 0, a)));
    assert!(plays(&req).contains(&play(0, 0, o)));
}

/// Blithe Acceptance [obf]: its vampire blocks, enters combat: the card
/// burns before the first round.
#[test]
fn blithe_acceptance_burns_as_its_vampire_enters_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let card = in_play(&mut e, 1, "Blithe Acceptance", Some(b));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    request(&mut e);
    assert!(!e.state.cards[card.index()].present());
    assert!(ash_has(&e, 1, "Blithe Acceptance"));
}

/// Blithe Acceptance [OBF]: each combat its vampire enters, 1 blood or the
/// card, the controller's choice; a second combat asks again.
/// [ANK 20230527] If a new combat begins, you need to burn 1 blood again
/// (or the card).
#[test]
fn blithe_acceptance_superior_pays_a_blood_each_new_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let a2 = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let card = in_play_mode(&mut e, 1, "Blithe Acceptance", Some(b), 1);
    // The first combat: 1 blood paid, the card stays.
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    let req = expect(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
    assert!(req.options.contains(&Choice::BurnBloodToAvoid));
    pick(&mut e, Choice::BurnBloodToAvoid);
    assert_eq!(blood(&e, b), 3);
    assert!(e.state.cards[card.index()].present());
    // A second combat: asked again; declining burns the card.
    e.state.minions[b.index()].locked = false;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, bleed(a2));
    pick_when(&mut e, block(b));
    let req = expect(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
    let before = blood(&e, b);
    e.choose(index_of(&req, Choice::Pass));
    assert_eq!(blood(&e, b), before);
    assert!(!e.state.cards[card.index()].present());
    assert!(ash_has(&e, 1, "Blithe Acceptance"));
}

/// Blithe Acceptance [OBF]: Psyche! starts a new combat of the same action;
/// its vampire is asked again.
/// [ANK 20230527] If a new combat begins (eg. Psyche!), you need to burn 1
/// blood again (or the card).
#[test]
fn blithe_acceptance_superior_is_asked_again_after_psyche() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("CEL", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let card = in_play_mode(&mut e, 1, "Blithe Acceptance", Some(b), 1);
    hand(&mut e, 0, &["Psyche!"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    let req = expect(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
    e.choose(index_of(&req, Choice::BurnBloodToAvoid));
    let paid = blood(&e, b);
    pick_when(&mut e, play(0, 1, a));
    let req = expect(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
    assert!(req.options.contains(&Choice::BurnBloodToAvoid));
    e.choose(index_of(&req, Choice::BurnBloodToAvoid));
    assert!(blood(&e, b) < paid);
    assert!(e.state.cards[card.index()].present());
    assert!(e.state.seats[0].ash.contains(&find_library("Psyche!")));
}

/// Chill of Oblivion [nec]: put on a ready vampire (another Methuselah's, a
/// directed action), not on one with one already; its bearer burns it in
/// combat and takes the aggravated damage as normal for the combat.
#[test]
fn chill_of_oblivion_turns_aggravated_damage_normal_for_the_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("nec", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Chill of Oblivion"]);
    pick(&mut e, play_on(0, 0, a, Target::Minion(b)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let chill = placed_on(&e, "Chill of Oblivion", Some(b));
    // A second one is not put on it.
    let mut e2 = game(2);
    let a2 = e2.add_minion(SeatId(0), vampire_with("nec", 0), 4, false);
    e2.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b2 = e2.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let c2 = e2.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    in_play(&mut e2, 1, "Chill of Oblivion", Some(b2));
    hand(&mut e2, 0, &["Chill of Oblivion"]);
    let req = request(&mut e2);
    assert!(!plays(&req).contains(&play_on(0, 0, a2, Target::Minion(b2))));
    assert!(plays(&req).contains(&play_on(0, 0, a2, Target::Minion(c2))));
    // The bearer in combat, struck for 2 aggravated: burns the card first.
    let x = e.add_minion(SeatId(0), plain("Brujah", 2), 4, false);
    e.state.minions[x.index()].timed.push((
        zvtes::cards::Lasting::AggravatedHands,
        zvtes::state::Expiry::EndOfTurn,
        SeatId(0),
    ));
    e.state.minions[x.index()].timed.push((
        zvtes::cards::Lasting::Strength(1),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(0),
    ));
    pick(&mut e, bleed(x));
    pick_when(&mut e, block(b));
    pick_when(&mut e, use_on(chill, 0, None, Target::None));
    assert!(ash_has(&e, 0, "Chill of Oblivion"), "its owner's ash heap");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, b), 2, "2 normal damage, burned as blood");
    assert!(!e.state.minions[b.index()].in_torpor);
}

/// Chill of Oblivion [NEC]: the acting vampire, locked by the action,
/// unlocks as its controller's discard phase begins, this turn.
#[test]
fn chill_of_oblivion_superior_unlocks_the_actor_in_the_discard_phase() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("NEC", 0), 4, false);
    let o = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Chill of Oblivion"]);
    pick(&mut e, play_on(0, 1, a, Target::Minion(o)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "Chill of Oblivion", Some(o));
    assert!(e.state.minions[a.index()].locked);
    let req = request(&mut e);
    e.choose(index_of(&req, Choice::Pass));
    loop {
        let req = request(&mut e);
        if e.state.active != SeatId(0) || req.window == WindowLabel::DiscardPhase {
            break;
        }
        e.choose(fallback(&req));
    }
    assert!(
        !e.state.minions[a.index()].locked,
        "unlocked in the discard phase"
    );
    assert!(!e.state.minions[o.index()].locked);
}

/// Ex Nihilo [nec]: on the acting vampire, itself, from the action card.
#[test]
fn ex_nihilo_goes_on_the_acting_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("nec", 0), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Ex Nihilo"]);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "Ex Nihilo", Some(a));
    assert_eq!(blood(&e, a), 1);
}

/// Ex Nihilo: its vampire blocks; neither the opposing hand strike lands on
/// it nor does its own on the opponent; its hunt gains it nothing.
/// [LSJ 20010627] [LSJ 20010629] [LSJ 20090304-1] Immune: damage is not
/// inflicted, even unpreventable or outside combat. [ANK 20221019-2] Any
/// blood it would gain goes to the bank.
#[test]
fn ex_nihilo_vampire_takes_and_inflicts_no_normal_damage() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 3, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    in_play(&mut e, 1, "Ex Nihilo", Some(x));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 3, "immune");
    assert_eq!(blood(&e, a), 4, "inflicts no damage in combat");
    // Its hunt: the blood to the bank.
    let mut e = game(2);
    let x = e.add_minion(SeatId(0), plain("Ventrue", 0), 3, false);
    e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    in_play(&mut e, 0, "Ex Nihilo", Some(x));
    pick(&mut e, action(x, ActionKind::Hunt));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 3, "cannot gain blood");
}

/// Ex Nihilo: with aggravated damage treated as normal (Chill of
/// Oblivion), immune to it too.
/// [LSJ 20040120] If the owner treats aggravated damage as normal, he is
/// immune to them too.
#[test]
fn ex_nihilo_vampire_is_immune_to_aggravated_damage_it_treats_as_normal() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 3, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    in_play(&mut e, 1, "Ex Nihilo", Some(x));
    let chill = in_play(&mut e, 1, "Chill of Oblivion", Some(x));
    e.state.minions[a.index()].timed.push((
        zvtes::cards::Lasting::AggravatedHands,
        zvtes::state::Expiry::EndOfTurn,
        SeatId(0),
    ));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    pick_when(&mut e, use_on(chill, 0, None, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 3);
    assert!(!e.state.minions[x.index()].in_torpor);
}

/// Ex Nihilo: each master phase of its controller's, 1 blood or the
/// vampire, the controller's choice even with blood.
/// [ANK 20220503] The controller chooses between paying the blood or
/// burning the vampire.
#[test]
fn ex_nihilo_vampire_burns_a_blood_or_is_burned_in_the_master_phase() {
    let mut e = game(2);
    let x = e.add_minion(SeatId(0), plain("Brujah", 0), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let card = in_play(&mut e, 0, "Ex Nihilo", Some(x));
    for burn in [true, false] {
        // The master phase's one option, the rider, elides.
        let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
        assert!(req.options.contains(&Choice::BurnBloodToAvoid));
        assert!(e.state.cards[card.index()].present());
        if burn {
            e.choose(index_of(&req, Choice::BurnBloodToAvoid));
            assert_eq!(blood(&e, x), 2);
        } else {
            e.choose(index_of(&req, Choice::Pass));
            request(&mut e);
            assert!(!e.state.minions[x.index()].in_play, "burned");
        }
    }
}

/// Ex Nihilo [nec]: burned in its controller's unlock phase only; [NEC]
/// in another Methuselah's too.
#[test]
fn ex_nihilo_burns_in_its_unlock_phase_or_any_at_superior() {
    for superior in [false, true] {
        let mut e = game(2);
        let x = e.add_minion(SeatId(0), plain("Brujah", 0), 3, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let card = in_play_mode(&mut e, 0, "Ex Nihilo", Some(x), superior as u8);
        let burn = |o: &Choice| matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), .. } if *c == card);
        // Seat 1's unlock phase.
        let mut offered = false;
        loop {
            let req = request(&mut e);
            if (req.seat, req.window) == (SeatId(1), WindowLabel::MinionPhase) {
                break;
            }
            offered |= req.options.iter().any(burn);
            e.choose(fallback(&req));
        }
        assert_eq!(offered, superior);
        let mut used = false;
        loop {
            let req = request(&mut e);
            if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
                break;
            }
            if let Some(i) = req.options.iter().position(burn) {
                e.choose(i);
                used = true;
                continue;
            }
            e.choose(fallback(&req));
        }
        assert!(used, "its controller's unlock phase");
        assert!(ash_has(&e, 0, "Ex Nihilo"));
        assert!(e.state.minions[x.index()].in_play);
    }
}

/// Fantasy World [chi]: on the prey's ready vampire, locked; it does not
/// unlock as normal; in its controller's unlock phase they burn 1 blood
/// to burn the card.
#[test]
fn fantasy_world_locks_and_is_burned_for_a_blood_in_their_unlock_phase() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("chi", 0), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Fantasy World"]);
    pick(&mut e, play_on(0, 0, a, Target::Minion(t)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let card = placed_on(&e, "Fantasy World", Some(t));
    assert!(e.state.minions[t.index()].locked);
    assert_eq!(blood(&e, a), 1, "2 blood");
    let mut used = false;
    loop {
        let req = request(&mut e);
        if (req.seat, req.window) == (SeatId(1), WindowLabel::MinionPhase) {
            break;
        }
        if let Some(&u) = uses(&req, card.0).first() {
            assert_eq!(req.seat, SeatId(1));
            assert!(e.state.minions[t.index()].locked, "not unlocked as normal");
            pick(&mut e, u);
            used = true;
            continue;
        }
        e.choose(fallback(&req));
    }
    assert!(used);
    assert_eq!(blood(&e, t), 3);
    assert!(ash_has(&e, 0, "Fantasy World"));
}

/// Fantasy World: its minion plays no card; another of its player's does.
#[test]
fn fantasy_world_minion_plays_no_card() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let o = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 1, "Fantasy World", Some(t));
    hand(&mut e, 0, &[".44 Magnum"]);
    let req = request(&mut e);
    assert!(plays(&req)
        .iter()
        .any(|p| matches!(p, Choice::Play { actor: Some(m), .. } if *m == o)));
    assert!(!plays(&req)
        .iter()
        .any(|p| matches!(p, Choice::Play { actor: Some(m), .. } if *m == t)));
    assert!(req.options.contains(&bleed(t)), "rulebook actions stay");
}

/// Fantasy World: its minion rushed enters combat; the card burns.
#[test]
fn fantasy_world_burns_as_its_minion_enters_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let card = in_play(&mut e, 0, "Fantasy World", Some(t));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(t));
    request(&mut e);
    assert!(!e.state.cards[card.index()].present());
    assert!(ash_has(&e, 0, "Fantasy World"));
}

// ---- action_a: removals, damage, bleeds ----------------------------------------

/// Chair of Hades [nec]: a mortal ally of the prey's removed from the game
/// (not burned), the action directed at its controller; a ghoul retainer
/// leaves its vampire for the removed cards.
#[test]
fn chair_of_hades_removes_a_mortal_ally_or_a_ghoul_retainer() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("nec", 0), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let h = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let taxi = e.add_ally(SeatId(1), find_library("City Star Taxi"));
    let wraith = e.add_ally(SeatId(1), find_library("Spectral Servitor"));
    let ghoul = in_play(&mut e, 1, "Ghoul Retainer", Some(h));
    hand(&mut e, 0, &["Chair of Hades"]);
    let req = request(&mut e);
    assert!(plays(&req).contains(&play_on(0, 0, a, Target::Minion(taxi))));
    assert!(!plays(&req).contains(&play_on(0, 0, a, Target::Minion(wraith))));
    assert!(!plays(&req).contains(&play_on(0, 0, a, Target::Minion(h))));
    assert!(plays(&req).contains(&play_on(0, 1, a, Target::Card(ghoul))));
    pick(&mut e, play_on(0, 0, a, Target::Minion(taxi)));
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(1), WindowLabel::ActionBlock)
    );
    assert!(offers_block(&req, h), "directed at the ally's controller");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let card = find_library("City Star Taxi");
    assert!(!e.state.minions[taxi.index()].in_play);
    assert!(e.state.seats[1].removed.contains(&card));
    assert!(!e.state.seats[1].ash.contains(&card), "removed, not burned");
}

/// Chair of Hades [NEC]: a capacity-2 vampire removed (its crypt card to
/// the removed cards); a bigger vampire and a wraith ally are no targets; a
/// non-wraith retainer is.
#[test]
fn chair_of_hades_superior_removes_a_small_vampire_not_a_wraith() {
    let small = textless(|c| c.capacity == 2 && !c.adv, 0);
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("NEC", 0), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let s = e.add_minion(SeatId(1), small, 2, false);
    let big = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let wraith = e.add_ally(SeatId(1), find_library("Spectral Servitor"));
    let taxi = e.add_ally(SeatId(1), find_library("City Star Taxi"));
    let spirit = in_play(&mut e, 1, "Vengeful Spirit", Some(big));
    let ghoul = in_play(&mut e, 1, "Ghoul Retainer", Some(big));
    hand(&mut e, 0, &["Chair of Hades"]);
    let req = request(&mut e);
    let offered = |m| plays(&req).contains(&play_on(0, 2, a, Target::Minion(m)));
    assert!(offered(s) && offered(taxi));
    assert!(!offered(big) && !offered(wraith));
    assert!(plays(&req).contains(&play_on(0, 3, a, Target::Card(ghoul))));
    assert!(!plays(&req).contains(&play_on(0, 3, a, Target::Card(spirit))));
    pick(&mut e, play_on(0, 2, a, Target::Minion(s)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[s.index()].in_play);
    assert!(e.state.seats[1].removed_crypt.contains(&small));
}

/// Edged Illusion [chi]: the prey's ready vampire locked and 1 damage on
/// it, unpreventable; 2 blood paid.
#[test]
fn edged_illusion_locks_and_damages_a_ready_minion() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("chi", 0), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 2, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Edged Illusion"]);
    pick(&mut e, play_on(0, 0, a, Target::Minion(t)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[t.index()].locked);
    assert_eq!(blood(&e, t), 1);
    assert_eq!(blood(&e, a), 1);
}

/// Edged Illusion [CHI]: each ready minion of the chosen Methuselah takes
/// 1 damage, the torpid one none, one under Secure Haven (untargetable by
/// another's action) none, an Ex Nihilo vampire (immune) none.
/// [LSJ 20071024] Targets each ready minion of the chosen Methuselah.
/// [RTR 20080808] Minions it cannot target: no effect on them.
#[test]
fn edged_illusion_superior_damages_each_ready_minion_it_may_target() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("CHI", 0), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let r1 = e.add_minion(SeatId(1), plain("Ventrue", 0), 3, false);
    let ally = e.add_ally(SeatId(1), find_library("City Star Taxi"));
    let torpid = e.add_minion(SeatId(1), plain("Ventrue", 1), 2, true);
    let haven = e.add_minion(SeatId(1), plain("Ventrue", 2), 3, false);
    let immune = e.add_minion(SeatId(1), plain("Ventrue", 3), 3, false);
    in_play(&mut e, 1, "Secure Haven", Some(haven));
    in_play(&mut e, 1, "Ex Nihilo", Some(immune));
    hand(&mut e, 0, &["Edged Illusion"]);
    pick(&mut e, play_on(0, 1, a, Target::Seat(SeatId(1))));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, r1), 2);
    assert_eq!(blood(&e, ally), 1, "a life");
    assert_eq!(blood(&e, torpid), 2);
    assert_eq!(blood(&e, haven), 3);
    assert_eq!(blood(&e, immune), 3);
    assert!(!e.state.minions[r1.index()].locked, "no lock at [CHI]");
}

/// Edged Illusion [CHI]: the chosen Methuselah's only ready minion gone to
/// torpor before resolution, the action fizzles unpaid.
/// [LSJ 20071024] Each ready minion is a target. [rules-qa 117.1] All the
/// targets illegal: the action fizzles and is not paid.
#[test]
fn edged_illusion_superior_fizzles_unpaid_with_no_ready_minion_left() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("CHI", 0), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let r = e.add_minion(SeatId(1), plain("Ventrue", 0), 3, false);
    hand(&mut e, 0, &["Edged Illusion"]);
    pick(&mut e, play_on(0, 1, a, Target::Seat(SeatId(1))));
    request(&mut e);
    assert!(e.action().is_some(), "the action under way");
    e.state.minions[r.index()].in_torpor = true;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Fizzled));
    assert_eq!(blood(&e, a), 3, "not paid");
    assert_eq!(blood(&e, r), 3);
}

/// Edged Illusion [CHI]: a Dual Form vampire and its copy, both empty, go
/// to torpor at once: their controller keeps one, the other burns.
/// [LSJ 20050805] If more than one go to torpor at the same time (eg. from
/// Edged Illusion), the controller chooses which one they keep.
#[test]
fn edged_illusion_superior_sends_a_dual_form_pair_down_together() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("CHI", 0), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let d = e.add_minion(SeatId(1), plain("Gangrel", 0), 0, false);
    let copy = e.add_minion(SeatId(1), plain("Gangrel", 1), 0, false);
    e.state.minions[copy.index()].dual_of = Some(d);
    hand(&mut e, 0, &["Edged Illusion"]);
    pick(&mut e, play_on(0, 1, a, Target::Seat(SeatId(1))));
    let req = until(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
    assert_eq!(
        req.options,
        vec![Choice::ChooseMinion(d), Choice::ChooseMinion(copy)]
    );
    e.choose(index_of(&req, Choice::ChooseMinion(d)));
    request(&mut e);
    assert!(!e.state.minions[copy.index()].in_play, "burned");
    assert!(e.state.minions[d.index()].in_torpor, "kept");
}

/// Dream World [PRE]: each of the player's minions bleeds at +1 until a
/// bleed is blocked; [pre] is a bleed at +1, still offered after a [PRE]
/// this turn, a second [PRE] not.
#[test]
fn dream_world_superior_bleeds_until_a_bleed_is_blocked() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("PRE", 0), 3, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let c = e.add_minion(SeatId(0), plain("Brujah", 2), 4, false);
    let d = e.add_minion(SeatId(0), vampire_with("pre", 1), 4, false);
    let blocker = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Dream World", "Dream World"]);
    pick(&mut e, play(0, 1, a));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!plays(&req).contains(&play(0, 1, d)), "one superior a turn");
    assert!(plays(&req).contains(&play(0, 0, d)));
    pick(&mut e, bleed(b));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 8, "bled for 2");
    pick(&mut e, bleed(c));
    pick_when(&mut e, block(blocker));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, bleed(d));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 7, "the bonus ended with the block");
}

/// Enticement [ser]: the Edge burned for 2 of the prey's pool; the player's
/// minions then bleed no more this turn, by a card neither.
/// [LSJ 20010807] Is directed at your prey.
#[test]
fn enticement_burns_the_edge_for_prey_pool_and_bars_bleeds() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), vampire_with("ser", 0), 3, false);
    let b = e.add_minion(SeatId(0), vampire_with("pre", 0), 4, false);
    let prey = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Toreador", 0), 4, false);
    e.state.seats[0].has_edge = true;
    hand(&mut e, 0, &["Enticement", "Dream World"]);
    pick(&mut e, play_on(0, 0, a, Target::Seat(SeatId(1))));
    let req = request(&mut e);
    assert_eq!(req.seat, SeatId(1), "the prey blocks");
    assert!(offers_block(&req, prey));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.seats[0].has_edge);
    assert_eq!(e.state.seats[1].pool, 8);
    assert!(!req.options.contains(&bleed(b)));
    assert!(!plays(&req).contains(&play(0, 0, b)), "no bleed card");
    assert!(req.options.contains(&action(b, ActionKind::Hunt)));
}

/// Enticement [SER]: the Edge gone before it resolves: no pool burned, and
/// the bleeds still barred.
/// [LSJ 20100527] The Edge lost during the action, the prey takes no
/// damage but you still cannot bleed.
#[test]
fn enticement_without_the_edge_still_bars_bleeds() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), vampire_with("SER", 0), 3, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Toreador", 0), 4, false);
    e.state.seats[0].has_edge = true;
    hand(&mut e, 0, &["Enticement"]);
    pick(&mut e, play_on(0, 1, a, Target::Seat(SeatId(1))));
    expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    e.state.seats[0].has_edge = false;
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10);
    assert!(!req.options.contains(&bleed(b)));
}

/// Community Justice: an Anarch Brujah burns a vampire that diablerized a
/// Brujah this turn; one that diablerized a Toreador is no target, nor one
/// whose diablerie came during the player's last turn; a Camarilla vampire
/// does not play it.
/// [LSJ 20020429] "Since your last turn": between the end of your last turn
/// and now; your last turn does not qualify, your current turn does.
#[test]
fn community_justice_burns_a_diablerist_of_the_actors_clan() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].sect_override = Some(zvtes::crypt::Sect::Anarch);
    let cam = e.add_minion(SeatId(0), plain("Ventrue", 3), 4, false);
    e.state.minions[cam.index()].sect_override = Some(zvtes::crypt::Sect::Camarilla);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let y = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 2), 4, false);
    e.state.seats[0].previous_turn = 2;
    e.state.minions[x.index()]
        .diableries
        .0
        .push((4, Some("Brujah")));
    e.state.minions[y.index()]
        .diableries
        .0
        .push((4, Some("Toreador")));
    e.state.minions[z.index()]
        .diableries
        .0
        .push((2, Some("Brujah")));
    hand(&mut e, 0, &["Community Justice"]);
    let req = request(&mut e);
    assert!(plays(&req).contains(&play_on(0, 0, a, Target::Minion(x))));
    assert!(!plays(&req).contains(&play_on(0, 0, a, Target::Minion(y))));
    assert!(!plays(&req).contains(&play_on(0, 0, a, Target::Minion(z))));
    assert!(!plays(&req)
        .iter()
        .any(|p| matches!(p, Choice::Play { actor: Some(m), .. } if *m == cam)));
    pick(&mut e, play_on(0, 0, a, Target::Minion(x)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[x.index()].in_play);
    assert!(e.state.seats[1]
        .ash_crypt
        .iter()
        .any(|&c| c == plain("Ventrue", 0)));
}

// ---- Bestow Vigor ----------------------------------------------------------------

/// Bestow Vigor [for] (mode 0, by a [FOR] vampire): played on another
/// vampire of the player's, the acting vampire unlocked; the card on its
/// target.
#[test]
fn bestow_vigor_goes_on_a_vampire_and_unlocks_the_actor() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("FOR", 0), 2, false);
    let o = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Bestow Vigor"]);
    pick(&mut e, play_on(0, 0, a, Target::Minion(o)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "Bestow Vigor", Some(o));
    assert!(!e.state.minions[a.index()].locked, "unlocked");
    assert_eq!(blood(&e, a), 1);
}

/// The vampire seat 1 blocks with, its combat card plays offered.
fn blocker_plays(e: &mut Engine, b: MinionId) -> Vec<Choice> {
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    pick(e, bleed(a));
    pick_when(e, block(b));
    loop {
        let req = request(e);
        if req.seat == SeatId(1)
            && plays(&req)
                .iter()
                .any(|p| matches!(p, Choice::Play { actor: Some(m), .. } if *m == b))
        {
            return plays(&req);
        }
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            return Vec::new();
        }
        e.choose(fallback(&req));
    }
}

/// Bestow Vigor on a vampire without Fortitude: its [for] combat card at
/// basic, never at superior; it still does not have Fortitude.
/// [ANK 20180111] Never the superior version. [LSJ 19980303] It does not
/// actually have the Discipline.
#[test]
fn bestow_vigor_lets_a_vampire_play_fortitude_combat_cards_at_basic() {
    let mut e = game(2);
    let nofor = textless(
        |c| c.capacity >= 4 && !c.disciplines.iter().any(|d| d.eq_ignore_ascii_case("for")),
        0,
    );
    let b = e.add_minion(SeatId(1), nofor, 4, false);
    assert!(!e.state.has_discipline(b, "for"));
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    in_play(&mut e, 1, "Bestow Vigor", Some(b));
    hand(&mut e, 1, &["Skin of Night"]);
    let offered = blocker_plays(&mut e, b);
    assert!(offered.contains(&play(0, 0, b)));
    assert!(!offered.contains(&play(0, 1, b)), "not superior");
    assert!(!e.state.has_discipline(b, "for"));
}

/// Bestow Vigor meets one requirement of a combo combat card: with basic
/// Obeah, Penitent Resilience's [for][obe] is offered; without, not.
/// [LSJ 20011217] [LSJ 20090508] One, but not more than one, of a
/// multi-Discipline card's requirements.
#[test]
fn bestow_vigor_meets_one_requirement_of_a_combo_combat_card() {
    for obeah in [true, false] {
        let mut e = game(2);
        let nofor = textless(
            |c| {
                c.capacity >= 4
                    && !c
                        .disciplines
                        .iter()
                        .any(|d| ["for", "obe"].contains(&d.to_ascii_lowercase().as_str()))
            },
            0,
        );
        let b = e.add_minion(SeatId(1), nofor, 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        if obeah {
            e.state.minions[b.index()].gained_levels.push("obe");
        }
        in_play(&mut e, 1, "Bestow Vigor", Some(b));
        hand(&mut e, 1, &["Penitent Resilience"]);
        let offered = blocker_plays(&mut e, b);
        assert_eq!(offered.contains(&play(0, 1, b)), obeah, "obeah {obeah}");
    }
}

/// Bestow Vigor [FOR] on Tunnel Runner (its own [ani] grant): a [for]
/// combat card yes, Alpha Glint ([ani][for]) no.
/// [LSJ 20020218] Two similar effects never meet a multi-Discipline card's
/// requirements together.
#[test]
fn bestow_vigor_and_an_allys_own_grant_never_meet_a_combo_card_together() {
    let mut e = game(2);
    let b = e.add_ally(SeatId(1), find_library("Tunnel Runner"));
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    in_play_mode(&mut e, 1, "Bestow Vigor", Some(b), 1);
    hand(&mut e, 1, &["Alpha Glint", "Skin of Night"]);
    let offered = blocker_plays(&mut e, b);
    assert!(offered.contains(&play(1, 0, b)));
    assert!(!offered.contains(&play(0, 0, b)));
}

/// Bestow Vigor: in play through its turn's end and the next Methuselah's,
/// burned at the end of its controller's next turn.
#[test]
fn bestow_vigor_burns_at_the_end_of_its_controllers_next_turn() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("for", 0), 2, false);
    let o = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Bestow Vigor"]);
    pick(&mut e, play_on(0, 0, a, Target::Minion(o)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let card = placed_on(&e, "Bestow Vigor", Some(o));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.cards[card.index()].present());
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.cards[card.index()].present());
    assert!(ash_has(&e, 0, "Bestow Vigor"));
}

// ---- Fatuus Mastery ------------------------------------------------------------

/// Fatuus Mastery [CHI]: on the acting vampire with 1 counter; that counter
/// pays 1 of a [chi] card's blood (Edged Illusion: 2), announced with it;
/// a card not requiring Chimerstry is offered none.
#[test]
fn fatuus_mastery_counters_pay_chimerstry_blood_costs() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_discs(&["CHI", "pre"], 0), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 2, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Fatuus Mastery"]);
    pick(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let fatuus = placed_on(&e, "Fatuus Mastery", Some(a));
    assert_eq!(e.state.cards[fatuus.index()].counters, 1);
    // The next turn: 2 counters; a [pre] card is offered none.
    let req = request(&mut e);
    e.choose(index_of(&req, Choice::Pass));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[fatuus.index()].counters, 2);
    hand(&mut e, 0, &["Mind Numb"]);
    pick(&mut e, play_on(0, 0, a, Target::Minion(t)));
    let req = request(&mut e);
    assert!(!req
        .options
        .contains(&Choice::ResolveRider { card: fatuus.0 }));
    // A [chi] card: the counter pays 1 of its 2 blood.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_discs(&["CHI", "pre"], 0), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 2, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let fatuus = in_play_mode(&mut e, 0, "Fatuus Mastery", Some(a), 1);
    e.state.cards[fatuus.index()].counters = 1;
    hand(&mut e, 0, &["Edged Illusion"]);
    pick(&mut e, play_on(0, 0, a, Target::Minion(t)));
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    assert_eq!(
        req.options,
        vec![Choice::Pass, Choice::ResolveRider { card: fatuus.0 }]
    );
    pick(&mut e, Choice::ResolveRider { card: fatuus.0 });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 1, "1 blood and the counter");
    assert_eq!(e.state.cards[fatuus.index()].counters, 0);
    assert!(e.state.cards[fatuus.index()].present(), "not burned at 0");
}

/// Fatuus Mastery [chi]: no counter on entry; 1 more in its controller's
/// unlock phase, none in another's.
#[test]
fn fatuus_mastery_gains_a_counter_each_unlock_phase() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("chi", 0), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Fatuus Mastery"]);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let fatuus = placed_on(&e, "Fatuus Mastery", Some(a));
    assert_eq!(e.state.cards[fatuus.index()].counters, 0);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[fatuus.index()].counters, 1);
}

// ---- Picks: Bloodlust, Command the Legion, Condemn the Sins ------------------------

/// Bloodlust [pre], X=2: the actor and another minion of the player's chosen,
/// 2 blood paid, the card in play; the other takes its rush on the prey's
/// minion; the card burns at the end of the turn. It is a Frenzy card, as
/// Enrage.
#[test]
fn bloodlust_gives_its_chosen_a_press_and_a_rush() {
    use zvtes::library::{Label, LIBRARY};
    for name in ["Bloodlust", "Enrage"] {
        assert!(LIBRARY[find_library(name) as usize]
            .labels
            .contains(&Label::Frenzy));
    }
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("pre", 0), 3, false);
    let o = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let n = e.add_minion(SeatId(0), plain("Brujah", 2), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Bloodlust"]);
    pick(&mut e, play(0, 0, a));
    pick(&mut e, Choice::ChooseMinion(a));
    pick(&mut e, Choice::ChooseMinion(o));
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let card = placed_on(&e, "Bloodlust", None);
    assert_eq!(e.state.cards[card.index()].chosen, vec![a, o]);
    assert_eq!(blood(&e, a), 1, "X = 2");
    assert_eq!(e.state.combat_presses_of(o, t), 1);
    assert_eq!(e.state.combat_presses_of(n, t), 0);
    let req = request(&mut e);
    let rush = Choice::UseAbility {
        source: AbilitySource::Card(card),
        ability: 0,
        actor: Some(o),
        target: Target::Minion(t),
    };
    assert!(req.options.contains(&rush));
    assert!(!uses(&req, card.0)
        .iter()
        .any(|u| matches!(u, Choice::UseAbility { actor: Some(m), .. } if *m == n)));
    pick(&mut e, rush);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.cards[card.index()].present());
    assert!(ash_has(&e, 0, "Bloodlust"));
}

/// Bloodlust [PRE], X=0: one minion chosen, nothing paid; Pass not offered
/// before it.
#[test]
fn bloodlust_superior_chooses_x_plus_one() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("PRE", 0), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Bloodlust"]);
    pick(&mut e, play(0, 1, a));
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    assert!(!req.options.contains(&Choice::Pass));
    pick(&mut e, Choice::ChooseMinion(a));
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let card = placed_on(&e, "Bloodlust", None);
    assert_eq!(e.state.cards[card.index()].chosen, vec![a]);
    assert_eq!(blood(&e, a), 2, "X = 0");
}

/// Bloodlust: a minion of the prey's chosen makes the action directed at
/// the prey, who may block; a frenzy-immune one (Blade of Enoch) is not
/// offered.
#[test]
fn bloodlust_choosing_a_prey_minion_directs_the_action_at_them() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), vampire_with("pre", 0), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let calm = e.add_minion(SeatId(1), textless(|c| c.capacity >= 6, 0), 4, false);
    e.add_minion(SeatId(2), plain("Toreador", 0), 4, false);
    in_play(&mut e, 1, "Blade of Enoch", Some(calm));
    hand(&mut e, 0, &["Bloodlust"]);
    pick(&mut e, play(0, 0, a));
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    assert!(!req.options.contains(&Choice::ChooseMinion(calm)));
    pick(&mut e, Choice::ChooseMinion(t));
    pick(&mut e, Choice::Pass);
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(1), WindowLabel::ActionBlock)
    );
    assert!(offers_block(&req, t));
}

/// Bloodlust is played on each minion it chooses, not on its actor: a
/// frenzy-immune actor (The Path of the Scorched Heart) plays it choosing
/// another; Meditative Grove cancels it as it chooses its controller's
/// Salubri, X unpaid.
/// [rules-qa 28.11] [LSJ 20051113] A frenzy card is played on the minion it
/// chooses.
#[test]
fn bloodlust_is_played_on_its_chosen_minions() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("pre", 0), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), textless(|c| c.clan == "Salubri", 0), 4, false);
    in_play(&mut e, 0, "Path of the Scorched Heart, The", Some(a));
    let grove = in_play(&mut e, 1, "Meditative Grove", None);
    hand(&mut e, 0, &["Bloodlust"]);
    let req = request(&mut e);
    assert!(plays(&req).contains(&play(0, 0, a)), "an immune actor");
    pick(&mut e, play(0, 0, a));
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    assert!(
        !req.options.contains(&Choice::ChooseMinion(a)),
        "not on itself"
    );
    pick(&mut e, Choice::ChooseMinion(z));
    pick(&mut e, Choice::Pass);
    let req = request(&mut e);
    let cancel = use_on(grove, 1, None, Target::None);
    assert_eq!(req.seat, SeatId(1));
    assert!(req.options.contains(&cancel), "{:?}", req.options);
    e.choose(index_of(&req, cancel));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "Bloodlust"));
    assert!(e.state.cards[grove.index()].locked);
    assert_eq!(e.state.combat_presses_of(z, a), 0, "canceled");
    // An action card's cost is paid at resolution: canceled, X is not paid
    // (rules-qa 77.4, ANK 20260216's non-action card).
    assert_eq!(blood(&e, a), 3, "X unpaid");
}

/// Command the Legion [dom], X=3: two minions locked, the prey's ally and
/// its younger vampire; an older vampire is not offered; 3 blood paid; the
/// action directed at the prey.
#[test]
fn command_the_legion_locks_x_minus_one_allies_or_younger_vampires() {
    let small = textless(|c| c.capacity == 3 && !c.adv, 0);
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_discs(&["dom"], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("City Star Taxi"));
    let young = e.add_minion(SeatId(1), small, 3, false);
    let old = e.add_minion(SeatId(1), cap_with("", 7, 0), 4, false);
    hand(&mut e, 0, &["Command the Legion"]);
    pick(&mut e, play(0, 0, a));
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    assert!(req.options.contains(&Choice::ChooseMinion(ally)));
    assert!(req.options.contains(&Choice::ChooseMinion(young)));
    assert!(!req.options.contains(&Choice::ChooseMinion(old)));
    pick(&mut e, Choice::ChooseMinion(ally));
    pick(&mut e, Choice::ChooseMinion(young));
    // None left to pick: the Pass elides.
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(1), WindowLabel::ActionBlock)
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[ally.index()].locked);
    assert!(e.state.minions[young.index()].locked);
    assert!(!e.state.minions[old.index()].locked);
    assert_eq!(blood(&e, a), 1, "X = 3");
}

/// Command the Legion [DOM]: one older vampire with a younger one; a second
/// older is not offered.
#[test]
fn command_the_legion_superior_takes_one_older() {
    let small = textless(|c| c.capacity == 3 && !c.adv, 0);
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_discs(&["DOM"], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let young = e.add_minion(SeatId(1), small, 3, false);
    let old = e.add_minion(SeatId(1), cap_with("", 7, 0), 4, false);
    let old2 = e.add_minion(SeatId(1), cap_with("", 7, 1), 4, false);
    hand(&mut e, 0, &["Command the Legion"]);
    pick(&mut e, play(0, 1, a));
    pick(&mut e, Choice::ChooseMinion(old));
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    assert!(!req.options.contains(&Choice::ChooseMinion(old2)));
    pick(&mut e, Choice::ChooseMinion(young));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[old.index()].locked);
    assert!(e.state.minions[young.index()].locked);
    assert!(!e.state.minions[old2.index()].locked);
}

/// Command the Legion: picks of two other Methuselahs' minions; both may
/// block.
#[test]
fn command_the_legion_is_directed_at_each_picked_minions_controller() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), with_discs(&["dom"], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let x = e.add_ally(SeatId(1), find_library("City Star Taxi"));
    let b1 = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let y = e.add_ally(SeatId(2), find_library("City Star Taxi"));
    let b2 = e.add_minion(SeatId(2), plain("Toreador", 0), 4, false);
    hand(&mut e, 0, &["Command the Legion"]);
    pick(&mut e, play(0, 0, a));
    pick(&mut e, Choice::ChooseMinion(x));
    pick(&mut e, Choice::ChooseMinion(y));
    pick(&mut e, Choice::Pass);
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(1), WindowLabel::ActionBlock)
    );
    assert!(offers_block(&req, b1));
    e.choose(index_of(&req, Choice::Pass));
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(2), WindowLabel::ActionBlock)
    );
    assert!(offers_block(&req, b2));
}

/// Condemn the Sins of the Father [QUI]: every Brujah younger than the
/// actor locks and burns 1 blood, two Methuselahs', the older Brujah
/// spared; both controllers may block; a clan with no younger vampire is
/// not offered.
/// [RTR 20081119] [ANK 20180917] Directed at all Methuselah controlling one
/// of the chosen targets.
#[test]
fn condemn_the_sins_locks_every_younger_vampire_of_the_clan() {
    let young = |n| textless(|c| c.clan == "Brujah" && c.capacity <= 4 && !c.adv, n);
    let elder = textless(|c| c.clan != "Brujah" && c.capacity == 5 && !c.adv, 0);
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), elder, 4, false);
    e.state.minions[a.index()]
        .gained_levels
        .extend(["qui", "qui"]);
    e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    let y1 = e.add_minion(SeatId(1), young(0), 2, false);
    let y2 = e.add_minion(SeatId(2), young(1), 2, false);
    let toreador = e.add_minion(
        SeatId(1),
        textless(|c| c.clan == "Toreador" && c.capacity <= 4, 0),
        2,
        false,
    );
    // Of the same age or older: not younger.
    let old = e.add_minion(
        SeatId(2),
        textless(|c| c.clan == "Brujah" && c.capacity >= 5, 0),
        4,
        false,
    );
    hand(&mut e, 0, &["Condemn the Sins of the Father"]);
    pick(&mut e, play(0, 1, a));
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    assert!(req.options.contains(&Choice::ChooseClan("Brujah")));
    assert!(
        !req.options.contains(&Choice::ChooseClan("Salubri")),
        "none in play"
    );
    pick(&mut e, Choice::ChooseClan("Brujah"));
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(1), WindowLabel::ActionBlock)
    );
    e.choose(index_of(&req, Choice::Pass));
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(2), WindowLabel::ActionBlock)
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[y1.index()].locked && e.state.minions[y2.index()].locked);
    assert_eq!((blood(&e, y1), blood(&e, y2)), (1, 1));
    assert!(!e.state.minions[old.index()].locked);
    assert_eq!(blood(&e, old), 4);
    assert!(!e.state.minions[toreador.index()].locked);
}

/// Condemn the Sins of the Father with no vampire younger than the actor in
/// play: no clan to choose, the card not offered (found by the fuzz, 30b
/// seed 11: an empty clan choice).
#[test]
fn condemn_the_sins_is_not_offered_without_a_younger_vampire() {
    let elder = textless(|c| c.clan != "Brujah" && c.capacity == 5 && !c.adv, 0);
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), elder, 4, false);
    e.state.minions[a.index()]
        .gained_levels
        .extend(["qui", "qui"]);
    e.add_minion(
        SeatId(1),
        textless(|c| c.clan == "Brujah" && c.capacity >= 5, 0),
        4,
        false,
    );
    hand(&mut e, 0, &["Condemn the Sins of the Father"]);
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::MinionPhase);
    assert!(
        !req.options.iter().any(|o| matches!(o, Choice::Play { .. })),
        "{:?}",
        req.options
    );
}

// ---- Compel the Spirit -------------------------------------------------------------

/// Compel the Spirit [nec]: the player's ally burned this turn back to hand;
/// one burned during their last turn is not nameable, and with only it the
/// card is not offered.
/// [LSJ 20020429] "Since your last turn": the last turn does not qualify,
/// the current one does.
#[test]
fn compel_the_spirit_returns_a_recently_burned_ally_to_hand() {
    let taxi = find_library("City Star Taxi");
    let mystic = find_library("Vagabond Mystic");
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("nec", 0), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let old = e.add_ally(SeatId(0), mystic);
    e.state.turn = 2;
    e.burn(old);
    e.state.seats[0].previous_turn = 2;
    e.state.turn = 4;
    hand(&mut e, 0, &["Compel the Spirit"]);
    let req = request(&mut e);
    assert!(
        !plays(&req).contains(&play(0, 0, a)),
        "only one of the last turn"
    );
    let x = e.add_ally(SeatId(0), taxi);
    e.burn(x);
    let req = request(&mut e);
    e.choose(index_of(&req, Choice::Pass));
    let mut e = {
        let mut e = game(2);
        let a2 = e.add_minion(SeatId(0), vampire_with("nec", 0), 2, false);
        assert_eq!(a2, a);
        e
    };
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let old = e.add_ally(SeatId(0), mystic);
    e.state.turn = 2;
    e.burn(old);
    e.state.seats[0].previous_turn = 2;
    e.state.turn = 4;
    let x = e.add_ally(SeatId(0), taxi);
    e.burn(x);
    hand(&mut e, 0, &["Compel the Spirit"]);
    pick(&mut e, play(0, 0, a));
    // The one naming elides: the taxi.
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[0].hand.contains(&taxi));
    assert!(!e.state.seats[0].ash.contains(&taxi));
    assert!(e.state.seats[0].ash.contains(&mystic));
    assert_eq!(blood(&e, a), 1);
}

/// Compel the Spirit: the named ally moved out of the ash heap before it
/// resolves: no effect, no blood paid.
/// [RTR 19980928] [LSJ 19980929] Only if the ally or retainer is in the ash
/// heap, not if it was moved elsewhere.
#[test]
fn compel_the_spirit_needs_the_card_still_in_the_ash_heap() {
    let taxi = find_library("City Star Taxi");
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("nec", 0), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let x = e.add_ally(SeatId(0), taxi);
    e.burn(x);
    hand(&mut e, 0, &["Compel the Spirit"]);
    pick(&mut e, play(0, 0, a));
    expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    let seat = &mut e.state.seats[0];
    let k = seat.ash.iter().position(|&c| c == taxi).unwrap();
    let (c, copy) = seat.ash.take(k);
    seat.removed.put(c, copy);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.seats[0].hand.contains(&taxi));
    assert_eq!(blood(&e, a), 2, "not paid");
}

/// Compel the Spirit [NEC]: a burned ally back in play at its starting life,
/// a new minion able to act again this turn; a burned retainer on the acting
/// vampire.
/// [LSJ 20010627] A new ally, not bound by its previous incarnation's
/// actions. "Put card in play ignoring requirements": the inferior version
/// ([RBK equip] [RBK recruit-ally] [RBK employ-retainer]); X is zero, and it
/// may act this turn ([LSJ 20100204] [LSJ 20040518-2] [LSJ 20100302-1]).
#[test]
fn compel_the_spirit_superior_raises_an_ally_able_to_act_at_inferior() {
    let taxi = find_library("City Star Taxi");
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("NEC", 0), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let x = e.add_ally(SeatId(0), taxi);
    e.state.minions[x.index()].blood = 1;
    e.state.minions[x.index()].bled_this_turn = true;
    e.burn(x);
    hand(&mut e, 0, &["Compel the Spirit"]);
    pick(&mut e, play(0, 1, a));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let m = (0..e.state.minions.len())
        .map(|i| MinionId(i as u32))
        .find(|&m| {
            m != x
                && e.state.minions[m.index()].in_play
                && e.state.minions[m.index()].kind.ally_card() == Some(taxi)
        })
        .expect("back in play");
    assert_eq!(blood(&e, m), 2, "its starting life");
    assert!(
        req.options.contains(&bleed(m)),
        "acts this turn, a new ally"
    );
    // A retainer, burned with its vampire.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("NEC", 0), 2, false);
    let h = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let ghoul = in_play(&mut e, 0, "Ghoul Retainer", Some(h));
    e.state.cards[ghoul.index()].counters = 1;
    e.burn(h);
    hand(&mut e, 0, &["Compel the Spirit"]);
    pick(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let back = placed_on(&e, "Ghoul Retainer", Some(a));
    assert_eq!(e.state.cards[back.index()].counters, 2, "its starting life");
}

/// Compel the Spirit [NEC]: Reanimated Corpse back with X zero has no
/// pathos counter and is burned at once.
/// [LSJ 20071015] Brought back with X zero, it has no pathos counter and
/// burns immediately.
#[test]
fn compel_the_spirit_superior_returns_reanimated_corpse_with_no_pathos() {
    let corpse = find_library("Reanimated Corpse");
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("NEC", 0), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    // Burned from play this turn (a fixture: its burn removes it).
    let none = zvtes::copies::CopyId::NONE;
    e.state.seats[0].ash.put(corpse, none);
    e.state.seats[0].burned_minion_cards.0.push((none, 4));
    hand(&mut e, 0, &["Compel the Spirit"]);
    pick(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e
        .state
        .minions
        .iter()
        .any(|m| m.in_play && m.kind.ally_card() == Some(corpse)));
    assert!(
        !e.state.seats[0].ash.contains(&corpse),
        "not in the ash heap"
    );
}

/// Compel the Spirit is not offered on a card that represented another in
/// play: Khazar's Diary's wraith (an equipment card), a Spell of Life mummy
/// (a vampire in the ash heap), Jake Washington (a master card).
/// [LSJ 20071014] Not if the ally in the ash heap was representing
/// something else (Khazar's Diary). [LSJ 20071014] A burned mummy counts as
/// a vampire (not Compel the Spirit). [LSJ 20071014] [LSJ 20020115] [LSJ
/// 20030128] Jake Washington is not an ally in the ash heap.
#[test]
fn compel_the_spirit_is_not_played_on_a_card_that_represented_another() {
    let tasha = find_library("City Star Taxi");
    // The Diary's wraith.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("nec", 0), 2, false);
    let g = e.add_minion(SeatId(0), giovanni(0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0].ash = vec![tasha].into();
    let d = diary(&mut e, 0, 7);
    pick(&mut e, granted(d.0, g));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let w = represented(&e, tasha, false).expect("the wraith");
    e.burn(w);
    assert!(e.state.seats[0].ash.contains(&tasha));
    hand(&mut e, 0, &["Compel the Spirit"]);
    let req = request(&mut e);
    assert!(!plays(&req)
        .iter()
        .any(|p| matches!(p, Choice::Play { actor: Some(m), .. } if *m == a)));
    // A mummy.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("nec", 0), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let x = textless(|c| c.capacity == 4, 3);
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: x,
            copy: Default::default(),
            counters: 0,
            banished: None,
        });
    let mine: Vec<CardId> = (0..5)
        .map(|_| in_play(&mut e, 0, "Spell of Life", None))
        .collect();
    pick(&mut e, spell_burn(mine[0]));
    pick(&mut e, Choice::Slot(0));
    let m = represented(&e, x, true).expect("a mummy");
    e.burn(m);
    hand(&mut e, 0, &["Compel the Spirit"]);
    let req = request(&mut e);
    assert!(!plays(&req)
        .iter()
        .any(|p| matches!(p, Choice::Play { actor: Some(m), .. } if *m == a)));
    // Jake Washington.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("nec", 0), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Jake Washington"]);
    e.at_master_phase();
    pick(&mut e, master_play(0));
    request(&mut e);
    let jake = find_library("Jake Washington");
    let body = e
        .state
        .cards
        .iter()
        .find(|c| c.card == jake && c.in_play)
        .and_then(|c| c.body)
        .expect("an ally");
    e.burn(body);
    assert!(e.state.seats[0].ash.contains(&jake));
    hand(&mut e, 0, &["Compel the Spirit"]);
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!plays(&req)
        .iter()
        .any(|p| matches!(p, Choice::Play { actor: Some(m), .. } if *m == a)));
}

// ---- The Eternal Mask ----------------------------------------------------------------

/// A unique vampire in `seat`'s ash heap, burned from play: its crypt card.
fn ash_unique(e: &mut Engine, seat: usize, nth: usize) -> u16 {
    let v = textless(|c| c.unique && c.capacity >= 4 && !c.adv, nth);
    e.state.seats[seat].ash_crypt.put(v, Default::default());
    v
}

/// The Eternal Mask [SER] with X blood moved, `heap`'s vampire `v`: the card
/// on `a`, the vampire raised.
fn mask(e: &mut Engine, a: MinionId, heap: u8, v: u16, x: u8) {
    hand(e, 0, &["Eternal Mask, The"]);
    pick(e, play(0, 1, a));
    let req = request(e);
    if req.window == WindowLabel::PlayTerms
        && req.options.contains(&Choice::ChooseSeat(SeatId(heap)))
    {
        e.choose(index_of(&req, Choice::ChooseSeat(SeatId(heap))));
    }
    let req = request(e);
    if req.options.contains(&Choice::AshVampire(v)) {
        e.choose(index_of(&req, Choice::AshVampire(v)));
    }
    pick(e, set_x(x));
}

/// The Eternal Mask [SER]: the prey's burned unique vampire to the player's
/// ready region with 2 blood moved from the actor; the card on the actor,
/// which then acts, blocks, reacts and votes no more; no block offered to
/// the prey (undirected).
/// [LSJ 20010627] [ANK 20180202] An undirected action, even at another
/// Methuselah's ash heap.
#[test]
fn eternal_mask_superior_raises_a_unique_vampire_from_another_ash_heap() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("SER", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let v = ash_unique(&mut e, 1, 0);
    mask(&mut e, a, 1, v, 2);
    request(&mut e);
    assert_eq!(
        e.action().expect("the action").directed_seat(&e.state),
        None
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let card = placed_on(&e, "Eternal Mask, The", Some(a));
    let m = minion_of(&e, 0, v);
    assert_eq!(blood(&e, m), 2);
    assert_eq!(blood(&e, a), 1, "1 blood paid, 2 moved");
    assert_eq!(e.state.cards[card.index()].chosen, vec![m]);
    assert_eq!(
        e.state.minions[m.index()].owner_seat(),
        SeatId(1),
        "owned by its heap's"
    );
    assert!(!e.state.seats[1].ash_crypt.contains(&v));
    assert_eq!(e.state.only_actions(a), Some(&[][..]), "cannot act");
}

/// The Eternal Mask: no unique vampire in any ash heap, [SER] is not
/// offered; [ser] is.
/// [ANK 20200420-3] Cannot be played if no target is available in any ash
/// heap.
#[test]
fn eternal_mask_superior_needs_a_unique_vampire_in_an_ash_heap() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("SER", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Eternal Mask, The"]);
    let req = request(&mut e);
    assert!(!plays(&req).contains(&play(0, 1, a)));
    assert!(plays(&req).contains(&play(0, 0, a)));
}

/// The Eternal Mask: in another Methuselah's unlock phase its controller
/// burns it and its vampire; the vampire burned since, or burned and
/// returned anew, the ability is not offered; the Mask's bearer burned, the
/// vampire stays.
/// [LSJ 20070928-1] Not if the vampire is no longer in play. [LSJ 20040726]
/// [ANK 20191031] Not if it was burned and later returned. [LSJ 20071206]
/// [LSJ 20090725] The Mask burned otherwise, the vampire is unaffected.
#[test]
fn eternal_mask_burns_itself_and_its_vampire_in_any_unlock_phase() {
    let setup = || {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("SER", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let v = ash_unique(&mut e, 1, 0);
        mask(&mut e, a, 1, v, 1);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let card = placed_on(&e, "Eternal Mask, The", Some(a));
        let m = minion_of(&e, 0, v);
        (e, a, card, m, v)
    };
    let burn = |card: CardId| move |o: &Choice| matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), .. } if *c == card);
    // Seat 1's unlock phase: used.
    let (mut e, _, card, m, _) = setup();
    let req = request(&mut e);
    e.choose(index_of(&req, Choice::Pass));
    let req = loop {
        let req = request(&mut e);
        if req.options.iter().any(burn(card)) {
            break req;
        }
        e.choose(fallback(&req));
    };
    assert_eq!(req.seat, SeatId(0));
    assert_eq!(e.state.active, SeatId(1));
    let i = req.options.iter().position(burn(card)).unwrap();
    e.choose(i);
    request(&mut e);
    assert!(!e.state.minions[m.index()].in_play);
    assert!(!e.state.cards[card.index()].present());
    // Its vampire burned, then back: never offered.
    let (mut e, _, card, m, v) = setup();
    e.burn(m);
    let v2 = e.add_minion(SeatId(0), v, 3, false);
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, burn(card));
    assert!(e.state.minions[v2.index()].in_play);
    // The bearer burned: the vampire stays.
    let (mut e, a, card, m, _) = setup();
    e.burn(a);
    assert!(!e.state.cards[card.index()].present());
    assert!(e.state.minions[m.index()].in_play);
}

/// The Eternal Mask [SER] and Blessed Resilience: a unique vampire whose
/// copy the player controls is not offered (no voluntary self-contest);
/// one another Methuselah controls is.
/// [Rulebook "Contested Cards"] "You cannot voluntarily contest cards with
/// yourself."
#[test]
fn a_unique_vampire_one_controls_is_not_raised_from_an_ash_heap() {
    let setup = |owner: u8| {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("SER", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let v = ash_unique(&mut e, 1, 0);
        e.add_minion(SeatId(owner), v, 3, false);
        hand(&mut e, 0, &["Eternal Mask, The"]);
        (e, a)
    };
    let (mut e, a) = setup(0);
    let req = request(&mut e);
    assert!(!plays(&req).contains(&play(0, 1, a)), "one's own copy");
    let (mut e, a) = setup(1);
    let req = request(&mut e);
    assert!(plays(&req).contains(&play(0, 1, a)), "another's copy");
    // Blessed Resilience: the burned necromancer's copy controlled.
    let necromancer = textless(
        |c| c.unique && c.disciplines.contains(&"FOR") && c.disciplines.contains(&"NEC"),
        0,
    );
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), necromancer, 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.burn(n);
    e.add_minion(SeatId(0), necromancer, 3, false);
    hand(&mut e, 0, &["Blessed Resilience"]);
    e.at_master_phase();
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, master_play(0));
}

// ---- Gramle ----------------------------------------------------------------------------

/// Gramle: naming Govern the Unaligned of the prey's ash heap, the player's
/// copy found in their library, shown, to hand; the action undirected
/// (naming a card is no target); Gramle replaced only as the action ends.
/// [LSJ 20001120] Not replaced if blocked: replaced as the action ends,
/// after all combats. [LSJ 20101021] Replaced after every "after
/// resolution" effect; the action has ended.
#[test]
fn gramle_finds_a_named_library_card_replaced_as_the_action_ends() {
    let govern = find_library("Govern the Unaligned");
    let filler = find_library("Conceal");
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[1].ash.put(govern, Default::default());
    let mut lib = vec![govern];
    lib.extend([filler; 20]);
    e.state.seats[0].library = lib.into();
    hand(&mut e, 0, &["Gramle"]);
    e.notes = Some(Vec::new());
    let full = e.state.hand_size(SeatId(0));
    pick(&mut e, play(0, 0, a));
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    assert!(req.options.contains(&Choice::FindCard(govern)));
    pick(&mut e, Choice::FindCard(govern));
    request(&mut e);
    assert_eq!(
        e.action().expect("the action").directed_seat(&e.state),
        None
    );
    assert_eq!(e.state.seats[0].hand.len(), full - 1, "not yet replaced");
    pick_when(&mut e, Choice::FindCard(govern));
    assert!(e.action().is_some(), "the action under way");
    assert_eq!(e.state.seats[0].hand.len(), full, "the find, not replaced");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[0].hand.contains(&govern));
    assert!(!e.state.seats[0].library.contains(&govern));
    assert_eq!(
        e.state.seats[0].hand.len(),
        full,
        "replaced, then discarded down"
    );
    assert!(noted_reveals(&mut e)
        .iter()
        .any(|r| r.card == govern && !r.crypt && r.seat == SeatId(0)));
}

/// Gramle: naming a vampire in play, the player's crypt copy to their
/// uncontrolled region.
#[test]
fn gramle_finds_a_named_vampire_to_the_uncontrolled_region() {
    let v = plain("Ventrue", 0);
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), v, 4, false);
    e.state.seats[0].crypt = vec![plain("Toreador", 0), v].into();
    hand(&mut e, 0, &["Gramle"]);
    pick(&mut e, play(0, 0, a));
    pick(&mut e, Choice::AshVampire(v));
    pick_when(&mut e, Choice::AshVampire(v));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[0].uncontrolled.iter().any(|u| u.card == v));
    assert!(!e.state.seats[0].crypt.contains(&v));
}

// ---- Action modifiers (modifier) --------------------------------------------------------

/// The `nth` textless vampire with `disc` (case read) of exactly `cap`.
#[track_caller]
fn cap_with(disc: &str, cap: u8, nth: usize) -> u16 {
    textless(
        |c| c.capacity == cap && (disc.is_empty() || c.disciplines.contains(&disc)),
        nth,
    )
}

/// Blood Awakening [QUI]: "Only usable if a younger vampire attempts to
/// block this vampire. That attempt fails and the blocking vampire cannot
/// attempt to block this action again." The bleed lands; an older blocker's
/// attempt is never met by it.
#[test]
fn blood_awakening_fails_a_younger_vampires_attempt() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), cap_with("QUI", 6, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let young = e.add_minion(SeatId(1), cap_with("", 4, 0), 4, false);
    e.add_minion(SeatId(1), cap_with("", 4, 1), 4, false);
    hand(&mut e, 0, &["Blood Awakening"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(young));
    pick_when(&mut e, play(0, 1, a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(young));
    assert_eq!(pool(&e)[1], 9, "the bleed landed");
    assert_eq!(blood(&e, a), 3);

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), cap_with("QUI", 6, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let old = e.add_minion(SeatId(1), cap_with("", 7, 0), 4, false);
    e.add_minion(SeatId(1), cap_with("", 4, 1), 4, false);
    hand(&mut e, 0, &["Blood Awakening"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(old));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, play(0, 1, a));
}

/// Blood Awakening [qui] as a reaction: +1 intercept for the attempting
/// vampire against Fata Morgana [chi]'s +1 stealth; the block made.
#[test]
fn blood_awakening_gives_intercept_as_a_reaction() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_discs(&["chi"], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), vampire_with("qui", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Fata Morgana"]);
    hand(&mut e, 1, &["Blood Awakening"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, b));
    assert_eq!(blood(&e, b), 3, "the reaction's blood");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10, "blocked");
}

/// Fata Morgana [CHI]: "+1 bleed (limited), or +1 stealth and +1 bleed
/// (limited)": the bleed alone before any attempt; the stealth and bleed
/// only while stealth is needed.
#[test]
fn fata_morgana_bleeds_or_hides_and_bleeds() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_discs(&["CHI"], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Fata Morgana"]);
    pick(&mut e, bleed(a));
    let req = request(&mut e);
    assert!(req.options.contains(&play(0, 1, a)));
    assert!(!req.options.contains(&play(0, 2, a)), "no stealth needed");
    pick(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 8, "1 + 1 limited");

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_discs(&["CHI"], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Fata Morgana"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play(0, 2, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 8, "unblocked: 1 + 1 limited");
}

/// Inspire Greatness: two other vampires of the acting Methuselah, one
/// locked, play it on a younger acting vampire, one at superior: +1 and +2
/// strength in the block's combat (hand strike 1 + 3).
/// [LSJ 19980210] Can be played by a locked vampire. [ANK 20200515]
/// Multiple copies by different minions on the same action.
#[test]
fn inspire_greatness_strengthens_a_younger_acting_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), cap_with("", 3, 0), 3, false);
    let p1 = e.add_minion(
        SeatId(0),
        textless(
            |c| c.capacity >= 6 && c.disciplines.contains(&"pot") && c.disciplines.contains(&"pre"),
            0,
        ),
        4,
        false,
    );
    let p2 = e.add_minion(
        SeatId(0),
        textless(
            |c| c.capacity >= 6 && c.disciplines.contains(&"POT") && c.disciplines.contains(&"PRE"),
            0,
        ),
        4,
        false,
    );
    e.state.minions[p1.index()].locked = true;
    let b = e.add_minion(SeatId(1), cap_with("", 7, 0), 7, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Inspire Greatness", "Inspire Greatness"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play(0, 0, p1));
    pick_when(&mut e, play(0, 1, p2));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, b), 7 - 4, "hand strike 1 + 1 + 2");
}

/// Inspire Greatness: never by another Methuselah's vampire, never on an
/// older acting vampire; after it, the acting vampire still plays stealth.
/// [LSJ 19990425] Only by the controller of the acting minion. [ANK
/// 20210926] Stealth and intercept can be modified afterwards.
#[test]
fn inspire_greatness_is_only_for_the_acting_methuselah() {
    let ig = |c: &zvtes::crypt::CryptCard| {
        c.disciplines.contains(&"pot") && c.disciplines.contains(&"pre")
    };
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(
            |c| (3..=4).contains(&c.capacity) && c.disciplines.contains(&"chi"),
            0,
        ),
        3,
        false,
    );
    let p = e.add_minion(
        SeatId(0),
        textless(|c| c.capacity >= 6 && ig(c), 0),
        4,
        false,
    );
    let b = e.add_minion(
        SeatId(1),
        textless(|c| c.capacity < 6 && ig(c), 0),
        4,
        false,
    );
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Inspire Greatness", "Fata Morgana"]);
    hand(&mut e, 1, &["Inspire Greatness"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, p));
    pick_when(&mut e, block(b));
    let req = offering(&mut e, play(0, 0, a));
    assert_eq!(req.seat, SeatId(0), "stealth after it");
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { actor: Some(x), .. } if *x == b),
    );

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), cap_with("", 7, 0), 3, false);
    let p = e.add_minion(
        SeatId(0),
        textless(|c| (4..=6).contains(&c.capacity) && ig(c), 0),
        4,
        false,
    );
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Inspire Greatness"]);
    pick(&mut e, bleed(a));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, play(0, 0, p));
}

/// Inspire Greatness by Herald of Topheth, an ally playing [pot] cards as a
/// vampire of capacity 5: a vampire for that card, on an acting vampire
/// younger than 5 (+1 strength, 1 life paid), never an older one; the same
/// Herald no vampire for Powerbase: Zürich's action.
/// [rules-qa 73.6] An ally playing cards "as a vampire" is an Independent
/// vampire of its granted capacity for the card it so plays only.
#[test]
fn an_ally_playing_as_a_vampire_is_one_for_that_card_only() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), cap_with("", 3, 0), 3, false);
    let herald = e.add_ally(SeatId(0), find_library("Herald of Topheth"));
    let b = e.add_minion(SeatId(1), cap_with("", 7, 0), 7, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Inspire Greatness"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play(0, 0, herald));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, b), 7 - 2, "hand strike 1 + 1");
    assert_eq!(blood(&e, herald), 4, "1 life paid");

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), cap_with("", 6, 0), 3, false);
    let herald = e.add_ally(SeatId(0), find_library("Herald of Topheth"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Inspire Greatness"]);
    pick(&mut e, bleed(a));
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        play(0, 0, herald),
    );

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), crypt_where(|c| c.capacity == 6, 0), 4, false);
    let herald = e.add_ally(SeatId(0), find_library("Herald of Topheth"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: crypt_where(|c| c.capacity == 3, 1),
            copy: Default::default(),
            counters: 0,
            banished: None,
        });
    e.state.seats[0].has_edge = true;
    let z = in_play(&mut e, 0, "Powerbase: Zürich", None);
    let by = |m| Choice::UseAbility {
        source: AbilitySource::Card(z),
        ability: 0,
        actor: Some(m),
        target: Target::Uncontrolled(0),
    };
    let req = request(&mut e);
    assert!(req.options.contains(&by(a)));
    assert!(!req.options.contains(&by(herald)), "no vampire");
}

/// Mantle of the Moon: "This action is unblockable": 4 blood paid, no
/// block offered, the bleed lands; never offered to a non-Ravnos.
#[test]
fn mantle_of_the_moon_makes_the_action_unblockable() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| c.clan == "Ravnos" && c.capacity >= 6, 0),
        5,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Mantle of the Moon"]);
    pick(&mut e, bleed(a));
    pick(&mut e, play(0, 0, a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(b));
    assert_eq!(blood(&e, a), 1);
    assert_eq!(pool(&e)[1], 9);

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Mantle of the Moon"]);
    pick(&mut e, bleed(a));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, play(0, 0, a));
}

/// Truth of a Thousand Lies: [ser] +2 bleed (limited); [SER] after a
/// successful hunt unlocks the vampire, never after a blocked bleed.
/// [ANK 20190425] Only "after resolution" cards follow it.
#[test]
fn truth_of_a_thousand_lies_bleeds_and_unlocks_after_success() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_discs(&["ser"], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Truth of a Thousand Lies"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 7);
    assert_eq!(blood(&e, a), 2);

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_discs(&["SER"], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Truth of a Thousand Lies", "Fata Morgana"]);
    pick(&mut e, action(a, ActionKind::Hunt));
    let req = offering(&mut e, play(0, 1, a));
    assert_eq!(req.window, WindowLabel::ActionAfterResolution);
    pick(&mut e, play(0, 1, a));
    let req = request(&mut e);
    assert!(!e.state.minions[a.index()].locked);
    assert_eq!(blood(&e, a), 4 + 1 - 2, "hunted, then paid 2");
    assert!(
        !req.options.contains(&play(0, 0, a)),
        "no modifier after it"
    );

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_discs(&["SER"], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Truth of a Thousand Lies"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, play(0, 1, a));
}

/// Zapaderin: two other unlocked Ravnos of the acting Methuselah, capacity
/// 6 and 7, play one each before any attempt: a capacity-5 blocker gets -2,
/// a capacity-6 one -1 (younger than the capacity-7 one only), a
/// capacity-7 one, not younger than either, nothing; a locked Ravnos and
/// another Methuselah's are never offered it.
/// [LSJ 19990425] Only by the controller of the acting minion. [ANK
/// 20200515] Copies by different minions stack. [LSJ 20020612] No block
/// attempt required.
#[test]
fn zapaderin_lowers_younger_intercept_by_each_ravnos() {
    let ravnos = |cap: u8, nth: usize| textless(|c| c.clan == "Ravnos" && c.capacity == cap, nth);
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let r6 = e.add_minion(SeatId(0), ravnos(6, 0), 4, false);
    // No textless capacity-7 Ravnos: Chavi's text is inert here (her
    // stealth on her own actions, a hand size with no library).
    let r7 = e.add_minion(SeatId(0), find_card("Chavi Oraczko"), 4, false);
    let locked = e.add_minion(SeatId(0), ravnos(5, 0), 4, false);
    e.state.minions[locked.index()].locked = true;
    let y5 = e.add_minion(SeatId(1), cap_with("", 5, 1), 4, false);
    let y6 = e.add_minion(SeatId(1), cap_with("", 6, 1), 4, false);
    let y7 = e.add_minion(SeatId(1), cap_with("", 7, 0), 4, false);
    let theirs = e.add_minion(SeatId(1), ravnos(6, 1), 4, false);
    hand(&mut e, 0, &["Zapaderin", "Zapaderin"]);
    hand(&mut e, 1, &["Zapaderin"]);
    pick(&mut e, bleed(a));
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, locked)));
    pick(&mut e, play(0, 0, r6));
    pick_when(&mut e, play(0, 0, r7));
    request(&mut e);
    let act = e.action().expect("the bleed");
    assert_eq!(act.intercept_of(&e.state, y5), -2);
    assert_eq!(act.intercept_of(&e.state, y6), -1);
    assert_eq!(act.intercept_of(&e.state, y7), 0);
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        play(0, 0, theirs),
    );
}

/// Mirror Image [CHI]: "Strike: combat ends. If this vampire was blocked
/// while performing a non-bleed action, the action continues as if
/// unblocked": a blocked hunt continues and succeeds; a blocked bleed only
/// ends the combat.
#[test]
fn mirror_image_continues_a_blocked_non_bleed_action() {
    for kind in [ActionKind::Hunt, ActionKind::Bleed] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), with_discs(&["CHI"], 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let b = e.add_minion(SeatId(1), vampire_with("qui", 0), 4, false);
        hand(&mut e, 0, &["Mirror Image"]);
        hand(&mut e, 1, &["Blood Awakening"]);
        pick(&mut e, action(a, kind));
        pick_when(&mut e, block(b));
        if kind == ActionKind::Hunt {
            // The hunt's +1 stealth met by the reaction's intercept.
            pick_when(&mut e, play(0, 0, b));
        }
        pick_when(&mut e, play(0, 1, a));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let want = match kind {
            ActionKind::Hunt => zvtes::actions::Outcome::Successful,
            _ => zvtes::actions::Outcome::Blocked,
        };
        assert_eq!(e.last_outcome, Some(want), "{kind:?}");
        assert_eq!(pool(&e)[1], 10);
    }
}

/// Mirror's Visage [chi], played as its bleed is blocked: the vampire
/// unlocked, the blocker locked, no combat, the bleed ended; that vampire
/// takes no action again this turn.
/// [RTR 20180719] [ANK 20200207] The action ends unsuccessfully at once, no
/// modifier or reaction after it.
#[test]
fn mirrors_visage_ends_the_blocked_action_and_bars_actions() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_discs(&["chi"], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Mirror's Visage", "Fata Morgana"]);
    e.notes = Some(Vec::new());
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    let req = offering(&mut e, play(0, 0, a));
    assert_eq!(req.window, WindowLabel::ActionBlocked);
    pick(&mut e, play(0, 0, a));
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(0), WindowLabel::MinionPhase)
    );
    assert!(e.action().is_none(), "the action ended");
    assert!(!e.state.minions[a.index()].locked);
    assert!(e.state.minions[b.index()].locked);
    assert!(noted_combats(&mut e).is_empty(), "no combat");
    assert_eq!(pool(&e)[1], 10);
    assert!(
        !req.options
            .iter()
            .any(|o| matches!(o, Choice::TakeAction { minion, .. } if *minion == a)),
        "{:?}",
        req.options
    );
}

/// Mirror's Visage [CHI]: "+2 stealth if this is an undirected action,
/// otherwise +1 stealth": a hunt +2 (only that mode offered), a bleed +1.
#[test]
fn mirrors_visage_superior_stealth_by_direction() {
    for (kind, mode, other, stealth) in [(ActionKind::Hunt, 1, 2, 3), (ActionKind::Bleed, 2, 1, 1)]
    {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), with_discs(&["CHI"], 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let b = e.add_minion(SeatId(1), vampire_with("qui", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Mirror's Visage"]);
        hand(&mut e, 1, &["Blood Awakening"]);
        pick(&mut e, action(a, kind));
        pick_when(&mut e, block(b));
        if kind == ActionKind::Hunt {
            pick_when(&mut e, play(0, 0, b));
        }
        let req = offering(&mut e, play(0, mode, a));
        assert!(!req.options.contains(&play(0, other, a)), "{kind:?}");
        pick(&mut e, play(0, mode, a));
        request(&mut e);
        assert_eq!(
            e.action().expect("action").stealth(&e.state),
            stealth,
            "{kind:?}"
        );
    }
}

/// Heart's Desire: during a younger vampire's attempt the attempt fails
/// for good and the bleed gets +1; during an older one's, +1 only, the
/// attempt standing; at superior before any attempt, +2.
#[test]
fn hearts_desire_fails_a_younger_blockers_attempt_and_bleeds() {
    let both = |c: &zvtes::crypt::CryptCard, sup: bool| {
        ["aus", "chi"].iter().all(|d| {
            c.disciplines
                .iter()
                .any(|x| x.eq_ignore_ascii_case(d) && (!sup || x.to_uppercase() == *x))
        })
    };
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| (5..=6).contains(&c.capacity) && both(c, false), 0),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let young = e.add_minion(SeatId(1), cap_with("", 4, 0), 4, false);
    e.add_minion(SeatId(1), cap_with("", 4, 1), 4, false);
    hand(&mut e, 0, &["Heart's Desire"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(young));
    pick_when(&mut e, play(0, 0, a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(young));
    assert_eq!(pool(&e)[1], 8);

    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| (5..=6).contains(&c.capacity) && both(c, false), 0),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let old = e.add_minion(SeatId(1), cap_with("", 7, 0), 4, false);
    e.add_minion(SeatId(1), cap_with("", 4, 1), 4, false);
    hand(&mut e, 0, &["Heart's Desire"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(old));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        e.last_outcome,
        Some(zvtes::actions::Outcome::Blocked),
        "the older vampire's attempt stood"
    );
    assert_eq!(pool(&e)[1], 10);
    assert_eq!(blood(&e, a), 4 - 1 - 1, "the card, the hand strike");

    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| c.capacity >= 4 && both(c, true), 0),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Heart's Desire"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 7);
}

/// Divine Image [ser], as its bleed is announced: -1 stealth, +1 strength
/// (the hand strike 2), 1 damage prevented in the block's combat.
#[test]
fn divine_image_strengthens_and_protects_in_the_block_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_discs(&["ser"], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Divine Image"]);
    pick(&mut e, bleed(a));
    expect(&mut e, SeatId(0), WindowLabel::ActionAsAnnounced);
    pick(&mut e, play(0, 0, a));
    request(&mut e);
    assert_eq!(e.action().expect("bleed").stealth(&e.state), -1);
    pick_when(&mut e, block(b));
    pick_when(&mut e, Choice::UsePrevent);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, b), 4 - 2, "strength 1 + 1");
    assert_eq!(blood(&e, a), 4 - 1, "its cost; the strike prevented");
}

/// Divine Image [SER]: +1 bleed not counting against the limit: Truth of a
/// Thousand Lies' limited +2 still played after it (1 + 1 + 2).
/// [LSJ 20100218] [RTR 20180511-2] A limited bleed modifier can be played
/// before or after.
#[test]
fn divine_image_superior_bleed_does_not_count_against_the_limit() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_discs(&["SER"], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Divine Image", "Truth of a Thousand Lies"]);
    pick(&mut e, bleed(a));
    pick(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 6);
    assert_eq!(blood(&e, a), 4 - 3);
}

/// The card `name` in play on `host` (the first): its id.
#[track_caller]
fn card_on(e: &Engine, name: &str, host: MinionId) -> CardId {
    let card = find_library(name);
    let i = e
        .state
        .cards
        .iter()
        .position(|c| c.card == card && c.present() && c.host == Some(host))
        .unwrap_or_else(|| panic!("{name} not on {host:?}"));
    CardId(i as u32)
}

/// Forger's Hammer [pot], played once the equip's blocks are declined: the
/// card itself on the weapon, out of the ash heap, +1 damage to its melee
/// strike; [POT] pays the weapon's 2 pool less 1.
/// [RTR 19970630] [TOM 19960225] The damage the strike's own kind (the
/// weapon's layer, as Blood Tempering's).
#[test]
fn forgers_hammer_goes_on_the_equipped_weapon() {
    for (disc, mode, paid) in [("pot", 0, 2), ("POT", 1, 1)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), with_discs(&[disc], 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Bang Nakh — Tiger's Claws", "Forger's Hammer"]);
        pick(&mut e, play(0, 0, a));
        let req = offering(&mut e, play(0, mode, a));
        assert_eq!(req.window, WindowLabel::ActionBlocksDeclined);
        pick(&mut e, play(0, mode, a));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let w = card_on(&e, "Bang Nakh — Tiger's Claws", a);
        let h = placed_on(&e, "Forger's Hammer", Some(a));
        assert_eq!(e.state.cards[h.index()].on_card, Some(w));
        assert_eq!(e.state.melee_damage_bonus(a, Some(w)), 1);
        assert_eq!(e.state.melee_damage_bonus(a, None), 0, "the weapon's only");
        assert_eq!(pool(&e)[0], 10 - paid, "{disc}");
    }
}

/// Forger's Hammer: never for a unique melee weapon, nor for an equip from
/// another minion ("from your hand").
#[test]
fn forgers_hammer_needs_a_non_unique_melee_weapon_from_hand() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_discs(&["pot"], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Sengir Dagger", "Forger's Hammer"]);
    pick(&mut e, play(0, 0, a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
    card_on(&e, "Sengir Dagger", a);

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_discs(&["pot"], 0), 4, false);
    let giver = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 0, "Bang Nakh — Tiger's Claws", Some(giver));
    hand(&mut e, 0, &["Forger's Hammer"]);
    pick(&mut e, take_from(a, giver));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
    card_on(&e, "Bang Nakh — Tiger's Claws", a);
}

/// False Resonance [chi]: Telepathic Misdirection [aus] played for +1
/// intercept against Fata Morgana's +1 stealth is canceled as played, its
/// blood not paid; still played, it is not offered again; the attempt
/// fails, the bleed lands.
/// [ANK 20190104] [LSJ 19980212] The canceled card has still been played:
/// the same reaction cannot be played again by the same minion.
#[test]
fn false_resonance_cancels_an_auspex_reaction_unpaid() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_discs(&["chi"], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), vampire_with("aus", 0), 4, false);
    hand(&mut e, 0, &["Fata Morgana", "False Resonance"]);
    hand(
        &mut e,
        1,
        &["Telepathic Misdirection", "Telepathic Misdirection"],
    );
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, b));
    let req = offering(&mut e, play(0, 0, a));
    assert_eq!(req.window, WindowLabel::CardAsPlayed);
    pick(&mut e, play(0, 0, a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, b));
    assert_eq!(blood(&e, b), 4, "its cost not paid");
    assert!(ash_has(&e, 1, "Telepathic Misdirection"));
    assert_eq!(pool(&e)[1], 9, "the attempt failed");
    assert_eq!(blood(&e, a), 4 - 2);
}

/// False Resonance [CHI]: +1 stealth and the card on its vampire for the
/// action; Telepathic Misdirection played before it stands; Eagle's Sight
/// after it is canceled for 1 of the vampire's blood, once: Enhanced Senses
/// next is not; the card burned as the action ends.
/// [ANK 20180201] [CHI] cannot get stealth then cancel the [aus] card that
/// provided intercept. [ANK 20210226] The burned blood is an effect, not
/// reduced.
#[test]
fn false_resonance_superior_cancels_once_for_blood() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_discs(&["CHI"], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), vampire_with("aus", 0), 4, false);
    hand(&mut e, 0, &["Fata Morgana", "False Resonance"]);
    hand(
        &mut e,
        1,
        &[
            "Telepathic Misdirection",
            "Eagle's Sight",
            "Enhanced Senses",
        ],
    );
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, b));
    pick_when(&mut e, play(0, 1, a));
    request(&mut e);
    let fr = card_on(&e, "False Resonance", a);
    assert!(!e.state.cards[fr.index()].effect_only);
    assert!(!ash_has(&e, 0, "False Resonance"));
    let act = e.action().expect("the bleed");
    assert_eq!(act.intercept_of(&e.state, b), 1, "resolved, not canceled");
    assert_eq!(act.stealth(&e.state), 2);
    pick_when(&mut e, play(0, 0, b));
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::CardAsPlayed);
    let cancel = uses_of(&req, fr);
    assert_eq!(cancel.len(), 1, "{:?}", req.options);
    e.choose(index_of(&req, cancel[0]));
    request(&mut e);
    assert!(ash_has(&e, 1, "Eagle's Sight"));
    assert_eq!(blood(&e, a), 4 - 1 - 1 - 1, "two cards and the burn");
    pick_when(&mut e, play(0, 0, b));
    let req = request(&mut e);
    assert!(uses_of(&req, fr).is_empty(), "once this action");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "False Resonance"));
    assert!(!e.state.cards[fr.index()].present());
}

/// Seat 0's `a` plays Whispers from the Dead [nec] naming Conceal, then
/// Shroudsight in `mode` as it is announced; seat 1's `b` blocks if given.
fn shroudsight_whispers(nec: &str, mode: u8, blocked: bool) -> (Engine, MinionId) {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with(nec, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Whispers from the Dead", "Shroudsight"]);
    fill_hand(&mut e, 0);
    let conceal = find_library("Conceal");
    e.state.seats[0].ash = vec![conceal].into();
    e.state.seats[0].library = vec![find_library("Form of Mist"); 5].into();
    // Conceal, the one card to name, elides.
    pick(&mut e, play(0, 0, a));
    let req = expect(&mut e, SeatId(0), WindowLabel::ActionAsAnnounced);
    let shroud = req
        .options
        .iter()
        .copied()
        .find(|o| matches!(o, Choice::Play { mode: m, .. } if *m == mode))
        .expect("Shroudsight offered");
    e.choose(index_of(&req, shroud));
    if blocked {
        pick_when(&mut e, block(b));
    }
    (e, a)
}

fn ash_to_top(card: &str) -> Choice {
    Choice::TakeFrom {
        zone: zvtes::decision::Zone::AshHeap,
        card: find_library(card),
    }
}

/// Shroudsight [nec]: after resolution the Whispers from the Dead used for
/// the action goes to the library's top instead of the ash heap; passed, it
/// reaches the ash heap; blocked, offered after the combat (rules-qa 55.5,
/// 23.3; owner, 2026-11-14).
#[test]
fn shroudsight_returns_the_action_card_to_the_library_top() {
    let whispers = find_library("Whispers from the Dead");
    let (mut e, _) = shroudsight_whispers("nec", 0, false);
    let req = offering(&mut e, ash_to_top("Whispers from the Dead"));
    assert_eq!(req.window, WindowLabel::ActionAfterResolution);
    assert_eq!(req.seat, SeatId(0), "\"you\": the acting Methuselah");
    assert!(
        !req.options.contains(&ash_to_top("Shroudsight")),
        "[nec]: the action card only"
    );
    e.choose(index_of(&req, ash_to_top("Whispers from the Dead")));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].library.last(), Some(&whispers));
    assert!(!ash_has(&e, 0, "Whispers from the Dead"));
    assert!(ash_has(&e, 0, "Shroudsight"));

    let (mut e, _) = shroudsight_whispers("nec", 0, false);
    let req = offering(&mut e, ash_to_top("Whispers from the Dead"));
    e.choose(index_of(&req, Choice::Pass));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "Whispers from the Dead"));

    let (mut e, _) = shroudsight_whispers("nec", 0, true);
    let req = offering(&mut e, ash_to_top("Whispers from the Dead"));
    assert_eq!(req.window, WindowLabel::ActionAfterResolution);
    e.choose(index_of(&req, ash_to_top("Whispers from the Dead")));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
    assert_eq!(e.state.seats[0].library.last(), Some(&whispers));
    assert!(!ash_has(&e, 0, "Whispers from the Dead"));
}

/// Shroudsight [NEC]: "any card played during this action": the action card
/// or Shroudsight itself, one of them; the other stays where it goes.
#[test]
fn shroudsight_superior_returns_any_card_played_this_action() {
    let shroud = find_library("Shroudsight");
    let (mut e, _) = shroudsight_whispers("NEC", 1, false);
    let req = offering(&mut e, ash_to_top("Shroudsight"));
    assert!(req.options.contains(&ash_to_top("Whispers from the Dead")));
    assert!(
        !req.options.contains(&ash_to_top("Conceal")),
        "not played this action"
    );
    e.choose(index_of(&req, ash_to_top("Shroudsight")));
    let req = request(&mut e);
    assert!(
        !req.options
            .iter()
            .any(|o| matches!(o, Choice::TakeFrom { .. })),
        "one move"
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].library.last(), Some(&shroud));
    assert!(ash_has(&e, 0, "Whispers from the Dead"));
}

/// Shroudsight: only as an action requiring Necromancy or Giovanni is
/// announced: a Giovanni's rulebook bleed never offers it (rules-qa 41.12).
#[test]
fn shroudsight_needs_an_action_requiring_necromancy_or_giovanni() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), giovanni(0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Shroudsight"]);
    pick(&mut e, bleed(a));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, play(0, 0, a));
}

/// Will-o'-the-Wisp [chi], before any attempt: X = 1, two points on the
/// prey's vampire: -2 intercept, its Eagle's Sight cannot reach the bleed;
/// 1 blood paid. X = 0: one point, nothing paid.
/// [LSJ 20030502] X can be zero, not less. [LSJ 20020612] Any time before
/// resolution; no block attempt required.
#[test]
fn will_o_the_wisp_spreads_intercept_points() {
    for points in [2u8, 1] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), with_discs(&["chi"], 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let b = e.add_minion(SeatId(1), vampire_with("aus", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Will-o'-the-Wisp"]);
        hand(&mut e, 1, &["Eagle's Sight"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, play(0, 0, a));
        for _ in 0..points {
            let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
            e.choose(index_of(&req, Choice::ChooseMinion(b)));
        }
        let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
        e.choose(index_of(&req, Choice::Pass));
        request(&mut e);
        let act = e.action().expect("the bleed");
        assert_eq!(act.intercept_of(&e.state, b), -(points as i32));
        if points == 2 {
            never_reaching_until(&mut e, SeatId(0), WindowLabel::MinionPhase, b);
            assert_eq!(pool(&e)[1], 9);
        } else {
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        }
        assert_eq!(blood(&e, a), 4 - (points - 1), "X = points - 1");
    }
}

/// Will-o'-the-Wisp [chi]: the prey's vampire under Secure Haven gets no
/// point, as [CHI] cannot choose it; the other may.
/// [rules-qa 35.1] [LSJ 20031010] Choosing is targeting.
#[test]
fn will_o_the_wisp_gives_no_point_to_a_secure_haven_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_discs(&["chi"], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let safe = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    in_play(&mut e, 1, "Secure Haven", Some(safe));
    hand(&mut e, 0, &["Will-o'-the-Wisp"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, a));
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    assert!(!req.options.contains(&Choice::ChooseMinion(safe)));
    assert!(req.options.contains(&Choice::ChooseMinion(b)));
}

/// Will-o'-the-Wisp [CHI], as the bleed is announced: X = 3, two younger
/// vampires of the prey chosen (an older one not offered): neither blocks
/// nor plays Telepathic Misdirection once blocks are declined; the older one
/// may still block.
/// [ANK 20190425] Played as announced, before regular modifiers. [ANK
/// 20181208] The action card replaced before it.
#[test]
fn will_o_the_wisp_superior_bars_chosen_minions() {
    for played in [false, true] {
        let mut e = game(3);
        let a = e.add_minion(
            SeatId(0),
            textless(|c| c.capacity == 6 && c.disciplines.contains(&"CHI"), 0),
            4,
            false,
        );
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        // Superior Auspex: Telepathic Misdirection's redirect.
        let y1 = e.add_minion(
            SeatId(1),
            textless(|c| c.capacity <= 5 && c.disciplines.contains(&"AUS"), 0),
            4,
            false,
        );
        let y2 = e.add_minion(
            SeatId(1),
            textless(|c| c.capacity <= 5 && c.disciplines.contains(&"AUS"), 1),
            4,
            false,
        );
        let old = e.add_minion(SeatId(1), cap_with("", 7, 0), 4, false);
        e.add_minion(SeatId(2), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Will-o'-the-Wisp"]);
        hand(&mut e, 1, &["Telepathic Misdirection"]);
        pick(&mut e, bleed(a));
        let redirect = Choice::Play {
            card: 0,
            mode: 1,
            actor: Some(y1),
            target: Target::Seat(SeatId(2)),
        };
        if !played {
            // Unbarred, y1 may redirect once blocks are declined.
            let req = offering(&mut e, redirect);
            assert_eq!(req.window, WindowLabel::ActionBlocksDeclined);
            continue;
        }
        expect(&mut e, SeatId(0), WindowLabel::ActionAsAnnounced);
        pick(&mut e, play(0, 1, a));
        let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
        assert!(!req.options.contains(&Choice::ChooseMinion(old)));
        e.choose(index_of(&req, Choice::ChooseMinion(y1)));
        pick(&mut e, Choice::ChooseMinion(y2));
        let req = request(&mut e);
        if req.window == WindowLabel::PlayTerms {
            e.choose(index_of(&req, Choice::Pass));
        }
        let req = until(&mut e, SeatId(1), WindowLabel::ActionBlock);
        assert!(offers_block(&req, old));
        assert!(!offers_block(&req, y1) && !offers_block(&req, y2));
        never_offered_where(
            &mut e,
            SeatId(0),
            WindowLabel::MinionPhase,
            |o| matches!(o, Choice::Play { actor: Some(m), .. } | Choice::DeclareBlock { minion: m } if *m == y1 || *m == y2),
        );
        assert_eq!(blood(&e, a), 4 - 3, "X = 3");
        assert_eq!(pool(&e)[1], 9);
    }
}

// ---- Minions and equipment (minions_equip) ----------------------------------

/// A bleeds, X blocks: their combat begins.
fn bleed_blocked(e: &mut Engine, a: MinionId, x: MinionId) {
    pick(e, bleed(a));
    pick_when(e, block(x));
}

/// The card `name` on `host`, its owner's, with its first mode's layers.
fn borne(e: &mut Engine, name: &str, host: MinionId) -> CardId {
    let seat = e.state.minions[host.index()].owner.0;
    in_play(e, seat, name, Some(host))
}

/// The ability `ability` of the card `card`, by its bearer, no target.
fn card_use(card: CardId, ability: u8) -> Choice {
    use_on(card, ability, None, Target::None)
}

/// Kpist m/45: "1R damage each strike … When bearer strikes with this gun,
/// he or she gets an optional additional strike (limited) this round, only
/// usable to strike with this gun." After its strike, X is offered Pass or
/// the Kpist only; A takes 1 + 1.
#[test]
fn kpist_m45_strikes_1r_with_one_additional_strike_only_with_itself() {
    let (mut e, a, x) = duel(&[], &[]);
    let kpist = borne(&mut e, "Kpist m/45", x);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, card_use(kpist, 0));
    let req = offering(&mut e, card_use(kpist, 0));
    assert_eq!(req.seat, SeatId(1));
    assert_eq!(req.options, vec![Choice::Pass, card_use(kpist, 0)]);
    pick(&mut e, card_use(kpist, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, x)), (4 - 2, 4 - 1));
}

/// Kpist m/45's "Weapon. Gun." (a period) is a gun: Kevlar Vest's gun
/// branch is offered against its strike.
/// [rules-qa 116.10] The type line names the gun.
#[test]
fn kpist_m45_is_a_gun_for_kevlar_vest() {
    assert!(zvtes::cards::is_gun(find_library("Kpist m/45")));
    let (mut e, a, x) = duel(&[], &[]);
    let kpist = borne(&mut e, "Kpist m/45", a);
    let vest = borne(&mut e, "Kevlar Vest", x);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, card_use(kpist, 0));
    let req = offering(&mut e, card_use(vest, 0));
    assert_eq!(req.seat, SeatId(1));
}

/// Talith: "A Ravnos can use this weapon to strike: burn weapon with first
/// strike." The burn is offered at the blocker's .44 Magnum and lands
/// before the Magnum strikes: the Magnum in its owner's ash heap, A
/// unharmed. Against an unarmed blocker it is never offered, the
/// strength+1 strike is.
/// [RTR 19980928] Not chosen with no weapon to destroy.
#[test]
fn talith_burns_the_opposing_weapon_with_first_strike() {
    let (mut e, _, x) = duel(&[], &[]);
    let a = e.add_minion(SeatId(0), of_clan("Ravnos", "", 0), 4, false);
    let talith = borne(&mut e, "Talith", a);
    let magnum = borne(&mut e, ".44 Magnum", x);
    bleed_blocked(&mut e, a, x);
    let burn = use_on(talith, 1, None, Target::Card(magnum));
    let req = offering(&mut e, burn);
    assert!(req.options.contains(&card_use(talith, 0)));
    e.choose(index_of(&req, burn));
    if let Some(i) = request(&mut e)
        .options
        .iter()
        .position(|o| *o == card_use(magnum, 0))
    {
        e.choose(i);
    }
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 1, ".44 Magnum"));
    assert!(!e.state.cards[magnum.index()].in_play);
    assert_eq!(blood(&e, a), 4, "the Magnum burned before it struck");

    let (mut e, _, x) = duel(&[], &[]);
    let a = e.add_minion(SeatId(0), of_clan("Ravnos", "", 0), 4, false);
    let talith = borne(&mut e, "Talith", a);
    bleed_blocked(&mut e, a, x);
    let req = offering(&mut e, card_use(talith, 0));
    assert!(!req
        .options
        .iter()
        .any(|o| matches!(o, Choice::UseAbility { ability: 1, .. })));
}

/// Meat Hook torn away by Potence: its burn strike resolves first, so the
/// bearer's strike with the Hook does nothing; the tearer pays its blood
/// and takes no damage. (Its effects once listed first strike before the
/// strike, which a new strike resets.)
#[test]
fn meat_hook_torn_away_with_first_strike_never_strikes() {
    let (mut e, a, _) = duel(&[], &[]);
    let z = e.add_minion(SeatId(1), vampire_with("pot", 0), 4, false);
    let hook = borne(&mut e, "Meat Hook", a);
    bleed_blocked(&mut e, a, z);
    pick_when(&mut e, card_use(hook, 0));
    pick_when(&mut e, use_on(hook, 1, Some(z), Target::Card(hook)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "Meat Hook"));
    assert_eq!(blood(&e, z), 4 - 1, "its cost only");
}

/// Talith taken by a non-Ravnos: its strength+1 strike only.
/// [ANK 20180719-3] Clan equipment taken from its minion by another.
#[test]
fn talith_taken_by_a_non_ravnos_strikes_but_never_burns_a_weapon() {
    let (mut e, a, x) = duel(&[], &[]);
    let card = e.state.minions[a.index()].crypt_card().expect("a vampire");
    assert_ne!(CRYPT[card as usize].clan, "Ravnos");
    let talith = borne(&mut e, "Talith", a);
    let magnum = borne(&mut e, ".44 Magnum", x);
    bleed_blocked(&mut e, a, x);
    let req = offering(&mut e, card_use(talith, 0));
    assert!(!req
        .options
        .contains(&use_on(talith, 1, None, Target::Card(magnum))));
    e.choose(index_of(&req, card_use(talith, 0)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4 - 2, "strength+1");
}

/// The choice of `card`'s ability `ability` in `req` whose target matches.
fn use_of(
    req: &zvtes::decision::DecisionRequest,
    card: CardId,
    ability: u8,
    target: Target,
) -> Option<Choice> {
    req.options.iter().copied().find(|o| {
        matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), ability: a, target: t, .. }
            if *c == card && *a == ability && *t == target)
    })
}

/// Pass until `card`'s ability `ability` at `target` is offered, and use it.
#[track_caller]
fn use_when(e: &mut Engine, card: CardId, ability: u8, target: Target) {
    for _ in 0..200 {
        let req = request(e);
        if let Some(c) = use_of(&req, card, ability, target) {
            e.choose(index_of(&req, c));
            return;
        }
        e.choose(fallback(&req));
    }
    panic!("ability {ability} of {card:?} never offered");
}

/// Is `card`'s ability `ability` offered in `o`, whatever its target?
fn is_use(o: &Choice, card: CardId, ability: u8) -> bool {
    matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), ability: a, .. }
        if *c == card && *a == ability)
}

/// An ally's card in play.
fn ally_card_of(e: &Engine, m: MinionId) -> CardId {
    e.state.minions[m.index()].ally_id().expect("an ally")
}

/// Omael Kuman: "The employer can burn 1 blood to set the range for the
/// round before range is determined during the first round of combat."
/// Both ranges offered before range; long chosen: a blood burned, the round
/// at long range (no maneuver asked, no hand strike lands).
/// [RTR 19970630] [ANK 20180720] Set, the Determine Range step skipped.
#[test]
fn omael_kuman_sets_the_first_rounds_range_for_a_blood() {
    let (mut e, a, x) = duel(&[], &[]);
    let omael = borne(&mut e, "Omael Kuman", a);
    bleed_blocked(&mut e, a, x);
    let req = request(&mut e);
    assert!(use_of(&req, omael, 0, Target::None).is_some());
    assert!(use_of(&req, omael, 1, Target::None).is_some());
    e.choose(index_of(&req, card_use(omael, 1)));
    let req = request(&mut e);
    assert_ne!(req.window, WindowLabel::CombatDetermineRange);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, x)), (4 - 1, 4), "long range");
}

/// Omael Kuman yields to a range another effect set first: the blocker's
/// Cold Aura sets long range, Omael is offered no more.
/// [PIB 20120214] The effect that set the range first has priority.
#[test]
fn omael_kuman_yields_to_a_range_already_set() {
    let (mut e, a, _) = duel(&[], &[]);
    let x = e.add_minion(SeatId(1), vampire_with("nec", 0), 4, false);
    let omael = borne(&mut e, "Omael Kuman", a);
    hand(&mut e, 1, &["Cold Aura"]);
    bleed_blocked(&mut e, a, x);
    let req = request(&mut e);
    assert!(
        use_of(&req, omael, 0, Target::None).is_some(),
        "offered first"
    );
    e.choose(fallback(&req));
    pick_when(&mut e, play(0, 0, x));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        is_use(o, omael, 0) || is_use(o, omael, 1)
    });
    assert_eq!(blood(&e, a), 4);
}

/// Shambling Hordes: "After this ally enters play, remove an ally or
/// vampire in your ash heap from the game or burn this ally. This ally gets
/// +1 strength for each life it has." The entry offers Pass, the ash ally
/// and the ash vampire; the vampire removed, the Hordes stays with 3
/// strength. Declined while a card was there: burned.
/// [LSJ 20030701-2] Its controller can choose not to remove a minion, and
/// burn the Hordes.
#[test]
fn shambling_hordes_removes_an_ash_minion_or_burns() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let vamp = plain("Brujah", 1);
    e.state.seats[0].ash = vec![find_library("Political Ally"), find_library("Majesty")].into();
    e.state.seats[0].ash_crypt = vec![vamp].into();
    let h = e.add_ally(SeatId(0), find_library("Shambling Hordes"));
    let req = expect(&mut e, SeatId(0), WindowLabel::EntryEffect);
    assert_eq!(
        req.options,
        vec![
            Choice::Pass,
            Choice::FindCard(find_library("Political Ally")),
            Choice::AshVampire(vamp),
        ]
    );
    pick(&mut e, Choice::AshVampire(vamp));
    request(&mut e);
    assert!(e.state.minions[h.index()].in_play);
    assert_eq!(e.state.seats[0].removed_crypt, vec![vamp]);
    assert_eq!(e.state.base_strength(h), 3);

    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0].ash = vec![find_library("Political Ally")].into();
    let h = e.add_ally(SeatId(0), find_library("Shambling Hordes"));
    pick(&mut e, Choice::Pass);
    request(&mut e);
    assert!(!e.state.minions[h.index()].in_play);
    assert!(ash_has(&e, 0, "Shambling Hordes"));
    assert!(ash_has(&e, 0, "Political Ally"), "nothing removed");
}

/// Seat 0's Shambling Hordes in play (its entry paid with an ash vampire),
/// in `mode`, rushing X: the game, the Hordes, X.
fn hordes_rush(x_card: u16, mode: u8) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let x = e.add_minion(SeatId(1), x_card, 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0].ash_crypt = vec![plain("Brujah", 1)].into();
    let h = e.add_ally_version(SeatId(0), find_library("Shambling Hordes"), mode);
    pick(&mut e, Choice::AshVampire(plain("Brujah", 1)));
    let card = ally_card_of(&e, h);
    use_when(&mut e, card, 0, Target::Minion(x));
    (e, h, x)
}

/// Shambling Hordes' strike is read as strike resolution begins: X's
/// simultaneous Theft of Vitae (steal 1 life) leaves its 3 damage whole;
/// Elimelech's first strike (2 damage) lands first, leaving it 1.
/// [LSJ 20030307] [LSJ 20100119] Damage set as strike resolution begins:
/// a simultaneous steal does not lower it, first strike damage does.
#[test]
fn shambling_hordes_strike_reads_its_life_as_strikes_resolve() {
    let (mut e, h, x) = hordes_rush(vampire_with("tha", 0), 0);
    hand(&mut e, 1, &["Theft of Vitae"]);
    pick_when(&mut e, play(0, 0, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, h), 3 - 1, "stolen");
    assert_eq!(blood(&e, x), 4 - 3 + 1, "3 damage, 1 stolen");

    let elimelech = zvtes::state::find_card("Elimelech the Twice-Damned");
    let (mut e, h, x) = hordes_rush(elimelech, 0);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, h), 3 - 2);
    assert_eq!(blood(&e, x), 4 - 1, "struck for its 1 life left");
}

/// Shambling Hordes against Dam the Heart's River [QUI]: its strike costs a
/// life first, so it strikes for 2.
/// [LSJ 20090703] A strike is paid at declaration; damage depending on the
/// striker's life is reckoned after it is paid.
#[test]
fn shambling_hordes_pays_a_strike_toll_before_its_strength() {
    let (mut e, h, x) = hordes_rush(vampire_with("QUI", 0), 0);
    hand(&mut e, 1, &["Dam the Heart's River"]);
    pick_when(&mut e, play(0, 1, x));
    let req = offering(&mut e, Choice::HandStrike);
    if req.seat == SeatId(1) {
        e.choose(index_of(&req, Choice::Pass));
        pick_when(&mut e, Choice::HandStrike);
    } else {
        e.choose(index_of(&req, Choice::HandStrike));
    }
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4 - 2, "2 life left as it struck");
    assert!(
        !e.state.minions[h.index()].in_play,
        "its toll and X's 2 damage burn it"
    );
}

/// Shambling Hordes burned from play and raised by Pressing Flesh: its
/// entry asks again, in its new controller's ash heap.
/// [ANK 20170309-2] The enter-play effect however it enters play.
#[test]
fn pressing_flesh_raises_shambling_hordes_with_its_entry_again() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("thn", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let vamp = plain("Brujah", 1);
    e.state.seats[0].ash_crypt = vec![vamp].into();
    hand(&mut e, 0, &["Pressing Flesh"]);
    // Seat 1's Hordes, nothing in its ash heap: burned from play at once.
    let h = e.add_ally(SeatId(1), find_library("Shambling Hordes"));
    request(&mut e);
    assert!(!e.state.minions[h.index()].in_play);
    assert!(ash_has(&e, 1, "Shambling Hordes"));
    pick(&mut e, play(0, 0, a));
    let req = until(&mut e, SeatId(0), WindowLabel::EntryEffect);
    assert_eq!(req.options, vec![Choice::Pass, Choice::AshVampire(vamp)]);
    pick(&mut e, Choice::AshVampire(vamp));
    request(&mut e);
    assert_eq!(e.state.seats[0].removed_crypt, vec![vamp]);
}

/// Shambling Hordes [NEC]: "this ally has 4 life": 4 life, 4 strength.
#[test]
fn superior_shambling_hordes_has_four_life_and_strength() {
    let (mut e, h, x) = hordes_rush(plain("Ventrue", 1), 1);
    assert_eq!((blood(&e, h), e.state.base_strength(h)), (4, 4));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4 - 4);
}

/// Nephren-Ka plays cards requiring basic Necromancy as a vampire: in his
/// rush, Spiritual Intervention's [nec] dodge, never its [NEC] mode; in his
/// bleed, Trochomancy's [nec] mode, never its [aus][nec] one.
/// [ANK 20180111] Basic only, no superior. [LSJ 20011217] [LSJ 20090508]
/// One requirement of a multi-Discipline card, not more.
#[test]
fn nephren_ka_plays_basic_necromancy_as_a_vampire_and_no_superior() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let nk = e.add_ally(SeatId(0), find_library("Nephren-Ka"));
    hand(&mut e, 0, &["Spiritual Intervention"]);
    let card = ally_card_of(&e, nk);
    use_when(&mut e, card, 1, Target::Minion(x));
    let req = offering(&mut e, play(0, 0, nk));
    assert!(!req.options.contains(&play(0, 1, nk)));

    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let nk = e.add_ally(SeatId(0), find_library("Nephren-Ka"));
    e.state.seats[1].ash = vec![find_library("Majesty"); 13].into();
    hand(&mut e, 0, &["Trochomancy"]);
    pick(&mut e, bleed(nk));
    let req = offering(&mut e, play(0, 0, nk));
    assert!(!req.options.contains(&play(0, 1, nk)));
}

/// Nephren-Ka: "can prevent 1 non-aggravated damage each combat." X's 2
/// normal damage: prevented once, he loses 1 life. Against a Sengir
/// Dagger's aggravated damage: never offered.
/// [LSJ 20040812-2] [PIB 20130327] Never aggravated damage.
#[test]
fn nephren_ka_prevents_one_normal_damage_each_combat_never_aggravated() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[x.index()].timed.push((
        zvtes::cards::Lasting::Strength(1),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(1),
    ));
    let nk = e.add_ally(SeatId(0), find_library("Nephren-Ka"));
    let card = ally_card_of(&e, nk);
    use_when(&mut e, card, 1, Target::Minion(x));
    use_when(&mut e, card, 0, Target::None);
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        is_use(o, card, 0)
    });
    assert_eq!(blood(&e, nk), 5 - 1);

    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let dagger = borne(&mut e, "Sengir Dagger", x);
    let nk = e.add_ally(SeatId(0), find_library("Nephren-Ka"));
    let card = ally_card_of(&e, nk);
    use_when(&mut e, card, 1, Target::Minion(x));
    pick_when(&mut e, card_use(dagger, 0));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        is_use(o, card, 0)
    });
    assert_eq!(blood(&e, nk), 5 - 1, "the aggravated point lands");
}

/// Nephren-Ka's rush: at another Methuselah's ready minion, never his own
/// Methuselah's; unblocked, they fight.
/// [rules-qa 17.3] A rush at another Methuselah's minion.
#[test]
fn nephren_ka_rushes_another_methuselahs_minion() {
    let mut e = game(2);
    let own = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let nk = e.add_ally(SeatId(0), find_library("Nephren-Ka"));
    let card = ally_card_of(&e, nk);
    let req = request(&mut e);
    assert!(use_of(&req, card, 1, Target::Minion(own)).is_none());
    e.choose(index_of(
        &req,
        use_of(&req, card, 1, Target::Minion(x)).expect("at X"),
    ));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4 - 2, "his 2 strength");
}

/// Nephren-Ka and Tutu the Doubly Evil One burned by Set's Curse: each
/// shuffled into its owner's library, out of the ash heap, and the Curse
/// still enters play as an ally.
/// [ANK 20200813] Shuffled after it is burned: other effects of its burn
/// still apply.
#[test]
fn nephren_ka_and_tutu_shuffle_into_the_library_when_burned() {
    for name in ["Nephren-Ka", "Tutu the Doubly Evil One"] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("ser", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let m = e.add_ally(SeatId(1), find_library(name));
        fill_hand(&mut e, 1);
        let library = e.state.seats[1].library.len();
        hand(&mut e, 0, &["Set's Curse"]);
        pick(&mut e, play_on(0, 0, a, Target::Minion(m)));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(!e.state.minions[m.index()].in_play, "{name}");
        assert!(e.state.seats[1].library.contains(&find_library(name)));
        assert_eq!(e.state.seats[1].library.len(), library + 1);
        assert!(!ash_has(&e, 1, name));
        assert!(curse_ally(&e).is_some(), "{name}: the Curse in play");
    }
}

/// Tutu: "During your minion phase, Tutu can unlock. Tutu can steal an
/// equipment from a vampire in torpor as a Ⓓ action." Offered at the prey's
/// torpid vampire's Magnum, not at his own torpid vampire's Vest; the
/// Magnum moves to Tutu. Locked by his action, he unlocks once that minion
/// phase, never twice.
#[test]
fn tutu_steals_equipment_from_a_torpid_vampire_and_unlocks_once() {
    let mut e = game(2);
    let own = e.add_minion(SeatId(0), plain("Brujah", 0), 2, true);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 2, true);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let vest = borne(&mut e, "Kevlar Vest", own);
    let magnum = borne(&mut e, ".44 Magnum", x);
    let tutu = e.add_ally(SeatId(0), find_library("Tutu the Doubly Evil One"));
    let card = ally_card_of(&e, tutu);
    let req = request(&mut e);
    assert!(use_of(&req, card, 2, Target::Card(vest)).is_none());
    assert!(use_of(&req, card, 1, Target::None).is_none(), "unlocked");
    e.choose(index_of(
        &req,
        use_of(&req, card, 2, Target::Card(magnum)).expect("at the Magnum"),
    ));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[magnum.index()].host, Some(tutu));
    assert!(e.state.minions[tutu.index()].locked);
    e.choose(index_of(
        &req,
        use_of(&req, card, 1, Target::None).expect("the unlock"),
    ));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[tutu.index()].locked);
    e.choose(index_of(&req, bleed(tutu)));
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
        is_use(o, card, 1)
    });
    assert!(e.state.minions[tutu.index()].locked);
}

/// Tutu: "Once each combat, Tutu can strike: dodge." Offered in the first
/// round; after a press, not in the second.
#[test]
fn tutu_dodges_once_each_combat() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let x = e.add_minion(SeatId(1), vampire_with("cel", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let tutu = e.add_ally(SeatId(0), find_library("Tutu the Doubly Evil One"));
    let card = ally_card_of(&e, tutu);
    hand(&mut e, 1, &["Psyche!"]);
    pick(&mut e, bleed(tutu));
    pick_when(&mut e, block(x));
    use_when(&mut e, card, 0, Target::None);
    pick_when(&mut e, play(0, 0, x));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        is_use(o, card, 0)
    });
}

/// A Ghoul Retainer on `host`, with its layers, 2 life.
fn ghoul_retainer_on(e: &mut Engine, host: MinionId) -> CardId {
    let g = borne(e, "Ghoul Retainer", host);
    e.state.cards[g.index()].counters = 2;
    g
}

/// Garrote: "If the opposing vampire would go to torpor during the strike
/// resolution step of this strike and the bearer remains ready, the bearer
/// may burn this card to burn the opposing vampire instead. This is not
/// considered diablerie." X, empty, struck by the Garrote: A burns it, X is
/// burned (its ash heap, not torpor), no blood hunt, A gains nothing.
#[test]
fn garrote_burns_the_vampire_its_strike_sends_to_torpor() {
    let (mut e, a, x) = duel(&[], &[]);
    e.state.minions[x.index()].blood = 0;
    let garrote = borne(&mut e, "Garrote", a);
    let xc = e.state.minions[x.index()].crypt_card().expect("a vampire");
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, card_use(garrote, 0));
    use_when(&mut e, garrote, 1, Target::None);
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[x.index()].in_play, "burned");
    assert!(e.state.seats[1].ash_crypt.contains(&xc));
    assert!(ash_has(&e, 0, "Garrote"));
    assert!(e.referendum().is_none());
    assert_eq!(req.seat, SeatId(0), "no blood hunt: seat 0 plays on");
    assert_eq!(blood(&e, a), 4 - 1, "X's hand strike only, no blood gained");
}

/// Garrote dodged: X dodges it (Spiritual Intervention) but goes to torpor
/// from the Ghoul Retainer's bite in that strike resolution: the burn is
/// offered and used.
/// [ANK 20210612] [LSJ 20020416-2] The burn is not part of the strike: it
/// can be used if the opponent dodges but still goes to torpor.
#[test]
fn garrote_burns_even_when_its_strike_was_dodged() {
    let (mut e, a, _) = duel(&[], &[]);
    let x = e.add_minion(SeatId(1), vampire_with("nec", 0), 0, false);
    let garrote = borne(&mut e, "Garrote", a);
    ghoul_retainer_on(&mut e, a);
    hand(&mut e, 1, &["Spiritual Intervention"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, card_use(garrote, 0));
    pick_when(&mut e, play(0, 0, x));
    use_when(&mut e, garrote, 1, Target::None);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[x.index()].in_play, "burned");
    assert!(ash_has(&e, 0, "Garrote"));
}

/// Garrote used by the employer's Ghoul Retainer: A strikes with its hand,
/// X goes to torpor; the burn is never offered.
/// [TOM 19951114] [LSJ 20080210] The Ghoul is not the bearer: using the
/// Garrote does not let the bearer burn the opponent.
#[test]
fn garrote_used_by_a_ghoul_retainer_burns_no_one() {
    let (mut e, a, x) = duel(&[], &[]);
    e.state.minions[x.index()].blood = 0;
    let garrote = borne(&mut e, "Garrote", a);
    let ghoul = ghoul_retainer_on(&mut e, a);
    bleed_blocked(&mut e, a, x);
    use_when(&mut e, ghoul, 0, Target::Card(garrote));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        is_use(o, garrote, 1)
    });
    assert!(e.state.minions[x.index()].in_torpor);
    assert!(e.state.cards[garrote.index()].in_play);
}

/// Garrote: "and the bearer remains ready": A going to torpor in the same
/// strike resolution is never offered the burn.
#[test]
fn garrote_is_not_offered_to_a_bearer_going_down() {
    let (mut e, a, x) = duel(&[], &[]);
    e.state.minions[x.index()].blood = 0;
    e.state.minions[a.index()].blood = 1;
    e.state.minions[x.index()].timed.push((
        zvtes::cards::Lasting::Strength(1),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(1),
    ));
    let garrote = borne(&mut e, "Garrote", a);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, card_use(garrote, 0));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        is_use(o, garrote, 1)
    });
    assert!(e.state.minions[x.index()].in_torpor);
    assert!(e.state.minions[a.index()].in_torpor);
}

/// A Ravnos of capacity 5 with superior Chimerstry, seat 0's, 4 blood;
/// in seat 0's uncontrolled region a younger Ravnos, an older Ravnos and a
/// younger non-Ravnos, empty. The game, the Ravnos.
fn vrana_table() -> (Engine, MinionId) {
    let (mut e, _, _) = duel(&[], &[]);
    let card = textless(|c| c.clan == "Ravnos" && c.capacity == 5, 0);
    let cap = CRYPT[card as usize].capacity;
    let r = e.add_minion(SeatId(0), card, 4, false);
    e.state.minions[r.index()].gained_levels = vec!["chi", "chi"];
    for f in [
        &(|c: &zvtes::crypt::CryptCard| c.clan == "Ravnos" && c.capacity < cap)
            as &dyn Fn(&zvtes::crypt::CryptCard) -> bool,
        &|c| c.clan == "Ravnos" && c.capacity > cap,
        &|c| c.clan != "Ravnos" && c.capacity < cap,
    ] {
        e.state.seats[0]
            .uncontrolled
            .push(zvtes::state::UncontrolledSlot {
                card: textless(f, 0),
                copy: Default::default(),
                counters: 0,
                banished: None,
            });
    }
    (e, r)
}

/// Karavalanisha Vrana: "This Ravnos can add 2 blood to a younger Ravnos in
/// your uncontrolled region as a +1 stealth action that costs 1 blood."
/// Offered at the younger Ravnos only; it gains 2, the actor pays 1.
#[test]
fn karavalanisha_vrana_feeds_a_younger_ravnos() {
    let (mut e, r) = vrana_table();
    let vrana = borne(&mut e, "Karavalanisha Vrana", r);
    let req = request(&mut e);
    assert!(
        use_of(&req, vrana, 0, Target::Uncontrolled(1)).is_none(),
        "older"
    );
    assert!(
        use_of(&req, vrana, 0, Target::Uncontrolled(2)).is_none(),
        "no Ravnos"
    );
    let c = use_of(&req, vrana, 0, Target::Uncontrolled(0)).expect("the younger Ravnos");
    e.choose(index_of(&req, c));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].uncontrolled[0].counters, 2);
    assert_eq!(blood(&e, r), 4 - 1);
}

/// Karavalanisha Vrana: "Cards requiring Chimerstry [chi] cost this Ravnos
/// 1 fewer blood." Mirror Image [CHI] (1 blood) as A's strike: free with
/// the Vrana, 1 blood without; on a non-Ravnos it reduces nothing and
/// grants no action.
/// [ANK 20180719-3] Clan equipment taken by another minion that does not
/// meet its requirements: "this Ravnos" gives it nothing.
#[test]
fn karavalanisha_vrana_cheapens_chimerstry_for_its_ravnos_only() {
    let paid = |vrana: bool| {
        let (mut e, r) = vrana_table();
        if vrana {
            borne(&mut e, "Karavalanisha Vrana", r);
        }
        hand(&mut e, 0, &["Mirror Image"]);
        pick(&mut e, bleed(r));
        pick_when(&mut e, block(MinionId(2)));
        pick_when(&mut e, play(0, 1, r));
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        4 - blood(&e, r)
    };
    assert_eq!(paid(true) + 1, paid(false));

    let (mut e, a, _) = duel(&[], &[]);
    let card = e.state.minions[a.index()].crypt_card().expect("a vampire");
    assert_ne!(CRYPT[card as usize].clan, "Ravnos");
    e.state.minions[a.index()].gained_levels = vec!["chi", "chi"];
    let vrana = borne(&mut e, "Karavalanisha Vrana", a);
    hand(&mut e, 0, &["Mirror Image"]);
    let req = request(&mut e);
    assert!(!req.options.iter().any(|o| is_use(o, vrana, 0)));
    e.choose(index_of(&req, bleed(a)));
    pick_when(&mut e, block(MinionId(2)));
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(blood(&e, a) < 4, "paid in full");
}

/// Karavalanisha Vrana's action, the Vrana moved off its Ravnos before
/// resolution: it fizzles, no blood moved.
/// [RTR 19960221] The action provided by an equipment fizzles if the
/// equipment is not on the acting minion as it resolves unblocked.
#[test]
fn karavalanisha_vrana_action_fizzles_without_it() {
    let (mut e, r) = vrana_table();
    let vrana = borne(&mut e, "Karavalanisha Vrana", r);
    use_when(&mut e, vrana, 0, Target::Uncontrolled(0));
    expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    e.state.cards[vrana.index()].host = Some(MinionId(1));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Fizzled));
    assert_eq!(e.state.seats[0].uncontrolled[0].counters, 0);
}

/// A textless vampire of `sect`, capacity 4 to 8.
fn of_sect(sect: zvtes::crypt::Sect, nth: usize) -> u16 {
    textless(
        |c| c.sect == Some(sect) && (4..=8).contains(&c.capacity),
        nth,
    )
}

/// A, an Anarch with Malajit, bleeds; X (+1 intercept from a Sport Bike)
/// attempts to block: the game, A, X, Malajit.
fn malajit_table() -> (Engine, MinionId, MinionId, CardId) {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(zvtes::crypt::Sect::Anarch, 0), 4, false);
    e.add_minion(SeatId(0), of_sect(zvtes::crypt::Sect::Anarch, 1), 4, false);
    let x = e.add_minion(
        SeatId(1),
        of_sect(zvtes::crypt::Sect::Camarilla, 0),
        4,
        false,
    );
    e.add_minion(
        SeatId(1),
        of_sect(zvtes::crypt::Sect::Camarilla, 1),
        4,
        false,
    );
    borne(&mut e, "Sport Bike", x);
    let malajit = borne(&mut e, "Malajit Chandramouli", a);
    (e, a, x, malajit)
}

/// Malajit Chandramouli: "Malajit's employer may lock him to get +1 stealth
/// for the current action. If that action is blocked, burn Malajit."
/// Offered as X's attempt needs it; used, the attempt still blocks (1 to
/// 1): Malajit is in the ash heap at the combat's first decision (A's
/// Magnum), before any combat. A Camarilla vampire is not offered his
/// employ.
/// [ANK 20220116] [ANK 20211207] [ANK 20210627] [RTR 19991206] [LSJ
/// 20091125] "If blocked" effects apply first, before any other card or
/// effect.
#[test]
fn malajit_gives_stealth_and_burns_as_the_action_is_blocked() {
    let (mut e, a, x, malajit) = malajit_table();
    let magnum = borne(&mut e, ".44 Magnum", a);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    use_when(&mut e, malajit, 0, Target::None);
    let req = offering(&mut e, card_use(magnum, 0));
    assert!(e.combat().is_some());
    assert!(
        !e.state.cards[malajit.index()].in_play,
        "burned at the block"
    );
    assert!(ash_has(&e, 0, "Malajit Chandramouli"));
    assert_eq!(req.seat, SeatId(0));

    let mut e = game(2);
    let cam = e.add_minion(
        SeatId(0),
        of_sect(zvtes::crypt::Sect::Camarilla, 0),
        4,
        false,
    );
    let anarch = e.add_minion(SeatId(0), of_sect(zvtes::crypt::Sect::Anarch, 0), 4, false);
    e.add_minion(
        SeatId(1),
        of_sect(zvtes::crypt::Sect::Camarilla, 1),
        4,
        false,
    );
    hand(&mut e, 0, &["Malajit Chandramouli"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, cam)));
    assert!(req.options.contains(&play(0, 0, anarch)));
}

/// Malajit used, A's bleed blocked: when Change of Target is offered, he
/// is already burned.
/// [ANK 20220116] [ANK 20211207] [ANK 20210627] [RTR 19991206] [LSJ
/// 20091125] "If blocked" effects apply before any other card or effect
/// (eg. Change of Target).
#[test]
fn malajit_burns_before_change_of_target_is_offered() {
    let (mut e, a, x, malajit) = malajit_table();
    hand(&mut e, 0, &["Change of Target"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    use_when(&mut e, malajit, 0, Target::None);
    offering(&mut e, play(0, 0, a));
    assert!(!e.state.cards[malajit.index()].in_play);
    assert!(ash_has(&e, 0, "Malajit Chandramouli"));
}

/// Malajit locked in an earlier, unblocked action: A's next bleed is
/// blocked, and Malajit stays (the burn is that action's).
#[test]
fn malajit_used_in_an_earlier_action_does_not_burn() {
    let (mut e, a, x, malajit) = malajit_table();
    e.state.minions[a.index()].timed.push((
        zvtes::cards::Lasting::Stealth(1),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(0),
    ));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    use_when(&mut e, malajit, 0, Target::None);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 1, "unblocked");
    assert!(e.state.cards[malajit.index()].locked);
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[malajit.index()].locked, "unlocked");
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 1, "blocked");
    assert!(e.state.cards[malajit.index()].in_play);
}

/// Ritual Scalpel's retainer cut targets "a mortal retainer": Malajit, a
/// mortal, is offered; Omael Kuman, a ghoul, is not.
/// [ANK 20200203-2] [LSJ 20060515] Ghouls are monsters, not mortal.
#[test]
fn ritual_scalpel_cuts_mortal_malajit_never_ghoul_omael() {
    let mut e = game(2);
    let le_dinh_tho = zvtes::state::find_card("Le Dinh Tho");
    let n = e.add_minion(SeatId(0), le_dinh_tho, 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let scalpel = borne(&mut e, "Ritual Scalpel", n);
    let malajit = borne(&mut e, "Malajit Chandramouli", x);
    let omael = borne(&mut e, "Omael Kuman", x);
    let req = request(&mut e);
    assert!(use_of(&req, scalpel, 1, Target::Card(malajit)).is_some());
    assert!(use_of(&req, scalpel, 1, Target::Card(omael)).is_none());
}

/// Pass until `seat`'s block window, decline there.
fn decline_at(e: &mut Engine, seat: SeatId) {
    for _ in 0..50 {
        let req = request(e);
        if req.seat == seat && req.window == WindowLabel::ActionBlock {
            e.choose(index_of(&req, Choice::Pass));
            return;
        }
        e.choose(fallback(&req));
    }
    panic!("{seat:?} never asked to block");
}

/// A, seat 0's, with an Ambulance and a .44 Magnum, hunts on a three-seat
/// table: its prey (seat 1) has X, its predator (seat 2) Z and W, each with
/// a Sport Bike (+1 intercept). Seat 1 declines, Z blocks; after their
/// combat, A locks the Ambulance to continue. The game, A, X, W, the
/// Ambulance, the Magnum.
fn ambulance_continued() -> (Engine, MinionId, MinionId, MinionId, CardId, CardId) {
    let mut e = game(3);
    let mid = |c: &zvtes::crypt::CryptCard| (4..=8).contains(&c.capacity);
    let a = e.add_minion(SeatId(0), textless(mid, 0), 2, false);
    e.add_minion(SeatId(0), textless(mid, 1), 4, false);
    let x = e.add_minion(SeatId(1), textless(mid, 2), 4, false);
    let z = e.add_minion(SeatId(2), textless(mid, 3), 4, false);
    let w = e.add_minion(SeatId(2), textless(mid, 4), 4, false);
    for m in [x, z, w] {
        borne(&mut e, "Sport Bike", m);
    }
    let ambulance = borne(&mut e, "Ambulance", a);
    let magnum = borne(&mut e, ".44 Magnum", a);
    pick(&mut e, action(a, ActionKind::Hunt));
    decline_at(&mut e, SeatId(1));
    pick_when(&mut e, block(z));
    pick_when(&mut e, card_use(magnum, 0));
    let req = until(&mut e, SeatId(0), WindowLabel::ActionAfterCombat);
    let go = use_of(&req, ambulance, 0, Target::None).expect("the Ambulance offered");
    e.choose(index_of(&req, go));
    (e, a, x, w, ambulance, magnum)
}

/// Ambulance: "After a combat between this acting minion and a blocking
/// minion, you may lock the Ambulance to continue the action as if
/// unblocked." Locked; the prey's decline stands (X never asked again);
/// W passes: the hunt succeeds.
/// [ANK 20190116] Methuselahs that declined to block get no new chance.
#[test]
fn ambulance_continues_a_blocked_action_keeping_the_declines() {
    let (mut e, a, x, _, ambulance, _) = ambulance_continued();
    assert!(e.state.cards[ambulance.index()].locked);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(x));
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Successful));
    assert!(e.state.cards[ambulance.index()].in_play);
    assert_eq!(blood(&e, a), 2 - 1 + 1, "Z's hand strike, the hunt's blood");
}

/// The Ambulance continuing keeps the action's modifiers: A's bleed at
/// stealth 1 (Mirror Image) is blocked by X (+1 intercept); continued, the
/// prey's Y cannot reach that stealth, and the bleed lands.
/// [RTR 19941109] [ANK 20190116] Modifiers and reactions stay in effect.
#[test]
fn ambulance_continuation_keeps_the_stealth_played() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("chi", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let y = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    borne(&mut e, "Sport Bike", x);
    let ambulance = borne(&mut e, "Ambulance", a);
    hand(&mut e, 0, &["Mirror Image"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(0, 0, a));
    use_when(&mut e, ambulance, 0, Target::None);
    never_reaching_until(&mut e, SeatId(0), WindowLabel::MinionPhase, y);
    assert_eq!(e.state.seats[1].pool, 10 - 1);
}

/// Ambulance: "If the action is blocked again, burn this card." W blocks
/// the continued hunt: the Ambulance is in its owner's ash heap at the
/// combat's first decision.
/// [ANK 20220116] [ANK 20211207] [ANK 20210627] [RTR 19991206] [LSJ
/// 20091125] Blocked after continuing: the burn applies first.
#[test]
fn ambulance_burns_as_the_continued_action_is_blocked_again() {
    let (mut e, _, _, w, ambulance, magnum) = ambulance_continued();
    pick_when(&mut e, block(w));
    offering(&mut e, card_use(magnum, 0));
    assert!(e.combat().is_some());
    assert!(!e.state.cards[ambulance.index()].in_play);
    assert!(ash_has(&e, 0, "Ambulance"));
}

/// A Nosferatu with an Ambulance, empty, sent to torpor in its hunt's
/// combat and brought back by Warsaw Station: its action has ended, the
/// Ambulance is never offered.
/// [ANK 20220218] [LSJ 20090325-2] The action ends and cannot be continued
/// (eg. with Ambulance).
#[test]
fn ambulance_cannot_continue_an_action_its_minion_lost_in_torpor() {
    let mut e = game(2);
    let n = e.add_minion(
        SeatId(0),
        textless(
            |c| c.clan == "Nosferatu" && (4..=8).contains(&c.capacity),
            0,
        ),
        0,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    borne(&mut e, "Sport Bike", z);
    let ambulance = borne(&mut e, "Ambulance", n);
    let w = in_play(&mut e, 0, "Warsaw Station", None);
    pick_when(&mut e, block(z));
    use_when(&mut e, w, 1, Target::Minion(n));
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
        is_use(o, ambulance, 0)
    });
    assert!(!e.state.minions[n.index()].in_torpor);
    assert!(!e.state.cards[ambulance.index()].locked);
}

/// Go-getter [OBF] and Shadow Boxing [OBF][POT]: a vampire sent to torpor
/// in its hunt's combat and brought back (Warsaw Station for the
/// Nosferatu; the Ravnos Go-getter needs, by hand), its blood refilled:
/// its action has ended, neither continuation is offered.
/// [ANK 20220218] [LSJ 20090325-2] The action ends and cannot be continued
/// (eg. with Ambulance).
#[test]
fn a_continuation_is_not_offered_once_its_vampire_came_back_from_torpor() {
    for card in ["Go-getter", "Shadow Boxing"] {
        let mut e = game(2);
        let clan = if card == "Go-getter" {
            "Ravnos"
        } else {
            "Nosferatu"
        };
        let n = e.add_minion(
            SeatId(0),
            textless(|c| c.clan == clan && (4..=8).contains(&c.capacity), 0),
            0,
            false,
        );
        e.state.minions[n.index()].gained_levels = vec!["OBF", "POT"];
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 0, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        borne(&mut e, "Sport Bike", z);
        let w = in_play(&mut e, 0, "Warsaw Station", None);
        hand(&mut e, 0, &[card]);
        pick_when(&mut e, block(z));
        if card == "Shadow Boxing" {
            use_when(&mut e, w, 1, Target::Minion(n));
        } else {
            // A torpid Nosferatu Warsaw Station may raise keeps the windows
            // open.
            let spare = textless(|c| c.clan == "Nosferatu" && c.capacity >= 4, 0);
            e.add_minion(SeatId(0), spare, 0, true);
            while !e.state.minions[n.index()].in_torpor {
                let req = request(&mut e);
                e.choose(fallback(&req));
            }
            assert!(e.action().is_some(), "the action under way");
            e.state.minions[n.index()].in_torpor = false;
        }
        e.state.minions[n.index()].blood = 3;
        let mode = if card == "Go-getter" { 1 } else { 2 };
        never_offered_where(
            &mut e,
            SeatId(1),
            WindowLabel::MinionPhase,
            |o| matches!(o, Choice::Play { card: 0, mode: m, .. } if *m == mode),
        );
        assert!(!e.state.minions[n.index()].in_torpor, "{card}: back");
        assert!(
            e.state.minions[z.index()].in_torpor,
            "{card}: the blocker down"
        );
    }
}

/// Ambulance: "This minion may lock the Ambulance to attempt to burn an
/// incapacitated imbued as a +1 stealth Ⓓ action." Its own action, not the
/// rulebook's (rules-qa 121.3): A's Ambulance locked as declared;
/// unblocked, the imbued burned with its .44 Magnum, the gun never offered
/// to A, the other ready imbued offered no conviction to avenge it.
#[test]
fn ambulance_burns_an_incapacitated_imbued_as_its_own_action() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let marion = zvtes::state::find_card("Marion \"Teacher193\" Perks");
    let pedro = zvtes::state::find_card("Pedro Cortez");
    let i = e.add_minion(SeatId(1), marion, 1, false);
    let j = e.add_minion(SeatId(1), pedro, CRYPT[pedro as usize].capacity, false);
    let sight = in_play(&mut e, 1, "Second Sight", Some(j));
    let gun = in_play(&mut e, 1, ".44 Magnum", Some(i));
    e.damage(i, 1);
    assert!(e.state.minions[i.index()].incapacitated);
    let ambulance = borne(&mut e, "Ambulance", a);
    let req = request(&mut e);
    let burn = use_of(&req, ambulance, 1, Target::Minion(i)).expect("at the imbued");
    e.choose(index_of(&req, burn));
    request(&mut e);
    assert!(
        e.state.cards[ambulance.index()].locked,
        "locked as declared"
    );
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::BurnInPlay { card } if *card == sight.0)
            || *o == Choice::ChooseCard(gun.0)
    });
    assert!(!e.state.minions[i.index()].in_play, "burned");
    assert!(!e.state.cards[gun.index()].in_play, "burned with her");
    assert_eq!(blood(&e, a), 4, "not avenged");
}

/// Ambulance's burn action, the Ambulance moved off A before resolution:
/// it fizzles, the imbued still incapacitated.
/// [RTR 19960221] The action provided by an equipment fizzles if the
/// equipment is not on the acting minion as it resolves unblocked.
#[test]
fn ambulance_action_fizzles_without_the_ambulance() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let other = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let marion = zvtes::state::find_card("Marion \"Teacher193\" Perks");
    let i = e.add_minion(SeatId(1), marion, 1, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.damage(i, 1);
    let ambulance = borne(&mut e, "Ambulance", a);
    use_when(&mut e, ambulance, 1, Target::Minion(i));
    expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    e.state.cards[ambulance.index()].host = Some(other);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Fizzled));
    assert!(e.state.minions[i.index()].in_play && e.state.minions[i.index()].incapacitated);
}

/// Ambulance: "A minion may have only one vehicle": a minion with a Sport
/// Bike is not offered the Ambulance's equip.
#[test]
fn ambulance_is_a_vehicle() {
    let (mut e, a, _) = duel(&[], &[]);
    borne(&mut e, "Sport Bike", a);
    hand(&mut e, 0, &["Ambulance"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, a)));
    assert!(req.options.contains(&play(0, 0, MinionId(1))));
}

/// Three seats: seat 0's Giovanni G with Charigger, The Axe; seat 1 (its
/// prey) with V (a Soul Gem of Etrius on it, a crypt to draw from) and
/// Saatet-ta; seat 2 with an ally. The game, G, Charigger, V, Saatet-ta,
/// seat 2's ally.
fn charigger_table() -> (Engine, MinionId, CardId, MinionId, MinionId, MinionId) {
    let mut e = game(3);
    let g = e.add_minion(SeatId(0), giovanni(0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.seats[1].crypt = vec![plain("Ventrue", 2)].into();
    borne(&mut e, "Soul Gem of Etrius", v);
    let saatet = e.add_ally(SeatId(1), find_library("Saatet-ta"));
    e.add_minion(SeatId(2), plain("Tremere", 0), 4, false);
    let other = e.add_ally(SeatId(2), find_library("Political Ally"));
    let charigger = borne(&mut e, "Charigger, The Axe", g);
    (e, g, charigger, v, saatet, other)
}

/// Charigger, The Axe: "If a minion controlled by your prey would be
/// burned, instead remove that burning minion from the game and put 1 soul
/// counter on Charigger. This Giovanni gets +1 bleed for each soul counter
/// on Charigger." The prey's Saatet-ta burned: removed, not shuffled into
/// its library; its vampire burned: removed, the Soul Gem draws nothing;
/// two counters, the Giovanni bleeds for 3.
/// [LSJ 20071003-2] [ANK 20230816] Before effects depending on the minion
/// burning (Soul Gem of Etrius).
#[test]
fn charigger_removes_the_preys_burning_minion_and_feeds_its_giovanni() {
    let (mut e, g, charigger, v, saatet, _) = charigger_table();
    let saatet_card = find_library("Saatet-ta");
    let vc = e.state.minions[v.index()].crypt_card().expect("a vampire");
    e.burn(saatet);
    assert!(!e.state.minions[saatet.index()].in_play);
    assert!(e.state.seats[1].removed.contains(&saatet_card));
    assert!(
        !e.state.seats[1].library.contains(&saatet_card),
        "not shuffled"
    );
    assert!(!ash_has(&e, 1, "Saatet-ta"));
    e.burn(v);
    assert_eq!(e.state.seats[1].removed_crypt, vec![vc]);
    assert!(!e.state.seats[1].ash_crypt.contains(&vc));
    assert_eq!(e.state.seats[1].crypt.len(), 1, "no Soul Gem draw");
    assert_eq!(e.state.cards[charigger.index()].counters, 2);
    pick(&mut e, bleed(g));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 3);
}

/// Charigger spares the minions of a Methuselah not its controller's prey:
/// seat 2's ally burned goes to its ash heap, no counter.
#[test]
fn charigger_leaves_other_methuselahs_minions_burned() {
    let (mut e, _, charigger, _, _, other) = charigger_table();
    e.burn(other);
    assert!(ash_has(&e, 2, "Political Ally"));
    assert_eq!(e.state.cards[charigger.index()].counters, 0);
}

/// Charigger: "This Giovanni gets +1 stealth when hunting": its hunt at
/// stealth 2, its bleed at 0.
#[test]
fn charigger_gives_stealth_on_hunts_only() {
    let (mut e, g, _, _, _, _) = charigger_table();
    pick(&mut e, action(g, ActionKind::Hunt));
    request(&mut e);
    assert_eq!(e.action().expect("hunt").stealth(&e.state), 2);
    let (mut e, g, _, _, _, _) = charigger_table();
    pick(&mut e, bleed(g));
    request(&mut e);
    assert_eq!(e.action().expect("bleed").stealth(&e.state), 0);
}

/// Charigger beside Byzar's own burn replacement: the Methuselah whose
/// turn it is (seat 0, Charigger's) chooses which applies, both offered.
/// Charigger's: Byzar, the prey's, removed from the game, a soul counter;
/// Byzar's: torpor with a burn counter, Charigger nothing.
/// [ANK 20230816] Another replacement may be applied first, by sequencing.
#[test]
fn charigger_or_the_burning_minions_own_replacement_as_the_acting_methuselah_chooses() {
    for own in [false, true] {
        let (mut e, _, charigger, _, _, _) = charigger_table();
        let byzar = e.add_minion(SeatId(1), zvtes::state::find_card("Byzar"), 4, false);
        e.burn(byzar);
        let req = request(&mut e);
        let (printed, rider) = (
            Choice::ResolvePrinted { minion: byzar },
            Choice::ResolveRider { card: charigger.0 },
        );
        assert_eq!(
            (req.seat, req.window, &req.options[..]),
            (
                SeatId(0),
                WindowLabel::ResolutionChoice,
                &[printed, rider][..]
            )
        );
        pick(&mut e, if own { printed } else { rider });
        let b = &e.state.minions[byzar.index()];
        if own {
            assert!(b.in_play && b.in_torpor);
            assert_eq!(b.burn_counters.0, 1);
            assert_eq!(e.state.cards[charigger.index()].counters, 0);
        } else {
            assert!(!b.in_play);
            assert!(e.state.seats[1]
                .removed_crypt
                .contains(&zvtes::state::find_card("Byzar")));
            assert_eq!(e.state.cards[charigger.index()].counters, 1);
        }
    }
}

/// Charigger at a diablerie of its prey's vampire: the victim is removed
/// instead of burned, so the diablerie is unsuccessful, as with Byzar: no
/// blood, no .44 Magnum, no blood hunt; the victim removed with its cards,
/// a soul counter on Charigger.
/// [LSJ 20100228] A replaced burn makes a diablerie unsuccessful (Byzar).
#[test]
fn charigger_makes_a_diablerie_of_the_preys_vampire_unsuccessful() {
    let (mut e, _, charigger, _, _, _) = charigger_table();
    let d = MinionId(1);
    let victim = e.add_minion(SeatId(1), plain("Ventrue", 3), 3, true);
    let vc = e.state.minions[victim.index()]
        .crypt_card()
        .expect("a vampire");
    let gun = borne(&mut e, ".44 Magnum", victim);
    let before = blood(&e, d);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: d,
            action: ActionKind::Diablerie,
            target: Some(victim),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.referendum().is_none(), "no blood hunt");
    assert!(!e.state.minions[victim.index()].in_play);
    assert!(e.state.seats[1].removed_crypt.contains(&vc));
    assert_eq!(e.state.cards[charigger.index()].counters, 1);
    assert!(!e.state.cards[gun.index()].in_play, "not taken");
    assert_eq!(blood(&e, d), before, "the diablerist gets nothing");
    assert!(e.state.minions[d.index()].diableries.0.is_empty());
}

/// +n strength to `m` this turn.
fn stronger(e: &mut Engine, m: MinionId, n: i8) {
    let seat = e.state.minions[m.index()].owner;
    e.state.minions[m.index()].timed.push((
        zvtes::cards::Lasting::Strength(n),
        zvtes::state::Expiry::EndOfTurn,
        seat,
    ));
}

/// Nightstick: "Only usable once each round": after its strength+1 strike,
/// Blur's additional strike offers the second Nightstick's strikes, never
/// the first's.
/// [ANK 20230316] A second copy allows a second use in the same round.
#[test]
fn nightstick_strikes_once_a_round_either_way() {
    let (mut e, a, x) = duel(&[], &[]);
    e.state.minions[a.index()].gained_levels = vec!["cel"];
    let n1 = borne(&mut e, "Nightstick", a);
    let n2 = borne(&mut e, "Nightstick", a);
    hand(&mut e, 0, &["Blur"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, card_use(n1, 0));
    pick_when(&mut e, play(0, 0, a));
    let req = offering(&mut e, card_use(n2, 1));
    assert!(req.options.contains(&card_use(n2, 0)));
    assert!(!req
        .options
        .iter()
        .any(|o| is_use(o, n1, 0) || is_use(o, n1, 1)));
}

/// Nightstick's guard against X's hand strikes: 4 damage, A takes 1; 2
/// damage, all prevented, the surplus lost: Blur's additional hand strike
/// lands in full.
/// [RTR 20041202] Fewer points: all prevented. [ANK 20200318] [LSJ
/// 20001114] Unused points are not carried over.
#[test]
fn nightstick_guard_prevents_three_of_the_opposing_hand_strike() {
    let (mut e, a, x) = duel(&[], &[]);
    stronger(&mut e, x, 3);
    let stick = borne(&mut e, "Nightstick", a);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, card_use(stick, 1));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4 - 1);

    let (mut e, a, x) = duel(&[], &[]);
    stronger(&mut e, x, 1);
    e.state.minions[x.index()].gained_levels = vec!["cel"];
    let stick = borne(&mut e, "Nightstick", a);
    hand(&mut e, 1, &["Blur"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, card_use(stick, 1));
    pick_when(&mut e, play(0, 0, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        blood(&e, a),
        4 - 2,
        "the first 2 prevented, the second's 2 not"
    );
}

/// Nightstick's guard at long range: offered. At close range against X's
/// .44 Magnum and Blur: the gun's 2 land (the guard waits), then the
/// additional hand strike's 1 is prevented.
/// [ANK 20260522] The prevent is effective at long range.
#[test]
fn nightstick_guard_works_at_long_range_and_waits_for_a_hand_strike() {
    let (mut e, a, x) = duel(&[], &[]);
    let stick = borne(&mut e, "Nightstick", a);
    let magnum = borne(&mut e, ".44 Magnum", x);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, card_use(magnum, 1));
    offering(&mut e, card_use(stick, 1));
    assert_eq!(
        e.combat().expect("combat").range,
        zvtes::combat::Range::Long
    );

    let (mut e, a, x) = duel(&[], &[]);
    e.state.minions[x.index()].gained_levels = vec!["cel"];
    let stick = borne(&mut e, "Nightstick", a);
    let magnum = borne(&mut e, ".44 Magnum", x);
    hand(&mut e, 1, &["Blur"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, card_use(stick, 1));
    pick_when(&mut e, card_use(magnum, 0));
    pick_when(&mut e, play(0, 0, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        blood(&e, a),
        4 - 2,
        "the Magnum's 2, the hand strike's 1 prevented"
    );
}

/// Nightstick's guard against Elimelech's first strike (2 damage): all
/// prevented, the guard set as the strikes are chosen.
#[test]
fn nightstick_guard_covers_a_first_strike() {
    let (mut e, a, _) = duel(&[], &[]);
    let elimelech = zvtes::state::find_card("Elimelech the Twice-Damned");
    let x = e.add_minion(SeatId(1), elimelech, 4, false);
    let stick = borne(&mut e, "Nightstick", a);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, card_use(stick, 1));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4);
}

/// The `nth` textless Ministry vampire with Serpentis, capacity 4 to 8.
fn ministry(nth: usize) -> u16 {
    textless(
        |c| {
            c.clan == "Ministry"
                && (4..=8).contains(&c.capacity)
                && c.disciplines.iter().any(|d| d.eq_ignore_ascii_case("ser"))
        },
        nth,
    )
}

/// Seat 0's Ministry M and another vampire, seat 1's X and Y: the game, M,
/// the other, X.
fn mokole_table() -> (Engine, MinionId, MinionId, MinionId) {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), ministry(0), 4, false);
    let other = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    (e, m, other, x)
}

/// The play of a card stored on `store` by `m`, in `req`, if offered.
fn stored_play(e: &Engine, req: &zvtes::decision::DecisionRequest, m: MinionId) -> Option<Choice> {
    let hand_len = e.state.seats[0].hand.len() as u8;
    req.options.iter().copied().find(
        |o| matches!(o, Choice::Play { card, actor: Some(a), .. } if *card >= hand_len && *a == m),
    )
}

/// Mokolé Blood: "Search your library (shuffle afterward) and/or ash heap
/// for up to four cards requiring Serpentis [ser] and put them on this
/// card." Equipped: the search offers Pass, the library's Typhonic Beast and
/// the ash heap's Velvet Tongue, never a card without Serpentis; both
/// taken, the card in play holds them; in combat its Ministry plays the
/// Beast from it, as if from hand, not replaced.
/// [LSJ 20081126] Played from it: no replacement.
#[test]
fn mokole_blood_stores_serpentis_cards_from_library_and_ash_heap() {
    let (mut e, m, _, x) = mokole_table();
    let (beast, tongue) = (
        find_library("Typhonic Beast"),
        find_library("Velvet Tongue"),
    );
    let majesty = find_library("Majesty");
    e.state.seats[0].library = vec![majesty, beast, majesty, majesty].into();
    e.state.seats[0].ash = vec![tongue, majesty].into();
    hand(&mut e, 0, &["Mokolé Blood"]);
    fill_hand(&mut e, 0);
    pick(&mut e, play(0, 0, m));
    let req = offering(&mut e, Choice::FindCard(beast));
    let from_ash = Choice::TakeFrom {
        zone: zvtes::decision::Zone::AshHeap,
        card: tongue,
    };
    assert_eq!(
        req.options,
        vec![Choice::Pass, Choice::FindCard(beast), from_ash]
    );
    e.choose(index_of(&req, Choice::FindCard(beast)));
    pick(&mut e, from_ash);
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let mokole = placed_on(&e, "Mokolé Blood", Some(m));
    let mut stored = e.state.cards[mokole.index()].stored.to_cards();
    stored.sort();
    let mut want = vec![beast, tongue];
    want.sort();
    assert_eq!(stored, want);
    assert!(!e.state.seats[0].library.contains(&beast));
    assert_eq!(
        e.state.seats[0].library.len(),
        4 - 1 - 1,
        "its replacement, the Beast"
    );
    assert_eq!(e.state.seats[0].ash.to_cards(), vec![majesty]);
    e.choose(fallback(&req));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, bleed(m));
    pick_when(&mut e, block(x));
    let hand_before = e.state.seats[0].hand.len();
    let req = until(&mut e, SeatId(0), WindowLabel::CombatBeforeRange);
    let beast_play = stored_play(&e, &req, m).expect("the Beast from Mokolé Blood");
    e.choose(index_of(&req, beast_play));
    request(&mut e);
    assert_eq!(
        e.state.cards[mokole.index()].stored.to_cards(),
        vec![tongue]
    );
    assert_eq!(e.state.seats[0].hand.len(), hand_before, "not replaced");
}

/// Mokolé Blood finding no card requiring Serpentis: burned at once.
#[test]
fn mokole_blood_finding_nothing_burns() {
    let (mut e, m, _, _) = mokole_table();
    e.state.seats[0].library = vec![find_library("Majesty")].into();
    hand(&mut e, 0, &["Mokolé Blood"]);
    pick(&mut e, play(0, 0, m));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "Mokolé Blood"));
    assert!(!e
        .state
        .cards
        .iter()
        .any(|c| c.in_play && c.card == find_library("Mokolé Blood")));
}

/// Mokolé Blood equipped while seat 1 controls a copy: its search is asked
/// with the new copy not yet contested; the cards on it, it contests.
/// [ANK 20221028] [LSJ 20090428-1] With another copy in play, the cards
/// are put on it before it enters contest.
#[test]
fn mokole_blood_second_copy_searches_before_its_contest() {
    let (mut e, m, _, x) = mokole_table();
    let theirs = borne(&mut e, "Mokolé Blood", x);
    let beast = find_library("Typhonic Beast");
    e.state.cards[theirs.index()]
        .stored
        .put(beast, zvtes::copies::CopyId::NONE);
    e.state.seats[0].library = vec![beast, find_library("Majesty")].into();
    hand(&mut e, 0, &["Mokolé Blood"]);
    fill_hand(&mut e, 0);
    pick(&mut e, play(0, 0, m));
    offering(&mut e, Choice::FindCard(beast));
    let mine = e
        .state
        .cards
        .iter()
        .rposition(|c| c.in_play && c.card == find_library("Mokolé Blood") && c.host == Some(m))
        .map(|i| CardId(i as u32))
        .expect("in play as it searches");
    assert!(!e.state.cards[mine.index()].contested);
    pick(&mut e, Choice::FindCard(beast));
    request(&mut e);
    assert!(e.state.cards[mine.index()].contested);
    assert_eq!(e.state.cards[mine.index()].stored.to_cards(), vec![beast]);
}

/// Mokolé Blood put on by Alastor, not equipped: no search, it holds
/// nothing and burns.
/// [LSJ 20090506] Merely put into play, it gets no cards.
#[test]
fn mokole_blood_put_by_alastor_holds_nothing_and_burns() {
    let mut e = game(2);
    let j = e.add_minion(
        SeatId(0),
        of_sect(zvtes::crypt::Sect::Camarilla, 0),
        6,
        false,
    );
    e.state.minions[j.index()].granted_title = Some(Title::Justicar("Ventrue"));
    e.add_minion(
        SeatId(0),
        of_sect(zvtes::crypt::Sect::Camarilla, 1),
        4,
        false,
    );
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Alastor"]);
    fill_hand(&mut e, 0);
    let beast = find_library("Typhonic Beast");
    e.state.seats[0].library = vec![beast, find_library("Mokolé Blood"), beast, beast].into();
    pick(&mut e, play(0, 0, j));
    pick_when(&mut e, votes(j, true));
    pick_when(&mut e, Choice::FindCard(find_library("Mokolé Blood")));
    request(&mut e);
    assert!(ash_has(&e, 0, "Mokolé Blood"));
    assert_eq!(
        e.state.seats[0].library.len(),
        4 - 1 - 1,
        "its replacement, Mokolé Blood: no Beast searched"
    );
}

/// Mokolé Blood equipped by Magic of the Smith [tha]: equipped, it
/// searches (the Beast onto it).
/// [LSJ 20090506] It does get cards if it is equipped (eg. Magic of the
/// Smith).
#[test]
fn mokole_blood_equipped_by_magic_of_the_smith_searches() {
    let (mut e, m, _, _) = mokole_table();
    e.state.minions[m.index()].gained_levels = vec!["tha"];
    let beast = find_library("Typhonic Beast");
    let majesty = find_library("Majesty");
    e.state.seats[0].library = vec![beast, find_library("Mokolé Blood"), majesty, majesty].into();
    hand(&mut e, 0, &["Magic of the Smith"]);
    fill_hand(&mut e, 0);
    pick(&mut e, play(0, 0, m));
    pick_when(&mut e, Choice::FindCard(find_library("Mokolé Blood")));
    pick_when(&mut e, Choice::FindCard(beast));
    request(&mut e);
    let mokole = placed_on(&e, "Mokolé Blood", Some(m));
    assert_eq!(e.state.cards[mokole.index()].stored.to_cards(), vec![beast]);
}

/// Mokolé Blood on a non-Ministry: its card is not offered; back on a
/// Ministry, it is.
/// [ANK 20180719-3] Clan equipment taken by another minion.
#[test]
fn mokole_blood_taken_by_a_non_ministry_offers_nothing() {
    let (mut e, _, other, x) = mokole_table();
    let mokole = borne(&mut e, "Mokolé Blood", other);
    let beast = find_library("Typhonic Beast");
    e.state.cards[mokole.index()]
        .stored
        .put(beast, zvtes::copies::CopyId::NONE);
    pick(&mut e, bleed(other));
    pick_when(&mut e, block(x));
    never_offered_where(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { actor: Some(a), .. } if *a == other),
    );

    let (mut e, m2, _, x) = mokole_table();
    let mokole = borne(&mut e, "Mokolé Blood", m2);
    e.state.cards[mokole.index()]
        .stored
        .put(beast, zvtes::copies::CopyId::NONE);
    pick(&mut e, bleed(m2));
    pick_when(&mut e, block(x));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatBeforeRange);
    assert!(stored_play(&e, &req, m2).is_some());
}

/// A card played from Mokolé Blood is canceled as played: Serpent's
/// Numbing Kiss, by Hall of Mirrors [CHI].
/// [LSJ 20060106] A card played "as if from hand" can be canceled "as
/// played".
#[test]
fn mokole_blood_card_can_be_canceled_as_played() {
    let (mut e, m, _, _) = mokole_table();
    e.state.minions[m.index()].gained_levels = vec!["pre"];
    let x = e.add_minion(SeatId(1), vampire_with("CHI", 0), 4, false);
    let mokole = borne(&mut e, "Mokolé Blood", m);
    let kiss = find_library("Serpent's Numbing Kiss");
    e.state.cards[mokole.index()]
        .stored
        .put(kiss, zvtes::copies::CopyId::NONE);
    hand(&mut e, 1, &["Hall of Mirrors"]);
    e.notes = Some(Vec::new());
    pick(&mut e, bleed(m));
    pick_when(&mut e, block(x));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    let from_store = stored_play(&e, &req, m).expect("the Kiss from Mokolé Blood");
    e.choose(index_of(&req, from_store));
    pick_when(&mut e, play(0, 1, x));
    request(&mut e);
    let plays = noted_plays(&mut e);
    assert!(
        plays.iter().any(|p| p.card == kiss && p.canceled),
        "{plays:?}"
    );
}

/// Mokolé Blood's bearer burned: the card and the cards on it go to the ash
/// heap.
/// [LSJ 20021128] A card holding cards leaving play: the cards on it are
/// burned.
#[test]
fn mokole_blood_leaving_play_burns_its_cards() {
    let (mut e, m, _, _) = mokole_table();
    let mokole = borne(&mut e, "Mokolé Blood", m);
    let beast = find_library("Typhonic Beast");
    e.state.cards[mokole.index()]
        .stored
        .put(beast, zvtes::copies::CopyId::NONE);
    e.burn(m);
    assert!(ash_has(&e, 0, "Mokolé Blood"));
    assert!(ash_has(&e, 0, "Typhonic Beast"));
}

// ---- Vampires (vamp_a) ---------------------------------------------------------------

/// The crypt card `name` (not advanced).
#[track_caller]
fn find_card(name: &str) -> u16 {
    CRYPT
        .iter()
        .position(|c| c.name == name && !c.adv)
        .unwrap_or_else(|| panic!("no {name}")) as u16
}

/// `m`'s printed ability `ability` offered in `req`, whatever its actor
/// and target.
fn own_use(req: &zvtes::decision::DecisionRequest, m: MinionId, ability: u8) -> Option<Choice> {
    req.options.iter().copied().find(|o| {
        matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(v), ability: a, .. }
            if *v == m && *a == ability)
    })
}

/// Pass (or hand strike) until `m`'s printed ability `ability` is
/// offered, and use it.
#[track_caller]
fn own_when(e: &mut Engine, m: MinionId, ability: u8) {
    for _ in 0..200 {
        let req = request(e);
        if let Some(c) = own_use(&req, m, ability) {
            e.choose(index_of(&req, c));
            return;
        }
        e.choose(fallback(&req));
    }
    panic!("{m:?}'s ability {ability} never offered");
}

/// Pass (or hand strike) until `seat`'s `window`, asserting `m`'s printed
/// ability `ability` is never offered on the way.
fn own_never_until(e: &mut Engine, m: MinionId, ability: u8, seat: SeatId, window: WindowLabel) {
    never_offered_where(e, seat, window, |o| {
        matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(v), ability: a, .. }
            if *v == m && *a == ability)
    });
}

/// `m`'s printed action at `target`.
fn printed_at(m: MinionId, target: Target) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Vampire(m),
        ability: 0,
        actor: Some(m),
        target,
    }
}

/// +n stealth to `m` this turn.
fn stealthier(e: &mut Engine, m: MinionId, n: i8) {
    let seat = e.state.minions[m.index()].owner;
    e.state.minions[m.index()].timed.push((
        zvtes::cards::Lasting::Stealth(n),
        zvtes::state::Expiry::EndOfTurn,
        seat,
    ));
}

/// The vampires printing only traits: no definition, their sect, title and
/// blood curse generated.
#[test]
fn the_round_30_text_only_vampires_carry_their_printed_traits() {
    use zvtes::crypt::Sect::Independent;
    let cases: [(&str, Title, bool); 16] = [
        ("Ahmose Chambers", Title::None, false),
        ("Anjalika Underwood", Title::None, false),
        ("Belle Equitone", Title::None, false),
        ("Bertrand d'Anjou", Title::None, true),
        ("Brian Thompson", Title::None, false),
        ("Clea Auguste d'Holford", Title::None, false),
        ("Evan Rogers", Title::None, true),
        ("Kashan", Title::Votes(1), true),
        ("Kay Polerno", Title::None, false),
        ("Layla bint-Nadr", Title::None, true),
        ("Marla Kenyon", Title::None, false),
        ("Neel Ramanathan", Title::None, false),
        ("Renenet", Title::None, false),
        ("Sisocharis", Title::None, false),
        ("Stephen Milliner", Title::None, false),
        ("Sukainah", Title::None, true),
    ];
    let mut e = game(2);
    for (name, title, cursed) in cases {
        let card = find_card(name);
        assert!(
            zvtes::cards::vampire(card).is_none(),
            "{name}: no text to define"
        );
        assert_eq!(CRYPT[card as usize].blood_cursed, cursed, "{name}");
        let m = e.add_minion(SeatId(0), card, 1, false);
        assert_eq!(e.state.sect_of(m), Some(Independent), "{name}");
        assert_eq!(e.state.title_of(m), title, "{name}");
    }
}

/// Arnulf Jormungandrsson: once he attempts to block, the acting minion
/// plays no action modifier requiring [dom] (Conditioning) to the action's
/// end, failed attempt or not, while an [obf] one (Lost in Crowds) is still
/// offered; before his attempt Conditioning is offered; the next action's
/// actor is free again.
#[test]
fn arnulf_bars_the_acting_minions_dominate_modifiers_after_his_attempt() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let b = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 1), 4, false);
    for m in [a, b] {
        e.state.minions[m.index()].gained_levels = vec!["dom", "obf"];
    }
    let arnulf = e.add_minion(SeatId(1), find_card("Arnulf Jormungandrsson"), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    hand(
        &mut e,
        0,
        &["Conditioning", "Lost in Crowds", "Conditioning"],
    );
    pick(&mut e, bleed(a));
    let req = request(&mut e);
    assert!(req.options.contains(&play(0, 0, a)), "{:?}", req.options);
    pick_when(&mut e, block(arnulf));
    let req = offering(&mut e, play(1, 0, a));
    assert!(
        !plays(&req)
            .iter()
            .any(|o| matches!(o, Choice::Play { card: 0, .. })),
        "{:?}",
        req.options
    );
    e.choose(index_of(&req, play(1, 0, a)));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::Play { card: 0, .. })
    });
    assert_eq!(
        e.state.seats[1].pool,
        10 - 1,
        "the attempt failed, unblocked"
    );
    pick(&mut e, bleed(b));
    let req = request(&mut e);
    assert!(req.options.contains(&play(0, 0, b)), "{:?}", req.options);
}

/// Bakr's rush: +1 stealth from the declaration, 1 blood paid as it
/// resolves, a combat with the target; at 0 blood it is not offered.
#[test]
fn bakr_rushes_for_a_blood_at_plus_one_stealth() {
    let mut e = game(2);
    let bakr = e.add_minion(SeatId(0), find_card("Bakr"), 3, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let x = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    e.notes = Some(Vec::new());
    pick(&mut e, printed_at(bakr, Target::Minion(x)));
    request(&mut e);
    assert_eq!(e.action().expect("action").stealth(&e.state), 1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(noted_combats(&mut e).iter().any(|n| matches!(
        n,
        zvtes::combat::CombatNote::Begins { minions } if *minions == [bakr, x]
    )));
    // The blood paid, then X's hand strike.
    assert_eq!(blood(&e, bakr), 3 - 1 - 1);

    let mut e = game(2);
    let bakr = e.add_minion(SeatId(0), find_card("Bakr"), 0, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let x = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    let req = request(&mut e);
    assert!(!req.options.contains(&printed_at(bakr, Target::Minion(x))));
}

/// Chavi Oraczko: her stealth once an action, for a blood, against a
/// blocker; +1 hand size while ready, none in torpor.
#[test]
fn chavi_buys_stealth_once_and_widens_the_hand() {
    let mut e = game(2);
    let chavi = e.add_minion(SeatId(0), find_card("Chavi Oraczko"), 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let x = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    assert_eq!(e.state.hand_size(SeatId(0)), 7 + 1);
    pick(&mut e, bleed(chavi));
    pick_when(&mut e, block(x));
    own_when(&mut e, chavi, 0);
    assert_eq!(blood(&e, chavi), 3);
    own_never_until(&mut e, chavi, 0, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 1, "the block failed");
    e.state.minions[chavi.index()].in_torpor = true;
    assert_eq!(e.state.hand_size(SeatId(0)), 7);
}

/// Dmitri Borodin, blocked by a strength-3 X with Blur: his prevention takes
/// all of the first strike's damage for 1 blood, and is not offered for the
/// additional strike of the same combat.
/// [LSJ 20001114] Prevention only with strike damage pending.
#[test]
fn dmitri_prevents_a_whole_strike_once_a_combat() {
    let mut e = game(2);
    let dmitri = e.add_minion(SeatId(0), find_card("Dmitri Borodin"), 6, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let x = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    e.state.minions[x.index()].gained_levels = vec!["cel"];
    stronger(&mut e, x, 2);
    hand(&mut e, 1, &["Blur"]);
    bleed_blocked(&mut e, dmitri, x);
    own_when(&mut e, dmitri, 0);
    assert_eq!(blood(&e, dmitri), 6 - 1, "the whole strike prevented");
    pick_when(&mut e, play(0, 0, x));
    own_never_until(&mut e, dmitri, 0, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, dmitri), 6 - 1 - 3, "the additional strike lands");
}

/// Dmitri Borodin's stealth: once an action, for a blood, against a
/// blocker.
#[test]
fn dmitri_buys_stealth_once_an_action() {
    let mut e = game(2);
    let dmitri = e.add_minion(SeatId(0), find_card("Dmitri Borodin"), 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let x = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    pick(&mut e, bleed(dmitri));
    pick_when(&mut e, block(x));
    own_when(&mut e, dmitri, 1);
    assert_eq!(blood(&e, dmitri), 3);
    own_never_until(&mut e, dmitri, 1, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 1, "the block failed");
}

/// Durga Syn steals, with first strike, the .44 Magnum X strikes with: X's
/// strike does nothing, the gun is Durga's, her blood 1 less; a Kevlar Vest
/// (no weapon) is a target too.
#[test]
fn durga_syn_steals_equipment_with_first_strike() {
    let mut e = game(2);
    let durga = e.add_minion(SeatId(0), find_card("Durga Syn"), 6, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let x = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    let gun = in_play(&mut e, 1, ".44 Magnum", Some(x));
    let vest = in_play(&mut e, 1, "Kevlar Vest", Some(x));
    bleed_blocked(&mut e, durga, x);
    let steal = |c: CardId| Choice::UseAbility {
        source: AbilitySource::Vampire(durga),
        ability: 0,
        actor: None,
        target: Target::Card(c),
    };
    let mut stolen = false;
    let mut struck = false;
    while !(stolen && struck) {
        let req = request(&mut e);
        if !stolen && req.options.contains(&steal(gun)) {
            assert!(req.options.contains(&steal(vest)), "{:?}", req.options);
            e.choose(index_of(&req, steal(gun)));
            stolen = true;
        } else if let Some(c) = use_of(&req, gun, 0, Target::None).filter(|_| !struck) {
            e.choose(index_of(&req, c));
            struck = true;
        } else {
            e.choose(fallback(&req));
        }
    }
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[gun.index()].host, Some(durga));
    assert_eq!(e.state.cards[vest.index()].host, Some(x));
    assert_eq!(blood(&e, durga), 6 - 1, "the gun's strike did nothing");
}

/// Durga Syn's steal never takes a unique equipment whose copy her
/// Methuselah already controls (rulebook "Contested Cards"; advanced rules
/// §1.13.1): with no other equipment, not offered.
#[test]
fn durga_syn_steals_no_unique_copy_her_methuselah_controls() {
    let mut e = game(2);
    let durga = e.add_minion(SeatId(0), find_card("Durga Syn"), 6, false);
    let b = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let x = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    in_play(&mut e, 0, "Ivory Bow", Some(b));
    in_play(&mut e, 1, "Ivory Bow", Some(x));
    bleed_blocked(&mut e, durga, x);
    own_never_until(&mut e, durga, 0, SeatId(0), WindowLabel::MinionPhase);
}

/// Durga Syn stays unlocked blocking an ally or a capacity-4 vampire; a
/// capacity-5 vampire's block locks her.
#[test]
fn durga_syn_stays_unlocked_blocking_allies_and_small_vampires() {
    for (actor, locked) in [(Some(4u8), false), (None, false), (Some(5), true)] {
        let mut e = game(2);
        let a = match actor {
            Some(cap) => e.add_minion(SeatId(0), cap_with("", cap, 0), 4, false),
            None => e.add_ally(SeatId(0), find_library("Gravebound Drone")),
        };
        e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
        let durga = e.add_minion(SeatId(1), find_card("Durga Syn"), 6, false);
        e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(durga));
        request(&mut e);
        assert_eq!(
            e.state.minions[durga.index()].locked,
            locked,
            "actor {actor:?}"
        );
    }
}

/// Francis Milliner hits a Toreador and a Toreador antitribu for 2, a
/// Ventrue for 1.
#[test]
fn francis_milliner_hits_harder_on_toreador() {
    for (clan, damage) in [("Toreador", 2), ("Toreador antitribu", 2), ("Ventrue", 1)] {
        let mut e = game(2);
        let f = e.add_minion(SeatId(0), find_card("Francis Milliner"), 4, false);
        e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
        let x = e.add_minion(SeatId(1), of_clan(clan, "", 0), 4, false);
        e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
        bleed_blocked(&mut e, f, x);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, x), 4 - damage, "{clan}");
    }
}

/// Ganesh bleeds for 2 a Methuselah with a ready Giovanni, for 1 once it
/// is in torpor.
#[test]
fn ganesh_bleeds_harder_against_a_giovanni_keeper() {
    for (torpor, bled) in [(false, 2), (true, 1)] {
        let mut e = game(2);
        let g = e.add_minion(SeatId(0), find_card("Ganesh"), 4, false);
        e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
        e.add_minion(SeatId(1), giovanni(0), 4, torpor);
        e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
        pick(&mut e, bleed(g));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[1].pool, 10 - bled, "torpor {torpor}");
    }
}

/// Gwen Brand gets a founder counter in her controller's unlock phase while
/// another Methuselah controls a Gehenna event; none for her own event, none
/// in the other's unlock phase.
#[test]
fn gwen_brand_gathers_founder_counters_under_anothers_gehenna() {
    for (seat, counters) in [(Some(1u8), 1u8), (Some(0), 0), (None, 0)] {
        let mut e = game(2);
        let gwen = e.add_minion(SeatId(0), find_card("Gwen Brand"), 3, false);
        e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
        e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
        if let Some(s) = seat {
            in_play(&mut e, s, "Fall of the Sabbat", None);
        }
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(e.state.minions[gwen.index()].founder_counters.0, 0);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(
            e.state.minions[gwen.index()].founder_counters.0,
            counters,
            "event of {seat:?}"
        );
    }
}

/// Gwen Brand at 4 founder counters: superior Animalism, Auspex, Chimerstry
/// and Fortitude, capacity 4, a bleed of 2; at 3, her printed text only.
#[test]
fn gwen_brand_founded_at_four_counters() {
    for (n, sup, cap, bled) in [(4u8, 2u8, 4u8, 2), (3, 0, 3, 1)] {
        let mut e = game(2);
        let gwen = e.add_minion(SeatId(0), find_card("Gwen Brand"), 3, false);
        e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
        e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
        e.state.minions[gwen.index()].founder_counters.0 = n;
        assert_eq!(e.state.discipline_level(gwen, "ani"), sup, "{n}");
        assert_eq!(e.state.discipline_level(gwen, "for"), sup, "{n}");
        assert_eq!(e.state.discipline_level(gwen, "aus"), sup.max(1), "{n}");
        assert_eq!(e.state.discipline_level(gwen, "chi"), sup.max(1), "{n}");
        assert_eq!(e.state.capacity_of(gwen), Some(cap), "{n}");
        pick(&mut e, bleed(gwen));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[1].pool, 10 - bled, "{n}");
    }
}

/// Hafsa, The Watcher: a Camarilla vampire burns 1 blood to attempt to
/// block her, one without blood cannot; an Independent attempts for free.
#[test]
fn hafsa_taxes_camarilla_blockers() {
    use zvtes::crypt::Sect;
    for (sect, start, offered, left) in [
        (Sect::Camarilla, 2u8, true, 1u8),
        (Sect::Camarilla, 0, false, 0),
        (Sect::Independent, 2, true, 2),
    ] {
        let mut e = game(2);
        let hafsa = e.add_minion(SeatId(0), find_card("Hafsa, The Watcher"), 4, false);
        e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
        let x = e.add_minion(SeatId(1), of_sect(sect, 0), start, false);
        e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 0), 4, false);
        pick(&mut e, bleed(hafsa));
        if offered {
            pick_when(&mut e, block(x));
            assert_eq!(blood(&e, x), left, "{sect:?}");
        } else {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(x));
        }
    }
}

/// Hafsa, The Watcher: another Methuselah's Banu Haqim bleeds her
/// controller for 2, a Ventrue for 1.
#[test]
fn hafsa_draws_banu_haqim_bleeds() {
    for (clan, bled) in [("Banu Haqim", 2), ("Ventrue", 1)] {
        let mut e = game(2);
        let b = e.add_minion(SeatId(0), of_clan(clan, "", 0), 4, false);
        e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
        e.add_minion(SeatId(1), find_card("Hafsa, The Watcher"), 4, true);
        e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, true);
        pick(&mut e, bleed(b));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[1].pool, 10 - bled, "{clan}");
    }
}

/// Halim Bey, ready: Saatet-ta, a mummy his Methuselah controls, bleeds
/// with 1 stealth; with Halim in torpor, 0.
#[test]
fn halim_bey_hides_his_mummies() {
    for (torpor, stealth) in [(false, 1), (true, 0)] {
        let mut e = game(2);
        e.add_minion(SeatId(0), find_card("Halim Bey"), 4, torpor);
        let mummy = e.add_ally(SeatId(0), find_library("Saatet-ta"));
        e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 0), 4, false);
        pick(&mut e, bleed(mummy));
        request(&mut e);
        assert_eq!(
            e.action().expect("action").stealth(&e.state),
            stealth,
            "torpor {torpor}"
        );
    }
}

/// Izhim abd Azrael: Loss [qui] (1 blood) costs him nothing; a .44 Magnum
/// (2 pool) 1 pool.
#[test]
fn izhim_pays_less_for_quietus_and_weapons() {
    let mut e = game(2);
    let izhim = e.add_minion(SeatId(0), find_card("Izhim abd Azrael"), 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let x = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    let vest = in_play(&mut e, 1, "Kevlar Vest", Some(x));
    hand(&mut e, 0, &["Loss"]);
    pick(&mut e, play_on(0, 0, izhim, Target::Card(vest)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 1, "Kevlar Vest"), "burned");
    assert_eq!(blood(&e, izhim), 4);

    let mut e = game(2);
    let izhim = e.add_minion(SeatId(0), find_card("Izhim abd Azrael"), 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    hand(&mut e, 0, &[".44 Magnum"]);
    pick(&mut e, play(0, 0, izhim));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, ".44 Magnum", Some(izhim));
    assert_eq!(e.state.seats[0].pool, 10 - 1);
}

/// Izhim abd Azrael made Independent keeps Black Hand: Fall of the Sabbat
/// is still not played while he is ready.
/// [LSJ 20070322] [ANK 20180807] Black Hand is a trait, not lost with the
/// sect.
#[test]
fn izhim_keeps_black_hand_made_independent() {
    let mut e = game(2);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let izhim = e.add_minion(SeatId(1), find_card("Izhim abd Azrael"), 4, false);
    e.state.minions[izhim.index()].sect_override = Some(zvtes::crypt::Sect::Independent);
    for name in ["Thirst", "Torpid Blood", "Blood Weakens"] {
        in_play(&mut e, 1, name, None);
    }
    hand(&mut e, 0, &["Fall of the Sabbat", "Minion Tap"]);
    let req = to_window(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    assert_eq!(
        e.state.sect_of(izhim),
        Some(zvtes::crypt::Sect::Independent)
    );
    assert!(e.state.is_black_hand(izhim));
    assert!(
        !plays(&req)
            .iter()
            .any(|o| matches!(o, Choice::Play { card: 0, .. })),
        "{req:?}"
    );
}

/// Jamal hits an ally and a Camarilla vampire for 2, a Sabbat vampire for
/// 1; his bleed is 2.
#[test]
fn jamal_hits_allies_and_the_camarilla_harder() {
    use zvtes::crypt::Sect;
    for (foe, damage) in [
        (Some(Sect::Camarilla), 2u8),
        (Some(Sect::Sabbat), 1),
        (None, 2),
    ] {
        let mut e = game(2);
        let jamal = e.add_minion(SeatId(0), find_card("Jamal"), 4, false);
        e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
        let x = match foe {
            Some(s) => e.add_minion(SeatId(1), of_sect(s, 0), 4, false),
            None => e.add_ally(SeatId(1), find_library("Gravebound Drone")),
        };
        e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
        let before = blood(&e, x);
        bleed_blocked(&mut e, jamal, x);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(before - blood(&e, x), damage, "{foe:?}");
    }
    let mut e = game(2);
    let jamal = e.add_minion(SeatId(0), find_card("Jamal"), 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, true);
    pick(&mut e, bleed(jamal));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 2);
}

/// Jamal and Lady Constancia made Anarch keep their 2 votes (titled): each
/// casts 2 in a referendum.
/// [LSJ 20010714] Personal titles are kept with their votes through a sect
/// change.
#[test]
fn round30_personal_titles_survive_a_sect_change() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let jamal = e.add_minion(SeatId(0), find_card("Jamal"), 4, false);
    let lady = e.add_minion(SeatId(0), find_card("Lady Constancia"), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    for m in [jamal, lady] {
        e.state.minions[m.index()].sect_override = Some(zvtes::crypt::Sect::Anarch);
        assert_eq!(e.state.title_of(m), Title::Votes(2));
    }
    hand(&mut e, 0, &["Kine Resources Contested"]);
    pick(&mut e, play(0, 0, caller));
    pick(&mut e, alloc(0, 3));
    to_window(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
    let r = e.referendum().expect("referendum");
    assert_eq!(r.votes_of(&e.state, jamal), 2);
    assert_eq!(r.votes_of(&e.state, lady), 2);
}

/// Jayakumar steals another Methuselah's equipment for 2 blood, never his
/// own Methuselah's; with 1 blood the action is not offered.
#[test]
fn jayakumar_steals_equipment_for_two_blood() {
    let mut e = game(2);
    let jay = e.add_minion(SeatId(0), find_card("Jayakumar"), 3, false);
    let b = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let x = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    let own = in_play(&mut e, 0, "Kevlar Vest", Some(b));
    let theirs = in_play(&mut e, 1, ".44 Magnum", Some(x));
    let req = request(&mut e);
    assert!(!req.options.contains(&printed_at(jay, Target::Card(own))));
    e.choose(index_of(&req, printed_at(jay, Target::Card(theirs))));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[theirs.index()].host, Some(jay));
    assert_eq!(blood(&e, jay), 1);

    let mut e = game(2);
    let jay = e.add_minion(SeatId(0), find_card("Jayakumar"), 1, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let x = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    let theirs = in_play(&mut e, 1, ".44 Magnum", Some(x));
    let req = request(&mut e);
    assert!(!req.options.contains(&printed_at(jay, Target::Card(theirs))));
}

/// Johann Matheson's successful hunt: another Methuselah's XTC-Laced Blood
/// is not offered on him; another vampire's hunt, it is.
#[test]
fn johann_matheson_is_no_target_of_xtc_laced_blood() {
    let mut e = game(2);
    let johann = e.add_minion(SeatId(0), find_card("Johann Matheson"), 1, false);
    let h = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 1, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    hand(&mut e, 1, &["XTC-Laced Blood"]);
    let xtc = |m| Choice::Play {
        card: 0,
        mode: 0,
        actor: None,
        target: Target::Minion(m),
    };
    pick(&mut e, action(johann, ActionKind::Hunt));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, xtc(johann));
    assert_eq!(blood(&e, johann), 2, "the hunt succeeded");
    pick(&mut e, action(h, ActionKind::Hunt));
    let req = offering(&mut e, xtc(h));
    assert_eq!(req.seat, SeatId(1));
}

/// Kabede Maru, at no printed intercept, reaches a 1-stealth political
/// action (Kine Resources Contested); a +1 stealth bleed he cannot.
#[test]
fn kabede_maru_intercepts_politics() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 1), 4, false);
    let kabede = e.add_minion(SeatId(1), find_card("Kabede Maru"), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    stealthier(&mut e, a, 1);
    hand(&mut e, 0, &["Kine Resources Contested"]);
    pick(&mut e, play(0, 0, caller));
    let req = offering(&mut e, block(kabede));
    assert_eq!(e.action().expect("action").stealth(&e.state), 1);
    assert!(e.block_can_reach(req.seat, kabede));
    e.choose(index_of(&req, Choice::Pass));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, bleed(a));
    never_reaching_until(&mut e, SeatId(0), WindowLabel::MinionPhase, kabede);
}

/// Kiradin, blocked: 1 blood for a limited additional strike, once in the
/// round (X takes 2, Kiradin 1 + 1); Blur's limited strike is not offered
/// after it. Blur used first, his is not offered.
#[test]
fn kiradin_buys_one_limited_strike_a_round() {
    for kiradin_first in [true, false] {
        let mut e = game(2);
        let k = e.add_minion(SeatId(0), find_card("Kiradin"), 4, false);
        e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
        let x = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
        e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
        e.state.minions[k.index()].gained_levels = vec!["cel"];
        hand(&mut e, 0, &["Blur"]);
        bleed_blocked(&mut e, k, x);
        if kiradin_first {
            own_when(&mut e, k, 0);
            never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
                matches!(o, Choice::Play { card: 0, .. })
                    || matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(v), .. } if *v == k)
            });
            assert_eq!(blood(&e, k), 4 - 1 - 1);
            assert_eq!(blood(&e, x), 4 - 2);
        } else {
            pick_when(&mut e, play(0, 0, k));
            own_never_until(&mut e, k, 0, SeatId(0), WindowLabel::MinionPhase);
        }
    }
}

/// Lady Constancia: the Edge burned and a discard phase action spent, a
/// card discarded at random (a Form of Mist of a full hand) is replaced by a
/// library card of the ash heap she picks, never the one discarded, none
/// drawn; no second use without the Edge.
#[test]
fn lady_constancia_replaces_a_random_discard_from_the_ash_heap() {
    let mut e = game(2);
    let lady = e.add_minion(SeatId(0), find_card("Lady Constancia"), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 0), 4, false);
    e.state.seats[0].has_edge = true;
    fill_hand(&mut e, 0);
    let mist = find_library("Form of Mist");
    let hand_size = e.state.seats[0].hand.len();
    for c in ["Blur", "Lost in Crowds"] {
        e.state.seats[0].ash.push(find_library(c));
    }
    e.state.seats[0].library.push(find_library("Loss"));
    own_when(&mut e, lady, 0);
    let req = request(&mut e);
    let mut offered = req.options.clone();
    offered.sort_by_key(|o| format!("{o:?}"));
    let mut want = vec![
        Choice::FindCard(find_library("Blur")),
        Choice::FindCard(find_library("Lost in Crowds")),
    ];
    want.sort_by_key(|o| format!("{o:?}"));
    assert_eq!(offered, want, "never the card discarded");
    e.choose(index_of(
        &req,
        Choice::FindCard(find_library("Lost in Crowds")),
    ));
    assert!(!e.state.seats[0].has_edge);
    let held = e.state.seats[0].hand.to_cards();
    assert_eq!(held.len(), hand_size);
    assert_eq!(held.iter().filter(|&&c| c == mist).count(), hand_size - 1);
    assert!(held.contains(&find_library("Lost in Crowds")));
    assert!(ash_has(&e, 0, "Form of Mist"));
    assert_eq!(e.state.seats[0].library.len(), 1, "nothing drawn");
    own_never_until(&mut e, lady, 0, SeatId(1), WindowLabel::MinionPhase);
}

/// Lady Constancia beside Visit from the Capuchin: the replacement offered
/// either way, the ash heap card or the Capuchin's counter; the counter
/// chosen, nothing taken from the ash heap, nothing drawn.
/// [rules-qa 8.3] Visit from the Capuchin counts every card the rules
/// replace. [ANK 20200905] Another alternative for card replacement: you
/// choose which to apply.
#[test]
fn lady_constancia_replacement_meets_visit_from_the_capuchin() {
    let mut e = game(2);
    let lady = e.add_minion(SeatId(0), find_card("Lady Constancia"), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 0), 4, false);
    e.state.seats[0].has_edge = true;
    let visit = in_play(&mut e, 0, "Visit from the Capuchin", None);
    e.state.cards[visit.index()].counters = 4;
    fill_hand(&mut e, 0);
    let hand_size = e.state.seats[0].hand.len();
    let blur = find_library("Blur");
    e.state.seats[0].ash.push(blur);
    e.state.seats[0].library.push(find_library("Loss"));
    own_when(&mut e, lady, 0);
    let req = request(&mut e);
    assert_eq!(
        req.options,
        vec![Choice::DrawLibrary, Choice::FindCard(blur)]
    );
    e.choose(index_of(&req, Choice::DrawLibrary));
    assert_eq!(e.state.cards[visit.index()].counters, 3, "a counter burned");
    assert_eq!(e.state.seats[0].hand.len(), hand_size - 1, "not replaced");
    assert!(ash_has(&e, 0, "Blur"));
    assert_eq!(e.state.seats[0].library.len(), 1, "nothing drawn");
}

/// Lady Constancia's ability needs the Edge and a library card in the ash
/// heap: with only a burned vampire in the heap, or no Edge, not offered.
#[test]
fn lady_constancia_needs_the_edge_and_a_library_card_in_the_ash_heap() {
    for (edge, ash) in [(true, false), (false, true)] {
        let mut e = game(2);
        let lady = e.add_minion(SeatId(0), find_card("Lady Constancia"), 4, false);
        e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 0), 4, false);
        e.state.seats[0].has_edge = edge;
        hand(&mut e, 0, &["Conditioning"]);
        e.state.seats[0]
            .ash_crypt
            .push(textless(|c| c.capacity >= 4, 1));
        if ash {
            e.state.seats[0].ash.push(find_library("Blur"));
        }
        own_never_until(&mut e, lady, 0, SeatId(1), WindowLabel::MinionPhase);
    }
}

// ---- Combat cards ------------------------------------------------------------

/// Seat 0's A and a second vampire, seat 1's X and a second one: plain
/// textless vampires of capacity 4 to 8, 4 blood each, A with `a_levels`
/// and X with `x_levels` gained.
fn duel(a_levels: &[&'static str], x_levels: &[&'static str]) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let mid = |c: &zvtes::crypt::CryptCard| (4..=8).contains(&c.capacity);
    let a = e.add_minion(SeatId(0), textless(mid, 0), 4, false);
    e.add_minion(SeatId(0), textless(mid, 1), 4, false);
    let x = e.add_minion(SeatId(1), textless(mid, 2), 4, false);
    e.add_minion(SeatId(1), textless(mid, 3), 4, false);
    e.state.minions[a.index()].gained_levels = a_levels.to_vec();
    e.state.minions[x.index()].gained_levels = x_levels.to_vec();
    (e, a, x)
}

fn pending(e: &Engine, side: usize) -> (u8, u8) {
    let p = e.combat().expect("combat").sides[side].pending;
    (p.normal, p.aggravated)
}

// ---- Amria ------------------------------------------------------------------

/// Amria [chi]: "Maneuver, only usable to get to long range." Offered at
/// close range; A at long range after its maneuver, X maneuvering back is
/// then left with no Amria offered to A.
#[test]
fn amria_maneuvers_to_long_range_only() {
    let (mut e, a, x) = duel(&["chi"], &[]);
    hand(&mut e, 0, &["Amria", "Amria"]);
    hand(&mut e, 1, &["Fake Out"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    assert_eq!(e.combat().expect("combat").range, Range::Long);
    pick_when(&mut e, play(0, 0, x));
    // Back at close, the second is offered again.
    let req = request(&mut e);
    assert_eq!(e.combat().expect("combat").range, Range::Close);
    assert!(req.options.contains(&play(0, 0, a)), "{:?}", req.options);
}

/// Amria [CHI]: "Strike: dodge, with 1 optional press, only usable to end
/// combat." "The optional press can only be used during the current
/// round." [TOM 19960521] A dodges X's strike; no press for A to continue;
/// X presses to continue (Relentless Pursuit), A's press ends the combat.
#[test]
fn amria_superior_dodges_with_an_ending_press_this_round() {
    let (mut e, a, x) = duel(&["chi", "chi"], &["pot"]);
    hand(&mut e, 0, &["Amria"]);
    hand(&mut e, 1, &["Relentless Pursuit"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    // A's press step elided (no press to continue for it): X's.
    let req = expect(&mut e, SeatId(1), WindowLabel::CombatPress);
    assert!(req.options.contains(&play(0, 0, x)));
    assert_eq!(blood(&e, a), 4, "dodged");
    pick(&mut e, play(0, 0, x));
    pick_when(&mut e, Choice::UsePress);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.combat().is_none());
    assert_eq!(blood(&e, a), 4, "no second round");
}

// ---- Apparition -------------------------------------------------------------

/// Apparition [CHI]: "Can be played at any time in combat, even if there is
/// no damage to prevent yet." [LSJ 20010315] "If fewer points of
/// (preventable) damage are being resolved, then the effect prevents all of
/// those points." [RTR 20041202] Played before range, no damage pending;
/// X's 3 prevented to 1, then in round 2 its 1 prevented whole; replaced
/// only after combat.
#[test]
fn apparition_is_played_before_damage_and_prevents_each_round() {
    let (mut e, a, x) = duel(&["chi", "chi"], &["pot"]);
    stronger(&mut e, x, 2);
    hand(&mut e, 0, &["Apparition"]);
    hand(&mut e, 1, &["Relentless Pursuit"]);
    e.state.seats[0].library = vec![find_library("Fake Out"); 3].into();
    bleed_blocked(&mut e, a, x);
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::CombatBeforeRange);
    pick(&mut e, play(0, 1, a));
    pick_when(&mut e, Choice::UsePrevent);
    pick(&mut e, Choice::UsePrevent);
    request(&mut e);
    assert_eq!(blood(&e, a), 4 - 1 - 1, "its cost; 3 less 2");
    assert_eq!(e.state.seats[0].held[zvtes::state::Release::AfterCombat], 1);
    pick_when(&mut e, play(0, 0, x));
    // Round 2: X's strength 3 again, 2 prevented.
    pick_when(&mut e, Choice::UsePrevent);
    pick(&mut e, Choice::UsePrevent);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4 - 1 - 2);
    assert_eq!(e.state.seats[0].held[zvtes::state::Release::AfterCombat], 0);
}

/// Apparition [chi]: "Unused prevention points can't be carried over unless
/// optional ('can/may prevent')." [ANK 20200318] [LSJ 20001114] Its
/// optional point of round 1 left unused, round 2 still allows 1 only.
#[test]
fn apparition_does_not_carry_unused_prevention() {
    let (mut e, a, x) = duel(&["chi"], &["pot"]);
    stronger(&mut e, x, 1);
    hand(&mut e, 0, &["Apparition"]);
    hand(&mut e, 1, &["Relentless Pursuit"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    // Round 1: the point not used.
    let req = until(&mut e, SeatId(0), WindowLabel::CombatPrevention);
    assert!(req.options.contains(&Choice::UsePrevent));
    pick(&mut e, Choice::Pass);
    pick_when(&mut e, play(0, 0, x));
    pick_when(&mut e, Choice::UsePrevent);
    let req = request(&mut e);
    assert!(!req.options.contains(&Choice::UsePrevent), "one a round");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4 - 1 - 2 - 1, "its cost; 2; 2 less 1");
    assert!(!e.state.minions[a.index()].in_torpor);
}

// ---- Flesh Bond -------------------------------------------------------------

/// Flesh Bond: "[ani] Strike: 2R damage. [ANI] Strike: combat ends." At
/// long range, X takes 2 and A none; [ANI], neither takes damage.
#[test]
fn flesh_bond_strikes_two_at_long_range_or_ends_combat() {
    let (mut e, a, x) = duel(&["ani"], &[]);
    hand(&mut e, 0, &["Fake Out", "Flesh Bond"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, x)), (4 - 1, 4 - 2), "its cost; 2R");

    let (mut e, a, x) = duel(&["ani", "ani"], &[]);
    hand(&mut e, 0, &["Flesh Bond"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, x)), (4 - 1, 4), "its cost only");
}

// ---- Mercury's Arrow --------------------------------------------------------

/// Mercury's Arrow [cel]: "Strike: 1R damage, with an optional maneuver."
/// Played at the maneuver, to long range, it commits the strike: X takes
/// its 1, A nothing from X's hand. [CEL]: 3R.
#[test]
fn mercurys_arrow_maneuvers_and_commits_its_strike() {
    let (mut e, a, x) = duel(&["cel"], &[]);
    hand(&mut e, 0, &["Mercury's Arrow"]);
    bleed_blocked(&mut e, a, x);
    let req = until(&mut e, SeatId(0), WindowLabel::CombatDetermineRange);
    assert!(req.options.contains(&play(0, 0, a)));
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, x)), (4 - 1, 4 - 1));

    let (mut e, a, x) = duel(&["cel", "cel"], &[]);
    hand(&mut e, 0, &["Mercury's Arrow"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, x)), (4 - 1 - 1, 4 - 3));
}

/// Mercury's Arrow [cel]: "The optional maneuver cannot be used if the
/// strike cannot be used (eg. Hidden Lurker)." [LSJ 20021028] A under its
/// Eyes of Blades [AUS][CEL] ("this vampire cannot strike"): no Mercury's
/// Arrow at the maneuver, nor as a strike.
#[test]
fn mercurys_arrow_maneuver_is_not_used_while_strikes_are_barred() {
    let (mut e, a, x) = duel(&["aus", "aus", "cel", "cel"], &[]);
    hand(&mut e, 0, &["Eyes of Blades", "Mercury's Arrow"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
}

// ---- Shadow Feint -----------------------------------------------------------

/// Shadow Feint [cel][obf]: "This vampire gets first strike this round." A's
/// hand strike lands first: X, at 1 blood, goes to torpor and its strike is
/// lost (rulebook "First Strike").
#[test]
fn shadow_feint_strikes_first_this_round() {
    let (mut e, a, x) = duel(&["cel", "obf"], &[]);
    e.state.minions[x.index()].blood = 0;
    hand(&mut e, 0, &["Shadow Feint"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[x.index()].in_torpor);
    assert_eq!(blood(&e, a), 4 - 1, "its cost, no damage");
}

/// Shadow Feint [CEL][OBF]: "Does not prevent the opponent from dodging,
/// the dodge just has no effect." [LSJ 20030902-2] [LSJ 20060808-1] X's
/// Read Intentions [AUS] is played; A's hand strike still hits.
#[test]
fn shadow_feint_superior_strikes_through_a_dodge() {
    let (mut e, a, x) = duel(&["cel", "cel", "obf", "obf"], &["aus", "aus"]);
    hand(&mut e, 0, &["Shadow Feint"]);
    hand(&mut e, 1, &["Read Intentions"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 1, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4 - 1, "the dodge played, A's 1 landed");
    assert!(ash_has(&e, 1, "Read Intentions"));
}

// ---- Quick Jab --------------------------------------------------------------

/// Quick Jab: "Only one damage is inflicted means only one damage needs be
/// prevented." [LSJ 20071117] A at strength 3: 1 pending on X, with first
/// strike: X at 1 blood goes to torpor before striking. Replaced after
/// combat.
#[test]
fn quick_jab_strikes_first_for_one() {
    let (mut e, a, x) = duel(&[], &["for"]);
    stronger(&mut e, a, 2);
    hand(&mut e, 0, &["Quick Jab"]);
    hand(&mut e, 1, &["Armor of Vitality"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    offering(&mut e, play(0, 0, x));
    assert_eq!(pending(&e, 1), (1, 0));
    assert_eq!(e.state.seats[0].held[zvtes::state::Release::AfterCombat], 1);

    let (mut e, a, x) = duel(&[], &[]);
    stronger(&mut e, a, 2);
    e.state.minions[x.index()].blood = 0;
    hand(&mut e, 0, &["Quick Jab"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[x.index()].in_torpor);
    assert_eq!(blood(&e, a), 4, "X never struck");
}

/// Quick Jab: "If more than 1 damage is inflicted with this strike, ignore
/// the excess": Target Head's +2 too, X pending 1.
#[test]
fn quick_jab_caps_an_aimed_strike() {
    let (mut e, a, x) = duel(&[], &["for"]);
    hand(&mut e, 0, &["Quick Jab", "Target Head"]);
    hand(&mut e, 1, &["Armor of Vitality"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, a));
    offering(&mut e, play(0, 0, x));
    assert_eq!(pending(&e, 1), (1, 0));
}

// ---- Selective Silence ------------------------------------------------------

/// Selective Silence [qui]: "Only usable during the first round of combat.
/// Maneuver." Round 1 at close: A maneuvers to long, the hand strikes miss.
#[test]
fn selective_silence_maneuvers_in_the_first_round() {
    let (mut e, a, x) = duel(&["qui"], &[]);
    hand(&mut e, 0, &["Selective Silence"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, x)), (4, 4));
}

/// Selective Silence [QUI]: "The card cannot be played if the minion cannot
/// afford to burn the blood." [ANK 20210226] "Once the range is set, no
/// other effect can be used to reset the range that round … Skip the
/// Determine Range step for that round." [RTR 19970630] [ANK 20180720] Its
/// blood burned, an effect no cost reduction reaches (ANK 20210226): long
/// set, X's Fake Out never offered; at 0 blood, not offered.
#[test]
fn selective_silence_sets_the_range_for_blood() {
    let (mut e, a, x) = duel(&["qui", "qui"], &[]);
    hand(&mut e, 0, &["Selective Silence"]);
    hand(&mut e, 1, &["Fake Out"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 2, a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, x));
    assert_eq!(
        (blood(&e, a), blood(&e, x)),
        (4 - 1, 4),
        "its blood; long range"
    );

    let (mut e, a, x) = duel(&["qui", "qui"], &[]);
    hand(&mut e, 0, &["Selective Silence"]);
    bleed_blocked(&mut e, a, x);
    e.state.minions[a.index()].blood = 0;
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        *o == play(0, 1, a) || *o == play(0, 2, a)
    });
}

/// Selective Silence [QUI]: "If another effect was already setting that
/// round's range (e.g. Neutral Guard), that effect has priority." [PIB
/// 20120214] X's Cold Aura [nec] first: A's range setting not offered.
#[test]
fn selective_silence_yields_to_a_range_already_set() {
    let (mut e, a, x) = duel(&["qui", "qui"], &["nec"]);
    hand(&mut e, 0, &["Selective Silence"]);
    hand(&mut e, 1, &["Cold Aura"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, x));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        *o == play(0, 1, a) || *o == play(0, 2, a)
    });
    assert_eq!(
        (blood(&e, a), blood(&e, x)),
        (4, 4 - 1),
        "long; Cold Aura's cost"
    );
}

// ---- The Jones --------------------------------------------------------------

/// A of capacity `a_cap` with `a_levels`, X of `x_cap`: seat 0 and seat 1
/// each with a second vampire; X's seat with a library to replace from.
fn ages(a_cap: u8, a_levels: &[&'static str], x_cap: u8) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity == a_cap, 0), 4, false);
    e.add_minion(
        SeatId(0),
        textless(|c| (4..=8).contains(&c.capacity), 5),
        4,
        false,
    );
    let x = e.add_minion(SeatId(1), textless(|c| c.capacity == x_cap, 1), 4, false);
    e.add_minion(
        SeatId(1),
        textless(|c| (4..=8).contains(&c.capacity), 6),
        4,
        false,
    );
    e.state.minions[a.index()].gained_levels = a_levels.to_vec();
    e.state.seats[1].library = vec![find_library("Fake Out"); 12].into();
    (e, a, x)
}

/// The Jones [ser]: "Only usable in combat with an ally or younger vampire.
/// Strike: dodge, with 1 optional press, only usable to end combat." "The
/// optional press can only be used during the current round." [TOM
/// 19960521] Against a younger X: A dodges, then ends the combat X pressed
/// to continue. Against an older one, never offered.
#[test]
fn the_jones_dodges_with_an_ending_press_against_a_younger_vampire() {
    let (mut e, a, x) = ages(7, &["ser"], 4);
    e.state.minions[x.index()].gained_levels = vec!["pot"];
    hand(&mut e, 0, &["Jones, The"]);
    hand(&mut e, 1, &["Relentless Pursuit"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, x));
    pick_when(&mut e, Choice::UsePress);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4, "dodged, one round");

    let (mut e, a, x) = ages(4, &["ser"], 7);
    hand(&mut e, 0, &["Jones, The"]);
    bleed_blocked(&mut e, a, x);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
}

/// The Jones [SER]: "If the canceled card had a 'do not replace until'
/// clause …, that clause is canceled as well and the card is replaced
/// normally." [LSJ 20080630] [LSJ 20011023] "Cancel the opposing minion's
/// strike card as it is played, and its cost is not paid (the minion
/// chooses another strike)." X's Quick Jab canceled: X's hand full again at
/// once, Quick Jab in the ash heap; X strikes by hand.
#[test]
fn the_jones_cancels_quick_jab_which_is_replaced_at_once() {
    let (mut e, a, x) = ages(7, &["ser", "ser"], 4);
    hand(&mut e, 0, &["Jones, The"]);
    hand(&mut e, 1, &["Quick Jab", "Quick Jab"]);
    let size = e.state.hand_size(SeatId(1));
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, Choice::HandStrike);
    pick_when(&mut e, play(0, 0, x));
    pick_when(&mut e, play(0, 1, a));
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(1), WindowLabel::CombatStrike)
    );
    assert!(req.options.contains(&Choice::HandStrike));
    assert_eq!(e.state.seats[1].hand.len(), size, "replaced at once");
    assert!(ash_has(&e, 1, "Quick Jab"));
    pick(&mut e, Choice::HandStrike);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4 - 1, "X's hand strike");
}

/// The Jones [SER]: "A vampire can play only one The Jones at superior each
/// round." X's second strike card that round is not canceled.
#[test]
fn the_jones_superior_once_each_round() {
    let (mut e, a, x) = ages(7, &["ser", "ser"], 4);
    hand(&mut e, 0, &["Jones, The", "Jones, The"]);
    hand(&mut e, 1, &["Quick Jab", "Quick Jab"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, Choice::HandStrike);
    pick_when(&mut e, play(0, 0, x));
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 0, x));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, a));
}

// ---- The Khabar: Honor ------------------------------------------------------

/// A Banu Haqim A against X, and X's contract card in play choosing `chosen`.
fn khabar(contract: &str, chosen: bool) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(
            |c| c.clan == "Banu Haqim" && (4..=8).contains(&c.capacity),
            0,
        ),
        4,
        false,
    );
    e.add_minion(
        SeatId(0),
        textless(|c| (4..=8).contains(&c.capacity), 5),
        4,
        false,
    );
    let x = e.add_minion(
        SeatId(1),
        textless(|c| (4..=8).contains(&c.capacity), 6),
        4,
        false,
    );
    e.add_minion(
        SeatId(1),
        textless(|c| (4..=8).contains(&c.capacity), 7),
        4,
        false,
    );
    e.state.minions[x.index()].gained_levels = vec!["for"];
    let k = borne(&mut e, contract, x);
    if chosen {
        e.state.cards[k.index()].chosen = vec![a];
    }
    (e, a, x)
}

/// The Khabar: Honor: "Only usable if this Banu Haqim has been chosen for a
/// contract on the opposing minion. Strike: hand strike at +3 damage." X's
/// Clandestine Contract chose A: X has 4 pending; not chosen, not offered.
#[test]
fn khabar_honor_strikes_harder_against_its_contracts_minion() {
    let (mut e, a, x) = khabar("Clandestine Contract", true);
    hand(&mut e, 0, &["Khabar: Honor, The"]);
    hand(&mut e, 1, &["Armor of Vitality"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    offering(&mut e, play(0, 0, x));
    assert_eq!(pending(&e, 1), (4, 0));

    let (mut e, a, x) = khabar("Clandestine Contract", false);
    hand(&mut e, 0, &["Khabar: Honor, The"]);
    bleed_blocked(&mut e, a, x);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
}

/// The Khabar: Honor: "Additional damage inherits all of the properties of
/// the base damage." [TOM 19960225] A's aggravated hands: all 4 aggravated.
#[test]
fn khabar_honor_adds_of_the_hand_strikes_kind() {
    let (mut e, a, x) = khabar("Clandestine Contract", true);
    e.state.minions[a.index()].timed.push((
        zvtes::cards::Lasting::AggravatedHands,
        zvtes::state::Expiry::EndOfTurn,
        SeatId(0),
    ));
    hand(&mut e, 0, &["Khabar: Honor, The"]);
    hand(&mut e, 1, &["Armor of Vitality"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    offering(&mut e, play(0, 0, x));
    assert_eq!(pending(&e, 1), (0, 4));
}

/// Shakar: the Hunt: "Every Black Hand vampire is considered chosen for this
/// contract": A, Black Hand, may play The Khabar: Honor against its host.
#[test]
fn khabar_honor_counts_shakars_black_hand() {
    let (mut e, a, x) = khabar("Shakar: the Hunt", false);
    hand(&mut e, 0, &["Khabar: Honor, The"]);
    bleed_blocked(&mut e, a, x);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));

    let (mut e, a, x) = khabar("Shakar: the Hunt", false);
    e.state.minions[a.index()].gained_black_hand = true;
    hand(&mut e, 0, &["Khabar: Honor, The"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 0, "4 damage on 4 blood");
}

// ---- Mayaparisatya ----------------------------------------------------------

/// Mayaparisatya [chi]: "The opposing minion's weapons inflict no damage on
/// this vampire this combat." Played before range: X's Baseball Bat
/// strike deals A nothing; without it, 2.
#[test]
fn mayaparisatya_blunts_the_opposing_weapons_for_the_combat() {
    let (mut e, a, x) = duel(&["chi"], &[]);
    let bat = borne(&mut e, "Baseball Bat", x);
    hand(&mut e, 0, &["Mayaparisatya"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, card_use(bat, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4 - 2, "its cost, no damage");

    let (mut e, a, x) = duel(&["chi"], &[]);
    let bat = borne(&mut e, "Baseball Bat", x);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, card_use(bat, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4 - 2, "the bat's 2");
}

/// Mayaparisatya [CHI]: "Only usable at long range. Strike: 1R unpreventable
/// aggravated damage." At close, not offered; at long, X takes 1
/// aggravated, its Armor of Vitality never offered: to torpor.
#[test]
fn mayaparisatya_superior_strikes_unpreventably_at_long_range() {
    let (mut e, a, x) = duel(&["chi", "chi"], &[]);
    hand(&mut e, 0, &["Mayaparisatya"]);
    bleed_blocked(&mut e, a, x);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, a));

    let (mut e, a, x) = duel(&["chi", "chi"], &["for"]);
    hand(&mut e, 0, &["Fake Out", "Mayaparisatya"]);
    hand(&mut e, 1, &["Armor of Vitality"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 1, a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, x));
    assert!(e.state.minions[x.index()].in_torpor);
}

// ---- Revelation of Wrath ----------------------------------------------------

/// Revelation of Wrath [ser]: "Must be played after the presses are handled
/// and can be played if the round ends prematurely." [RTR 20030519] "Does
/// not allow to take control of the opponent." [ANK 20200925] [ANK
/// 20211105] X (strength 2) hits A for 2, all prevented: offered at the end
/// of the round, after X's press window; A puts its counter on X, X still
/// X's. X's 1 only: not offered.
#[test]
fn revelation_of_wrath_corrupts_after_a_round_of_two_damage() {
    let (mut e, a, x) = duel(&["ser", "for"], &[]);
    stronger(&mut e, x, 1);
    hand(&mut e, 0, &["Revelation of Wrath", "Armor of Vitality"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(1, 0, a));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatEndOfRound);
    assert!(req.options.contains(&play(0, 0, a)));
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[x.index()];
    assert_eq!(v.corruption_of(SeatId(0)), 1);
    assert_eq!(v.owner, SeatId(1), "no control taken");

    let (mut e, a, x) = duel(&["ser"], &[]);
    hand(&mut e, 0, &["Revelation of Wrath"]);
    bleed_blocked(&mut e, a, x);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
}

/// Revelation of Wrath [SER]: "put 1 additional corruption counter if the
/// opposing minion inflicted 3 or more damage or 2 or more aggravated
/// damage this round." X's aggravated hand strike of 1: the base counter
/// only; of 3 (strength 3): two.
#[test]
fn revelation_of_wrath_superior_adds_one_for_three() {
    for (more, counters) in [(0, 1), (2, 2)] {
        let (mut e, a, x) = duel(&["ser", "ser"], &[]);
        e.state.minions[a.index()].blood = 8;
        stronger(&mut e, x, more);
        e.state.minions[x.index()].timed.push((
            zvtes::cards::Lasting::AggravatedHands,
            zvtes::state::Expiry::EndOfTurn,
            SeatId(1),
        ));
        hand(&mut e, 0, &["Revelation of Wrath"]);
        bleed_blocked(&mut e, a, x);
        pick_when(&mut e, play(0, 1, a));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(
            e.state.minions[x.index()].corruption_of(SeatId(0)),
            counters
        );
    }
}

/// Revelation of Wrath: "Does not count environmental damage." [ANK
/// 20200925] X's hand strike's 1 and its Razor Bat's 1: not offered.
#[test]
fn revelation_of_wrath_ignores_retainer_and_environmental_damage() {
    let (mut e, a, x) = duel(&["ser"], &[]);
    borne(&mut e, "Razor Bat", x);
    hand(&mut e, 0, &["Revelation of Wrath"]);
    bleed_blocked(&mut e, a, x);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
    assert_eq!(blood(&e, a), 4 - 2, "both landed");
}

/// Revelation of Wrath: "Can be played by a vampire going to torpor."
/// [ANK 20201122] [ANK 20200926] A, at 1 blood, hit for 2 by X: in torpor,
/// still offered at the round's end.
#[test]
fn revelation_of_wrath_by_a_vampire_going_to_torpor() {
    let (mut e, a, x) = duel(&["ser"], &[]);
    stronger(&mut e, x, 1);
    hand(&mut e, 0, &["Revelation of Wrath"]);
    bleed_blocked(&mut e, a, x);
    e.state.minions[a.index()].blood = 1;
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[a.index()].in_torpor);
    assert_eq!(e.state.minions[x.index()].corruption_of(SeatId(0)), 1);
}

// ---- Sympathetic Agony ------------------------------------------------------

/// Sympathetic Agony [chi][for]: "if any damage is successfully inflicted on
/// this vampire in a round, the opposing minion cannot press that round."
/// X's hand strike lands: its Relentless Pursuit not offered at the press;
/// all prevented (Armor of Vitality), offered.
#[test]
fn sympathetic_agony_bars_the_press_after_a_hit() {
    let (mut e, a, x) = duel(&["chi", "for"], &["pot"]);
    hand(&mut e, 0, &["Sympathetic Agony"]);
    hand(&mut e, 1, &["Relentless Pursuit"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, x));

    let (mut e, a, x) = duel(&["chi", "for"], &["pot"]);
    hand(&mut e, 0, &["Sympathetic Agony", "Armor of Vitality"]);
    hand(&mut e, 1, &["Relentless Pursuit"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, a));
    offering(&mut e, play(0, 0, x));
}

/// Sympathetic Agony [CHI][FOR]: "The affected minion cannot play a card
/// that provides an additional strike, even if just to benefit from
/// another effect." [LSJ 20011214-5] X's hand strike lands: its Blur never
/// offered.
#[test]
fn sympathetic_agony_superior_bars_additional_strike_cards() {
    let (mut e, a, x) = duel(&["chi", "chi", "for", "for"], &["cel"]);
    hand(&mut e, 0, &["Sympathetic Agony"]);
    hand(&mut e, 1, &["Blur"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, x));

    let (mut e, a, x) = duel(&["chi", "chi", "for", "for"], &["cel"]);
    hand(&mut e, 0, &["Sympathetic Agony"]);
    hand(&mut e, 1, &["Blur"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    offering(&mut e, play(0, 0, x));
}

// ---- Forearm Block ----------------------------------------------------------

/// Forearm Block: "If fewer points of (preventable) damage are being
/// resolved, then the effect prevents all of those points." [RTR 20041202]
/// X's hand strike at strength 3: A takes 1; at strength 1, none.
#[test]
fn forearm_block_prevents_two_of_the_next_hand_strike() {
    for (more, taken) in [(2, 1), (0, 0)] {
        let (mut e, a, x) = duel(&[], &[]);
        stronger(&mut e, x, more);
        hand(&mut e, 0, &["Forearm Block"]);
        bleed_blocked(&mut e, a, x);
        pick_when(&mut e, play(0, 0, a));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, a), 4 - taken);
        assert_eq!(blood(&e, x), 4, "its strike deals none");
    }
}

/// Forearm Block: "Unused prevention points can't be carried over unless
/// optional ('can/may prevent')." [ANK 20200318] [LSJ 20001114] It waits
/// for X's next hand strike this round: X's Baseball Bat hits A for 2,
/// then X's additional hand strike (Blur) is prevented.
#[test]
fn forearm_block_waits_for_the_next_hand_strike_this_round() {
    let (mut e, a, x) = duel(&[], &["cel"]);
    let bat = borne(&mut e, "Baseball Bat", x);
    hand(&mut e, 0, &["Forearm Block"]);
    hand(&mut e, 1, &["Blur"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, card_use(bat, 0));
    pick_when(&mut e, play(0, 0, x));
    pick_when(&mut e, Choice::HandStrike);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        blood(&e, a),
        4 - 2,
        "the bat's 2, the hand strike's 1 prevented"
    );
}

/// Forearm Block: "If another round of combat occurs, this minion gets
/// first strike on his or her initial strike that round." A presses on;
/// in round 2 A's hand strike lands first, X at 0 blood goes to torpor
/// and never strikes.
#[test]
fn forearm_block_gives_first_strike_next_round() {
    let (mut e, a, x) = duel(&["pot"], &[]);
    hand(&mut e, 0, &["Forearm Block", "Relentless Pursuit"]);
    bleed_blocked(&mut e, a, x);
    e.state.minions[x.index()].blood = 0;
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[x.index()].in_torpor);
    assert_eq!(
        blood(&e, a),
        4 - 1,
        "Relentless Pursuit's blood; X never hit"
    );
}

// ---- Baal's Bloody Talons ---------------------------------------------------

/// Baal's Bloody Talons [QUI]: "The ability to make the damage from the
/// vampire's strikes aggravated that round must be used before strike
/// resolution and damage prevention in order to affect the current
/// strike. Can be played after both strikes have been chosen." [RTR
/// 19960112] A's Baseball Bat's 2, aggravated, +1 aggravated: 3 pending
/// on X; the bat burned as the round ends.
#[test]
fn baals_bloody_talons_aggravates_this_weapons_strike_and_burns_it() {
    let (mut e, a, x) = duel(&["qui", "qui"], &["for"]);
    let bat = borne(&mut e, "Baseball Bat", a);
    hand(&mut e, 0, &["Baal's Bloody Talons"]);
    hand(&mut e, 1, &["Armor of Vitality"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, card_use(bat, 0));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatBeforeResolution);
    assert!(req.options.contains(&play(0, 1, a)));
    pick(&mut e, play(0, 1, a));
    offering(&mut e, play(0, 0, x));
    assert_eq!(pending(&e, 1), (0, 3));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[bat.index()].in_play);
    assert!(ash_has(&e, 0, "Baseball Bat"));
}

/// Baal's Bloody Talons: "'Before resolution' means after strikes have been
/// declared but before they resolve." [RTR 19990105] "Cannot be played on
/// an opponent's weapon." [LSJ 20020425] Not before A's strike is chosen;
/// A striking by hand against X's Baseball Bat, never.
#[test]
fn baals_bloody_talons_needs_the_users_melee_weapon_strike() {
    let (mut e, a, x) = duel(&["qui"], &[]);
    borne(&mut e, "Baseball Bat", a);
    let theirs = borne(&mut e, "Baseball Bat", x);
    hand(&mut e, 0, &["Baal's Bloody Talons"]);
    bleed_blocked(&mut e, a, x);
    never_offered_where(&mut e, SeatId(0), WindowLabel::CombatStrike, |o| {
        *o == play(0, 0, a)
    });
    let req = request(&mut e);
    e.choose(index_of(&req, Choice::HandStrike));
    pick_when(&mut e, card_use(theirs, 0));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
}

/// Baal's Bloody Talons: "The weapon does not burn if it is temporarily out
/// of play (eg. Kerrie, Dagger)." [LSJ 20100211-2] A's Dagger's ranged
/// strike: out of play as the round ends, not burned, back on A as the
/// action ends.
#[test]
fn baals_bloody_talons_spares_a_weapon_out_of_play() {
    let (mut e, a, x) = duel(&["qui"], &[]);
    let dagger = borne(&mut e, "Dagger", a);
    hand(&mut e, 0, &["Baal's Bloody Talons"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, card_use(dagger, 1));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let d = &e.state.cards[dagger.index()];
    assert!(d.in_play && !d.out_of_play && d.host == Some(a));
    assert!(
        e.state.minions[x.index()].in_torpor,
        "its ranged strike's 1, aggravated"
    );
}

// ---- Strike at the True Flesh -----------------------------------------------

/// Strike at the True Flesh [QUI]: "'Before resolution' means after strikes
/// have been declared but before they resolve." [RTR 19990105] "Cannot be
/// played on an opponent's weapon." [LSJ 20020425] "Additional damage
/// inherits all of the properties of the base damage." [RTR 19970630] [TOM
/// 19960225] A's Baseball Bat's 2 +1, normal: X's Armor of Vitality never
/// offered, its Flak Jacket (equipment) prevents 1: X takes 2.
#[test]
fn strike_at_the_true_flesh_bars_card_prevention() {
    let (mut e, a, x) = duel(&["qui", "qui"], &["for"]);
    let bat = borne(&mut e, "Baseball Bat", a);
    let jacket = borne(&mut e, "Flak Jacket", x);
    hand(&mut e, 0, &["Strike at the True Flesh"]);
    hand(&mut e, 1, &["Armor of Vitality"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, card_use(bat, 0));
    pick_when(&mut e, play(0, 1, a));
    let req = until(&mut e, SeatId(1), WindowLabel::CombatPrevention);
    assert_eq!(pending(&e, 1), (3, 0));
    assert!(!req.options.contains(&play(0, 0, x)), "{:?}", req.options);
    pick(&mut e, card_use(jacket, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4 - 2);
}

/// Strike at the True Flesh (owner's reading asked): "except by equipment or
/// by other strikes" bars a prevention allowance from a card played
/// earlier: X's Apparition [chi], played before range, prevents none of
/// A's Baseball Bat's 2.
#[test]
fn strike_at_the_true_flesh_bars_apparition() {
    let (mut e, a, x) = duel(&["qui"], &["chi"]);
    let bat = borne(&mut e, "Baseball Bat", a);
    hand(&mut e, 0, &["Strike at the True Flesh"]);
    hand(&mut e, 1, &["Apparition"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, x));
    pick_when(&mut e, card_use(bat, 0));
    pick_when(&mut e, play(0, 0, a));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        *o == Choice::UsePrevent
    });
    assert_eq!(blood(&e, x), 4 - 1 - 2, "Apparition's cost, the bat's 2");
}

/// Strike at the True Flesh: another strike's prevention reaches it (Stunt
/// Cycle [pot]'s 1): A's Dagger's ranged strike at long range, 1 +1 [QUI],
/// 1 prevented by X's strike.
#[test]
fn strike_at_the_true_flesh_yields_to_a_strike_shield() {
    let (mut e, a, x) = duel(&["qui", "qui"], &["pot"]);
    let dagger = borne(&mut e, "Dagger", a);
    hand(&mut e, 0, &["Fake Out", "Strike at the True Flesh"]);
    hand(&mut e, 1, &["Stunt Cycle"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, card_use(dagger, 1));
    pick_when(&mut e, play(0, 0, x));
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4 - 1, "2 less 1");
    assert!(
        !e.state.cards[dagger.index()].in_play,
        "its damage prevented"
    );
}

// ---- Cobra Fangs ------------------------------------------------------------

/// Seat 0's A (`levels`) bleeds, seat 1's ECTU Operative (3 life) blocks.
fn ally_blocks(levels: &[&'static str]) -> (Engine, MinionId, MinionId) {
    let (mut e, a, _) = duel(levels, &[]);
    let ally = e.add_ally(SeatId(1), find_library("ECTU Operative"));
    bleed_blocked(&mut e, a, ally);
    (e, a, ally)
}

/// Cobra Fangs [ser]: "If any damage from this strike is successfully
/// inflicted on an opposing ally, burn that ally after action resolution."
/// The ally, hit for 1 of its 3 life, stays through the combat and burns
/// as the action ends.
#[test]
fn cobra_fangs_burns_a_struck_ally_after_resolution() {
    let (mut e, a, ally) = ally_blocks(&["ser"]);
    hand(&mut e, 0, &["Cobra Fangs"]);
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[ally.index()].in_play);
    assert!(ash_has(&e, 1, "ECTU Operative"));
}

/// Cobra Fangs [ser]: none successfully inflicted (the ally's Forearm Block
/// prevents it), the ally stays.
#[test]
fn cobra_fangs_spares_an_ally_when_prevented() {
    let (mut e, a, ally) = ally_blocks(&["ser"]);
    hand(&mut e, 0, &["Cobra Fangs"]);
    hand(&mut e, 1, &["Forearm Block"]);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, ally));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[ally.index()].present());
}

/// Cobra Fangs [SER]: "if any damage from this strike is successfully
/// inflicted on an opposing vampire, the opposing vampire goes to torpor
/// during their next unlock phase." X, hit for 1, ready after the action;
/// in seat 1's next unlock phase it goes to torpor, the effect gone.
#[test]
fn cobra_fangs_superior_sends_a_struck_vampire_to_torpor_at_its_unlock() {
    let (mut e, a, x) = duel(&["ser", "ser"], &[]);
    hand(&mut e, 0, &["Cobra Fangs"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[x.index()].ready());
    assert!(ash_has(&e, 0, "Cobra Fangs"), "its effect alone on X");
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.minions[x.index()].in_torpor);
    let fangs = find_library("Cobra Fangs");
    assert!(!e.state.cards.iter().any(|c| c.card == fangs && c.present()));
}

// ---- Scorpion's Touch -------------------------------------------------------

/// Scorpion's Touch [qui]: "put this card on the opposing minion": as A's
/// hand strike lands, the card itself on X, out of A's ash heap; X's own
/// strike that resolution unaffected (A takes 1).
#[test]
fn scorpions_touch_lands_on_the_opposing_minion() {
    let (mut e, a, x) = duel(&["qui"], &[]);
    hand(&mut e, 0, &["Scorpion's Touch"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "Scorpion's Touch", Some(x));
    assert_eq!((blood(&e, a), blood(&e, x)), (4 - 1 - 1, 4 - 1));
}

/// Scorpion's Touch: "If it is declared with first strike and if the
/// opposing minion declared a strike based on strength, that minion's
/// strength is decreased by Scorpion's Touch when it resolves." [PIB
/// 20110830] Under Shadow Feint, A's strike lands first: X's hand strike
/// at strength 0, A takes none.
#[test]
fn scorpions_touch_with_first_strike_weakens_the_opposing_strike() {
    let (mut e, a, x) = duel(&["cel", "obf", "qui"], &[]);
    hand(&mut e, 0, &["Shadow Feint", "Scorpion's Touch"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "Scorpion's Touch", Some(x));
    assert_eq!(blood(&e, a), 4 - 1 - 1, "the two cards' blood, no damage");
}

/// Scorpion's Touch: "use a melee weapon strike": A's Baseball Bat, 2
/// damage, and the card on X.
#[test]
fn scorpions_touch_through_a_melee_weapon() {
    let (mut e, a, x) = duel(&["qui"], &[]);
    let bat = borne(&mut e, "Baseball Bat", a);
    hand(&mut e, 0, &["Scorpion's Touch"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play_on(0, 1, a, Target::Card(bat)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "Scorpion's Touch", Some(x));
    assert_eq!(blood(&e, x), 4 - 2);
}

/// Scorpion's Touch (rules-qa 99.2): "A minion can have only one": on X
/// with one, the second is played, not put on, to the ash heap.
#[test]
fn a_second_scorpions_touch_is_not_put_on() {
    let (mut e, a, x) = duel(&["qui"], &[]);
    borne(&mut e, "Scorpion's Touch", x);
    hand(&mut e, 0, &["Scorpion's Touch"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let st = find_library("Scorpion's Touch");
    assert_eq!(
        e.state
            .cards
            .iter()
            .filter(|c| c.card == st && c.present())
            .count(),
        1
    );
    assert!(ash_has(&e, 0, "Scorpion's Touch"));
    assert_eq!(blood(&e, x), 4 - 1, "the strike landed");
}

/// Scorpion's Touch [QUI]: "the attached minion burns 1 blood or life
/// during their unlock phase"; [qui]'s does not. "Any vampire can burn
/// this card as a +1 stealth action": offered to X's own vampire.
#[test]
fn scorpions_touch_superior_burns_blood_at_unlock() {
    for (mode, burned) in [(2, 1), (0, 0)] {
        let (mut e, a, x) = duel(&["qui", "qui"], &[]);
        hand(&mut e, 0, &["Scorpion's Touch"]);
        bleed_blocked(&mut e, a, x);
        pick_when(&mut e, play(0, mode, a));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let before = blood(&e, x);
        let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, x), before - burned);
        let st = e
            .state
            .cards
            .iter()
            .position(|c| c.card == find_library("Scorpion's Touch") && c.present())
            .expect("on X") as u32;
        assert!(!uses(&req, st).is_empty(), "its burn action");
    }
}

// ---- Unholy Penance ---------------------------------------------------------

/// Unholy Penance [PRE]: "The additional effect takes place after the end
/// of combat." [RTR 19970630] At long range, the card on X as the combat
/// ends; A's bleeds against X's Methuselah +1, another vampire's not.
#[test]
fn unholy_penance_goes_on_after_combat_at_any_range() {
    let (mut e, a, x) = duel(&["pre", "pre"], &[]);
    hand(&mut e, 0, &["Fake Out", "Unholy Penance"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "Unholy Penance", Some(x));
    assert_eq!(e.state.bleed_against_mod(a, SeatId(1)), 1);
    let other = e
        .state
        .minions_of(SeatId(0))
        .find(|&m| m != a)
        .expect("second");
    assert_eq!(e.state.bleed_against_mod(other, SeatId(1)), 0);
}

/// Unholy Penance [PRE]: "If the combat continues or a new combat begins,
/// the after combat effect does not happen" [RTR 20020501] [LSJ 19980109]:
/// X's Telepathic Tracking continues it; X's Dodge, the card never placed.
#[test]
fn unholy_penance_is_lost_if_dodged_or_the_combat_continues() {
    for (levels, card, mode) in [
        (vec!["aus", "aus"], "Telepathic Tracking", 1),
        (vec![], "Dodge", 0),
    ] {
        let (mut e, a, x) = duel(&["pre", "pre"], &[]);
        e.state.minions[x.index()].gained_levels = levels;
        hand(&mut e, 0, &["Unholy Penance"]);
        hand(&mut e, 1, &[card]);
        bleed_blocked(&mut e, a, x);
        pick_when(&mut e, play(0, 1, a));
        pick_when(&mut e, play(0, mode, x));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let up = find_library("Unholy Penance");
        assert!(
            !e.state.cards.iter().any(|c| c.card == up && c.present()),
            "{card}"
        );
        assert!(ash_has(&e, 0, "Unholy Penance"));
    }
}

/// Unholy Penance [PRE]: "This minion may burn this card as a +1 stealth
/// action": offered to X, not to X's other vampire.
#[test]
fn unholy_penance_is_burned_by_its_minion() {
    let (mut e, a, x) = duel(&["pre", "pre"], &[]);
    hand(&mut e, 0, &["Unholy Penance"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let up = e
        .state
        .cards
        .iter()
        .position(|c| c.card == find_library("Unholy Penance") && c.present())
        .expect("on X") as u32;
    let offered = uses(&req, up);
    assert!(!offered.is_empty());
    assert!(offered.iter().all(|o| matches!(o,
        Choice::UseAbility { actor, .. } if *actor == Some(x) || actor.is_none())));
}

// ---- Fata Amria -------------------------------------------------------------

/// Fata Amria [chi]: "Does not prevent the opponent from dodging, the dodge
/// just has no effect." [LSJ 20030902-2] [LSJ 20060808-1] X's Dodge
/// played: the card itself goes on X, out of A's ash heap.
#[test]
fn fata_amria_strikes_through_a_dodge_and_goes_on_the_vampire() {
    let (mut e, a, x) = duel(&["chi"], &[]);
    hand(&mut e, 0, &["Fata Amria"]);
    hand(&mut e, 1, &["Dodge"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "Fata Amria", Some(x));
    assert!(ash_has(&e, 1, "Dodge"));
}

/// Fata Amria: "The attached vampire cannot play cards requiring any
/// superior Disciplines": X's Armor of Vitality [FOR] not offered, its
/// [for] offered.
#[test]
fn fata_amria_bars_superior_plays() {
    let (mut e, a, x) = duel(&[], &["for", "for"]);
    borne(&mut e, "Fata Amria", x);
    hand(&mut e, 1, &["Armor of Vitality"]);
    bleed_blocked(&mut e, a, x);
    let req = offering(&mut e, play(0, 0, x));
    assert!(!req.options.contains(&play(0, 1, x)));
}

/// Fata Amria: "The target can burn it at unlock even if they're already
/// unlocked, but cannot burn it if they do not unlock as normal (eg.
/// Sensory Deprivation)." [LSJ 20010814-3] [LSJ 20020109] X, unlocked, is
/// asked and burns it, staying unlocked; X locked, passing, unlocks; X not
/// unlocking as normal, not asked, still locked.
#[test]
fn fata_amria_burns_instead_of_unlocking_unless_prevented() {
    let (mut e, _, x) = duel(&[], &[]);
    let fata = borne(&mut e, "Fata Amria", x);
    let req = until(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
    assert_eq!(
        req.options,
        vec![Choice::Pass, Choice::BurnInPlay { card: fata.0 }]
    );
    pick(&mut e, Choice::BurnInPlay { card: fata.0 });
    request(&mut e);
    assert!(!e.state.cards[fata.index()].in_play);
    assert!(!e.state.minions[x.index()].locked);

    let (mut e, _, x) = duel(&[], &[]);
    let fata = borne(&mut e, "Fata Amria", x);
    e.state.minions[x.index()].locked = true;
    until(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
    pick(&mut e, Choice::Pass);
    request(&mut e);
    assert!(e.state.cards[fata.index()].in_play);
    assert!(!e.state.minions[x.index()].locked);

    let (mut e, _, x) = duel(&[], &[]);
    let fata = borne(&mut e, "Fata Amria", x);
    e.state.minions[x.index()].locked = true;
    e.state.minions[x.index()].skip_unlock = true;
    let unlocked = to_minion_phase(&mut e, SeatId(1));
    assert_eq!(unlocked.seat, SeatId(1));
    assert!(e.state.cards[fata.index()].in_play);
}

/// Fata Amria [CHI]: "Is played after resolution, after all combats (if
/// any) are handled, but still during the action." [PIB 20150915] [LSJ
/// 19981028] [ANK 20190425] A's successful bleed of seat 1: X puts it on A
/// after resolution; blocked, not offered.
#[test]
fn fata_amria_reaction_goes_on_the_acting_vampire_after_resolution() {
    let (mut e, a, x) = duel(&[], &["chi", "chi"]);
    hand(&mut e, 1, &["Fata Amria"]);
    pick(&mut e, bleed(a));
    let req = offering(&mut e, play(0, 1, x));
    assert_eq!(req.window, WindowLabel::ActionAfterResolution);
    e.choose(index_of(&req, play(0, 1, x)));
    request(&mut e);
    placed_on(&e, "Fata Amria", Some(a));

    let (mut e, a, x) = duel(&[], &["chi", "chi"]);
    hand(&mut e, 1, &["Fata Amria"]);
    let other = e
        .state
        .minions_of(SeatId(1))
        .find(|&m| m != x)
        .expect("second");
    bleed_blocked(&mut e, a, other);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, x));
}

// ---- Target Hand, Target Head, Target Leg -----------------------------------

/// Target Hand: "If any damage from this strike is successfully inflicted
/// on the opposing minion, he or she gets -1 strength this action, and you
/// may destroy a weapon he or she has." A's hand strike lands: X -1
/// strength for the action, A burns X's Baseball Bat.
#[test]
fn target_hand_weakens_and_disarms_on_a_hit() {
    let (mut e, a, x) = duel(&[], &[]);
    let bat = borne(&mut e, "Baseball Bat", x);
    hand(&mut e, 0, &["Target Hand"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert_eq!(
        req.options,
        vec![Choice::Pass, Choice::BurnInPlay { card: bat.0 }]
    );
    assert!(e.state.minions[x.index()]
        .timed
        .iter()
        .any(|&(l, until, _)| {
            l == zvtes::cards::Lasting::Strength(-1) && until == zvtes::state::Expiry::Action
        }));
    pick(&mut e, Choice::BurnInPlay { card: bat.0 });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 1, "Baseball Bat"));
    assert!(
        e.state.minions[x.index()].timed.is_empty(),
        "the action over"
    );
}

/// Target Hand, prevented whole (X's Armor of Vitality): no clause.
#[test]
fn target_hand_needs_damage_successfully_inflicted() {
    let (mut e, a, x) = duel(&[], &["for"]);
    let bat = borne(&mut e, "Baseball Bat", x);
    hand(&mut e, 0, &["Target Hand"]);
    hand(&mut e, 1, &["Armor of Vitality"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, x));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::BurnInPlay { card: bat.0 },
    );
    assert!(e.state.cards[bat.index()].in_play);
}

/// Target Head: "The strike does +2 damage … he or she cannot use any
/// additional strikes or presses this round." "The affected minion cannot
/// play a card that provides an additional strike." [LSJ 20011214-5]
/// "Additional damage inherits all of the properties of the base damage."
/// [RTR 19970630] [TOM 19960225] A's aggravated hand
/// strike +2: 3 aggravated pending; X's Blur and press never offered.
#[test]
fn target_head_adds_two_and_bars_presses_and_additional_strikes() {
    let (mut e, a, x) = duel(&[], &["for", "cel"]);
    e.state.minions[a.index()].timed.push((
        zvtes::cards::Lasting::AggravatedHands,
        zvtes::state::Expiry::EndOfTurn,
        SeatId(0),
    ));
    hand(&mut e, 0, &["Target Head"]);
    hand(&mut e, 1, &["Armor of Vitality", "Blur"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    offering(&mut e, play(0, 0, x));
    assert_eq!(pending(&e, 1), (0, 3));

    let (mut e, a, x) = duel(&[], &["cel", "pot"]);
    e.state.minions[x.index()].blood = 8;
    hand(&mut e, 0, &["Target Head"]);
    hand(&mut e, 1, &["Blur", "Relentless Pursuit"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        *o == play(0, 0, x) || *o == play(1, 0, x)
    });
}

/// Target Head: "Can be played on a strike that does no damage, even a
/// dodge or a combat ends, but has no effect in that case." [RTR 19960221]
/// [PIB 20130319] On A's Dodge: X takes nothing, Target Head spent.
#[test]
fn target_head_adds_nothing_to_a_dodge() {
    let (mut e, a, x) = duel(&[], &[]);
    hand(&mut e, 0, &["Dodge", "Target Head"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, x)), (4, 4));
    assert!(ash_has(&e, 0, "Target Head"));
}

/// Target Head: "you may set the range for the next round." "Once the range
/// is set, no other effect can be used to reset the range that round …
/// Skip the Determine Range step for that round." [RTR 19970630] [ANK
/// 20180720] "If another effect was already setting that round's range
/// …, that effect has priority." [PIB 20120214] A sets it long and presses
/// on: round 2 at long range, X's Fake Out never offered, no hand strike
/// landing.
#[test]
fn target_head_sets_the_next_rounds_range() {
    let (mut e, a, x) = duel(&["pot"], &[]);
    hand(&mut e, 0, &["Target Head", "Relentless Pursuit"]);
    hand(&mut e, 1, &["Fake Out"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert_eq!(
        req.options,
        vec![Choice::Pass, Choice::SetX(0), Choice::SetX(1)]
    );
    assert_eq!(blood(&e, x), 4 - 3, "round 1's 1 + 2");
    pick(&mut e, Choice::SetX(1));
    pick_when(&mut e, play(0, 0, a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, x));
    assert_eq!(blood(&e, x), 4 - 3, "none at long range");
    assert_eq!(blood(&e, a), 4 - 1 - 1, "round 1's 1, Relentless Pursuit");
}

/// Target Leg: "he or she may use maneuvers or presses only if they
/// require Obfuscate, Blood Sorcery or Flight this action." Hit, X's
/// Relentless Pursuit press is barred; in the next round (A presses on),
/// its Pursuit [cel] maneuver too, its Swallowed by the Night [OBF] offered.
#[test]
fn target_leg_hobbles_maneuvers_and_presses() {
    let (mut e, a, x) = duel(&["pot"], &["cel", "obf", "obf", "pot"]);
    e.state.minions[x.index()].blood = 8;
    hand(&mut e, 0, &["Target Leg", "Relentless Pursuit"]);
    hand(
        &mut e,
        1,
        &["Pursuit", "Swallowed by the Night", "Relentless Pursuit"],
    );
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    // A's press first, X's press to end barred.
    pick_when(&mut e, play(0, 0, a));
    never_offered_where(&mut e, SeatId(1), WindowLabel::CombatDetermineRange, |o| {
        *o == play(2, 0, x)
    });
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, x)), "{:?}", req.options);
    assert!(req.options.contains(&play(1, 1, x)), "{:?}", req.options);
    assert_eq!(e.combat().expect("combat").round, 2);
}

/// Target Leg under Hunter's Mark [cel][tha]: the press it gave A requires
/// Blood Sorcery, so A, hit by X's Target Leg, still uses it (rules-qa
/// 121.161), and the combat goes on.
#[test]
fn target_leg_spares_a_press_from_a_card_requiring_blood_sorcery() {
    let (mut e, a, x) = duel(&["cel", "tha"], &[]);
    hand(&mut e, 0, &["Hunter's Mark"]);
    hand(&mut e, 1, &["Target Leg"]);
    pick(&mut e, play_on(0, 0, a, Target::Minion(x)));
    pick_when(&mut e, play(0, 0, x));
    let req = offering(&mut e, Choice::UsePress);
    assert_eq!(req.seat, SeatId(0));
    assert!(e.state.hobbled(a).is_some(), "hit");
    e.choose(index_of(&req, Choice::UsePress));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4 - 2, "a second round");
}

/// Target Hand, Head, Leg: "Is played during the Strike phase even if you
/// commit to a strike in the Determine Range phase (eg. Thrown Gate).
/// Cannot be played if the strike cannot be chosen anymore during the
/// Strike phase (eg. Immortal Grapple)." [LSJ 20080409] X's Thrown Gate,
/// committed at the maneuver, takes each (Target Head's +2); A back at
/// close range and grappling, none.
#[test]
fn target_aims_take_a_committed_strike_unless_a_grapple_bars_it() {
    for (aim, damage) in [("Target Hand", 1), ("Target Head", 3), ("Target Leg", 1)] {
        let (mut e, a, x) = duel(&[], &["pot"]);
        hand(&mut e, 1, &["Thrown Gate", aim]);
        bleed_blocked(&mut e, a, x);
        pick_when(&mut e, play(0, 0, x));
        pick_when(&mut e, play(0, 0, x));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, a), 4 - damage, "{aim}");
        assert!(ash_has(&e, 1, aim));

        let (mut e, a, x) = duel(&["pot"], &["pot"]);
        hand(&mut e, 0, &["Fake Out", "Immortal Grapple"]);
        hand(&mut e, 1, &["Thrown Gate", aim]);
        bleed_blocked(&mut e, a, x);
        pick_when(&mut e, play(0, 0, x));
        pick_when(&mut e, play(0, 0, a));
        pick_when(&mut e, play(0, 0, a));
        never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, x));
        assert_eq!(e.state.seats[1].hand.len(), 1, "{aim}: no aim played");
    }
}

/// Target Hand, Head, Leg: "If it is canceled, any blood cost (eg. Terror
/// Frenzy) has to be paid." [PIB 20121031] Under X's Terror Frenzy [ANI],
/// X discards the combat cards to cancel (two, one for Target Head): A
/// pays 1.
#[test]
fn target_aims_canceled_their_cost_paid() {
    for (aim, cards) in [
        (
            "Target Hand",
            &["Terror Frenzy", "Side Strike", "Glancing Blow"][..],
        ),
        ("Target Head", &["Terror Frenzy", "Side Strike"][..]),
        (
            "Target Leg",
            &["Terror Frenzy", "Side Strike", "Glancing Blow"][..],
        ),
    ] {
        let (mut e, a, x) = duel(&[], &["ani", "ani"]);
        hand(&mut e, 0, &[aim]);
        hand(&mut e, 1, cards);
        bleed_blocked(&mut e, a, x);
        pick_when(&mut e, play(0, 1, x));
        pick_when(&mut e, play(0, 0, a));
        pick_when(&mut e, Choice::PayToCancel);
        request(&mut e);
        assert_eq!(e.state.seats[1].hand.len(), 0, "{aim}: the combat cards");
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, x), 4 - 1 - 1, "{aim}: Terror Frenzy, A's strike");
        assert_eq!(blood(&e, a), 4 - 1 - 1, "{aim}: its cost, X's strike");
    }
}

// ---- Gemini's Mirror --------------------------------------------------------

/// A and X as `duel`, the game's seed `seed`.
fn duel_seeded(seed: u64, a_levels: &[&'static str]) -> (Engine, MinionId, MinionId) {
    let mut e = Engine::bare_seeded(seed, 2);
    for s in &mut e.state.seats {
        s.pool = 10;
    }
    e.state.turn = 4;
    let mid = |c: &zvtes::crypt::CryptCard| (4..=8).contains(&c.capacity);
    let a = e.add_minion(SeatId(0), textless(mid, 0), 4, false);
    e.add_minion(SeatId(0), textless(mid, 1), 4, false);
    let x = e.add_minion(SeatId(1), textless(mid, 2), 4, false);
    e.add_minion(SeatId(1), textless(mid, 3), 4, false);
    e.state.minions[a.index()].gained_levels = a_levels.to_vec();
    (e, a, x)
}

/// Gemini's Mirror [obf]: "The optional maneuver cannot be used if the
/// strike cannot be used (eg. Hidden Lurker)." [LSJ 20021028] Played at
/// the maneuver, it commits A's dodge (X's hand strike misses at long
/// range); under A's Eyes of Blades [AUS][CEL], never offered.
#[test]
fn geminis_mirror_maneuver_commits_the_dodge_unless_strikes_are_barred() {
    let (mut e, a, x) = duel(&["obf"], &[]);
    hand(&mut e, 0, &["Gemini's Mirror"]);
    bleed_blocked(&mut e, a, x);
    let req = until(&mut e, SeatId(0), WindowLabel::CombatDetermineRange);
    assert!(req.options.contains(&play(0, 0, a)));
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, x)), (4 - 1, 4));

    let (mut e, a, x) = duel(&["aus", "aus", "cel", "cel", "obf"], &[]);
    hand(&mut e, 0, &["Eyes of Blades", "Gemini's Mirror"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
}

/// Gemini's Mirror [OBF]: "For each strike made against this vampire, flip
/// a coin. If it's tails, the strike has no effect on this vampire. This
/// lasts until heads is flipped." Over seeds, X's strength-2 hand strike
/// either has no effect, the mirror kept, or lands, the mirror gone; both
/// happen.
#[test]
fn geminis_mirror_flips_for_each_strike_until_heads() {
    let mut seen = [false; 2];
    for seed in 0..40 {
        let (mut e, a, x) = duel_seeded(seed, &["obf", "obf", "pot"]);
        stronger(&mut e, x, 1);
        hand(&mut e, 0, &["Gemini's Mirror", "Relentless Pursuit"]);
        bleed_blocked(&mut e, a, x);
        pick_when(&mut e, play(0, 1, a));
        until(&mut e, SeatId(0), WindowLabel::CombatPress);
        let mirror = e.combat().expect("combat").sides[0].mirror;
        let lost = 4 - 1 - blood(&e, a);
        assert_eq!(lost, if mirror { 0 } else { 2 }, "seed {seed}");
        seen[usize::from(mirror)] = true;
    }
    assert_eq!(seen, [true, true]);
}

/// Gemini's Mirror: "Does not protect the minion's equipment, retainers, or
/// any other cards on him. Just him and his blood." [LSJ 20031118] X's
/// Heroic Might strike burns A's Baseball Bat whatever the seed.
#[test]
fn geminis_mirror_spares_the_vampire_not_its_equipment() {
    for seed in 0..8 {
        let (mut e, a, x) = duel_seeded(seed, &["obf", "obf"]);
        let bat = borne(&mut e, "Baseball Bat", a);
        let might = borne(&mut e, "Heroic Might", x);
        hand(&mut e, 0, &["Gemini's Mirror"]);
        bleed_blocked(&mut e, a, x);
        pick_when(&mut e, play(0, 1, a));
        pick_when(
            &mut e,
            Choice::UseAbility {
                source: AbilitySource::Card(might),
                ability: 0,
                actor: None,
                target: Target::Card(bat),
            },
        );
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(!e.state.cards[bat.index()].in_play, "seed {seed}");
    }
}

/// Gemini's Mirror [OBF] (owner's reading asked): a strike: combat ends
/// against the vampire flips too; tails, it has no effect and the combat
/// goes on (A's hand strike lands on X), heads, the combat ends (none).
#[test]
fn geminis_mirror_tails_voids_a_combat_ends_strike() {
    let mut seen = [false; 2];
    for seed in 0..40 {
        let (mut e, a, x) = duel_seeded(seed, &["obf", "obf"]);
        e.state.minions[x.index()].gained_levels = vec!["ani", "ani"];
        hand(&mut e, 0, &["Gemini's Mirror"]);
        hand(&mut e, 1, &["Flesh Bond"]);
        bleed_blocked(&mut e, a, x);
        pick_when(&mut e, play(0, 1, a));
        pick_when(&mut e, play(0, 1, x));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let hit = blood(&e, x) == 4 - 1 - 1;
        seen[usize::from(hit)] = true;
    }
    assert_eq!(seen, [true, true]);
}

/// Gemini's Mirror: "A vampire can play only one Gemini's Mirror at
/// superior each combat."
#[test]
fn geminis_mirror_superior_once_each_combat() {
    let (mut e, a, x) = duel(&["obf", "obf"], &[]);
    hand(&mut e, 0, &["Gemini's Mirror", "Gemini's Mirror"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, a));
}

// ---- Eyes of Blades ---------------------------------------------------------

/// Eyes of Blades [aus][cel]: "Strike: dodge, and this vampire may prevent 1
/// damage from a retainer." X's hand strike dodged; its Razor Bat's 1
/// prevented.
#[test]
fn eyes_of_blades_dodges_and_prevents_a_retainers_damage() {
    let (mut e, a, x) = duel(&["aus", "cel"], &[]);
    borne(&mut e, "Razor Bat", x);
    hand(&mut e, 0, &["Eyes of Blades"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::UsePrevent);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4);
}

/// Eyes of Blades [AUS][CEL]: "Minions that cannot strike, cannot play or
/// use the maneuver from a strike." [LSJ 20001126] This round X's strikes
/// and retainers have no effect on A or its cards: X's Razor Bat deals
/// none; X's Heroic Might strike burns none of A's Baseball Bat; A strikes
/// not (X untouched).
#[test]
fn eyes_of_blades_superior_voids_strikes_and_retainers() {
    let (mut e, a, x) = duel(&["aus", "aus", "cel", "cel"], &[]);
    let bat = borne(&mut e, "Baseball Bat", a);
    borne(&mut e, "Razor Bat", x);
    let might = borne(&mut e, "Heroic Might", x);
    hand(&mut e, 0, &["Eyes of Blades"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Card(might),
            ability: 0,
            actor: None,
            target: Target::Card(bat),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.cards[bat.index()].in_play);
    assert_eq!((blood(&e, a), blood(&e, x)), (4, 4));
}

/// Eyes of Blades [AUS][CEL] (rules-qa 113.120's reading): X's strike:
/// combat ends has no effect on A, the combat goes on to the press step.
#[test]
fn eyes_of_blades_superior_voids_a_combat_ends_strike() {
    let (mut e, a, x) = duel(&["aus", "aus", "cel", "cel", "pot"], &["ani", "ani"]);
    hand(&mut e, 0, &["Eyes of Blades", "Relentless Pursuit"]);
    hand(&mut e, 1, &["Flesh Bond"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 1, x));
    let req = offering(&mut e, play(0, 0, a));
    assert_eq!(req.window, WindowLabel::CombatPress);
}

/// Eyes of Blades [AUS][CEL]: "Minions can (uselessly) gain additional
/// strikes even if they cannot strike." [LSJ 19970814-2] A's Blur still
/// offered.
#[test]
fn eyes_of_blades_superior_takes_useless_additional_strikes() {
    let (mut e, a, x) = duel(&["aus", "aus", "cel", "cel"], &[]);
    hand(&mut e, 0, &["Eyes of Blades", "Blur"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    let req = offering(&mut e, play(0, 0, a));
    assert_eq!(req.window, WindowLabel::CombatAdditionalStrikes);
}

/// Eyes of Blades: "Is replaced before unlocking cards: other unlock effects
/// cannot be ordered before." [ANK 20200629-2] [LSJ 20091208] [RTR
/// 19951017] Held until seat 0's next unlock phase.
#[test]
fn eyes_of_blades_is_replaced_at_the_unlock_phase() {
    let (mut e, a, x) = duel(&["aus", "cel"], &[]);
    hand(&mut e, 0, &["Eyes of Blades"]);
    e.state.seats[0].library = vec![find_library("Fake Out"); 3].into();
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let held = |e: &Engine| e.state.seats[0].held[zvtes::state::Release::NextUnlockPhase];
    assert_eq!(held(&e), 1);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(held(&e), 1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(held(&e), 0);
}

// ---- Blood Trade --------------------------------------------------------------

/// Blood Trade: "No more boons can be put in play": Minor Boon, a boon
/// putting itself on a vampire, is not played (rules-qa 115.7); the
/// vampire goes to torpor.
#[test]
fn blood_trade_bars_a_boon_put_on_a_vampire() {
    let mut e = game(3);
    let mid = |c: &zvtes::crypt::CryptCard| (4..=8).contains(&c.capacity);
    let b = e.add_minion(SeatId(0), textless(mid, 0), 4, false);
    e.add_minion(SeatId(0), textless(mid, 1), 4, false);
    let v = e.add_minion(SeatId(1), textless(mid, 2), 0, false);
    e.add_minion(SeatId(1), textless(mid, 3), 4, false);
    e.add_minion(SeatId(2), textless(mid, 4), 4, false);
    e.state.cards.push(CardInPlay::new(
        find_library("Blood Trade"),
        SeatId(0),
        None,
        0,
    ));
    let trade = CardId(e.state.cards.len() as u32 - 1);
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::NoBoons,
        source: trade,
    });
    hand(&mut e, 2, &["Minor Boon"]);
    pick(&mut e, bleed(b));
    pick_when(&mut e, block(v));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::Play { card: 0, .. })
    });
    assert!(e.state.minions[v.index()].in_torpor);
}

// ---- Reactions ----------------------------------------------------------------

/// The `nth` textless vampire of capacity `cap` printing none of `without`.
#[track_caller]
fn of_cap(cap: u8, without: &[&str], nth: usize) -> u16 {
    textless(
        |c| {
            c.capacity == cap
                && c.clan != "Banu Haqim"
                && c.disciplines
                    .iter()
                    .all(|d| !without.iter().any(|w| d.eq_ignore_ascii_case(w)))
        },
        nth,
    )
}

/// The Disciplines the fixtures give by hand, never printed on them.
const GIVEN: &[&str] = &["cel", "chi", "ser", "qui", "for", "obf", "pre"];

/// Three seats: seat 0's acting A (capacity `a_cap`) and a second vampire,
/// seat 1's X (capacity `x_cap`, `x_levels` gained) and a second, seat 2's
/// Z; 4 blood each.
fn reactors(a_cap: u8, x_cap: u8, x_levels: &[&'static str]) -> (Engine, MinionId, MinionId) {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), of_cap(a_cap, &["cel", "flight"], 0), 4, false);
    e.add_minion(SeatId(0), of_cap(5, GIVEN, 1), 4, false);
    let x = e.add_minion(SeatId(1), of_cap(x_cap, GIVEN, 2), 4, false);
    e.add_minion(SeatId(1), of_cap(5, GIVEN, 3), 4, false);
    e.add_minion(SeatId(2), of_cap(5, GIVEN, 4), 4, false);
    e.state.minions[x.index()].gained_levels = x_levels.to_vec();
    (e, a, x)
}

/// Rooftop Shadow: "[cel] +1 intercept. Not usable if the acting minion has
/// Celerity [cel]. [CEL] +1 intercept. Not usable if the acting minion has
/// superior Celerity [CEL]." Against no Celerity both, against basic only
/// [CEL], against superior neither; played, the block lands.
#[test]
fn rooftop_shadow_intercepts_only_a_minion_short_of_its_celerity() {
    for (acting, modes) in [(0usize, &[0u8, 1][..]), (1, &[1][..]), (2, &[][..])] {
        let (mut e, a, x) = reactors(4, 6, &["cel", "cel"]);
        e.state.minions[a.index()].gained_levels = vec!["cel"; acting];
        stealthier(&mut e, a, 1);
        hand(&mut e, 1, &["Rooftop Shadow"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(x));
        let req = until(&mut e, SeatId(1), WindowLabel::ActionBlock);
        let offered: Vec<u8> = plays(&req)
            .iter()
            .filter_map(|c| match c {
                Choice::Play { card: 0, mode, .. } => Some(*mode),
                _ => None,
            })
            .collect();
        assert_eq!(offered, modes, "acting with {acting} cel");
        if let Some(&m) = modes.first() {
            pick(&mut e, play(0, m, x));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(e.state.seats[1].pool, 10, "blocked");
            assert!(e.state.minions[x.index()].locked);
        }
    }
}

/// Ignis Fatuus [chi]: "The acting minion gets -1 stealth until the end of
/// the turn": any time before resolution, no attempt needed (LSJ
/// 20020612), counted by the attempt's reach; the minion's next action this
/// turn starts 1 lower, the next turn not.
/// Ledger: `[LSJ 20020612]`.
#[test]
fn ignis_fatuus_strips_stealth_until_the_end_of_the_turn() {
    let (mut e, a, x) = reactors(4, 6, &[]);
    let z = e.state.minions_of(SeatId(2)).next().unwrap();
    e.state.minions[z.index()].gained_levels = vec!["chi"];
    stealthier(&mut e, a, 1);
    hand(&mut e, 2, &["Ignis Fatuus"]);
    pick(&mut e, bleed(a));
    let req = expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(req.options.contains(&block(x)));
    assert!(
        e.block_can_reach(SeatId(1), x),
        "Ignis Fatuus in a third hand"
    );
    e.choose(index_of(&req, Choice::Pass));
    pick_when(&mut e, play(0, 0, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.lasting(a).0,
        0,
        "+1 and -1 for the rest of the turn"
    );
    e.state.minions[a.index()].locked = false;
    pick(&mut e, action(a, zvtes::decision::ActionKind::Hunt));
    request(&mut e);
    assert_eq!(
        e.action().unwrap().stealth(&e.state),
        1,
        "its hunt: 1 + 1 - 1"
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.lasting(a).0, 0);
    assert!(e.state.minions[a.index()]
        .timed
        .iter()
        .all(|t| t.0 != zvtes::cards::Lasting::Stealth(-1)));
}

/// Ignis Fatuus [CHI]: "Reduce a bleed against any Methuselah by 1": by a
/// third Methuselah, with the impulse (ANK 20200607).
/// Ledger: `[ANK 20200607]`.
#[test]
fn ignis_fatuus_reduces_another_methuselahs_bleed() {
    let (mut e, a, _) = reactors(4, 6, &[]);
    let z = e.state.minions_of(SeatId(2)).next().unwrap();
    e.state.minions[z.index()].gained_levels = vec!["chi", "chi"];
    hand(&mut e, 2, &["Ignis Fatuus"]);
    stronger(&mut e, a, 0);
    e.state.minions[a.index()].timed.push((
        zvtes::cards::Lasting::Bleed(1),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(0),
    ));
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 1, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 9, "2 - 1");
}

/// Nest of Eagles: "Not usable if the acting minion is an Banu Haqim or
/// wraith or has flight [FLIGHT]. Reduce a bleed against you by 1. If the
/// acting minion is an ally or vampire with capacity 5 or less, reduce the
/// bleed amount by 3 instead." A capacity-5 bleeder: 3 off; capacity 6: 1.
#[test]
fn nest_of_eagles_reduces_a_small_bleeder_by_three() {
    for (cap, left) in [(5u8, 10), (6, 8)] {
        let mut e = game(3);
        let a = e.add_minion(SeatId(0), of_cap(cap, &["flight"], 0), 4, false);
        e.add_minion(SeatId(0), of_cap(5, GIVEN, 1), 4, false);
        e.state.minions[a.index()].timed.push((
            zvtes::cards::Lasting::Bleed(2),
            zvtes::state::Expiry::EndOfTurn,
            SeatId(0),
        ));
        let r = e.add_minion(SeatId(1), textless(|c| c.clan == "Banu Haqim", 0), 4, false);
        e.add_minion(SeatId(1), of_cap(5, GIVEN, 3), 4, false);
        hand(&mut e, 1, &["Nest of Eagles"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, play(0, 0, r));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[1].pool, left, "a bleed of 3, capacity {cap}");
    }
}

/// Nest of Eagles: an ally's bleed of 3 reduced by 3 (Political Ally); not
/// against a wraith (Screamer).
#[test]
fn nest_of_eagles_reduces_an_ally_by_three_not_a_wraith() {
    for (ally, offered) in [("Political Ally", true), ("Screamer", false)] {
        let mut e = game(3);
        let al = e.add_ally(SeatId(0), find_library(ally));
        e.add_minion(SeatId(0), of_cap(5, GIVEN, 1), 4, false);
        let r = e.add_minion(SeatId(1), textless(|c| c.clan == "Banu Haqim", 0), 4, false);
        e.add_minion(SeatId(1), of_cap(5, GIVEN, 3), 4, false);
        hand(&mut e, 1, &["Nest of Eagles"]);
        pick(&mut e, bleed(al));
        if offered {
            pick_when(&mut e, play(0, 0, r));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(e.state.seats[1].pool, 10, "3 - 3");
        } else {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, r));
        }
    }
}

/// Nest of Eagles: not against a Banu Haqim, a vampire with flight, nor a
/// bleed of another Methuselah.
#[test]
fn nest_of_eagles_spares_banu_haqim_flight_and_others_bleeds() {
    for case in 0..3 {
        let mut e = game(3);
        let bleeder = if case == 0 {
            textless(|c| c.clan == "Banu Haqim", 1)
        } else {
            of_cap(5, &["flight"], 0)
        };
        let a = e.add_minion(SeatId(0), bleeder, 4, false);
        e.add_minion(SeatId(0), of_cap(5, GIVEN, 1), 4, false);
        if case == 1 {
            e.state.minions[a.index()].gained_levels = vec!["flight"];
        }
        let seat = if case == 2 { 2 } else { 1 };
        let r = e.add_minion(
            SeatId(seat),
            textless(|c| c.clan == "Banu Haqim", 0),
            4,
            false,
        );
        e.add_minion(SeatId(seat), of_cap(5, GIVEN, 3), 4, false);
        hand(&mut e, seat, &["Nest of Eagles"]);
        pick(&mut e, bleed(a));
        never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, r));
    }
}

/// Ecstasy [SER]: "after action resolution, if the bleed resolved for 0 (or
/// less), the acting minion burns 1 blood or life": the acting Methuselah
/// orders it among the after-resolution effects, no pass (ANK 20180206:
/// beside Freak Drive [for]); a bleed resolving for 1 burns none.
/// Ledger: `[ANK 20180206]`.
#[test]
fn ecstasy_superior_burns_the_acting_minion_after_a_bleed_for_zero() {
    for extra in [0i8, 1] {
        let (mut e, a, x) = reactors(4, 6, &["ser", "ser"]);
        e.state.minions[a.index()].gained_levels = vec!["for"];
        e.state.minions[a.index()].timed.push((
            zvtes::cards::Lasting::Bleed(extra),
            zvtes::state::Expiry::EndOfTurn,
            SeatId(0),
        ));
        hand(&mut e, 0, &["Freak Drive"]);
        hand(&mut e, 1, &["Ecstasy"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, play(0, 1, x));
        if extra == 0 {
            let req = to_window(&mut e, SeatId(0), WindowLabel::ActionAfterResolution);
            assert!(!req.options.contains(&Choice::Pass));
            assert!(req.options.contains(&play(0, 0, a)), "{:?}", req.options);
            e.choose(index_of(&req, Choice::TakeDamageAfterResolution));
            request(&mut e);
            assert_eq!(blood(&e, a), 3);
            assert_eq!(e.state.seats[1].pool, 10);
        } else {
            never_offered_until(
                &mut e,
                SeatId(0),
                WindowLabel::MinionPhase,
                Choice::TakeDamageAfterResolution,
            );
            assert_eq!(blood(&e, a), 4);
            assert_eq!(e.state.seats[1].pool, 9);
        }
    }
}

/// Ecstasy [SER]: an acting ally at 1 life burns; a blocked bleed resolves
/// for nothing, no burn.
#[test]
fn ecstasy_superior_burns_an_ally_and_spares_a_blocked_bleed() {
    let (mut e, _, x) = reactors(4, 6, &["ser", "ser"]);
    let ally = e.add_ally(SeatId(0), find_library("Gravebound Drone"));
    e.state.minions[ally.index()].blood = 1;
    hand(&mut e, 1, &["Ecstasy"]);
    pick(&mut e, bleed(ally));
    pick_when(&mut e, play(0, 1, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[ally.index()].in_play, "burned");
    assert_eq!(e.state.seats[1].pool, 10);

    let (mut e, a, x) = reactors(4, 6, &["ser", "ser"]);
    let y = e.state.minions_of(SeatId(1)).nth(1).unwrap();
    hand(&mut e, 1, &["Ecstasy"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 1, x));
    pick_when(&mut e, block(y));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::TakeDamageAfterResolution,
    );
}

/// Army of Apparitions [chi]: "Usable by a locked vampire. … Only usable
/// during the polling step of a political action. This vampire gets +3
/// votes": a locked vampire's 3 votes.
#[test]
fn army_of_apparitions_votes_from_a_locked_vampire() {
    let (mut e, a, x) = reactors(4, 6, &["chi"]);
    e.state.minions[x.index()].locked = true;
    hand(&mut e, 0, &["Ancilla Empowerment"]);
    hand(&mut e, 1, &["Army of Apparitions"]);
    pick(&mut e, play(0, 0, a));
    to_referendum(&mut e);
    pick_when(&mut e, play(0, 0, x));
    request(&mut e);
    let rf = e.referendum().expect("referendum");
    let gave: Vec<_> = rf
        .records
        .iter()
        .filter(|v| v.source == zvtes::referendum::Source::Vampire(x))
        .map(|v| v.n)
        .collect();
    assert!(gave.contains(&3) || rf.bonus.contains(&(x, 3)), "{gave:?}");
}

/// Army of Apparitions [CHI]: "Only usable if an ally or younger vampire is
/// performing an action directed at a minion you control. Burn 2 blood to
/// have the action fail." A locked vampire burns 2: Bum's Rush at its
/// other vampire fails, no combat, its NRA spent (ANK 20211015); the burn
/// mandatory, 2 blood needed (ANK 20210226).
/// Ledger: `[ANK 20211015] [RTR 20180719]`.
#[test]
fn army_of_apparitions_burns_two_blood_to_fail_an_action_at_your_minion() {
    let (mut e, a, x) = reactors(4, 6, &["chi", "chi"]);
    let y = e.state.minions_of(SeatId(1)).nth(1).unwrap();
    e.state.minions[x.index()].locked = true;
    hand(&mut e, 0, &["Bum's Rush"]);
    hand(&mut e, 1, &["Army of Apparitions"]);
    e.notes = Some(Vec::new());
    pick(&mut e, play_on(0, 0, a, Target::Minion(y)));
    pick_when(&mut e, play(0, 1, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 2);
    assert!(noted_combats(&mut e).is_empty(), "failed: no combat");
    assert_eq!(blood(&e, y), 4);
    let a_done = e.state.minions[a.index()].acted.len();
    assert_eq!(a_done, 1, "it reached resolution: NRA");
    assert!(ash_has(&e, 1, "Army of Apparitions"));
}

/// Army of Apparitions [CHI]: not with 1 blood (ANK 20210226), not against
/// an older acting vampire, not for a bleed (the Methuselah, not a minion),
/// nor once a block is attempted (ANK 20260316, LSJ 20041022).
/// Ledger: `[ANK 20210226]`, `[ANK 20260316] [LSJ 20041022]`.
#[test]
fn army_of_apparitions_superior_needs_its_conditions() {
    for case in 0..4 {
        let (a_cap, x_blood) = match case {
            0 => (4, 1),
            1 => (7, 4),
            _ => (4, 4),
        };
        let (mut e, a, x) = reactors(a_cap, 6, &["chi", "chi"]);
        let y = e.state.minions_of(SeatId(1)).nth(1).unwrap();
        e.state.minions[x.index()].blood = x_blood;
        hand(&mut e, 0, &["Bum's Rush"]);
        hand(&mut e, 1, &["Army of Apparitions"]);
        if case == 2 {
            pick(&mut e, bleed(a));
        } else {
            pick(&mut e, play_on(0, 0, a, Target::Minion(y)));
        }
        if case == 3 {
            pick_when(&mut e, block(y));
        }
        never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, x));
    }
}

/// Lost in Translation: "Only usable when an ally or younger vampire is
/// bleeding you, after blocks are declined. Lock this reacting vampire.
/// Choose another Methuselah other than the acting minion's controller.
/// The acting minion is now bleeding the chosen Methuselah. Only one Lost
/// in Translation may be played each action." The new target may block
/// (rules-qa 1.3), but plays no second copy this action.
#[test]
fn lost_in_translation_redirects_a_younger_bleeder_once_an_action() {
    let (mut e, a, x) = reactors(4, 6, &[]);
    let z = e.state.minions_of(SeatId(2)).next().unwrap();
    let z2 = e.add_minion(SeatId(2), of_cap(5, GIVEN, 5), 4, false);
    hand(&mut e, 1, &["Lost in Translation"]);
    hand(&mut e, 2, &["Lost in Translation"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play_on(0, 0, x, Target::Seat(SeatId(2))));
    let req = offering(&mut e, block(z));
    assert_eq!(req.seat, SeatId(2), "the new target blocks");
    assert!(e.state.minions[x.index()].locked);
    e.choose(index_of(&req, Choice::Pass));
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { card: 0, actor: Some(m), .. } if *m == z || *m == z2),
    );
    assert_eq!((e.state.seats[1].pool, e.state.seats[2].pool), (10, 9));
}

/// Lost in Translation after an attempt short of the stealth: "it is still
/// declining to block" (ANK 20211003); not against an older bleeder.
/// Ledger: `[ANK 20211003]`.
#[test]
fn lost_in_translation_follows_a_failed_block_attempt() {
    let (mut e, a, x) = reactors(4, 6, &[]);
    stealthier(&mut e, a, 1);
    hand(&mut e, 1, &["Lost in Translation"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    let req = offering(&mut e, play_on(0, 0, x, Target::Seat(SeatId(2))));
    assert_eq!(req.window, WindowLabel::ActionBlocksDeclined);

    let (mut e, a, x) = reactors(7, 6, &[]);
    hand(&mut e, 1, &["Lost in Translation"]);
    pick(&mut e, bleed(a));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        play_on(0, 0, x, Target::Seat(SeatId(2))),
    );
}

/// Lost in Translation: "must be played before action resolution, ie.
/// before any modifier or reaction played 'as the action/bleed would be
/// successful'" (LSJ 19980105): never in that window (Strix [STR] offered
/// there).
/// Ledger: `[LSJ 19980105]`.
#[test]
fn lost_in_translation_not_in_the_would_be_successful_window() {
    let (mut e, a, x) = reactors(4, 6, &["str", "str"]);
    hand(&mut e, 1, &["Lost in Translation", "Strix"]);
    pick(&mut e, bleed(a));
    let req = offering(&mut e, play(1, 1, x));
    assert_eq!(req.window, WindowLabel::ActionWouldBeSuccessful);
    assert!(!plays(&req)
        .iter()
        .any(|c| matches!(c, Choice::Play { card: 0, .. })));
}

/// Fillip: "Requires a vampire. Usable by a locked vampire. Choose a
/// younger locked vampire you control. The chosen vampire can play reaction
/// cards and attempt to block as though unlocked until the current action
/// is concluded. A vampire may play only one Fillip each turn." The locked
/// younger vampire blocks; an older one is no target; a second Fillip by
/// the same vampire this turn is not played.
#[test]
fn fillip_wakes_a_younger_locked_vampire_once_a_turn() {
    let (mut e, a, x) = reactors(4, 6, &[]);
    let y = e.state.minions_of(SeatId(1)).nth(1).unwrap();
    let old = e.add_minion(SeatId(1), of_cap(7, &[], 0), 4, false);
    for m in [x, y, old] {
        e.state.minions[m.index()].locked = true;
    }
    hand(&mut e, 1, &["Fillip", "Fillip"]);
    pick(&mut e, bleed(a));
    let req = offering(&mut e, play_on(0, 0, x, Target::Minion(y)));
    assert!(!req.options.contains(&play_on(0, 0, x, Target::Minion(old))));
    assert!(!req.options.contains(&block(y)));
    e.choose(index_of(&req, play_on(0, 0, x, Target::Minion(y))));
    pick_when(&mut e, block(y));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10, "blocked by the locked vampire");

    let a2 = e.state.minions_of(SeatId(0)).nth(1).unwrap();
    e.state.minions[y.index()].locked = true;
    pick(&mut e, bleed(a2));
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { card: 0, actor: Some(m), .. } if *m == x),
    );
}

/// Foul Blood [qui]: "Only usable if a vampire would successfully hunt. …
/// The acting vampire gets -1 hunt and, after action resolution, takes 1
/// unpreventable environmental damage. Lock this reacting vampire." A
/// rulebook hunt gains nothing — not a successful hunt (LSJ 20050720-2), so
/// a second Foul Blood finds none to spoil; the hunter burns 1.
/// Ledger: `[LSJ 20050720-2]`.
#[test]
fn foul_blood_makes_a_hunt_gain_nothing_and_damages_after() {
    let (mut e, a, x) = reactors(4, 6, &["qui"]);
    let y = e.state.minions_of(SeatId(1)).nth(1).unwrap();
    e.state.minions[y.index()].gained_levels = vec!["qui"];
    e.state.minions[a.index()].blood = 2;
    hand(&mut e, 1, &["Foul Blood", "Foul Blood"]);
    pick(&mut e, action(a, zvtes::decision::ActionKind::Hunt));
    let req = offering(&mut e, play(0, 0, x));
    assert_eq!(req.window, WindowLabel::ActionBlocksDeclined);
    e.choose(index_of(&req, play(0, 0, x)));
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == y),
    );
    assert_eq!(blood(&e, a), 1, "no blood gained, 1 damage");
    assert!(e.state.minions[x.index()].locked);
    assert_eq!(blood(&e, x), 3, "its cost");
    assert_eq!(
        e.state.minions[a.index()].hunted_turn,
        None,
        "not a successful hunt"
    );
}

/// Fillip in an action card's as-played window, its action not declared
/// yet (rules-qa 96.4, 2.4): the chosen vampire is woken there and cancels
/// the card (Touch of Clarity [dem]).
#[test]
fn fillip_wakes_in_an_action_cards_as_played_window() {
    let (mut e, a, x) = reactors(4, 6, &[]);
    let y = e.state.minions_of(SeatId(1)).nth(1).unwrap();
    for m in [x, y] {
        e.state.minions[m.index()].locked = true;
    }
    e.state.minions[a.index()].gained_levels = vec!["dom"];
    e.state.minions[y.index()].gained_levels = vec!["dem"];
    hand(&mut e, 0, &["Govern the Unaligned"]);
    hand(&mut e, 1, &["Fillip", "Touch of Clarity"]);
    pick(&mut e, play(0, 0, a));
    let req = expect(&mut e, SeatId(1), WindowLabel::CardAsPlayed);
    e.choose(index_of(&req, play_on(0, 0, x, Target::Minion(y))));
    let req = expect(&mut e, SeatId(1), WindowLabel::CardAsPlayed);
    e.choose(index_of(&req, play(0, 0, y)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "Govern the Unaligned"));
    assert_eq!(e.state.seats[1].pool, 10, "canceled");
}

/// Foul Blood [QUI]: "As above, but do not lock this reacting vampire";
/// not on a diseased vampire's hunt, which gains nothing anyway (rules-qa
/// 59.4).
#[test]
fn foul_blood_superior_leaves_the_reactor_unlocked() {
    let (mut e, a, x) = reactors(4, 6, &["qui", "qui"]);
    e.state.minions[a.index()].blood = 2;
    hand(&mut e, 1, &["Foul Blood"]);
    pick(&mut e, action(a, zvtes::decision::ActionKind::Hunt));
    pick_when(&mut e, play(0, 1, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 1);
    assert!(!e.state.minions[x.index()].locked);

    let (mut e, a, x) = reactors(4, 6, &["qui", "qui"]);
    e.state.minions[a.index()].disease = true;
    hand(&mut e, 1, &["Foul Blood"]);
    pick(&mut e, action(a, zvtes::decision::ActionKind::Hunt));
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == x),
    );
}

/// Hard Case [for]: "Only usable when this vampire successfully blocks an
/// ally or younger vampire. Cancel the resulting combat." The blocker
/// locked by its block (ANK 20190708) plays it: no combat, the bleed
/// blocked; not against an older acting vampire.
/// Ledger: `[ANK 20190708]`.
#[test]
fn hard_case_cancels_the_blocks_combat_the_blocker_locked() {
    for (a_cap, offered) in [(4u8, true), (7, false)] {
        let (mut e, a, x) = reactors(a_cap, 6, &["for"]);
        hand(&mut e, 1, &["Hard Case"]);
        e.notes = Some(Vec::new());
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(x));
        if !offered {
            never_offered_where(
                &mut e,
                SeatId(0),
                WindowLabel::MinionPhase,
                |o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == x),
            );
            continue;
        }
        let req = offering(&mut e, play(0, 0, x));
        assert_eq!(req.window, WindowLabel::ActionBlocked);
        assert!(e.state.minions[x.index()].locked);
        e.choose(index_of(&req, play(0, 0, x)));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(noted_combats(&mut e).is_empty(), "no combat");
        assert_eq!(e.state.seats[1].pool, 10, "blocked");
        assert!(e.state.minions[x.index()].locked);
        assert_eq!(blood(&e, x), 3);
    }
}

/// Hard Case [FOR]: "As above, and destroy a weapon or vehicle on the
/// acting minion": the chosen weapon burned, the vehicle kept; nothing to
/// destroy, [FOR] is not offered (advanced rules §1.6.5).
#[test]
fn hard_case_superior_burns_a_weapon_of_the_acting_minion() {
    let (mut e, a, x) = reactors(4, 6, &["for", "for"]);
    let gun = borne(&mut e, ".44 Magnum", a);
    let bike = borne(&mut e, "Sport Bike", a);
    let own = borne(&mut e, "Baseball Bat", x);
    hand(&mut e, 1, &["Hard Case"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    let req = offering(&mut e, play_on(0, 1, x, Target::Card(gun)));
    assert!(req.options.contains(&play_on(0, 1, x, Target::Card(bike))));
    assert!(!req.options.contains(&play_on(0, 1, x, Target::Card(own))));
    e.choose(index_of(&req, play_on(0, 1, x, Target::Card(gun))));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[gun.index()].in_play);
    assert!(ash_has(&e, 0, ".44 Magnum"));
    assert!(e.state.cards[bike.index()].in_play);

    let (mut e, a, x) = reactors(4, 6, &["for", "for"]);
    hand(&mut e, 1, &["Hard Case"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    let req = offering(&mut e, play(0, 0, x));
    assert!(!plays(&req)
        .iter()
        .any(|c| matches!(c, Choice::Play { mode: 1, .. })));
}

/// Pass until a combat is under way.
fn to_combat(e: &mut Engine) -> zvtes::decision::DecisionRequest {
    for _ in 0..100 {
        let req = request(e);
        if e.combat().is_some() {
            return req;
        }
        e.choose(fallback(&req));
    }
    panic!("no combat");
}

/// The Clandestine Contract on A choosing `chosen` (none: chosen by no
/// play, LSJ 20071109).
fn dossier(chosen: Option<MinionId>, a: MinionId, e: &mut Engine) {
    let k = borne(e, "Clandestine Contract", a);
    e.state.cards[k.index()].chosen = chosen.into_iter().collect();
}

/// Open Dossier: "Only usable by a vampire who has been chosen for a
/// contract on the acting minion. This vampire attempts to block with +2
/// intercept and gets an optional maneuver in the resulting combat if
/// successful. Usable by a locked vampire even if intercept is not yet
/// needed." Locked X, no attempt pending, blocks a +2 stealth bleed with
/// its maneuver in the combat.
#[test]
fn open_dossier_lets_a_locked_contract_holder_block() {
    let (mut e, a, x) = reactors(4, 6, &[]);
    e.state.minions[x.index()].locked = true;
    dossier(Some(x), a, &mut e);
    stealthier(&mut e, a, 2);
    hand(&mut e, 1, &["Open Dossier"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, x));
    to_combat(&mut e);
    let c = e.combat().expect("the block's combat");
    let side = c.side_of_minion(x).expect("X fights");
    assert_eq!(c.sides[1 - side].minion, a);
    assert_eq!(c.sides[side].combat_maneuvers, 1);
}

/// Open Dossier by a vampire already attempting (ANK 20181122-2): its +2
/// make the attempt reach a +2 stealth bleed.
/// Ledger: `[ANK 20181122-2]`.
#[test]
fn open_dossier_helps_an_attempt_under_way() {
    let (mut e, a, x) = reactors(4, 6, &[]);
    dossier(Some(x), a, &mut e);
    stealthier(&mut e, a, 2);
    hand(&mut e, 1, &["Open Dossier"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(0, 0, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10, "blocked");
}

/// Open Dossier: locked, it plays no other reaction as if unlocked (LSJ
/// 20080605): no Rooftop Shadow while its attempt is short; one attempt
/// only (LSJ 20030227): failed, it is offered none again.
/// Ledger: `[LSJ 20080605]`, `[LSJ 20030227]`.
#[test]
fn open_dossier_attempts_once_and_no_other_reaction() {
    let (mut e, a, x) = reactors(4, 6, &["cel"]);
    e.state.minions[x.index()].locked = true;
    dossier(Some(x), a, &mut e);
    stealthier(&mut e, a, 3);
    hand(&mut e, 1, &["Open Dossier", "Rooftop Shadow"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, x));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::Play { card: 1, .. }) || *o == Choice::DeclareBlock { minion: x }
    });
    assert_eq!(e.state.seats[1].pool, 9, "unblocked");
}

/// Open Dossier: "chosen for a contract": not by a contract choosing none
/// (Vidal Jarbeaux, Mata Hari: LSJ 20070708, LSJ 20071109), nor one on
/// another minion; not if it cannot attempt (Daring the Dawn: LSJ
/// 20010714-2), nor beyond the normal restrictions: a third Methuselah's
/// vampire cannot block a bleed of another (ANK 20180623).
/// Ledger: `[LSJ 20070708] [LSJ 20071109]`, `[LSJ 20010714-2]`, `[ANK 20180623]`.
#[test]
fn open_dossier_needs_a_contract_choice_on_the_acting_minion() {
    for case in 0..4 {
        let (mut e, a, x) = reactors(4, 6, &[]);
        let x = if case == 3 {
            e.state.minions_of(SeatId(2)).next().unwrap()
        } else {
            x
        };
        e.state.minions[x.index()].locked = true;
        match case {
            0 => dossier(None, a, &mut e),
            1 => {
                let a2 = e.state.minions_of(SeatId(0)).nth(1).unwrap();
                dossier(Some(x), a2, &mut e);
            }
            2 => {
                dossier(Some(x), a, &mut e);
                e.state.minions[a.index()].gained_levels = vec!["for", "for"];
                hand(&mut e, 0, &["Daring the Dawn"]);
            }
            _ => dossier(Some(x), a, &mut e),
        }
        let seat = e.state.minions[x.index()].owner.0;
        hand(&mut e, seat, &["Open Dossier"]);
        pick(&mut e, bleed(a));
        if case == 2 {
            pick_when(&mut e, play(0, 0, a));
        }
        never_offered_where(
            &mut e,
            SeatId(0),
            WindowLabel::MinionPhase,
            |o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == x),
        );
    }
}

/// Pseudo-Blindness [chi]: "Only usable by a blocking vampire. Cancel an
/// action modifier card requiring Chimerstry [chi] or Obfuscate [obf] as it
/// is played." Elder Impersonation canceled, its blood paid (ANK
/// 20260216), the stealth unchanged: the block lands.
/// Ledger: `[ANK 20260216]`.
#[test]
fn pseudo_blindness_cancels_an_obfuscate_modifier_its_cost_paid() {
    let (mut e, a, x) = reactors(4, 6, &["chi"]);
    e.state.minions[a.index()].gained_levels = vec!["obf", "chi"];
    hand(&mut e, 0, &["Elder Impersonation", "Amria"]);
    hand(&mut e, 1, &["Pseudo-Blindness"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(0, 0, a));
    let req = offering(&mut e, play(0, 0, x));
    assert_eq!(req.window, WindowLabel::CardAsPlayed);
    e.choose(index_of(&req, play(0, 0, x)));
    to_combat(&mut e);
    assert_eq!(blood(&e, a), 3, "Elder Impersonation's cost paid");
    assert!(ash_has(&e, 0, "Elder Impersonation"));
    assert_eq!(
        e.combat().unwrap().sides[1].minion,
        x,
        "blocked: no stealth"
    );
}

/// Pseudo-Blindness: not by a vampire not attempting, nor on a modifier
/// requiring another Discipline.
#[test]
fn pseudo_blindness_needs_the_blocker_and_its_disciplines() {
    for case in 0..2 {
        let (mut e, a, x) = reactors(4, 6, &["chi"]);
        let z = e.state.minions_of(SeatId(2)).next().unwrap();
        e.state.minions[z.index()].gained_levels = vec!["chi"];
        e.state.minions[a.index()].gained_levels = vec!["obf", "pre", "pro"];
        hand(
            &mut e,
            0,
            &[if case == 0 {
                "Elder Impersonation"
            } else {
                "Enchanting Gaze"
            }],
        );
        hand(&mut e, 1, &["Pseudo-Blindness"]);
        hand(&mut e, 2, &["Pseudo-Blindness"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(x));
        let req = offering(&mut e, play(0, 0, a));
        e.choose(index_of(&req, play(0, 0, a)));
        never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
            matches!(o, Choice::Play { actor: Some(m), .. }
                if *m == z || case == 1 && *m == x)
        });
    }
}

/// Pseudo-Blindness [CHI]: "As above, and the acting minion cannot use
/// cards requiring Chimerstry or Obfuscate during the resulting combat if
/// this vampire blocks": Amria [chi] not offered to A there.
#[test]
fn pseudo_blindness_superior_bars_chimerstry_in_the_block_combat() {
    for superior in [false, true] {
        let (mut e, a, x) = reactors(4, 6, &["chi", "chi"]);
        e.state.minions[a.index()].gained_levels = vec!["obf", "chi"];
        hand(&mut e, 0, &["Elder Impersonation", "Amria"]);
        hand(&mut e, 1, &["Pseudo-Blindness"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(x));
        pick_when(&mut e, play(0, 0, a));
        pick_when(&mut e, play(0, superior as u8, x));
        e.notes = Some(Vec::new());
        let by_a = |o: &Choice| matches!(o, Choice::Play { actor: Some(m), .. } if *m == a);
        if superior {
            never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, by_a);
            assert!(!noted_combats(&mut e).is_empty(), "blocked");
        } else {
            let req = to_combat(&mut e);
            assert_eq!(req.window, WindowLabel::CombatDetermineRange);
            assert!(req.options.iter().any(by_a));
        }
    }
}

/// Revelation of Despair [ser]: "Only usable if this vampire blocks, before
/// block resolution. The acting minion burns 1 blood or life." An acting
/// ally at 1 life burns: the action ends, the blocker not locked (RBK
/// resolve-the-action); a vampire burns 1 and the combat follows.
/// Ledger: `[RBK resolve-the-action]`.
#[test]
fn revelation_of_despair_burns_an_acting_ally_and_leaves_the_blocker_unlocked() {
    let (mut e, _, x) = reactors(4, 6, &["ser"]);
    let ally = e.add_ally(SeatId(0), find_library("Gravebound Drone"));
    e.state.minions[ally.index()].blood = 1;
    hand(&mut e, 1, &["Revelation of Despair"]);
    e.notes = Some(Vec::new());
    pick(&mut e, bleed(ally));
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(0, 0, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[ally.index()].in_play);
    assert!(
        !e.state.minions[x.index()].locked,
        "not locked for the block"
    );
    assert!(noted_combats(&mut e).is_empty());
    assert_eq!(e.state.seats[1].pool, 10);

    let (mut e, a, x) = reactors(4, 6, &["ser"]);
    e.state.minions[a.index()].gained_levels = vec!["chi"];
    hand(&mut e, 0, &["Amria"]);
    hand(&mut e, 1, &["Revelation of Despair"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(0, 0, x));
    to_combat(&mut e);
    assert_eq!(blood(&e, a), 3);
    assert!(e.state.minions[x.index()].locked);
}

/// Revelation of Despair [SER]: "Put 1 corruption counter on the acting
/// minion. If the number of your corruption counters on the minion equals
/// or exceeds their capacity or cost, you can burn those counters to steal
/// the target minion." Stolen, the block stands and the blocker locks (ANK
/// 20220127), but no combat begins between the two, now both the
/// reactor's (rules-qa 121.129); declined, the counters stay.
/// Ledger: `[ANK 20220127]`.
#[test]
fn revelation_of_despair_steals_the_acting_minion_and_the_block_stands() {
    for steal in [true, false] {
        let (mut e, a, x) = reactors(4, 6, &["ser", "ser"]);
        e.state.minions[a.index()].add_corruption(SeatId(1), 3);
        e.state.minions[a.index()].gained_levels = vec!["chi"];
        hand(&mut e, 0, &["Amria"]);
        hand(&mut e, 1, &["Revelation of Despair"]);
        e.notes = Some(Vec::new());
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(x));
        pick_when(&mut e, play(0, 1, x));
        let req = expect(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
        if !steal {
            e.choose(index_of(&req, Choice::Pass));
            to_combat(&mut e);
            assert_eq!(e.state.minions[a.index()].corruption_of(SeatId(1)), 4);
            assert_eq!(e.state.minions[a.index()].owner, SeatId(0));
            continue;
        }
        e.choose(index_of(&req, Choice::ChooseMinion(a)));
        request(&mut e);
        assert_eq!(e.state.minions[a.index()].owner, SeatId(1), "stolen");
        assert_eq!(e.state.minions[a.index()].corruption_of(SeatId(1)), 0);
        assert!(e.state.minions[x.index()].locked, "the block stands");
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[1].pool, 10);
        let begun: Vec<_> = noted_combats(&mut e)
            .into_iter()
            .filter_map(|n| match n {
                zvtes::combat::CombatNote::Begins { minions } => Some(minions),
                _ => None,
            })
            .collect();
        assert!(begun.is_empty(), "no combat between the reactor's own");
    }
}

/// Pseudo-Blindness cancels a card played "as if from hand" (LSJ
/// 20060106): Elder Impersonation played from The Erciyes Fragments, no
/// stealth gained, the card out of the Fragments.
/// Ledger: `[LSJ 20060106]`.
#[test]
fn pseudo_blindness_cancels_a_modifier_played_from_the_erciyes_fragments() {
    let (mut e, a, x) = reactors(4, 6, &["chi"]);
    e.state.minions[a.index()].gained_levels = vec!["obf", "chi"];
    e.state.cards.push(CardInPlay::new(
        find_library("Erciyes Fragments, The"),
        SeatId(0),
        None,
        0,
    ));
    let frag = CardId(e.state.cards.len() as u32 - 1);
    e.state.cards[frag.index()].stored = vec![find_library("Elder Impersonation")].into();
    e.state.cards[frag.index()].stored_owner = Some(SeatId(2));
    hand(&mut e, 0, &["Amria"]);
    hand(&mut e, 1, &["Pseudo-Blindness"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    let req = offering(&mut e, play(1, 0, a));
    e.choose(index_of(&req, play(1, 0, a)));
    pick(&mut e, play(0, 0, x));
    to_combat(&mut e);
    assert!(e.state.cards[frag.index()].stored.is_empty());
    assert_eq!(
        e.combat().unwrap().sides[1].minion,
        x,
        "blocked: no stealth"
    );
}

// ---- Political actions, masters, an event -----------------------------------

/// The `nth` textless untitled vampire matching `f`, of any capacity.
fn crypt_where(f: impl Fn(&zvtes::crypt::CryptCard) -> bool, nth: usize) -> u16 {
    textless(|c| f(c) && !c.adv, nth)
}

/// A master card of the hand played in `mode` at `target`.
fn master_at(card: u8, mode: u8, target: Target) -> Choice {
    Choice::Play {
        card,
        mode,
        actor: None,
        target,
    }
}

/// From seat 0's minion phase to its next master phase.
fn to_master(e: &mut Engine) -> zvtes::decision::DecisionRequest {
    pick(e, Choice::Pass);
    until(e, SeatId(0), WindowLabel::MasterPhase)
}

/// The library card `nth` requiring only `disc` with a blood cost.
fn card_requiring(disc: &str, nth: usize) -> u16 {
    zvtes::library::LIBRARY
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            c.disciplines == [disc]
                && matches!(c.blood_cost, zvtes::library::Cost::Fixed(n) if n > 0)
                && zvtes::cards::lookup(*i as u16).is_some()
        })
        .nth(nth)
        .map(|(i, _)| i as u16)
        .expect("a card requiring the Discipline")
}

/// Chimerstry, Necromancy, Quietus, Serpentis: on any Methuselah's vampire
/// without the superior level, +1 level and +1 capacity, the card itself in
/// play; a vampire with the superior level is no target.
#[test]
fn discipline_cards_raise_level_and_capacity() {
    for (name, d, sup) in [
        ("Chimerstry", "chi", "CHI"),
        ("Necromancy", "nec", "NEC"),
        ("Quietus", "qui", "QUI"),
        ("Serpentis", "ser", "SER"),
    ] {
        let mut e = game(2);
        e.add_minion(
            SeatId(0),
            crypt_where(
                |c| c.capacity >= 4 && !c.disciplines.iter().any(|x| x.eq_ignore_ascii_case(d)),
                0,
            ),
            4,
            false,
        );
        let v = e.add_minion(
            SeatId(1),
            crypt_where(
                |c| c.capacity >= 4 && !c.disciplines.iter().any(|x| x.eq_ignore_ascii_case(d)),
                1,
            ),
            4,
            false,
        );
        let s = e.add_minion(
            SeatId(1),
            crypt_where(|c| c.disciplines.contains(&sup), 0),
            4,
            false,
        );
        hand(&mut e, 0, &[name]);
        let cap = e.state.capacity_of(v).unwrap();
        let req = to_master(&mut e);
        assert!(
            !req.options.contains(&master_at(0, 0, Target::Minion(s))),
            "{name}: superior"
        );
        pick(&mut e, master_at(0, 0, Target::Minion(v)));
        request(&mut e);
        placed_on(&e, name, Some(v));
        assert_eq!(e.state.discipline_level(v, d), 1, "{name}");
        assert_eq!(e.state.capacity_of(v), Some(cap + 1), "{name}");
    }
}

/// The Path of Blood and The Path of Paradox: a card requiring Quietus
/// (Chimerstry) costs a Banu Haqim (Ravnos) 1 fewer blood, another clan
/// nothing off; another Methuselah's vampire burns the Path as an action,
/// taking 1 unpreventable damage (rules-qa 84.25).
#[test]
fn the_paths_of_blood_and_paradox_discount_their_clans_and_bite_their_burner() {
    for (path, clan, d) in [
        ("Path of Blood, The", "Banu Haqim", "qui"),
        ("Path of Paradox, The", "Ravnos", "chi"),
    ] {
        let mut e = game(2);
        let g = e.add_minion(SeatId(0), plain(clan, 0), 6, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let p = in_play(&mut e, 0, path, None);
        let card = card_requiring(d, 0);
        assert_eq!(e.state.printed_cost_off(g, card, 0).blood, 1, "{path}");
        assert_eq!(
            e.state.printed_cost_off(z, card, 0).blood,
            0,
            "{path}: not the clan"
        );
        e.state.active = SeatId(1);
        let req = to_minion_phase(&mut e, SeatId(1));
        let burn = req
            .options
            .iter()
            .find(|o| matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), actor: Some(m), .. } if *c == p && *m == z))
            .copied()
            .expect("the burn offered");
        let before = blood(&e, z);
        pick(&mut e, burn);
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert!(!e.state.cards[p.index()].in_play, "{path}");
        assert!(ash_has(&e, 0, path), "{path}");
        assert_eq!(blood(&e, z), before - 1, "{path}");
    }
}

/// Svadharma: 2 blood to any Methuselah's ready Ravnos; a torpid Ravnos is
/// no target.
#[test]
fn svadharma_adds_two_blood_to_any_ready_ravnos() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Ravnos", 0), 4, false);
    let r = e.add_minion(SeatId(1), plain("Ravnos", 1), 2, false);
    let t = e.add_minion(SeatId(1), plain("Ravnos", 2), 2, true);
    hand(&mut e, 0, &["Svadharma"]);
    let req = to_master(&mut e);
    assert!(!req.options.contains(&master_at(0, 0, Target::Minion(t))));
    pick(&mut e, master_at(0, 0, Target::Minion(r)));
    request(&mut e);
    assert_eq!(blood(&e, r), 4);
}

/// The Crocodile Temple: locked after a successful bleed at its controller,
/// 1 damage to the acting vampire, caused by the Temple.
#[test]
fn the_crocodile_temple_damages_the_successful_bleeder() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let t = in_play(&mut e, 1, "Crocodile Temple, The", None);
    pick(&mut e, bleed(a));
    pick_when(&mut e, ability(t.0, 0));
    request(&mut e);
    assert!(e.state.cards[t.index()].locked);
    assert_eq!(blood(&e, a), 3);
    assert_eq!(e.state.seats[1].pool, 9, "bled");
}

/// The Crocodile Temple is not used by a Methuselah the action ousted.
/// Ledger: `[ANK 20180326]`.
#[test]
fn the_crocodile_temple_is_not_used_by_an_ousted_controller() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    let t = in_play(&mut e, 1, "Crocodile Temple, The", None);
    e.state.seats[1].pool = 1;
    pick(&mut e, bleed(a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, ability(t.0, 0));
    assert!(e.state.seats[1].ousted);
    assert_eq!(blood(&e, a), 4);
}

/// The Eternals of Sirius: played at 4 pool with a ready Ministry, its cost
/// paid as the pool is gained (5, not ousted); not offered at 3. Ledger:
/// `[LSJ 20020620] [LSJ 20080612]`, `[ANK 20210714-2]`.
#[test]
fn the_eternals_of_sirius_pays_four_as_it_gains_five() {
    for pool in [4, 3] {
        let mut e = game(2);
        e.add_minion(SeatId(0), plain("Ministry", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Eternals of Sirius, The"]);
        e.state.seats[0].pool = pool;
        if pool == 3 {
            pick(&mut e, Choice::Pass);
            never_offered_until(
                &mut e,
                SeatId(0),
                WindowLabel::MinionPhase,
                master_at(0, 0, Target::None),
            );
            continue;
        }
        to_master(&mut e);
        pick(&mut e, master_at(0, 0, Target::None));
        request(&mut e);
        assert!(!e.state.seats[0].ousted);
        assert_eq!(e.state.seats[0].pool, 5);
    }
}

/// The Eternals of Sirius: 5 blood to a Ministry of capacity 9 or more in
/// the player's uncontrolled region; one of capacity 8 is no target; with
/// no ready Ministry the first branch still played, gaining nothing
/// (rules-digest D14).
#[test]
fn the_eternals_of_sirius_adds_five_blood_to_a_big_uncontrolled_ministry() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    // Uncontrolled, a vampire's text does nothing: any Ministry of the
    // capacity (round 30's own exhaust the textless ones of 9).
    for cap in [9, 8] {
        let card = CRYPT
            .iter()
            .position(|c| c.clan == "Ministry" && c.capacity == cap && !c.adv && !c.imbued)
            .expect("a Ministry") as u16;
        e.state.seats[0]
            .uncontrolled
            .push(zvtes::state::UncontrolledSlot {
                card,
                copy: Default::default(),
                counters: 1,
                banished: None,
            });
    }
    hand(&mut e, 0, &["Eternals of Sirius, The"]);
    let req = to_master(&mut e);
    assert!(!req
        .options
        .contains(&master_at(0, 1, Target::Uncontrolled(1))));
    assert!(
        req.options.contains(&master_at(0, 0, Target::None)),
        "no ready Ministry: the first branch a legal no-op"
    );
    pick(&mut e, master_at(0, 1, Target::Uncontrolled(0)));
    request(&mut e);
    assert_eq!(e.state.seats[0].uncontrolled[0].counters, 6);
    assert_eq!(e.state.seats[0].pool, 6, "4 paid");
    // The first branch with no ready Ministry: 4 paid, nothing gained.
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Eternals of Sirius, The"]);
    to_master(&mut e);
    pick(&mut e, master_at(0, 0, Target::None));
    request(&mut e);
    assert_eq!(e.state.seats[0].pool, 6);
    assert!(ash_has(&e, 0, "Eternals of Sirius, The"));
}

/// Hierophant: the referendum passes, the card itself goes on the chosen
/// Ministry, any Methuselah's, and gives it a vote.
#[test]
fn hierophant_puts_itself_on_the_chosen_ministry_for_a_vote() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), plain("Ministry", 0), 4, false);
    e.state.minions[caller.index()].granted_title = Some(zvtes::crypt::Title::Votes(1));
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let m = e.add_minion(SeatId(1), plain("Ministry", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Hierophant"]);
    assert_eq!(e.state.votes_held(m), 0);
    pick(&mut e, play(0, 0, caller));
    pick_when(&mut e, Choice::ChooseMinion(m));
    pick_when(&mut e, votes(caller, true));
    close_polling(&mut e);
    request(&mut e);
    placed_on(&e, "Hierophant", Some(m));
    assert_eq!(e.state.votes_held(m), 1);
}

/// Hierophant: a vampire blocking its bearer burns 1 blood before block
/// resolution, caused by the card.
#[test]
fn hierophant_bites_a_vampire_blocking_its_bearer() {
    let after = |with: bool| {
        let (mut e, a, x) = duel(&[], &[]);
        if with {
            in_play(&mut e, 0, "Hierophant", Some(a));
        }
        e.notes = Some(Vec::new());
        bleed_blocked(&mut e, a, x);
        request(&mut e);
        assert!(!noted_combats(&mut e).is_empty(), "the combat after");
        (blood(&e, a), blood(&e, x))
    };
    let (a0, x0) = after(false);
    assert_eq!(after(true), (a0, x0 - 1));
}

/// Warrant: not played with no younger ready vampire to choose. Ledger:
/// group "Political action with illegal terms" `[LSJ 20100129] [ANK
/// 20191228]`.
#[test]
fn warrant_is_not_played_without_a_younger_ready_vampire() {
    for younger in [false, true] {
        let mut e = game(2);
        let caller = e.add_minion(SeatId(0), crypt_where(|c| c.capacity == 5, 0), 4, false);
        e.add_minion(
            SeatId(1),
            crypt_where(|c| c.capacity == if younger { 4 } else { 7 }, 0),
            4,
            false,
        );
        hand(&mut e, 0, &["Warrant"]);
        let req = request(&mut e);
        assert_eq!(req.options.contains(&play(0, 0, caller)), younger);
    }
}

/// Warrant: the card on the chosen vampire; another Methuselah's ready
/// vampire enters combat with it as an action, its 1 blood paid at
/// resolution.
#[test]
fn warrant_lets_any_vampire_rush_its_bearer_for_a_blood() {
    let (mut e, a, x) = duel(&[], &[]);
    let w = in_play(&mut e, 0, "Warrant", Some(x));
    e.notes = Some(Vec::new());
    let rush = Choice::UseAbility {
        source: AbilitySource::Card(w),
        ability: 0,
        actor: Some(a),
        target: Target::Minion(x),
    };
    pick(&mut e, rush);
    to_minion_phase(&mut e, SeatId(0));
    let begun: Vec<[MinionId; 2]> = noted_combats(&mut e)
        .into_iter()
        .filter_map(|n| match n {
            zvtes::combat::CombatNote::Begins { minions } => Some(minions),
            _ => None,
        })
        .collect();
    assert_eq!(begun.len(), 1);
    assert!(begun[0].contains(&a) && begun[0].contains(&x));
    // Equal hand strikes: the actor short its 1 blood more.
    assert_eq!(blood(&e, a) + 1, blood(&e, x), "the blood paid");
}

/// Tajdid: a blood cursed Banu Haqim is offered no diablerie until it is
/// played; once in a game across the table.
#[test]
fn tajdid_lets_banu_haqim_diablerize() {
    let mut e = game(2);
    let cursed = crypt_where(|c| c.clan == "Banu Haqim" && c.blood_cursed, 0);
    let a = e.add_minion(SeatId(0), cursed, 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 2, true);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let diablerie = Choice::TakeAction {
        minion: a,
        action: zvtes::decision::ActionKind::Diablerie,
        target: Some(v),
    };
    assert!(!request(&mut e).options.contains(&diablerie));
    hand(&mut e, 0, &["Tajdid", "Tajdid"]);
    to_master(&mut e);
    pick(&mut e, master_at(0, 0, Target::None));
    let req = to_minion_phase(&mut e, SeatId(0));
    assert!(req.options.contains(&diablerie));
    assert!(e.state.blood_cursed(a), "still blood cursed");
    // The second copy: never offered again.
    pick(&mut e, Choice::Pass);
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        master_at(0, 0, Target::None),
    );
}

/// Urban Jungle: 2 votes against every blood hunt, cast by no Methuselah:
/// 1 in favor fails, 3 pass.
#[test]
fn urban_jungle_adds_two_votes_against_every_blood_hunt() {
    for favor in [1, 3] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 2, true);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        e.state.minions[z.index()].granted_title = Some(zvtes::crypt::Title::Votes(favor));
        let uj = in_play(&mut e, 0, "Urban Jungle", None);
        pick(
            &mut e,
            Choice::TakeAction {
                minion: a,
                action: zvtes::decision::ActionKind::Diablerie,
                target: Some(v),
            },
        );
        pick_when(&mut e, votes(z, true));
        let r = e.referendum().expect("the blood hunt");
        assert!(r
            .records
            .iter()
            .any(|v| v.source == zvtes::referendum::Source::Event(uj)
                && !v.favor
                && v.n == 2
                && !v.by_methuselah()));
        close_polling(&mut e);
        request(&mut e);
        assert_eq!(
            e.state.minions[a.index()].in_play,
            favor == 1,
            "{favor} in favor"
        );
    }
}

/// Jericho Founding: every location burned but those their Methuselah
/// repays; seat 1 keeps one at its pool cost.
#[test]
fn jericho_founding_burns_the_locations_not_repaid() {
    let mut e = game(2);
    let caller = e.add_minion(
        SeatId(0),
        crypt_where(|c| c.sect == Some(Sect::Anarch) && c.capacity >= 4, 0),
        4,
        false,
    );
    e.state.minions[caller.index()].granted_title = Some(zvtes::crypt::Title::Votes(1));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let mine = in_play(&mut e, 0, "Elysium: The Arboretum", None);
    let kept = in_play(&mut e, 1, "Powerbase: Montreal", None);
    let lost = in_play(&mut e, 1, "Crocodile Temple, The", None);
    hand(&mut e, 0, &["Jericho Founding"]);
    pick(&mut e, play(0, 0, caller));
    pick_when(&mut e, votes(caller, true));
    close_polling(&mut e);
    // Clockwise from the caller: seat 0 keeps none.
    assert_eq!(request(&mut e).seat, SeatId(0));
    pick(&mut e, Choice::Pass);
    pick(&mut e, Choice::ChooseCard(kept.0));
    pick(&mut e, Choice::Pass);
    request(&mut e);
    assert!(!e.state.cards[mine.index()].in_play);
    assert!(!e.state.cards[lost.index()].in_play);
    assert!(e.state.cards[kept.index()].in_play);
    let cost = match zvtes::library::LIBRARY[find_library("Powerbase: Montreal") as usize].pool_cost
    {
        zvtes::library::Cost::Fixed(n) => n as i32,
        _ => 0,
    };
    assert_eq!(e.state.seats[1].pool, 10 - cost);
}

/// Jericho Founding needs a non-Camarilla vampire; Garibaldi-Meucci Museum
/// does not retrieve it ("non-Camarilla" requires no Anarch). Ledger:
/// mention in Garibaldi-Meucci Museum `[ANK 20250320] [LSJ 20080603] [LSJ
/// 20050128]`.
#[test]
fn garibaldi_meucci_museum_does_not_take_jericho_founding() {
    let jf = find_library("Jericho Founding");
    assert!(!zvtes::library::LIBRARY[jf as usize]
        .labels
        .contains(&zvtes::library::Label::RequiresAnarch));
    let mut e = game(2);
    let cam = e.add_minion(
        SeatId(0),
        crypt_where(|c| c.sect == Some(Sect::Camarilla) && c.capacity >= 4, 0),
        4,
        false,
    );
    e.state.minions[cam.index()].granted_title = Some(zvtes::crypt::Title::Votes(1));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Jericho Founding"]);
    assert!(!request(&mut e).options.contains(&play(0, 0, cam)));
    // The museum, at its controller's unlock phase, with only Jericho
    // Founding in the ash heap: not offered; Reckless Agitation is.
    let museum = in_play(&mut e, 0, "Garibaldi-Meucci Museum", None);
    e.state.seats[0].ash.push(jf);
    pick(&mut e, Choice::Pass);
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::UnlockAbilities,
        |o| matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), .. } if *c == museum),
    );
    e.state.seats[0]
        .ash
        .push(find_library("Reckless Agitation"));
    let req = request(&mut e);
    assert!(!uses(&req, museum.0).is_empty(), "an Anarch card retrieved");
}

/// The card `name` in play: its id.
fn card_in_play(e: &Engine, name: &str) -> CardId {
    let card = find_library(name);
    (0..e.state.cards.len())
        .map(|i| CardId(i as u32))
        .find(|c| e.state.cards[c.index()].card == card && e.state.cards[c.index()].in_play)
        .expect("in play")
}

/// Swiss Cut: in play with 2 blood (its own play adds none); another
/// investment card played adds 1.
#[test]
fn swiss_cut_enters_with_two_blood_and_grows_with_each_investment() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Giovanni", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Swiss Cut", "Secret Horde"]);
    to_master(&mut e);
    pick(&mut e, master_at(0, 0, Target::None));
    request(&mut e);
    let sc = card_in_play(&e, "Swiss Cut");
    assert!(placed_on_none(&e, "Swiss Cut"));
    assert_eq!(e.state.cards[sc.index()].counters, 2);
    to_master(&mut e);
    pick(&mut e, master_at(0, 0, Target::None));
    pick(&mut e, Choice::SetX(1));
    request(&mut e);
    assert_eq!(e.state.cards[sc.index()].counters, 3);
    assert_eq!(
        e.state.cards[card_in_play(&e, "Secret Horde").index()].counters,
        2
    );
}

/// The card `name` entered play itself, on no minion, out of every ash
/// heap.
fn placed_on_none(e: &Engine, name: &str) -> bool {
    let card = find_library(name);
    e.state
        .cards
        .iter()
        .any(|c| c.card == card && c.in_play && !c.effect_only && c.host.is_none())
        && e.state.seats.iter().all(|s| !s.ash.contains(&card))
}

/// Swiss Cut: each master phase action moves 1 blood to its player's pool
/// or a ready Giovanni of theirs; burned with none left.
#[test]
fn swiss_cut_moves_blood_for_master_phase_actions_and_burns_empty() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), plain("Giovanni", 0), 2, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let sc = in_play(&mut e, 0, "Swiss Cut", None);
    e.state.cards[sc.index()].counters = 2;
    to_master(&mut e);
    pick(&mut e, ability(sc.0, 0));
    request(&mut e);
    assert_eq!(e.state.seats[0].pool, 11);
    assert_eq!(e.state.cards[sc.index()].counters, 1);
    to_master(&mut e);
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Card(sc),
            ability: 1,
            actor: None,
            target: Target::Minion(g),
        },
    );
    request(&mut e);
    assert_eq!(blood(&e, g), 3);
    assert!(!e.state.cards[sc.index()].in_play, "burned empty");
}

/// Alan Sovereign: an investment card his player plays gets 1 more counter.
#[test]
fn alan_sovereign_adds_a_counter_to_his_players_investments() {
    let alan = (0..CRYPT.len())
        .find(|&i| CRYPT[i].name == "Alan Sovereign" && !CRYPT[i].adv)
        .unwrap() as u16;
    let mut e = game(2);
    e.add_minion(SeatId(0), alan, 4, false);
    e.add_minion(SeatId(0), plain("Giovanni", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Swiss Cut"]);
    to_master(&mut e);
    pick(&mut e, master_at(0, 0, Target::None));
    request(&mut e);
    assert_eq!(
        e.state.cards[card_in_play(&e, "Swiss Cut").index()].counters,
        3
    );
}

/// Haqim's Law: Judgment: its Independent or Anarch Banu Haqim locked, the
/// card itself on a younger vampire; an older one is no target.
#[test]
fn haqims_law_judgment_locks_its_banu_haqim_and_goes_on_a_younger_vampire() {
    let mut e = game(2);
    let h = e.add_minion(
        SeatId(0),
        crypt_where(
            |c| c.clan == "Banu Haqim" && c.sect == Some(Sect::Independent) && c.capacity == 6,
            0,
        ),
        4,
        false,
    );
    let y = e.add_minion(SeatId(1), crypt_where(|c| c.capacity == 4, 0), 4, false);
    let o = e.add_minion(SeatId(1), crypt_where(|c| c.capacity == 7, 0), 4, false);
    hand(&mut e, 0, &["Haqim's Law: Judgment"]);
    let req = to_master(&mut e);
    let at = |t| Choice::Play {
        card: 0,
        mode: 0,
        actor: Some(h),
        target: Target::Minion(t),
    };
    assert!(!req.options.contains(&at(o)), "older");
    pick(&mut e, at(y));
    request(&mut e);
    placed_on(&e, "Haqim's Law: Judgment", Some(y));
    assert!(e.state.minions[h.index()].locked);
}

/// Haqim's Law: Judgment: any Independent or Anarch Banu Haqim, chosen for
/// the contract, may rush the attached vampire; a Camarilla Banu Haqim may
/// not. Ledger: group "Action requirements from a card in play" `[LSJ
/// 20080604]`.
#[test]
fn haqims_law_judgment_rush_is_an_independent_or_anarch_banu_haqims() {
    let mut e = game(2);
    let ind = e.add_minion(
        SeatId(0),
        crypt_where(
            |c| c.clan == "Banu Haqim" && c.sect == Some(Sect::Independent) && c.capacity >= 4,
            0,
        ),
        4,
        false,
    );
    let cam = e.add_minion(
        SeatId(0),
        crypt_where(
            |c| c.clan == "Banu Haqim" && c.sect == Some(Sect::Camarilla) && c.capacity >= 4,
            0,
        ),
        4,
        false,
    );
    let y = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let c = in_play(&mut e, 0, "Haqim's Law: Judgment", Some(y));
    let rush = |m| Choice::UseAbility {
        source: AbilitySource::Card(c),
        ability: 0,
        actor: Some(m),
        target: Target::Minion(y),
    };
    let req = request(&mut e);
    assert!(req.options.contains(&rush(ind)));
    assert!(!req.options.contains(&rush(cam)));
    assert!(e.state.chosen_for_contract(c, ind));
    assert!(!e.state.chosen_for_contract(c, cam));
    e.notes = Some(Vec::new());
    pick(&mut e, rush(ind));
    to_minion_phase(&mut e, SeatId(0));
    assert!(noted_combats(&mut e).iter().any(|n| matches!(n,
        zvtes::combat::CombatNote::Begins { minions } if minions.contains(&ind) && minions.contains(&y))));
}

/// Powerbase: Zürich: while its controller has the Edge, their vampire
/// adds 2 blood to a younger vampire in their uncontrolled region; an older
/// one is no target; without the Edge, not offered.
#[test]
fn powerbase_zurich_adds_two_blood_while_its_controller_has_the_edge() {
    for edge in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), crypt_where(|c| c.capacity == 6, 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        for cap in [3, 7] {
            e.state.seats[0]
                .uncontrolled
                .push(zvtes::state::UncontrolledSlot {
                    card: crypt_where(|c| c.capacity == cap, 1),
                    copy: Default::default(),
                    counters: 0,
                    banished: None,
                });
        }
        e.state.seats[0].has_edge = edge;
        let z = in_play(&mut e, 0, "Powerbase: Zürich", None);
        let at = |i| Choice::UseAbility {
            source: AbilitySource::Card(z),
            ability: 0,
            actor: Some(a),
            target: Target::Uncontrolled(i),
        };
        let req = request(&mut e);
        assert_eq!(req.options.contains(&at(0)), edge);
        assert!(!req.options.contains(&at(1)), "older");
        if edge {
            pick(&mut e, at(0));
            to_minion_phase(&mut e, SeatId(0));
            assert_eq!(e.state.seats[0].uncontrolled[0].counters, 2);
            assert!(e.state.seats[0].has_edge, "kept, unblocked");
        }
    }
}

/// Powerbase: Zürich: its action blocked, the Edge is burned before the
/// combat. Ledger: group "Triggered by block" `[ANK 20220116] [ANK
/// 20211207] [ANK 20210627] [RTR 19991206] [LSJ 20091125]`.
#[test]
fn powerbase_zurich_burns_the_edge_as_its_action_is_blocked() {
    let (mut e, a, x) = duel(&[], &[]);
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: crypt_where(|c| c.capacity == 2, 0),
            copy: Default::default(),
            counters: 0,
            banished: None,
        });
    e.state.seats[0].has_edge = true;
    let z = in_play(&mut e, 0, "Powerbase: Zürich", None);
    stealthier(&mut e, a, -1);
    e.notes = Some(Vec::new());
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Card(z),
            ability: 0,
            actor: Some(a),
            target: Target::Uncontrolled(0),
        },
    );
    pick_when(&mut e, block(x));
    request(&mut e);
    assert!(!e.state.seats[0].has_edge);
    assert!(!noted_combats(&mut e).is_empty(), "the block's combat");
    assert_eq!(e.state.seats[0].uncontrolled[0].counters, 0, "blocked");
}

/// Powerbase: Zürich: another Methuselah burns the Edge in their master
/// phase to steal it; its controller is offered no steal.
#[test]
fn powerbase_zurich_is_stolen_for_the_edge() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let z = in_play(&mut e, 1, "Powerbase: Zürich", None);
    e.state.seats[0].has_edge = true;
    let req = to_master(&mut e);
    assert!(req.options.contains(&ability(z.0, 1)));
    pick(&mut e, ability(z.0, 1));
    request(&mut e);
    assert_eq!(e.state.card_controller(z), SeatId(0));
    assert!(!e.state.seats[0].has_edge);
}

/// Seat 0's Giovanni at full capacity in its uncontrolled region, its
/// minion phase passed: the influence request after its move (the minion).
fn influence_giovanni(e: &mut Engine) -> MinionId {
    let g = plain("Giovanni", 0);
    let cap = CRYPT[g as usize].capacity;
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: g,
            copy: Default::default(),
            counters: cap,
            banished: None,
        });
    pick(e, Choice::Pass);
    pick_when(e, Choice::InfluenceOut { slot: 0 });
    MinionId(e.state.minions.len() as u32 - 1)
}

/// Proxy Kissed: played in the influence phase after a Giovanni moves to
/// the ready region: on it, +1 Potence, +1 Fortitude, +1 capacity, +1
/// blood; it spends the next master phase action and bars another
/// out-of-turn card until then. Not offered before the move. Ledger: `[PIB
/// 20150524]`.
#[test]
fn proxy_kissed_is_played_in_the_influence_phase_for_the_next_master_phase_action() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Proxy Kissed", "Lesser Boon"]);
    let g = influence_giovanni(&mut e);
    let cap = e.state.capacity_of(g).unwrap();
    let (pot, fort) = (
        e.state.discipline_level(g, "pot"),
        e.state.discipline_level(g, "for"),
    );
    let pk = master_at(0, 0, Target::Minion(g));
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::InfluenceTransfers);
    pick(&mut e, pk);
    request(&mut e);
    placed_on(&e, "Proxy Kissed", Some(g));
    assert_eq!(e.state.discipline_level(g, "pot"), (pot + 1).min(2));
    assert_eq!(e.state.discipline_level(g, "for"), (fort + 1).min(2));
    assert_eq!(e.state.capacity_of(g), Some(cap + 1));
    assert_eq!(blood(&e, g), cap + 1);
    assert!(e.state.seats[0].out_of_turn.is_some());
    // Its next master phase: none left for a master card.
    hand(&mut e, 0, &["Svadharma"]);
    pick(&mut e, Choice::Pass);
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::Play { actor: None, .. })
    });
}

/// Proxy Kissed: no Giovanni moved this phase, not offered.
#[test]
fn proxy_kissed_needs_a_giovanni_just_influenced() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), plain("Giovanni", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Proxy Kissed"]);
    pick(&mut e, Choice::Pass);
    let req = request(&mut e);
    assert!(!req.options.contains(&master_at(0, 0, Target::Minion(g))));
}

/// Proxy Kissed: on a Giovanni re-influenced that already has one. Ledger:
/// `[LSJ 20050720-1]`.
#[test]
fn proxy_kissed_goes_again_on_a_reinfluenced_giovanni() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Proxy Kissed"]);
    let g = influence_giovanni(&mut e);
    in_play(&mut e, 0, "Proxy Kissed", Some(g));
    let req = request(&mut e);
    assert!(req.options.contains(&master_at(0, 0, Target::Minion(g))));
}

/// A trophy in play for seat `seat`, not awarded yet (no layer).
fn unawarded(e: &mut Engine, seat: u8, name: &str) -> CardId {
    e.state
        .cards
        .push(CardInPlay::new(find_library(name), SeatId(seat), None, 0));
    CardId(e.state.cards.len() as u32 - 1)
}

/// Seat 0's vampire A, seat 1's torpid Red List vampire: A's diablerie
/// action declared, the award offered next.
fn red_list_kill() -> (Engine, MinionId) {
    let red = (0..CRYPT.len() as u16)
        .find(|&c| CRYPT[c as usize].red_list)
        .expect("a Red List vampire");
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let v = e.add_minion(SeatId(1), red, 2, true);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: a,
            action: zvtes::decision::ActionKind::Diablerie,
            target: Some(v),
        },
    );
    (e, a)
}

/// Trophy: Library: played, no effect until awarded; moved to the Red
/// List killer, +2 hand size for that vampire's controller, no blood
/// question asked; the vampire changing hands, the hand size follows.
#[test]
fn trophy_library_counts_only_once_awarded_and_follows_its_vampire() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Trophy: Library"]);
    let size = e.state.hand_size(SeatId(0));
    to_master(&mut e);
    pick(&mut e, master_at(0, 0, Target::None));
    request(&mut e);
    assert!(placed_on_none(&e, "Trophy: Library"));
    assert_eq!(e.state.hand_size(SeatId(0)), size, "not awarded");
    // Awarded (the engine's Red List rule).
    let (mut e2, a2) = red_list_kill();
    let lib2 = unawarded(&mut e2, 0, "Trophy: Library");
    pick_when(&mut e2, Choice::ResolveRider { card: lib2.0 });
    let req = request(&mut e2);
    assert!(
        !req.options.contains(&Choice::ResolveRider { card: lib2.0 }),
        "no blood asked"
    );
    assert_eq!(e2.state.cards[lib2.index()].host, Some(a2));
    assert_eq!(e2.state.hand_size(SeatId(0)), size + 2);
    e2.state.minions[a2.index()].owner = SeatId(1);
    assert_eq!(e2.state.card_controller(lib2), SeatId(1));
    assert_eq!(e2.state.hand_size(SeatId(1)), size + 2);
    assert_eq!(e2.state.hand_size(SeatId(0)), size);
}

/// Several trophies at one award (rulebook "Red List"): those in play moved
/// one by one, one searched; none past its cap (Trophy: Revered: one,
/// Trophy: Library: two).
#[test]
fn trophies_awarded_together_and_capped() {
    let (mut e, a) = red_list_kill();
    in_play(&mut e, 0, "Trophy: Revered", Some(a));
    in_play(&mut e, 0, "Trophy: Library", Some(a));
    let lib = unawarded(&mut e, 0, "Trophy: Library");
    let nq = unawarded(&mut e, 0, "Trophy: No Questions");
    hand(&mut e, 0, &["Trophy: Revered", "Trophy: Library"]);
    let from_hand = |name| Choice::TakeFrom {
        zone: zvtes::decision::Zone::Hand,
        card: find_library(name),
    };
    let req = offering(&mut e, Choice::ResolveRider { card: lib.0 });
    assert!(req.options.contains(&Choice::ResolveRider { card: nq.0 }));
    assert!(req.options.contains(&from_hand("Trophy: Library")));
    assert!(
        !req.options.contains(&from_hand("Trophy: Revered")),
        "one Revered"
    );
    e.choose(index_of(&req, Choice::ResolveRider { card: lib.0 }));
    let req = request(&mut e);
    assert!(
        !req.options.contains(&from_hand("Trophy: Library")),
        "two Libraries"
    );
    assert!(req.options.contains(&Choice::ResolveRider { card: nq.0 }));
    e.choose(index_of(&req, Choice::ResolveRider { card: nq.0 }));
    request(&mut e);
    assert_eq!(e.state.cards[nq.index()].host, Some(a));
    assert_eq!(e.state.cards[lib.index()].host, Some(a));
}

/// Trophy: No Questions: as its vampire announces an action, a counter
/// burned makes it unblockable; with none left, the card burns.
#[test]
fn trophy_no_questions_makes_an_action_unblockable() {
    let (mut e, a, x) = duel(&[], &[]);
    let nq = in_play(&mut e, 0, "Trophy: No Questions", Some(a));
    e.state.cards[nq.index()].counters = 1;
    pick(&mut e, bleed(a));
    pick_when(&mut e, ability(nq.0, 0));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(x));
    assert_eq!(e.state.seats[1].pool, 9, "bled");
    assert!(!e.state.cards[nq.index()].in_play);
    assert!(ash_has(&e, 0, "Trophy: No Questions"));
}

/// Trophy: No Questions searched for an award: its 2 counters with it.
#[test]
fn trophy_no_questions_awarded_with_its_counters() {
    let (mut e, a) = red_list_kill();
    hand(&mut e, 0, &["Trophy: No Questions"]);
    pick_when(
        &mut e,
        Choice::TakeFrom {
            zone: zvtes::decision::Zone::Hand,
            card: find_library("Trophy: No Questions"),
        },
    );
    request(&mut e);
    let nq = card_in_play(&e, "Trophy: No Questions");
    assert_eq!(e.state.cards[nq.index()].host, Some(a));
    assert_eq!(e.state.cards[nq.index()].counters, 2);
}

/// Trophy: Revered: +2 bleed for its vampire.
#[test]
fn trophy_revered_gives_two_bleed() {
    let (mut e, a, _) = duel(&[], &[]);
    in_play(&mut e, 0, "Trophy: Revered", Some(a));
    pick(&mut e, bleed(a));
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(e.state.seats[1].pool, 7);
}

/// Burden the Mind: out of its minion's turn, a wake of that minion costs
/// its user 1 more pool; on its own turn, nothing more.
#[test]
fn burden_the_mind_taxes_a_wake_out_of_turn() {
    for burden in [false, true] {
        let (mut e, a, x) = duel(&[], &[]);
        e.state.minions[x.index()].locked = true;
        if burden {
            in_play(&mut e, 0, "Burden the Mind", Some(x));
        }
        hand(&mut e, 1, &["On the Qui Vive"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, play(0, 0, x));
        pick_when(&mut e, block(x));
        request(&mut e);
        assert_eq!(
            e.state.seats[1].pool,
            if burden { 9 } else { 10 },
            "{burden}"
        );
    }
    let mut e = game(2);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let x = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    in_play(&mut e, 1, "Burden the Mind", Some(x));
    assert_eq!(e.state.unlock_toll_on(x), 0, "its own turn");
}

/// Burden the Mind: Horrific Countenance [pro] unlocking the blocking
/// minion out of its turn costs the acting Methuselah 1 pool more.
/// [Card text] "Using an effect to unlock this minion ... costs an
/// additional pool."
#[test]
fn burden_the_mind_taxes_an_unlock_of_the_blocker() {
    for burden in [false, true] {
        let (mut e, a, x) = duel(&["pro"], &[]);
        e.state.minions[a.index()].blood = 6;
        if burden {
            in_play(&mut e, 0, "Burden the Mind", Some(x));
        }
        hand(&mut e, 0, &["Horrific Countenance"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(x));
        pick_when(&mut e, play(0, 0, a));
        request(&mut e);
        assert!(!e.state.minions[x.index()].locked, "{burden}: unlocked");
        assert_eq!(
            e.state.seats[0].pool,
            if burden { 9 } else { 10 },
            "{burden}"
        );
    }
}

/// Burden the Mind on Helena: her "as if unlocked" for her last pool out of
/// her turn ousts her Methuselah.
/// [Card text] "... or to allow this minion to block as if unlocked costs
/// an additional pool." [rules-qa Batch 118] An oust happens as the pool
/// reaches 0.
#[test]
fn burden_the_mind_toll_on_an_ability_can_oust() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let h = e.add_minion(SeatId(1), find_card("Helena"), 6, false);
    e.add_minion(SeatId(2), plain("Ventrue", 0), 4, false);
    e.state.minions[h.index()].locked = true;
    e.state.seats[1].pool = 1;
    in_play(&mut e, 0, "Burden the Mind", Some(h));
    pick(&mut e, bleed(a));
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Vampire(h),
            ability: 0,
            actor: None,
            target: Target::None,
        },
    );
    request(&mut e);
    assert_eq!(e.state.seats[1].pool, 0);
    assert!(e.state.seats[1].ousted, "out of pool by the toll");
}

/// Burden the Mind: its minion burns it as an action directed at the card's
/// player; the card in that player's ash heap, the minion unlocked.
#[test]
fn burden_the_mind_is_burned_by_its_minions_action_which_unlocks_it() {
    let (mut e, a, _) = duel(&[], &[]);
    let b = in_play(&mut e, 1, "Burden the Mind", Some(a));
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Card(b),
            ability: 0,
            actor: Some(a),
            target: Target::None,
        },
    );
    to_minion_phase(&mut e, SeatId(0));
    assert!(!e.state.cards[b.index()].in_play);
    assert!(ash_has(&e, 1, "Burden the Mind"));
    assert!(!e.state.minions[a.index()].locked);
}

/// Lesser Boon: as its player's block would succeed, the action resolves as
/// if unblocked: the blocker unlocked, no "if blocked" effect (Hierophant),
/// no combat, the bleed lands; the card itself on the acting vampire, the
/// next master phase action spent. Ledger: `[RTR 20080808]`.
#[test]
fn lesser_boon_resolves_the_action_as_if_unblocked() {
    let (mut e, a, x) = duel(&[], &[]);
    in_play(&mut e, 0, "Hierophant", Some(a));
    hand(&mut e, 1, &["Lesser Boon"]);
    e.notes = Some(Vec::new());
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(1), WindowLabel::ResolutionChoice)
    );
    e.choose(index_of(&req, master_at(0, 0, Target::Minion(a))));
    to_minion_phase(&mut e, SeatId(0));
    assert!(!e.state.minions[x.index()].locked);
    assert_eq!(blood(&e, x), 4, "no Hierophant burn");
    assert!(noted_combats(&mut e).is_empty());
    assert_eq!(e.state.seats[1].pool, 9, "bled");
    placed_on(&e, "Lesser Boon", Some(a));
    assert!(e.state.seats[1].out_of_turn.is_some());
}

/// Lesser Boon: passed, the block succeeds as usual.
#[test]
fn lesser_boon_passed_the_block_succeeds() {
    let (mut e, a, x) = duel(&[], &[]);
    hand(&mut e, 1, &["Lesser Boon"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    pick(&mut e, Choice::Pass);
    to_minion_phase(&mut e, SeatId(0));
    assert!(e.state.minions[x.index()].locked);
    assert_eq!(e.state.seats[1].pool, 10, "blocked");
}

/// Lesser Boon: its minion cannot block the Boon player's minions, still
/// another's; the card burns as its minion's controller drops below 6
/// pool, and when the Boon's player blocks it again.
#[test]
fn lesser_boon_bars_its_minion_and_burns() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let p = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let q = e.add_minion(SeatId(2), plain("Toreador", 0), 4, false);
    let lb = in_play(&mut e, 1, "Lesser Boon", Some(a));
    assert!(e.state.cannot_block(a, p));
    assert!(!e.state.cannot_block(a, q));
    e.state.seats[0].pool = 5;
    request(&mut e);
    assert!(!e.state.cards[lb.index()].in_play, "below 6");
    // Blocked again by the player's minion: burned.
    let (mut e, a, x) = duel(&[], &[]);
    let lb = in_play(&mut e, 1, "Lesser Boon", Some(a));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    request(&mut e);
    assert!(!e.state.cards[lb.index()].in_play, "blocked again");
}

/// Blood Trade bars Lesser Boon: the block succeeds, no boon asked.
#[test]
fn blood_trade_bars_lesser_boon() {
    let (mut e, a, x) = duel(&[], &[]);
    in_play(&mut e, 0, "Blood Trade", None);
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::NoBoons,
        source: CardId(e.state.cards.len() as u32 - 1),
    });
    hand(&mut e, 1, &["Lesser Boon"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        matches!(
            o,
            Choice::Play {
                card: 0,
                actor: None,
                ..
            }
        )
    });
    assert!(e.state.minions[x.index()].locked);
}

/// Donnybrook [ser], now reachable (the Serpentis card): its steal targets a
/// minion with less than 2 blood, taking what it has. Ledger: `[RTR
/// 20010711]`.
#[test]
fn donnybrook_serpentis_steals_from_a_minion_with_less_than_two() {
    let (mut e, a, x) = duel(&["ser"], &[]);
    e.state.minions[a.index()].sect_override = Some(Sect::Anarch);
    e.state.minions[x.index()].blood = 1;
    hand(&mut e, 0, &["Donnybrook"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(0, 2, a));
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(blood(&e, x), 0);
    assert!(blood(&e, a) >= 4, "the stolen blood");
}

// ---- Vampires (vamp_b) -------------------------------------------------------

/// Vampire `m`'s printed ability `ability`, no target.
fn vamp(m: MinionId, ability: u8) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Vampire(m),
        ability,
        actor: None,
        target: Target::None,
    }
}

/// Seat 0's `name` and a plain second vampire, seat 1's two plain ones of
/// capacity 4 to 8, 4 blood each: (engine, `name`, seat 1's first).
fn lord(name: &str, blood: u8) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let mid = |c: &zvtes::crypt::CryptCard| (4..=8).contains(&c.capacity);
    let v = e.add_minion(SeatId(0), zvtes::state::find_card(name), blood, false);
    e.add_minion(SeatId(0), textless(mid, 0), 4, false);
    let x = e.add_minion(SeatId(1), textless(mid, 1), 4, false);
    e.add_minion(SeatId(1), textless(mid, 2), 4, false);
    (e, v, x)
}

/// Lizette, Papa Legba: "During a bleed action, … can burn 1 blood to get
/// +1 bleed": during their own bleed, once, before resolution [ANK
/// 20231124] [LSJ 19970718]; not in a hunt.
#[test]
fn lizette_and_papa_legba_burn_blood_for_a_bleed_once() {
    for (name, ability) in [("Lizette", 1), ("Papa Legba", 0)] {
        let (mut e, v, _) = lord(name, 4);
        pick(&mut e, bleed(v));
        pick_when(&mut e, vamp(v, ability));
        never_offered_until(
            &mut e,
            SeatId(1),
            WindowLabel::MinionPhase,
            vamp(v, ability),
        );
        assert_eq!(blood(&e, v), 3, "{name}");
        assert_eq!(e.state.seats[1].pool, 10 - 2, "{name}");
        // A hunt: not offered.
        let (mut e, v, _) = lord(name, 2);
        pick(&mut e, action(v, zvtes::decision::ActionKind::Hunt));
        never_offered_until(
            &mut e,
            SeatId(1),
            WindowLabel::MinionPhase,
            vamp(v, ability),
        );
    }
}

/// Lizette: "can prevent 1 damage each combat": offered with damage
/// pending, the opposing hand strike's 1 prevented.
#[test]
fn lizette_prevents_one_damage_in_combat() {
    let (mut e, v, x) = lord("Lizette", 4);
    to_minion_phase(&mut e, SeatId(1));
    pick(&mut e, bleed(x));
    pick_when(&mut e, block(v));
    pick_when(&mut e, vamp(v, 0));
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(blood(&e, v), 4, "her strength 1 damage prevented");
}

/// Sundervere: "+1 bleed" and "During an action, he can burn 1 blood to get
/// an additional +1 bleed": 2, then 3, once.
#[test]
fn sundervere_bleeds_two_and_buys_one_more() {
    let (mut e, v, _) = lord("Sundervere, The Devil Brahmin", 4);
    pick(&mut e, bleed(v));
    pick_when(&mut e, vamp(v, 0));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, vamp(v, 0));
    assert_eq!((blood(&e, v), e.state.seats[1].pool), (3, 10 - 3));
}

/// Marcia Felicia Licinia: "Once each combat, Marcia can burn 1 blood to
/// strike: dodge": the opposing hand strike has no effect, her blood paid.
#[test]
fn marcia_dodges_for_blood_once_a_combat() {
    let (mut e, v, x) = lord("Marcia Felicia Licinia", 4);
    to_minion_phase(&mut e, SeatId(1));
    pick(&mut e, bleed(x));
    pick_when(&mut e, block(v));
    pick_when(&mut e, vamp(v, 0));
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!((blood(&e, v), blood(&e, x)), (3, 4));
}

/// Margaret Milliner: "+1 bleed" while any Methuselah's Francis Milliner is
/// ready (rules-qa 73.7), and [for] cards cost her 1 more blood.
#[test]
fn margaret_milliner_bleeds_more_with_francis_ready_and_pays_more_for_fortitude() {
    use zvtes::state::PrintedCtx;
    let (mut e, v, _) = lord("Margaret Milliner", 4);
    let bleeding = PrintedCtx {
        kind: Some(zvtes::decision::ActionKind::Bleed),
        ..PrintedCtx::default()
    };
    let stat = zvtes::cards::Stat::Bleed;
    assert_eq!(e.state.printed_mod(v, stat, bleeding), 0);
    let f = e.add_minion(
        SeatId(1),
        zvtes::state::find_card("Francis Milliner"),
        4,
        false,
    );
    assert_eq!(e.state.printed_mod(v, stat, bleeding), 1);
    e.state.minions[f.index()].in_torpor = true;
    assert_eq!(e.state.printed_mod(v, stat, bleeding), 0);
    // Hidden Strength: played using [for].
    let card = |name: &str, mode: u8| PrintedCtx {
        card: Some((find_library(name), mode)),
        ..PrintedCtx::default()
    };
    let more = zvtes::cards::Stat::BloodCostMore;
    assert_eq!(e.state.printed_mod(v, more, card("Hidden Strength", 0)), 1);
    assert_eq!(e.state.printed_mod(v, more, card("Form of Mist", 0)), 0);
}

/// Phaedyme: "+1 stealth when bleeding a Methuselah who controls a ready
/// Camarilla vampire. +1 strength": read live on the bleed's target.
#[test]
fn phaedyme_gets_stealth_bleeding_a_camarilla_controller() {
    use zvtes::state::PrintedCtx;
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), zvtes::state::find_card("Phaedyme"), 4, false);
    let other = |c: &zvtes::crypt::CryptCard| c.sect != Some(Sect::Camarilla);
    e.add_minion(SeatId(1), textless(other, 0), 4, false);
    let cam = textless(|c| c.sect == Some(Sect::Camarilla), 0);
    let bleeding = PrintedCtx {
        kind: Some(zvtes::decision::ActionKind::Bleed),
        target: Some(SeatId(1)),
        ..PrintedCtx::default()
    };
    let stealth = zvtes::cards::Stat::Stealth;
    assert_eq!(e.state.printed_mod(v, stealth, bleeding), 0);
    let c = e.add_minion(SeatId(1), cam, 4, false);
    assert_eq!(e.state.printed_mod(v, stealth, bleeding), 1);
    let hunting = PrintedCtx {
        kind: Some(zvtes::decision::ActionKind::Hunt),
        ..bleeding
    };
    assert_eq!(e.state.printed_mod(v, stealth, hunting), 0);
    e.state.minions[c.index()].in_torpor = true;
    assert_eq!(e.state.printed_mod(v, stealth, bleeding), 0);
    let strength = e
        .state
        .printed_mod(v, zvtes::cards::Stat::Strength, PrintedCtx::default());
    assert_eq!(strength, 1);
}

/// Sajid al Misbah: [qui] cards cost him 1 fewer blood; "He cannot block
/// older vampires": an older acting vampire is never offered his block, a
/// younger one is.
#[test]
fn sajid_pays_less_for_quietus_and_cannot_block_older_vampires() {
    use zvtes::state::PrintedCtx;
    let mut e = game(2);
    let sajid = e.add_minion(
        SeatId(1),
        zvtes::state::find_card("Sajid al Misbah"),
        4,
        false,
    );
    e.add_minion(SeatId(1), textless(|c| c.capacity == 2, 0), 2, false);
    let old = e.add_minion(SeatId(0), textless(|c| c.capacity == 6, 0), 4, false);
    let young = e.add_minion(SeatId(0), textless(|c| c.capacity == 3, 0), 3, false);
    let qui = card_requiring("qui", 0);
    let ctx = PrintedCtx {
        card: Some((qui, 0)),
        ..PrintedCtx::default()
    };
    let off = zvtes::cards::Stat::BloodCostOff;
    assert_eq!(e.state.printed_mod(sajid, off, ctx), 1);
    pick(&mut e, bleed(old));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(sajid));
    pick(&mut e, bleed(young));
    assert!(offered_before(
        &mut e,
        block(sajid),
        SeatId(1),
        WindowLabel::MinionPhase
    ));
}

/// Vasiliy Vasilevich: "can unlock after successfully bleeding your prey.
/// +1 bleed": after his bleed of his prey, offered once; a bleed
/// redirected to his predator (Deflection) offers nothing.
#[test]
fn vasiliy_unlocks_after_bleeding_his_prey() {
    let (mut e, v, _) = lord("Vasiliy Vasilevich", 4);
    pick(&mut e, bleed(v));
    pick_when(&mut e, vamp(v, 0));
    assert!(!e.state.minions[v.index()].locked);
    assert_eq!(e.state.seats[1].pool, 10 - 2);
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, vamp(v, 0));
    // Three seats: seat 1 deflects the bleed to seat 2, his predator.
    let mut e = game(3);
    let v = e.add_minion(
        SeatId(0),
        zvtes::state::find_card("Vasiliy Vasilevich"),
        4,
        false,
    );
    e.add_minion(
        SeatId(0),
        textless(|c| (4..=8).contains(&c.capacity), 0),
        4,
        false,
    );
    let d = e.add_minion(SeatId(1), vampire_with("DOM", 0), 4, false);
    e.add_minion(
        SeatId(2),
        textless(|c| (4..=8).contains(&c.capacity), 1),
        4,
        false,
    );
    hand(&mut e, 1, &["Deflection"]);
    pick(&mut e, bleed(v));
    pick_when(&mut e, play_on(0, 0, d, Target::Seat(SeatId(2))));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, vamp(v, 0));
    assert_eq!(e.state.seats[2].pool, 10 - 2, "bled for 2");
    assert!(e.state.minions[v.index()].locked);
}

/// Vassily Taltos: "can burn a hunting ground or a card named <Blood Doll>
/// as a Ⓓ action": another Methuselah's hunting ground burned; a Blood Doll
/// targetable, another master not.
#[test]
fn vassily_taltos_burns_a_hunting_ground_or_a_blood_doll() {
    let (mut e, v, x) = lord("Vassily Taltos", 4);
    let ground = in_play(&mut e, 1, "Academic Hunting Ground", None);
    let doll = in_play(&mut e, 1, "Blood Doll", Some(x));
    let other = in_play(&mut e, 1, "Dreams of the Sphinx", None);
    let at = |id: CardId| Choice::UseAbility {
        source: AbilitySource::Vampire(v),
        ability: 0,
        actor: Some(v),
        target: Target::Card(id),
    };
    let req = request(&mut e);
    assert!(req.options.contains(&at(doll)), "{:?}", req.options);
    assert!(!req.options.contains(&at(other)));
    pick(&mut e, at(ground));
    to_minion_phase(&mut e, SeatId(1));
    assert!(!e.state.cards[ground.index()].in_play);
    assert!(ash_has(&e, 1, "Academic Hunting Ground"));
    assert!(e.state.cards[doll.index()].in_play);
}

/// Nakhthorheb: "Minions with any of your corruption counters cannot block
/// him": the prey's vampire with his Methuselah's counter is never offered
/// a block, another's is; "During your minion phase, Nakhthorheb can
/// unlock": offered after his bleed. A block that succeeded stands though
/// the blocker then gets a counter [LSJ 20071021].
#[test]
fn nakhthorheb_cannot_be_blocked_by_his_corrupted_and_unlocks_in_his_minion_phase() {
    let (mut e, n, y) = lord("Nakhthorheb", 4);
    let z = MinionId(3);
    e.state.minions[y.index()].add_corruption(SeatId(0), 1);
    pick(&mut e, bleed(n));
    let req = offering(&mut e, block(z));
    assert!(!req.options.contains(&block(y)), "{:?}", req.options);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(y));
    assert!(e.state.minions[n.index()].locked);
    pick(&mut e, vamp(n, 0));
    assert!(!e.state.minions[n.index()].locked);
    // Another's counter: the block succeeds; a counter of his after it
    // undoes nothing.
    let (mut e, n, y) = lord("Nakhthorheb", 4);
    e.state.minions[y.index()].add_corruption(SeatId(1), 1);
    pick(&mut e, bleed(n));
    pick_when(&mut e, block(y));
    e.state.minions[y.index()].add_corruption(SeatId(0), 1);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10, "blocked, no bleed");
    assert_eq!(blood(&e, y), 4 - 2, "his strength 2 struck it");
}

/// Mask of a Thousand Faces: after blocks are declined, the masked
/// minion's unblockability its own (Nakhthorheb) [PIB 20150924-1] [PIB
/// 20150930]: the only would-be blocker carries his Methuselah's counter,
/// and another vampire of his takes the bleed over.
#[test]
fn mask_of_a_thousand_faces_takes_over_nakhthorhebs_unblockable_bleed() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), zvtes::state::find_card("Nakhthorheb"), 4, false);
    let m = e.add_minion(SeatId(0), vampire_with("obf", 0), 4, false);
    let y = e.add_minion(
        SeatId(1),
        textless(|c| (4..=8).contains(&c.capacity), 0),
        4,
        false,
    );
    e.state.minions[y.index()].add_corruption(SeatId(0), 1);
    hand(&mut e, 0, &["Mask of a Thousand Faces"]);
    pick(&mut e, bleed(n));
    pick_when(&mut e, play(0, 0, m));
    request(&mut e);
    assert_eq!(e.action().expect("the bleed").acting, m);
    to_minion_phase(&mut e, SeatId(1));
    assert!(e.state.minions[m.index()].locked);
    assert!(!e.state.minions[n.index()].locked);
}

/// Paolo Sardenzo: "As a wraith enters play, Paolo can gain 1 blood": a
/// wraith recruited by another Methuselah; "Giovanni get +1 intercept
/// against him".
#[test]
fn paolo_sardenzo_gains_blood_as_a_wraith_enters_and_giovanni_intercept_him() {
    use zvtes::state::PrintedCtx;
    let mut e = game(2);
    let p = e.add_minion(
        SeatId(0),
        zvtes::state::find_card("Paolo Sardenzo"),
        4,
        false,
    );
    e.add_minion(
        SeatId(0),
        textless(|c| (4..=8).contains(&c.capacity), 0),
        4,
        false,
    );
    let h = e.add_minion(SeatId(1), plain("Hecata", 0), 4, false);
    let g = e.add_minion(SeatId(1), plain("Giovanni", 0), 4, false);
    let ctx = |o| PrintedCtx {
        opponent: Some(o),
        ..PrintedCtx::default()
    };
    let against = zvtes::cards::Stat::InterceptAgainst;
    assert_eq!(e.state.printed_mod(p, against, ctx(g)), 1);
    assert_eq!(e.state.printed_mod(p, against, ctx(h)), 0);
    to_minion_phase(&mut e, SeatId(1));
    hand(&mut e, 1, &["Screamer"]);
    pick(&mut e, play(0, 0, h));
    pick_when(&mut e, Choice::ResolvePrinted { minion: p });
    assert_eq!(blood(&e, p), 5);
}

/// Ogwon: "Any ally or younger vampire who attempts to block Ogwon and
/// fails burns 1 blood or life before action resolution": a younger vampire
/// and an ally failing (his stealth) burn 1 blood and 1 life, an older one
/// nothing; blocked by another, the action never resolves and nobody burns.
#[test]
fn ogwon_burns_younger_and_ally_failed_blockers_at_resolution() {
    for blocked in [false, true] {
        let mut e = game(2);
        let o = e.add_minion(SeatId(0), zvtes::state::find_card("Ogwon"), 4, false);
        e.add_minion(
            SeatId(0),
            textless(|c| (4..=8).contains(&c.capacity), 0),
            4,
            false,
        );
        let young = e.add_minion(SeatId(1), textless(|c| c.capacity == 5, 0), 4, false);
        // No textless vampire is older than Ogwon (7): Lazarus James (8),
        // "+1 bleed.", inert as a blocker.
        let old = e.add_minion(
            SeatId(1),
            zvtes::state::find_card("Lazarus James"),
            4,
            false,
        );
        // "+1 intercept": reaches Ogwon's stealth.
        let other = e.add_minion(
            SeatId(1),
            zvtes::state::find_card("Hafthor Thorsteinsson"),
            4,
            false,
        );
        let ally = e.add_ally(SeatId(1), find_library("Gravebound Drone"));
        stealthier(&mut e, o, 1);
        pick(&mut e, bleed(o));
        pick_when(&mut e, block(young));
        pick_when(&mut e, block(ally));
        if blocked {
            pick_when(&mut e, block(other));
        } else {
            pick_when(&mut e, block(old));
        }
        to_minion_phase(&mut e, SeatId(1));
        let burnt = u8::from(!blocked);
        assert_eq!(blood(&e, young), 4 - burnt, "blocked: {blocked}");
        assert_eq!(blood(&e, ally), 2 - burnt, "its life, blocked: {blocked}");
        assert_eq!(blood(&e, old), 4, "blocked: {blocked}");
        assert_eq!(e.state.seats[1].pool, 10 - burnt as i32);
    }
}

/// Shemti: "While Shemti is ready and unlocked, minions take 1
/// environmental damage after performing a successful action directed at
/// you": the acting Methuselah orders it with Vasiliy's unlock in the
/// after-resolution window, no pass before it [ANK 20180206]; with Shemti
/// locked, or on an undirected action, nothing.
#[test]
fn shemti_damages_a_successful_directed_actor_after_resolution() {
    for (locked, hunt) in [(false, false), (true, false), (false, true)] {
        let (mut e, s, _) = lord("Shemti", 4);
        let x = e.add_minion(
            SeatId(1),
            zvtes::state::find_card("Vasiliy Vasilevich"),
            4,
            false,
        );
        e.state.minions[s.index()].locked = locked;
        to_minion_phase(&mut e, SeatId(1));
        let kind = if hunt {
            zvtes::decision::ActionKind::Hunt
        } else {
            zvtes::decision::ActionKind::Bleed
        };
        pick(&mut e, action(x, kind));
        if locked || hunt {
            never_offered_until(
                &mut e,
                SeatId(0),
                WindowLabel::MinionPhase,
                Choice::TakeDamageAfterResolution,
            );
            let gain = if hunt { 1 } else { 0 };
            assert_eq!(blood(&e, x), 4 + gain, "locked {locked}, hunt {hunt}");
        } else {
            let req = until(&mut e, SeatId(1), WindowLabel::ActionAfterResolution);
            assert!(!req.options.contains(&Choice::Pass), "{:?}", req.options);
            assert!(req.options.contains(&vamp(x, 0)), "{:?}", req.options);
            e.choose(index_of(&req, Choice::TakeDamageAfterResolution));
            pick(&mut e, vamp(x, 0));
            to_minion_phase(&mut e, SeatId(0));
            assert_eq!(blood(&e, x), 3);
            assert_eq!(e.state.seats[0].pool, 10 - 2);
        }
    }
}

/// Shemti's damage is environmental [RTR 19970630]: an acting ally at 1
/// life takes it and burns.
#[test]
fn shemti_damage_is_environmental() {
    let (mut e, _, _) = lord("Shemti", 4);
    let ally = e.add_ally(SeatId(1), find_library("Gravebound Drone"));
    e.state.minions[ally.index()].blood = 1;
    to_minion_phase(&mut e, SeatId(1));
    pick(&mut e, bleed(ally));
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(e.state.seats[0].pool, 10 - 1, "bled");
    assert!(!e.state.minions[ally.index()].in_play);
    assert!(ash_has(&e, 1, "Gravebound Drone"));
}

/// Sundervere: "After Sundervere is burned, you burn 1 pool": burned in
/// torpor by another Methuselah's diablerie, his controller's pool.
#[test]
fn sundervere_burns_his_controllers_pool_when_burned_from_torpor() {
    let mut e = game(2);
    let s = e.add_minion(
        SeatId(1),
        zvtes::state::find_card("Sundervere, The Devil Brahmin"),
        2,
        true,
    );
    e.add_minion(
        SeatId(1),
        textless(|c| (4..=8).contains(&c.capacity), 0),
        4,
        false,
    );
    let d = e.add_minion(SeatId(0), textless(|c| c.capacity >= 7, 0), 4, false);
    e.add_minion(
        SeatId(0),
        textless(|c| (4..=8).contains(&c.capacity), 1),
        4,
        false,
    );
    pick(
        &mut e,
        Choice::TakeAction {
            minion: d,
            action: zvtes::decision::ActionKind::Diablerie,
            target: Some(s),
        },
    );
    to_minion_phase(&mut e, SeatId(1));
    assert!(!e.state.minions[s.index()].in_play);
    assert_eq!(e.state.seats[1].pool, 10 - 1);
    assert_eq!(e.state.seats[0].pool, 10);
}

/// Pass every decision until a referendum's polling step offers a card's
/// vote.
fn to_polling(e: &mut Engine) {
    to_referendum(e);
    for _ in 0..50 {
        let req = request(e);
        if req.window == WindowLabel::ReferendumPolling
            && req.options.contains(&Choice::CastCardVote { favor: true })
        {
            return;
        }
        e.choose(fallback(&req));
    }
    panic!("no polling");
}

/// Luna Giovanni: "During the polling step of any referendum, Luna can
/// remove X political action cards in any ash heaps from the game to get +X
/// votes": locked, two of the prey's and one of her Methuselah's removed
/// for 1 + 3 votes, once [ANK 20220705]; Charming Lobby is no political
/// action card [ANK 20220704] [LSJ 20070927] [LSJ 20050407], nor is the
/// card of the action under way, in limbo [LSJ 20070927].
#[test]
fn luna_giovanni_removes_political_cards_from_any_ash_heap_for_votes() {
    let mut e = game(2);
    let caller = e.add_minion(
        SeatId(0),
        textless(|c| c.sect == Some(Sect::Camarilla), 0),
        4,
        false,
    );
    let luna = e.add_minion(
        SeatId(0),
        zvtes::state::find_card("Luna Giovanni"),
        4,
        false,
    );
    e.state.minions[luna.index()].locked = true;
    e.add_minion(
        SeatId(1),
        textless(|c| (4..=8).contains(&c.capacity), 0),
        4,
        false,
    );
    let (kine, persecution, lobby) = (
        find_library("Kine Resources Contested"),
        find_library("Autarkis Persecution"),
        find_library("Charming Lobby"),
    );
    e.state.seats[1].ash.push(persecution);
    e.state.seats[1].ash.push(persecution);
    e.state.seats[0].ash.push(persecution);
    e.state.seats[0].ash.push(lobby);
    hand(&mut e, 0, &["Kine Resources Contested"]);
    pick(&mut e, play(0, 0, caller));
    to_polling(&mut e);
    pick(&mut e, vamp(luna, 0));
    // Each heap's one political action card is found at once (a single
    // option): Charming Lobby is never a second.
    let req = request(&mut e);
    assert!(!req.options.contains(&Choice::Pass), "one at least");
    for heap in [1, 1, 0] {
        pick(&mut e, Choice::ChooseSeat(SeatId(heap)));
    }
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::ReferendumPolling);
    assert_eq!(
        e.referendum().expect("referendum").votes_of(&e.state, luna),
        1 + 3
    );
    assert_eq!(e.state.seats[1].ash, Vec::<u16>::new());
    assert_eq!(e.state.seats[0].ash, vec![lobby]);
    let removed = |s: usize| {
        e.state.seats[s]
            .removed
            .iter()
            .filter(|&&c| c == persecution)
            .count()
    };
    assert_eq!((removed(0), removed(1)), (1, 2));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, vamp(luna, 0));
    assert!(
        e.state.seats[0].ash.contains(&kine),
        "the calling card, after"
    );
}

/// Luna Giovanni: "Cannot be used during a referendum that is
/// automatically passing." [PIB 20150105] [LSJ 19980107] Cryptic Rider's
/// passing referendum: never offered, though political action cards wait
/// in the ash heap.
#[test]
fn luna_giovanni_is_not_offered_on_an_automatic_pass() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let luna = e.add_minion(
        SeatId(0),
        zvtes::state::find_card("Luna Giovanni"),
        4,
        false,
    );
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[1]
        .ash
        .push(find_library("Kine Resources Contested"));
    hand(
        &mut e,
        0,
        &[
            "Autarkis Persecution",
            "Autarkis Persecution",
            "Cryptic Rider",
        ],
    );
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    close_polling(&mut e);
    pick_when(&mut e, play(1, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, play(0, 0, b));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, vamp(luna, 0));
    assert!(e.state.seats[1]
        .ash
        .contains(&find_library("Kine Resources Contested")));
}

/// Luna Giovanni, Marcia Felicia Licinia, Shemti: "Keeps their title and
/// votes if they changes sects." [LSJ 20010714] Made Anarch, 1, 2 and 1
/// votes in a polling.
#[test]
fn lords_of_the_night_personal_titles_survive_a_sect_change() {
    let mut e = game(2);
    let caller = e.add_minion(
        SeatId(0),
        textless(|c| c.sect == Some(Sect::Camarilla), 0),
        4,
        false,
    );
    let lords: Vec<(MinionId, u8)> = [
        ("Luna Giovanni", 1),
        ("Marcia Felicia Licinia", 2),
        ("Shemti", 1),
    ]
    .iter()
    .map(|&(name, n)| {
        let m = e.add_minion(SeatId(0), zvtes::state::find_card(name), 4, false);
        e.state.minions[m.index()].sect_override = Some(Sect::Anarch);
        (m, n)
    })
    .collect();
    e.add_minion(
        SeatId(1),
        textless(|c| (4..=8).contains(&c.capacity), 0),
        4,
        false,
    );
    hand(&mut e, 0, &["Kine Resources Contested"]);
    pick(&mut e, play(0, 0, caller));
    to_polling(&mut e);
    let r = e.referendum().expect("referendum");
    for (m, n) in lords {
        assert_eq!(r.votes_of(&e.state, m), n);
    }
}

/// The `nth` crypt vampire titled as `t` matches, with no printed-text
/// definition.
fn titled(t: impl Fn(zvtes::crypt::Title) -> bool, nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| t(c.title) && !c.adv && zvtes::cards::vampire(*i as u16).is_none())
        .nth(nth)
        .map(|(i, _)| i as u16)
        .expect("a titled textless vampire")
}

/// Monty Coven bleeds with 2 aggravated hand damage; seat 1's `victim` (0
/// blood) blocks and burns. The request offering his bishop ability.
fn monty_burns(victim: u16) -> (Engine, MinionId, zvtes::decision::DecisionRequest) {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), zvtes::state::find_card("Monty Coven"), 4, false);
    e.add_minion(
        SeatId(0),
        textless(|c| (4..=8).contains(&c.capacity), 0),
        4,
        false,
    );
    let v = e.add_minion(SeatId(1), victim, 0, false);
    e.add_minion(
        SeatId(1),
        textless(|c| (4..=8).contains(&c.capacity), 1),
        4,
        false,
    );
    stronger(&mut e, m, 1);
    let owner = SeatId(0);
    e.state.minions[m.index()].timed.push((
        zvtes::cards::Lasting::AggravatedHands,
        zvtes::state::Expiry::EndOfTurn,
        owner,
    ));
    pick(&mut e, bleed(m));
    pick_when(&mut e, block(v));
    let req = offering(&mut e, vamp(m, 0));
    assert!(!e.state.minions[v.index()].in_play, "burned");
    (e, m, req)
}

/// Monty Coven: "After Monty burns a titled vampire, he can become a
/// bishop": a primogen burned in combat, the bishop offered at the round's
/// end, not the priscus; an untitled one, nothing.
#[test]
fn monty_coven_becomes_a_bishop_after_burning_a_titled_vampire() {
    use zvtes::crypt::Title;
    let (mut e, m, req) = monty_burns(titled(|t| t == Title::Primogen, 0));
    assert!(!req.options.contains(&vamp(m, 1)), "{:?}", req.options);
    e.choose(index_of(&req, vamp(m, 0)));
    request(&mut e);
    assert_eq!(e.state.title_of(m), Title::Bishop);
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, vamp(m, 1));
    // Untitled.
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), zvtes::state::find_card("Monty Coven"), 4, false);
    e.add_minion(
        SeatId(0),
        textless(|c| (4..=8).contains(&c.capacity), 0),
        4,
        false,
    );
    let v = e.add_minion(
        SeatId(1),
        textless(|c| (4..=8).contains(&c.capacity), 1),
        0,
        false,
    );
    e.add_minion(
        SeatId(1),
        textless(|c| (4..=8).contains(&c.capacity), 2),
        4,
        false,
    );
    stronger(&mut e, m, 1);
    e.state.minions[m.index()].timed.push((
        zvtes::cards::Lasting::AggravatedHands,
        zvtes::state::Expiry::EndOfTurn,
        SeatId(0),
    ));
    pick(&mut e, bleed(m));
    pick_when(&mut e, block(v));
    never_offered_where(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(x), .. } if *x == m),
    );
    assert!(!e.state.minions[v.index()].in_play);
    assert_eq!(e.state.title_of(m), Title::None);
}

/// Monty Coven: "If the burned vampire's title was worth 3 or more votes,
/// he can become a priscus instead": a justicar burned, both offered, the
/// priscus taken.
#[test]
fn monty_coven_may_become_a_priscus_after_burning_a_justicar() {
    use zvtes::crypt::Title;
    let (mut e, m, req) = monty_burns(titled(|t| matches!(t, Title::Justicar(_)), 0));
    e.choose(index_of(&req, vamp(m, 1)));
    request(&mut e);
    assert_eq!(e.state.title_of(m), Title::Priscus);
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, vamp(m, 0));
}

/// Monty Coven, red reading: a priscus's title is worth no votes (rules-qa
/// 90.8): burning one, only the bishop.
#[test]
fn monty_burning_a_priscus_becomes_only_a_bishop() {
    use zvtes::crypt::Title;
    let (_, m, req) = monty_burns(titled(|t| t == Title::Priscus, 0));
    assert!(req.options.contains(&vamp(m, 0)));
    assert!(!req.options.contains(&vamp(m, 1)));
}

/// Monty Coven: a diablerie of his burns the victim, before its blood hunt
/// (RTR 19991206, as Taking the Skin): the bishop offered after it.
#[test]
fn monty_coven_becomes_a_bishop_after_diablerizing_a_titled_vampire() {
    use zvtes::crypt::Title;
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), zvtes::state::find_card("Monty Coven"), 4, false);
    e.add_minion(
        SeatId(0),
        textless(|c| (4..=8).contains(&c.capacity), 0),
        4,
        false,
    );
    let v = e.add_minion(SeatId(1), titled(|t| t == Title::Primogen, 0), 0, true);
    e.add_minion(
        SeatId(1),
        textless(|c| (4..=8).contains(&c.capacity), 1),
        4,
        false,
    );
    pick(
        &mut e,
        Choice::TakeAction {
            minion: m,
            action: zvtes::decision::ActionKind::Diablerie,
            target: Some(v),
        },
    );
    let req = offering(&mut e, vamp(m, 2));
    assert!(e.referendum().is_none(), "before the blood hunt");
    e.choose(index_of(&req, vamp(m, 2)));
    request(&mut e);
    assert_eq!(e.state.title_of(m), Title::Bishop);
}

/// Lorenzo Detuono (7 blood) bleeds; seat 1's `blocker` blocks: (engine,
/// Lorenzo, blocker).
fn lorenzo_blocked_by(
    e: &mut Engine,
    blocker: impl FnOnce(&mut Engine) -> MinionId,
) -> (MinionId, MinionId) {
    let l = e.add_minion(
        SeatId(0),
        zvtes::state::find_card("Lorenzo Detuono"),
        7,
        false,
    );
    e.add_minion(
        SeatId(0),
        textless(|c| (4..=8).contains(&c.capacity), 0),
        4,
        false,
    );
    let b = blocker(e);
    e.add_minion(
        SeatId(1),
        textless(|c| (4..=8).contains(&c.capacity), 1),
        4,
        false,
    );
    pick(e, bleed(l));
    pick_when(e, block(b));
    (l, b)
}

/// Lorenzo Detuono: "If Lorenzo is ready at the end of combat and the
/// opposing minion would not, instead Lorenzo can burn 5 blood to move the
/// opposing minion to your ready region, with 1 life for an ally with no
/// life": an ally at 1 life struck for 1 is his Methuselah's, with 1 life,
/// the combat over.
#[test]
fn lorenzo_takes_an_ally_about_to_burn_for_five_blood() {
    let mut e = game(2);
    let (l, ally) = lorenzo_blocked_by(&mut e, |e| {
        let a = e.add_ally(SeatId(1), find_library("Gravebound Drone"));
        e.state.minions[a.index()].blood = 1;
        a
    });
    pick_when(&mut e, vamp(l, 0));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let v = &e.state.minions[ally.index()];
    assert!(v.in_play && !v.in_torpor);
    assert_eq!((v.owner, v.blood), (SeatId(0), 1));
    assert!(!ash_has(&e, 1, "Gravebound Drone"));
    assert_eq!(blood(&e, l), 7 - 1 - 5, "its hit mended, then the 5");
    assert_eq!(e.state.seats[1].pool, 10, "blocked");
}

/// Lorenzo Detuono: a vampire wounded by his strike is his Methuselah's,
/// ready, at the blood it kept (none).
#[test]
fn lorenzo_takes_a_vampire_going_to_torpor() {
    let mut e = game(2);
    let (l, v) = lorenzo_blocked_by(&mut e, |e| {
        e.add_minion(
            SeatId(1),
            textless(|c| (4..=8).contains(&c.capacity), 2),
            0,
            false,
        )
    });
    pick_when(&mut e, vamp(l, 0));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let x = &e.state.minions[v.index()];
    assert!(x.in_play && !x.in_torpor);
    assert_eq!((x.owner, x.blood), (SeatId(0), 0));
}

/// Lorenzo Detuono, red reading: a vampire about to be burned by aggravated
/// damage "would not" be ready either: taken at 0 blood.
#[test]
fn lorenzo_takes_a_vampire_aggravated_damage_would_burn() {
    let mut e = game(2);
    let (l, v) = lorenzo_blocked_by(&mut e, |e| {
        e.add_minion(
            SeatId(1),
            textless(|c| (4..=8).contains(&c.capacity), 2),
            0,
            false,
        )
    });
    stronger(&mut e, l, 1);
    e.state.minions[l.index()].timed.push((
        zvtes::cards::Lasting::AggravatedHands,
        zvtes::state::Expiry::EndOfTurn,
        SeatId(0),
    ));
    pick_when(&mut e, vamp(l, 0));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let x = &e.state.minions[v.index()];
    assert!(x.in_play && !x.in_torpor);
    assert_eq!(x.owner, SeatId(0));
}

/// Lorenzo Detuono: "Cannot use his ability after Amaranth resolves." [ANK
/// 20210705] Amaranth played first diablerizes the vampire and Lorenzo is
/// never offered; Lorenzo's first, Amaranth is not (RTR 20010711).
#[test]
fn lorenzo_is_not_offered_after_amaranth() {
    for amaranth_first in [true, false] {
        let mut e = game(2);
        hand(&mut e, 0, &["Amaranth"]);
        let (l, v) = lorenzo_blocked_by(&mut e, |e| {
            e.add_minion(SeatId(1), textless(|c| c.capacity <= 6, 0), 0, false)
        });
        let amaranth = play(0, 0, l);
        let req = offering(&mut e, vamp(l, 0));
        assert!(req.options.contains(&amaranth), "{:?}", req.options);
        if amaranth_first {
            e.choose(index_of(&req, amaranth));
            never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
                *o == vamp(l, 0)
            });
            assert!(!e.state.minions[v.index()].in_play, "diablerized");
        } else {
            e.choose(index_of(&req, vamp(l, 0)));
            never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, amaranth);
            assert_eq!(e.state.minions[v.index()].owner, SeatId(0));
        }
    }
}

/// Lorenzo Detuono: "If Lorenzo is ready at the end of combat": going to
/// torpor himself, not offered.
#[test]
fn lorenzo_is_not_offered_while_going_down() {
    let mut e = game(2);
    let (l, v) = lorenzo_blocked_by(&mut e, |e| {
        e.add_minion(
            SeatId(1),
            textless(|c| (4..=8).contains(&c.capacity), 2),
            0,
            false,
        )
    });
    stronger(&mut e, v, 7);
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, vamp(l, 0));
    assert!(e.state.minions[l.index()].in_torpor);
    assert_eq!(e.state.minions[v.index()].owner, SeatId(1));
}

/// Pentweret (4 blood) bleeds; seat 1's vampire of capacity `cap` (4 blood)
/// blocks, `dodge` in its hand: (engine, Pentweret, blocker).
fn pentweret_vs(cap: u8, blood_: u8, dodge: bool) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let p = e.add_minion(
        SeatId(0),
        zvtes::state::find_card("Pentweret"),
        blood_,
        false,
    );
    e.add_minion(
        SeatId(0),
        textless(|c| (4..=8).contains(&c.capacity), 0),
        4,
        false,
    );
    let x = e.add_minion(
        SeatId(1),
        textless(|c| c.capacity == cap, 0),
        cap.min(4),
        false,
    );
    e.add_minion(
        SeatId(1),
        textless(|c| (4..=8).contains(&c.capacity), 1),
        4,
        false,
    );
    if dodge {
        hand(&mut e, 1, &["Dodge"]);
    }
    pick(&mut e, bleed(p));
    pick_when(&mut e, block(x));
    (e, p, x)
}

/// Pentweret: "In combat with … a vampire with capacity 3 or less, Pentweret
/// can burn 2 blood to strike: steal that minion": the capacity-3 vampire
/// is his Methuselah's for good, the combat over; not offered against
/// capacity 4, nor with 1 blood.
#[test]
fn pentweret_steals_a_small_minion_with_a_strike() {
    let (mut e, p, x) = pentweret_vs(3, 4, false);
    pick_when(&mut e, vamp(p, 0));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[x.index()].owner, SeatId(0));
    assert!(e.state.minions[x.index()].ready());
    assert_eq!(blood(&e, p), 4 - 2 - 1, "paid 2, its hand strike mended");
    for (cap, b) in [(4, 4), (3, 1)] {
        let (mut e, p, x) = pentweret_vs(cap, b, false);
        never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, vamp(p, 0));
        assert_eq!(e.state.minions[x.index()].owner, SeatId(1), "{cap} {b}");
    }
}

/// Pentweret: a dodge stops his strike, nothing stolen, the 2 blood spent.
#[test]
fn pentweret_steals_nothing_against_a_dodge() {
    let (mut e, p, x) = pentweret_vs(3, 4, true);
    pick_when(&mut e, vamp(p, 0));
    pick_when(&mut e, play(0, 0, x));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[x.index()].owner, SeatId(1));
    assert_eq!(blood(&e, p), 4 - 2);
}

/// Seat 0 influences out `name` (its slot full) in its influence phase:
/// the new minion.
fn influence_out(e: &mut Engine, name: &str) -> MinionId {
    let card = zvtes::state::find_card(name);
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card,
            copy: Default::default(),
            counters: CRYPT[card as usize].capacity,
            banished: None,
        });
    until(e, SeatId(0), WindowLabel::InfluenceTransfers);
    let slot = e.state.seats[0].uncontrolled.len() as u8 - 1;
    pick(e, Choice::InfluenceOut { slot });
    MinionId(e.state.minions.len() as u32 - 1)
}

/// Porphyrion: "As Porphyrion enters play, you can search your library for
/// a card named <Treaty of Laibach> … (shuffle and discard down
/// afterward). +1 stealth." The search optional: taken, Treaty of Laibach
/// out of the pool, it finds nothing and shuffles (rules-qa 80.9);
/// declined, no shuffle; either before his contest with another
/// Methuselah's copy [LSJ 20080526]; +1 stealth on his actions.
#[test]
fn porphyrion_searches_for_the_treaty_before_his_contest() {
    for search in [true, false] {
        let mut e = game(2);
        e.add_minion(
            SeatId(0),
            textless(|c| (4..=8).contains(&c.capacity), 0),
            4,
            false,
        );
        e.add_minion(SeatId(1), zvtes::state::find_card("Porphyrion"), 4, false);
        let names = [
            "Deflection",
            "Dodge",
            "Form of Mist",
            "Amaranth",
            "Blood Doll",
            "Screamer",
            "Hidden Strength",
            "Charming Lobby",
        ];
        fill_hand(&mut e, 0);
        for n in names {
            e.state.seats[0].library.push(find_library(n));
        }
        let before: Vec<u16> = e.state.seats[0].library.iter().copied().collect();
        let hand_before = e.state.seats[0].hand.len();
        let p = influence_out(&mut e, "Porphyrion");
        let req = request(&mut e);
        let yes = Choice::ResolvePrinted { minion: p };
        assert_eq!(req.options, vec![Choice::Pass, yes]);
        e.choose(index_of(&req, if search { yes } else { Choice::Pass }));
        request(&mut e);
        let after: Vec<u16> = e.state.seats[0].library.iter().copied().collect();
        if search {
            assert_ne!(after, before, "shuffled");
            let (mut a, mut b) = (after, before);
            a.sort_unstable();
            b.sort_unstable();
            assert_eq!(a, b);
        } else {
            assert_eq!(after, before, "no search, no shuffle");
        }
        assert_eq!(e.state.seats[0].hand.len(), hand_before);
        assert!(e.state.minions[p.index()].contested, "then contested");
    }
    let (e, q, _) = lord("Porphyrion", 4);
    let stealth = zvtes::cards::Stat::Stealth;
    let ctx = zvtes::state::PrintedCtx::default();
    assert_eq!(e.state.printed_mod(q, stealth, ctx), 1);
}

/// Seterpenre: "As Seterpenre enters play, put up to four master Discipline
/// cards on him from your hand, ash heap and/or library (shuffle
/// afterward). Discipline cards do not affect his capacity." Two from the
/// library, one from the ash heap, one from hand (replaced): Chimerstry and
/// Quietus at superior, his capacity still 10.
#[test]
fn seterpenre_takes_four_masters_and_keeps_his_capacity() {
    use zvtes::decision::Zone;
    let mut e = game(2);
    e.add_minion(
        SeatId(0),
        textless(|c| (4..=8).contains(&c.capacity), 0),
        4,
        false,
    );
    e.add_minion(
        SeatId(1),
        textless(|c| (4..=8).contains(&c.capacity), 1),
        4,
        false,
    );
    let (chi, qui, filler) = (
        find_library("Chimerstry"),
        find_library("Quietus"),
        find_library("Form of Mist"),
    );
    fill_hand(&mut e, 0);
    for c in [filler, filler, filler, chi, qui] {
        e.state.seats[0].library.push(c);
    }
    e.state.seats[0].ash.push(qui);
    e.state.seats[0].hand.push(chi);
    let s = influence_out(&mut e, "Seterpenre");
    for (zone, card) in [
        (Zone::Library, chi),
        (Zone::Library, qui),
        (Zone::AshHeap, qui),
        (Zone::Hand, chi),
    ] {
        let req = request(&mut e);
        assert_eq!(req.window, WindowLabel::LibrarySearch);
        e.choose(index_of(&req, Choice::TakeFrom { zone, card }));
    }
    request(&mut e);
    let st = &e.state;
    assert_eq!(st.capacity_of(s), Some(10));
    assert_eq!(st.discipline_level(s, "chi"), 2);
    assert_eq!(st.discipline_level(s, "qui"), 2);
    assert!(!st.seats[0].ash.contains(&qui));
    assert!(!st.seats[0].hand.contains(&chi));
    assert!(st.seats[0].library.iter().all(|&c| c == filler));
    // A pass after one stops.
    let mut e = game(2);
    e.add_minion(
        SeatId(0),
        textless(|c| (4..=8).contains(&c.capacity), 0),
        4,
        false,
    );
    e.add_minion(
        SeatId(1),
        textless(|c| (4..=8).contains(&c.capacity), 1),
        4,
        false,
    );
    fill_hand(&mut e, 0);
    e.state.seats[0].ash.push(qui);
    e.state.seats[0].ash.push(chi);
    let s = influence_out(&mut e, "Seterpenre");
    pick(
        &mut e,
        Choice::TakeFrom {
            zone: Zone::AshHeap,
            card: qui,
        },
    );
    pick(&mut e, Choice::Pass);
    request(&mut e);
    assert_eq!(e.state.discipline_level(s, "qui"), 1);
    assert!(e.state.seats[0].ash.contains(&chi));
}

/// Seterpenre: "His 'as he enters play' ability can be used before he
/// enters contest if there is another one in play." [LSJ 20080526] Another
/// Methuselah's copy in play: the cards are offered while he is not yet
/// contested.
#[test]
fn seterpenre_puts_masters_before_his_contest() {
    use zvtes::decision::Zone;
    let mut e = game(2);
    e.add_minion(
        SeatId(0),
        textless(|c| (4..=8).contains(&c.capacity), 0),
        4,
        false,
    );
    e.add_minion(SeatId(1), zvtes::state::find_card("Seterpenre"), 4, false);
    let qui = find_library("Quietus");
    fill_hand(&mut e, 0);
    e.state.seats[0].ash.push(qui);
    let s = influence_out(&mut e, "Seterpenre");
    let req = request(&mut e);
    assert!(
        req.options.contains(&Choice::TakeFrom {
            zone: Zone::AshHeap,
            card: qui
        }),
        "{req:?}"
    );
    assert!(!e.state.minions[s.index()].contested, "before the contest");
    e.choose(index_of(
        &req,
        Choice::TakeFrom {
            zone: Zone::AshHeap,
            card: qui,
        },
    ));
    request(&mut e);
    assert!(e.state.minions[s.index()].contested);
}

// ---- Investment cards ------------------------------------------------------------

/// Alan Sovereign's card: the advanced one or the base one.
fn alan(adv: bool) -> u16 {
    CRYPT
        .iter()
        .position(|c| c.name == "Alan Sovereign" && c.adv == adv)
        .expect("Alan Sovereign") as u16
}

/// Alan Sovereign (advanced): "While Alan is ready, you may pay some or all
/// of the pool cost of equipping from any investment cards you control."
/// [LSJ 20080125] "Paying from another source (eg. Alan Sovereign (ADV))
/// does not reduce the cost as seen by Concealed Weapon": with 5 counters
/// on Secret Horde, the AK-47 (5 pool) is still not offered; the .44
/// Magnum's 2 pool come off the Horde. Alan in torpor: from the pool.
#[test]
fn alan_sovereign_advanced_pays_an_equip_from_his_investments() {
    for ready in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Ravnos", 0), 4, false);
        e.add_minion(SeatId(0), alan(true), 4, !ready);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let horde = in_play(&mut e, 0, "Secret Horde", None);
        e.state.cards[horde.index()].counters = 5;
        hand(&mut e, 0, &["Concealed Weapon", ".44 Magnum", "AK-47"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        pick_when(&mut e, play(0, 0, a));
        let req = request(&mut e);
        let ak = Choice::EmployFromHand {
            card: find_library("AK-47"),
            mode: 0,
        };
        assert!(!req.options.contains(&ak), "{req:?}");
        let from_horde = Choice::ResolveRider { card: horde.0 };
        assert_eq!(req.options.contains(&from_horde), ready, "{req:?}");
        if ready {
            pick(&mut e, from_horde);
            pick(&mut e, Choice::SetX(2));
            request(&mut e);
        }
        assert_eq!(pool(&e)[0], if ready { 10 } else { 8 }, "ready {ready}");
        assert_eq!(
            e.state.cards[horde.index()].counters,
            if ready { 3 } else { 5 }
        );
        assert!(e.state.seats[0].hand.contains(&find_library("AK-47")));
    }
}

/// Alan Sovereign merged: "[MERGED] During your master phase, if Alan is
/// ready, you may move a counter from any investment card to your pool."
/// Another Methuselah's Secret Horde ("any", where his payment says "you
/// control"), once in the phase (rules-qa 6.6); its last counter taken, it
/// burns ("Burn this card when it has no counters").
#[test]
fn alan_sovereign_merged_takes_a_counter_from_any_investment() {
    let mut e = game(2);
    let al = e.add_minion(SeatId(0), alan(true), 4, false);
    e.state.minions[al.index()].merged = Some(alan(false));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let horde = in_play(&mut e, 1, "Secret Horde", None);
    e.state.cards[horde.index()].counters = 2;
    let take = Choice::UseAbility {
        source: AbilitySource::Vampire(al),
        ability: 0,
        actor: None,
        target: Target::Card(horde),
    };
    let req = to_master(&mut e);
    assert!(own_use(&req, al, 0).is_some(), "{req:?}");
    pick(&mut e, own_use(&req, al, 0).unwrap());
    let req = request(&mut e);
    assert_eq!(pool(&e)[0], 11);
    assert_eq!(e.state.cards[horde.index()].counters, 1);
    assert!(own_use(&req, al, 0).is_none(), "once a phase: {req:?}");
    let req = to_master(&mut e);
    assert_eq!(own_use(&req, al, 0), Some(take), "{req:?}");
    pick(&mut e, take);
    request(&mut e);
    assert_eq!(pool(&e)[0], 12);
    assert!(!e.state.cards[horde.index()].in_play, "burned empty");
    assert!(ash_has(&e, 1, "Secret Horde"));
}

/// Wall Street Night: "A minion you control can lock this location to
/// attempt to move 1 counter from an investment card to your pool as a +1
/// stealth Ⓓ action." At another Methuselah's Swiss Cut: directed at them,
/// the location locked as the action is declared; its last blood taken,
/// Swiss Cut burns ("Burn this card if it has no blood").
#[test]
fn wall_street_night_takes_a_counter_from_an_investment_as_an_action() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let wsn = in_play(&mut e, 0, "Wall Street Night, Financial Newspaper", None);
    let sc = in_play(&mut e, 1, "Swiss Cut", None);
    e.state.cards[sc.index()].counters = 1;
    pick(&mut e, use_on(wsn, 1, Some(a), Target::Card(sc)));
    request(&mut e);
    let act = e.action().expect("action");
    assert_eq!(act.kind, ActionKind::Ability);
    assert_eq!(act.directed_seat(&e.state), Some(SeatId(1)));
    assert_eq!(act.stealth(&e.state), 1);
    assert!(e.state.cards[wsn.index()].locked);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[0], 11);
    assert!(!e.state.cards[sc.index()].in_play, "burned empty");
    assert!(ash_has(&e, 1, "Swiss Cut"));
}

// ---- Gaps closed after the merge -------------------------------------------------

/// Johann Matheson: "Cards named Ecstasy, The Jones and XTC-Laced Blood
/// cannot target Johann." Neither names a target; the engine reads both as
/// aimed at him (rules-qa 121.75): Ecstasy is not offered against
/// his bleed (it is against another bleeder's), The Jones not in combat
/// with him (it is against another younger vampire).
#[test]
fn johann_matheson_shuns_ecstasy_and_the_jones() {
    let mut e = game(2);
    let johann = e.add_minion(SeatId(0), find_card("Johann Matheson"), 4, false);
    let other = e.add_minion(SeatId(0), textless(|c| c.capacity == 4, 0), 4, false);
    let x = e.add_minion(SeatId(1), textless(|c| c.capacity == 7, 0), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity == 5, 0), 4, false);
    e.state.minions[x.index()].gained_levels = vec!["ser"];
    hand(&mut e, 1, &["Ecstasy"]);
    pick(&mut e, bleed(johann));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, x));
    assert_eq!(e.state.seats[1].pool, 9, "Johann's bleed of 1");
    pick(&mut e, bleed(other));
    offering(&mut e, play(0, 0, x));

    for (actor, offered) in [(johann, false), (other, true)] {
        let mut e = game(2);
        let johann = e.add_minion(SeatId(0), find_card("Johann Matheson"), 4, false);
        let other = e.add_minion(SeatId(0), textless(|c| c.capacity == 4, 0), 4, false);
        let x = e.add_minion(SeatId(1), textless(|c| c.capacity == 7, 0), 4, false);
        e.add_minion(SeatId(1), textless(|c| c.capacity == 5, 0), 4, false);
        e.state.minions[x.index()].gained_levels = vec!["ser"];
        hand(&mut e, 1, &["Jones, The"]);
        let a = if actor == johann { johann } else { other };
        bleed_blocked(&mut e, a, x);
        if offered {
            offering(&mut e, play(0, 0, x));
        } else {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, x));
        }
    }
}

/// Ogwon: a younger vampire failing to block him twice in one action burns
/// 1 blood for each failure, as the action resolves (rules-qa 121.108): its
/// attempt left for the older vampire's fails (48.3), is offered again once
/// something changed (48.6), and fails again at its closure.
#[test]
fn ogwon_burns_a_blocker_failing_twice_twice() {
    let mut e = game(2);
    let o = e.add_minion(SeatId(0), zvtes::state::find_card("Ogwon"), 4, false);
    e.add_minion(
        SeatId(0),
        textless(|c| (4..=8).contains(&c.capacity), 0),
        4,
        false,
    );
    let young = e.add_minion(SeatId(1), textless(|c| c.capacity == 5, 0), 4, false);
    // Older than Ogwon, "+1 bleed.": inert as a blocker.
    let old = e.add_minion(
        SeatId(1),
        zvtes::state::find_card("Lazarus James"),
        4,
        false,
    );
    // Ogwon's age (7), not younger: keeps the block window open.
    e.add_minion(SeatId(1), textless(|c| c.capacity == 7, 0), 4, false);
    stealthier(&mut e, o, 1);
    pick(&mut e, bleed(o));
    pick_when(&mut e, block(young));
    pick(&mut e, block(old));
    let req = expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(!offers_block(&req, young), "nothing changed (48.6)");
    e.state.seats[1].library.push(find_library("Soak"));
    let req = expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(offers_block(&req, young));
    pick(&mut e, block(young));
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(e.state.seats[1].pool, 9, "the bleed resolved");
    assert_eq!(blood(&e, young), 4 - 1 - 1, "burned for each failure");
    assert_eq!(blood(&e, old), 4, "older: nothing");
}

/// Divine Image [ser]: "can prevent 1 damage each combat": Psyche! queues a
/// second combat of the same action; the prevention is offered again
/// there, each of B's hand strikes prevented.
#[test]
fn divine_image_prevents_in_each_combat_of_the_action() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("CEL", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["ser"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Divine Image", "Psyche!"]);
    pick(&mut e, bleed(a));
    expect(&mut e, SeatId(0), WindowLabel::ActionAsAnnounced);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, Choice::UsePrevent);
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, Choice::UsePrevent);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "Psyche!"));
    assert_eq!(blood(&e, b), 6 - 2 - 2, "two combats at strength 2");
    assert_eq!(blood(&e, a), 4 - 1, "its cost; both strikes prevented");
}

/// Forger's Hammer: no weapon goes in play from hand — Topaz takes the
/// prey's .44 Magnum instead ("discard the original equipment card"): the
/// Hammer stays in the ash heap, on nothing (its readings' FH3; Ghouled's
/// precedent, rules-qa 111.16).
#[test]
fn forgers_hammer_stays_in_the_ash_heap_with_no_weapon_from_hand() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), find_card("Topaz"), 7, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Bang Nakh — Tiger's Claws", "Forger's Hammer"]);
    hand(&mut e, 1, &[".44 Magnum"]);
    pick(&mut e, play(0, 0, t));
    pick_when(&mut e, play(0, 0, t));
    pick_when(&mut e, Choice::FindCard(find_library(".44 Magnum")));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    card_on(&e, ".44 Magnum", t);
    assert!(ash_has(&e, 0, "Bang Nakh — Tiger's Claws"), "discarded");
    assert!(ash_has(&e, 0, "Forger's Hammer"));
    let hammer = find_library("Forger's Hammer");
    assert!(!e.state.cards.iter().any(|c| c.card == hammer && c.in_play));
}

/// Nightstick's guard: "prevent 3 damage … from the opposing minion's next
/// hand … strike": X's aggravated hand strike (2) prevented in full (its
/// readings' NS5: the text names no kind).
#[test]
fn nightstick_guard_prevents_aggravated_damage_too() {
    let (mut e, a, x) = duel(&[], &[]);
    stronger(&mut e, x, 1);
    e.state.minions[x.index()].timed.push((
        zvtes::cards::Lasting::AggravatedHands,
        zvtes::state::Expiry::EndOfTurn,
        SeatId(1),
    ));
    let stick = borne(&mut e, "Nightstick", a);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, card_use(stick, 1));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4);
    assert!(!e.state.minions[a.index()].in_torpor);
}

/// Pentweret: "In combat with an ally costing 3 or fewer blood or pool …
/// steal that minion": Loyal Street Gang (1 pool) stolen; Renegade Garou
/// (5 pool) not offered.
#[test]
fn pentweret_steals_an_ally_costing_three_or_less() {
    for (name, offered) in [("Loyal Street Gang", true), ("Renegade Garou", false)] {
        let mut e = game(2);
        let p = e.add_minion(SeatId(0), zvtes::state::find_card("Pentweret"), 4, false);
        e.add_minion(
            SeatId(0),
            textless(|c| (4..=8).contains(&c.capacity), 0),
            4,
            false,
        );
        e.add_minion(
            SeatId(1),
            textless(|c| (4..=8).contains(&c.capacity), 1),
            4,
            false,
        );
        let ally = e.add_ally(SeatId(1), find_library(name));
        pick(&mut e, bleed(p));
        pick_when(&mut e, block(ally));
        if offered {
            pick_when(&mut e, vamp(p, 0));
            until(&mut e, SeatId(1), WindowLabel::MinionPhase);
            assert_eq!(e.state.minions[ally.index()].owner, SeatId(0));
            assert!(e.state.minions[ally.index()].in_play);
        } else {
            never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, vamp(p, 0));
            assert_eq!(e.state.minions[ally.index()].owner, SeatId(1), "{name}");
        }
    }
}

// ---- The graders' untested readings ------------------------------------------------

/// Enticement: its Methuselah does not hold the Edge (the prey does): not
/// offered; holding it, offered.
/// [advanced rules §1.6.5] A card naming the object it acts on is not
/// played with none (the Edge it burns).
#[test]
fn enticement_is_not_offered_without_the_edge() {
    for edge in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("ser", 0), 3, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.seats[0].has_edge = edge;
        e.state.seats[1].has_edge = !edge;
        hand(&mut e, 0, &["Enticement"]);
        let req = request(&mut e);
        assert_eq!(
            plays(&req).contains(&play_on(0, 0, a, Target::Seat(SeatId(1)))),
            edge
        );
    }
}

/// Gramle: naming a card another Methuselah controls in play (a vampire of
/// the prey's) leaves the action undirected.
/// [rules-qa 17.2] Only the name is used: nothing of theirs is aimed at.
#[test]
fn gramle_naming_anothers_card_in_play_is_undirected() {
    let v = plain("Ventrue", 0);
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Toreador", 0), 4, false);
    e.add_minion(SeatId(2), v, 4, false);
    e.state.seats[0].crypt = vec![plain("Toreador", 1), v].into();
    hand(&mut e, 0, &["Gramle"]);
    pick(&mut e, play(0, 0, a));
    pick(&mut e, Choice::AshVampire(v));
    request(&mut e);
    assert_eq!(
        e.action().expect("the action").directed_seat(&e.state),
        None
    );
}

/// Bloodlust: two copies choosing the same minion give it two optional
/// presses each combat.
/// [D11] Each copy's effect is its own.
#[test]
fn bloodlust_two_copies_give_two_presses() {
    let mut e = game(2);
    let o = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    for _ in 0..2 {
        let c = in_play(&mut e, 0, "Bloodlust", None);
        e.state.cards[c.index()].chosen = vec![o];
    }
    assert_eq!(e.state.combat_presses_of(o, t), 2);
}

/// Fatuus Mastery: two copies on one vampire each pay part of a
/// Chimerstry card's blood cost with its counters.
#[test]
fn fatuus_mastery_two_copies_each_pay() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_discs(&["CHI", "pre"], 0), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 2, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let f = [
        in_play_mode(&mut e, 0, "Fatuus Mastery", Some(a), 1),
        in_play_mode(&mut e, 0, "Fatuus Mastery", Some(a), 1),
    ];
    for c in f {
        e.state.cards[c.index()].counters = 1;
    }
    hand(&mut e, 0, &["Edged Illusion"]);
    pick(&mut e, play_on(0, 0, a, Target::Minion(t)));
    for c in f {
        pick(&mut e, Choice::ResolveRider { card: c.0 });
    }
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 2, "both counters, no blood");
    for c in f {
        assert_eq!(e.state.cards[c.index()].counters, 0);
    }
}

/// Ex Nihilo: two copies on one vampire, two dues each master phase.
/// [ANK 20220503] Each is 1 blood or the vampire.
#[test]
fn ex_nihilo_two_copies_two_master_phase_dues() {
    let mut e = game(2);
    let x = e.add_minion(SeatId(0), plain("Brujah", 0), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 0, "Ex Nihilo", Some(x));
    in_play(&mut e, 0, "Ex Nihilo", Some(x));
    for left in [2, 1] {
        let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
        e.choose(index_of(&req, Choice::BurnBloodToAvoid));
        assert_eq!(blood(&e, x), left);
    }
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 1, "two dues, no third");
}

/// Fantasy World on an ally: in its unlock phase it burns 1 life to burn
/// the card.
/// [rules-qa 73.5] An ally pays life where a vampire pays blood.
#[test]
fn fantasy_world_on_an_ally_is_burned_for_a_life() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let t = e.add_ally(SeatId(1), find_library("City Star Taxi"));
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[t.index()].locked = true;
    let card = in_play(&mut e, 0, "Fantasy World", Some(t));
    let req = request(&mut e);
    e.choose(index_of(&req, Choice::Pass));
    let req = offering_use(&mut e, card);
    assert_eq!(req.seat, SeatId(1));
    let u = uses(&req, card.0)[0];
    e.choose(index_of(&req, u));
    assert_eq!(blood(&e, t), 1, "1 of its 2 life");
    assert!(ash_has(&e, 0, "Fantasy World"));
    assert!(e.state.minions[t.index()].in_play);
}

/// Pass until a use of `card` is offered: that request, unanswered.
fn offering_use(e: &mut Engine, card: CardId) -> zvtes::decision::DecisionRequest {
    for _ in 0..200 {
        let req = request(e);
        if !uses(&req, card.0).is_empty() {
            return req;
        }
        e.choose(fallback(&req));
    }
    panic!("no use of {card:?} offered");
}

/// Chill of Oblivion [NEC] fizzling (its target gone to torpor before
/// resolution): the actor does not unlock in the discard phase.
/// [rules-qa 117.1] A fizzled action has no effect.
#[test]
fn chill_of_oblivion_superior_fizzled_unlocks_nothing() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("NEC", 0), 4, false);
    let o = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Chill of Oblivion"]);
    pick(&mut e, play_on(0, 1, a, Target::Minion(o)));
    request(&mut e);
    e.state.minions[o.index()].in_torpor = true;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Fizzled));
    let req = request(&mut e);
    e.choose(index_of(&req, Choice::Pass));
    loop {
        let req = request(&mut e);
        if e.state.active != SeatId(0) {
            break;
        }
        e.choose(fallback(&req));
    }
    assert!(e.state.minions[a.index()].locked, "still locked");
}

/// Nightmare Curse [aus][chi] on an infernal vampire: its controller may
/// still pay a pool to unlock it in the unlock phase.
/// [LSJ 20050114] "Does not unlock as normal" is redundant with being
/// infernal; the infernal unlock stays.
#[test]
fn nightmare_curse_leaves_an_infernal_vampire_its_paid_unlock() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let t = e.add_minion(SeatId(1), textless(|c| c.capacity == 7, 0), 4, false);
    e.state.minions[t.index()].made_infernal = true;
    e.state.minions[t.index()].locked = true;
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    in_play_mode(&mut e, 0, "Nightmare Curse", Some(t), 1);
    let req = request(&mut e);
    e.choose(index_of(&req, Choice::Pass));
    assert!(offered_before(
        &mut e,
        Choice::UnlockInfernal { minion: t },
        SeatId(1),
        WindowLabel::MinionPhase
    ));
    assert!(!e.state.minions[t.index()].locked);
}

/// Poison the Well of Life [qui] at one's own hunting ground: undirected,
/// the hunting ground burned.
/// [D12] An action at what its own Methuselah controls is undirected.
#[test]
fn poison_the_well_at_ones_own_hunting_ground_is_undirected() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("qui", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let park = in_play(&mut e, 0, "Park Hunting Ground", None);
    hand(&mut e, 0, &["Poison the Well of Life"]);
    pick(&mut e, play_on(0, 0, a, Target::Card(park)));
    request(&mut e);
    assert_eq!(e.action().expect("action").directed_seat(&e.state), None);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[park.index()].in_play);
}

/// Khazar's Diary at 7: another Methuselah's Giovanni uses the Diary's
/// action on their own turn; the wraith is theirs.
/// [D11] "Giovanni can …": every Methuselah's.
#[test]
fn khazars_diary_action_is_used_by_anothers_giovanni() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let g = e.add_minion(SeatId(1), giovanni(0), 4, false);
    let x = plain("Toreador", 0);
    e.state.seats[0].ash_crypt = vec![x].into();
    let d = diary(&mut e, 0, 7);
    let req = request(&mut e);
    e.choose(index_of(&req, Choice::Pass));
    pick_when(&mut e, granted(d.0, g));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let w = represented(&e, x, true).expect("the wraith");
    assert_eq!(e.state.minions[w.index()].owner, SeatId(1));
}

/// Temptation on a vampire in torpor: the steal is still offered and taken
/// (counters 2, blood 2).
/// [D25] A printed ability with no "if ready" works in torpor.
#[test]
fn temptation_steals_a_vampire_in_torpor() {
    let mut e = game(2);
    e.add_minion(SeatId(0), vampire_with("ser", 0), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 2, true);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let card = in_play(&mut e, 0, "Temptation", Some(t));
    e.state.cards[card.index()].counters = 2;
    pick(&mut e, use_on(card, 1, None, Target::None));
    let v = &e.state.minions[t.index()];
    assert_eq!(v.owner, SeatId(0));
    assert!(v.in_torpor, "stays in torpor");
}

/// Shell Break on its own Giovanni: as he burns there is no Giovanni to
/// move to, the card burns with him; on another's vampire with its
/// Giovanni in torpor, the card still moves to him.
#[test]
fn shell_break_on_its_giovanni_burns_and_moves_to_a_torpid_one() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), giovanni(0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Shell Break"]);
    pick(&mut e, play_on(0, 0, g, Target::Minion(g)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let shell = placed_on(&e, "Shell Break", Some(g));
    e.burn(g);
    assert!(!e.state.cards[shell.index()].in_play);
    assert!(ash_has(&e, 0, "Shell Break"));

    let mut e = game(2);
    let g = e.add_minion(SeatId(0), giovanni(0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Shell Break"]);
    pick(&mut e, play_on(0, 0, g, Target::Minion(v)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let shell = placed_on(&e, "Shell Break", Some(v));
    e.state.minions[g.index()].in_torpor = true;
    e.burn(v);
    let c = &e.state.cards[shell.index()];
    assert!(
        c.in_play && c.host == Some(g),
        "moved to the torpid Giovanni"
    );
}

/// Soul Feasting: the wraith chosen gone before resolution: the action
/// fizzles, no hunt.
/// [rules-qa 117.3] The target named at declaration gone: fizzle.
#[test]
fn soul_feasting_fizzles_with_its_wraith_gone() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("nec", 0), 1, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let tye = e.add_ally(SeatId(0), find_library("Tye Cooper"));
    hand(&mut e, 0, &["Soul Feasting"]);
    pick(&mut e, play_on(0, 1, a, Target::Minion(tye)));
    request(&mut e);
    e.burn(tye);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Fizzled));
    assert_eq!(blood(&e, a), 1, "no hunt");
}

/// Precision: its prey (the target seat at declaration) ousted before
/// resolution: the action fizzles; the new prey discards nothing.
/// [rules-qa 117.1] cause 1/: the target no longer legal.
#[test]
fn precision_fizzles_once_its_prey_is_ousted() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), vampire_with("cel", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    hand(&mut e, 2, &["Conceal"]);
    hand(&mut e, 0, &["Precision"]);
    pick(&mut e, play_on(0, 0, a, Target::Seat(SeatId(1))));
    pick(&mut e, Choice::FindCard(find_library("Conceal")));
    request(&mut e);
    e.state.seats[1].pool = 0;
    e.state.seats[1].ousted = true;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Fizzled));
    assert!(e.state.seats[2].hand.contains(&find_library("Conceal")));
}

/// Set's Curse [SER]: a capacity-2 vampire in torpor is a target, burned.
/// [D28] "A vampire" with no "ready": a torpid one too.
#[test]
fn sets_curse_superior_burns_a_small_torpid_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("SER", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let small_card = textless(|c| c.capacity == 2, 0);
    let small = e.add_minion(SeatId(1), small_card, 0, true);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Set's Curse"]);
    pick(&mut e, play_on(0, 2, a, Target::Minion(small)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[small.index()].in_play);
    assert!(e.state.seats[1].ash_crypt.contains(&small_card));
    assert!(curse_ally(&e).is_some());
}

/// Truth of Blood blocked by a minion whose Methuselah holds 1 card: that
/// one card discarded.
/// [D4] "Discards 2 cards": as many as there are.
#[test]
fn truth_of_blood_blocked_discards_what_the_hand_holds() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_discs(&["aus", "qui"], 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 1, &["Form of Mist"]);
    hand(&mut e, 0, &["Truth of Blood"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, block(b));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[1].hand.is_empty());
    assert_eq!(e.state.seats[1].ash.len(), 1);
    assert!(ash_has(&e, 1, "Form of Mist"));
}

/// Apparition [chi]: two copies add, 2 prevented each round: X's 3 leaves
/// 1.
/// [advanced rules §1.15] Numeric bonuses from multiple copies add.
#[test]
fn apparition_two_copies_add_each_round() {
    let (mut e, a, x) = duel(&["chi"], &[]);
    stronger(&mut e, x, 2);
    hand(&mut e, 0, &["Apparition", "Apparition"]);
    bleed_blocked(&mut e, a, x);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::UsePrevent);
    pick(&mut e, Choice::UsePrevent);
    let req = request(&mut e);
    assert!(!req.options.contains(&Choice::UsePrevent), "two, no more");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4 - 2 - 1, "two costs; 3 less 2");
}

/// Cobra Fangs [SER] landing twice (an additional strike): two dues on
/// X, one torpor in its unlock phase, both gone after.
/// [advanced rules §1.15] Each copy triggers and resolves on its own.
#[test]
fn cobra_fangs_superior_two_landing_give_one_torpor() {
    let (mut e, a, x) = duel(&["ser", "ser", "cel"], &[]);
    e.state.minions[a.index()].blood = 8;
    hand(&mut e, 0, &["Cobra Fangs", "Blur", "Cobra Fangs"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let fangs = find_library("Cobra Fangs");
    let dues = |e: &Engine| {
        e.state
            .cards
            .iter()
            .filter(|c| c.card == fangs && c.present() && c.host == Some(x))
            .count()
    };
    assert_eq!(dues(&e), 2);
    assert_eq!(blood(&e, x), 4 - 2);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.minions[x.index()].in_torpor);
    assert_eq!(blood(&e, x), 4 - 2, "one torpor, nothing more");
    assert_eq!(dues(&e), 0);
}

/// Eyes of Blades [AUS][CEL]: an undodgeable strike (X's Fata Amria) has
/// no effect on A either: the card is not put on A.
/// [advanced rules §4.6.4] "No effect" is not a dodge.
#[test]
fn eyes_of_blades_superior_voids_an_undodgeable_strike() {
    let (mut e, a, x) = duel(&["aus", "aus", "cel", "cel"], &["chi"]);
    hand(&mut e, 0, &["Eyes of Blades"]);
    hand(&mut e, 1, &["Fata Amria"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 0, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 1, "Fata Amria"), "not on A");
    assert_eq!(blood(&e, a), 4);
}

/// Fata Amria [chi]: "put this card on the opposing vampire": against an
/// ally, never offered.
/// [advanced rules §4.3.3] A strike card naming a vampire is not played
/// against an ally.
#[test]
fn fata_amria_is_not_played_against_an_ally() {
    let (mut e, a, _) = ally_blocks(&["chi"]);
    hand(&mut e, 0, &["Fata Amria"]);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
}

/// Fata Amria: two on one vampire: one burned instead of unlocking; the
/// vampire then not unlocking as normal, the second is not offered.
/// [LSJ 20010814-3] It cannot be burned when not unlocking as normal.
#[test]
fn fata_amria_two_copies_burn_one_each_unlock_phase() {
    let (mut e, _, x) = duel(&[], &[]);
    let f1 = borne(&mut e, "Fata Amria", x);
    let f2 = borne(&mut e, "Fata Amria", x);
    let req = until(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
    assert_eq!(
        req.options,
        vec![Choice::Pass, Choice::BurnInPlay { card: f1.0 }]
    );
    e.choose(index_of(&req, Choice::BurnInPlay { card: f1.0 }));
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        Choice::BurnInPlay { card: f2.0 },
    );
    assert!(!e.state.cards[f1.index()].in_play);
    assert!(e.state.cards[f2.index()].in_play);
}

/// Forearm Block: its shield set as it is chosen covers X's Quick Jab, a
/// strike card that is a hand strike resolving first (X strength 2, its
/// 1 prevented); played at long range too.
/// [Forearm Block] "(including any currently-resolving hand strike)";
/// rulebook: a defensive strike is usable at any range.
#[test]
fn forearm_block_covers_a_first_strike_hand_strike_card_at_any_range() {
    let (mut e, a, x) = duel(&[], &[]);
    stronger(&mut e, x, 1);
    hand(&mut e, 0, &["Forearm Block"]);
    hand(&mut e, 1, &["Quick Jab"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4, "prevented");

    let (mut e, a, x) = duel(&[], &[]);
    hand(&mut e, 0, &["Fake Out", "Forearm Block"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    assert_eq!(e.combat().expect("combat").range, Range::Long);
    offering(&mut e, play(0, 0, a));
}

/// Forearm Block: X's dodge undoes neither its first strike next round:
/// A presses on, its hand strike lands first, X (0 blood) to torpor
/// before striking.
/// [advanced rules §4.6.1] A dodge protects from the strike, not from
/// what the strike gives its own minion.
#[test]
fn forearm_block_first_strike_survives_a_dodge() {
    let (mut e, a, x) = duel(&["pot"], &[]);
    hand(&mut e, 0, &["Forearm Block", "Relentless Pursuit"]);
    hand(&mut e, 1, &["Dodge"]);
    bleed_blocked(&mut e, a, x);
    e.state.minions[x.index()].blood = 0;
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, x));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[x.index()].in_torpor);
    assert_eq!(blood(&e, a), 4 - 1, "Relentless Pursuit's blood only");
}

/// Gemini's Mirror [OBF]: an undodgeable strike (X's Fata Amria) flips
/// too: over seeds, the card lands on A or has no effect.
/// [advanced rules §4.6.4] "No effect" is not a dodge.
#[test]
fn geminis_mirror_flips_for_an_undodgeable_strike() {
    let mut seen = [false; 2];
    for seed in 0..40 {
        let (mut e, a, x) = duel_seeded(seed, &["obf", "obf"]);
        e.state.minions[x.index()].gained_levels = vec!["chi"];
        hand(&mut e, 0, &["Gemini's Mirror"]);
        hand(&mut e, 1, &["Fata Amria"]);
        bleed_blocked(&mut e, a, x);
        pick_when(&mut e, play(0, 1, a));
        pick_when(&mut e, play(0, 0, x));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let fata = find_library("Fata Amria");
        let on_a = e
            .state
            .cards
            .iter()
            .any(|c| c.card == fata && c.present() && c.host == Some(a));
        assert_eq!(on_a, !ash_has(&e, 1, "Fata Amria"), "seed {seed}");
        seen[usize::from(on_a)] = true;
    }
    assert_eq!(seen, [true, true]);
}

/// The Jones [SER]: a strike card played at the maneuver (X's Gemini's
/// Mirror [obf]) is canceled as played: no maneuver, its cost unpaid, X
/// strikes again.
/// [advanced rules §4.3.1] A strike played at the maneuver is a strike
/// card played.
#[test]
fn the_jones_cancels_a_strike_card_played_at_the_maneuver() {
    let (mut e, a, x) = ages(7, &["ser", "ser"], 4);
    e.state.minions[x.index()].gained_levels = vec!["obf"];
    hand(&mut e, 0, &["Jones, The"]);
    hand(&mut e, 1, &["Gemini's Mirror"]);
    bleed_blocked(&mut e, a, x);
    let req = until(&mut e, SeatId(1), WindowLabel::CombatDetermineRange);
    e.choose(index_of(&req, play(0, 0, x)));
    pick(&mut e, play(0, 1, a));
    let req = request(&mut e);
    assert_eq!(e.combat().expect("combat").range, Range::Close);
    assert!(ash_has(&e, 1, "Gemini's Mirror"));
    assert_eq!(blood(&e, x), 4, "its cost not paid");
    assert_eq!(req.seat, SeatId(1));
}

/// Mayaparisatya [chi]: X's Ghoul Retainer using X's Baseball Bat deals A
/// nothing; X's hand strike still lands.
/// [rules-qa 116.41] A retainer using the minion's weapon: still that
/// weapon's damage.
#[test]
fn mayaparisatya_covers_a_retainer_using_the_opposing_weapon() {
    let (mut e, a, x) = duel(&["chi"], &[]);
    let bat = borne(&mut e, "Baseball Bat", x);
    let ghoul = ghoul_retainer_on(&mut e, x);
    hand(&mut e, 0, &["Mayaparisatya"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    use_when(&mut e, ghoul, 0, Target::Card(bat));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4 - 2 - 1, "its cost; X's hand strike");
}

/// Mayaparisatya [chi]: X's Baseball Bat strike aimed at A's retainer
/// (Target Retainer) still deals it its 2: only A is covered.
#[test]
fn mayaparisatya_leaves_a_retainer_struck_by_a_weapon() {
    let (mut e, a, x) = duel(&["chi"], &[]);
    let bat = borne(&mut e, "Baseball Bat", x);
    let ghoul = ghoul_retainer_on(&mut e, a);
    hand(&mut e, 0, &["Mayaparisatya"]);
    hand(&mut e, 1, &["Target Retainer"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, card_use(bat, 0));
    pick_when(&mut e, play_on(0, 0, x, Target::Card(ghoul)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[ghoul.index()].in_play, "its 2 life taken");
    assert_eq!(blood(&e, a), 4 - 2, "its cost only");
}

/// Revelation of Wrath: "A vampire can play only one … each round": the
/// second copy not offered that round.
#[test]
fn revelation_of_wrath_once_each_round() {
    let (mut e, a, x) = duel(&["ser"], &[]);
    stronger(&mut e, x, 1);
    hand(&mut e, 0, &["Revelation of Wrath", "Revelation of Wrath"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
    assert_eq!(e.state.minions[x.index()].corruption_of(SeatId(0)), 1);
}

/// Strike at the True Flesh: a dodge is unaffected (X's Dodge, A's bat
/// strike has no effect); aggravated damage stays preventable (under
/// Baal's Bloody Talons, X's Armor of Vitality offered).
/// [Strike at the True Flesh] "Non-aggravated damage … cannot be
/// prevented"; rulebook: a dodge is not prevention.
#[test]
fn strike_at_the_true_flesh_spares_dodges_and_aggravated_prevention() {
    let (mut e, a, x) = duel(&["qui"], &[]);
    let bat = borne(&mut e, "Baseball Bat", a);
    hand(&mut e, 0, &["Strike at the True Flesh"]);
    hand(&mut e, 1, &["Dodge"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, card_use(bat, 0));
    pick_when(&mut e, play(0, 0, x));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4, "dodged");

    let (mut e, a, x) = duel(&["qui"], &["for"]);
    e.state.minions[x.index()].blood = 8;
    let bat = borne(&mut e, "Baseball Bat", a);
    hand(
        &mut e,
        0,
        &["Strike at the True Flesh", "Baal's Bloody Talons"],
    );
    hand(&mut e, 1, &["Armor of Vitality"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, card_use(bat, 0));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, a));
    let req = until(&mut e, SeatId(1), WindowLabel::CombatPrevention);
    assert_eq!(pending(&e, 1), (0, 2));
    assert!(req.options.contains(&play(0, 0, x)), "{:?}", req.options);
}

/// Sympathetic Agony: untimed, offered before range is determined and at
/// the maneuver alike.
/// [rules-qa 2.2] A combat card with no timing is played whenever its
/// player holds the impulse in combat.
#[test]
fn sympathetic_agony_is_untimed() {
    let (mut e, a, x) = duel(&["chi", "for"], &[]);
    hand(&mut e, 0, &["Sympathetic Agony"]);
    bleed_blocked(&mut e, a, x);
    let req = until(&mut e, SeatId(0), WindowLabel::CombatBeforeRange);
    assert!(req.options.contains(&play(0, 0, a)));
    e.choose(index_of(&req, Choice::Pass));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatDetermineRange);
    assert!(req.options.contains(&play(0, 0, a)));
}

/// Unholy Penance [PRE] on a minion that has one: played, the combat ends,
/// but not put on (its Methuselah's ash heap); no bleed bonus.
/// [rules-qa 99.2] "Only one": the second is played, not put on.
#[test]
fn unholy_penance_second_is_played_not_put_on() {
    let (mut e, a, x) = duel(&["pre", "pre"], &[]);
    borne(&mut e, "Unholy Penance", x);
    hand(&mut e, 0, &["Unholy Penance"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let up = find_library("Unholy Penance");
    assert_eq!(
        e.state
            .cards
            .iter()
            .filter(|c| c.card == up && c.present())
            .count(),
        1
    );
    assert!(ash_has(&e, 0, "Unholy Penance"));
    assert_eq!(e.state.bleed_against_mod(a, SeatId(1)), 0);
}

/// Forger's Hammer: its +1 follows the weapon to a new bearer, and is of
/// the strike's kind: under Baal's Bloody Talons the Baseball Bat's 2
/// aggravated and the Hammer's 1 aggravated too.
/// [RTR 19970630] [TOM 19960225] Additional damage inherits the base
/// damage's properties.
#[test]
fn forgers_hammer_follows_its_weapon_and_inherits_its_kind() {
    let (mut e, a, x) = duel(&["qui"], &["for"]);
    hand(&mut e, 1, &["Armor of Vitality"]);
    let bat = borne(&mut e, "Baseball Bat", a);
    let h = in_play(&mut e, 0, "Forger's Hammer", Some(a));
    e.state.cards[h.index()].on_card = Some(bat);
    let other = e
        .state
        .minions_of(SeatId(0))
        .find(|&m| m != a)
        .expect("second");
    e.state.cards[bat.index()].host = Some(other);
    assert_eq!(e.state.melee_damage_bonus(other, Some(bat)), 1);
    e.state.cards[bat.index()].host = Some(a);
    hand(&mut e, 0, &["Baal's Bloody Talons"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, card_use(bat, 0));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(1), WindowLabel::CombatPrevention);
    assert_eq!(pending(&e, 1), (0, 3));
}

/// False Resonance [chi]: played by the acting minion only: another
/// vampire of the acting Methuselah's with Chimerstry is never offered it.
/// [D18] An action modifier is played by the acting minion.
#[test]
fn false_resonance_is_played_by_the_acting_minion_only() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_discs(&["chi"], 0), 4, false);
    let o = e.add_minion(SeatId(0), with_discs(&["chi"], 1), 4, false);
    let b = e.add_minion(SeatId(1), vampire_with("aus", 0), 4, false);
    hand(&mut e, 0, &["Fata Morgana", "False Resonance"]);
    hand(&mut e, 1, &["Telepathic Misdirection"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, b));
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::CardAsPlayed);
    assert!(req.options.contains(&play(0, 0, a)));
    assert!(!req.options.contains(&play(0, 0, o)));
}

/// Divine Image: its prevention covers environmental damage in combat:
/// X's hand strike dodged, its Razor Bat's 1 prevented.
/// [rules-qa 30.6] Beast Meld's "prevents 1 damage … at any point of the
/// combat": any damage to the vampire.
#[test]
fn divine_image_prevents_environmental_damage() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_discs(&["ser"], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    borne(&mut e, "Razor Bat", b);
    hand(&mut e, 0, &["Divine Image", "Dodge"]);
    pick(&mut e, bleed(a));
    expect(&mut e, SeatId(0), WindowLabel::ActionAsAnnounced);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::UsePrevent);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4 - 1, "its cost; the Razor Bat's 1 prevented");
}

/// Mirror Image [CHI] played by the blocker: the combat ends, the hunt
/// stays blocked (only the acting vampire's action continues).
#[test]
fn mirror_image_by_the_blocker_only_ends_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), with_discs(&["CHI"], 0), 4, false);
    e.state.minions[b.index()].gained_levels = vec!["qui"];
    hand(&mut e, 1, &["Blood Awakening", "Mirror Image"]);
    pick(&mut e, action(a, ActionKind::Hunt));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play(0, 0, b));
    pick_when(&mut e, play(0, 1, b));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
    assert_eq!(blood(&e, a), 4, "no hunt, no damage");
}

/// Truth of a Thousand Lies [SER] on a vampire already unlocked (Freak
/// Drive first): still offered, a legal no-op.
/// [D14] An unlock of an unlocked vampire is a legal no-op.
#[test]
fn truth_of_a_thousand_lies_superior_on_an_unlocked_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_discs(&["SER"], 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["for"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Freak Drive", "Truth of a Thousand Lies"]);
    pick(&mut e, action(a, ActionKind::Hunt));
    pick_when(&mut e, play(0, 0, a));
    let req = request(&mut e);
    assert!(!e.state.minions[a.index()].locked);
    assert!(req.options.contains(&play(0, 1, a)), "{:?}", req.options);
}

/// Will-o'-the-Wisp [CHI] choosing nobody: X = 1 (never 0), 1 blood paid;
/// no intercept point given (not [chi]'s effect).
/// [LSJ 20030502] X not less than zero: X - 1 >= 0. [digest §4.5] No "As
/// above".
#[test]
fn will_o_the_wisp_superior_with_no_pick_costs_one_and_gives_no_points() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_discs(&["CHI"], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Will-o'-the-Wisp"]);
    pick(&mut e, bleed(a));
    expect(&mut e, SeatId(0), WindowLabel::ActionAsAnnounced);
    pick(&mut e, play(0, 1, a));
    request(&mut e);
    assert_eq!(e.action().expect("bleed").intercept_of(&e.state, b), 0);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4 - 1, "X = 1");
}

/// Ecstasy [SER] by two vampires of the bled Methuselah: a bleed of 1
/// reduced to -1, resolved for less than 0; the acting minion burns 1 for
/// each.
/// [D4] No cap printed; [D26] once per minion.
#[test]
fn ecstasy_superior_by_two_vampires_burns_two() {
    let (mut e, a, x) = reactors(4, 6, &["ser", "ser"]);
    let y = e.state.minions_of(SeatId(1)).nth(1).unwrap();
    e.state.minions[y.index()].gained_levels = vec!["ser", "ser"];
    hand(&mut e, 1, &["Ecstasy", "Ecstasy"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 1, x));
    pick_when(&mut e, play(0, 1, y));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4 - 2);
    assert_eq!(e.state.seats[1].pool, 10);
}

/// Ecstasy [SER], then the bleed redirected (Deflection) to seat 2: the -1
/// stays with the bleed, seat 2 loses nothing, and the acting minion burns
/// 1 as it resolved for 0.
/// [rules-qa 116.52] The reduction stays with the bleed after a redirect.
#[test]
fn ecstasy_superior_stays_with_a_redirected_bleed() {
    let (mut e, a, x) = reactors(4, 6, &["ser", "ser", "dom"]);
    hand(&mut e, 1, &["Ecstasy", "Deflection"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 1, x));
    pick_when(&mut e, play_on(0, 0, x, Target::Seat(SeatId(2))));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e), vec![10, 10, 10]);
    assert_eq!(blood(&e, a), 4 - 1);
}

/// Fillip: a younger locked vampire in torpor is a legal (useless) choice;
/// an ally of the reacting Methuselah's never plays Fillip ("Requires a
/// vampire").
/// [D28] "Locked vampire" with no "ready": a torpid one too; [D14] a legal
/// no-op.
#[test]
fn fillip_chooses_a_torpid_vampire_and_needs_a_vampire() {
    let (mut e, a, x) = reactors(4, 6, &[]);
    let y = e.state.minions_of(SeatId(1)).nth(1).unwrap();
    let ally = e.add_ally(SeatId(1), find_library("Gravebound Drone"));
    e.state.minions[x.index()].locked = true;
    e.state.minions[y.index()].locked = true;
    e.state.minions[y.index()].in_torpor = true;
    hand(&mut e, 1, &["Fillip"]);
    pick(&mut e, bleed(a));
    let req = offering(&mut e, play_on(0, 0, x, Target::Minion(y)));
    assert!(!req
        .options
        .iter()
        .any(|o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == ally)));
}

/// Foul Blood on a hunt stealing blood (Restricted Vitae's): 1 less
/// taken, the younger vampire keeps its blood.
/// [digest §3.1] [RTR 20030519] A hunt taking blood from a target takes
/// the hunt bonus from it.
#[test]
fn foul_blood_takes_one_less_from_a_stealing_hunt() {
    let (mut e, a, x) = reactors(4, 6, &["qui"]);
    e.state.minions[a.index()].blood = 2;
    e.state.minions[a.index()].must_hunt = true;
    let young = e.add_minion(SeatId(1), textless(|c| c.capacity == 3, 0), 2, false);
    let vitae = in_play(&mut e, 2, "Restricted Vitae", None);
    hand(&mut e, 1, &["Foul Blood"]);
    pick_when(&mut e, use_on(vitae, 0, Some(a), Target::Minion(young)));
    pick_when(&mut e, play(0, 0, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, young), 2, "nothing taken");
    assert_eq!(blood(&e, a), 2 - 1, "nothing gained, 1 damage");
}

/// Nest of Eagles: the capacity read as the bleed resolves (a capacity-6
/// bleeder made 5 after the play: 3 off); two reductions by two vampires
/// both apply (never limited).
/// [D13] Conditions read live; [advanced rules §3.7.1] Reductions are
/// unrestricted.
#[test]
fn nest_of_eagles_reads_capacity_live_and_is_never_limited() {
    let bleed3 = |e: &mut Engine, a: MinionId| {
        e.state.minions[a.index()].timed.push((
            zvtes::cards::Lasting::Bleed(2),
            zvtes::state::Expiry::EndOfTurn,
            SeatId(0),
        ));
    };
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), of_cap(6, &["flight"], 0), 4, false);
    e.add_minion(SeatId(0), of_cap(5, GIVEN, 1), 4, false);
    bleed3(&mut e, a);
    let r = e.add_minion(SeatId(1), textless(|c| c.clan == "Banu Haqim", 0), 4, false);
    e.add_minion(SeatId(1), of_cap(5, GIVEN, 3), 4, false);
    hand(&mut e, 1, &["Nest of Eagles"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, r));
    request(&mut e);
    e.state.minions[a.index()].capacity_less = 1;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10, "3 - 3, capacity 5 now");

    let mut e = game(3);
    let a = e.add_minion(SeatId(0), of_cap(6, &["flight"], 0), 4, false);
    e.add_minion(SeatId(0), of_cap(5, GIVEN, 1), 4, false);
    bleed3(&mut e, a);
    let r = e.add_minion(SeatId(1), textless(|c| c.clan == "Banu Haqim", 0), 4, false);
    let r2 = e.add_minion(SeatId(1), textless(|c| c.clan == "Banu Haqim", 1), 4, false);
    hand(&mut e, 1, &["Nest of Eagles", "Nest of Eagles"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, r));
    pick_when(&mut e, play(0, 0, r2));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 9, "3 - 1 - 1");
}

/// Burden the Mind: copies add their tolls (a wake out of turn under two,
/// 2 pool more); a toll its user cannot pay is not offered (three, 3 pool
/// more, at 2 pool).
/// [LSJ 19980225] Copies' costs are cumulative. [D3] A cost unpayable,
/// the play is not offered.
#[test]
fn burden_the_mind_two_copies_add_and_an_unpayable_toll_is_not_offered() {
    for (pool, copies) in [(10, 2), (2, 3)] {
        let (mut e, a, x) = duel(&[], &[]);
        e.state.minions[x.index()].locked = true;
        e.state.seats[1].pool = pool;
        for _ in 0..copies {
            in_play(&mut e, 0, "Burden the Mind", Some(x));
        }
        hand(&mut e, 1, &["On the Qui Vive"]);
        pick(&mut e, bleed(a));
        if pool == 2 {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, x));
            continue;
        }
        pick_when(&mut e, play(0, 0, x));
        request(&mut e);
        assert_eq!(e.state.seats[1].pool, 8, "two tolls");
    }
}

/// The Crocodile Temple: after a successful action at a card its
/// controller controls (Poison the Well of Life at their hunting
/// ground), and after a fizzled action at them (Soul Feasting at their
/// wraith, gone before resolution), it is used: 1 damage each time.
/// [D12] At a card you control is at you. [ANK 20220218] A fizzled action
/// is successful.
#[test]
fn the_crocodile_temple_after_an_action_at_its_card_or_a_fizzle() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_discs(&["qui"], 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let t = in_play(&mut e, 1, "Crocodile Temple, The", None);
    let park = in_play(&mut e, 1, "Park Hunting Ground", None);
    hand(&mut e, 0, &["Poison the Well of Life"]);
    pick(&mut e, play_on(0, 0, a, Target::Card(park)));
    pick_when(&mut e, ability(t.0, 0));
    request(&mut e);
    assert!(!e.state.cards[park.index()].in_play);
    assert_eq!(blood(&e, a), 3);

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_discs(&["nec"], 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let tye = e.add_ally(SeatId(1), find_library("Tye Cooper"));
    let t = in_play(&mut e, 1, "Crocodile Temple, The", None);
    hand(&mut e, 0, &["Soul Feasting"]);
    pick(&mut e, play_on(0, 1, a, Target::Minion(tye)));
    request(&mut e);
    e.burn(tye);
    pick_when(&mut e, ability(t.0, 0));
    request(&mut e);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Fizzled));
    assert_eq!(blood(&e, a), 3);
}

/// The Eternals of Sirius canceled as played (Sudden Reversal) at 4 pool:
/// its cost not paid ("and its cost is not paid"), no gain, not ousted.
#[test]
fn the_eternals_of_sirius_canceled_at_four_pool() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Ministry", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Eternals of Sirius, The"]);
    hand(&mut e, 1, &["Sudden Reversal"]);
    e.state.seats[0].pool = 4;
    to_master(&mut e);
    pick(&mut e, master_at(0, 0, Target::None));
    pick_when(&mut e, master_at(0, 0, Target::None));
    request(&mut e);
    assert!(ash_has(&e, 0, "Eternals of Sirius, The"));
    assert!(!e.state.seats[0].ousted);
    assert_eq!(e.state.seats[0].pool, 4);
}

/// Haqim's Law: Judgment is a trifle (its master phase action given
/// back); its rush is an action requiring an Anarch.
/// [rulebook "Trifle"] [LSJ 20080604] The granted action requires the
/// listed sect.
#[test]
fn haqims_law_judgment_is_a_trifle_and_its_rush_requires_an_anarch() {
    let mut e = game(2);
    let h = e.add_minion(
        SeatId(0),
        crypt_where(
            |c| c.clan == "Banu Haqim" && c.sect == Some(Sect::Independent) && c.capacity == 6,
            0,
        ),
        4,
        false,
    );
    let ind = e.add_minion(
        SeatId(0),
        crypt_where(
            |c| c.clan == "Banu Haqim" && c.sect == Some(Sect::Independent) && c.capacity >= 4,
            1,
        ),
        4,
        false,
    );
    let y = e.add_minion(SeatId(1), crypt_where(|c| c.capacity == 4, 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Haqim's Law: Judgment"]);
    to_master(&mut e);
    let mpa = e.state.seats[0].mpa;
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(h),
            target: Target::Minion(y),
        },
    );
    request(&mut e);
    assert_eq!(e.state.seats[0].mpa, mpa, "a trifle");
    let c = placed_on(&e, "Haqim's Law: Judgment", Some(y));
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Card(c),
            ability: 0,
            actor: Some(ind),
            target: Target::Minion(y),
        },
    );
    request(&mut e);
    assert!(e.action().expect("the rush").requires_anarch(&e.state));
}

/// Hierophant: no Ministry in play to choose, not played; two on one
/// vampire, +2 votes and its blocker burns 2.
/// [LSJ 20100129] [ANK 20191228] Terms that cannot be met: not played.
/// [LSJ 19980225] Copies are cumulative.
#[test]
fn hierophant_needs_a_ministry_and_two_copies_add() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Hierophant"]);
    assert!(!request(&mut e).options.contains(&play(0, 0, caller)));

    let after = |copies: usize| {
        let (mut e, a, x) = duel(&[], &[]);
        for _ in 0..copies {
            in_play(&mut e, 0, "Hierophant", Some(a));
        }
        assert_eq!(e.state.votes_held(a), copies as u8);
        bleed_blocked(&mut e, a, x);
        request(&mut e);
        blood(&e, x)
    };
    assert_eq!(after(2), after(0) - 2);
}

/// Jericho Founding is called with no location in play.
/// [RTR 20010711] Kindred Segregation, Peace Treaty: callable with none
/// to burn.
#[test]
fn jericho_founding_is_called_with_no_location() {
    let mut e = game(2);
    let caller = e.add_minion(
        SeatId(0),
        crypt_where(|c| c.sect == Some(Sect::Anarch) && c.capacity >= 4, 0),
        4,
        false,
    );
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Jericho Founding"]);
    assert!(request(&mut e).options.contains(&play(0, 0, caller)));
}

/// Lesser Boon: two from two players on one minion, each its own: it
/// blocks neither player's minions; its controller below 6 pool burns
/// both.
/// [rules-qa 116.58] Two boons on a vampire, each its player's.
#[test]
fn lesser_boon_two_copies_from_two_players() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let p = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let q = e.add_minion(SeatId(2), plain("Toreador", 0), 4, false);
    let one = in_play(&mut e, 1, "Lesser Boon", Some(a));
    let two = in_play(&mut e, 2, "Lesser Boon", Some(a));
    assert!(e.state.cannot_block(a, p));
    assert!(e.state.cannot_block(a, q));
    e.state.seats[0].pool = 5;
    request(&mut e);
    assert!(!e.state.cards[one.index()].in_play);
    assert!(!e.state.cards[two.index()].in_play);
}

/// Powerbase: Zürich: a Methuselah controlling another copy is not
/// offered the steal (it would contest the card with themselves); without
/// one, offered.
/// [rulebook "Contested Cards"] "You cannot voluntarily contest cards
/// with yourself."
#[test]
fn powerbase_zurich_is_not_stolen_into_a_self_contest() {
    for own in [true, false] {
        let mut e = game(2);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        if own {
            in_play(&mut e, 0, "Powerbase: Zürich", None);
        }
        let z = in_play(&mut e, 1, "Powerbase: Zürich", None);
        e.state.seats[0].has_edge = true;
        if own {
            pick(&mut e, Choice::Pass);
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, ability(z.0, 1));
        } else {
            assert!(to_master(&mut e).options.contains(&ability(z.0, 1)));
        }
    }
}

/// Swiss Cut: an investment card canceled as played (Sudden Reversal) is
/// still played: 1 more blood.
/// [rules-qa 111.2] A canceled card is still played and fires "after it
/// is played".
#[test]
fn swiss_cut_grows_with_a_canceled_investment() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Giovanni", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let sc = in_play(&mut e, 0, "Swiss Cut", None);
    e.state.cards[sc.index()].counters = 2;
    hand(&mut e, 0, &["Secret Horde"]);
    hand(&mut e, 1, &["Sudden Reversal"]);
    to_master(&mut e);
    pick(&mut e, master_at(0, 0, Target::None));
    pick(&mut e, Choice::SetX(1));
    pick_when(&mut e, master_at(0, 0, Target::None));
    request(&mut e);
    assert!(ash_has(&e, 0, "Secret Horde"), "canceled");
    assert_eq!(e.state.cards[sc.index()].counters, 3);
}

/// Urban Jungle, once a game across the table: in play, another
/// Methuselah's copy is never offered in their discard phase.
/// [D27] [rules-qa 40.1] An event is played once a game by all.
#[test]
fn urban_jungle_once_a_game() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Urban Jungle"]);
    hand(&mut e, 1, &["Urban Jungle"]);
    pick(&mut e, Choice::Pass);
    let req = until(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    e.choose(index_of(&req, master_at(0, 0, Target::None)));
    request(&mut e);
    card_in_play(&e, "Urban Jungle");
    never_offered_where(&mut e, SeatId(1), WindowLabel::DiscardPhase, |o| {
        matches!(
            o,
            Choice::Play {
                card: 0,
                actor: None,
                ..
            }
        )
    });
}

/// Warrant: its bearer gone to torpor before the rush resolves: the
/// action is successful with no effect (no combat), its blood unpaid.
/// [rules-qa 17.3] A target no longer qualifying at resolution:
/// successful, no effect.
#[test]
fn warrant_rush_at_a_bearer_no_longer_ready_has_no_effect() {
    let (mut e, a, x) = duel(&[], &[]);
    let w = in_play(&mut e, 0, "Warrant", Some(x));
    e.notes = Some(Vec::new());
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Card(w),
            ability: 0,
            actor: Some(a),
            target: Target::Minion(x),
        },
    );
    request(&mut e);
    e.state.minions[x.index()].in_torpor = true;
    to_minion_phase(&mut e, SeatId(0));
    assert!(noted_combats(&mut e).is_empty(), "no combat");
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Fizzled));
    assert_eq!(blood(&e, a), 4, "its blood unpaid");
}

/// Kpist m/45 dodged: its additional strike is still granted ("dodged or
/// not").
#[test]
fn kpist_m45_additional_strike_after_a_dodge() {
    let (mut e, a, x) = duel(&[], &[]);
    let kpist = borne(&mut e, "Kpist m/45", x);
    hand(&mut e, 0, &["Dodge"]);
    bleed_blocked(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, card_use(kpist, 0));
    let req = offering(&mut e, card_use(kpist, 0));
    assert_eq!(req.seat, SeatId(1));
    assert_eq!(blood(&e, a), 4, "dodged");
    e.choose(index_of(&req, card_use(kpist, 0)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4 - 1, "the additional strike lands");
}

/// Seat 0's A (`levels`) bleeds, Arnulf (seat 1) attempts to block and
/// fails short of A's stealth (Lost in Crowds, the last card of A's hand).
fn arnulf_attempted(levels: &[&'static str], cards: &[&str]) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 1), 4, false);
    e.state.minions[a.index()].gained_levels = levels.to_vec();
    let arnulf = e.add_minion(SeatId(1), find_card("Arnulf Jormungandrsson"), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    hand(&mut e, 0, cards);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(arnulf));
    (e, a, arnulf)
}

/// Arnulf: the bar reads the Discipline the card is played with: after his
/// attempt Dominant Personality ([pot] or [pre]) is played with [pot],
/// never with [pre] alone; a dual action modifier/reaction the acting
/// minion plays (Murmur of the False Will [dom]) is barred; Arnulf burned,
/// the bar stays to the action's end.
/// [D29] [LSJ 20011204-3] Played, a card requires the Discipline used.
/// [D18] A dual card the acting minion plays is an action modifier. [D33]
/// [rules-qa 74.2] The effect outlives its source.
#[test]
fn arnulf_bars_by_the_mode_played_and_outlives_himself() {
    for (levels, offered) in [
        (vec!["obf", "pot"], true),
        (vec!["obf", "pot", "pre"], true),
        (vec!["obf", "pre"], false),
    ] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), of_cap(6, &["pot", "pre", "obf"], 0), 4, false);
        e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 1), 4, false);
        e.state.minions[a.index()].gained_levels = levels.clone();
        e.state.minions[a.index()].sect_override = Some(Sect::Sabbat);
        e.state.minions[a.index()].path_override = Some("Power and the Inner Voice");
        let arnulf = e.add_minion(SeatId(1), find_card("Arnulf Jormungandrsson"), 4, false);
        e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
        hand(&mut e, 0, &["Dominant Personality", "Lost in Crowds"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(arnulf));
        let req = offering(&mut e, play(1, 0, a));
        assert_eq!(
            req.options.contains(&play(0, 0, a)),
            offered,
            "{levels:?}: {:?}",
            req.options
        );
    }

    let (mut e, a, arnulf) = arnulf_attempted(
        &["obf", "dom"],
        &["Murmur of the False Will", "Conditioning", "Lost in Crowds"],
    );
    let req = offering(&mut e, play(2, 0, a));
    e.choose(index_of(&req, play(2, 0, a)));
    e.burn(arnulf);
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { card: 0 | 1, actor: Some(m), .. } if *m == a),
    );
    assert_eq!(e.state.seats[1].pool, 10 - 1, "unblocked, no bonus");
}

/// Arnulf forced to attempt (Special Report): the bar fires as for his own
/// attempt.
/// [Arnulf] "If Arnulf attempts to block": any attempt of his.
#[test]
fn arnulf_bar_fires_on_a_forced_attempt() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 1), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["dom", "obf"];
    let arnulf = e.add_minion(SeatId(1), find_card("Arnulf Jormungandrsson"), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    e.state.minions[arnulf.index()].locked = true;
    hand(&mut e, 0, &["Conditioning", "Lost in Crowds"]);
    hand(&mut e, 1, &["Special Report"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, master_at(0, 0, Target::Minion(arnulf)));
    let req = offering(&mut e, play(1, 0, a));
    assert!(!req.options.contains(&play(0, 0, a)), "{:?}", req.options);
}

/// Durga Syn's steal: with no opposing equipment, not offered; a vehicle
/// she cannot hold (she has one), struck, stays where it is; offered again
/// in the next round, as often as paid.
/// [RTR 19980928] Blessing of Durga Syn: "Cannot be used if there is no
/// equipment to steal." [rules-qa 70.11] Not if the striker cannot have
/// it. [D7] An ability with no limit is as often as paid.
#[test]
fn durga_syn_steals_only_what_she_can_hold_as_often_as_paid() {
    let mut e = game(2);
    let durga = e.add_minion(SeatId(0), find_card("Durga Syn"), 6, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let x = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    bleed_blocked(&mut e, durga, x);
    own_never_until(&mut e, durga, 0, SeatId(0), WindowLabel::MinionPhase);

    let mut e = game(2);
    let durga = e.add_minion(SeatId(0), find_card("Durga Syn"), 8, false);
    e.state.minions[durga.index()].gained_levels = vec!["pot"];
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let x = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    in_play(&mut e, 0, "Sport Bike", Some(durga));
    let cruiser = in_play(&mut e, 1, "Stolen Police Cruiser", Some(x));
    let gun = in_play(&mut e, 1, ".44 Magnum", Some(x));
    let vest = in_play(&mut e, 1, "Kevlar Vest", Some(x));
    hand(&mut e, 0, &["Relentless Pursuit"]);
    let steal = |c: CardId| Choice::UseAbility {
        source: AbilitySource::Vampire(durga),
        ability: 0,
        actor: None,
        target: Target::Card(c),
    };
    bleed_blocked(&mut e, durga, x);
    pick_when(&mut e, steal(cruiser));
    pick_when(&mut e, play(0, 0, durga));
    request(&mut e);
    assert_eq!(e.state.cards[cruiser.index()].host, Some(x), "stays");
    pick_when(&mut e, steal(vest));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[vest.index()].host, Some(durga), "round 2");
    assert_eq!(e.state.cards[cruiser.index()].host, Some(x));
    assert_eq!(e.state.cards[gun.index()].host, Some(x));
}

/// Gwen Brand's founder counters: kept in torpor (still founded); she
/// burned and brought back (Possession [NEC]), a new minion with none.
/// [D25] Her text works in torpor. [LSJ 20040726] Nothing carries over
/// from the ash heap.
#[test]
fn gwen_brand_keeps_founder_counters_through_torpor_not_leaving_play() {
    let mut e = game(2);
    let card = find_card("Gwen Brand");
    let gwen = e.add_minion(SeatId(0), card, 3, false);
    let a = e.add_minion(SeatId(0), vampire_with("NEC", 0), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    e.state.minions[gwen.index()].founder_counters.0 = 4;
    e.state.minions[gwen.index()].in_torpor = true;
    assert_eq!(e.state.capacity_of(gwen), Some(4), "founded in torpor");
    e.burn(gwen);
    hand(&mut e, 0, &["Possession"]);
    pick(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let back = minion_of(&e, 0, card);
    assert_ne!(back, gwen);
    assert_eq!(e.state.minions[back.index()].founder_counters.0, 0);
    assert_eq!(e.state.capacity_of(back), Some(3));
}

/// Lizette's +1 bleed is not the limited bonus: Conditioning's limited +2
/// still played after it (1 + 1 + 2).
/// [rules-qa 42.7] A vampire's "+1 bleed" for blood is not the rulebook's
/// limited bleed bonus.
#[test]
fn lizette_bleed_is_not_limited() {
    let (mut e, v, _) = lord("Lizette", 4);
    e.state.minions[v.index()].gained_levels = vec!["dom"];
    hand(&mut e, 0, &["Conditioning"]);
    pick(&mut e, bleed(v));
    pick_when(&mut e, vamp(v, 1));
    pick_when(&mut e, play(0, 0, v));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 4);
}

/// Luna Giovanni in a polling: in torpor, still offered; with no political
/// action card in any ash heap, not offered.
/// [rules-qa 15.2] [rules-qa 58.8] A printed ability with no "ready" works
/// in torpor. [D14] A card naming an object there is none of: not offered.
#[test]
fn luna_giovanni_in_torpor_and_with_no_political_card() {
    for (torpor, heap) in [(true, true), (false, false)] {
        let mut e = game(2);
        let caller = e.add_minion(
            SeatId(0),
            textless(|c| c.sect == Some(Sect::Camarilla), 0),
            4,
            false,
        );
        let luna = e.add_minion(
            SeatId(0),
            zvtes::state::find_card("Luna Giovanni"),
            4,
            torpor,
        );
        e.add_minion(
            SeatId(1),
            textless(|c| (4..=8).contains(&c.capacity), 0),
            4,
            false,
        );
        if heap {
            e.state.seats[1]
                .ash
                .push(find_library("Autarkis Persecution"));
        }
        hand(&mut e, 0, &["Kine Resources Contested"]);
        pick(&mut e, play(0, 0, caller));
        to_polling(&mut e);
        let req = request(&mut e);
        assert_eq!(
            req.options.contains(&vamp(luna, 0)),
            heap,
            "torpor {torpor}"
        );
    }
}

/// Monty Coven under Persona Non Grata ("cannot gain a non-Anarch
/// title"): the bishop offered, a legal no-op; he stays untitled.
/// [D14] A legal no-op stays offered.
#[test]
fn monty_coven_under_persona_non_grata_gains_no_title() {
    use zvtes::crypt::Title;
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), zvtes::state::find_card("Monty Coven"), 4, false);
    e.add_minion(
        SeatId(0),
        textless(|c| (4..=8).contains(&c.capacity), 0),
        4,
        false,
    );
    in_play(&mut e, 1, "Persona Non Grata", Some(m));
    let v = e.add_minion(SeatId(1), titled(|t| t == Title::Primogen, 0), 0, false);
    e.add_minion(
        SeatId(1),
        textless(|c| (4..=8).contains(&c.capacity), 1),
        4,
        false,
    );
    stronger(&mut e, m, 1);
    e.state.minions[m.index()].timed.push((
        zvtes::cards::Lasting::AggravatedHands,
        zvtes::state::Expiry::EndOfTurn,
        SeatId(0),
    ));
    pick(&mut e, bleed(m));
    pick_when(&mut e, block(v));
    pick_when(&mut e, vamp(m, 0));
    request(&mut e);
    assert!(!e.state.minions[v.index()].in_play, "burned");
    assert_eq!(e.state.title_of(m), Title::None);
}

/// Pentweret: an imbued ally costs its starting life (3: stolen; 4: no
/// target); Set's Curse's animal ally costs 0 (stolen); his strike again in
/// a second round, after a dodge.
/// [ANK 20220528] The cost of an imbued is their starting life. [LSJ
/// 20080804] Set's Curse in play is an ally with a cost of zero.
#[test]
fn pentweret_reads_an_imbueds_life_and_a_token_allys_zero() {
    for (who, offered) in [
        ("Béatrice \"Oracle171\" Tremblay", true),
        ("Marion \"Teacher193\" Perks", false),
        ("Set's Curse", true),
    ] {
        let mut e = game(2);
        let p = e.add_minion(SeatId(0), zvtes::state::find_card("Pentweret"), 6, false);
        e.add_minion(
            SeatId(0),
            textless(|c| (4..=8).contains(&c.capacity), 0),
            4,
            false,
        );
        e.add_minion(
            SeatId(1),
            textless(|c| (4..=8).contains(&c.capacity), 1),
            4,
            false,
        );
        let x = if who == "Set's Curse" {
            curse_ally_of(&mut e, 1)
        } else {
            let card = zvtes::state::find_card(who);
            e.add_minion(SeatId(1), card, CRYPT[card as usize].capacity, false)
        };
        pick(&mut e, bleed(p));
        pick_when(&mut e, block(x));
        if offered {
            pick_when(&mut e, vamp(p, 0));
            until(&mut e, SeatId(1), WindowLabel::MinionPhase);
            assert_eq!(e.state.minions[x.index()].owner, SeatId(0), "{who}");
        } else {
            never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, vamp(p, 0));
            assert_eq!(e.state.minions[x.index()].owner, SeatId(1), "{who}");
        }
    }

    let mut e = game(2);
    let p = e.add_minion(SeatId(0), zvtes::state::find_card("Pentweret"), 6, false);
    e.state.minions[p.index()].gained_levels = vec!["pot"];
    e.add_minion(
        SeatId(0),
        textless(|c| (4..=8).contains(&c.capacity), 0),
        4,
        false,
    );
    let x = e.add_minion(SeatId(1), textless(|c| c.capacity == 3, 0), 3, false);
    e.add_minion(
        SeatId(1),
        textless(|c| (4..=8).contains(&c.capacity), 1),
        4,
        false,
    );
    hand(&mut e, 0, &["Relentless Pursuit"]);
    hand(&mut e, 1, &["Dodge"]);
    pick(&mut e, bleed(p));
    pick_when(&mut e, block(x));
    pick_when(&mut e, vamp(p, 0));
    pick_when(&mut e, play(0, 0, x));
    pick_when(&mut e, play(0, 0, p));
    pick_when(&mut e, vamp(p, 0));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[x.index()].owner, SeatId(0), "round 2");
}

/// Shemti: a directed action at his Methuselah that fizzles (Soul Feasting
/// at their wraith, gone before resolution) is successful: its minion
/// takes his damage.
/// [digest §3.2] A fizzled action is successful.
#[test]
fn shemti_damages_after_a_fizzled_directed_action() {
    let (mut e, _, _) = lord("Shemti", 4);
    let tye = e.add_ally(SeatId(0), find_library("Tye Cooper"));
    let x = e.add_minion(SeatId(1), with_discs(&["nec"], 0), 4, false);
    hand(&mut e, 1, &["Soul Feasting"]);
    to_minion_phase(&mut e, SeatId(1));
    pick(&mut e, play_on(0, 1, x, Target::Minion(tye)));
    request(&mut e);
    e.burn(tye);
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Fizzled));
    assert_eq!(blood(&e, x), 4 - 1);
}

/// Sundervere contested: burned (yielded), his text does nothing: no pool
/// burned.
/// [docs/engine.md §6.12] A contested card's text is suspended.
#[test]
fn sundervere_contested_burns_no_pool() {
    let (mut e, v, _) = lord("Sundervere, The Devil Brahmin", 4);
    e.state.minions[v.index()].contested = true;
    e.burn(v);
    assert!(!e.state.minions[v.index()].in_play);
    assert_eq!(e.state.seats[0].pool, 10);
}

/// Forger's Hammer: never offered for a gun's equip (.44 Magnum): "a
/// melee weapon" only.
#[test]
fn forgers_hammer_is_not_played_on_a_gun() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_discs(&["pot"], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &[".44 Magnum", "Forger's Hammer"]);
    pick(&mut e, play(0, 0, a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
    card_on(&e, ".44 Magnum", a);
    assert!(e.state.seats[0]
        .hand
        .contains(&find_library("Forger's Hammer")));
}
