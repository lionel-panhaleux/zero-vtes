//! Round 28 (`ADDED_BATCHES`): Heirs to the Blood (owner, 2026-11-09).
//! Windows with a single option are elided, so the scripts only stop where
//! a real decision exists.

use zvtes::cards::{AbilitySource, Stat, Target};
use zvtes::crypt::{Sect, Title, CRYPT};
use zvtes::decision::{ActionKind, Choice, WindowLabel, Zone};
use zvtes::game::Engine;
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::{find_library, CardId, CardInPlay, PrintedCtx, Release};

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

/// The `nth` textless untitled vampire of `clan`, any capacity.
fn of_clan(clan: &str, nth: usize) -> u16 {
    textless(|c| c.clan == clan, nth)
}

/// The `nth` Daughter of Cacophony: every one is defined since round 28;
/// these print a block toll or an intercept penalty, inert where they
/// serve.
fn daughter(nth: usize) -> u16 {
    find_card(["Janet Langer", "Harlan Graves", "Evil Jensen"][nth])
}

/// The crypt card `name` (not advanced).
fn find_card(name: &str) -> u16 {
    CRYPT
        .iter()
        .position(|c| c.name == name && !c.adv)
        .unwrap_or_else(|| panic!("no {name}")) as u16
}

/// A card already in play for `seat`, on `host` if given: its id.
fn in_play(e: &mut Engine, seat: u8, name: &str, host: Option<MinionId>) -> CardId {
    e.state
        .cards
        .push(CardInPlay::new(find_library(name), SeatId(seat), host, 0));
    CardId(e.state.cards.len() as u32 - 1)
}

/// The master card `card` of the hand played on `target`.
fn master_on(card: u8, target: Target) -> Choice {
    Choice::Play {
        card,
        mode: 0,
        actor: None,
        target,
    }
}

/// The card in play named `name`, present.
fn card_named(e: &Engine, name: &str) -> CardId {
    let c = find_library(name);
    CardId(
        e.state
            .cards
            .iter()
            .position(|k| k.card == c && k.present())
            .unwrap_or_else(|| panic!("{name} not in play")) as u32,
    )
}

/// Card `card`'s ability `ability`, used by the Methuselah on `target`.
fn use_on(card: CardId, ability: u8, target: Target) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Card(card),
        ability,
        actor: None,
        target,
    }
}

/// Card `card`'s ability `ability` used by `actor`.
fn used_by(card: CardId, ability: u8, actor: MinionId) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Card(card),
        ability,
        actor: Some(actor),
        target: Target::None,
    }
}

// ---- Agent of Power ---------------------------------------------------------

/// Agent of Power: "Put this card on a vampire you control and choose a
/// Discipline. This vampire gains 1 level of that Discipline. Burn this card
/// during your discard phase." The card itself in play, +1 capacity (a
/// master: Discipline), gone in the same turn's discard phase.
#[test]
fn agent_of_power_lends_a_chosen_discipline_until_the_discard_phase() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let cap = e.state.capacity_of(a).expect("a vampire");
    hand(&mut e, 0, &["Agent of Power"]);
    e.at_master_phase();
    let req = request(&mut e);
    assert!(
        !req.options.contains(&master_on(0, Target::Minion(z))),
        "a vampire you control"
    );
    pick(&mut e, master_on(0, Target::Minion(a)));
    pick_when(&mut e, Choice::ChooseDiscipline("dom"));
    request(&mut e);
    assert_eq!(e.state.discipline_level(a, "dom"), 1);
    assert_eq!(e.state.capacity_of(a), Some(cap + 1));
    let c = card_named(&e, "Agent of Power");
    assert!(!e.state.cards[c.index()].effect_only);
    assert!(!e.state.seats[0]
        .ash
        .contains(&find_library("Agent of Power")));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(
        !e.state.cards[c.index()].present(),
        "burned in the discard phase"
    );
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Agent of Power")));
    assert_eq!(e.state.discipline_level(a, "dom"), 0);
}

/// [RBK contested-cards] "You still gain the trifle master phase action,
/// even if this card contests when played." Another Methuselah's copy in
/// play: the new one contests, gives no level, and its trifle action is
/// still gained.
#[test]
fn agent_of_power_contested_still_gives_its_trifle() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let theirs = in_play(&mut e, 1, "Agent of Power", Some(z));
    e.state.cards[theirs.index()].discipline = Some("aus");
    hand(&mut e, 0, &["Agent of Power"]);
    e.at_master_phase();
    let before = e.state.seats[0].mpa;
    pick(&mut e, master_on(0, Target::Minion(a)));
    pick_when(&mut e, Choice::ChooseDiscipline("dom"));
    request(&mut e);
    let mine = (0..e.state.cards.len())
        .map(|i| CardId(i as u32))
        .find(|&c| e.state.cards[c.index()].host == Some(a))
        .expect("put on her");
    assert!(e.state.cards[mine.index()].contested, "unique: contested");
    assert_eq!(e.state.discipline_level(a, "dom"), 0);
    assert_eq!(e.state.seats[0].mpa, before, "the trifle's action gained");
}

/// [LSJ 20071001-1] [LSJ 20100422] "Is unique and provides a discipline
/// even when it was not played in the normal fashion": put on Childe of the
/// Revolution's new vampire by its search, its Discipline chosen then
/// ([LSJ 20071001-1] Bima: "its card text is applied as normal (choose a
/// discipline and burn it in your discard phase)").
#[test]
fn childe_of_the_revolution_can_take_agent_of_power() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 5, false);
    e.state.minions[b.index()].granted_title = Some(Title::Baron("Paris"));
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Childe of the Revolution", "Agent of Power"]);
    let before = e.state.minions.len();
    pick(&mut e, play(0, 0, b));
    pick_when(
        &mut e,
        Choice::TakeFrom {
            zone: Zone::Hand,
            card: find_library("Agent of Power"),
        },
    );
    let childe = MinionId(before as u32);
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::ResolutionChoice);
    assert!(req.options.contains(&Choice::ChooseDiscipline("ani")));
    pick(&mut e, Choice::ChooseDiscipline("ani"));
    assert_eq!(e.state.discipline_level(childe, "ani"), 1);
    let c = card_named(&e, "Agent of Power");
    assert_eq!(e.state.cards[c.index()].host, Some(childe));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(
        !e.state.cards[c.index()].present(),
        "burned in the discard phase"
    );
    assert_eq!(e.state.discipline_level(childe, "ani"), 0);
}

// ---- Benefit Performance ----------------------------------------------------

/// Benefit Performance: "Successful referendum means you gain 1 pool for
/// each ready Daughter of Cacophony you control, and each non-Daughter of
/// Cacophony vampire casting votes or ballots in favor gains 1 blood." Two
/// ready Daughters (a torpid one uncounted); a non-Daughter voting for
/// gains, one voting against and a Daughter voting for do not.
#[test]
fn benefit_performance_pays_per_daughter_and_feeds_supporters() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), daughter(0), 2, false);
    let d2 = e.add_minion(SeatId(0), daughter(1), 2, false);
    e.state.minions[d2.index()].granted_title = Some(Title::Votes(1));
    let t = e.add_minion(SeatId(0), daughter(2), 2, true);
    let pro = e.add_minion(SeatId(1), plain("Ventrue", 0), 2, false);
    e.state.minions[pro.index()].granted_title = Some(Title::Primogen);
    let con = e.add_minion(SeatId(1), plain("Ventrue", 1), 2, false);
    e.state.minions[con.index()].granted_title = Some(Title::Primogen);
    hand(&mut e, 0, &["Benefit Performance"]);
    pick(&mut e, play(0, 0, d));
    pick_when(&mut e, votes(d2, true));
    pick_when(&mut e, votes(pro, true));
    pick_when(&mut e, votes(con, false));
    close_polling(&mut e);
    request(&mut e);
    assert_eq!(e.state.seats[0].pool, 12, "two ready Daughters");
    assert_eq!(blood(&e, pro), 3);
    assert_eq!((blood(&e, con), blood(&e, d2), blood(&e, t)), (2, 2, 2));
}

// ---- Blessed Resilience -----------------------------------------------------

/// The textless vampire with superior Fortitude and Necromancy.
fn necromancer() -> u16 {
    textless(
        |c| c.disciplines.contains(&"FOR") && c.disciplines.contains(&"NEC"),
        0,
    )
}

/// Blessed Resilience: "Move a vampire in your ash heap with superior
/// Fortitude and superior Necromancy who was burned from play to your
/// ready region (with no blood)." Burned from play: back, ready, empty.
#[test]
fn blessed_resilience_returns_a_burned_necromancer() {
    let mut e = game(2);
    let card = necromancer();
    let n = e.add_minion(SeatId(0), card, 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.burn(n);
    assert!(e.state.seats[0].ash_crypt.contains(&card));
    hand(&mut e, 0, &["Blessed Resilience"]);
    e.at_master_phase();
    // The one vampire named: elided.
    pick(&mut e, master_play(0));
    request(&mut e);
    assert!(!e.state.seats[0].ash_crypt.contains(&card));
    let back = e
        .state
        .minions_of(SeatId(0))
        .find(|&m| m != n && e.state.minions[m.index()].crypt_card() == Some(card))
        .expect("in play again");
    let v = &e.state.minions[back.index()];
    assert!(v.ready() && v.present());
    assert_eq!(v.blood, 0);
}

/// [ANK 20210813] "Only a controlled vampire can be burned from play.
/// Uncontrolled and contested vampires do not qualify." A contested copy
/// burned and a crypt card burned from the region: no Blessed Resilience to
/// play.
#[test]
fn blessed_resilience_ignores_vampires_never_in_play() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let card = necromancer();
    let c = e.add_minion(SeatId(0), card, 3, false);
    e.state.minions[c.index()].contested = true;
    e.burn(c);
    assert!(e.state.seats[0].ash_crypt.contains(&card));
    // Never in play, its text cannot apply.
    let never = find_card("Hecate");
    e.state.seats[0].burn_vampire(never, zvtes::copies::CopyId::NONE, false);
    hand(&mut e, 0, &["Blessed Resilience"]);
    e.at_master_phase();
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, master_play(0));
}

// ---- Blessing of the Beast --------------------------------------------------

/// Blessing of the Beast: "Put this card on an Ahrimane you control with
/// any cards requiring Spiritus [spi] from your hand (before replacing),
/// face down and out of play … This Ahrimane can play these cards as if
/// from your hand. Burn this card if it has no cards on it." [PIB
/// 20150428] [LSJ 20100206-2]: the number announced as played, from the
/// hand before the replacement (the library's copy drawn after is not
/// counted); [LSJ 20081126]: played from it, not replaced; the last one
/// played, it burns.
#[test]
fn blessing_of_the_beast_holds_spiritus_cards() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Ahrimane", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let nose = find_library("Nose of the Hound");
    hand(
        &mut e,
        0,
        &[
            "Blessing of the Beast",
            "Nose of the Hound",
            "Nose of the Hound",
        ],
    );
    fill_hand(&mut e, 0);
    e.state.seats[0].library.push(nose);
    e.at_master_phase();
    pick(&mut e, master_on(0, Target::Minion(a)));
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    assert_eq!(req.options.last(), Some(&set_x(2)), "the hand's two");
    // Each pick a single card: elided.
    pick(&mut e, set_x(2));
    request(&mut e);
    let c = card_named(&e, "Blessing of the Beast");
    assert_eq!(e.state.cards[c.index()].host, Some(a));
    assert_eq!(e.state.cards[c.index()].stored.len(), 2);
    assert!(!e.state.cards[c.index()].effect_only);
    let hand = |e: &Engine| e.state.seats[0].hand.iter().filter(|&&h| h == nose).count();
    assert_eq!(hand(&e), 1, "the replacement drawn after, kept");
    assert!(e.state.seats[0].library.is_empty());
    // Played from it by the Ahrimane: not replaced (a full hand: no
    // draw-up either).
    e.state.minions[z.index()].locked = true;
    fill_hand(&mut e, 0);
    e.state.seats[0].library.push(find_library("Soak"));
    let hand_len = e.state.seats[0].hand.len();
    let req = request(&mut e);
    let stored = req
        .options
        .iter()
        .copied()
        .find(|o| {
            matches!(o, Choice::Play { card, actor: Some(m), target: Target::Minion(t), .. }
                if *card as usize >= hand_len && *m == a && *t == z)
        })
        .expect("as if from hand");
    pick(&mut e, stored);
    request(&mut e);
    assert_eq!(e.state.cards[c.index()].stored.len(), 1);
    assert_eq!(e.state.seats[0].hand.len(), hand_len, "not replaced");
    assert_eq!(e.state.seats[0].library.len(), 1);
}

// ---- Blooding by the Code ---------------------------------------------------

/// [LSJ 20051012] "Can be played on a vampire with superior Valeren": the
/// level capped at superior, +1 capacity and +1 strength all the same; "a
/// vampire can have only one".
#[test]
fn blooding_by_the_code_strengthens_even_a_superior_valeren() {
    let mut e = game(2);
    let s = e.add_minion(
        SeatId(0),
        textless(
            |c| c.clan == "Salubri antitribu" && c.disciplines.contains(&"VAL"),
            0,
        ),
        4,
        false,
    );
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let cap = e.state.capacity_of(s).expect("a vampire");
    let strength = |e: &Engine| e.state.bearer_mod(s, Stat::Strength, ActionKind::Bleed);
    assert_eq!(strength(&e), 0);
    let _ = z;
    hand(&mut e, 0, &["Blooding by the Code", "Blooding by the Code"]);
    e.at_master_phase();
    request(&mut e);
    e.state.seats[0].mpa = 2;
    pick(&mut e, master_on(0, Target::Minion(s)));
    request(&mut e);
    assert_eq!(e.state.discipline_level(s, "val"), 2);
    assert_eq!(e.state.capacity_of(s), Some(cap + 1));
    assert_eq!(strength(&e), 1);
    let c = card_named(&e, "Blooding by the Code");
    assert!(!e.state.cards[c.index()].effect_only);
    let req = request(&mut e);
    assert!(
        !req.options.contains(&master_on(0, Target::Minion(s))),
        "only one a vampire"
    );
}

// ---- Capitalist -------------------------------------------------------------

/// [ANK 20190113] "Cards and effects that can be played 'after resolution'
/// (and only them) can be played after the gain blood effect." Capitalist:
/// "Once per turn, when this vampire successfully bleeds his or her prey,
/// he or she gains 1 blood": gained as the bleed succeeds, the
/// after-resolution window after it; a second bleed that turn: nothing.
#[test]
fn capitalist_feeds_a_sabbat_bleeder_once_a_turn() {
    let mut e = game(2);
    let v = e.add_minion(
        SeatId(0),
        textless(
            |c| {
                let has = |d: &str| c.disciplines.iter().any(|x| x.eq_ignore_ascii_case(d));
                c.sect == Some(zvtes::crypt::Sect::Sabbat) && has("dom") && has("for")
            },
            0,
        ),
        2,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(
        &mut e,
        0,
        &["Capitalist", "Freak Drive", "Govern the Unaligned"],
    );
    e.at_master_phase();
    pick(&mut e, master_on(0, Target::Minion(v)));
    request(&mut e);
    let c = card_named(&e, "Capitalist");
    assert!(!e.state.cards[c.index()].effect_only);
    pick_when(&mut e, bleed(v));
    let req = until(&mut e, SeatId(0), WindowLabel::ActionAfterResolution);
    assert_eq!(blood(&e, v), 3, "gained as the bleed succeeded");
    assert!(plays(&req)
        .iter()
        .any(|p| matches!(p, Choice::Play { card: 0, .. })));
    pick(&mut e, Choice::Pass);
    // Unlocked again: a second bleed this turn, by a card.
    e.state.minions[v.index()].locked = false;
    let govern = e.state.seats[0]
        .hand
        .iter()
        .position(|&c| c == find_library("Govern the Unaligned"))
        .expect("in hand") as u8;
    pick_when(&mut e, play(govern, 0, v));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 1 - 3, "both bled");
    assert_eq!(blood(&e, v), 3, "once per turn");
}

// ---- Cavalier ---------------------------------------------------------------

/// [ANK 20190113] after resolution; [ANK 20180719-4] "once each turn"
/// means once during each player's turn; [ANK 20210309-3] the vampire's
/// controller chooses. Cavalier: "Once each turn, this vampire can burn 1
/// blood to unlock after performing a successful action costing blood." A
/// bleed costs nothing: not offered; leaving torpor (2 blood): offered,
/// taken; again that turn: not offered.
#[test]
fn cavalier_unlocks_after_a_blood_costing_action() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), plain("Brujah", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Cavalier"]);
    e.at_master_phase();
    pick(&mut e, master_on(0, Target::Minion(v)));
    request(&mut e);
    let c = card_named(&e, "Cavalier");
    assert!(!e.state.cards[c.index()].effect_only);
    let unlock = ability(c.0, 0);
    pick_when(&mut e, bleed(v));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, unlock);
    e.state.minions[v.index()].locked = false;
    e.state.minions[v.index()].in_torpor = true;
    pick(&mut e, action(v, ActionKind::LeaveTorpor));
    pick_when(&mut e, unlock);
    request(&mut e);
    assert!(!e.state.minions[v.index()].locked);
    assert_eq!(blood(&e, v), 5 - 2 - 1);
    e.state.minions[v.index()].in_torpor = true;
    pick(&mut e, action(v, ActionKind::LeaveTorpor));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, unlock);
}

/// [LSJ 20100315] "Applies even if the vampire it's on is not the one
/// paying the cost (eg. rescuing from torpor)." [PIB 20150725] [LSJ
/// 20100429] the cost as modified: the rescue paid in part by the rescued
/// vampire, and a surcharge making a bleed cost blood (Kuta's "directed
/// actions cost this vampire an additional blood").
#[test]
fn cavalier_reads_the_cost_paid_whoever_paid_it() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let t = e.add_minion(SeatId(0), plain("Brujah", 1), 4, true);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let c = in_play(&mut e, 0, "Cavalier", Some(v));
    let unlock = ability(c.0, 0);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: v,
            action: ActionKind::Rescue,
            target: Some(t),
        },
    );
    pick(&mut e, Choice::PayRescue { from_acting: 0 });
    pick_when(&mut e, unlock);
    request(&mut e);
    assert!(!e.state.minions[v.index()].locked);
    assert_eq!(blood(&e, v), 3, "only the unlock's blood");
    // Another turn: Kuta's surcharge on a bleed.
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let c = in_play(&mut e, 0, "Cavalier", Some(v));
    let kuta = in_play(&mut e, 1, "Kuta", Some(v));
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::HostDirectedToll(1),
        source: kuta,
    });
    pick(&mut e, bleed(v));
    pick_when(&mut e, ability(c.0, 0));
    request(&mut e);
    assert!(!e.state.minions[v.index()].locked);
    assert_eq!(blood(&e, v), 2);
}

/// Cavalier, "after performing a successful action costing blood": a blood
/// cost the Ravnos Carnival's counters pay wholly is no blood cost (owner,
/// 2026-11-10); one counter and one blood, it is. Propaganda (2 blood).
#[test]
fn cavalier_reads_a_cost_paid_by_carnival_counters_as_no_blood() {
    for counters in [2u8, 1] {
        let mut e = game(2);
        let r = e.add_minion(SeatId(0), plain("Ravnos", 0), 4, false);
        e.state.minions[r.index()].gained_levels = vec!["pre"];
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let c = in_play(&mut e, 0, "Cavalier", Some(r));
        let carnival = in_play(&mut e, 0, "Ravnos Carnival", None);
        e.state.cards[carnival.index()].counters = 2;
        e.state.layers.push(zvtes::cards::Layer {
            scope: zvtes::cards::Scope::InPlay,
            modifier: zvtes::cards::Modifier::CostSource(zvtes::cards::CostSource::Carnival(
                "Ravnos",
            )),
            source: carnival,
        });
        hand(&mut e, 0, &["Propaganda"]);
        let req = request(&mut e);
        let propaganda = *req
            .options
            .iter()
            .find(|o| matches!(o, Choice::Play { card: 0, mode: 0, actor: Some(a), .. } if *a == r))
            .expect("Propaganda offered");
        e.choose(index_of(&req, propaganda));
        pick(&mut e, Choice::ResolveRider { card: carnival.0 });
        pick(&mut e, Choice::SetX(counters));
        if counters == 2 {
            never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, ability(c.0, 0));
            assert_eq!(blood(&e, r), 4, "the counters paid it all");
            assert_eq!(e.state.seats[1].pool, 10 - 2, "the bleed succeeded");
        } else {
            pick_when(&mut e, ability(c.0, 0));
            request(&mut e);
            assert!(!e.state.minions[r.index()].locked);
            assert_eq!(
                blood(&e, r),
                4 - 1 - 1,
                "a counter and a blood, the unlock's"
            );
        }
    }
}

// ---- Code of Samiel ---------------------------------------------------------

/// Code of Samiel: "During your unlock phase, a Salubri antitribu you
/// control can gain 1 blood. If a Salubri antitribu burns a ready vampire
/// or sends a vampire to torpor in combat or as a Ⓓ action, that Salubri
/// antitribu can unlock after the end of the minion phase." Its unlock
/// phase blood once; its Salubri sends a blocker to torpor in the bleed's
/// combat and may unlock as the minion phase ends.
#[test]
fn code_of_samiel_feeds_and_unlocks_its_salubri() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), of_clan("Salubri antitribu", 0), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 0, false);
    hand(&mut e, 0, &["Code of Samiel"]);
    e.at_master_phase();
    pick(&mut e, master_play(0));
    request(&mut e);
    let code = card_named(&e, "Code of Samiel");
    assert!(!e.state.cards[code.index()].effect_only);
    pick_when(&mut e, bleed(s));
    pick_when(&mut e, block(z));
    let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert!(e.state.minions[z.index()].in_torpor, "{req:?}");
    assert!(e.state.minions[s.index()].locked);
    pick(&mut e, Choice::ResolvePrinted { minion: s });
    assert!(!e.state.minions[s.index()].locked);
    // The next unlock phase of its controller: 1 blood, once.
    until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    let feed = use_on(code, 0, Target::Minion(s));
    let before = blood(&e, s);
    pick_when(&mut e, feed);
    assert_eq!(blood(&e, s), before + 1);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, feed);
}

/// Code of Samiel, another Methuselah's minion phase (any Methuselah's
/// Salubri antitribu, the one whose minion phase it is): blocking and
/// sending the acting vampire to torpor earns the unlock after that minion
/// phase.
#[test]
fn code_of_samiel_unlocks_a_blocker_after_anothers_minion_phase() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let a = e.add_minion(SeatId(0), plain("Brujah", 1), 1, false);
    let s = e.add_minion(SeatId(1), of_clan("Salubri antitribu", 0), 3, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    // A second strength: the hand strike sends the 1-blood actor to torpor.
    let fist = in_play(&mut e, 1, "Blooding by the Code", Some(s));
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::Bearer {
            stat: Stat::Strength,
            n: 1,
            during: &[],
            who: zvtes::cards::Bearer::Any,
        },
        source: fist,
    });
    in_play(&mut e, 0, "Code of Samiel", None);
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::UnlockAfterMinionPhaseForClan("Salubri antitribu"),
        source: CardId(e.state.cards.len() as u32 - 1),
    });
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(s));
    let req = until(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
    assert!(e.state.minions[a.index()].in_torpor);
    assert_eq!(
        req.options,
        vec![Choice::Pass, Choice::ResolvePrinted { minion: s }]
    );
    assert_eq!(e.state.active, SeatId(0), "after seat 0's minion phase");
}

/// [LSJ 20100203-2] "The fact it is Unique has no bearing on the burn
/// option. You cannot use the burn option if you meet the requirement, even
/// if you already have a copy of the card in play." Code of Samiel and
/// Command Performance, a copy of each in play: not discarded while their
/// clan's vampire is controlled; discarded without it.
#[test]
fn unique_burn_option_masters_ignore_their_copy_in_play() {
    for (name, clan) in [
        ("Code of Samiel", "Salubri antitribu"),
        ("Command Performance", "Daughter of Cacophony"),
    ] {
        for meets in [true, false] {
            let mut e = game(2);
            e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
            e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
            if meets {
                let card = if clan == "Daughter of Cacophony" {
                    daughter(0)
                } else {
                    of_clan(clan, 0)
                };
                e.add_minion(SeatId(1), card, 4, false);
            }
            in_play(&mut e, 1, name, None);
            hand(&mut e, 1, &[name]);
            e.state.seats[1].library.push(find_library("Soak"));
            pick(&mut e, Choice::Pass);
            let found = loop {
                let req = request(&mut e);
                if req.window == WindowLabel::UnlockAbilities && req.seat == SeatId(1) {
                    break Some(req);
                }
                if (req.seat, req.window) == (SeatId(1), WindowLabel::MinionPhase) {
                    break None;
                }
                e.choose(fallback(&req));
            };
            let offered = found.is_some_and(|r| r.options.contains(&Choice::Discard(0)));
            assert_eq!(offered, !meets, "{name}, meets: {meets}");
        }
    }
}

// ---- Command Performance ----------------------------------------------------

/// Command Performance: "You can lock this card during your minion phase to
/// unlock a ready Daughter of Cacophony. Titled vampires can call a
/// referendum to burn this card as a +1 stealth political action." Any
/// Methuselah's Daughter; not in another's minion phase; another's prince
/// votes it away.
#[test]
fn command_performance_unlocks_a_daughter_and_can_be_voted_away() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), daughter(0), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let p = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[p.index()].granted_title = Some(Title::Prince("Paris"));
    hand(&mut e, 0, &["Command Performance"]);
    e.at_master_phase();
    pick(&mut e, master_play(0));
    request(&mut e);
    let c = card_named(&e, "Command Performance");
    assert!(!e.state.cards[c.index()].effect_only);
    e.state.minions[d.index()].locked = true;
    pick_when(&mut e, use_on(c, 0, Target::Minion(d)));
    request(&mut e);
    assert!(!e.state.minions[d.index()].locked);
    assert!(e.state.cards[c.index()].locked);
    // Seat 1's turn: the card unlocked, not usable; the prince's
    // referendum burns it.
    e.state.minions[d.index()].locked = true;
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    e.state.cards[c.index()].locked = false;
    let lock = use_on(c, 0, Target::Minion(d));
    let req = request(&mut e);
    assert!(!req.options.contains(&lock), "not another's minion phase");
    pick(&mut e, used_by(c, 1, p));
    pick_when(&mut e, votes(p, true));
    close_polling(&mut e);
    request(&mut e);
    assert!(!e.state.cards[c.index()].present());
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Command Performance")));
}

// ---- Coroner's Contact ------------------------------------------------------

/// [PIB 20150530] Lisandro Giovanni "cannot be treated as a … Samedi while
/// uncontrolled or in the crypt (eg. Coroner's Contact cannot fetch him)."
/// Coroner's Contact: "Search your crypt to find a Samedi … place it in your
/// uncontrolled region with 1 blood from the bank."
#[test]
fn coroners_contact_fetches_a_samedi_with_blood() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let samedi = of_clan("Samedi", 0);
    let lisandro = find_card("Lisandro Giovanni");
    e.state.seats[0].crypt.push(samedi);
    e.state.seats[0].crypt.push(lisandro);
    hand(&mut e, 0, &["Coroner's Contact"]);
    e.at_master_phase();
    pick(&mut e, master_play(0));
    let req = request(&mut e);
    assert!(req.options.contains(&Choice::AshVampire(samedi)));
    assert!(!req.options.contains(&Choice::AshVampire(lisandro)));
    pick(&mut e, Choice::AshVampire(samedi));
    let slot = e.state.seats[0].uncontrolled.last().expect("placed");
    assert_eq!((slot.card, slot.counters), (samedi, 1));
    assert_eq!(e.state.seats[0].pool, 9);
}

// ---- Development ------------------------------------------------------------

/// Development: "When drawing to replace this card, you may take your
/// choice of one of the cards from your research area instead of drawing
/// from your library."
#[test]
fn development_replaces_from_the_research_area() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Development"]);
    fill_hand(&mut e, 0);
    let dodge = find_library("Dodge");
    let deflection = find_library("Deflection");
    e.state.seats[0].research.push(dodge);
    e.state.seats[0].research.push(deflection);
    e.state.seats[0].library.push(find_library("Soak"));
    e.at_master_phase();
    pick(&mut e, master_play(0));
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::Draw);
    assert!(req.options.contains(&Choice::DrawLibrary));
    assert!(req.options.contains(&Choice::FindCard(dodge)));
    assert!(req.options.contains(&Choice::FindCard(deflection)));
    pick(&mut e, Choice::FindCard(deflection));
    assert!(e.state.seats[0].hand.contains(&deflection));
    assert_eq!(e.state.seats[0].research.len(), 1);
    assert_eq!(e.state.seats[0].library.len(), 1, "nothing drawn");
}

/// Development held by Hagar Stone (rules-qa 107.3): "when drawing to
/// replace this card" is still that draw once the hold ends, as Hagar
/// locks (rules-qa 73.1, an extra draw following a delayed replacement):
/// the research area is offered then.
#[test]
fn development_held_by_hagar_stone_keeps_its_research_option() {
    let mut e = game(2);
    let hagar = e.add_minion(SeatId(0), find_card("Hagar Stone"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Development"]);
    fill_hand(&mut e, 0);
    let deflection = find_library("Deflection");
    e.state.seats[0].research.push(deflection);
    e.state.seats[0].library.push(find_library("Soak"));
    e.at_master_phase();
    pick(&mut e, master_play(0));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!req.options.contains(&Choice::FindCard(deflection)));
    assert_eq!(
        e.state.seats[0].held.total(),
        1,
        "held while Hagar is unlocked"
    );
    pick(&mut e, action(hagar, ActionKind::Hunt));
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::Draw);
    assert!(req.options.contains(&Choice::DrawLibrary));
    pick(&mut e, Choice::FindCard(deflection));
    assert!(e.state.seats[0].hand.contains(&deflection));
    assert!(e.state.seats[0].research.is_empty());
    assert_eq!(e.state.seats[0].library.len(), 1, "nothing drawn");
}

// ---- Fanfare for Elysium ----------------------------------------------------

/// Fanfare for Elysium [mel]: "Choose up to five older vampires. Successful
/// referendum means each chosen vampire gains 1 blood." Older than the
/// caller, any Methuselah's; the card to the ash heap.
#[test]
fn fanfare_for_elysium_feeds_older_vampires() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), find_card("Harlan Graves"), 3, false);
    e.state.minions[caller.index()].granted_title = Some(Title::Votes(1));
    let old = e.add_minion(SeatId(0), textless(|c| c.capacity >= 7, 0), 2, false);
    let young = e.add_minion(SeatId(1), textless(|c| c.capacity <= 3, 0), 1, false);
    let old2 = e.add_minion(SeatId(1), textless(|c| c.capacity >= 7, 1), 2, false);
    hand(&mut e, 0, &["Fanfare for Elysium"]);
    pick(&mut e, play(0, 0, caller));
    let req = request(&mut e);
    assert!(
        !req.options.contains(&Choice::ChooseMinion(young)),
        "not older"
    );
    pick(&mut e, Choice::ChooseMinion(old));
    pick(&mut e, Choice::ChooseMinion(old2));
    pick_when(&mut e, votes(caller, true));
    close_polling(&mut e);
    request(&mut e);
    assert_eq!(
        (blood(&e, old), blood(&e, old2), blood(&e, young)),
        (3, 3, 1)
    );
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Fanfare for Elysium")));
}

/// Fanfare for Elysium [MEL]: "As above, and successful referendum means
/// this card is put in play. … You can burn this card before range is
/// determined to end a combat." In play, then burned to end another
/// Methuselah's combat.
#[test]
fn fanfare_for_elysium_superior_stays_to_end_a_combat() {
    let mut e = game(2);
    // Hillanvale: -1 intercept against a Camarilla bleeder, none here.
    let caller = e.add_minion(SeatId(0), find_card("Hillanvale"), 3, false);
    e.state.minions[caller.index()].granted_title = Some(Title::Votes(1));
    let independent = |c: &zvtes::crypt::CryptCard| {
        c.sect == Some(zvtes::crypt::Sect::Independent) && c.capacity >= 4
    };
    let z = e.add_minion(SeatId(1), textless(independent, 0), 4, false);
    let y = e.add_minion(SeatId(1), textless(independent, 1), 4, false);
    hand(&mut e, 0, &["Fanfare for Elysium"]);
    pick(&mut e, play(0, 1, caller));
    pick_when(&mut e, votes(caller, true));
    close_polling(&mut e);
    request(&mut e);
    let c = card_named(&e, "Fanfare for Elysium");
    assert!(!e.state.cards[c.index()].effect_only);
    assert!(!e.state.seats[0]
        .ash
        .contains(&find_library("Fanfare for Elysium")));
    // Seat 1's bleed, blocked by seat 0's vampire: before range, burned.
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    e.state.minions[caller.index()].locked = false;
    pick(&mut e, bleed(z));
    pick_when(&mut e, block(caller));
    pick_when(&mut e, use_on(c, 0, Target::None));
    request(&mut e);
    assert!(e.combat().is_none(), "the combat ended");
    assert!(!e.state.cards[c.index()].present());
    assert_eq!((blood(&e, z), blood(&e, caller)), (4, 3));
    let _ = y;
}

// ---- Houngan ----------------------------------------------------------------

/// Houngan: "Put this card on a ready Samedi you control. During your
/// unlock phase, this Samedi gains 1 blood from the bank, and he or she may
/// lock to gain an additional blood. Any other Samedi may move this card to
/// himself or herself as a +1 stealth action." The mandatory blood, the
/// optional lock once; another Samedi of its Methuselah's takes it.
#[test]
fn houngan_feeds_its_samedi_and_can_be_taken() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), of_clan("Samedi", 0), 2, false);
    let s2 = e.add_minion(SeatId(0), of_clan("Samedi", 1), 2, false);
    let torpid = e.add_minion(SeatId(0), of_clan("Samedi", 2), 2, true);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Houngan"]);
    e.at_master_phase();
    let req = request(&mut e);
    assert!(
        !req.options.contains(&master_on(0, Target::Minion(torpid))),
        "a ready Samedi"
    );
    pick(&mut e, master_on(0, Target::Minion(s)));
    request(&mut e);
    let h = card_named(&e, "Houngan");
    assert!(!e.state.cards[h.index()].effect_only);
    // Seat 0's next unlock phase.
    until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    let lock = ability(h.0, 0);
    pick_when(&mut e, lock);
    assert!(e.state.minions[s.index()].locked);
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(blood(&e, s), 4, "1 mandatory, 1 for the lock");
    // The other Samedi takes it.
    pick(&mut e, used_by(h, 1, s2));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[h.index()].host, Some(s2));
}

/// Houngan moved by another Methuselah's Samedi: a +1 stealth action
/// directed at the card's controller; the card stays theirs, its blood in
/// their unlock phase.
#[test]
fn houngan_taken_by_anothers_samedi_stays_its_controllers() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), of_clan("Samedi", 0), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let theirs = e.add_minion(SeatId(1), of_clan("Samedi", 1), 2, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let h = in_play(&mut e, 0, "Houngan", Some(s));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, used_by(h, 1, theirs));
    let a = e.action().expect("the action");
    assert_eq!(a.directed_seat(&e.state), Some(SeatId(0)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[h.index()].host, Some(theirs));
    assert_eq!(e.state.card_controller(h), SeatId(0));
    let before = blood(&e, theirs);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        blood(&e, theirs),
        before + 1,
        "in its controller's unlock phase"
    );
}

/// [LSJ 20071001-1] (Legion, "Create vampire from Master: Discipline") "If
/// an Agent of Power is chosen its card text is not applied. The created
/// vampire has no disciplines, is still non-unique and does not burn in your
/// discard phase."
#[test]
fn legion_fledgling_from_agent_of_power_has_no_discipline() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 7, 0), 7, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let l = in_play(&mut e, 0, "Legion", Some(a));
    hand(&mut e, 0, &["Agent of Power", "Conceal"]);
    e.state.seats[0].library = std::iter::repeat_n(find_library("Conceal"), 10).collect();
    pick(&mut e, used_by(l, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let f = e
        .state
        .minions
        .iter()
        .position(|m| m.in_play && m.tethered_to == Some(a))
        .map(|i| MinionId(i as u32))
        .expect("the fledgling");
    let body = card_named(&e, "Agent of Power");
    assert_eq!(e.state.cards[body.index()].body, Some(f));
    assert_eq!(e.state.capacity_of(f), Some(1));
    assert!(zvtes::state::disciplines()
        .iter()
        .all(|d| e.state.discipline_level(f, d) == 0));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(
        e.state.cards[body.index()].present(),
        "no discard-phase burn"
    );
    assert!(e.state.minions[f.index()].in_play);
}

/// [LSJ 20071005-1] "If you play Agent of Power on a vampire and that
/// vampire plays a Dual Form at superior [PRO], the Dual Form gains the
/// Discipline provided by the Agent of Power and keeps it once the Agent of
/// Power burns."
#[test]
fn dual_form_keeps_agent_of_powers_discipline() {
    let mut e = game(2);
    // Octane: no Dominate printed.
    let a = e.add_minion(SeatId(0), plain("Brujah", 1), 6, false);
    e.state.minions[a.index()].gained_levels = vec!["pro", "pro"];
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Agent of Power", "Dual Form"]);
    e.at_master_phase();
    pick(&mut e, master_on(0, Target::Minion(a)));
    pick_when(&mut e, Choice::ChooseDiscipline("dom"));
    pick_when(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 1,
            actor: Some(a),
            target: Target::None,
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let copy = MinionId(e.state.minions.len() as u32 - 1);
    assert_eq!(e.state.discipline_level(copy, "dom"), 1);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.discipline_level(a, "dom"), 0, "the Agent burned");
    assert_eq!(e.state.discipline_level(copy, "dom"), 1, "kept");
}

// ---- Hunger Moon ------------------------------------------------------------

/// Hunger Moon: "If a vampire successfully hunts, move 1 blood from that
/// vampire to this card after resolution. Burn this card if it has 5
/// blood." Any Methuselah's vampire; burned at the fifth.
#[test]
fn hunger_moon_takes_a_blood_from_each_successful_hunt() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 2, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 2), 2, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let moon = in_play(&mut e, 1, "Hunger Moon", None);
    e.state.cards[moon.index()].counters = 3;
    pick(&mut e, action(a, ActionKind::Hunt));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 2, "1 gained, 1 moved");
    assert_eq!(e.state.cards[moon.index()].counters, 4);
    pick(&mut e, action(b, ActionKind::Hunt));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, b), 2);
    assert!(!e.state.cards[moon.index()].present(), "burned at 5");
    assert!(e.state.seats[1].ash.contains(&find_library("Hunger Moon")));
}

/// [LSJ 20050720-2] "If the vampire gains no blood from their hunt, it's
/// not successful, so Hunger Moon's effect does not apply." A diseased
/// vampire's hunt moves nothing; a hunt at capacity is successful (rules-qa
/// 89.26) and moves 1.
#[test]
fn hunger_moon_ignores_a_hunt_that_gains_nothing() {
    let mut e = game(2);
    let sick = e.add_minion(SeatId(0), plain("Brujah", 0), 2, false);
    e.state.minions[sick.index()].disease = true;
    let full = e.add_minion(SeatId(0), plain("Brujah", 1), 0, false);
    let cap = e.state.capacity_of(full).expect("a vampire");
    e.state.minions[full.index()].blood = cap;
    e.add_minion(SeatId(0), plain("Brujah", 2), 2, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let moon = in_play(&mut e, 0, "Hunger Moon", None);
    pick(&mut e, action(sick, ActionKind::Hunt));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, sick), 2);
    assert_eq!(e.state.cards[moon.index()].counters, 0, "not successful");
    pick(&mut e, action(full, ActionKind::Hunt));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[moon.index()].counters, 1, "at capacity");
    assert_eq!(blood(&e, full), cap - 1);
}

// ---- Lily Prelude -----------------------------------------------------------

/// [LSJ 20020819] "Can allocate more points than a Methuselah has pool."
/// [LSJ 20000309] (several ousts) the VP to each predator, the 6 pool to
/// survivors. Lily Prelude [mel]: "Allocate 4 points among one or more
/// Methuselahs …": all 4 to a prey with 2 pool ousts them. [MEL]: "… and
/// choose a Methuselah. Successful referendum means the chosen Methuselah
/// gains 1 pool": the chosen may take points too.
#[test]
fn lily_prelude_allocates_four_and_feeds_the_chosen() {
    let caller = |e: &mut Engine, sup: bool| {
        let m = e.add_minion(
            SeatId(0),
            find_card(if sup { "Hillanvale" } else { "Harlan Graves" }),
            3,
            false,
        );
        e.state.minions[m.index()].granted_title = Some(Title::Votes(1));
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(2), plain("Gangrel", 0), 4, false);
        hand(e, 0, &["Lily Prelude"]);
        m
    };
    let mut e = game(3);
    let m = caller(&mut e, false);
    e.state.seats[1].pool = 2;
    pick(&mut e, play(0, 0, m));
    pick_when(&mut e, alloc(1, 4));
    pick_when(&mut e, votes(m, true));
    close_polling(&mut e);
    request(&mut e);
    assert!(e.state.seats[1].ousted, "4 points on 2 pool");
    assert_eq!(e.state.seats[0].half_vps, 2);
    assert_eq!(e.state.seats[0].pool, 16);

    let mut e = game(3);
    let m = caller(&mut e, true);
    pick(&mut e, play(0, 1, m));
    pick_when(&mut e, Choice::ChooseSeat(SeatId(0)));
    pick_when(&mut e, alloc(0, 2));
    // Seat 1 none, seat 2's 2 forced.
    pick_when(&mut e, alloc(1, 0));
    pick_when(&mut e, votes(m, true));
    close_polling(&mut e);
    request(&mut e);
    assert_eq!(pool(&e), vec![9, 10, 8], "the chosen burns 2, gains 1");
}

// ---- Maleficia, Striga ------------------------------------------------------

/// Maleficia: "Put this card on an infernal vampire. This vampire may play
/// cards that require Maleficia [mal] (not a Discipline). With two of these
/// cards, … superior Maleficia [MAL]. Burn this card if this vampire is
/// not infernal." Striga the same with [str]. Any Methuselah's infernal
/// vampire; two make superior; burned once not infernal, read live; no
/// Discipline: Agent of Power's choice does not list it.
#[test]
fn maleficia_and_striga_lend_their_power_to_an_infernal() {
    let mut e = game(2);
    let mine = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let theirs = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.state.minions[theirs.index()].made_infernal = true;
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[z.index()].made_infernal = true;
    hand(
        &mut e,
        0,
        &["Maleficia", "Maleficia", "Striga", "Agent of Power"],
    );
    e.at_master_phase();
    e.state.seats[0].mpa = 4;
    let req = request(&mut e);
    assert!(
        !req.options.contains(&master_on(0, Target::Minion(mine))),
        "not infernal"
    );
    assert!(
        req.options.contains(&master_on(0, Target::Minion(z))),
        "any Methuselah's"
    );
    pick(&mut e, master_on(0, Target::Minion(theirs)));
    request(&mut e);
    assert_eq!(e.state.discipline_level(theirs, "mal"), 1);
    let c = card_named(&e, "Maleficia");
    assert!(!e.state.cards[c.index()].effect_only);
    assert!(!e.state.seats[0].ash.contains(&find_library("Maleficia")));
    pick(&mut e, master_on(0, Target::Minion(theirs)));
    request(&mut e);
    assert_eq!(e.state.discipline_level(theirs, "mal"), 2, "two: superior");
    pick(&mut e, master_on(0, Target::Minion(theirs)));
    request(&mut e);
    assert_eq!(e.state.discipline_level(theirs, "str"), 1);
    // Not a Discipline: Agent of Power on the same vampire never lists them.
    pick(&mut e, master_on(0, Target::Minion(theirs)));
    let req = request(&mut e);
    assert!(req.options.contains(&Choice::ChooseDiscipline("dom")));
    assert!(!req.options.iter().any(|o| matches!(
        o,
        Choice::ChooseDiscipline(d) if d.eq_ignore_ascii_case("mal") || d.eq_ignore_ascii_case("str")
    )));
    pick(&mut e, Choice::ChooseDiscipline("dom"));
    request(&mut e);
    // No longer infernal: all three burn.
    e.state.minions[theirs.index()].made_infernal = false;
    request(&mut e);
    assert_eq!(e.state.discipline_level(theirs, "mal"), 0);
    assert_eq!(e.state.discipline_level(theirs, "str"), 0);
    let ash = &e.state.seats[0].ash;
    assert_eq!(
        ash.iter()
            .filter(|&&c| c == find_library("Maleficia") || c == find_library("Striga"))
            .count(),
        3
    );
}

// ---- Obtenebration, Thaumaturgy ---------------------------------------------

/// Obtenebration: "Put this card on a vampire. This vampire gets +1 level of
/// Obtenebration [obt] and +1 capacity. Cannot be put on a vampire with
/// superior Obtenebration [OBT]." Thaumaturgy the same with Blood Sorcery
/// [tha]. Any Methuselah's vampire; two raise it to superior, a third is
/// not offered.
#[test]
fn obtenebration_and_thaumaturgy_add_a_level_and_capacity() {
    for (name, d) in [("Obtenebration", "obt"), ("Thaumaturgy", "tha")] {
        let mut e = game(2);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(
            SeatId(1),
            textless(
                |c| !c.disciplines.iter().any(|x| x.eq_ignore_ascii_case(d)),
                0,
            ),
            2,
            false,
        );
        let cap = e.state.capacity_of(z).expect("a vampire");
        hand(&mut e, 0, &[name, name, name]);
        e.at_master_phase();
        e.state.seats[0].mpa = 3;
        pick(&mut e, master_on(0, Target::Minion(z)));
        request(&mut e);
        assert_eq!(e.state.discipline_level(z, d), 1, "{name}");
        assert_eq!(e.state.capacity_of(z), Some(cap + 1));
        let c = card_named(&e, name);
        assert!(!e.state.cards[c.index()].effect_only);
        assert!(!e.state.seats[0].ash.contains(&find_library(name)));
        pick(&mut e, master_on(0, Target::Minion(z)));
        let req = request(&mut e);
        assert_eq!(e.state.discipline_level(z, d), 2, "{name}");
        assert_eq!(e.state.capacity_of(z), Some(cap + 2));
        assert!(
            !req.options.contains(&master_on(0, Target::Minion(z))),
            "{name}: not on a superior vampire"
        );
    }
}

// ---- The Paths of Harmony, Retribution and Service ----------------------------

/// The `nth` library card whose first mode requires `d`.
fn requiring(d: &str, nth: usize) -> u16 {
    zvtes::library::LIBRARY
        .iter()
        .enumerate()
        .filter(|(_, c)| c.modes.first().is_some_and(|m| m.tags.contains(&d)))
        .nth(nth)
        .map(|(i, _)| i as u16)
        .expect("a card requiring it")
}

/// The Path of Harmony: "Cards requiring Spiritus [spi] cost Ahrimanes 1
/// fewer blood. Any minion can burn this card as a Ⓓ action …"; The Path
/// of Retribution the same for Valeren and Salubri antitribu ("if that
/// minion is a vampire, he or she takes 1 unpreventable damage"); The Path
/// of Service for Visceratika and slave Gargoyles only (a freed one is
/// not). Put in play, any Methuselah's vampires discounted; Retribution
/// burned by another's vampire, who takes 1.
#[test]
fn the_paths_of_harmony_retribution_and_service_discount_and_bite() {
    let mut e = game(2);
    let ahri = e.add_minion(SeatId(0), of_clan("Ahrimane", 0), 3, false);
    let salu = e.add_minion(SeatId(0), of_clan("Salubri antitribu", 0), 3, false);
    let ahri2 = e.add_minion(SeatId(1), of_clan("Ahrimane", 1), 3, false);
    let slave = e.add_minion(SeatId(0), slave_of("Tremere antitribu"), 3, false);
    let free = e.add_minion(SeatId(1), find_card("Grotesque"), 3, false);
    e.state.minions[free.index()].freed_slave = true;
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let (spi, val, vis) = (
        requiring("spi", 0),
        requiring("val", 0),
        requiring("vis", 0),
    );
    hand(
        &mut e,
        0,
        &[
            "Path of Harmony, The",
            "Path of Retribution, The",
            "Path of Service, The",
        ],
    );
    e.at_master_phase();
    e.state.seats[0].mpa = 3;
    for _ in 0..3 {
        pick(&mut e, master_play(0));
        request(&mut e);
    }
    assert_eq!(e.state.seats[0].pool, 7, "1 pool each");
    let off = |m, card| e.state.printed_cost_off(m, card, 0).blood;
    assert_eq!(off(ahri, spi), 1);
    assert_eq!(off(ahri2, spi), 1, "any Methuselah's");
    assert_eq!(off(salu, val), 1);
    assert_eq!(off(slave, vis), 1);
    assert_eq!(off(free, vis), 0, "a freed Gargoyle");
    assert_eq!(off(z, spi), 0);
    let ret = card_named(&e, "Path of Retribution, The");
    assert!(!e.state.cards[ret.index()].effect_only);
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let burn = req
        .options
        .iter()
        .find(|o| matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), actor: Some(m), .. } if *c == ret && *m == z))
        .copied()
        .expect("the burn offered");
    pick(&mut e, burn);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.cards[ret.index()].present());
    assert_eq!(blood(&e, z), 3);
}

// ---- Research ---------------------------------------------------------------

/// Research: "Search your library for three cards (shuffle afterward) and
/// place them face down in your research area. You may look at the cards
/// in your research area at any time. A Methuselah may play only one
/// Research in a game." Up to three, by card; hidden from the others; never
/// a second one.
#[test]
fn research_puts_three_cards_in_the_research_area() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let picks = ["Dodge", "Deflection", "Conceal"].map(find_library);
    hand(&mut e, 0, &["Research", "Research"]);
    fill_hand(&mut e, 0);
    e.state.seats[0].library = ["Dodge", "Deflection", "Conceal", "Soak"]
        .iter()
        .map(|n| find_library(n))
        .collect();
    e.at_master_phase();
    e.state.seats[0].mpa = 2;
    pick(&mut e, master_play(0));
    for c in picks {
        let req = request(&mut e);
        assert_eq!(req.window, WindowLabel::LibrarySearch);
        assert!(req.options.contains(&Choice::Pass), "may find fewer");
        pick(&mut e, Choice::FindCard(c));
    }
    let req = request(&mut e);
    let mut research = e.state.seats[0].research.to_cards();
    research.sort_unstable();
    let mut want = picks.to_vec();
    want.sort_unstable();
    assert_eq!(research, want);
    assert!(e.state.seats[0].library.iter().all(|c| !picks.contains(c)));
    let theirs = e.state.legal_view(SeatId(1));
    assert!(theirs.seats[0]
        .research
        .iter()
        .all(|&c| !picks.contains(&c)));
    assert_eq!(theirs.seats[0].research.len(), 3, "its size public");
    assert!(
        !req.options.contains(&master_play(0)),
        "only one Research in a game"
    );
}

// ---- The Rising -------------------------------------------------------------

/// The Rising in play for seat 0, its layer on.
fn rising(e: &mut Engine) -> CardId {
    let c = in_play(e, 0, "Rising, The", None);
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::PoolGainNeedsEdgeOrVp,
        source: c,
    });
    c
}

/// [ANK 20181105] "Anything that would increase the pool amount is
/// considered 'gaining pool'." The Rising: "A Methuselah cannot gain pool
/// during their own turn unless they have the Edge or at least 1 victory
/// point (instead, any pool they would gain goes to the blood bank)." Minion
/// Tap's blood lost to the bank; with the Edge, or a victory point, kept;
/// another Methuselah's gain in this turn (Lily Prelude's chosen) kept.
#[test]
fn the_rising_withholds_pool_without_the_edge_on_ones_own_turn() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Gangrel", 0), 4, false);
    rising(&mut e);
    hand(&mut e, 0, &["Minion Tap", "Minion Tap", "Minion Tap"]);
    e.at_master_phase();
    e.state.seats[0].mpa = 3;
    pick(&mut e, master_on(0, Target::Minion(a)));
    pick_when(&mut e, set_x(1));
    request(&mut e);
    assert_eq!(
        (blood(&e, a), e.state.seats[0].pool),
        (3, 10),
        "to the bank"
    );
    e.state.seats[0].has_edge = true;
    pick(&mut e, master_on(0, Target::Minion(a)));
    pick_when(&mut e, set_x(1));
    request(&mut e);
    assert_eq!(e.state.seats[0].pool, 11, "with the Edge");
    e.state.seats[0].has_edge = false;
    e.state.seats[0].half_vps = 2;
    pick(&mut e, master_on(0, Target::Minion(a)));
    pick_when(&mut e, set_x(1));
    request(&mut e);
    assert_eq!(e.state.seats[0].pool, 12, "with a victory point");

    let mut e = game(3);
    let m = e.add_minion(SeatId(0), find_card("Hillanvale"), 3, false);
    e.state.minions[m.index()].granted_title = Some(Title::Votes(1));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Gangrel", 0), 4, false);
    rising(&mut e);
    hand(&mut e, 0, &["Lily Prelude"]);
    pick(&mut e, play(0, 1, m));
    pick_when(&mut e, Choice::ChooseSeat(SeatId(1)));
    pick_when(&mut e, alloc(0, 0));
    pick_when(&mut e, alloc(1, 0));
    pick_when(&mut e, votes(m, true));
    close_polling(&mut e);
    request(&mut e);
    assert_eq!(pool(&e), vec![10, 11, 6], "not seat 1's own turn");
}

/// [LSJ 20100206-2] "Methuselahs do not gain the pool granted by a bleed
/// action (eg. Kindred Spirits) if they do not have the edge or a VP
/// beforehand." [LSJ 20100210-2] "Methuselahs gain the pool granted by an
/// effect after a successful bleed action (eg. The unnamed), since they
/// get the edge before the effect kicks in."
#[test]
fn the_rising_takes_a_bleeds_pool_before_its_edge() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("dem", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    rising(&mut e);
    hand(&mut e, 0, &["Kindred Spirits"]);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(a),
            target: Target::Seat(SeatId(1)),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[0].has_edge, "the Edge taken");
    assert_eq!(pool(&e), vec![10, 9], "its pool before the Edge: lost");

    let mut e = game(2);
    let u = e.add_minion(SeatId(0), find_card("unnamed, The"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    rising(&mut e);
    pick(&mut e, bleed(u));
    let after = Choice::UseAbility {
        source: AbilitySource::Vampire(u),
        ability: 1,
        actor: None,
        target: Target::None,
    };
    pick_when(&mut e, after);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 12, "after the Edge: gained");
}

/// [LSJ 20100208] "When ousting their prey, Methuselahs decide if they gain
/// the VP before or after gaining the pool." Ousting the prey without the
/// Edge or a victory point: the victory point first, then the 6 pool
/// kept.
#[test]
fn the_rising_scores_the_victory_point_before_the_ousts_pool() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Gangrel", 0), 4, false);
    rising(&mut e);
    e.state.seats[1].pool = 1;
    pick(&mut e, bleed(a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[1].ousted);
    assert_eq!(e.state.seats[0].half_vps, 2);
    assert_eq!(e.state.seats[0].pool, 16);
}

/// [LSJ 20080805] "Is not replaced until the condition is met, even if it
/// is burned." The Rising: "Requires at least 2 other Gehenna events in
/// play. Do not replace until your prey is ousted." Not replaced when
/// another Methuselah is ousted; replaced as its player's prey is.
#[test]
fn the_rising_waits_for_the_prey_ousted() {
    let mut e = game(4);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Gangrel", 0), 4, false);
    e.add_minion(SeatId(3), plain("Toreador", 0), 4, false);
    in_play(&mut e, 1, "Wormwood", None);
    in_play(&mut e, 1, "Fueled by Heart's Blood", None);
    hand(&mut e, 0, &["Rising, The"]);
    e.state.seats[0].library = std::iter::repeat_n(find_library("Soak"), 5).collect();
    let req = until(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    let play = req
        .options
        .iter()
        .find(|o| matches!(o, Choice::Play { card: 0, .. }))
        .copied()
        .expect("the event offered");
    pick(&mut e, play);
    request(&mut e);
    let c = card_named(&e, "Rising, The");
    assert!(!e.state.cards[c.index()].effect_only);
    let held = |e: &Engine| e.state.seats[0].held[Release::PreyOusted];
    assert_eq!(held(&e), 1, "not replaced");
    // Seat 2 (not seat 0's prey) ousted by seat 1: still held.
    e.state.seats[2].pool = 1;
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(v));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[2].ousted);
    assert_eq!(held(&e), 1, "another's oust");
    // Seat 0's prey ousted by its bleed: replaced.
    e.state.seats[1].pool = 1;
    pick(&mut e, bleed(a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[1].ousted);
    assert_eq!(held(&e), 0, "replaced");
}

// ---- Tend the Flock ---------------------------------------------------------

fn slot(card: u16, counters: u8) -> zvtes::state::UncontrolledSlot {
    zvtes::state::UncontrolledSlot {
        card,
        copy: Default::default(),
        counters,
        banished: None,
    }
}

/// An infernal Baali of capacity 7: Xeper, his votes and steal inert in a
/// master phase (every Baali has printed text since round 28).
fn flock_tender() -> u16 {
    find_card("Xeper, Sultan of Lepers")
}

/// [ANK 20190311] "The blood comes from the blood bank." Tend the Flock:
/// "Choose a ready unlocked infernal Baali you control. Add 3 blood to a
/// younger infernal vampire in your uncontrolled region." An older
/// infernal and a younger non-infernal are not offered.
#[test]
fn tend_the_flock_feeds_a_younger_infernal_in_the_uncontrolled_region() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), flock_tender(), 3, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let cap = e.state.capacity_of(b).expect("a vampire");
    let young = find_card("Horde, The");
    assert!(CRYPT[young as usize].capacity < cap);
    let old = CRYPT
        .iter()
        .position(|c| c.infernal && c.capacity > cap)
        .expect("an older infernal") as u16;
    let mortal = textless(|c| !c.infernal && c.capacity < cap, 0);
    for c in [old, mortal, young] {
        e.state.seats[0].uncontrolled.push(slot(c, 0));
    }
    hand(&mut e, 0, &["Tend the Flock"]);
    e.at_master_phase();
    pick(&mut e, master_on(0, Target::Minion(b)));
    // The only younger infernal: the pick elides.
    request(&mut e);
    let region = &e.state.seats[0].uncontrolled;
    assert_eq!(
        region.iter().map(|u| u.counters).collect::<Vec<_>>(),
        vec![0, 0, 3]
    );
    assert_eq!(e.state.seats[0].pool, 10);
    assert!(!e.state.minions[b.index()].locked, "not locked");
}

/// [LSJ 20031010] [LSJ 20090319] [ANK 20180907] (Secure Haven) "Choosing
/// is targeting: cards that 'choose' the vampire like … Tend the Flock are
/// considered to target it, their cost is increased." Its Baali in a
/// Secure Haven: 1 pool more.
#[test]
fn tend_the_flock_pays_for_a_secure_haven() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), flock_tender(), 3, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let cap = e.state.capacity_of(b).expect("a vampire");
    assert!(CRYPT[find_card("Horde, The") as usize].capacity < cap);
    e.state.seats[0]
        .uncontrolled
        .push(slot(find_card("Horde, The"), 0));
    hand(&mut e, 0, &["Secure Haven", "Tend the Flock"]);
    e.at_master_phase();
    e.state.seats[0].mpa = 2;
    pick(&mut e, master_on(0, Target::Minion(b)));
    request(&mut e);
    let before = e.state.seats[0].pool;
    pick(&mut e, master_on(0, Target::Minion(b)));
    request(&mut e);
    assert_eq!(e.state.seats[0].pool, before - 1, "the haven's +1");
    assert_eq!(e.state.seats[0].uncontrolled[0].counters, 3);
}

// ---- Thicker than Blood -----------------------------------------------------

/// [LSJ 20100206-2] "The vampire coming out of uncontrolled keeps the cards
/// and blood he has, if any." [LSJ 20020609] (Circle) "A vampire with no
/// circle designation is in his own unique circle." Thicker than Blood:
/// "Choose a ready Blood Brother you control. Move a unique Blood Brother of
/// the same circle from your uncontrolled region to your ready region. Not
/// usable if they would contest a card in play." A circle-mate comes out
/// with its blood; one of another circle is not offered; a circleless
/// chooser offers nothing; one whose copy is in play is not offered.
#[test]
fn thicker_than_blood_brings_out_a_circle_mate_with_its_blood() {
    use zvtes::crypt::CryptCard;
    let bb = |c: &CryptCard| c.clan == "Blood Brother" && c.unique;
    let chooser = textless(|c| bb(c) && c.circle.is_some(), 0);
    let circle = CRYPT[chooser as usize].circle;
    let mate = |nth| {
        textless(
            |c| bb(c) && c.circle == circle && c.name != CRYPT[chooser as usize].name,
            nth,
        )
    };
    let other = textless(|c| bb(c) && c.circle.is_some() && c.circle != circle, 0);
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), chooser, 3, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let (m0, m1) = (mate(0), mate(1));
    assert!(CRYPT[m0 as usize].capacity > 0);
    for (c, n) in [(other, 2), (m0, 3), (m1, 1)] {
        e.state.seats[0].uncontrolled.push(slot(c, n));
    }
    // Seat 1 controls a copy of the second mate: it would contest.
    e.add_minion(SeatId(1), m1, 3, false);
    hand(&mut e, 0, &["Thicker than Blood"]);
    e.at_master_phase();
    pick(&mut e, master_on(0, Target::Minion(b)));
    // The first mate alone qualifies: elided.
    request(&mut e);
    let out = e
        .state
        .minions
        .iter()
        .position(|v| v.in_play && v.crypt_card() == Some(m0))
        .map(|i| MinionId(i as u32))
        .expect("the mate is out");
    assert!(e.state.minions[out.index()].ready());
    assert_eq!(e.state.minions[out.index()].owner, SeatId(0));
    assert_eq!(blood(&e, out), 3, "its blood kept");
    assert_eq!(e.state.seats[0].uncontrolled.len(), 2);
    assert_eq!(e.state.seats[0].pool, 8);
    let _ = z;

    // A circleless Blood Brother has no circle-mate: not offered.
    let mut e = game(2);
    let lone = e.add_minion(SeatId(0), find_card("New Blood"), 2, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0].uncontrolled.push(slot(m0, 2));
    hand(&mut e, 0, &["Thicker than Blood"]);
    e.at_master_phase();
    let req = request(&mut e);
    assert!(!req.options.contains(&master_on(0, Target::Minion(lone))));
}

// ---- Action cards -----------------------------------------------------------

/// Action card `card` of the hand, mode `mode`, by `actor` at `target`.
fn act(card: u8, mode: u8, actor: MinionId, target: Target) -> Choice {
    Choice::Play {
        card,
        mode,
        actor: Some(actor),
        target,
    }
}

/// Card `name` in play on `host` with its layers (`in_play` pushes none).
fn on_with(
    e: &mut Engine,
    seat: u8,
    name: &str,
    host: MinionId,
    layers: &[zvtes::cards::Modifier],
) -> CardId {
    let id = in_play(e, seat, name, Some(host));
    for &modifier in layers {
        e.state.layers.push(zvtes::cards::Layer {
            scope: zvtes::cards::Scope::InPlay,
            modifier,
            source: id,
        });
    }
    id
}

/// `n` Maleficia on infernal vampire `m` of seat `seat`: [mal], [MAL] at 2.
fn maleficia(e: &mut Engine, seat: u8, m: MinionId, n: usize) {
    for _ in 0..n {
        on_with(
            e,
            seat,
            "Maleficia",
            m,
            &[zvtes::cards::Modifier::BearerDisciplineLevel("mal")],
        );
    }
}

/// Seat `seat`'s plain vampire (Aintzane Arriaga, capacity 6) made
/// infernal: every printed infernal has text since round 28.
fn infernal(e: &mut Engine, seat: u8, blood: u8, torpor: bool) -> MinionId {
    let m = e.add_minion(SeatId(seat), plain("Lasombra", 0), blood, torpor);
    e.state.minions[m.index()].made_infernal = true;
    m
}

/// The Ailing Spirit [obe]: "Bleed with +1 bleed. Vampires with Dementation
/// [dem], mortals, and mages get -1 intercept during this action." The
/// prey's Dementation vampire, mortal and mage ally cannot reach the
/// 0-stealth bleed, another vampire can; the bleed is for 2.
/// [LSJ 20011204-3] [PIB 20130704] played with [obe], it counts as
/// requiring Obfuscate: its [obe] text applies.
#[test]
fn ailing_spirit_obfuscate_bleed_strips_dementia_mortal_and_mage_intercept() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["obe"];
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let d = e.add_minion(SeatId(1), vampire_with("dem", 0), 4, false);
    let p = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let mortal = e.add_ally(SeatId(1), find_library("Political Ally"));
    let mage = e.add_ally(SeatId(1), find_library("Veneficti"));
    assert!(e.state.discipline_level(p, "dem") == 0);
    hand(&mut e, 0, &["Ailing Spirit, The"]);
    pick(&mut e, act(0, 1, a, Target::None));
    until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    for m in [d, mortal, mage] {
        assert!(!e.block_can_reach(SeatId(1), m), "-1 intercept");
    }
    assert!(e.block_can_reach(SeatId(1), p));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 8, "bleed with +1 bleed");
}

/// The Ailing Spirit [OBE]: "+1 stealth action. Ⓓ Take control of a
/// Malkavian, Malkavian antitribu, mortal, or mage until the end of the
/// turn." Another Methuselah's mage taken, back as the turn ends; a
/// Ventrue is no target.
#[test]
fn ailing_spirit_superior_controls_a_mage_until_the_turn_ends() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["obe", "obe"];
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let malk = e.add_minion(SeatId(1), of_clan("Malkavian antitribu", 0), 4, false);
    let mage = e.add_ally(SeatId(1), find_library("Nephandus"));
    hand(&mut e, 0, &["Ailing Spirit, The"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&act(0, 2, a, Target::Minion(v))));
    assert!(req.options.contains(&act(0, 2, a, Target::Minion(malk))));
    pick(&mut e, act(0, 2, a, Target::Minion(mage)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[mage.index()].owner, SeatId(0));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.minions[mage.index()].owner,
        SeatId(1),
        "until the end of the turn"
    );
}

/// Barrenness [mal]: "Put this card on a vampire and unlock this acting
/// vampire. This vampire is sterile and his or her capacity decreases by 2
/// (to a minimum of 1.)" Played with a Maleficia (not a Discipline): the
/// card itself on the prey's Tremere, its capacity 2 less, sterile — no
/// Create Gargoyle for it — and the actor unlocked.
#[test]
fn barrenness_sterilizes_and_shrinks_a_vampire() {
    let mut e = game(2);
    let a = infernal(&mut e, 0, 4, false);
    maleficia(&mut e, 0, a, 1);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let t = e.add_minion(SeatId(1), plain("Tremere", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let cap = e.state.capacity_of(t).expect("a vampire");
    hand(&mut e, 0, &["Barrenness"]);
    hand(&mut e, 1, &["Create Gargoyle"]);
    pick(&mut e, act(0, 0, a, Target::Minion(t)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let c = card_named(&e, "Barrenness");
    assert_eq!(e.state.cards[c.index()].host, Some(t));
    assert!(!e.state.cards[c.index()].effect_only);
    assert!(!e.state.seats[0].ash.contains(&find_library("Barrenness")));
    assert_eq!(e.state.capacity_of(t), Some(cap - 2));
    assert!(e.state.is_sterile(t));
    assert!(!e.state.minions[a.index()].locked, "the actor unlocked");
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(
        !req.options.contains(&act(0, 0, t, Target::None)),
        "sterile"
    );
}

/// [ANK 20180719-1] "[mal] is unaffected by Orun. [MAL] is affected by Orun
/// (add and subtract before applying the minimum of 1)." A 1-capacity
/// Laibon of capacity 3 with an Orun and two Barrenness reads 3 + 2 - 4 = 1
/// to another Methuselah, not 1 + 2: [MAL] burns it; one with no
/// Barrenness reads 5.
#[test]
fn barrenness_superior_reads_orun_before_the_minimum() {
    use zvtes::cards::Modifier as M;
    let mut e = game(2);
    let a = infernal(&mut e, 0, 4, false);
    maleficia(&mut e, 0, a, 2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let t = e.add_minion(SeatId(1), textless(|c| c.capacity == 3, 1), 1, false);
    e.state.minions[t.index()].sect_override = Some(zvtes::crypt::Sect::Laibon);
    let big = e.add_minion(SeatId(1), textless(|c| c.capacity == 3, 0), 1, false);
    e.state.minions[big.index()].sect_override = Some(zvtes::crypt::Sect::Laibon);
    for m in [t, big] {
        on_with(
            &mut e,
            1,
            "Orun",
            m,
            &[M::CapacityToOthers(2, zvtes::crypt::Sect::Laibon)],
        );
    }
    for _ in 0..2 {
        on_with(
            &mut e,
            0,
            "Barrenness",
            t,
            &[M::HostSterile, M::HostCapacityDown(2)],
        );
    }
    assert_eq!(e.state.capacity_by(t, SeatId(0)), Some(1));
    assert_eq!(e.state.capacity_by(big, SeatId(0)), Some(5));
    hand(&mut e, 0, &["Barrenness"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&act(0, 1, a, Target::Minion(big))));
    pick(&mut e, act(0, 1, a, Target::Minion(t)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[t.index()].in_play, "burned");
}

/// [LSJ 20091203] "The burn option can be used if all your vampires with
/// [mal] are in torpor." Barrenness and Minor Curse discarded in the unlock
/// phase while the Maleficia vampire is torpid, not while it is ready.
#[test]
fn barrenness_burn_option_with_its_vampire_in_torpor() {
    for (card, torpid) in [
        ("Barrenness", true),
        ("Barrenness", false),
        ("Minor Curse", true),
        ("Minor Curse", false),
    ] {
        let mut e = game(2);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let m = infernal(&mut e, 1, 4, torpid);
        maleficia(&mut e, 1, m, 1);
        hand(&mut e, 1, &[card]);
        e.state.seats[1].library = vec![find_library("Soak"); 2].into();
        if !torpid {
            never_offered_until(
                &mut e,
                SeatId(1),
                WindowLabel::MinionPhase,
                Choice::Discard(0),
            );
            continue;
        }
        let req = until(&mut e, SeatId(1), WindowLabel::UnlockAbilities);
        assert!(req.options.contains(&Choice::Discard(0)), "{card}");
        pick(&mut e, Choice::Discard(0));
        assert!(e.state.seats[1].ash.contains(&find_library(card)));
    }
}

/// Blessing of the Name: "Usable by a locked Salubri antitribu. You may not
/// play this card if you have a Blessing of the Name in play. Put this card
/// in play and choose this acting vampire and up to two other minions you
/// control. The chosen minions unlock, and they have +1 intercept … During
/// your unlock phase, burn this card and lock the chosen minions." A locked
/// Salubri antitribu plays it, choosing a locked ally: both unlock, +1
/// intercept; no second copy; the next unlock phase burns it, both locked.
#[test]
fn blessing_of_the_name_unlocks_and_marks_its_chosen() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), of_clan("Salubri antitribu", 0), 4, false);
    let ally = e.add_ally(SeatId(0), find_library("Carlton Van Wyk"));
    let torpid = e.add_minion(SeatId(0), plain("Brujah", 0), 2, true);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    for m in [s, ally, torpid] {
        e.state.minions[m.index()].locked = true;
    }
    hand(&mut e, 0, &["Blessing of the Name", "Blessing of the Name"]);
    pick(&mut e, act(0, 0, s, Target::None));
    pick(&mut e, Choice::ChooseMinion(ally));
    // "Minions you control": a torpid one too (rules-digest D28).
    pick(&mut e, Choice::ChooseMinion(torpid));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let c = card_named(&e, "Blessing of the Name");
    assert!(!e.state.cards[c.index()].effect_only);
    assert_eq!(e.state.cards[c.index()].chosen, vec![s, ally, torpid]);
    for m in [s, ally, torpid] {
        assert!(!e.state.minions[m.index()].locked);
    }
    for m in [s, ally] {
        assert_eq!(e.state.bearer_mod(m, Stat::Intercept, ActionKind::Bleed), 1);
    }
    assert!(
        !req.options.contains(&act(0, 0, s, Target::None)),
        "one in play"
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[c.index()].present(), "burned");
    for m in [s, ally, torpid] {
        assert!(e.state.minions[m.index()].locked);
    }
}

/// Strike: `w`'s melee weapon strike by `a`, blocked by `z` (Fortitude,
/// Relentless Reaper in hand, so the damage stops before prevention):
/// (normal, aggravated) pending on `z`.
fn melee_pending(e: &mut Engine, a: MinionId, z: MinionId, w: Option<CardId>) -> (u8, u8) {
    e.state.minions[z.index()].gained_levels = vec!["for"];
    hand(e, 1, &["Relentless Reaper"]);
    pick(e, bleed(a));
    pick_when(e, block(z));
    if let Some(w) = w {
        pick_when(e, use_on(w, 0, Target::None));
    }
    offering(e, play(0, 0, z));
    let d = e.combat().expect("combat").sides[1].pending;
    (d.normal, d.aggravated)
}

/// [RTR 19970630] [TOM 19960225] "Additional damage inherits all of the
/// properties of the base damage." Blessing of the Name's chosen strikes
/// with Femur of Toomler (strength+1 aggravated): 1 + 1 + 1 aggravated; its
/// hand strike gets nothing.
#[test]
fn blessing_of_the_name_adds_aggravated_damage_to_a_melee_weapon() {
    use zvtes::cards::Modifier as M;
    for weapon in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let w = in_play(&mut e, 0, "Femur of Toomler", Some(a));
        let b = on_with(
            &mut e,
            0,
            "Blessing of the Name",
            a,
            &[
                M::ChosenStat {
                    stat: Stat::Intercept,
                    n: 1,
                },
                M::ChosenStat {
                    stat: Stat::MeleeDamage,
                    n: 1,
                },
            ],
        );
        e.state.cards[b.index()].host = None;
        e.state.cards[b.index()].chosen = vec![a];
        let pending = melee_pending(&mut e, a, z, weapon.then_some(w));
        assert_eq!(pending, if weapon { (0, 3) } else { (1, 0) });
    }
}

/// Blood Tempering [for][qui]: "Put this card on an equipment. This melee
/// weapon inflicts +1 damage each strike." On a Femur of Toomler: 1 + 1 + 1
/// aggravated [RTR 19970630] [TOM 19960225]; the card itself on the weapon;
/// "An equipment can have only one Blood Tempering": no second offered;
/// [FOR][QUI] unlocks the actor.
#[test]
fn blood_tempering_sharpens_a_melee_weapon_once() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let smith = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.state.minions[smith.index()].gained_levels = vec!["for", "for", "qui", "qui"];
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let w = in_play(&mut e, 0, "Femur of Toomler", Some(a));
    hand(&mut e, 0, &["Blood Tempering", "Blood Tempering"]);
    pick(&mut e, act(0, 1, smith, Target::Card(w)));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let c = card_named(&e, "Blood Tempering");
    assert_eq!(e.state.cards[c.index()].on_card, Some(w));
    assert!(!e.state.cards[c.index()].effect_only);
    assert!(!e.state.seats[0]
        .ash
        .contains(&find_library("Blood Tempering")));
    assert!(!e.state.minions[smith.index()].locked, "[FOR][QUI] unlocks");
    assert!(
        !req.options.contains(&act(0, 0, smith, Target::Card(w))),
        "only one"
    );
    assert_eq!(melee_pending(&mut e, a, z, Some(w)), (0, 3));
}

/// Blood Tempering: "If this equipment prevents damage in combat, it can
/// prevent 1 additional point." Flak Jacket prevents both points of a
/// Femur of Toomler's strike.
#[test]
fn blood_tempering_adds_a_point_to_armor() {
    use zvtes::cards::Modifier as M;
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let w = in_play(&mut e, 0, "Femur of Toomler", Some(a));
    let jacket = in_play(&mut e, 1, "Flak Jacket", Some(z));
    let t = on_with(
        &mut e,
        1,
        "Blood Tempering",
        z,
        &[M::WeaponDamage(1), M::EquipmentPreventsMore(1)],
    );
    e.state.cards[t.index()].on_card = Some(jacket);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, use_on(w, 0, Target::None));
    pick_when(&mut e, use_on(jacket, 0, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[z.index()];
    assert!(
        v.ready() && v.blood == 4,
        "both aggravated points prevented"
    );
}

/// Blood Tempering on a Kevlar Vest ("Once each combat, the bearer can
/// prevent 2 damage from gun strikes or 1 damage from any other source";
/// [LSJ 20081210] the 2 spread over gun strikes): 3 gun points a combat,
/// or 2 of any other source (rules-qa 113.60). An AK-47's two strikes
/// (2 + 2): 2 prevented, then 1, the fourth point not offered. A tempered
/// Femur of Toomler's 3 aggravated: 2 prevented, the last sends to torpor.
#[test]
fn blood_tempering_on_a_kevlar_vest_prevents_three_gun_points_or_two_others() {
    use zvtes::cards::Modifier as M;
    for gun in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let w = in_play(
            &mut e,
            0,
            if gun { "AK-47" } else { "Femur of Toomler" },
            Some(a),
        );
        if !gun {
            let t = on_with(&mut e, 0, "Blood Tempering", a, &[M::WeaponDamage(1)]);
            e.state.cards[t.index()].on_card = Some(w);
        }
        let vest = in_play(&mut e, 1, "Kevlar Vest", Some(z));
        let t = on_with(
            &mut e,
            1,
            "Blood Tempering",
            z,
            &[M::WeaponDamage(1), M::EquipmentPreventsMore(1)],
        );
        e.state.cards[t.index()].on_card = Some(vest);
        blocked(&mut e, a, z);
        pick_when(&mut e, use_on(w, 0, Target::None));
        if gun {
            pick_when(&mut e, use_on(vest, 0, Target::None));
            pick_when(&mut e, use_on(vest, 0, Target::None));
            pick_when(&mut e, use_on(w, 0, Target::None));
            pick_when(&mut e, use_on(vest, 0, Target::None));
            never_offered_where(
                &mut e,
                SeatId(0),
                WindowLabel::MinionPhase,
                uses_ability(vest, 0),
            );
            let v = &e.state.minions[z.index()];
            assert!(v.ready() && v.blood == 4 - 1, "3 of 4 gun points prevented");
        } else {
            offering(&mut e, use_on(vest, 1, Target::None));
            assert_eq!(e.combat().expect("combat").sides[1].pending.aggravated, 3);
            pick(&mut e, use_on(vest, 1, Target::None));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            let v = &e.state.minions[z.index()];
            assert!(!v.ready() && v.blood == 4, "2 of 3 aggravated prevented");
        }
    }
}

/// The vampire `card` (in play) became, its minion.
fn body_of(e: &Engine, card: &str) -> MinionId {
    e.state.cards[card_named(e, card).index()]
        .body
        .expect("a created vampire")
}

/// Brother in Arms: "Requires a ready, non-sterile Salubri antitribu with
/// capacity above 2. Put this card in play; it becomes a 2-capacity,
/// non-unique Salubri antitribu of the same sect as the acting Salubri
/// antitribu with basic Valeren [val]. Move 1 blood from the acting vampire
/// to this vampire. This vampire cannot act this turn." Then, its sire in
/// torpor at the next unlock phase, one Brother against no ready unique
/// Salubri antitribu: it burns; with its sire ready it stays. [LSJ
/// 20020115] "Is not a vampire in the ash heap": its library card there.
#[test]
fn brother_in_arms_joins_and_burns_in_excess() {
    for sire_ready in [true, false] {
        let mut e = game(2);
        let s = e.add_minion(
            SeatId(0),
            textless(|c| c.clan == "Salubri antitribu" && c.capacity >= 3, 0),
            4,
            false,
        );
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Brother in Arms"]);
        pick(&mut e, act(0, 0, s, Target::None));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let b = body_of(&e, "Brother in Arms");
        let v = &e.state.minions[b.index()];
        assert!(v.is_vampire() && v.recruited && v.crypt_card().is_none());
        assert_eq!(v.clan(), Some("Salubri antitribu"));
        assert_eq!(e.state.capacity_of(b), Some(2));
        assert_eq!(e.state.discipline_level(b, "val"), 1);
        assert_eq!(e.state.sect_of(b), e.state.sect_of(s));
        assert_eq!((blood(&e, b), blood(&e, s)), (1, 3));
        assert_eq!(pool(&e)[0], 9, "1 pool");
        if !sire_ready {
            e.state.minions[s.index()].in_torpor = true;
        }
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.minions[b.index()].in_play, sire_ready);
        if !sire_ready {
            assert!(e.state.seats[0]
                .ash
                .contains(&find_library("Brother in Arms")));
        }
    }
}

/// Charge of the Buffalo [spi]: "Ⓓ Enter combat with a minion. Set the
/// range for the first round of that combat to close, and this acting
/// vampire's initial strike during that round is strike: hand strike at +1
/// damage." [RTR 19970630] [ANK 20180720] the range set, no Determine Range
/// step; the committed hand strike its only initial strike (elided),
/// strength + 1; with aggravated hands, aggravated [TOM 19960225].
/// [LSJ 20011204-3] [PIB 20130704]: played with [spi], its [spi] text.
#[test]
fn charge_of_the_buffalo_spirit_commits_a_close_hand_strike() {
    for aggravated in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("spi", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        e.state.minions[t.index()].gained_levels = vec!["for"];
        in_play(&mut e, 0, "Femur of Toomler", Some(a));
        if aggravated {
            e.state.minions[a.index()].timed.push((
                zvtes::cards::Lasting::AggravatedHands,
                zvtes::state::Expiry::EndOfTurn,
                SeatId(0),
            ));
        }
        let strength = e.state.base_strength(a);
        hand(&mut e, 0, &["Charge of the Buffalo"]);
        hand(&mut e, 1, &["Relentless Reaper"]);
        pick(&mut e, act(0, 1, a, Target::Minion(t)));
        loop {
            let req = request(&mut e);
            assert_ne!(req.window, WindowLabel::CombatDetermineRange, "range set");
            assert!(
                !(req.seat == SeatId(0) && req.window == WindowLabel::CombatStrike),
                "the committed strike only"
            );
            if req.options.contains(&play(0, 0, t)) {
                break;
            }
            e.choose(fallback(&req));
        }
        let c = e.combat().expect("combat");
        assert!(c.range_set && c.range == zvtes::combat::Range::Close);
        let d = c.sides[1].pending;
        let want = if aggravated {
            (0, strength + 1)
        } else {
            (strength + 1, 0)
        };
        assert_eq!((d.normal, d.aggravated), want);
    }
}

/// Charge of the Buffalo [for]: "Ⓓ Enter combat with a locked minion."
/// [ANK 20210124] [LSJ 20070411] [LSJ 20090514] the target unlocked by
/// resolution: it fizzles, no combat.
#[test]
fn charge_of_the_buffalo_fortitude_fizzles_on_an_unlocked_target() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("for", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[t.index()].locked = true;
    hand(&mut e, 0, &["Charge of the Buffalo"]);
    pick(&mut e, act(0, 0, a, Target::Minion(t)));
    e.state.minions[t.index()].locked = false;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Fizzled));
    assert_eq!(blood(&e, t), 4, "no combat");
}

/// Cheat the Fates [tem]: "Ⓓ Bleed. Minions without Temporis [tem] cannot
/// block this action". [TEM]: "and unlock this vampire". The prey's
/// Temporis vampire may attempt, its other vampire and ally may not; the
/// bleed for 1, the actor unlocked after it, 3 blood paid. [pre]: +2 bleed.
/// [LSJ 20011204-3] [PIB 20130704]: each played with its Discipline, its
/// own text.
#[test]
fn cheat_the_fates_temporis_bars_blockers_and_unlocks() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 1), 5, false);
    e.state.minions[a.index()].gained_levels = vec!["tem", "tem"];
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let tem = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[tem.index()].gained_levels = vec!["tem"];
    let p = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Carlton Van Wyk"));
    hand(&mut e, 0, &["Cheat the Fates", "Cheat the Fates"]);
    pick(&mut e, act(0, 2, a, Target::None));
    let req = until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(req.options.contains(&block(tem)));
    assert!(!req.options.contains(&block(p)) && !req.options.contains(&block(ally)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 9);
    assert_eq!(blood(&e, a), 2, "3 blood");
    assert!(!e.state.minions[a.index()].locked);
    pick(&mut e, act(0, 0, b, Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 9 - 3, "bleed with +2 bleed");
}

/// Create Gargoyle: "it represents a 2-capacity slave Gargoyle of the same
/// sect enslaved to the acting vampire's clan with basic Visceratika [vis]
/// and flight [FLIGHT] … Move up to 3 blood from the acting vampire to this
/// Gargoyle. This Gargoyle cannot act this turn." [ANK 20190203] played by
/// a Tremere antitribu: the Gargoyle its clan's slave, 3 blood moved.
#[test]
fn create_gargoyle_by_a_tremere_antitribu_makes_its_slave() {
    let mut e = game(2);
    let t = e.add_minion(
        SeatId(0),
        textless(|c| c.clan == "Tremere antitribu" && c.capacity >= 5, 0),
        5,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Create Gargoyle"]);
    pick(&mut e, act(0, 0, t, Target::None));
    pick_when(&mut e, Choice::SetX(3));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let g = body_of(&e, "Create Gargoyle");
    let v = &e.state.minions[g.index()];
    assert!(v.is_vampire() && v.recruited);
    assert_eq!(v.clan(), Some("Gargoyle"));
    assert_eq!(e.state.slave_clan(g), Some("Tremere antitribu"));
    assert_eq!(e.state.capacity_of(g), Some(2));
    assert_eq!(e.state.discipline_level(g, "vis"), 1);
    assert!(e.state.has_discipline(g, "flight"));
    assert_eq!(e.state.sect_of(g), e.state.sect_of(t));
    assert_eq!((blood(&e, g), blood(&e, t)), (2, 2), "3 moved, 1 drained");
    assert_eq!(pool(&e)[0], 8, "2 pool");
}

/// Decompose [thn]: "Put this card on any minion. The attached minion gets
/// -1 stealth on non-hunt actions and cannot use additional strikes."
/// [LSJ 20011214-5] "The affected minion cannot play a card that provides
/// an additional strike, even if just to benefit from another effect": the
/// prey's Celerity vampire offered no Blur in the combat of its block.
/// Its bleed at -1 stealth, its hunt at +1.
#[test]
fn decompose_slows_and_bars_additional_strikes() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("thn", 0), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let t = e.add_minion(SeatId(1), vampire_with("cel", 0), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Decompose"]);
    hand(&mut e, 1, &["Blur"]);
    pick(&mut e, act(0, 0, a, Target::Minion(t)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let c = card_named(&e, "Decompose");
    assert_eq!(e.state.cards[c.index()].host, Some(t));
    assert!(!e.state.cards[c.index()].effect_only);
    pick(&mut e, bleed(b));
    pick_when(&mut e, block(t));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        act(0, 0, t, Target::None),
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(t));
    assert_eq!(e.action().expect("bleed").stealth(&e.state), -1);
}

/// Decompose [thn]: "During their unlock phase, the attached minion can
/// burn 2 blood or life to burn this card." Its host, the prey's, in its
/// controller's unlock phase: 2 blood, the card to its owner's ash heap.
#[test]
fn decompose_burns_for_two_blood_in_its_hosts_unlock_phase() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("thn", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Decompose"]);
    pick(&mut e, act(0, 0, a, Target::Minion(t)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let c = card_named(&e, "Decompose");
    pick_when(&mut e, ability(c.0, 0));
    request(&mut e);
    assert_eq!(e.state.active, SeatId(1));
    assert_eq!(blood(&e, t), 2);
    assert!(!e.state.cards[c.index()].present());
    assert!(e.state.seats[0].ash.contains(&find_library("Decompose")));
}

/// The prey's `t` blocks `b`'s bleed; its Psyche! offered at the press:
/// the blood it costs `t`.
fn psyche_cost(e: &mut Engine, b: MinionId, t: MinionId) -> u8 {
    hand(e, 1, &["Psyche!"]);
    pick(e, bleed(b));
    pick_when(e, block(t));
    let psyche = play(0, 0, t);
    let req = offering(e, psyche);
    let before = blood(e, t);
    e.choose(index_of(&req, psyche));
    request(e);
    before - blood(e, t)
}

/// Decompose [THN]: "presses cost the attached minion 1 additional blood
/// or life to use." Psyche! (no cost) costs its host 1 blood more than
/// with [thn]'s Decompose.
#[test]
fn decompose_superior_taxes_presses() {
    let cost = |mode: u8| {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("THN", 0), 4, false);
        let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let t = e.add_minion(SeatId(1), vampire_with("cel", 0), 6, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Decompose"]);
        pick(&mut e, act(0, mode, a, Target::Minion(t)));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        psyche_cost(&mut e, b, t)
    };
    assert_eq!(cost(1), cost(0) + 1);
}

/// Defender of the Haven: "Put this card on a slave Gargoyle with a
/// capacity above 4. This Gargoyle gets +2 intercept against directed
/// actions. A Gargoyle can have only one Defender of the Haven." [ANK
/// 20190203] played by a Tremere antitribu on its own Tremere slave
/// Gargoyle: +2 intercept blocking a bleed, none against a hunt; no second
/// one, none on a 4-capacity slave.
#[test]
fn defender_of_the_haven_guards_a_big_slave_against_directed_actions() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), of_clan("Tremere antitribu", 0), 4, false);
    // Cedric (capacity 6) and Rusticus (4): slaves whose text the test
    // never reaches.
    let g = e.add_minion(SeatId(0), find_card("Cedric"), 4, false);
    let small = e.add_minion(SeatId(0), find_card("Rusticus"), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(
        &mut e,
        0,
        &["Defender of the Haven", "Defender of the Haven"],
    );
    let req = request(&mut e);
    assert!(!req.options.contains(&act(0, 0, t, Target::Minion(small))));
    pick(&mut e, act(0, 0, t, Target::Minion(g)));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let c = card_named(&e, "Defender of the Haven");
    assert_eq!(e.state.cards[c.index()].host, Some(g));
    assert!(!plays(&req).iter().any(|o| matches!(o,
        Choice::Play { target: Target::Minion(m), .. } if *m == g)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(z));
    until(&mut e, SeatId(0), WindowLabel::ActionBlock);
    assert_eq!(e.action().expect("bleed").intercept_of(&e.state, g), 2);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let z2 = e.add_minion(SeatId(1), plain("Ventrue", 2), 4, false);
    pick(&mut e, action(z2, ActionKind::Hunt));
    until(&mut e, SeatId(0), WindowLabel::ActionBlock);
    assert_eq!(e.action().expect("hunt").intercept_of(&e.state, g), 0);
}

/// Greater Curse [dai]: "+1 stealth action. Ⓓ Bleed and gain 1 pool."
/// [LSJ 20011204-3] [PIB 20130704]: played with [dai], its [dai] text.
#[test]
fn greater_curse_daimoinon_bleeds_and_gains_a_pool() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["dai"];
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Greater Curse"]);
    request(&mut e);
    let before = blood(&e, a);
    pick(&mut e, act(0, 0, a, Target::None));
    request(&mut e);
    assert_eq!(e.action().expect("bleed").stealth(&e.state), 1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e), vec![11, 9]);
    assert_eq!(blood(&e, a), before - 1, "1 blood");
}

/// Greater Curse [mal]: "Put this card on an ally or a younger vampire.
/// Bleed actions and combat cards cost this minion an additional blood or
/// life." A younger prey vampire (not an older one): its bleed costs it 1
/// blood, its Psyche! 1.
#[test]
fn greater_curse_taxes_bleeds_and_combat_cards() {
    let mut e = game(2);
    let a = infernal(&mut e, 0, 4, false);
    maleficia(&mut e, 0, a, 1);
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let cap = e.state.capacity_of(a).expect("a vampire");
    let t = e.add_minion(
        SeatId(1),
        textless(|c| c.capacity < cap && c.disciplines.contains(&"cel"), 0),
        3,
        false,
    );
    let old = e.add_minion(SeatId(1), textless(|c| c.capacity > cap, 0), 4, false);
    hand(&mut e, 0, &["Greater Curse"]);
    let req = request(&mut e);
    assert!(
        !req.options.contains(&act(0, 1, a, Target::Minion(old))),
        "younger"
    );
    pick(&mut e, act(0, 1, a, Target::Minion(t)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let c = card_named(&e, "Greater Curse");
    assert_eq!(e.state.cards[c.index()].host, Some(t));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let before = blood(&e, t);
    pick(&mut e, bleed(t));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, t), before - 1, "the bleed's toll");
    e.state.minions[t.index()].blood = 3;
    e.state.minions[t.index()].locked = false;
    let taxed = psyche_cost(&mut e, b, t);
    let mut plain_game = game(2);
    let b2 = plain_game.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    plain_game.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t2 = plain_game.add_minion(
        SeatId(1),
        e.state.minions[t.index()].crypt_card().unwrap(),
        3,
        false,
    );
    plain_game.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    assert_eq!(
        taxed,
        psyche_cost(&mut plain_game, b2, t2) + 1,
        "a combat card's +1"
    );
}

/// Harmony: "+1 stealth action. Ⓓ Your prey burns 3 pool for each card
/// named Choir you control. Burn all those cards." No Choir: offered, a
/// legal no-op.
#[test]
fn harmony_without_a_choir_is_a_legal_no_op() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), daughter(0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Harmony"]);
    pick(&mut e, act(0, 0, d, Target::Seat(SeatId(1))));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Successful));
    assert_eq!(pool(&e), vec![10, 10]);
}

/// Hatchling: "This card represents a 1-capacity Gargoyle of the same sect
/// with basic Visceratika [vis]. Move 1 blood from the bank to this
/// Gargoyle. This Gargoyle cannot act this turn. Once this game, this
/// Gargoyle can burn 1 blood during your unlock phase to gain flight."
/// [LSJ 20100221] "The created vampire is not unique": two Hatchlings in
/// play, no contest; flight bought once.
#[test]
fn hatchlings_are_non_unique_gargoyles_that_learn_flight_once() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), of_clan("Gargoyle", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Hatchling", "Hatchling"]);
    request(&mut e);
    let before = blood(&e, g);
    pick(&mut e, act(0, 0, g, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let h = body_of(&e, "Hatchling");
    let v = &e.state.minions[h.index()];
    assert!(v.is_vampire() && v.recruited && e.state.slave_clan(h).is_none());
    assert_eq!(v.clan(), Some("Gargoyle"));
    assert_eq!(
        (blood(&e, h), blood(&e, g)),
        (1, before - 2),
        "1 from the bank, 2 paid"
    );
    assert_eq!(e.state.capacity_of(h), Some(1));
    assert_eq!(e.state.discipline_level(h, "vis"), 1);
    assert!(!e.state.has_discipline(h, "flight"));
    let c = card_named(&e, "Hatchling");
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick_when(&mut e, ability(c.0, 0));
    request(&mut e);
    assert!(e.state.has_discipline(h, "flight"));
    assert_eq!(blood(&e, h), 0);
    e.state.minions[h.index()].blood = 1;
    let req = to_minion_phase(&mut e, SeatId(0));
    let _ = req;
    pick(&mut e, act(0, 0, g, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let hatchlings: Vec<_> = e
        .state
        .minions
        .iter()
        .filter(|m| m.in_play && m.clan() == Some("Gargoyle") && m.crypt_card().is_none())
        .collect();
    assert_eq!(hatchlings.len(), 2);
    assert!(hatchlings.iter().all(|m| !m.contested));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, ability(c.0, 0));
}

/// Horseshoes [POT]: "Ⓓ Inflict 2 unpreventable damage on a ready minion."
/// [ANK 20220507] "If used on an ally and it burns as a result, this counts
/// as a Ⓓ action burning an ally (eg. Trophies, Predator's
/// Transformation)": the prey's 2-life ally burned, Predator's
/// Transformation [ABO] after resolution: the prey burns 2 pool.
#[test]
fn horseshoes_burning_a_prey_ally_feeds_predators_transformation() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("POT", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["abo", "abo"];
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Carlton Van Wyk"));
    hand(&mut e, 0, &["Horseshoes", "Predator's Transformation"]);
    pick(&mut e, act(0, 1, a, Target::Minion(ally)));
    // Horseshoes not replaced (no library): the next card is first.
    pick_when(&mut e, act(0, 3, a, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[ally.index()].in_play, "burned");
    assert_eq!(pool(&e)[1], 8);
}

/// Little Mountain Cemetery: "+1 stealth action. This acting Samedi and all
/// younger Samedi in your uncontrolled region each gain a blood from the
/// bank." The acting Samedi gains 1; in the uncontrolled region a younger
/// Samedi gains 1, an older Samedi and a younger non-Samedi none.
#[test]
fn little_mountain_cemetery_feeds_younger_uncontrolled_samedi() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| c.clan == "Samedi" && c.capacity == 6, 0),
        2,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let young = textless(|c| c.clan == "Samedi" && c.capacity < 6, 0);
    // Uncontrolled, his text cannot apply.
    let old = find_card("Jean Lisle");
    assert!(CRYPT[old as usize].capacity > 6);
    let other = textless(|c| c.clan == "Ventrue" && c.capacity < 6, 0);
    e.state.seats[0].uncontrolled = vec![slot(young, 0), slot(old, 0), slot(other, 0)];
    hand(&mut e, 0, &["Little Mountain Cemetery"]);
    pick(&mut e, act(0, 0, a, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Successful));
    assert_eq!(blood(&e, a), 3);
    let counters: Vec<u8> = e.state.seats[0]
        .uncontrolled
        .iter()
        .map(|s| s.counters)
        .collect();
    assert_eq!(counters, vec![1, 0, 0]);
}

/// Off Kilter: "+1 stealth action. Gain 1 pool. If you do not have the
/// Edge, you get the Edge. Otherwise, you can burn the Edge to gain 1
/// additional pool." Without the Edge: 1 pool and the Edge. With it: 1
/// pool, then the Edge burned for 1 more, or kept.
#[test]
fn off_kilter_gets_or_trades_the_edge() {
    for (edge, burn) in [(false, false), (true, true), (true, false)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), of_clan("Samedi", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.seats[0].has_edge = edge;
        hand(&mut e, 0, &["Off Kilter"]);
        pick(&mut e, act(0, 0, a, Target::None));
        if edge {
            let req = offering(&mut e, Choice::ResolvePrinted { minion: a });
            assert_eq!(e.state.seats[0].pool, 11, "the first pool first");
            let i = if burn {
                index_of(&req, Choice::ResolvePrinted { minion: a })
            } else {
                index_of(&req, Choice::Pass)
            };
            e.choose(i);
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let want = if burn { (12, false) } else { (11, true) };
        assert_eq!(
            (e.state.seats[0].pool, e.state.seats[0].has_edge),
            want,
            "edge {edge} burn {burn}"
        );
    }
}

/// [LSJ 20100207] "Off Kilter's additional pool cannot be gained if you do
/// not have a VP (edge is burned before pool is gained)." Under The Rising
/// with the Edge and no victory point: the first pool kept, the Edge burned,
/// the second pool to the bank.
#[test]
fn off_kilter_under_the_rising_loses_the_extra_pool() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Samedi", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    rising(&mut e);
    e.state.seats[0].has_edge = true;
    hand(&mut e, 0, &["Off Kilter"]);
    pick(&mut e, act(0, 0, a, Target::None));
    pick_when(&mut e, Choice::ResolvePrinted { minion: a });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.seats[0].has_edge);
    assert_eq!(e.state.seats[0].pool, 11);
}

/// Visionquest: "Put this card on this Ahrimane. This Ahrimane gets +1
/// level of Spiritus [spi]. If this action is blocked, this Ahrimane does
/// not unlock as normal during their next unlock phase." Unblocked: the
/// card itself on her, +1 Spiritus. Blocked: not unlocked in her next unlock
/// phase.
#[test]
fn visionquest_grants_spirit_or_skips_the_unlock() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), ahrimane(), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let before = e.state.discipline_level(a, "spi");
    hand(&mut e, 0, &["Visionquest"]);
    pick(&mut e, act(0, 0, a, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let c = card_named(&e, "Visionquest");
    assert_eq!(e.state.cards[c.index()].host, Some(a));
    assert!(!e.state.cards[c.index()].effect_only);
    assert!(!e.state.seats[0].ash.contains(&find_library("Visionquest")));
    assert_eq!(e.state.discipline_level(a, "spi"), before + 1);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].locked, "unblocked: unlocks");

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Ahrimane", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    interceptor(&mut e, z);
    hand(&mut e, 0, &["Visionquest"]);
    pick(&mut e, act(0, 0, a, Target::None));
    pick_when(&mut e, block(z));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.minions[a.index()].locked);
    assert!(e.state.minions[a.index()].skip_unlock);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        e.state.minions[a.index()].locked,
        "blocked: no unlock as normal"
    );
}

/// +1 intercept for `z` from a card on it, to block a +1 stealth action.
fn interceptor(e: &mut Engine, z: MinionId) {
    let seat = e.state.minions[z.index()].owner.0;
    on_with(
        e,
        seat,
        "Thaumaturgy",
        z,
        &[zvtes::cards::Modifier::Bearer {
            stat: Stat::Intercept,
            n: 1,
            during: &[],
            who: zvtes::cards::Bearer::Any,
        }],
    );
}

/// An Ahrimane with basic Spiritus, no printed text.
fn ahrimane() -> u16 {
    textless(
        |c| c.clan == "Ahrimane" && c.disciplines.contains(&"spi"),
        0,
    )
}

/// [LSJ 20050114] "The 'does not unlock as normal' effect is redundant with
/// being infernal. If the minion is infernal, his controller can still pay
/// a pool to unlock him." A blocked infernal Ahrimane: its controller pays
/// 1 pool in the unlock phase to unlock it.
#[test]
fn visionquest_blocked_infernal_still_pays_to_unlock() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Ahrimane", 0), 4, false);
    e.state.minions[a.index()].made_infernal = true;
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    interceptor(&mut e, z);
    hand(&mut e, 0, &["Visionquest"]);
    pick(&mut e, act(0, 0, a, Target::None));
    pick_when(&mut e, block(z));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick_when(&mut e, Choice::UnlockInfernal { minion: a });
    assert!(!e.state.minions[a.index()].locked);
    assert_eq!(e.state.seats[0].pool, 9);
}

/// New Moon Sigil [SPI]: "Put this card on a vampire you control. The
/// attached vampire gets 1 optional maneuver each combat. A vampire can
/// have only one New Moon Sigil. … during an action, the attached vampire
/// can burn 2 blood to get +1 stealth." Another's vampire is no target; on
/// the other vampire, a second Sigil not offered; its bleed buys +1 stealth
/// for 2 blood once, not twice.
#[test]
fn new_moon_sigil_superior_buys_stealth_once_an_action() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("SPI", 0), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 6, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[z.index()].gained_levels = vec!["aus", "aus"];
    hand(&mut e, 0, &["New Moon Sigil", "New Moon Sigil"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&act(0, 1, a, Target::Minion(z))));
    pick(&mut e, act(0, 1, a, Target::Minion(b)));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let c = card_named(&e, "New Moon Sigil");
    assert_eq!(e.state.cards[c.index()].host, Some(b));
    assert!(!e.state.cards[c.index()].effect_only);
    assert!(
        !req.options.contains(&act(0, 1, a, Target::Minion(b))),
        "only one"
    );
    let before = blood(&e, b);
    pick(&mut e, bleed(b));
    pick_when(&mut e, block(z));
    pick_when(&mut e, ability(c.0, 1));
    assert_eq!(blood(&e, b), before - 2);
    assert_eq!(e.action().expect("bleed").stealth(&e.state), 1);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, ability(c.0, 1));
}

/// A Harbinger of Skulls: Erebus, his cheaper wraiths inert here (every
/// Harbinger has printed text since round 28).
fn harbinger() -> u16 {
    find_card("Erebus")
}

/// Threading the Path of Orpheus: "+1 stealth action. Burn a card played by
/// another Methuselah on a vampire you control. Lock that vampire." The
/// prey's card on the actor's other vampire burned and that vampire locked;
/// a card of the player's own on it is no target.
#[test]
fn threading_the_path_burns_a_foreign_card_and_locks_its_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), harbinger(), 4, false);
    let v = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let theirs = in_play(&mut e, 1, "Agent of Power", Some(v));
    let mine = in_play(&mut e, 0, "Thaumaturgy", Some(v));
    hand(&mut e, 0, &["Threading the Path of Orpheus"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&act(0, 0, a, Target::Card(mine))));
    pick(&mut e, act(0, 0, a, Target::Card(theirs)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[theirs.index()].present(), "burned");
    assert!(e.state.cards[mine.index()].present());
    assert!(e.state.minions[v.index()].locked, "that vampire locked");
}

/// Warding the Beast [val]: "+1 stealth action. Rescue a non-infernal
/// vampire from torpor." [ANK 20181017] "Is not the default action, no blood
/// is paid for the rescue." [LSJ 20011204-3] [PIB 20130704] played with
/// [val], its [val] text: the torpid vampire out with no blood paid, an
/// infernal one no target.
#[test]
fn warding_the_beast_valeren_rescues_without_blood() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Gangrel", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["val"];
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let t = e.add_minion(SeatId(0), plain("Brujah", 1), 1, true);
    let hell = infernal(&mut e, 0, 1, true);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Warding the Beast"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&act(0, 1, a, Target::Minion(hell))));
    pick(&mut e, act(0, 1, a, Target::Minion(t)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[t.index()].in_torpor);
    assert_eq!((blood(&e, a), blood(&e, t)), (4, 1), "no blood paid");
}

/// Warding the Beast [pre]: "+1 stealth action. Unlock a younger
/// non-infernal vampire." The prey's younger locked vampire unlocked; an
/// older one is no target.
#[test]
fn warding_the_beast_presence_unlocks_a_younger_vampire() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| c.capacity == 6 && c.disciplines.contains(&"pre"), 0),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let young = e.add_minion(SeatId(1), textless(|c| c.capacity < 6, 0), 3, false);
    let old = e.add_minion(SeatId(1), textless(|c| c.capacity > 6, 0), 4, false);
    e.state.minions[young.index()].locked = true;
    hand(&mut e, 0, &["Warding the Beast"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&act(0, 0, a, Target::Minion(old))));
    pick(&mut e, act(0, 0, a, Target::Minion(young)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[young.index()].locked);
}

/// [ANK 20210124] [LSJ 20070411] [LSJ 20090514] "If the action is unblocked
/// when it resolves and the target is no longer in torpor, the action
/// 'fizzles' (cost, if any, is paid, and it is considered successful, but
/// has no effect)." Warding the Beast [val]'s target out of torpor before
/// it resolves: fizzled.
#[test]
fn warding_the_beast_fizzles_once_its_target_left_torpor() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("val", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let t = e.add_minion(SeatId(0), plain("Brujah", 1), 1, true);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Warding the Beast"]);
    pick(&mut e, act(0, 1, a, Target::Minion(t)));
    e.state.minions[t.index()].in_torpor = false;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Fizzled));
}

/// Warding the Beast [VAL]: "Ⓓ Diablerize a vampire in torpor. Vampires
/// cannot cast votes or ballots in favor of the blood hunt unless they are
/// Baali, Tremere or Tremere antitribu." In the blood hunt, a titled
/// Ventrue may cast against, not in favor; a titled Tremere in favor.
#[test]
fn warding_the_beast_superior_blood_hunt_only_tremere_vote_for() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), vampire_with("VAL", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let victim = e.add_minion(SeatId(1), plain("Gangrel", 0), 0, true);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[v.index()].granted_title = Some(Title::Votes(1));
    let t = e.add_minion(SeatId(2), plain("Tremere", 0), 4, false);
    e.state.minions[t.index()].granted_title = Some(Title::Votes(1));
    hand(&mut e, 0, &["Warding the Beast"]);
    pick(&mut e, act(0, 2, a, Target::Minion(victim)));
    let req = until(&mut e, SeatId(1), WindowLabel::BloodHuntPolling);
    assert!(req.options.contains(&votes(v, false)));
    assert!(!req.options.contains(&votes(v, true)), "a Ventrue");
    e.choose(index_of(&req, Choice::Pass));
    let req = until(&mut e, SeatId(2), WindowLabel::BloodHuntPolling);
    assert!(req.options.contains(&votes(t, true)), "a Tremere");
}

/// Lord of Serenity [for]: "+1 stealth action. Rescue up to two vampires
/// from torpor. Vampires can ignore the normal prey, predator or target
/// restrictions for blocking this action." [ANK 20181017]: no blood paid
/// for the rescues, the card's 3 blood only. [LSJ 20011204-3] [PIB
/// 20130704]: played with [for], its [for] text. Its own and the prey's
/// torpid vampires out; a vampire of a Methuselah neither its prey nor its
/// predator may attempt to block it, that Methuselah's ally not.
#[test]
fn lord_of_serenity_rescues_two_without_blood_and_opens_the_block() {
    let mut e = game(4);
    let a = e.add_minion(SeatId(0), vampire_with("for", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let own = e.add_minion(SeatId(0), plain("Brujah", 1), 1, true);
    let theirs = e.add_minion(SeatId(1), plain("Ventrue", 0), 1, true);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let far = e.add_minion(SeatId(2), plain("Gangrel", 0), 4, false);
    let ally = e.add_ally(SeatId(2), find_library("Carlton Van Wyk"));
    e.add_minion(SeatId(3), plain("Toreador", 0), 4, false);
    hand(&mut e, 0, &["Lord of Serenity"]);
    pick(&mut e, act(0, 0, a, Target::Minion(own)));
    pick(&mut e, Choice::ChooseMinion(theirs));
    let req = until(&mut e, SeatId(2), WindowLabel::ActionBlock);
    assert!(req.options.contains(&block(far)));
    assert!(!req.options.contains(&block(ally)), "an ally");
    e.choose(index_of(&req, Choice::Pass));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Successful));
    assert!(!e.state.minions[own.index()].in_torpor);
    assert!(!e.state.minions[theirs.index()].in_torpor);
    assert_eq!(blood(&e, a), 3, "the card's 3 blood only");
    assert_eq!((blood(&e, own), blood(&e, theirs)), (1, 1));
}

/// [ANK 20210124] [LSJ 20070411] [LSJ 20090514] "If the action is unblocked
/// when it resolves and the target is no longer in torpor, the action
/// 'fizzles'." Lord of Serenity: one target out of torpor, the other still
/// rescued; both out, the action fizzles.
#[test]
fn lord_of_serenity_fizzles_once_its_targets_left_torpor() {
    for both in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("for", 0), 6, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let x = e.add_minion(SeatId(0), plain("Brujah", 1), 1, true);
        let y = e.add_minion(SeatId(0), plain("Brujah", 2), 1, true);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Lord of Serenity"]);
        pick(&mut e, act(0, 0, a, Target::Minion(x)));
        pick(&mut e, Choice::ChooseMinion(y));
        e.state.minions[x.index()].in_torpor = false;
        if both {
            e.state.minions[y.index()].in_torpor = false;
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        if both {
            assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Fizzled));
        } else {
            assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Successful));
            assert!(!e.state.minions[y.index()].in_torpor, "the other rescued");
        }
    }
}

/// Lord of Serenity [OBE]: "Put this card on this vampire. During your
/// unlock phase, this vampire can gain 2 blood. Vampires can burn this card
/// as a Ⓓ action." Its next unlock phase: +2 blood, once; the prey's
/// vampire burns it as a directed action.
#[test]
fn lord_of_serenity_obfuscate_feeds_its_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 1), 6, false);
    e.state.minions[a.index()].gained_levels = vec!["obe", "obe"];
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Lord of Serenity"]);
    pick(&mut e, act(0, 2, a, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let c = card_named(&e, "Lord of Serenity");
    assert_eq!(e.state.cards[c.index()].host, Some(a));
    assert!(!e.state.cards[c.index()].effect_only);
    assert_eq!(blood(&e, a), 3);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick_when(&mut e, ability(c.0, 1));
    assert_eq!(blood(&e, a), 5);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, ability(c.0, 1));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, used_by(c, 2, z));
    assert_eq!(
        e.action().expect("burn").directed_seat(&e.state),
        Some(SeatId(0))
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.cards[c.index()].present(), "burned");
}

/// Scarlet Lore [nec]: "Move a minion card from your ash heap to the top of
/// your library." [LSJ 20010627] "The card to retrieve must be announced
/// when declaring the action." [LSJ 20011204-3] [PIB 20130704]: played with
/// [nec], its [nec] text. The ally named as the action is declared (a
/// combat card offered too, a master card not) lands on top of the library.
#[test]
fn scarlet_lore_names_its_ash_heap_minion_at_declaration() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("nec", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let ally = find_library("Carlton Van Wyk");
    let master = find_library("Thaumaturgy");
    e.state.seats[0].ash.push(ally);
    e.state.seats[0].ash.push(master);
    e.state.seats[0].ash.push(find_library("Form of Mist"));
    hand(&mut e, 0, &["Scarlet Lore"]);
    fill_hand(&mut e, 0);
    e.state.seats[0].library = vec![find_library("Soak"); 3].into();
    pick(&mut e, act(0, 0, a, Target::None));
    let req = request(&mut e);
    assert!(req.options.contains(&Choice::FindCard(ally)), "{req:?}");
    assert!(!req.options.contains(&Choice::FindCard(master)));
    pick(&mut e, Choice::FindCard(ally));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].library.last(), Some(&ally));
    assert!(!e.state.seats[0].ash.contains(&ally));
}

/// Scarlet Lore [MYT]: "Search your library for a minion card (shuffle
/// afterward), reveal it, and move it to the top of your library … with an
/// additional +1 stealth." The ally found ends on top; +2 stealth.
#[test]
fn scarlet_lore_mythherceria_tops_a_searched_minion_card() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("MYT", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let ally = find_library("Carlton Van Wyk");
    hand(&mut e, 0, &["Scarlet Lore"]);
    fill_hand(&mut e, 0);
    let mut lib = vec![find_library("Soak"); 6];
    lib.insert(2, ally);
    e.state.seats[0].library = lib.into();
    pick(&mut e, act(0, 2, a, Target::None));
    request(&mut e);
    assert_eq!(e.action().expect("action").stealth(&e.state), 2);
    let req = offering(&mut e, Choice::FindCard(ally));
    assert!(!req
        .options
        .contains(&Choice::FindCard(find_library("Thaumaturgy"))));
    pick(&mut e, Choice::FindCard(ally));
    request(&mut e);
    assert_eq!(e.state.seats[0].library.last(), Some(&ally));
}

/// Shadowed Eyes [aus][obt] (a combo): "Put this card on an ally or
/// younger vampire. The
/// attached minion gets -2 intercept and cannot perform directed actions or
/// use ranged strikes." On the prey's younger vampire with a gun: -2
/// intercept, no bleed offered in its turn, its gun's strike not offered in
/// combat (offered without the card); an older vampire no target.
#[test]
fn shadowed_eyes_blinds_its_bearer() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| c.capacity == 6 && c.disciplines.contains(&"aus"), 0),
        4,
        false,
    );
    e.state.minions[a.index()].gained_levels = vec!["obt"];
    let p = e.add_minion(SeatId(0), vampire_with("pot", 0), 4, false);
    let cap = e.state.capacity_of(a).expect("a vampire");
    let t = e.add_minion(SeatId(1), textless(|c| c.capacity < cap, 0), 4, false);
    let old = e.add_minion(SeatId(1), textless(|c| c.capacity > cap, 0), 4, false);
    let gun = in_play(&mut e, 1, ".44 Magnum", Some(t));
    hand(&mut e, 0, &["Shadowed Eyes", "Bum's Rush"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&act(0, 0, a, Target::Minion(old))));
    pick(&mut e, act(0, 0, a, Target::Minion(t)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let c = card_named(&e, "Shadowed Eyes");
    assert_eq!(e.state.cards[c.index()].host, Some(t));
    assert!(!e.state.cards[c.index()].effect_only);
    assert_eq!(
        e.state.bearer_mod(t, Stat::Intercept, ActionKind::Bleed),
        -2
    );
    pick(&mut e, act(0, 0, p, Target::Minion(t)));
    never_offered_where(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), .. } if *c == gun),
    );
    let req = request(&mut e);
    assert!(!req.options.contains(&bleed(t)), "no directed action");
    assert!(req.options.contains(&bleed(old)));

    // Without it, the gun is offered in that combat.
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), vampire_with("pot", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let gun = in_play(&mut e, 1, ".44 Magnum", Some(t));
    hand(&mut e, 0, &["Bum's Rush"]);
    pick(&mut e, act(0, 0, p, Target::Minion(t)));
    offering(&mut e, ability(gun.0, 1));
}

/// Shadowed Eyes [AUS]: "… Any minion can burn this card as a +1 stealth
/// action. … the action to burn this card costs 1 additional blood or
/// life." Its own bearer burns it, for 1 blood, as an undirected action
/// (the card its controller's); the basic version's free burn not offered.
#[test]
fn shadowed_eyes_superior_burn_costs_more() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| c.capacity == 6 && c.disciplines.contains(&"AUS"), 0),
        4,
        false,
    );
    e.state.minions[a.index()].gained_levels = vec!["obt", "obt"];
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let cap = e.state.capacity_of(a).expect("a vampire");
    let t = e.add_minion(SeatId(1), textless(|c| c.capacity < cap, 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Shadowed Eyes"]);
    pick(&mut e, act(0, 1, a, Target::Minion(t)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let c = card_named(&e, "Shadowed Eyes");
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!req.options.contains(&used_by(c, 0, t)));
    let before = blood(&e, t);
    pick(&mut e, used_by(c, 1, t));
    assert_eq!(e.action().expect("burn").directed_seat(&e.state), None);
    assert_eq!(e.action().expect("burn").stealth(&e.state), 1);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.cards[c.index()].present());
    assert_eq!(blood(&e, t), before - 1, "1 blood more");
}

/// An infernal Baali of capacity 9 or more.
fn great_baali() -> u16 {
    find_card("Annazir")
}

/// Shatter the Gate: "Put this card on this Baali; if this Baali has three
/// copies of this card, burn these copies to give each other Methuselah who
/// has no shattered counters 1 shattered counter with 'During your unlock
/// phase, burn 3 pool'." The third copy: the three burned, the others
/// shattered, its own Methuselah not; each burns 3 pool in their unlock
/// phase.
#[test]
fn shatter_the_gate_third_copy_shatters_the_others() {
    let mut e = game(3);
    let b = e.add_minion(SeatId(0), great_baali(), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Gangrel", 0), 4, false);
    in_play(&mut e, 0, "Shatter the Gate", Some(b));
    in_play(&mut e, 0, "Shatter the Gate", Some(b));
    hand(&mut e, 0, &["Shatter the Gate"]);
    pick(&mut e, act(0, 0, b, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let card = find_library("Shatter the Gate");
    assert_eq!(
        e.state.seats[0].ash.iter().filter(|&&c| c == card).count(),
        3
    );
    let shattered: Vec<u8> = e.state.seats.iter().map(|s| s.shattered.0).collect();
    assert_eq!(shattered, vec![0, 3, 3]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(pool(&e), vec![10, 7, 10]);
}

/// [LSJ 20100206-2] "The counters stay active after the acting Methuselah is
/// ousted." Its Baali's Methuselah ousted by their predator's bleed: the
/// prey still burns 3 pool in its next unlock phase.
#[test]
fn shattered_counters_outlast_their_ousted_giver() {
    let mut e = game(3);
    let b = e.add_minion(SeatId(0), great_baali(), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let g = e.add_minion(SeatId(2), plain("Gangrel", 0), 4, false);
    e.add_minion(SeatId(2), plain("Gangrel", 1), 4, false);
    in_play(&mut e, 0, "Shatter the Gate", Some(b));
    in_play(&mut e, 0, "Shatter the Gate", Some(b));
    hand(&mut e, 0, &["Shatter the Gate"]);
    pick(&mut e, act(0, 0, b, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.state.seats[0].pool = 1;
    until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    pick(&mut e, bleed(g));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.seats[0].ousted);
    assert_eq!(e.state.seats[1].pool, 4, "3 again");
}

/// [LSJ 20080604] "The action provided by this card is considered to be an
/// action requiring the listed clan/sect/capacity/title." Shatter the
/// Gate's burn: "A vampire with capacity 6 or more can burn the Edge to burn
/// this card as a Ⓓ action." A 6-capacity vampire with the Edge takes it,
/// the Edge and the card burned; a smaller one, or one without the Edge,
/// not offered.
#[test]
fn shatter_the_gate_burn_action_needs_capacity_six_and_the_edge() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), great_baali(), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let big = e.add_minion(SeatId(1), textless(|c| c.capacity == 6, 0), 4, false);
    let small = e.add_minion(SeatId(1), textless(|c| c.capacity == 5, 0), 4, false);
    let c = in_play(&mut e, 0, "Shatter the Gate", Some(b));
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!req.options.contains(&used_by(c, 0, big)), "no Edge");
    e.state.seats[1].has_edge = true;
    e.state.seats[0].has_edge = false;
    let req = request(&mut e);
    assert!(!req.options.contains(&used_by(c, 0, small)), "capacity 5");
    pick(&mut e, used_by(c, 0, big));
    assert_eq!(
        e.action().expect("burn").directed_seat(&e.state),
        Some(SeatId(0))
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.cards[c.index()].present());
    assert!(!e.state.seats[1].has_edge, "the Edge burned");
}

/// [LSJ 20100208-2] [LSJ 20100823] "[mel], [MEL] Requires to have the
/// second in hand to play it." Shattering Crescendo [mel]: "Discard a
/// Shattering Crescendo to inflict 2 unpreventable damage on each of up to
/// two minions controlled by a single Methuselah." [LSJ 20011204-3] [PIB
/// 20130704]: played with [mel], its [mel] text. With one copy, not
/// offered; with two, the prey's two vampires take 2 each, the second copy
/// discarded; a third Methuselah's minion is no second pick.
#[test]
fn shattering_crescendo_needs_a_second_copy() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), find_card("Harlan Graves"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let y = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let w = e.add_minion(SeatId(2), plain("Gangrel", 0), 4, false);
    hand(&mut e, 0, &["Shattering Crescendo"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&act(0, 2, a, Target::Minion(x))));
    hand(&mut e, 0, &["Shattering Crescendo"]);
    pick(&mut e, act(0, 2, a, Target::Minion(x)));
    let req = request(&mut e);
    assert!(!req.options.contains(&Choice::ChooseMinion(w)));
    pick(&mut e, Choice::ChooseMinion(y));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, x), blood(&e, y), blood(&e, w)), (2, 2, 4));
    let card = find_library("Shattering Crescendo");
    assert_eq!(
        e.state.seats[0].ash.iter().filter(|&&c| c == card).count(),
        2
    );
    assert!(e.state.seats[0].hand.is_empty());
}

/// [LSJ 20031010] [LSJ 20090319] [ANK 20180907] "Choosing is targeting: …
/// Actions that choose the vampire cannot be played against him (eg. …
/// Shattering Crescendo)." The prey's vampire under Secure Haven is neither
/// the target nor the second pick.
#[test]
fn shattering_crescendo_cannot_choose_a_secure_haven_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Harlan Graves"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let y = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 2), 4, false);
    on_with(
        &mut e,
        1,
        "Secure Haven",
        y,
        &[zvtes::cards::Modifier::Haven],
    );
    hand(&mut e, 0, &["Shattering Crescendo", "Shattering Crescendo"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&act(0, 2, a, Target::Minion(y))));
    pick(&mut e, act(0, 2, a, Target::Minion(x)));
    let req = request(&mut e);
    assert!(!req.options.contains(&Choice::ChooseMinion(y)), "{req:?}");
}

/// Shattering Crescendo [pro]: "Ⓓ Inflict 1 unpreventable damage on an ally
/// or retainer." The prey's retainer loses its 1 life and burns.
#[test]
fn shattering_crescendo_protean_hits_a_retainer() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("pro", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let r = in_play(&mut e, 1, "Mr. Winthrop", Some(z));
    e.state.cards[r.index()].counters = 1;
    hand(&mut e, 0, &["Shattering Crescendo"]);
    pick(&mut e, act(0, 1, a, Target::Card(r)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[r.index()].present(), "burned");
}

/// A Tremere slave Gargoyle, capacity 4, printing only Flight and the
/// slave (Rusticus).
fn tremere_slave() -> u16 {
    find_card("Rusticus")
}

/// [ANK 20190203] "Only requires one clan or the other (Tremere or Tremere
/// antitribu) to be played." Soul of the Earth: "Put this card on a slave
/// Gargoyle. The Gargoyle with this card can burn it to reduce the cost of
/// recruiting or employing an ally or retainer that requires a Gargoyle by
/// up to 3 blood or pool." A Tremere puts it on its slave; the slave's
/// Razor Bat (2 pool) then costs nothing, the card burned.
#[test]
fn soul_of_the_earth_discounts_a_gargoyle_hire() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), of_clan("Tremere", 0), 4, false);
    let g = e.add_minion(SeatId(0), tremere_slave(), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Soul of the Earth", "Razor Bat"]);
    pick(&mut e, act(0, 0, t, Target::Minion(g)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let c = card_named(&e, "Soul of the Earth");
    assert_eq!(e.state.cards[c.index()].host, Some(g));
    assert!(!e.state.cards[c.index()].effect_only);
    pick(&mut e, act(0, 0, g, Target::None));
    pick_when(&mut e, Choice::ResolveRider { card: c.0 });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(card_named(&e, "Razor Bat").0 < u32::MAX);
    assert_eq!(e.state.seats[0].pool, 10, "2 pool off");
    assert!(!e.state.cards[c.index()].present(), "burned");
}

/// [ANK 20210928] "Reduction applies to allies or retainers recruited in a
/// non-normal way (eg. Piper)." The card's Gargoyle, an Anarch, employs a
/// Razor Bat through Piper: the cost reduced, the card burned. Another
/// Gargoyle of its controller's employing one may take it too (the red
/// reading: any hire of its controller's).
#[test]
fn soul_of_the_earth_discounts_a_piper_hire_and_anothers() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), tremere_slave(), 4, false);
    e.state.minions[g.index()].sect_override = Some(zvtes::crypt::Sect::Anarch);
    let other = e.add_minion(SeatId(0), of_clan("Gargoyle", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let c = on_with(
        &mut e,
        0,
        "Soul of the Earth",
        g,
        &[zvtes::cards::Modifier::CostSource(
            zvtes::cards::CostSource::HireRequiringClan {
                clan: "Gargoyle",
                n: 3,
            },
        )],
    );
    hand(&mut e, 0, &["Piper", "Razor Bat", "Razor Bat"]);
    e.at_master_phase();
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(g),
            target: Target::None,
        },
    );
    // The one card to name: elided.
    pick_when(&mut e, Choice::ResolveRider { card: c.0 });
    request(&mut e);
    assert_eq!(e.state.seats[0].pool, 10, "2 pool off");
    assert!(!e.state.cards[c.index()].present());

    let mut e = game(2);
    let g = e.add_minion(SeatId(0), tremere_slave(), 4, false);
    let other2 = e.add_minion(SeatId(0), of_clan("Gargoyle", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let c = on_with(
        &mut e,
        0,
        "Soul of the Earth",
        g,
        &[zvtes::cards::Modifier::CostSource(
            zvtes::cards::CostSource::HireRequiringClan {
                clan: "Gargoyle",
                n: 3,
            },
        )],
    );
    let _ = other;
    hand(&mut e, 0, &["Razor Bat"]);
    pick(&mut e, act(0, 0, other2, Target::None));
    let req = request(&mut e);
    assert!(
        req.options.contains(&Choice::ResolveRider { card: c.0 }),
        "another Gargoyle's hire"
    );
}

/// Soul of the Earth, two copies on one slave Gargoyle: each its own cost
/// source, both burned for one hire requiring a Gargoyle (rules-qa
/// 113.142): Rock Cat's 4 pool reduced by 3, then by the last 1.
#[test]
fn soul_of_the_earth_two_copies_pay_one_rock_cat() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), tremere_slave(), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let soul = [zvtes::cards::Modifier::CostSource(
        zvtes::cards::CostSource::HireRequiringClan {
            clan: "Gargoyle",
            n: 3,
        },
    )];
    let c1 = on_with(&mut e, 0, "Soul of the Earth", g, &soul);
    let c2 = on_with(&mut e, 0, "Soul of the Earth", g, &soul);
    hand(&mut e, 0, &["Rock Cat"]);
    pick(&mut e, act(0, 0, g, Target::None));
    pick_when(&mut e, Choice::ResolveRider { card: c1.0 });
    pick_when(&mut e, Choice::ResolveRider { card: c2.0 });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    card_named(&e, "Rock Cat");
    assert_eq!(e.state.seats[0].pool, 10, "4 pool off");
    assert!(!e.state.cards[c1.index()].present());
    assert!(!e.state.cards[c2.index()].present());
}

/// Minor Curse [mal]: "Put this card on any minion. This minion's
/// controller doesn't replace cards played by this minion until the end of
/// the action." On the prey's vampire: the action card it plays in its turn
/// is not replaced until that action ends.
#[test]
fn minor_curse_holds_the_cursed_minions_replacements() {
    for superior in [false, true] {
        let mut e = game(2);
        let a = infernal(&mut e, 0, 4, false);
        maleficia(&mut e, 0, a, if superior { 2 } else { 1 });
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), vampire_with("dom", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Minor Curse"]);
        pick(&mut e, act(0, u8::from(superior), a, Target::Minion(z)));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let c = card_named(&e, "Minor Curse");
        assert_eq!(e.state.cards[c.index()].host, Some(z));
        assert!(!e.state.cards[c.index()].effect_only);
        e.state.seats[1].library = vec![find_library("Soak"); 5].into();
        hand(&mut e, 1, &["Govern the Unaligned"]);
        fill_hand(&mut e, 1);
        let full = e.state.seats[1].hand.len();
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        let i = e.state.seats[1]
            .hand
            .iter()
            .position(|&k| k == find_library("Govern the Unaligned"))
            .expect("in hand") as u8;
        let req = request(&mut e);
        let play = plays(&req)
            .into_iter()
            .find(
                |o| matches!(o, Choice::Play { card, actor: Some(m), .. } if *card == i && *m == z),
            )
            .expect("Govern the Unaligned");
        pick(&mut e, play);
        request(&mut e);
        assert_eq!(e.state.seats[1].hand.len(), full - 1, "held");
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        let after = e.state.seats[1].hand.len();
        if superior {
            assert_eq!(after, full - 1, "until the discard phase");
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(e.state.seats[1].hand.len(), full);
        } else {
            assert_eq!(after, full, "at the end of the action");
        }
    }
}

/// A Gargoyle slave of `clan`: Rusticus (Flight and the slave only), or
/// Fustuk (an ability these tests never use).
fn slave_of(clan: &str) -> u16 {
    find_card(match clan {
        "Tremere" => "Rusticus",
        _ => "Fustuk",
    })
}

/// [ANK 20190203] "Only requires one clan or the other (Tremere or Tremere
/// antitribu) to be played." Potio Martyrium: "Put this card on a Gargoyle
/// enslaved to this vampire's clan. If this Gargoyle is burned or sent to
/// torpor in combat, he or she burns and inflicts 3 aggravated damage on
/// the opposing minion and each retainer in combat." A Tremere antitribu
/// puts it on its slave (a Tremere's slave is no target); the Gargoyle,
/// emptied, sent to torpor by its blocker's hand strike, burns; the
/// blocker takes 3 aggravated, its 3-life retainer burns.
#[test]
fn potio_martyrium_gargoyle_explodes_when_downed() {
    let mut e = game(2);
    let ta = e.add_minion(SeatId(0), of_clan("Tremere antitribu", 0), 4, false);
    let g = e.add_minion(SeatId(0), slave_of("Tremere antitribu"), 1, false);
    let other = e.add_minion(SeatId(0), slave_of("Tremere"), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let r = in_play(&mut e, 1, "Mr. Winthrop", Some(z));
    e.state.cards[r.index()].counters = 3;
    hand(&mut e, 0, &["Potio Martyrium"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&act(0, 0, ta, Target::Minion(other))));
    pick(&mut e, act(0, 0, ta, Target::Minion(g)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let c = card_named(&e, "Potio Martyrium");
    assert_eq!(e.state.cards[c.index()].host, Some(g));
    assert!(!e.state.cards[c.index()].effect_only);
    pick(&mut e, bleed(g));
    e.state.minions[g.index()].blood = 0;
    pick_when(&mut e, block(z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[g.index()].in_play, "burned, not in torpor");
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Potio Martyrium")));
    assert!(!e.state.cards[r.index()].present(), "the retainer burned");
    assert!(
        e.state.minions[z.index()].in_torpor || !e.state.minions[z.index()].in_play,
        "3 aggravated"
    );
}

/// [LSJ 20100205] "The damage is done during combat, the opponent can use
/// combat cards to prevent it." [ANK 20181024] "The damage does not depend
/// on range." At long range, the blocker's gun sends the Gargoyle to
/// torpor: it burns, and the blocker prevents 1 of the 3 aggravated with
/// Indomitability.
#[test]
fn potio_martyrium_damage_can_be_prevented_at_long_range() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), of_clan("Tremere", 0), 4, false);
    let g = e.add_minion(SeatId(0), slave_of("Tremere"), 1, false);
    let z = e.add_minion(SeatId(1), vampire_with("for", 0), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let gun = in_play(&mut e, 1, ".44 Magnum", Some(z));
    let c = on_with(
        &mut e,
        0,
        "Potio Martyrium",
        g,
        &[zvtes::cards::Modifier::HostMartyrdom(3)],
    );
    let _ = t;
    hand(&mut e, 1, &["Indomitability"]);
    pick(&mut e, bleed(g));
    e.state.minions[g.index()].blood = 0;
    pick_when(&mut e, block(z));
    // The gun's maneuver commits its strike (LSJ 20090528-2-2).
    pick_when(&mut e, ability(gun.0, 1));
    let before = blood(&e, z);
    pick_when(&mut e, play(0, 0, z));
    assert!(!e.state.minions[g.index()].in_play);
    assert!(!e.state.cards[c.index()].present());
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[z.index()];
    assert!(v.in_torpor, "2 aggravated still land");
    assert_eq!(blood(&e, z), before - 1, "1 prevented: 1 blood burned");
}

/// A Thanatosis vampire raising the prey's ally `name`, burned from play:
/// the game, the actor and the ally's card (every card numbered, so a
/// discarded copy is not taken for a burned one).
fn pressing_flesh_game(players: usize, from: u8, name: &str, superior: bool) -> (Engine, MinionId) {
    let mut e = game(players);
    let disc = if superior { "THN" } else { "thn" };
    let a = e.add_minion(SeatId(0), vampire_with(disc, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    for s in 1..players as u8 {
        e.add_minion(SeatId(s), plain("Ventrue", s as usize), 4, false);
    }
    let ally = e.add_ally(SeatId(from), find_library(name));
    hand(&mut e, 0, &["Pressing Flesh"]);
    zvtes::copies::number_unnumbered(&mut e);
    e.burn(ally);
    (e, a)
}

/// The zombie Pressing Flesh raised: the minion its card is on.
fn zombie(e: &Engine) -> MinionId {
    let c = card_named(e, "Pressing Flesh");
    e.state.cards[c.index()].host.expect("on the zombie")
}

/// Pressing Flesh [THN]: "Move an ally in any ash heap who was burned from
/// play to your ready region with this card and life equal to its starting
/// life. This ally is a zombie instead of what it used to be, gets -1
/// bleed, cannot gain life … [THN] As above, but with 1 additional life."
/// [LSJ 20010627] [ANK 20180202] "Is an undirected action, even when
/// targeting another Methuselah's ash heap." [LSJ 20100204] [LSJ
/// 20040518-2] [LSJ 20100302-1] "If the effect puts/moves a minion into the
/// ready region, that minion can act this turn." The prey's burned mortal
/// (1 bleed, 1 life) comes to the player's ready region with this card on
/// it (not in an ash heap): a zombie, not a mortal, 0 bleed, 2 life, no
/// life gained, offered an action this turn; the action undirected; burned
/// again, its card to its owner's ash heap.
#[test]
fn pressing_flesh_raises_a_burned_ally_as_a_zombie() {
    let (mut e, a) = pressing_flesh_game(2, 1, "Political Ally", true);
    let card = find_library("Political Ally");
    pick(&mut e, act(0, 1, a, Target::None));
    request(&mut e);
    assert_eq!(e.action().expect("action").directed_seat(&e.state), None);
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let z = zombie(&e);
    let c = card_named(&e, "Pressing Flesh");
    assert!(!e.state.cards[c.index()].effect_only);
    assert!(!e.state.seats[0]
        .ash
        .contains(&find_library("Pressing Flesh")));
    assert!(!e.state.seats[1].ash.contains(&card));
    let v = &e.state.minions[z.index()];
    assert_eq!((v.owner, v.kind.ally_card()), (SeatId(0), Some(card)));
    assert!(e.state.has_subtype(z, zvtes::library::Label::Zombie));
    assert!(!e.state.has_subtype(z, zvtes::library::Label::Mortal));
    assert!(!e.state.is_mortal(z));
    assert_eq!(blood(&e, z), 2, "1 life and 1 more");
    assert_eq!(
        e.state.bearer_mod(z, Stat::Bleed, ActionKind::Bleed),
        -1,
        "-1 bleed"
    );
    assert!(e.state.cannot_gain_blood(z));
    assert!(req.options.contains(&bleed(z)), "acts this turn");
    e.burn(z);
    assert!(e.state.seats[1].ash.contains(&card), "its owner's");
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Pressing Flesh")));
}

/// [ANK 20200420-3] "Cannot be played if no target is available in any
/// Methuselah's ash heap." [LSJ 20100325-2] "A card burned while
/// representing something else … is not a valid target."
/// [PIB 20130128-2] [LSJ 20100325-2] Minions so put in play "do not count
/// has allies burned from play". Only a discarded ally and a burned Jake
/// Washington (a master card become an ally) in the ash heaps: Pressing
/// Flesh is not offered.
#[test]
fn pressing_flesh_needs_an_ally_burned_from_play() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("thn", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[1].ash.push(find_library("Political Ally"));
    hand(&mut e, 1, &["Jake Washington"]);
    hand(&mut e, 0, &["Pressing Flesh"]);
    zvtes::copies::number_unnumbered(&mut e);
    until(&mut e, SeatId(1), WindowLabel::MasterPhase);
    pick(&mut e, master_play(0));
    request(&mut e);
    let jake = body_of(&e, "Jake Washington");
    e.burn(jake);
    assert!(e.state.seats[1]
        .ash
        .contains(&find_library("Jake Washington")));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!plays(&req)
        .iter()
        .any(|o| matches!(o, Choice::Play { card: 0, actor: Some(m), .. } if *m == a)));
}

/// [LSJ 20100325-2] Only a card burned from play: the burned ally taken
/// back to hand, then discarded, is in the ash heap by a discard, and
/// Pressing Flesh is not offered.
#[test]
fn pressing_flesh_ignores_a_burned_ally_discarded_since() {
    let (mut e, a) = pressing_flesh_game(2, 1, "Political Ally", false);
    let seat = &mut e.state.seats[1];
    let (card, copy) = seat
        .ash
        .take_card(find_library("Political Ally"))
        .expect("burned");
    seat.hand.put(card, copy);
    let req = until(&mut e, SeatId(1), WindowLabel::DiscardPhase);
    assert!(req.options.contains(&Choice::Discard(0)));
    pick(&mut e, Choice::Discard(0));
    assert!(e.state.seats[1].ash.contains(&card), "discarded");
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!plays(&req)
        .iter()
        .any(|o| matches!(o, Choice::Play { card: 0, actor: Some(m), .. } if *m == a)));
}

/// [LSJ 19980303] "While a minion has an effect that allows them to play
/// cards that require Discipline(s), they do not actually have those
/// Discipline(s)." [LSJ 20011217] [LSJ 20090508] "Can be used to meet one
/// (but not more than one) of the normal Discipline requirements of a
/// multi-Discipline card." [ANK 20180111] [LSJ 20020218] nor stacked to a
/// superior or a second Discipline. [LSJ 20020503] [LSJ 20060126] "The
/// ally counts as 'independent' when playing cards as a vampire." The
/// zombie plays a basic Fortitude card (Indomitability [for]), not a
/// superior one, nor a mode of a multi-Discipline card needing Animalism
/// too (Ni Dieu ni Maître); it has no Fortitude; it plays as an
/// independent vampire.
#[test]
fn pressing_flesh_zombie_plays_basic_fortitude_as_a_vampire() {
    let (mut e, a) = pressing_flesh_game(2, 0, "Political Ally", false);
    pick(&mut e, act(0, 0, a, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let z = zombie(&e);
    assert!(!e.state.has_discipline(z, "for"));
    let indomitability = find_library("Indomitability");
    assert!(zvtes::cards::has_disciplines(
        &e.state,
        z,
        indomitability,
        0
    ));
    assert!(
        !zvtes::cards::has_disciplines(&e.state, z, indomitability, 1),
        "[FOR]"
    );
    // Ni Dieu ni Maître: its [for] mode, not its [ani][for] one.
    let ndnm = find_library("Ni Dieu ni Maître");
    assert!(zvtes::cards::has_disciplines(&e.state, z, ndnm, 1));
    assert!(!zvtes::cards::has_disciplines(&e.state, z, ndnm, 3));
    let g = zvtes::cards::plays_card_as_vampire(&e.state, z, indomitability, 0).expect("[for]");
    assert_eq!(g.sect, zvtes::crypt::Sect::Independent);
}

/// [ANK 20170309-2] "The enter play effect happens regardless of how it
/// comes into play (e.g, Pressing Flesh)." "An ally or retainer must be
/// burned when it comes into play, regardless of how it enters play." A
/// War Ghoul raised: its controller burns another ally of theirs.
#[test]
fn pressing_flesh_raised_war_ghoul_burns_an_ally() {
    let (mut e, a) = pressing_flesh_game(2, 1, "War Ghoul", false);
    let pa = e.add_ally(SeatId(0), find_library("Political Ally"));
    let pa_card = e.state.minions[pa.index()].ally_id().expect("a card");
    let _ = pa_card;
    pick(&mut e, act(0, 0, a, Target::None));
    // The one ally to burn: elided.
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[pa.index()].in_play, "burned on entry");
    assert!(e.state.minions[zombie(&e).index()].in_play);
}

/// [RBK equip] [RBK recruit-ally] [RBK employ-retainer] "Cards requiring a
/// discipline come in play at the inferior version." Underbridge Stray
/// ([ani] 1 life, [ANI] 2) raised by [thn]: its basic 1 life.
#[test]
fn pressing_flesh_raises_a_two_version_ally_at_basic() {
    let (mut e, a) = pressing_flesh_game(2, 1, "Underbridge Stray", false);
    pick(&mut e, act(0, 0, a, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let z = zombie(&e);
    assert_eq!(blood(&e, z), 1);
    let id = e.state.minions[z.index()].ally_id().expect("a card");
    assert_eq!(e.state.cards[id.index()].mode, 0);
}

/// [LSJ 20100212] Cry Wolf: "His ability continues to apply fully if he is
/// not a werewolf (eg. Pressing Flesh)." Raised, a zombie, not a werewolf:
/// he enters combat by his mandatory action and gets his press in it.
#[test]
fn pressing_flesh_raises_cry_wolf_who_keeps_his_ability() {
    let (mut e, a) = pressing_flesh_game(2, 1, "Cry Wolf", false);
    pick(&mut e, act(0, 0, a, Target::None));
    offering(&mut e, Choice::UsePress);
    let z = zombie(&e);
    assert!(!e.state.has_subtype(z, zvtes::library::Label::Werewolf));
    let c = e.combat().expect("his combat");
    assert!(
        c.sides.iter().any(|s| s.minion == z),
        "his mandatory combat"
    );
}

/// [LSJ 20071015] Reanimated Corpse: "If it is brought back with X being
/// zero (eg. Compel the Spirit), it has no pathos counter and burns
/// immediately." Raised by Pressing Flesh (X 0, LSJ 20100204) it is gone
/// as it enters: no zombie, nothing for Pressing Flesh's layers to hold,
/// the card to its player's ash heap (the fuzz found the engine panicking
/// on those layers).
#[test]
fn pressing_flesh_raising_a_reanimated_corpse_leaves_nothing_in_play() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("thn", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Reanimated Corpse", "Pressing Flesh"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, set_x(1));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let card = card_named(&e, "Reanimated Corpse");
    let corpse = e.state.cards[card.index()].body.expect("an ally");
    e.burn(corpse);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let pressing = find_library("Pressing Flesh");
    let k = e.state.seats[0].hand.iter().position(|&c| c == pressing);
    pick(&mut e, act(k.expect("in hand") as u8, 0, a, Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let raised = |e: &Engine| {
        e.state
            .minions
            .iter()
            .any(|m| m.in_play && m.kind.ally_card() == Some(find_library("Reanimated Corpse")))
    };
    assert!(!raised(&e), "gone as it entered");
    assert!(!e
        .state
        .cards
        .iter()
        .any(|c| c.in_play && c.card == pressing));
    assert!(e.state.seats[0].ash.contains(&pressing));
}

// ---- Combat cards -----------------------------------------------------------

/// Seat 0's `a` bleeds, seat 1's `z` blocks: the combat begins.
fn blocked(e: &mut Engine, a: MinionId, z: MinionId) {
    pick(e, bleed(a));
    pick_when(e, block(z));
}

/// Card `card` of the hand played by `actor` in `mode` at the card `on`.
fn at_card(card: u8, mode: u8, actor: MinionId, on: CardId) -> Choice {
    Choice::Play {
        card,
        mode,
        actor: Some(actor),
        target: Target::Card(on),
    }
}

/// [LSJ 20020211] Compress: "[thn], [THN] Not a hand strike." With
/// Immortal Grapple played ("strikes that are not hand strikes cannot be
/// used") only [pot]'s hand strike is offered; without it [thn]'s 2
/// aggravated damage sends the blocker to torpor. [LSJ 20011204-3] [PIB
/// 20130704] played with [thn], it counts as requiring Thanatosis only.
#[test]
fn compress_thanatosis_is_not_a_hand_strike() {
    for grapple in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("pot", 0), 6, false);
        e.state.minions[a.index()].gained_levels = vec!["thn"];
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Compress", "Immortal Grapple"]);
        blocked(&mut e, a, z);
        if grapple {
            pick_when(&mut e, play(1, 0, a));
        }
        let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
        assert!(req.options.contains(&play(0, 0, a)), "[pot]");
        assert_eq!(req.options.contains(&play(0, 1, a)), !grapple, "[thn]");
        if !grapple {
            pick(&mut e, play(0, 1, a));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert!(e.state.minions[z.index()].in_torpor, "2 aggravated");
            assert_eq!(blood(&e, a), 6 - 2 - 1, "2 blood, the blocker's hand");
        }
    }
}

/// [TOM 19960225] "Additional damage inherits all of the properties of the
/// base damage." Compress [pot] (+2), Blissful Agony [ani], Lead Fist
/// [tha], Loving Agony [ani] (+1) and Pounce (+2) by a vampire whose hand
/// strikes are aggravated this turn: aggravated damage, the blocker to
/// torpor (normal damage it would mend).
#[test]
fn hand_bonus_strikes_inherit_aggravation() {
    for (card, disc) in [
        ("Compress", "pot"),
        ("Blissful Agony", "ani"),
        ("Lead Fist", "tha"),
        ("Loving Agony", "ani"),
        ("Pounce", "FLIGHT"),
    ] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with(disc, 0), 6, false);
        e.state.minions[a.index()].timed.push((
            zvtes::cards::Lasting::AggravatedHands,
            zvtes::state::Expiry::EndOfTurn,
            SeatId(0),
        ));
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &[card]);
        blocked(&mut e, a, z);
        pick_when(&mut e, play(0, 0, a));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(e.state.minions[z.index()].in_torpor, "{card}: aggravated");
    }
}

/// Flames of the Netherworld [tha]: "Only usable at long range. Burn 1
/// blood to strike: 1R aggravated damage." [ANK 20210226] "The card cannot
/// be played if the minion cannot afford to burn the blood." Not offered at
/// close range; at long range (Flash), 1 blood burned and the blocker sent
/// to torpor; with no blood, not offered.
#[test]
fn flames_of_the_netherworld_burns_blood_at_long_range_only() {
    for blood_left in [3, 0] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("tha", 0), 4, false);
        e.state.minions[a.index()].gained_levels = vec!["cel"];
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Flames of the Netherworld", "Flash"]);
        blocked(&mut e, a, z);
        let mut close = e.clone();
        never_offered_until(
            &mut close,
            SeatId(0),
            WindowLabel::MinionPhase,
            play(0, 0, a),
        );
        e.state.minions[a.index()].blood = blood_left;
        pick_when(&mut e, play(1, 0, a));
        if blood_left == 0 {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
            continue;
        }
        let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
        assert_eq!(
            e.combat().expect("combat").range,
            zvtes::combat::Range::Long
        );
        assert!(req.options.contains(&play(0, 0, a)));
        pick(&mut e, play(0, 0, a));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, a), blood_left - 1);
        assert!(e.state.minions[z.index()].in_torpor, "1R aggravated");
    }
}

/// A Celerity master card on seat 1's `z`: its id.
fn celerity_on(e: &mut Engine, z: MinionId) -> CardId {
    on_with(
        e,
        1,
        "Celerity",
        z,
        &[zvtes::cards::Modifier::DisciplineLevel("cel")],
    )
}

/// Absorb the Mind [MYT]: "Strike: ranged. Steal 1 blood and steal 1
/// master: Discipline card from the opposing vampire (put it on this
/// striking vampire)." The Celerity card named with the strike moves to the
/// striker, its level with it; 1 blood stolen.
#[test]
fn absorb_the_mind_steals_blood_and_a_master_discipline_card() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("MYT", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let cel = celerity_on(&mut e, z);
    let level = e.state.discipline_level(a, "cel");
    assert!(level < 2);
    hand(&mut e, 0, &["Absorb the Mind"]);
    blocked(&mut e, a, z);
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert!(
        !req.options.contains(&play(0, 3, a)),
        "a card to take: the steal is not skipped"
    );
    pick(&mut e, at_card(0, 2, a, cel));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[cel.index()].host, Some(a));
    assert_eq!(e.state.discipline_level(a, "cel"), level + 1);
    assert_eq!(blood(&e, z), 4 - 1, "1 blood stolen, no damage");
}

/// [RTR 20010711] Absorb the Mind "can be played against a vampire with no
/// blood": [myt] offered against an empty blocker, which keeps its 0.
#[test]
fn absorb_the_mind_steals_from_a_vampire_with_no_blood() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("myt", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 1, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Absorb the Mind"]);
    blocked(&mut e, a, z);
    e.state.minions[z.index()].blood = 0;
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert!(req.options.contains(&play(0, 1, a)));
    pick(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 0);
}

/// [LSJ 20011205-2] "[MYT] cannot be used to steal a discipline that could
/// not be played on the stealing vampire": a striker with superior
/// Celerity cannot name the Celerity card; with nothing else to take,
/// [MYT] steals the blood alone.
#[test]
fn absorb_the_mind_cannot_take_a_discipline_already_superior() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("MYT", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["cel", "cel"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let cel = celerity_on(&mut e, z);
    hand(&mut e, 0, &["Absorb the Mind"]);
    blocked(&mut e, a, z);
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert!(!req.options.contains(&at_card(0, 2, a, cel)));
    assert!(req.options.contains(&play(0, 3, a)), "the steal alone");
    pick(&mut e, play(0, 3, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[cel.index()].host, Some(z));
    assert_eq!(blood(&e, z), 3);
}

/// [LSJ 20071112] Autonomic Mastery [DOM]: "Is a hand strike, and its
/// resolution is simultaneous with the opponent's strike … The opponent's
/// strike do damage." Against a younger vampire: no damage dealt, the
/// blocker's hand strike lands, and the combat ends (its Psyche! press
/// never offered); dodged (the blocker's Dodge), the combat goes on to the
/// press.
#[test]
fn autonomic_mastery_ends_combat_after_its_hand_strike_and_takes_the_damage() {
    for dodged in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("DOM", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let cap = e.state.capacity_of(a).expect("a vampire");
        let z = e.add_minion(
            SeatId(1),
            textless(|c| c.capacity < cap && c.disciplines.contains(&"cel"), 0),
            3,
            false,
        );
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Autonomic Mastery"]);
        hand(&mut e, 1, &["Psyche!", "Dodge"]);
        blocked(&mut e, a, z);
        pick_when(&mut e, play(0, 2, a));
        if dodged {
            pick_when(&mut e, play(1, 0, z));
            offering(&mut e, play(0, 0, z));
        } else {
            never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, play(0, 0, z));
            assert_eq!(blood(&e, z), 3, "no damage");
            assert_eq!(blood(&e, a), 3, "the blocker's hand strike");
        }
    }
}

/// Autonomic Mastery [DOM]: "Only usable in combat with a non-wraith ally
/// or a younger vampire": not offered against an older blocker.
#[test]
fn autonomic_mastery_superior_needs_a_younger_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("DOM", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let cap = e.state.capacity_of(a).expect("a vampire");
    let z = e.add_minion(SeatId(1), textless(|c| c.capacity > cap, 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Autonomic Mastery"]);
    blocked(&mut e, a, z);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 2, a));
}

/// Autonomic Mastery [dom]: "Strike: burn … a non-wraith retainer on the
/// opposing minion": the Raven Spy named with the strike burns; a Vengeful
/// Spirit (a wraith) is no target.
#[test]
fn autonomic_mastery_burns_a_non_wraith_retainer() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("dom", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let spy = in_play(&mut e, 1, "Raven Spy", Some(z));
    e.state.cards[spy.index()].counters = 1;
    let spirit = in_play(&mut e, 1, "Vengeful Spirit", Some(z));
    e.state.cards[spirit.index()].counters = 1;
    hand(&mut e, 0, &["Autonomic Mastery"]);
    blocked(&mut e, a, z);
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert!(!req.options.contains(&at_card(0, 1, a, spirit)), "a wraith");
    pick(&mut e, at_card(0, 1, a, spy));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[spy.index()].in_play, "burned");
    assert!(e.state.cards[spirit.index()].in_play);
}

/// The Black Beast: "Put this card on this vampire. In combat with a
/// Camarilla vampire, this vampire gets +1 strength and can burn this card
/// to get 1 press, only usable to continue combat." The card itself on the
/// Salubri antitribu, +1 strength against the Camarilla blocker (not
/// against a Sabbat vampire), its hand strike for 2; burned for the press,
/// a second round at strength 1.
#[test]
fn the_black_beast_goes_on_its_vampire_for_strength_and_a_press_against_camarilla() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| c.clan == "Salubri antitribu" && c.capacity >= 4, 0),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(
        SeatId(1),
        textless(|c| c.sect == Some(Sect::Camarilla) && c.capacity >= 7, 0),
        7,
        false,
    );
    let s = e.add_minion(
        SeatId(1),
        textless(|c| c.sect == Some(Sect::Sabbat) && c.capacity >= 4, 0),
        4,
        false,
    );
    hand(&mut e, 0, &["Black Beast, The"]);
    blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    let bb = card_named(&e, "Black Beast, The");
    assert_eq!(e.state.cards[bb.index()].host, Some(a));
    assert!(!e.state.cards[bb.index()].effect_only);
    assert!(!e.state.seats[0]
        .ash
        .contains(&find_library("Black Beast, The")));
    assert_eq!(e.state.strength_against(a, z), 1);
    assert_eq!(e.state.strength_against(a, s), 0, "not Camarilla");
    pick_when(&mut e, ability(bb.0, 0));
    assert!(!e.state.cards[bb.index()].present(), "burned for the press");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 7 - 2 - 1, "2 then 1");
    assert_eq!(blood(&e, a), 4 - 2, "two rounds of hand strikes");
}

/// A textless vampire with both Dominate and Presence at `level`
/// ("dom"/"DOM").
fn dom_pre(dom: &str, pre: &str) -> u16 {
    let (dom, pre) = (dom.to_string(), pre.to_string());
    textless(
        move |c| c.disciplines.contains(&dom.as_str()) && c.disciplines.contains(&pre.as_str()),
        0,
    )
}

/// [LSJ 20020416] Bliss [dom][pre] "Can be used during a combat by a
/// vampire in torpor." [ANK 20181101] "Can be played by a minion of a
/// Methuselah not involved in the current combat." [RBK cancel-a-card]
/// [ANK 20260216] "The cost for a canceled non-action-card is still
/// paid": a third Methuselah's torpid vampire cancels the blocker's Terror
/// Frenzy, which is paid for and in the ash heap, its restraint gone.
/// [LSJ 20011204-3] [PIB 20130704] the combo needs both Disciplines.
#[test]
fn bliss_cancels_a_frenzy_card_from_a_vampire_out_of_the_combat_in_torpor() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), vampire_with("pot", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(
        SeatId(1),
        textless(|c| c.disciplines.contains(&"ani") && c.capacity >= 4, 0),
        4,
        false,
    );
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let w = e.add_minion(SeatId(2), dom_pre("dom", "pre"), 2, true);
    e.add_minion(SeatId(2), plain("Toreador", 0), 4, false);
    hand(&mut e, 0, &["Slam"]);
    hand(&mut e, 1, &["Terror Frenzy"]);
    hand(&mut e, 2, &["Bliss"]);
    blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 0, z));
    pick_when(&mut e, play(0, 1, w));
    until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert!(e.state.seats[1]
        .ash
        .contains(&find_library("Terror Frenzy")));
    assert_eq!(blood(&e, z), 3, "its cost paid");
    assert!(
        !e.combat().expect("combat").sides[0].no_close_maneuver,
        "canceled"
    );
}

/// Bliss [dom][pre] as a reaction: "Cancel a frenzy card as it is played."
/// [ANK 20200813-2] any action card; [RBK cancel-a-card] the action is not
/// performed, the minion free to try again: Deep Song [ANI] canceled, its
/// vampire still unlocked, the target untouched.
#[test]
fn bliss_cancels_a_frenzy_card_as_a_reaction() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("ANI", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let w = e.add_minion(SeatId(1), dom_pre("dom", "pre"), 4, false);
    hand(&mut e, 0, &["Deep Song"]);
    hand(&mut e, 1, &["Bliss"]);
    pick(&mut e, act(0, 1, a, Target::Minion(z)));
    pick_when(&mut e, play(0, 2, w));
    to_minion_phase(&mut e, SeatId(0));
    assert!(e.state.seats[0].ash.contains(&find_library("Deep Song")));
    assert!(!e.state.minions[a.index()].locked, "no action performed");
    assert!(!e.state.minions[z.index()].locked);
    assert!(e.combat().is_none());
}

/// Bliss [DOM][PRE]: "Strike: combat ends." Neither combatant is hurt.
#[test]
fn bliss_superior_strikes_combat_ends() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), dom_pre("DOM", "PRE"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Bliss"]);
    blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 3, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, z)), (4, 4));
}

/// Seat `seat`'s infernal vampire with `n` Maleficia.
fn malefic(e: &mut Engine, seat: u8, n: usize) -> MinionId {
    let m = infernal(e, seat, 4, false);
    maleficia(e, seat, m, n);
    m
}

/// Evil Eye [mal]: "Cancel a strike card played by the opposing minion as
/// it is played (no cost is paid). The opposing minion cannot choose
/// another strike and gets no strike instead." "Do not replace until your
/// unlock phase." [ANK 20200629-2] [LSJ 20091208] [RTR 19951017] replaced
/// before the unlocks. The acting vampire's Slam canceled unpaid, no strike
/// from it; Evil Eye's player one card short until their unlock phase.
#[test]
fn evil_eye_cancels_a_strike_card_and_the_opponent_gets_no_strike() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("pot", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = malefic(&mut e, 1, 1);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Slam"]);
    hand(&mut e, 1, &["Evil Eye"]);
    let filler = find_library("Form of Mist");
    for _ in 0..10 {
        e.state.seats[1].library.push(filler);
    }
    let size = e.state.hand_size(SeatId(1));
    blocked(&mut e, a, z);
    assert_eq!(e.state.seats[1].hand.len(), size);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[0].ash.contains(&find_library("Slam")));
    assert_eq!(blood(&e, a), 4 - 1, "no Slam cost; the blocker's hand");
    assert_eq!(blood(&e, z), 4 - 1, "Evil Eye's cost, no damage");
    assert_eq!(e.state.seats[1].hand.len(), size - 1, "not replaced");
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].hand.len(), size, "replaced");
}

/// Evil Eye [MAL]: "Cancel an action card as it is played and lock the
/// acting minion." [ANK 20200813-2] a recruit too: ECTU Operative
/// canceled, no pool paid, the vampire locked ([RBK cancel-a-card]: the
/// action not performed, retried only once unlocked). R6: [mal] is not a
/// Discipline: under The Slow Withering (cards requiring a Discipline at
/// superior cost +1 blood) [MAL] still costs 1.
#[test]
fn evil_eye_superior_cancels_an_action_card_and_locks_the_actor() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = malefic(&mut e, 1, 2);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let w = in_play(&mut e, 0, "Slow Withering, The", None);
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::SuperiorCardsCost(1),
        source: w,
    });
    hand(&mut e, 0, &["ECTU Operative"]);
    hand(&mut e, 1, &["Evil Eye"]);
    pick(&mut e, act(0, 0, a, Target::None));
    pick_when(&mut e, play(0, 1, z));
    request(&mut e);
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("ECTU Operative")));
    assert_eq!(pool(&e)[0], 10, "the recruit unpaid");
    assert!(
        e.state.minions[a.index()].locked,
        "the acting minion locked"
    );
    assert_eq!(blood(&e, z), 4 - 1, "[mal] is no Discipline: no surcharge");
}

/// Darkness Within [obt]: "Put this card in play. Move 1 blood or life from
/// the opposing minion to this card … each round this combat during normal
/// strike resolution. After combat ends, move half the blood rounded up
/// from this card to this vampire and burn this card." The card itself in
/// play; two rounds (Psyche!'s press): 2 blood on it, 1 back to the
/// vampire, the card burned. [LSJ 20051123] taken before damage is
/// mended: the blocker's last blood goes to the card, not to mending.
#[test]
fn darkness_within_drains_each_round_and_returns_half_rounded_up() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| c.disciplines.contains(&"obt") && c.capacity >= 5, 0),
        5,
        false,
    );
    e.state.minions[a.index()].gained_levels = vec!["cel"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Darkness Within", "Psyche!"]);
    blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    let dw = card_named(&e, "Darkness Within");
    assert!(!e.state.cards[dw.index()].effect_only);
    assert!(!e.state.seats[0]
        .ash
        .contains(&find_library("Darkness Within")));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[dw.index()].present(), "burned");
    assert_eq!(blood(&e, z), 0, "2 drained, 2 mended");
    assert_eq!(blood(&e, a), 5 - 1 - 2 + 1, "cost, two hands, half of 2");
}

/// Darkness Within, then Psyche! [CEL] queuing a new combat as the first
/// ends: the after-combat return happens as the first combat ends, before
/// the new one starts (rules-qa 113.77). At the new combat's first round
/// (a second Darkness Within offered there): 1 blood back to the vampire,
/// the card burned.
#[test]
fn darkness_within_returns_before_psyches_new_combat() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| c.disciplines.contains(&"obt") && c.capacity >= 5, 0),
        5,
        false,
    );
    e.state.minions[a.index()].gained_levels = vec!["cel", "cel"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(
        &mut e,
        0,
        &["Darkness Within", "Psyche!", "Darkness Within"],
    );
    blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    let dw = card_named(&e, "Darkness Within");
    pick_when(&mut e, play(0, 1, a));
    offering(&mut e, play(0, 0, a));
    assert_eq!(e.combat().expect("the queued combat").round, 1);
    assert!(!e.state.cards[dw.index()].present(), "burned");
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Darkness Within")));
    assert_eq!(blood(&e, z), 4 - 1 - 1, "1 drained, 1 mended");
    assert_eq!(blood(&e, a), 5 - 1 - 1 + 1, "cost, a hand, half of 1");
}

/// [LSJ 20051123] Darkness Within: "It is not removed if a 'strike: combat
/// ends' resolves." The blocker's Flash Grenade ends the first round: no
/// blood moves, the card burns empty.
#[test]
fn darkness_within_drains_nothing_on_a_combat_ends_round() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obt", 0), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let g = in_play(&mut e, 1, "Flash Grenade", Some(z));
    hand(&mut e, 0, &["Darkness Within"]);
    blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, ability(g.0, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 4, "nothing drained");
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Darkness Within")));
}

/// Clockwerx [san]: "Only usable before range is determined. Move an
/// equipment from a ready Blood Brother of the same circle to this Blood
/// Brother in combat." The circle-mate's .44 Magnum moves to the
/// combatant, which strikes with it; a torpid mate's gun is no target.
#[test]
fn clockwerx_moves_a_circle_mates_equipment_to_the_combatant() {
    let mut e = game(2);
    // Torrance's three textless Blood Brothers.
    let a = e.add_minion(SeatId(0), find_card("Karl"), 4, false);
    let b = e.add_minion(SeatId(0), find_card("Ilse"), 4, false);
    let t = e.add_minion(SeatId(0), find_card("Truman"), 2, true);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let gun = in_play(&mut e, 0, ".44 Magnum", Some(b));
    let torpid = in_play(&mut e, 0, ".44 Magnum", Some(t));
    hand(&mut e, 0, &["Clockwerx"]);
    blocked(&mut e, a, z);
    let req = offering(&mut e, at_card(0, 0, a, gun));
    assert!(
        !req.options.contains(&at_card(0, 0, a, torpid)),
        "not ready"
    );
    pick(&mut e, at_card(0, 0, a, gun));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert_eq!(e.state.cards[gun.index()].host, Some(a));
    assert!(!uses(&req, gun.0).is_empty(), "its strike usable");
}

/// Clockwerx [SAN]: "As above, but usable any time during combat":
/// before strikes, for a circle-mate's .44 Magnum; another circle's
/// Blood Brother's gun is no target.
#[test]
fn clockwerx_superior_comes_later_and_keeps_to_the_circle() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Truman"), 4, false);
    let b = e.add_minion(SeatId(0), find_card("Ilse"), 4, false);
    let k = e.add_minion(SeatId(0), find_card("Kyrylo"), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let gun = in_play(&mut e, 0, ".44 Magnum", Some(b));
    let kiev = in_play(&mut e, 0, ".44 Magnum", Some(k));
    hand(&mut e, 0, &["Clockwerx"]);
    blocked(&mut e, a, z);
    let req = until(&mut e, SeatId(0), WindowLabel::CombatBeforeStrikes);
    assert!(req.options.contains(&at_card(0, 1, a, gun)));
    assert!(!req.options.contains(&at_card(0, 1, a, kiev)), "Kiev");
    pick(&mut e, at_card(0, 1, a, gun));
    request(&mut e);
    assert_eq!(e.state.cards[gun.index()].host, Some(a));
}

/// Clockwerx [SAN]: "a ready Blood Brother of the same circle" is any
/// Methuselah's, the opposing combatant included (rules-qa 113.68): the
/// blocking circle-mate's .44 Magnum moves to the actor and its
/// Methuselah, and the actor strikes with it.
#[test]
fn clockwerx_takes_the_opposing_circle_mates_equipment() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Truman"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), find_card("Ilse"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let gun = in_play(&mut e, 1, ".44 Magnum", Some(z));
    hand(&mut e, 0, &["Clockwerx"]);
    blocked(&mut e, a, z);
    let req = until(&mut e, SeatId(0), WindowLabel::CombatBeforeStrikes);
    assert!(req.options.contains(&at_card(0, 1, a, gun)));
    pick(&mut e, at_card(0, 1, a, gun));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert_eq!(e.state.cards[gun.index()].host, Some(a));
    assert_eq!(e.state.card_controller(gun), SeatId(0));
    assert!(!uses(&req, gun.0).is_empty(), "its strike usable");
}

/// [RTR 19970630] Blissful Agony [val]: "Damage is environmental."
/// [RTR 19941109] [TOM 19951216] [PIB 20140324] [ANK 20200422] "Damage is
/// inflicted even against a dodge": the blocker dodges the first round and
/// still takes 1; the second round (Psyche!) 1 again, the second copy not
/// playable at [val] ("only one … each combat").
#[test]
fn blissful_agony_val_damages_each_close_round_through_a_dodge() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Silas"), 5, false);
    e.state.minions[a.index()].gained_levels = vec!["cel"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Blissful Agony", "Blissful Agony", "Psyche!"]);
    hand(&mut e, 1, &["Dodge"]);
    blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 0, z));
    pick_when(&mut e, play(1, 0, a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, a));
    assert_eq!(blood(&e, z), 4 - 1 - 1 - 1, "1 dodged, 1, and a hand");
}

/// Blissful Agony [VAL]: "Strike: combat ends. Choose a minion controlled
/// by a Methuselah other than the opposing vampire's controller. The
/// opposing vampire enters combat with that minion." [LSJ 20011214-3]
/// [LSJ 20031212] "In the new combat, the opponent goes first": the blocker
/// against a third Methuselah's vampire, the blocker acting. [LSJ
/// 19971003] no other combat queued afterwards: Psyche! [CEL] not offered.
/// [LSJ 20051020] "If dodged, there is no new combat" (Psyche! then
/// offered). [RTR 20020501] "If the combat continues (e.g. Telepathic
/// Tracking), the card has no effect."
#[test]
fn blissful_agony_superior_hands_the_opposing_vampire_to_a_third_minion() {
    for case in ["new", "dodged", "continued"] {
        let mut e = game(3);
        let a = e.add_minion(SeatId(0), find_card("Langa"), 4, false);
        e.state.minions[a.index()].gained_levels = vec!["cel", "cel"];
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.minions[z.index()].gained_levels = vec!["aus", "aus"];
        let y = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let w = e.add_minion(SeatId(2), plain("Toreador", 0), 4, false);
        hand(&mut e, 0, &["Blissful Agony", "Psyche!"]);
        hand(&mut e, 1, &["Dodge", "Telepathic Tracking"]);
        blocked(&mut e, a, z);
        let strike = |t: MinionId| Choice::Play {
            card: 0,
            mode: 2,
            actor: Some(a),
            target: Target::Minion(t),
        };
        let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
        assert!(
            !req.options.contains(&strike(y)),
            "the blocker's Methuselah"
        );
        pick(&mut e, strike(w));
        e.notes = Some(Vec::new());
        match case {
            "dodged" => {
                pick_when(&mut e, play(0, 0, z));
                pick_when(&mut e, play(0, 1, a));
            }
            "continued" => pick_when(&mut e, play(1, 1, z)),
            _ => never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, a)),
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let begun: Vec<[MinionId; 2]> = noted_combats(&mut e)
            .into_iter()
            .filter_map(|n| match n {
                zvtes::combat::CombatNote::Begins { minions } => Some(minions),
                _ => None,
            })
            .collect();
        match case {
            "new" => assert_eq!(begun, vec![[z, w]], "the blocker first"),
            "dodged" => assert_eq!(begun, vec![[a, z]], "Psyche!'s, not w's"),
            _ => assert!(begun.is_empty(), "{case}: {begun:?}"),
        }
    }
}

/// [PIB 20111105] Blissful Agony [VAL] "Cannot be played against an
/// ally." [RTR 20020501] "Cannot be used to start a new combat if there is
/// already a pending combat queued": not offered after Psyche! queued the
/// combat... (an ally blocker here).
#[test]
fn blissful_agony_superior_not_against_an_ally() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), find_card("Langa"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_ally(SeatId(1), find_library("Gregory Winter"));
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let w = e.add_minion(SeatId(2), plain("Toreador", 0), 4, false);
    hand(&mut e, 0, &["Blissful Agony"]);
    blocked(&mut e, a, z);
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::Play {
            card: 0,
            mode: 2,
            actor: Some(a),
            target: Target::Minion(w),
        },
    );
}

/// Eye of Unforgiving Heaven: "Strike: 2R aggravated damage to all
/// vampires, demon allies and demon retainers in combat. If this striking
/// vampire is burned during the resolution of this strike, you gain 2
/// pool." A 1-blood striker burns, its Methuselah gains 2 pool, the blocker
/// goes to torpor; at [AUS][VAL] the striker's 2 are normal: mended, no
/// pool.
#[test]
fn eye_of_unforgiving_heaven_hits_both_vampires_and_pays_if_the_striker_burns() {
    for superior in [false, true] {
        let mut e = game(2);
        let card = find_card(if superior { "Azrael" } else { "Nkechi" });
        let a = e.add_minion(SeatId(0), card, if superior { 4 } else { 1 }, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Eye of Unforgiving Heaven"]);
        blocked(&mut e, a, z);
        pick_when(&mut e, play(0, u8::from(superior), a));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(e.state.minions[z.index()].in_torpor, "2 aggravated");
        if superior {
            assert!(e.state.minions[a.index()].ready(), "2 normal, mended");
            assert_eq!(blood(&e, a), 4 - 2 - 1);
            assert_eq!(pool(&e)[0], 10);
        } else {
            assert!(!e.state.minions[a.index()].present(), "burned");
            assert_eq!(pool(&e)[0], 12, "2 pool");
        }
    }
}

/// Eye of Unforgiving Heaven by Noluthando ("+1 damage with ranged
/// strikes (even at close range)"): his bonus on every recipient (rules-qa
/// 113.87, the owner): at [AUS][VAL] his own share is 3 normal (3 blood,
/// beside the blocker's hand strike), the blocker's 3 aggravated.
#[test]
fn eye_of_unforgiving_heaven_carries_the_strikers_bonus_to_every_recipient() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Noluthando"), 6, false);
    e.state.minions[a.index()].gained_levels = vec!["aus", "val", "val"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Eye of Unforgiving Heaven"]);
    blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[z.index()].in_torpor, "aggravated");
    assert_eq!(
        blood(&e, a),
        6 - 3 - 1,
        "his own 3, the blocker's hand strike"
    );
}

/// Eye of Unforgiving Heaven against a mortal ally: "vampires, demon
/// allies and demon retainers" — the ally takes nothing, the striker its 2.
#[test]
fn eye_of_unforgiving_heaven_skips_a_non_demon_ally() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Nkechi"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_ally(SeatId(1), find_library("Gregory Winter"));
    let life = blood(&e, z);
    hand(&mut e, 0, &["Eye of Unforgiving Heaven"]);
    blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), life, "untouched");
    assert!(e.state.minions[a.index()].in_torpor, "its own 2 aggravated");
}

/// Fractura [str]: "This vampire gets X additional strikes (limited) each
/// round. Those additional strikes can only be hand strikes." X = 2 paid in
/// blood: two more hand strikes this round and the next (Psyche!); in
/// those, Compress [pot] (a hand strike) is offered, Fractura's [cel] dodge
/// not (it is, for the normal strike). [LSJ 20011204-3] [PIB 20130704]
/// [str] counts as required as played (Striga's level, its infernal bond
/// aside).
#[test]
fn fractura_gives_x_hand_strikes_each_round() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| c.disciplines.contains(&"cel") && c.capacity >= 6, 0),
        6,
        false,
    );
    e.state.minions[a.index()].gained_levels = vec!["pot"];
    on_with(
        &mut e,
        0,
        "Striga",
        a,
        &[zvtes::cards::Modifier::BearerDisciplineLevel("str")],
    );
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), textless(|c| c.capacity >= 7, 0), 7, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Fractura", "Compress", "Fractura", "Psyche!"]);
    blocked(&mut e, a, z);
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    // Copies are offered once, by their first index.
    assert!(
        req.options.contains(&play(0, 0, a)),
        "a normal strike's dodge"
    );
    pick(&mut e, Choice::HandStrike);
    pick_when(&mut e, play(0, 1, a));
    pick(&mut e, set_x(2));
    for _ in 0..2 {
        let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
        assert!(req.options.contains(&play(0, 0, a)), "a hand strike card");
        assert!(!req.options.contains(&play(1, 0, a)), "no dodge");
        pick(&mut e, Choice::HandStrike);
    }
    pick_when(&mut e, play(2, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 7 - 3 - 3, "three hand strikes a round");
    assert_eq!(blood(&e, a), 6 - 2 - 2, "X, the blocker's two");
}

/// [PIB 20120225] [LSJ 20011215] Blissful Agony [VAL] "If both vampires
/// play it, the acting Methuselah decides which resolves first. The other
/// is lost." The acting Methuselah alone is asked: its own first, the
/// blocker enters combat with the acting vampire's choice; the blocker's
/// first, the acting vampire with the blocker's.
#[test]
fn blissful_agony_both_superiors_the_acting_methuselah_picks_first() {
    for acting_first in [true, false] {
        let mut e = game(3);
        let a = e.add_minion(SeatId(0), find_card("Langa"), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), find_card("Azrael"), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let w = e.add_minion(SeatId(2), plain("Toreador", 0), 4, false);
        let u = e.add_minion(SeatId(2), plain("Toreador", 1), 4, false);
        hand(&mut e, 0, &["Blissful Agony"]);
        hand(&mut e, 1, &["Blissful Agony"]);
        blocked(&mut e, a, z);
        let strike = |m: MinionId, t: MinionId| Choice::Play {
            card: 0,
            mode: 2,
            actor: Some(m),
            target: Target::Minion(t),
        };
        pick_when(&mut e, strike(a, w));
        e.notes = Some(Vec::new());
        pick_when(&mut e, strike(z, u));
        let req = request(&mut e);
        assert_eq!(req.seat, SeatId(0), "the acting Methuselah");
        assert_eq!(
            req.options,
            vec![Choice::ChooseMinion(a), Choice::ChooseMinion(z)]
        );
        pick(
            &mut e,
            Choice::ChooseMinion(if acting_first { a } else { z }),
        );
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let begun: Vec<[MinionId; 2]> = noted_combats(&mut e)
            .into_iter()
            .filter_map(|n| match n {
                zvtes::combat::CombatNote::Begins { minions } => Some(minions),
                _ => None,
            })
            .collect();
        let want = if acting_first { [z, w] } else { [a, u] };
        assert_eq!(begun, vec![want], "one new combat, the other lost");
    }
}

/// [RTR 20020501] Blissful Agony [VAL] "Cannot be used to start a new
/// combat if there is already a pending combat queued": Hidden Lurker
/// queued one after the block; the strike is not offered in the block's
/// combat.
#[test]
fn blissful_agony_superior_not_with_a_combat_queued() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), find_card("Langa"), 4, false);
    let l = e.add_minion(SeatId(0), vampire_with("obf", 0), 3, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.add_minion(SeatId(2), plain("Toreador", 0), 4, false);
    hand(&mut e, 0, &["Hidden Lurker", "Blissful Agony"]);
    blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 0, l));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        matches!(
            o,
            Choice::Play {
                card: 0,
                mode: 2,
                ..
            }
        )
    });
}

/// [LSJ 20020301] Darkness Within "is ordered against steal blood effects
/// (eg. Theft of Vitae) by the acting minion's controller". The blocker, at
/// 0 blood, steals 1 with Theft of Vitae and takes a hand strike: the
/// steal first, the card drains that blood and the blocker cannot mend
/// (torpor); the drain first, it finds none and the blocker mends.
#[test]
fn darkness_within_is_ordered_against_a_blood_steal() {
    for drain_first in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(
            SeatId(0),
            textless(|c| c.disciplines.contains(&"obt") && c.capacity >= 5, 0),
            4,
            false,
        );
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 0, false);
        e.state.minions[z.index()].gained_levels = vec!["tha"];
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Darkness Within"]);
        hand(&mut e, 1, &["Theft of Vitae"]);
        blocked(&mut e, a, z);
        pick_when(&mut e, play(0, 0, a));
        pick_when(&mut e, play(0, 0, z));
        let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
        assert!(
            matches!(req.options[..], [Choice::Pass, Choice::ResolveRider { .. }]),
            "{:?}",
            req.options
        );
        e.choose(usize::from(drain_first));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let zv = &e.state.minions[z.index()];
        assert_eq!(zv.ready(), drain_first, "drain first {drain_first}");
        assert_eq!(blood(&e, z), 0);
        let card_back = u8::from(!drain_first);
        assert_eq!(
            blood(&e, a),
            4 - 1 - 1 + card_back,
            "cost, the steal, the card"
        );
    }
}

/// [LSJ 20091203] Evil Eye: "The burn option can be used if all your
/// vampires with [mal] are in torpor": discarded in the unlock phase while
/// the Maleficia vampire is torpid, not while it is ready.
#[test]
fn evil_eye_burn_option_with_its_vampire_in_torpor() {
    for torpid in [true, false] {
        let mut e = game(2);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let m = infernal(&mut e, 1, 4, torpid);
        maleficia(&mut e, 1, m, 1);
        hand(&mut e, 1, &["Evil Eye"]);
        e.state.seats[1].library = vec![find_library("Soak"); 2].into();
        if !torpid {
            never_offered_until(
                &mut e,
                SeatId(1),
                WindowLabel::MinionPhase,
                Choice::Discard(0),
            );
            continue;
        }
        until(&mut e, SeatId(1), WindowLabel::UnlockAbilities);
        pick(&mut e, Choice::Discard(0));
        assert!(e.state.seats[1].ash.contains(&find_library("Evil Eye")));
    }
}

/// The `nth` textless untitled vampire with `disc` of at least `cap`.
fn with_cap(disc: &str, cap: u8, nth: usize) -> u16 {
    textless(|c| c.disciplines.contains(&disc) && c.capacity >= cap, nth)
}

/// Groaning Corpse: "[thn] X is the number of Groaning Corpses already
/// played this combat." Either combatant's copies count: the first costs
/// nothing, the blocker's next one 1 blood, the acting vampire's third 2.
#[test]
fn groaning_corpse_costs_one_more_for_each_played_this_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_cap("thn", 4, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), with_cap("thn", 4, 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Groaning Corpse", "Groaning Corpse"]);
    // The blocker's second copy keeps the before-range window open.
    hand(&mut e, 1, &["Groaning Corpse", "Groaning Corpse"]);
    blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    assert_eq!(blood(&e, a), 4, "none played before: X = 0");
    pick_when(&mut e, play(0, 0, z));
    request(&mut e);
    assert_eq!(blood(&e, z), 3, "one played: X = 1");
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    assert_eq!(blood(&e, a), 2, "two played: X = 2");
}

/// [RTR 19941109] [TOM 19951216] [PIB 20140324] [ANK 20200422] Groaning
/// Corpse: "Damage is inflicted even against a dodge." The blocker dodges
/// the acting vampire's hand strike and still takes the 1 environmental
/// damage at close range; [THN] "A vampire can play only one Groaning
/// Corpse at superior each combat": a second copy is offered at [thn]
/// only.
#[test]
fn groaning_corpse_damages_at_close_range_through_a_dodge() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_cap("THN", 6, 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Groaning Corpse", "Groaning Corpse"]);
    hand(&mut e, 1, &["Dodge"]);
    blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 1, a));
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 1, a)), "one at superior");
    assert!(req.options.contains(&play(0, 0, a)), "[thn] still");
    pick_when(&mut e, play(0, 0, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 4 - 2, "2 environmental, dodged strike");
}

/// Hexe [STR]: "Strike: 1R aggravated damage … and the damage is
/// unpreventable." The blocker's Indomitability is never offered and the
/// aggravated point sends it to torpor; at [str] it prevents the point.
/// [LSJ 20011204-3] [PIB 20130704] played with [str] (Striga), it counts
/// as requiring that tag only.
#[test]
fn hexe_superior_is_unpreventable_aggravated() {
    for superior in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let striga = zvtes::cards::Modifier::BearerDisciplineLevel("str");
        on_with(&mut e, 0, "Striga", a, &[striga]);
        if superior {
            on_with(&mut e, 0, "Striga", a, &[striga]);
        }
        let z = e.add_minion(SeatId(1), vampire_with("for", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Hexe"]);
        hand(&mut e, 1, &["Indomitability"]);
        blocked(&mut e, a, z);
        let mode = if superior { 2 } else { 1 };
        pick_when(&mut e, play(0, mode, a));
        if superior {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, z));
            assert!(e.state.minions[z.index()].in_torpor, "1 aggravated");
        } else {
            pick_when(&mut e, play(0, 0, z));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert!(!e.state.minions[z.index()].in_torpor, "prevented");
        }
    }
}

/// Lead Fist [vis]: "once this round this vampire can burn 1 blood to get
/// 1 press, only usable to continue combat." [ANK 20210226] The blood an
/// effect: burned as the press is used. Offered in the round's press step,
/// the combat continuing (a's second hand strike lands); not in the next
/// round's.
#[test]
fn lead_fist_burns_a_blood_for_a_press_to_continue() {
    let mut e = game(2);
    // Malachai: [VIS], no slave, printing only Flight.
    let a = e.add_minion(SeatId(0), find_card("Malachai"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Lead Fist"]);
    blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 1, a));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatPress);
    assert!(req.options.contains(&Choice::UsePress));
    let before = blood(&e, a);
    pick(&mut e, Choice::UsePress);
    assert_eq!(blood(&e, a), before - 1, "1 blood for the press");
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::UsePress,
    );
    assert_eq!(
        blood(&e, z),
        4 - 2 - 1,
        "+1 hand strike, then a second round's"
    );
    assert_eq!(
        blood(&e, a),
        before - 1 - 1,
        "the blocker's second hand strike"
    );
}

/// [RTR 19970630] [LSJ 19990723] [LSJ 19980526] [RTR 20041202] [PIB
/// 20130319] Loving Agony [val]: "inflict 1 unpreventable damage on the
/// opposing minion after combat ends": the vampire's damage, after combat
/// (the blocker's Indomitability never offered for it, [LSJ 20100204-2]);
/// "If the strike is dodged, no damage is done."
#[test]
fn loving_agony_damages_after_combat_unless_dodged() {
    for dodged in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.state.minions[a.index()].gained_levels = vec!["val"];
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), vampire_with("for", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Loving Agony"]);
        hand(&mut e, 1, &["Dodge", "Indomitability"]);
        blocked(&mut e, a, z);
        pick_when(&mut e, play(0, 1, a));
        if dodged {
            pick_when(&mut e, play(0, 0, z));
        }
        never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(1, 0, z));
        assert_eq!(blood(&e, z), if dodged { 4 } else { 3 });
        assert_eq!(blood(&e, a), 3, "its blood");
    }
}

/// [ANK 20180618] Loving Agony [VAL]: "this vampire can burn 1 blood to
/// unlock before combat ends": offered in the end-of-round window, the
/// vampire unlocked for 1 blood. [ANK 20210226] an effect's blood.
#[test]
fn loving_agony_superior_unlocks_for_blood_after_the_end_of_round() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("VAL", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Loving Agony"]);
    blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 2, a));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatEndOfRound);
    assert!(e.state.minions[a.index()].locked);
    assert!(req.options.contains(&Choice::UnlockForBlood { minion: a }));
    pick(&mut e, Choice::UnlockForBlood { minion: a });
    assert!(!e.state.minions[a.index()].locked);
    assert_eq!(blood(&e, a), 4 - 1 - 1, "the card's blood, the unlock's");
}

/// [RTR 20020501] [LSJ 19980109] Loving Agony [VAL]: "If the combat
/// continues or a new combat begins, no damage is done. The vampire can
/// still burn 1 blood to unlock." The blocker's Telepathic Tracking starts
/// a new round: the after-combat damage is gone, the unlock still offered.
#[test]
fn loving_agony_superior_unlock_survives_a_continued_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("VAL", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("AUS", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Loving Agony"]);
    hand(&mut e, 1, &["Telepathic Tracking"]);
    blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 2, a));
    pick_when(&mut e, play(0, 1, z));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatBeforeRange);
    let c = e.combat().expect("combat");
    assert_eq!(c.round, 2);
    assert!(c.after_combat_damage.is_empty(), "no damage after combat");
    assert!(req.options.contains(&Choice::UnlockForBlood { minion: a }));
    pick(&mut e, Choice::UnlockForBlood { minion: a });
    assert!(!e.state.minions[a.index()].locked);
}

/// [RTR 19941109] [ANK 20190116] [LSJ 20070808-1] Morphean Blow [val]:
/// "Strike: combat ends, and put this card on the opposing minion. The
/// attached minion cannot act or block. Burn this card at the end of the
/// turn." The card itself on the blocker after combat (not in the ash
/// heap); unlocked, it is still offered no block of the next action; the
/// card burns as the turn ends.
#[test]
fn morphean_blow_puts_itself_on_the_opponent_who_cannot_act_or_block() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("val", 0), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let y = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Morphean Blow"]);
    blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let mb = card_named(&e, "Morphean Blow");
    assert_eq!(e.state.cards[mb.index()].host, Some(z));
    assert!(!e.state.cards[mb.index()].effect_only);
    assert!(!e.state.seats[0]
        .ash
        .contains(&find_library("Morphean Blow")));
    assert_eq!(e.state.only_actions(z), Some(&[][..]), "cannot act");
    e.state.minions[z.index()].locked = false;
    pick(&mut e, bleed(b));
    let req = offering(&mut e, block(y));
    assert!(!req.options.contains(&block(z)), "cannot block");
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(
        !e.state.cards[mb.index()].present(),
        "burned as the turn ended"
    );
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Morphean Blow")));
}

/// [RTR 20041202] Morphean Blow [val]: "If the strike is dodged, the card
/// is not put on the opposing minion." [RTR 20020501] [LSJ 19980109] "If
/// the combat continues or a new combat begins, the card is not put on the
/// opposing minion": the blocker's Dodge, or its Telepathic Tracking's new
/// round, leaves the card in the ash heap.
#[test]
fn morphean_blow_dodged_or_continued_puts_nothing() {
    for (blocker, card, mode) in [
        (plain("Ventrue", 0), "Dodge", 0),
        (vampire_with("AUS", 0), "Telepathic Tracking", 1),
    ] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("val", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), blocker, 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Morphean Blow"]);
        hand(&mut e, 1, &[card]);
        blocked(&mut e, a, z);
        pick_when(&mut e, play(0, 1, a));
        pick_when(&mut e, play(0, mode, z));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let mb = find_library("Morphean Blow");
        assert!(
            !e.state.cards.iter().any(|c| c.card == mb && c.present()),
            "{card}: not put on"
        );
        assert!(e.state.seats[0].ash.contains(&mb));
    }
}

/// [RTR 19970630] Morphean Blow [VAL]: "if this vampire was blocked while
/// performing a non-bleed action, the action continues as if unblocked":
/// a blocked hunt goes on and gains its blood; a blocked bleed does not
/// (no pool lost).
#[test]
fn morphean_blow_superior_continues_a_blocked_non_bleed_action() {
    for hunt in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("VAL", 0), 3, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        // +1 intercept: the hunt's +1 stealth reached.
        let spy = on_with(
            &mut e,
            1,
            "Raven Spy",
            z,
            &[zvtes::cards::Modifier::Bearer {
                stat: Stat::Intercept,
                n: 1,
                during: &[],
                who: zvtes::cards::Bearer::Any,
            }],
        );
        e.state.cards[spy.index()].counters = 1;
        hand(&mut e, 0, &["Morphean Blow"]);
        if hunt {
            pick(&mut e, action(a, ActionKind::Hunt));
        } else {
            pick(&mut e, bleed(a));
        }
        pick_when(&mut e, block(z));
        pick_when(&mut e, play(0, 2, a));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        if hunt {
            assert_eq!(blood(&e, a), 3 - 1 + 1, "the card's blood; the hunt's");
        } else {
            assert_eq!(blood(&e, a), 3 - 1);
            assert_eq!(e.state.seats[1].pool, 10, "no bleed");
        }
        assert_eq!(
            e.state.cards[card_named(&e, "Morphean Blow").index()].host,
            Some(z)
        );
    }
}

/// Olid Loa: "Retainers on the opposing minion lose their abilities this
/// combat." The blocker's Murder of Crows deals no damage to the dodging
/// Samedi; without Olid Loa its 1 damage lands through the dodge.
#[test]
fn olid_loa_silences_the_opponents_retainers() {
    for olid in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), of_clan("Samedi", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let crows = on_with(
            &mut e,
            1,
            "Murder of Crows",
            z,
            &[zvtes::cards::Modifier::RetainerDamage(1)],
        );
        e.state.cards[crows.index()].counters = 1;
        hand(&mut e, 0, &["Olid Loa", "Dodge"]);
        blocked(&mut e, a, z);
        if olid {
            pick_when(&mut e, play(0, 0, a));
        }
        pick_when(&mut e, play(if olid { 0 } else { 1 }, 0, a));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, a), if olid { 4 } else { 3 }, "the crows' damage");
        assert!(e.state.retainers_silenced.0.is_empty(), "this combat only");
    }
}

/// Olid Loa: "The opposing minion cannot maneuver to close or press to
/// continue this combat." Taken to long range by the Samedi's Pursuit,
/// the blocker's Pursuit (to close) and Apportation (a press to continue)
/// are never offered; not a frenzy card.
#[test]
fn olid_loa_bars_close_maneuvers_and_continue_presses() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Samedi", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["cel"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(
        SeatId(1),
        textless(
            |c| c.disciplines.contains(&"cel") && c.disciplines.contains(&"tha"),
            0,
        ),
        4,
        false,
    );
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Olid Loa", "Pursuit"]);
    hand(&mut e, 1, &["Pursuit", "Apportation"]);
    blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, a));
    let barred = [play(0, 0, z), play(1, 0, z)];
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        barred.contains(o)
    });
}

/// Penitent Resilience: "[for] This round, this vampire treats aggravated
/// damage as normal damage … [for][obe] As [for] above, but until the end
/// of combat." The blocker's aggravated hand strikes in two rounds (the
/// acting vampire's Flash presses to continue): at [for][obe] both mended
/// as normal; at [for] the second round's sends it to torpor.
#[test]
fn penitent_resilience_treats_aggravated_as_normal_this_round_or_combat() {
    for mode in [1, 0] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), with_cap("for", 5, 0), 5, false);
        e.state.minions[a.index()].gained_levels = vec!["obe", "cel"];
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.minions[z.index()].timed.push((
            zvtes::cards::Lasting::AggravatedHands,
            zvtes::state::Expiry::EndOfTurn,
            SeatId(1),
        ));
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Penitent Resilience", "Flash"]);
        blocked(&mut e, a, z);
        pick_when(&mut e, play(0, mode, a));
        until(&mut e, SeatId(0), WindowLabel::CombatPress);
        assert_eq!(blood(&e, a), 4, "1 normal damage mended");
        pick(&mut e, play(0, 0, a));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(
            e.state.minions[a.index()].in_torpor,
            mode == 0,
            "the second round's aggravated point"
        );
    }
}

/// Penitent Resilience: "cards named Flash Grenade have no effect on this
/// vampire". The blocker's Flash Grenade strike (combat ends) does not end
/// the combat: the acting vampire's hand strike lands and it is not stunned
/// after combat; without the card, the combat ends and it is. Either way
/// the bearer, at close range, is stunned and the grenade burned (owner,
/// 2026-11-10: the card has no effect on this vampire only).
#[test]
fn penitent_resilience_shrugs_off_a_flash_grenade() {
    for penitent in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("for", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let grenade = in_play(&mut e, 1, "Flash Grenade", Some(z));
        hand(&mut e, 0, &["Penitent Resilience"]);
        blocked(&mut e, a, z);
        if penitent {
            pick_when(&mut e, play(0, 0, a));
        }
        pick_when(&mut e, ability(grenade.0, 0));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(
            blood(&e, z),
            if penitent { 3 } else { 4 },
            "a's hand strike"
        );
        assert_eq!(e.state.minions[a.index()].skip_unlock, !penitent, "stunned");
        let bearer = &e.state.minions[z.index()];
        assert!(bearer.locked && bearer.skip_unlock, "the bearer stunned");
        assert!(e.state.seats[1]
            .ash
            .contains(&find_library("Flash Grenade")));
    }
}

/// [ANK 20190425] [ANK 20181208] Penitent Resilience [FOR][OBE]: "Only
/// usable as the action is announced. Choose a vampire. The chosen vampire
/// cannot block this action." The chosen blocker is not offered, the other
/// one is.
#[test]
fn penitent_resilience_superior_bars_the_chosen_blocker() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("FOR", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["obe", "obe"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let y = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Penitent Resilience"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, act(0, 2, a, Target::Minion(z)));
    let req = offering(&mut e, block(y));
    assert!(!req.options.contains(&block(z)));
}

/// [TOM 19960225] [LSJ 19970814-2] Pounce: "hand strike at +2 damage, and
/// the opposing minion cannot strike for the remainder of the round". The
/// blocker's hand strike in the same resolution lands; its Pursuit [CEL]
/// additional strike is still playable and gives no strike.
#[test]
fn pounce_bars_the_opponents_additional_strikes_this_round() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("FLIGHT", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), with_cap("CEL", 6, 0), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Pounce"]);
    hand(&mut e, 1, &["Pursuit"]);
    blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 1, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let strength = 1 + 2;
    assert!(blood(&e, z) <= 6 - strength, "+2 hand strike");
    assert_eq!(blood(&e, a), 4 - 1, "the normal pair only");
}

/// [LSJ 19970801] [TOM 19960521] Pounce: "If this strike is dodged, this
/// striking minion takes 1 damage during strike resolution and the
/// opposing minion gets an optional press." Environmental damage on the
/// striker; the blocker offered a press that round.
#[test]
fn pounce_dodged_damages_the_striker_and_gives_a_press() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("FLIGHT", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Pounce"]);
    hand(&mut e, 1, &["Dodge"]);
    blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, z));
    let req = until(&mut e, SeatId(1), WindowLabel::CombatPress);
    assert!(
        req.options.contains(&Choice::UsePress),
        "the optional press"
    );
    assert_eq!(blood(&e, a), 4 - 1, "1 damage, mended");
    assert_eq!(blood(&e, z), 4, "dodged");
}

/// [LSJ 20011214] [LSJ 20020624] Pounce "used with First Strike … prevents
/// the opponents normal non-dodge non-combat ends strike from resolving":
/// Elimelech (first strike) Pounces and the blocker's held hand strike
/// never lands.
#[test]
fn pounce_with_first_strike_voids_the_opposing_strike() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Elimelech the Twice-Damned"), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["flight", "flight"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), with_cap("for", 6, 0), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Pounce"]);
    blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4, "the held strike voided");
    assert!(blood(&e, z) < 6, "the Pounce landed");
}

/// Celerity and Presence, at either level.
fn celerity_presence(c: &zvtes::crypt::CryptCard) -> bool {
    ["cel", "pre"]
        .iter()
        .all(|d| c.disciplines.iter().any(|k| k.eq_ignore_ascii_case(d)))
}

/// Scourge of Alecto [cel][pre]: "Strike: X-2 ranged damage, where X is
/// this vampire's capacity minus the target's capacity or cost. If more
/// than 4 damage is inflicted with this strike, ignore the excess." A
/// 10-capacity striker against a 3-capacity blocker: 4, not 5 (to
/// torpor); against one of about its capacity: none.
#[test]
fn scourge_of_alecto_deals_the_capacity_gap_minus_two_capped_at_four() {
    // Grimgroth's +2 bleed and mage costs, Karsh's votes and his bar on
    // blocking infernals: inert in this combat.
    let card = find_card("Grimgroth");
    let cap = CRYPT[card as usize].capacity;
    for small in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), card, 4, false);
        e.state.minions[a.index()].gained_levels = vec!["cel", "pre"];
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let (blocker, blood_z) = if small {
            (textless(|c| c.capacity == 3, 0), 3)
        } else {
            let big = find_card("Karsh");
            assert!(CRYPT[big as usize].capacity + 1 >= cap);
            (big, 4)
        };
        let z = e.add_minion(SeatId(1), blocker, blood_z, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Scourge of Alecto"]);
        // A choice for the blocker: its strike window stays open.
        hand(&mut e, 1, &["Dodge"]);
        blocked(&mut e, a, z);
        pick_when(&mut e, play(0, 0, a));
        until(&mut e, SeatId(1), WindowLabel::CombatStrike);
        let c = e.combat().expect("combat");
        let k = c.sides[0].strike.expect("declared");
        let told = c.strike_told(&e.state, e.action(), 0, k).expect("damage");
        assert_eq!(told.n, if small { 4 } else { 0 });
        assert!(!told.aggravated);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        if small {
            assert!(cap - 3 - 2 > 4, "an excess to ignore");
            assert!(e.state.minions[z.index()].in_torpor);
        } else {
            assert_eq!(blood(&e, z), 4);
        }
    }
}

/// Scourge of Alecto's "if more than 4 damage is inflicted with this
/// strike, ignore the excess" counts all its damage (rules-qa 113.135, the
/// owner): Grimgroth (capacity 10) against a capacity-4 blocker bearing
/// Dam the Heart's River [QUI] (+1 each strike against it) inflicts 4, not
/// 5: its 4 blood burned, still ready. The blocker declines its strike
/// (Dam's toll).
#[test]
fn scourge_of_alecto_caps_dam_the_hearts_river_too() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Grimgroth"), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["cel", "pre", "qui", "qui"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), textless(|c| c.capacity == 4, 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Dam the Heart's River", "Scourge of Alecto"]);
    blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(1), WindowLabel::CombatStrike);
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[z.index()];
    assert!(v.ready(), "4 damage on 4 blood");
    assert_eq!(blood(&e, z), 0);
}

/// Scourge of Alecto: "Only usable against a vampire or werewolf": not
/// offered against a mortal ally blocker.
#[test]
fn scourge_of_alecto_not_against_a_mortal_ally() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(celerity_presence, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Political Ally"));
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Scourge of Alecto"]);
    blocked(&mut e, a, ally);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
}

/// Spirit Claws [PRO][SPI]: "+1 stealth action. Unique. Put this card on
/// this vampire. Damage from this vampire's hand strikes is aggravated."
/// The card itself on the actor (not in the ash heap); in a later combat
/// its hand strike sends the blocker to torpor.
#[test]
fn spirit_claws_on_the_vampire_aggravates_its_hand_strikes() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("PRO", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["spi", "spi"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Spirit Claws"]);
    pick(&mut e, play(0, 2, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let claws = card_named(&e, "Spirit Claws");
    assert_eq!(e.state.cards[claws.index()].host, Some(a));
    assert!(!e.state.cards[claws.index()].effect_only);
    assert!(!e.state.seats[0].ash.contains(&find_library("Spirit Claws")));
    e.state.minions[a.index()].locked = false;
    blocked(&mut e, a, z);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        e.state.minions[z.index()].in_torpor,
        "an aggravated hand strike"
    );
}

/// [ANK 20180629] Spirit Claws: "The [pro] and [spi][pro] versions do not
/// contest with it." [RTR 19960112] "Can be played after both strikes have
/// been chosen." With a copy in play on another of its vampires, [pro] is
/// still offered, played before resolution: the blocker goes to torpor,
/// the copy in play uncontested.
#[test]
fn spirit_claws_combat_versions_do_not_contest_the_action_version() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("pro", 0), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let held = on_with(
        &mut e,
        0,
        "Spirit Claws",
        b,
        &[zvtes::cards::Modifier::BearerAggravatedHands],
    );
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Spirit Claws"]);
    blocked(&mut e, a, z);
    let req = until(&mut e, SeatId(0), WindowLabel::CombatBeforeResolution);
    assert!(req.options.contains(&play(0, 0, a)));
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        e.state.minions[z.index()].in_torpor,
        "aggravated this round"
    );
    assert!(!e.state.cards[held.index()].contested);
}

/// Sword of the Righteous: "[ani] Choose one of this vampire's melee
/// weapons. For the remainder of combat, this vampire inflicts +1 damage
/// each strike with that weapon. … [val] … and the damage this vampire
/// inflicts with the weapon is aggravated." [RTR 19970630] [TOM 19960225]
/// Its Baseball Bat strikes for 3: normal at [ani], aggravated at [val]
/// (torpor).
#[test]
fn sword_of_the_righteous_adds_aggravated_damage_to_the_chosen_weapon() {
    for mode in [0, 1] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("val", 0), 4, false);
        e.state.minions[a.index()].gained_levels = vec!["ani"];
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let bat = in_play(&mut e, 0, "Baseball Bat", Some(a));
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Sword of the Righteous"]);
        blocked(&mut e, a, z);
        pick_when(&mut e, at_card(0, mode, a, bat));
        pick_when(&mut e, ability(bat.0, 0));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        if mode == 0 {
            assert_eq!(blood(&e, z), 4 - 3, "strength 1, +1, +1");
        } else {
            assert!(e.state.minions[z.index()].in_torpor, "aggravated");
        }
    }
}

/// Sword of the Righteous: "A vampire can play only one Sword of the
/// Righteous each combat": the second copy is not offered; without a melee
/// weapon, none is.
#[test]
fn sword_of_the_righteous_needs_a_melee_weapon_and_once_a_combat() {
    for armed in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("val", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(
            &mut e,
            0,
            &["Sword of the Righteous", "Sword of the Righteous"],
        );
        let sword = |o: &Choice| matches!(o, Choice::Play { card: 0, .. });
        if !armed {
            blocked(&mut e, a, z);
            never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, sword);
            continue;
        }
        let bat = in_play(&mut e, 0, "Baseball Bat", Some(a));
        blocked(&mut e, a, z);
        pick_when(&mut e, at_card(0, 1, a, bat));
        never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, sword);
    }
}

/// Sword of the Righteous [VAL]: "this vampire may prevent 1 damage this
/// round": the blocker's hand strike prevented.
#[test]
fn sword_of_the_righteous_superior_prevents_one() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("VAL", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let bat = in_play(&mut e, 0, "Baseball Bat", Some(a));
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Sword of the Righteous"]);
    blocked(&mut e, a, z);
    pick_when(&mut e, at_card(0, 2, a, bat));
    pick_when(&mut e, Choice::UsePrevent);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4 - 1, "the card's blood only");
}

/// [LSJ 20030902-2] [LSJ 20060808-1] Vengeance of Samiel [val]: "this
/// strike cannot be dodged": its Baseball Bat strike at +1 lands through
/// the blocker's Dodge; [pot]'s is dodged. [LSJ 20011204-3] [PIB 20130704]
/// played with [pot] or [val], it counts as requiring that Discipline.
#[test]
fn vengeance_of_samiel_strikes_through_a_dodge_with_a_weapon() {
    for mode in [3, 1] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("val", 0), 4, false);
        e.state.minions[a.index()].gained_levels = vec!["pot"];
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let bat = in_play(&mut e, 0, "Baseball Bat", Some(a));
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Vengeance of Samiel"]);
        hand(&mut e, 1, &["Dodge"]);
        blocked(&mut e, a, z);
        pick_when(&mut e, at_card(0, mode, a, bat));
        pick_when(&mut e, play(0, 0, z));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let hit = if mode == 3 { 1 + 1 + 1 } else { 0 };
        assert_eq!(blood(&e, z), 4 - hit);
    }
}

// ---- Reactions and modifiers ------------------------------------------------

/// Covincraft (Kiasyd): "[REACTION] Reduce a bleed against you by 1.
/// [REACTION] Only usable during a referendum. This Kiasyd gains 4 votes."
/// A bleed of 1 at seat 1 reduced to nothing; in seat 0's referendum the
/// Kiasyd casts the 4 votes against, the caller's 1 in favor lost.
/// [PIB 20150105] [LSJ 19980107] the polling step only: see
/// `dia_de_los_muertos_passes_the_first_referendum_without_a_polling_step`.
#[test]
fn covincraft_reduces_a_bleed_and_votes_four() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let k = e.add_minion(SeatId(1), of_clan("Kiasyd", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Kine Resources Contested"]);
    hand(&mut e, 1, &["Covincraft", "Covincraft"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, act(0, 0, k, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10, "bled for 1 - 1");
    pick(&mut e, play(0, 0, b));
    pick(&mut e, alloc(0, 1));
    pick(&mut e, Choice::CastCardVote { favor: true });
    pick_when(&mut e, act(0, 1, k, Target::None));
    pick_when(&mut e, votes(k, false));
    close_polling(&mut e);
    assert_eq!(pool(&e), vec![10, 10], "4 votes against 1: it fails");
}

/// Ears of the Hare: "[pro] Burn 1 blood to get +1 intercept. [spi] +1
/// intercept. [SPI] +2 intercept." [ANK 20210226] [pro] cannot be played
/// without the blood to burn, and burns it as an effect. [LSJ 20011204-3]
/// [PIB 20130704] played at [pro], it counts as requiring Protean.
#[test]
fn ears_of_the_hare_burns_blood_for_intercept() {
    for start in [0, 1] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("obf", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), vampire_with("pro", 0), start, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Elder Impersonation"]);
        hand(&mut e, 1, &["Ears of the Hare"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        pick_when(&mut e, act(0, 0, a, Target::None));
        let ears = act(0, 0, z, Target::None);
        if start == 0 {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, ears);
            assert_eq!(e.state.seats[1].pool, 10 - 1, "not blocked");
        } else {
            pick_when(&mut e, ears);
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(blood(&e, z), 0, "burned as an effect");
            assert_eq!(e.state.seats[1].pool, 10, "blocked");
        }
    }
}

/// Folderol: "[aus] This vampire burns 1 blood to reduce a bleed against
/// you by 1. [myt] … [MYT] Reduce a bleed against any Methuselah by 1."
/// [ANK 20200607] in a 3-player game seat 2's Mytherceria vampire reduces
/// seat 0's bleed of 3 at seat 1 in its turn of the sequence.
#[test]
fn folderol_reduces_another_methuselahs_bleed() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), vampire_with("dom", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let m = e.add_minion(SeatId(2), vampire_with("MYT", 0), 4, false);
    hand(&mut e, 0, &["Conditioning"]);
    hand(&mut e, 2, &["Folderol"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, act(0, 0, a, Target::None));
    pick_when(&mut e, act(0, 2, m, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 2, "bled for 3 - 1");
}

/// Folderol [aus]: [ANK 20210226] "The card cannot be played if the minion
/// cannot afford to burn the blood"; with it, burned as an effect (not
/// reduced). [LSJ 20011204-3] [PIB 20130704] at [aus], Auspex.
#[test]
fn folderol_burns_blood_to_reduce() {
    for start in [0, 2] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), vampire_with("aus", 0), start, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 1, &["Folderol"]);
        let folderol = act(0, 0, z, Target::None);
        pick(&mut e, bleed(a));
        if start == 0 {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, folderol);
            assert_eq!(e.state.seats[1].pool, 10 - 1);
        } else {
            pick_when(&mut e, folderol);
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(blood(&e, z), start - 1);
            assert_eq!(e.state.seats[1].pool, 10, "bled for 1 - 1");
        }
    }
}

/// Hide the Heart [val]: "The action fails, unless the acting minion burns
/// 1 blood." [LSJ 20050216] [ANK 20190701] an ally has life, not blood: it
/// cannot pay, its bleed fails. [ANK 20200607] a third Methuselah's vampire
/// plays it in its turn of the sequence. [ANK 20211015] [RTR 20180719]
/// [LSJ 20070219] failed, not blocked: no pool lost.
#[test]
fn hide_the_heart_fails_an_allys_action() {
    let mut e = game(3);
    let ally = e.add_ally(SeatId(0), find_library("Carlton Van Wyk"));
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let r = e.add_minion(SeatId(2), vampire_with("val", 0), 4, false);
    hand(&mut e, 2, &["Hide the Heart"]);
    pick(&mut e, bleed(ally));
    pick_when(&mut e, act(0, 1, r, Target::None));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::BurnBloodToAvoid,
    );
    assert_eq!(e.state.seats[1].pool, 10, "failed");
}

/// Hide the Heart [val]: the acting vampire burns 1 blood and its bleed
/// goes on; or does not, and fails.
#[test]
fn hide_the_heart_lets_a_vampire_pay_to_go_on() {
    for pay in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let r = e.add_minion(SeatId(1), vampire_with("val", 0), 4, false);
        hand(&mut e, 1, &["Hide the Heart"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, act(0, 1, r, Target::None));
        pick_when(
            &mut e,
            if pay {
                Choice::BurnBloodToAvoid
            } else {
                Choice::Pass
            },
        );
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, a), if pay { 3 } else { 4 });
        assert_eq!(e.state.seats[1].pool, if pay { 9 } else { 10 }, "{pay}");
    }
}

/// Hide the Heart: "Only one Hide the Heart can be played at [val] each
/// action": after a second vampire's [val], a third's [val] is not offered,
/// its [aus] still is; an [aus] played before did not bar the [val].
#[test]
fn hide_the_heart_once_at_val_each_action() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let r1 = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let r2 = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let r3 = e.add_minion(SeatId(1), plain("Ventrue", 2), 4, false);
    for r in [r1, r2, r3] {
        e.state.minions[r.index()].gained_levels = vec!["val", "aus"];
    }
    hand(
        &mut e,
        1,
        &["Hide the Heart", "Hide the Heart", "Hide the Heart"],
    );
    pick(&mut e, bleed(a));
    pick_when(&mut e, act(0, 0, r1, Target::None));
    pick_when(&mut e, act(0, 1, r2, Target::None));
    pick_when(&mut e, Choice::BurnBloodToAvoid);
    let req = offering(&mut e, act(0, 0, r3, Target::None));
    assert!(
        !req.options.contains(&act(0, 1, r3, Target::None)),
        "{req:?}"
    );
}

/// Hide the Heart [VAL]: "… or lock to reduce a bleed against any
/// Methuselah by 2." [ANK 20230817] [RBK wording-templates] the lock needs
/// an unlocked vampire: a woken one (On the Qui Vive) is not offered it; an
/// unlocked third party's locks and takes 2 off seat 0's bleed of 3 at seat
/// 1.
#[test]
fn hide_the_heart_locks_to_reduce_another_methuselahs_bleed() {
    for woken in [true, false] {
        let mut e = game(3);
        let a = e.add_minion(SeatId(0), vampire_with("dom", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let r = e.add_minion(SeatId(2), vampire_with("VAL", 0), 4, false);
        e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
        e.state.minions[r.index()].locked = woken;
        hand(&mut e, 0, &["Conditioning"]);
        hand(&mut e, 2, &["Hide the Heart", "On the Qui Vive"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, act(0, 0, a, Target::None));
        let lock = act(0, 3, r, Target::None);
        if woken {
            pick_when(&mut e, act(1, 0, r, Target::None));
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, lock);
            assert_eq!(e.state.seats[1].pool, 10 - 3);
        } else {
            pick_when(&mut e, lock);
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert!(e.state.minions[r.index()].locked);
            assert_eq!(e.state.seats[1].pool, 10 - 1, "bled for 3 - 2");
        }
    }
}

/// Knotted Cord: "Requires an infernal vampire. Only usable during the
/// polling step of a political action. Force a younger vampire to abstain
/// (this can cancel their votes and ballots)." A reacting infernal makes
/// the younger prince calling abstain after it voted: its votes canceled,
/// the referendum fails; an older vampire is no target. [PIB 20150105]
/// [LSJ 19980107] the polling step only: see
/// `dia_de_los_muertos_passes_the_first_referendum_without_a_polling_step`.
#[test]
fn knotted_cord_makes_a_younger_vampire_abstain() {
    let mut e = game(2);
    let inf = textless(|c| c.capacity == 6, 0);
    let cap = CRYPT[inf as usize].capacity;
    let prince = CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            matches!(c.title, Title::Prince(_))
                && c.capacity < cap
                && zvtes::cards::vampire(i as u16).is_none()
        })
        .expect("a younger textless prince") as u16;
    let a = e.add_minion(SeatId(0), prince, 4, false);
    let old = e.add_minion(SeatId(0), textless(|c| c.capacity > cap, 0), 4, false);
    let r = e.add_minion(SeatId(1), inf, 4, false);
    e.state.minions[r.index()].made_infernal = true;
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Kine Resources Contested"]);
    hand(&mut e, 1, &["Knotted Cord"]);
    pick(&mut e, play(0, 0, a));
    pick(&mut e, alloc(0, 1));
    pick(&mut e, votes(a, true));
    let req = offering(&mut e, act(0, 0, r, Target::Minion(a)));
    assert!(!req.options.contains(&act(0, 0, r, Target::Minion(old))));
    pick(&mut e, act(0, 0, r, Target::Minion(a)));
    close_polling(&mut e);
    assert_eq!(pool(&e), vec![10, 10], "the prince's votes canceled: fails");
}

/// Mole's Tunneling [ani]: "Only usable if an ally or younger vampire
/// attempts to block. Allies and younger vampires get -1 intercept." The
/// younger blocker's attempt at a 0-stealth bleed misses.
#[test]
fn moles_tunneling_lowers_younger_intercept() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| c.disciplines.contains(&"ani") && c.capacity >= 6, 0),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), textless(|c| c.capacity <= 3, 0), 3, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Mole's Tunneling"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, act(0, 0, a, Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 1, "the attempt missed");
    assert_eq!(blood(&e, a), 4 - 1, "the card's cost");
}

/// Mole's Tunneling [SPI]: "Burn 1 blood to get +2 stealth." (1 blood.)
/// [ANK 20210226] not playable without the blood to burn beside the cost;
/// with 2, both paid, the +2 stealth beats a +1 intercept. [LSJ 20011204-3]
/// [PIB 20130704] at [SPI], Spiritus.
#[test]
fn moles_tunneling_superior_burns_blood_beside_its_cost() {
    for start in [1, 2] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("SPI", 0), start, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        interceptor(&mut e, z);
        hand(&mut e, 0, &["Mole's Tunneling"]);
        let spi = act(0, 2, a, Target::None);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        if start == 1 {
            never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, spi);
        } else {
            pick_when(&mut e, spi);
            until(&mut e, SeatId(1), WindowLabel::MinionPhase);
            assert_eq!(blood(&e, a), 0, "cost and burn");
            assert_eq!(e.state.seats[1].pool, 10 - 1, "unblocked");
        }
    }
}

/// Masca [str]: "This acting vampire gets an optional additional strike
/// (limited) each round of combat until the end of this action." The card
/// on its vampire for the action, its strike used once in the block's
/// round, then burned as the action ends. [LSJ 20011204-3] [PIB 20130704]
/// at [str] (not a Discipline), its own.
#[test]
fn masca_gives_an_optional_strike_each_round_of_the_action() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    on_with(
        &mut e,
        0,
        "Striga",
        a,
        &[zvtes::cards::Modifier::BearerDisciplineLevel("str")],
    );
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Masca"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, act(0, 1, a, Target::None));
    request(&mut e);
    let m = card_named(&e, "Masca");
    assert_eq!(e.state.cards[m.index()].host, Some(a));
    assert!(!e.state.cards[m.index()].effect_only);
    assert!(!e.state.seats[0].ash.contains(&find_library("Masca")));
    pick_when(&mut e, block(z));
    pick_when(&mut e, use_on(m, 0, Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 6 - 2, "two hand strikes");
    assert!(
        !e.state.cards[m.index()].present(),
        "burned as the action ends"
    );
    assert!(e.state.seats[0].ash.contains(&find_library("Masca")));
}

/// Ensconced [vis]: "+1 stealth action. Put this card on a location and
/// unlock this vampire. While this vampire is ready, their controller can
/// burn this card during an action directed at the attached location to
/// have the action fail, and the acting minion and this vampire enter
/// combat." The card itself on seat 0's location, its vampire unlocked;
/// seat 1's Arson at the location fails into a combat with that vampire,
/// the location kept, Ensconced burned. [LSJ 20070219] failed, not
/// blocked: no Freak Drive after it.
#[test]
fn ensconced_goes_on_a_location_and_fails_an_action_at_it() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), vampire_with("vis", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let a = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let museum = in_play(&mut e, 0, "Art Museum", None);
    hand(&mut e, 0, &["Ensconced"]);
    hand(&mut e, 1, &["Arson", "Freak Drive"]);
    pick(&mut e, at_card(0, 0, v, museum));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let c = card_named(&e, "Ensconced");
    assert_eq!(e.state.cards[c.index()].on_card, Some(museum));
    assert!(!e.state.cards[c.index()].effect_only);
    assert!(!e.state.seats[0].ash.contains(&find_library("Ensconced")));
    assert!(!e.state.minions[v.index()].locked, "unlocked");
    assert!(req.options.contains(&bleed(v)), "it may act again");
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, at_card(0, 0, a, museum));
    pick_when(&mut e, used_by(c, 0, v));
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::Play { card: 0, .. })
    });
    assert!(e.state.cards[museum.index()].present(), "the Arson failed");
    assert!(!e.state.cards[c.index()].present());
    assert!(e.state.seats[0].ash.contains(&find_library("Ensconced")));
    assert_eq!(blood(&e, v), 4 - 1, "in combat with the acting vampire");
    assert_eq!(blood(&e, a), 4 - 1);
}

/// Ensconced [VIS]: "Only usable during an action directed at a location.
/// This vampire unlocks, the action fails, and the acting minion and this
/// vampire enter combat." A third Methuselah's vampire, at seat 0's Arson
/// on seat 1's location: the location kept, the combat fought. [LSJ
/// 20070217] the action goes on until the combat ends, its acting minion
/// the same; [LSJ 20070214-2] it fails: the location stays, whatever the
/// combat. [ANK 20211015] [RTR 20180719] it reached resolution: Arson's
/// action not offered again to its vampire.
#[test]
fn ensconced_reaction_fails_an_action_at_a_location_into_combat() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let museum = in_play(&mut e, 1, "Art Museum", None);
    let r = e.add_minion(SeatId(2), vampire_with("VIS", 0), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Arson", "Arson"]);
    hand(&mut e, 2, &["Ensconced"]);
    pick(&mut e, at_card(0, 0, a, museum));
    pick_when(&mut e, act(0, 1, r, Target::None));
    assert_eq!(e.action().expect("Arson").acting, a);
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.cards[museum.index()].present(), "failed");
    assert_eq!(blood(&e, r), 4 - 1, "in combat");
    assert_eq!(blood(&e, a), 4 - 1);
    assert!(!req.options.contains(&at_card(0, 0, a, museum)));
}

/// Ensconced [VIS] by a locked vampire: as any reaction card, only woken
/// (On the Qui Vive); "this vampire unlocks" then unlocks it.
#[test]
fn ensconced_reaction_needs_an_unlocked_or_woken_vampire() {
    for woken in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let r = e.add_minion(SeatId(1), vampire_with("VIS", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.minions[r.index()].locked = true;
        let museum = in_play(&mut e, 1, "Art Museum", None);
        hand(&mut e, 0, &["Arson"]);
        let cards: &[&str] = if woken {
            &["Ensconced", "On the Qui Vive"]
        } else {
            &["Ensconced"]
        };
        hand(&mut e, 1, cards);
        let vis = act(0, 1, r, Target::None);
        pick(&mut e, at_card(0, 0, a, museum));
        if woken {
            pick_when(&mut e, act(1, 0, r, Target::None));
            pick_when(&mut e, vis);
            request(&mut e);
            assert!(!e.state.minions[r.index()].locked, "unlocked");
        } else {
            never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, vis);
            assert!(!e.state.cards[museum.index()].present(), "burned");
        }
    }
}

/// [ANK 20260316] [LSJ 20041022] Ensconced: "Cannot be used while a block
/// is being attempted, or has already been successful": seat 1's blocker
/// attempts, blocks, and neither the burn nor [VIS] is offered.
#[test]
fn ensconced_is_not_used_during_a_block_attempt() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let v = e.add_minion(SeatId(1), vampire_with("VIS", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let museum = in_play(&mut e, 1, "Art Museum", None);
    let mut c = CardInPlay::new(find_library("Ensconced"), SeatId(1), None, 0);
    c.on_card = Some(museum);
    c.by = Some(v);
    e.state.cards.push(c);
    let ens = CardId(e.state.cards.len() as u32 - 1);
    hand(&mut e, 0, &["Arson"]);
    hand(&mut e, 1, &["Ensconced"]);
    pick(&mut e, at_card(0, 0, a, museum));
    pick_when(&mut e, block(z));
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
        *o == used_by(ens, 0, v) || *o == act(0, 1, v, Target::None)
    });
    assert!(e.state.cards[museum.index()].present(), "blocked");
}

/// Gift of Sleep [obe]: "Only usable if this vampire is about to enter
/// combat with an ally. Lock that ally and end the action." The acting
/// vampire blocked by an ally plays it: the ally locked, no combat, the
/// bleed ended unsuccessfully, the vampire still locked. [RTR 20180719]
/// [ANK 20200207] it ends at once: nothing more played on the action.
/// [LSJ 20011204-3] [PIB 20130704] at [obe], Obedience's.
#[test]
fn gift_of_sleep_ends_an_action_blocked_by_an_ally() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["obe"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Carlton Van Wyk"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let life = blood(&e, ally);
    hand(&mut e, 0, &["Gift of Sleep", "Gift of Sleep"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(ally));
    pick_when(&mut e, act(0, 1, a, Target::None));
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == a),
    );
    assert!(e.state.minions[ally.index()].locked, "kept locked");
    assert!(e.state.minions[a.index()].locked);
    assert_eq!(blood(&e, ally), life, "no combat");
    assert_eq!(e.state.seats[1].pool, 10, "not successful");
}

/// Gift of Sleep [obe] by the blocker: "(Do not lock this vampire if they
/// are blocking.)" Seat 1's vampire blocks seat 0's acting ally and plays
/// it: no combat, the ally locked, the blocker unlocked as before its
/// block.
#[test]
fn gift_of_sleep_by_a_blocker_ends_an_allys_action() {
    let mut e = game(2);
    let ally = e.add_ally(SeatId(0), find_library("Carlton Van Wyk"));
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[z.index()].gained_levels = vec!["obe"];
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 1, &["Gift of Sleep"]);
    pick(&mut e, bleed(ally));
    pick_when(&mut e, block(z));
    pick_when(&mut e, act(0, 2, z, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        !e.state.minions[z.index()].locked,
        "not locked by its block"
    );
    assert!(e.state.minions[ally.index()].locked);
    assert_eq!(blood(&e, z), 4, "no combat");
    assert_eq!(e.state.seats[1].pool, 10);
}

/// Gift of Sleep [obe] is not offered against a vampire: blocked by one,
/// the acting vampire is never offered it.
#[test]
fn gift_of_sleep_is_not_offered_against_a_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["obe"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Gift of Sleep"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        act(0, 1, a, Target::None),
    );
}

/// Two textless Blood Brothers of one circle.
fn circle_mates() -> (u16, u16) {
    use zvtes::crypt::CryptCard;
    let bb = |c: &CryptCard| c.clan == "Blood Brother" && c.circle.is_some();
    let first = textless(bb, 0);
    let circle = CRYPT[first as usize].circle;
    let mate = textless(
        |c| bb(c) && c.circle == circle && c.name != CRYPT[first as usize].name,
        0,
    );
    (first, mate)
}

/// Seat 0's circle-mates `a` (acting) and `b`, seat 1's two locations and
/// a vampire; seat 0 holds Arson and Hay Ride.
fn hay_ride_table() -> (Engine, MinionId, MinionId, CardId, CardId) {
    let (x, y) = circle_mates();
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), x, 4, false);
    e.state.minions[a.index()].gained_levels = vec!["san"];
    let b = e.add_minion(SeatId(0), y, 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let l1 = in_play(&mut e, 1, "Art Museum", None);
    let l2 = in_play(&mut e, 1, "Arcane Library", None);
    hand(&mut e, 0, &["Arson", "Hay Ride"]);
    (e, a, b, l1, l2)
}

/// Hay Ride [san]: "Choose a Blood Brother of the same circle as this Blood
/// Brother. As your next action, the chosen Blood Brother can play the
/// action card used for this action from your ash heap as if from your
/// hand." After Arson, the chosen circle-mate plays it from the ash heap,
/// not the acting one. [LSJ 20081126] "Ignores all 'do not replace'
/// clauses": played from there, Arson replaces nothing, its delayed
/// replacement held for no one.
#[test]
fn hay_ride_lets_the_circle_replay_the_action_card() {
    let (mut e, a, b, l1, l2) = hay_ride_table();
    pick(&mut e, at_card(0, 0, a, l1));
    pick_when(&mut e, act(0, 0, a, Target::Minion(b)));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4 - 1, "Hay Ride's blood");
    assert!(!e.state.cards[l1.index()].present());
    assert!(req.options.contains(&at_card(0, 0, b, l2)), "{req:?}");
    assert!(!req.options.contains(&at_card(0, 0, a, l2)));
    let held = e.state.seats[0].held;
    pick(&mut e, at_card(0, 0, b, l2));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[l2.index()].present(), "the second Arson");
    assert_eq!(e.state.seats[0].held, held, "no replacement held");
    assert!(e.state.seats[0].hand.is_empty());
    assert!(e.state.seats[0].ash.contains(&find_library("Arson")));
}

/// [RTR 19980623] Hay Ride: "Must be played after combat if the action is
/// blocked": not in the block's window, offered once its combat is over.
#[test]
fn hay_ride_is_played_after_the_blocks_combat() {
    let (mut e, a, b, l1, _) = hay_ride_table();
    let z = MinionId(2);
    let ride = act(0, 0, a, Target::Minion(b));
    pick(&mut e, at_card(0, 0, a, l1));
    pick_when(&mut e, block(z));
    let req = offering(&mut e, ride);
    assert_eq!(req.window, WindowLabel::ActionAfterResolution);
    assert_eq!(blood(&e, z), 4 - 1, "the combat fought first");
    pick(&mut e, ride);
    request(&mut e);
    assert_eq!(
        e.state.seats[0].next_from_ash.0,
        Some((b, find_library("Arson")))
    );
}

/// Hay Ride "As your next action": not lost at the turn's end, the chosen
/// one may take it in the player's next turn; declaring another action
/// first loses it.
#[test]
fn hay_ride_waits_for_the_players_next_action() {
    let (mut e, a, b, l1, l2) = hay_ride_table();
    e.state.minions[b.index()].locked = true;
    pick(&mut e, at_card(0, 0, a, l1));
    pick_when(&mut e, act(0, 0, a, Target::Minion(b)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(req.options.contains(&at_card(0, 0, b, l2)), "next turn");
    pick(&mut e, action(a, ActionKind::Hunt));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!req.options.contains(&at_card(0, 0, b, l2)), "lost");
    assert_eq!(e.state.seats[0].next_from_ash.0, None);
}

/// Hay Ride [SAN]: "As above, but this card costs 1 fewer blood": free.
#[test]
fn hay_ride_superior_costs_nothing() {
    let (mut e, a, b, l1, _) = hay_ride_table();
    e.state.minions[a.index()].gained_levels = vec!["san", "san"];
    pick(&mut e, at_card(0, 0, a, l1));
    pick_when(&mut e, act(0, 1, a, Target::Minion(b)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4);
    assert_eq!(
        e.state.seats[0].next_from_ash.0,
        Some((b, find_library("Arson")))
    );
}

/// Din of the Damned: "Only usable on an undirected action as the action is
/// announced. Equipment and conviction cards cannot grant intercept this
/// action (any intercept they grant is lost). Reaction cards (and powers)
/// that grant intercept for this action cost an additional blood or life."
/// A Harbinger's hunt (+1 stealth): the blocker's Sport Bike +1 is lost, its
/// Aura Absorption [aus] (burn 1 blood for +1 intercept) costs it 1 blood
/// more; without Din the bike reaches the stealth alone. [ANK 20190425] [ANK 20181208] as the action
/// is announced: see `as_announced_modifiers_precede_other_modifiers`.
#[test]
fn din_of_the_damned_strips_equipment_intercept_and_taxes_intercept_reactions() {
    for din in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), harbinger(), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), vampire_with("aus", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        on_with(
            &mut e,
            1,
            "Sport Bike",
            z,
            &[zvtes::cards::Modifier::Bearer {
                stat: Stat::Intercept,
                n: 1,
                during: &[],
                who: zvtes::cards::Bearer::Any,
            }],
        );
        if din {
            hand(&mut e, 0, &["Din of the Damned"]);
        }
        hand(&mut e, 1, &["Aura Absorption"]);
        pick(&mut e, action(a, ActionKind::Hunt));
        if din {
            pick_when(&mut e, act(0, 0, a, Target::None));
        }
        pick_when(&mut e, block(z));
        let have = e.action().expect("hunt").intercept_of(&e.state, z);
        assert_eq!(have, if din { 0 } else { 1 }, "{din}");
        if din {
            pick_when(&mut e, act(0, 0, z, Target::None));
        }
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, a), 4 - 1, "blocked: combat, no hunt");
        let aura = if din { 1 + 1 } else { 0 };
        assert_eq!(blood(&e, z), 4 - 1 - aura, "{din}");
    }
}

/// Din of the Damned on a directed action (a bleed): not offered.
#[test]
fn din_of_the_damned_is_not_offered_on_a_directed_action() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), harbinger(), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Din of the Damned"]);
    pick(&mut e, bleed(a));
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        act(0, 0, a, Target::None),
    );
}

/// Din of the Damned's "(and powers)": an imbued's Discern, its +1
/// intercept a power's reaction, costs 1 life more under Din.
#[test]
fn din_of_the_damned_taxes_a_powers_reaction_in_life() {
    for din in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), harbinger(), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let c = find_card("Marion \"Teacher193\" Perks");
        let life = CRYPT[c as usize].capacity;
        let m = e.add_minion(SeatId(1), c, life, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let d = in_play(&mut e, 1, "Discern", Some(m));
        if din {
            hand(&mut e, 0, &["Din of the Damned"]);
        }
        pick(&mut e, action(a, ActionKind::Hunt));
        if din {
            pick_when(&mut e, act(0, 0, a, Target::None));
        }
        pick_when(&mut e, block(m));
        pick_when(&mut e, use_on(d, 0, Target::None));
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, a), 4 - 1, "blocked");
        assert_eq!(blood(&e, m), life - 1 - u8::from(din), "{din}");
    }
}

/// [ANK 20221205] [ANK 20220118] [RTR 20180719] [LSJ 20041022] "If the
/// action fails, modifiers and reactions that can be used at the end/after
/// an action can still be used afterwards": Hide the Heart [val] fails a
/// Blood Brother's Arson (it does not pay), and Hay Ride ("successful or
/// not") is still played after its resolution.
#[test]
fn hay_ride_follows_a_failed_action() {
    let (mut e, a, b, l1, _) = hay_ride_table();
    let r = e.add_minion(SeatId(1), vampire_with("val", 0), 4, false);
    hand(&mut e, 1, &["Hide the Heart"]);
    pick(&mut e, at_card(0, 0, a, l1));
    pick_when(&mut e, act(0, 1, r, Target::None));
    pick_when(&mut e, Choice::Pass);
    let ride = act(0, 0, a, Target::Minion(b));
    let req = offering(&mut e, ride);
    assert_eq!(req.window, WindowLabel::ActionAfterResolution);
    assert!(e.state.cards[l1.index()].present(), "the Arson failed");
    pick(&mut e, ride);
    request(&mut e);
    assert_eq!(
        e.state.seats[0].next_from_ash.0,
        Some((b, find_library("Arson")))
    );
}

/// Seat 0's `a` bleeds with +1 stealth (Elder Impersonation) and seat 1's
/// `z` blocks: the block window after the stealth.
fn stealthy_bleed_blocked(e: &mut Engine, a: MinionId, z: MinionId) {
    hand(e, 0, &["Elder Impersonation"]);
    pick(e, bleed(a));
    pick_when(e, block(z));
    pick_when(e, act(0, 0, a, Target::None));
}

/// Patrol: "+1 intercept." (Flight.) A flying Gargoyle meets a +1 stealth
/// bleed with it and blocks; a vampire without flight is not offered it.
#[test]
fn patrol_needs_flight() {
    for flyer in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("obf", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = if flyer {
            e.add_minion(SeatId(1), slave_of("Tremere"), 1, false)
        } else {
            e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false)
        };
        assert_eq!(e.state.has_discipline(z, "flight"), flyer);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 1, &["Patrol"]);
        stealthy_bleed_blocked(&mut e, a, z);
        let patrol = act(0, 0, z, Target::None);
        if flyer {
            pick_when(&mut e, patrol);
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(e.state.seats[1].pool, 10, "blocked");
        } else {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, patrol);
            assert_eq!(e.state.seats[1].pool, 10 - 1, "not blocked");
        }
    }
}

/// Steadfastness: "[for] Only usable during an action directed at you (or a
/// card you control). +1 intercept. [FOR] Reduce a bleed against you by 1."
/// [for] blocks a +1 stealth bleed at its Methuselah; [FOR] takes 1 off a
/// bleed of 1, with no intercept ("as above" absent: its block falls short).
#[test]
fn steadfastness_intercepts_at_you_and_reduces_a_bleed() {
    for superior in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("obf", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), vampire_with("for", 0), 4, false);
        if superior {
            e.state.minions[z.index()].gained_levels = vec!["for"];
        }
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 1, &["Steadfastness"]);
        stealthy_bleed_blocked(&mut e, a, z);
        if superior {
            pick_when(&mut e, act(0, 1, z, Target::None));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(e.state.seats[1].pool, 10, "bled for 1 - 1, unblocked");
            assert!(!e.state.minions[z.index()].locked, "no block");
        } else {
            pick_when(&mut e, act(0, 0, z, Target::None));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(e.state.seats[1].pool, 10, "blocked");
            assert!(e.state.minions[z.index()].locked, "the blocker locked");
        }
    }
}

/// Stiff Contempt (Samedi): "Only usable as the action is announced.
/// Vampires must burn 1 blood to attempt to block this action." A vampire
/// pays 1 blood to attempt; an ally attempts for free (Ni Dieu ni Maître
/// [for]'s reading). [ANK 20190425] [ANK 20181208]: see
/// `as_announced_modifiers_precede_other_modifiers`.
#[test]
fn stiff_contempt_makes_vampires_pay_to_block() {
    for ally in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Samedi", 0), 4, false);
        e.add_minion(SeatId(0), plain("Samedi", 1), 4, false);
        let z = if ally {
            e.add_ally(SeatId(1), find_library("Carlton Van Wyk"))
        } else {
            e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false)
        };
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let before = blood(&e, z);
        hand(&mut e, 0, &["Stiff Contempt"]);
        pick(&mut e, bleed(a));
        pick(&mut e, act(0, 0, a, Target::None));
        pick_when(&mut e, block(z));
        assert_eq!(
            blood(&e, z),
            if ally { before } else { before - 1 },
            "{ally}"
        );
    }
}

/// Virtuosa: "[pre] +1 stealth. [mel] Only usable during a bleed action. +1
/// stealth and +2 bleed (limited), or +2 bleed (limited). [MEL] Only usable
/// as the action is announced. This action is unblockable." (3 blood.)
/// [MEL]: no attempt offered. [mel]: the bleed-only half before any attempt,
/// the stealth one not then (rules-qa 8.1); the bleed for 3.
/// [ANK 20190425] [ANK 20181208]: see
/// `as_announced_modifiers_precede_other_modifiers`; [PIB 20130704] played
/// at [mel], not a Presence card.
#[test]
fn virtuosa_bleeds_or_makes_the_action_unblockable() {
    for superior in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), find_card("Evil Jensen"), 6, false);
        if superior {
            e.state.minions[a.index()].gained_levels = vec!["mel"];
        }
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Virtuosa"]);
        pick(&mut e, bleed(a));
        if superior {
            pick(&mut e, act(0, 3, a, Target::None));
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(z));
            assert_eq!(blood(&e, a), 6 - 3, "its cost");
            assert_eq!(e.state.seats[1].pool, 10 - 1);
        } else {
            let req = offering(&mut e, act(0, 2, a, Target::None));
            assert!(
                !req.options.contains(&act(0, 1, a, Target::None)),
                "no attempt"
            );
            pick(&mut e, act(0, 2, a, Target::None));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(e.state.seats[1].pool, 10 - 3, "bled for 1 + 2");
        }
    }
}

/// Voices of the Castle [vis]: "+1 intercept, or give +1 intercept to a
/// vampire to whom this vampire is enslaved." A Gargoyle slave of the
/// Tremere gives its +1 to its controller's attempting Tremere, who then
/// blocks a +1 stealth bleed; not to an attempting Ventrue (rules-qa 84.9:
/// a vampire of its controller's of the clan it is enslaved to).
/// [ANK 20210226] the [tha] blood: see `adana_de_sforza_does_not_reduce_a_burn_blood_effect`.
#[test]
fn voices_of_the_castle_gives_intercept_to_the_tremere_master() {
    for tremere in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("obf", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = if tremere {
            e.add_minion(SeatId(1), plain("Tremere", 0), 4, false)
        } else {
            e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false)
        };
        let g = e.add_minion(SeatId(1), slave_of("Tremere"), 1, false);
        assert!(e.state.discipline_level(g, "vis") > 0);
        hand(&mut e, 1, &["Voices of the Castle"]);
        stealthy_bleed_blocked(&mut e, a, z);
        let give = act(0, 2, g, Target::None);
        if tremere {
            pick_when(&mut e, give);
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(e.state.seats[1].pool, 10, "blocked");
            assert!(e.state.minions[z.index()].locked);
            assert!(
                !e.state.minions[g.index()].locked,
                "the slave did not block"
            );
        } else {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, give);
            assert_eq!(e.state.seats[1].pool, 10 - 1);
        }
    }
}

/// `n` Striga on vampire `m` of seat `seat`: [str], [STR] at 2.
fn striga(e: &mut Engine, seat: u8, m: MinionId, n: usize) {
    for _ in 0..n {
        on_with(
            e,
            seat,
            "Striga",
            m,
            &[zvtes::cards::Modifier::BearerDisciplineLevel("str")],
        );
    }
}

/// Scobax [str]: "The acting minion gets -1 stealth. [str] is not a
/// Discipline." [ANK 20200607] in a 3-player game seat 2's vampire plays it
/// in its turn of the sequence, after seat 1's attempt; [LSJ 20020612] no
/// attempt of its own needed: the +1 stealth bleed is blocked.
#[test]
fn scobax_strips_stealth_from_a_third_party() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), vampire_with("obf", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let s = e.add_minion(SeatId(2), plain("Toreador", 0), 4, false);
    striga(&mut e, 2, s, 1);
    hand(&mut e, 2, &["Scobax"]);
    stealthy_bleed_blocked(&mut e, a, z);
    pick_when(&mut e, act(0, 1, s, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10, "blocked");
    assert!(e.state.minions[z.index()].locked, "its block");
    assert!(!e.state.minions[s.index()].locked);
}

/// Scobax [STR]: "As [str] above, and the action fails unless the acting
/// minion burns 1 blood or life." An acting ally with 1 life pays and burns
/// (its action ends with it); a vampire not paying fails, paying bleeds.
/// [ANK 20211015] [RTR 20180719] [LSJ 20070219] failed, not blocked: see
/// `an_action_hide_or_champion_fails_is_spent`.
#[test]
fn scobax_superior_fails_unless_paid() {
    for (ally, pay) in [(true, true), (false, false), (false, true)] {
        let mut e = game(2);
        let a = if ally {
            let a = e.add_ally(SeatId(0), find_library("Carlton Van Wyk"));
            e.state.minions[a.index()].blood = 1;
            a
        } else {
            e.add_minion(SeatId(0), plain("Brujah", 0), 4, false)
        };
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let r = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        striga(&mut e, 1, r, 2);
        hand(&mut e, 1, &["Scobax"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, act(0, 2, r, Target::None));
        pick_when(
            &mut e,
            if pay {
                Choice::BurnBloodToAvoid
            } else {
                Choice::Pass
            },
        );
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        if ally {
            assert!(!e.state.minions[a.index()].present(), "its last life");
            assert_eq!(e.state.seats[1].pool, 10);
        } else {
            assert_eq!(blood(&e, a), if pay { 3 } else { 4 });
            assert_eq!(e.state.seats[1].pool, if pay { 9 } else { 10 }, "{pay}");
        }
    }
}

/// Scobax in a hand keeps a block in reach (docs/engine.md §3: third-party
/// stealth strips counted by `block_can_reach`): a 0-intercept vampire
/// against a +1 stealth hunt reaches it only with a [str] vampire and a
/// Scobax at hand.
#[test]
fn scobax_in_hand_keeps_a_block_in_reach() {
    for scobax in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 2, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let s = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        striga(&mut e, 1, s, 1);
        hand(&mut e, 1, &[if scobax { "Scobax" } else { "Patrol" }]);
        pick(&mut e, action(a, ActionKind::Hunt));
        request(&mut e);
        assert_eq!(e.block_can_reach(SeatId(1), z), scobax);
    }
}

/// Psalm of the Damned [mal]: "Choose a minion. The chosen minion cannot
/// play reaction cards (including [REACTION] abilities of power cards) this
/// action." [MAL]: "… and the chosen minion cannot block this action." The
/// chosen imbued's Discern is not offered on its block; at [MAL] its block
/// is not offered at all.
#[test]
fn psalm_of_the_damned_bars_reactions_and_powers() {
    for superior in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        maleficia(&mut e, 0, a, if superior { 2 } else { 1 });
        let c = find_card("Marion \"Teacher193\" Perks");
        let life = CRYPT[c as usize].capacity;
        let m = e.add_minion(SeatId(1), c, life, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let d = in_play(&mut e, 1, "Discern", Some(m));
        hand(&mut e, 0, &["Psalm of the Damned"]);
        pick(&mut e, action(a, ActionKind::Hunt));
        let mode = if superior { 2 } else { 1 };
        pick_when(&mut e, act(0, mode, a, Target::Minion(m)));
        if superior {
            never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, block(m));
        } else {
            pick_when(&mut e, block(m));
            never_offered_until(
                &mut e,
                SeatId(1),
                WindowLabel::MinionPhase,
                use_on(d, 0, Target::None),
            );
            assert_eq!(blood(&e, m), life, "no Discern");
        }
        assert_eq!(blood(&e, a), 4 + 1, "the hunt unblocked");
    }
}

/// Unleashing the Bestial Soul: "The chosen minion cannot play reaction
/// cards this action": a [REACTION] power is no reaction card (advanced
/// rules §1.5.1): the chosen imbued still uses Discern and blocks.
#[test]
fn unleashing_the_bestial_soul_leaves_powers_usable() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Salubri", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["dom"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let c = find_card("Marion \"Teacher193\" Perks");
    let life = CRYPT[c as usize].capacity;
    let m = e.add_minion(SeatId(1), c, life, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let d = in_play(&mut e, 1, "Discern", Some(m));
    hand(&mut e, 0, &["Unleashing the Bestial Soul"]);
    pick(&mut e, action(a, ActionKind::Hunt));
    pick_when(&mut e, act(0, 0, a, Target::Minion(m)));
    pick_when(&mut e, block(m));
    pick_when(&mut e, use_on(d, 0, Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, m), life - 1, "Discern's life");
    assert!(blood(&e, a) < 4 + 1, "blocked");
}

/// [ANK 20231028] Psalm of the Damned [mal]: "An opponent can play a
/// reaction to wake as the card is played, before its effects apply": the
/// locked chosen vampire wakes in the card's as-played window and blocks,
/// but its Steadfastness is not offered after; the +1 stealth bleed goes
/// through.
#[test]
fn psalm_of_the_damned_lets_a_wake_come_first() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obf", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    maleficia(&mut e, 0, a, 1);
    let z = e.add_minion(SeatId(1), vampire_with("for", 0), 4, false);
    e.state.minions[z.index()].locked = true;
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Psalm of the Damned", "Elder Impersonation"]);
    hand(&mut e, 1, &["On the Qui Vive", "Steadfastness"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, act(0, 1, a, Target::Minion(z)));
    pick_when(&mut e, act(0, 0, z, Target::None));
    pick_when(&mut e, block(z));
    pick_when(&mut e, act(0, 0, a, Target::None));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        act(0, 0, z, Target::None),
    );
    assert_eq!(e.state.seats[1].pool, 10 - 1, "not blocked");
}

/// Psalm of the Damned [pre]: "Only usable during a bleed action. Burn 1
/// pool to get +1 bleed (limited)." The pool an effect: the bleed for 2.
#[test]
fn psalm_of_the_damned_burns_pool_for_bleed() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("pre", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Psalm of the Damned"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, act(0, 0, a, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e), vec![10 - 1, 10 - 2]);
}

/// Safe Passage [obe]: "As [for] above, and reaction cards cost +1 blood or
/// life this action." [ANK 20180805] "The additional cost does not apply to
/// reaction powers": a Gargoyle's Patrol costs 1 blood, an imbued's Discern
/// nothing.
#[test]
fn safe_passage_taxes_reactions_not_powers() {
    for power in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.state.minions[a.index()].gained_levels = vec!["obe"];
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let c = find_card("Marion \"Teacher193\" Perks");
        let life = CRYPT[c as usize].capacity;
        let z = if power {
            e.add_minion(SeatId(1), c, life, false)
        } else {
            e.add_minion(SeatId(1), slave_of("Tremere"), 2, false)
        };
        let d = in_play(&mut e, 1, "Discern", Some(z));
        hand(&mut e, 0, &["Safe Passage"]);
        hand(&mut e, 1, &["Patrol"]);
        pick(&mut e, action(a, ActionKind::Hunt));
        pick_when(&mut e, act(0, 1, a, Target::None));
        pick_when(&mut e, block(z));
        if power {
            pick_when(&mut e, use_on(d, 0, Target::None));
        } else {
            pick_when(&mut e, act(0, 0, z, Target::None));
        }
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        // The imbued, a mortal ally (rules-digest §14.12), falls short
        // under the allies' -1; the Gargoyle blocks.
        assert_eq!(blood(&e, a) < 4 + 1, !power, "blocked");
        let start = if power { life } else { 2 };
        // Discern costs nothing (its card locks); Patrol's 1 blood.
        assert_eq!(start - blood(&e, z), u8::from(!power), "{power}");
    }
}

/// Safe Passage [OBE]: "As [obe] above, and put this card on this vampire.
/// Actions directed at this attached vampire cost +1 blood or life. Burn
/// this card during your unlock phase." The card itself on the vampire;
/// another Methuselah's Bum's Rush at it costs 1 blood more (against the
/// same game without it); burned in its controller's unlock phase.
#[test]
fn safe_passage_goes_on_the_vampire_and_taxes_directed_actions() {
    let mut paid = Vec::new();
    for sp in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.state.minions[a.index()].gained_levels = vec!["obe", "obe"];
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let r = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &[if sp { "Safe Passage" } else { "Patrol" }]);
        hand(&mut e, 1, &["Bum's Rush"]);
        pick(&mut e, action(a, ActionKind::Hunt));
        if sp {
            pick_when(&mut e, act(0, 2, a, Target::None));
            request(&mut e);
            let c = card_named(&e, "Safe Passage");
            assert_eq!(e.state.cards[c.index()].host, Some(a));
            assert!(!e.state.cards[c.index()].effect_only);
            assert!(!e.state.seats[0].ash.contains(&find_library("Safe Passage")));
        }
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        pick(&mut e, act(0, 0, r, Target::Minion(a)));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        paid.push(blood(&e, r));
        if sp {
            assert!(
                !e.state
                    .cards
                    .iter()
                    .any(|c| c.card == find_library("Safe Passage") && c.present()),
                "burned in its unlock phase"
            );
            assert!(e.state.seats[0].ash.contains(&find_library("Safe Passage")));
        }
    }
    assert_eq!(paid[1] - paid[0], 1, "1 blood more: {paid:?}");
}

/// [LSJ 20020612] Safe Passage [for]: "Allies get -1 intercept this action",
/// played with no block attempt yet: an ally's attempt at the 0-stealth
/// bleed falls short.
#[test]
fn safe_passage_lowers_allies_intercept_without_an_attempt() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("for", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Political Ally"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Safe Passage"]);
    pick(&mut e, bleed(a));
    pick(&mut e, act(0, 0, a, Target::None));
    pick_when(&mut e, block(ally));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 1, "not blocked");
}

/// [LSJ 20010813] [LSJ 20010819-2] Scry the Hearthstone [vis]: "As [tha]
/// above, with 1 optional maneuver during the resulting combat if this
/// vampire blocks": its +1 meets a +1 stealth bleed at its Methuselah and
/// the maneuver is offered in the block's combat; at [tha], none.
/// [LSJ 20010814-2] again on a second block: the same `IfBlocks` rider, see
/// `guard_dogs_maneuver_is_given_again_to_a_second_block`.
#[test]
fn scry_the_hearthstone_maneuvers_after_its_block() {
    for (disc, mode, rider) in [("vis", 1, true), ("tha", 0, false)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("obf", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), vampire_with(disc, 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 1, &["Scry the Hearthstone"]);
        stealthy_bleed_blocked(&mut e, a, z);
        pick_when(&mut e, act(0, mode, z, Target::None));
        let maneuver = offered_before(
            &mut e,
            Choice::UseManeuver,
            SeatId(0),
            WindowLabel::MinionPhase,
        );
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(maneuver, rider, "{disc}");
        assert_eq!(e.state.seats[1].pool, 10, "blocked");
    }
}

/// Scry the Hearthstone [VIS]: "Cancel an action card or action modifier
/// card requiring Chimerstry [chi] or Obfuscate [obf] as it is played, and
/// its cost is not paid." Elder Impersonation's +1 stealth on a bleed at its
/// Methuselah canceled: the 0-intercept attempt blocks. [PIB 20130704] a
/// card requiring Presence (Bonding's +1 bleed) is not offered it.
#[test]
fn scry_the_hearthstone_cancels_an_obfuscate_modifier() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obf", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["dom"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let r = e.add_minion(SeatId(1), vampire_with("vis", 0), 4, false);
    e.state.minions[r.index()].gained_levels = vec!["vis"];
    hand(&mut e, 0, &["Conditioning", "Elder Impersonation"]);
    hand(&mut e, 1, &["Scry the Hearthstone"]);
    let cancel = act(0, 3, r, Target::None);
    pick(&mut e, bleed(a));
    pick_when(&mut e, act(0, 0, a, Target::None));
    let req = request(&mut e);
    assert!(!req.options.contains(&cancel), "a Dominate card");
    pick_when(&mut e, block(z));
    pick_when(&mut e, act(0, 0, a, Target::None));
    pick_when(&mut e, cancel);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10, "blocked");
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Elder Impersonation")));
}

/// Treat the Sick Mind [dem]: "[ACTION] Unlock an ally or younger
/// vampire." Its own locked younger vampire unlocked; an older one is no
/// target.
#[test]
fn treat_the_sick_mind_unlocks_an_ally_or_younger_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity == 6, 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["dem"];
    let y = e.add_minion(SeatId(0), textless(|c| c.capacity <= 3, 0), 2, false);
    let o = e.add_minion(SeatId(0), textless(|c| c.capacity > 6, 0), 4, false);
    e.state.minions[y.index()].locked = true;
    e.state.minions[o.index()].locked = true;
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Treat the Sick Mind"]);
    let req = request(&mut e);
    assert!(
        !req.options.contains(&act(0, 0, a, Target::Minion(o))),
        "older"
    );
    pick(&mut e, act(0, 0, a, Target::Minion(y)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[y.index()].locked, "unlocked");
    assert!(e.state.minions[a.index()].locked);
}

/// Treat the Sick Mind [OBE]: "Cancel a card as it is played by another
/// Methuselah that would be put on a vampire with Dementation [dem] they do
/// not control. Usable even if there is no action." In seat 1's master
/// phase, its Haven Uncovered put on seat 0's Dementation vampire is
/// canceled by seat 0's [OBE] vampire, the card to the ash heap; on a
/// vampire without Dementation it is not offered. [PIB 20120808]
/// [RBK cancel-a-card] [ANK 20260216] no action needed, the canceled
/// card's cost still paid (none here: see
/// `power_of_all_outside_actions_and_not_in_combat`).
#[test]
fn treat_the_sick_mind_cancels_a_card_put_on_a_dementation_vampire() {
    for dem in [true, false] {
        let mut e = game(2);
        let d = if dem {
            e.add_minion(SeatId(0), vampire_with("dem", 0), 4, false)
        } else {
            e.add_minion(SeatId(0), plain("Brujah", 0), 4, false)
        };
        let r = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.state.minions[r.index()].gained_levels = vec!["obe", "obe"];
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 1, &["Haven Uncovered"]);
        hand(&mut e, 0, &["Treat the Sick Mind"]);
        until(&mut e, SeatId(1), WindowLabel::MasterPhase);
        pick(&mut e, master_on(0, Target::Minion(d)));
        let cancel = act(0, 2, r, Target::None);
        if dem {
            pick_when(&mut e, cancel);
            request(&mut e);
            assert!(e.state.seats[1]
                .ash
                .contains(&find_library("Haven Uncovered")));
            assert!(!e
                .state
                .cards
                .iter()
                .any(|c| c.card == find_library("Haven Uncovered") && c.present()));
        } else {
            never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, cancel);
            let c = card_named(&e, "Haven Uncovered");
            assert_eq!(e.state.cards[c.index()].host, Some(d), "put on");
        }
    }
}

/// [ANK 20200221] Strix [STR]: "Only usable when a bleed against you is
/// about to be successful. The bleed burns no pool (but is still
/// successful), this vampire is locked and enters combat with the acting
/// minion." "The bleed is considered successful and the Edge gained. The
/// action is resolved before entering combat": no pool burned, the Edge
/// taken, the Strix vampire locked, both hurt by the combat's hand strikes.
#[test]
fn strix_takes_the_bleed_into_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let s = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    striga(&mut e, 1, s, 2);
    hand(&mut e, 1, &["Strix"]);
    pick(&mut e, bleed(a));
    let req = offering(&mut e, act(0, 1, s, Target::None));
    assert_eq!(req.window, WindowLabel::ActionWouldBeSuccessful);
    pick(&mut e, act(0, 1, s, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10, "no pool burned");
    assert!(e.state.seats[0].has_edge, "still successful");
    assert!(e.state.minions[s.index()].locked);
    assert_eq!(blood(&e, s), 4 - 1 - 1, "its cost and a's hand strike");
    assert_eq!(blood(&e, a), 4 - 1, "s's hand strike");
}

/// Strix [str]: "+1 intercept, even if intercept is not yet needed. If this
/// vampire successfully blocks this action, he or she unlocks at the end of
/// the action." Played on a 0-stealth bleed (intercept not needed), the
/// blocker is unlocked once the action ends; without it, locked.
#[test]
fn strix_blocker_unlocks_at_the_end_of_the_action() {
    for strix in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let s = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        striga(&mut e, 1, s, 1);
        hand(&mut e, 1, &[if strix { "Strix" } else { "Patrol" }]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(s));
        if strix {
            pick_when(&mut e, act(0, 0, s, Target::None));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[1].pool, 10, "blocked");
        assert_eq!(e.state.minions[s.index()].locked, !strix, "{strix}");
    }
}

/// [LSJ 20091203] Strix: "The burn option can be used if all your vampires
/// with [str] are in torpor": discarded at the unlock phase with its only
/// [str] vampire in torpor; not with it ready.
#[test]
fn strix_burn_option_with_the_striga_in_torpor() {
    for torpid in [true, false] {
        let mut e = game(2);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let m = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, torpid);
        striga(&mut e, 1, m, 1);
        hand(&mut e, 1, &["Strix"]);
        e.state.seats[1].library = vec![find_library("Soak"); 2].into();
        if !torpid {
            never_offered_until(
                &mut e,
                SeatId(1),
                WindowLabel::MinionPhase,
                Choice::Discard(0),
            );
            continue;
        }
        until(&mut e, SeatId(1), WindowLabel::UnlockAbilities);
        pick(&mut e, Choice::Discard(0));
        assert!(e.state.seats[1].ash.contains(&find_library("Strix")));
    }
}

/// Momentary Delay's table: seat 0's capacity 7+ `a` (6 blood, capacity
/// `cap` or more) and a second minion, seat 1's lone capacity 2-4 (or 6)
/// `z` with 2 blood; seat 0's hand.
fn md_table(cap: u8, big_blocker: bool, cards: &[&str]) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| c.capacity >= cap && c.capacity <= 9, 0),
        6,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(
        SeatId(1),
        textless(
            |c| {
                if big_blocker {
                    c.capacity == 6
                } else {
                    (2..=4).contains(&c.capacity)
                }
            },
            0,
        ),
        2,
        false,
    );
    hand(&mut e, 0, cards);
    (e, a, z)
}

/// Momentary Delay: "Requires a vampire with capacity 7 or more. Only usable
/// after resolution of an action blocked by an ally or a vampire with
/// capacity 5 or less. … The action continues as if unblocked." After the
/// block's single round, the bleed goes on and seat 1 is bled.
/// [RTR 19941109] [ANK 20190116] [LSJ 20070808-1] the continued action's
/// modifiers, declines and card: see `continuation_keeps_modifiers`,
/// `continuation_keeps_declines`, `continued_action_card_leaves_the_ash_heap`.
#[test]
fn momentary_delay_continues_an_action_blocked_by_a_small_vampire() {
    let (mut e, a, z) = md_table(7, false, &["Momentary Delay"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    let req = offering(&mut e, play(0, 0, a));
    assert_eq!(req.window, WindowLabel::ActionAfterResolution);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 1, "continued: bled");
}

/// Momentary Delay: "Not usable if, during this action, more than 1 round of
/// combat has occurred or this vampire has lost 3 or more blood or ended
/// combat before the press step." Not offered after a pressed second round
/// (Boxed In), after its vampire paid Virtuosa's 3 blood (blood paid counts
/// as lost), after its Majesty ended the combat, against a capacity 6
/// blocker, nor to a capacity 6 vampire.
#[test]
fn momentary_delay_conditions_bar_it() {
    for case in [
        "two rounds",
        "blood paid",
        "ended early",
        "big blocker",
        "small",
    ] {
        let extra = match case {
            "two rounds" => "Boxed In",
            "blood paid" => "Virtuosa",
            "ended early" => "Majesty",
            _ => "Patrol",
        };
        let (mut e, a, z) = md_table(
            if case == "small" { 6 } else { 7 },
            case == "big blocker",
            &["Momentary Delay", extra],
        );
        if case == "small" {
            let v = &e.state.minions[a.index()];
            assert_eq!(v.vampire().map(|c| c.capacity), Some(6));
        }
        e.state.minions[a.index()].gained_levels = vec!["mel", "pre"];
        pick(&mut e, bleed(a));
        if case == "blood paid" {
            pick_when(&mut e, play(1, 2, a));
        }
        pick_when(&mut e, block(z));
        if case == "blood paid" {
            assert_eq!(blood(&e, a), 6 - 3, "Virtuosa's cost");
        }
        match case {
            "two rounds" => pick_when(&mut e, play(1, 0, a)),
            "ended early" => pick_when(&mut e, play(1, 0, a)),
            _ => {}
        }
        never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
        assert_eq!(e.state.seats[1].pool, 10, "{case}: blocked");
    }
}

/// [ANK 20200114] Momentary Delay "can be used if a Slave locked to enter
/// combat instead of the blocking minion": Tupdog takes its Tremere
/// antitribu's combat, the action goes on as if unblocked.
#[test]
fn momentary_delay_after_a_slave_takes_the_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Ladislas Toth, The Torch"), 6, false);
    let t = e.add_minion(SeatId(0), find_card("Tupdog"), 1, false);
    let z = e.add_minion(
        SeatId(1),
        textless(|c| (2..=4).contains(&c.capacity), 0),
        2,
        false,
    );
    hand(&mut e, 0, &["Momentary Delay"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Vampire(t),
            ability: 1,
            actor: None,
            target: Target::None,
        },
    );
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[t.index()].locked, "the slave fought");
    assert_eq!(e.state.seats[1].pool, 10 - 1, "continued: bled");
}

/// [LSJ 20030130] [ANK 20181219] [ANK 20180906] Momentary Delay "is played
/// after all combats are handled. Can be played from torpor": its vampire,
/// sent to torpor by the blocker's hand strike, still plays it.
#[test]
fn momentary_delay_from_torpor() {
    let (mut e, a, z) = md_table(7, false, &["Momentary Delay"]);
    pick(&mut e, bleed(a));
    e.state.minions[a.index()].blood = 0;
    pick_when(&mut e, block(z));
    let req = offering(&mut e, play(0, 0, a));
    assert_eq!(req.window, WindowLabel::ActionAfterResolution);
    assert!(e.state.minions[a.index()].in_torpor);
}

/// [RTR 19970630] "An action cannot continue \"as if unblocked\" after a
/// combat resulting from a successful action (e.g. Bum's Rush)": a rush
/// was never blocked, Momentary Delay is not offered after its combat.
#[test]
fn momentary_delay_does_not_continue_a_rush() {
    let (mut e, a, z) = md_table(7, false, &["Momentary Delay", "Bum's Rush"]);
    pick(&mut e, act(1, 0, a, Target::Minion(z)));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
    assert!(blood(&e, z) < 2, "the rush's combat");
}

/// [LSJ 19981028] [LSJ 20100206-2] Momentary Delay "is played after
/// resolution, but still during the action. Cards and effects that played
/// \"after resolution\" (eg. Freak Drive) can be played before it, but not
/// after": Freak Drive [FOR] unlocks first, Momentary Delay follows; once
/// the action goes on, Freak Drive's blocked mode is gone with the block.
#[test]
fn freak_drive_before_momentary_delay_not_after() {
    for first in [true, false] {
        let (mut e, a, z) = md_table(7, false, &["Momentary Delay", "Freak Drive"]);
        e.state.minions[a.index()].gained_levels = vec!["for", "for"];
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        if first {
            pick_when(&mut e, play(1, 1, a));
            pick_when(&mut e, play(0, 0, a));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        } else {
            pick_when(&mut e, play(0, 0, a));
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, a));
        }
        assert_eq!(e.state.seats[1].pool, 10 - 1, "continued: bled");
    }
}

// ---- Minions and equipment --------------------------------------------------

/// Walk to the request offering `want`, no option on the way matching
/// `barred`.
fn offered_without(
    e: &mut Engine,
    want: Choice,
    barred: impl Fn(&Choice) -> bool,
) -> zvtes::decision::DecisionRequest {
    for _ in 0..200 {
        let req = request(e);
        if req.options.contains(&want) {
            return req;
        }
        assert!(
            !req.options.iter().any(&barred),
            "{:?} {:?}",
            req.window,
            req.options
        );
        let i = fallback(&req);
        e.choose(i);
    }
    panic!("{want:?} never offered");
}

/// Any use of card `c`'s ability `ability`.
fn uses_ability(c: CardId, ability: u8) -> impl Fn(&Choice) -> bool {
    move |o| {
        matches!(o, Choice::UseAbility { source: AbilitySource::Card(s), ability: a, .. }
            if *s == c && *a == ability)
    }
}

/// [LSJ 20021028] "The optional maneuver cannot be used if the strike
/// cannot be used." Foldable Machine Gun: "This maneuver cannot be used
/// during the first round. Once each round, if the bearer strikes with this
/// gun, they get 1 optional additional strike (limited) this round, only
/// usable to strike with this gun." No maneuver in round 1, the gun's
/// additional strike there; pressed on, round 2 maneuvers to long range and
/// strikes twice: 4 damage in all.
#[test]
fn foldable_machine_gun_maneuvers_only_after_the_first_round() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 5, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let gun = on_with(
        &mut e,
        0,
        "Foldable Machine Gun",
        a,
        &[zvtes::cards::Modifier::HostCombatPress(1)],
    );
    blocked(&mut e, a, z);
    offered_without(&mut e, ability(gun.0, 0), uses_ability(gun, 1));
    pick(&mut e, ability(gun.0, 0));
    pick_when(&mut e, ability(gun.0, 0));
    pick_when(&mut e, Choice::UsePress);
    pick_when(&mut e, ability(gun.0, 1));
    // The committed initial strike elides; the additional one is optional.
    let req = offering(&mut e, ability(gun.0, 0));
    assert!(req.options.contains(&Choice::Pass), "the additional strike");
    pick(&mut e, ability(gun.0, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 5 - 4, "two strikes a round, two rounds");
}

/// [ANK 20180719-3] "Once in play, can be equipped from the minion it's on
/// … by another minion that does not meet the requirement." Shaal
/// Fragment: "+2 hand size. Any vampire can steal this equipment as a Ⓓ
/// action." Its Harbinger's controller holds 9 cards; its own other
/// vampire is not offered the steal; another Methuselah's vampire steals it
/// onto itself, the hand sizes 7 and 9.
#[test]
fn shaal_fragment_gives_hand_size_and_can_be_stolen() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), harbinger(), 4, false);
    let other = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let base = e.state.hand_size(SeatId(1));
    hand(&mut e, 0, &["Shaal Fragment"]);
    pick(&mut e, play(0, 0, h));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let frag = card_named(&e, "Shaal Fragment");
    assert_eq!(e.state.cards[frag.index()].host, Some(h));
    assert_eq!(e.state.hand_size(SeatId(0)), base + 2);
    let steal = |m| Choice::UseAbility {
        source: AbilitySource::Card(frag),
        ability: 0,
        actor: Some(m),
        target: Target::None,
    };
    assert!(!req.options.contains(&steal(other)), "not its own");
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(req.options.contains(&steal(v)));
    pick(&mut e, steal(v));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[frag.index()].host, Some(v), "on the thief");
    assert_eq!(e.state.hand_size(SeatId(1)), base + 2);
    assert_eq!(e.state.hand_size(SeatId(0)), base);
}

/// Razor Bat: "The Gargoyle with this retainer gets +1 intercept. When the
/// Gargoyle with this retainer is in combat, the opposing minion takes 1R
/// damage during the initial strike resolution phase of each round." Its
/// Gargoyle has +1 intercept; blocked, its opponent takes 1 more than
/// without the bat.
#[test]
fn razor_bat_gives_intercept_and_bites_each_round() {
    let mut lost = Vec::new();
    for bat in [false, true] {
        let mut e = game(2);
        let g = e.add_minion(SeatId(0), tremere_slave(), 4, false);
        // A ready Tremere: its slave may bleed (rulebook "Slave").
        e.add_minion(SeatId(0), plain("Tremere", 0), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 5, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        if bat {
            let b = on_with(
                &mut e,
                0,
                "Razor Bat",
                g,
                &[
                    zvtes::cards::Modifier::Bearer {
                        stat: Stat::Intercept,
                        n: 1,
                        during: &[],
                        who: zvtes::cards::Bearer::Clan("Gargoyle"),
                    },
                    zvtes::cards::Modifier::RetainerDamage(1),
                ],
            );
            e.state.cards[b.index()].counters = 2;
            assert_eq!(e.state.bearer_mod(g, Stat::Intercept, ActionKind::Bleed), 1);
        }
        blocked(&mut e, g, z);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        lost.push(5 - blood(&e, z));
    }
    assert_eq!(lost[1], lost[0] + 1, "the bat's 1R");
}

/// A Nagaraja with room for a hunt (Le Dinh Tho: capacity 5, his text
/// inert here).
fn nagaraja() -> u16 {
    find_card("Le Dinh Tho")
}

/// [RTR 20030519] [RTR 20180511] "The hunt bonus applies to non-standard
/// hunt actions, blood comes from the target of the hunt." Ritual Scalpel:
/// "This Nagaraja gets +1 hunt. This Nagaraja can inflict 1 unpreventable
/// damage on a mortal (ally or retainer) to gain 2 blood as a Ⓓ action."
/// Equipped, its Nagaraja hunts for 2; then, in later games, it cuts
/// another Methuselah's mortal ally (1 life lost, 2 blood) or mortal
/// retainer, never a non-mortal ally nor a Ghouled one ("considered a
/// ghoul and not a mortal").
#[test]
fn ritual_scalpel_feeds_a_nagaraja_on_mortals() {
    let setup = || {
        let mut e = game(2);
        let n = e.add_minion(SeatId(0), nagaraja(), 1, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Ritual Scalpel"]);
        pick(&mut e, play(0, 0, n));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let s = card_named(&e, "Ritual Scalpel");
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        (e, n, z, s)
    };
    let (mut e, n, _, _) = setup();
    pick(&mut e, action(n, ActionKind::Hunt));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, n), 1 + 2, "+1 hunt");
    let cut = |s: CardId, ability: u8, n: MinionId, target: Target| Choice::UseAbility {
        source: AbilitySource::Card(s),
        ability,
        actor: Some(n),
        target,
    };
    let (mut e, n, _, s) = setup();
    let ally = e.add_ally(SeatId(1), find_library("47th Street Royals"));
    let cat = e.add_ally(SeatId(1), find_library("Rock Cat"));
    let ghoul = e.add_ally(SeatId(1), find_library("Political Ally"));
    on_with(
        &mut e,
        1,
        "Ghouled",
        ghoul,
        &[zvtes::cards::Modifier::BearerSubtypeInstead {
            is: zvtes::library::Label::Ghoul,
            not: zvtes::library::Label::Mortal,
        }],
    );
    let req = request(&mut e);
    assert!(
        !req.options.contains(&cut(s, 0, n, Target::Minion(cat))),
        "no mortal"
    );
    assert!(
        !req.options.contains(&cut(s, 0, n, Target::Minion(ghoul))),
        "a ghoul and not a mortal"
    );
    pick(&mut e, cut(s, 0, n, Target::Minion(ally)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, ally), 1, "1 damage");
    assert_eq!(blood(&e, n), 1 + 2);
    let (mut e, n, z, s) = setup();
    let r = in_play(&mut e, 1, "Mr. Winthrop", Some(z));
    e.state.cards[r.index()].counters = 2;
    pick(&mut e, cut(s, 1, n, Target::Card(r)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[r.index()].counters, 1, "1 life lost");
    assert_eq!(blood(&e, n), 1 + 2);
}

/// [ANK 20180719-3] Taken by a non-Nagaraja, the Scalpel gives neither the
/// hunt nor the action: "this Nagaraja" is its current bearer.
#[test]
fn ritual_scalpel_gives_a_non_nagaraja_nothing() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), nagaraja(), 2, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 1, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("47th Street Royals"));
    hand(&mut e, 0, &["Ritual Scalpel"]);
    pick(&mut e, play(0, 0, n));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let s = card_named(&e, "Ritual Scalpel");
    e.state.cards[s.index()].host = Some(b);
    let req = request(&mut e);
    assert!(!req.options.iter().any(uses_ability(s, 0)), "no action");
    let _ = ally;
    pick(&mut e, action(b, ActionKind::Hunt));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, b), 1 + 1, "no bonus");
}

/// [PIB 20130303] Torn Signpost "sets base strength. It can reduce the base
/// strength of a minion (eg. Rock Cat)." [LSJ 20001126] "Minions that
/// cannot strike, cannot play or use the maneuver from a strike." Rock Cat:
/// "Opposing vampires with capacity 3 or less cannot strike in the first
/// round. Rock Cat gets an optional press each combat. Rock Cat may play
/// cards requiring basic Potence [pot] as a 3-capacity vampire." It enters
/// combat with a capacity-3 vampire holding a .44 Magnum, plays Torn
/// Signpost [pot] (strength 2): round 1, no Magnum maneuver nor strike,
/// 2 damage; it presses on and round 2 the Magnum strikes. A capacity-4
/// vampire strikes in round 1.
#[test]
fn rock_cat_bars_small_vampires_first_round_strikes() {
    for cap in [3, 4] {
        let mut e = game(2);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let rc = e.add_ally(SeatId(0), find_library("Rock Cat"));
        let z = e.add_minion(SeatId(1), textless(|c| c.capacity == cap, 0), cap, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let magnum = on_with(&mut e, 1, ".44 Magnum", z, &[]);
        hand(&mut e, 0, &["Torn Signpost"]);
        let cat = card_named(&e, "Rock Cat");
        pick(
            &mut e,
            Choice::UseAbility {
                source: AbilitySource::Card(cat),
                ability: 0,
                actor: Some(rc),
                target: Target::Minion(z),
            },
        );
        pick_when(&mut e, play(0, 0, rc));
        if cap == 4 {
            offering(&mut e, ability(magnum.0, 0));
            continue;
        }
        let magnum_used = |o: &Choice| matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), .. } if *c == magnum);
        offered_without(&mut e, Choice::UsePress, magnum_used);
        assert_eq!(blood(&e, z), cap - 2, "strength 2");
        pick(&mut e, Choice::UsePress);
        offering(&mut e, ability(magnum.0, 0));
    }
}

/// The use of card `c`'s ability 0 at `target`, whoever offers it.
fn use_at(req: &zvtes::decision::DecisionRequest, c: CardId, target: Target) -> Option<Choice> {
    req.options.iter().copied().find(|o| {
        matches!(o, Choice::UseAbility { source: AbilitySource::Card(s), ability: 0, target: t, .. }
            if *s == c && *t == target)
    })
}

/// [ANK 20180517] "They can lock to use their ability the turn they where
/// recruited." Draeven Softfoot: "Draeven can lock to prevent 1 damage
/// inflicted in combat on a Kiasyd." Recruited this turn, he locks to spare
/// another Methuselah's Kiasyd 1 damage in a combat he is not in, never its
/// non-Kiasyd opponent.
#[test]
fn draeven_softfoot_locks_to_prevent_damage_on_any_kiasyd() {
    let mut lost = Vec::new();
    for draeven in [false, true] {
        let mut e = game(3);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let d = e.add_ally(SeatId(0), find_library("Draeven Softfoot"));
        e.state.minions[d.index()].recruited = true;
        let k = e.add_minion(SeatId(1), of_clan("Kiasyd", 0), 5, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let w = e.add_minion(SeatId(2), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
        let card = card_named(&e, "Draeven Softfoot");
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        blocked(&mut e, k, w);
        if draeven {
            let req = offering(&mut e, use_on(card, 0, Target::Minion(k)));
            assert!(use_at(&req, card, Target::Minion(w)).is_none(), "no Kiasyd");
            pick(
                &mut e,
                use_at(&req, card, Target::Minion(k)).expect("offered"),
            );
            assert!(e.state.minions[d.index()].locked);
        }
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        lost.push(5 - blood(&e, k));
    }
    assert_eq!(lost[1] + 1, lost[0], "1 prevented");
}

/// Draeven Softfoot: "Cards requiring Chimerstry [chi] can neither be
/// played while he is acting, blocking, or in combat, nor target him." The
/// [chi] vampire he blocks is never offered Hall of Mirrors [chi] in their
/// combat; against a vampire blocker it is.
#[test]
fn draeven_softfoot_shuns_chimerstry() {
    for draeven in [false, true] {
        let mut e = game(2);
        let v = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let d = e.add_ally(SeatId(0), find_library("Draeven Softfoot"));
        let c = e.add_minion(SeatId(1), vampire_with("chi", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 1, &["Hall of Mirrors"]);
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        pick(&mut e, bleed(c));
        if draeven {
            pick_when(&mut e, block(d));
            never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, play(0, 0, c));
        } else {
            pick_when(&mut e, block(v));
            offering(&mut e, play(0, 0, c));
        }
    }
}

/// Member of the Entourage: "This ally cannot perform actions. During an
/// action directed at you (or a card you control), this ally can burn 1
/// life to get +1 intercept. Burn this ally if he blocks a vampire, before
/// block resolution." No action of its own; a +1 stealth bleed at its
/// Methuselah: it burns a life for the intercept, once, blocks, and burns
/// as it does — the bleed blocked, with no combat.
#[test]
fn member_of_the_entourage_buys_intercept_and_burns_blocking_a_vampire() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let mem = e.add_ally(SeatId(0), find_library("Member of the Entourage"));
    let c = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    on_with(
        &mut e,
        1,
        "Foldable Machine Gun",
        c,
        &[zvtes::cards::Modifier::Bearer {
            stat: Stat::Stealth,
            n: 1,
            during: &[],
            who: zvtes::cards::Bearer::Any,
        }],
    );
    let card = card_named(&e, "Member of the Entourage");
    e.notes = Some(Vec::new());
    let req = request(&mut e);
    assert!(
        !req.options
            .iter()
            .any(|o| matches!(o, Choice::TakeAction { minion, .. } if *minion == mem)),
        "cannot act"
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(c));
    pick_when(&mut e, block(mem));
    let buy = ability(card.0, 0);
    pick_when(&mut e, buy);
    assert_eq!(blood(&e, mem), 1, "1 life");
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(
        !e.state.minions[mem.index()].in_play,
        "burned blocking a vampire"
    );
    assert_eq!(e.state.seats[0].pool, 10, "blocked");
    assert!(noted_combats(&mut e).is_empty(), "no combat");
}

/// Blocking an ally, Member of the Entourage stays: the combat follows.
#[test]
fn member_of_the_entourage_blocks_an_ally_into_combat() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let mem = e.add_ally(SeatId(0), find_library("Member of the Entourage"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let rc = e.add_ally(SeatId(1), find_library("Rock Cat"));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    e.notes = Some(Vec::new());
    let cat = card_named(&e, "Rock Cat");
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Card(cat),
            ability: 0,
            actor: Some(rc),
            target: Target::Minion(v),
        },
    );
    pick_when(&mut e, block(mem));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(
        noted_combats(&mut e).iter().any(|n| matches!(n,
            zvtes::combat::CombatNote::Begins { minions } if minions.contains(&mem))),
        "its combat"
    );
}

/// [LSJ 20100207-2] Journal of Hrorsh "Equipping it does not put a counter
/// on it." "Add 1 counter to this card after this Salubri performs a
/// successful action. … You can remove this ready Salubri from the game to
/// move these counters to your pool, except during an action." The equip
/// gives none, a successful bleed one, a blocked bleed none; removed, its
/// Salubri gives the counter as pool, out of the game, the Journal burned.
#[test]
fn journal_of_hrorsh_counts_successful_actions_but_not_its_equip() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), of_clan("Salubri", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Journal of Hrorsh"]);
    pick(&mut e, play(0, 0, s));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let j = card_named(&e, "Journal of Hrorsh");
    assert_eq!(e.state.cards[j.index()].counters, 0, "not for its equip");
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, bleed(s));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[j.index()].counters, 1, "a successful bleed");
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    blocked(&mut e, s, z);
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.cards[j.index()].counters,
        1,
        "not for a blocked one"
    );
    let pool = e.state.seats[0].pool;
    assert!(req.options.contains(&ability(j.0, 0)));
    pick(&mut e, ability(j.0, 0));
    request(&mut e);
    assert_eq!(e.state.seats[0].pool, pool + 1);
    assert!(!e.state.minions[s.index()].in_play);
    assert!(e.state.seats[0]
        .removed_crypt
        .contains(&e.state.minions[s.index()].crypt_card().expect("a vampire")));
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Journal of Hrorsh")));
}

/// [ANK 20180719-3] Taken by a non-Salubri, the Journal loses its counters
/// ("burn these counters if this card is moved"); its new bearer's
/// successful action adds none, and it cannot remove itself.
#[test]
fn journal_of_hrorsh_moved_loses_its_counters() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), of_clan("Salubri", 0), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Journal of Hrorsh"]);
    pick(&mut e, play(0, 0, s));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let j = card_named(&e, "Journal of Hrorsh");
    e.state.cards[j.index()].counters = 2;
    pick(&mut e, take_from(b, s));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[j.index()].host, Some(b));
    assert_eq!(e.state.cards[j.index()].counters, 0, "burned by the move");
    assert!(!req.options.contains(&ability(j.0, 0)), "no Salubri");
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, bleed(b));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.cards[j.index()].counters,
        0,
        "not a Salubri's action"
    );
}

/// [ANK 20190203] "Only requires one clan or the other." [LSJ 19971215]
/// "Using the Jar action does not force the minion to strike with it if he
/// is blocked." Jar of Skin Eaters: "The bearer may use this weapon only if
/// it has a blood on it. … This minion may inflict 3 aggravated damage on
/// another minion as a Ⓓ action. Burn after either use. Any Gargoyle
/// enslaved to this vampire may move 1 blood to this card as a +1 stealth
/// action." A Tremere antitribu equips it; empty, its action is not
/// offered; its slave Gargoyle moves a blood onto it; blocked, the action
/// leaves the Jar; unblocked, 3 aggravated damage sends the target to
/// torpor and the Jar burns.
#[test]
fn jar_of_skin_eaters_needs_a_blood_and_burns_after_use() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), of_clan("Tremere antitribu", 0), 5, false);
    let g = e.add_minion(SeatId(0), slave_of("Tremere antitribu"), 3, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 3, false);
    let y = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Jar of Skin Eaters"]);
    pick(&mut e, play(0, 0, t));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let jar = card_named(&e, "Jar of Skin Eaters");
    assert!(
        !req.options.iter().any(uses_ability(jar, 1)),
        "no blood on it"
    );
    let feed = Choice::UseAbility {
        source: AbilitySource::Card(jar),
        ability: 2,
        actor: Some(g),
        target: Target::None,
    };
    pick(&mut e, feed);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[jar.index()].counters, 1);
    assert_eq!(blood(&e, g), 2);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let throw = |target| Choice::UseAbility {
        source: AbilitySource::Card(jar),
        ability: 1,
        actor: Some(t),
        target: Target::Minion(target),
    };
    pick(&mut e, throw(z));
    pick_when(&mut e, block(y));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.cards[jar.index()].present(), "blocked: kept");
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, throw(z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[z.index()].in_torpor, "3 aggravated");
    assert!(!e.state.cards[jar.index()].present(), "burned after use");
}

/// [LSJ 20001127-2] [LSJ 20010806-1] [LSJ 19981006] "When used as a
/// strike, it does not burn if combat ends before it resolves (eg. if the
/// opponent uses a \"strike: combat ends\")." Jar of Skin Eaters' strike,
/// "3R aggravated damage", with a blood on it, burns after use; without
/// one it is not offered; against Majesty's combat ends, it stays.
#[test]
fn jar_of_skin_eaters_strikes_only_with_a_blood_and_burns() {
    for (counters, majesty) in [(0, false), (1, false), (1, true)] {
        let mut e = game(2);
        let t = e.add_minion(SeatId(0), of_clan("Tremere", 0), 5, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), vampire_with("pre", 0), 5, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let jar = on_with(&mut e, 0, "Jar of Skin Eaters", t, &[]);
        e.state.cards[jar.index()].counters = counters;
        if majesty {
            hand(&mut e, 1, &["Majesty"]);
        }
        blocked(&mut e, t, z);
        if counters == 0 {
            never_offered_until(
                &mut e,
                SeatId(0),
                WindowLabel::MinionPhase,
                ability(jar.0, 0),
            );
            continue;
        }
        pick_when(&mut e, ability(jar.0, 0));
        if majesty {
            pick_when(&mut e, play(0, 0, z));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(
            e.state.cards[jar.index()].present(),
            majesty,
            "burned after use"
        );
        assert_eq!(
            e.state.minions[z.index()].in_torpor,
            !majesty,
            "3 aggravated"
        );
    }
}

/// [ANK 20190104] "The canceled card has still been played." Joumlon's
/// Axe: "Once each combat, the bearer can cancel a grapple card played by
/// the opposing minion as it is played, its cost is not paid, and the
/// bearer's initial strike this round must be or have been with this
/// weapon." The opposing vampire's Immortal Grapple canceled: the round's
/// strike is the Axe's (strength+1: 2 damage); pressed on, its second
/// Immortal Grapple cannot be canceled.
#[test]
fn joumlons_axe_cancels_a_grapple_and_commits_its_strike() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("pot", 0), 5, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let axe = on_with(
        &mut e,
        0,
        "Joumlon's Axe",
        a,
        &[zvtes::cards::Modifier::HostCombatPress(1)],
    );
    hand(&mut e, 1, &["Immortal Grapple", "Immortal Grapple"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(z));
    pick_when(&mut e, block(a));
    pick_when(&mut e, play(0, 0, z));
    pick_when(&mut e, ability(axe.0, 1));
    pick_when(&mut e, Choice::UsePress);
    assert_eq!(blood(&e, z), 5 - 2, "the Axe's strike");
    pick_when(&mut e, play(0, 0, z));
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        ability(axe.0, 1),
    );
}

/// Joumlon's Axe: "(not usable otherwise)". The opposing minion's
/// Shoulder Drop, a grapple after the strikes: cancelable when the round's
/// initial strike was the Axe's, not after a hand strike.
#[test]
fn joumlons_axe_cancel_needs_its_strike_this_round() {
    for with_axe in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 5, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 5, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let axe = on_with(&mut e, 0, "Joumlon's Axe", a, &[]);
        hand(&mut e, 1, &["Shoulder Drop"]);
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        pick(&mut e, bleed(z));
        pick_when(&mut e, block(a));
        pick_when(
            &mut e,
            if with_axe {
                ability(axe.0, 0)
            } else {
                Choice::HandStrike
            },
        );
        pick_when(&mut e, play(0, 0, z));
        if with_axe {
            offering(&mut e, ability(axe.0, 1));
        } else {
            never_offered_until(
                &mut e,
                SeatId(1),
                WindowLabel::MinionPhase,
                ability(axe.0, 1),
            );
        }
    }
}

/// [LSJ 20021028] Tinglestripe: "This weapon can be used to strike only
/// once each round. [myt] Strike: 2R damage. [MYT] As above, or strike: 1R
/// damage, with 1 optional maneuver each combat." At [myt], the 2R strike
/// and no maneuver; at [MYT], the maneuver to long range commits the 1R
/// strike (1 damage), and pressed on, round 2 offers both strikes but no
/// second maneuver.
#[test]
fn tinglestripe_strikes_once_a_round_and_superior_maneuvers_for_its_1r() {
    for superior in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 5, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 5, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let t = on_with(
            &mut e,
            0,
            "Tinglestripe",
            a,
            &[zvtes::cards::Modifier::HostCombatPress(1)],
        );
        e.state.cards[t.index()].mode = superior as u8;
        blocked(&mut e, a, z);
        if !superior {
            let req = offered_without(&mut e, ability(t.0, 0), uses_ability(t, 2));
            assert!(!req.options.contains(&ability(t.0, 1)), "no 1R at [myt]");
            continue;
        }
        pick_when(&mut e, ability(t.0, 2));
        pick_when(&mut e, Choice::UsePress);
        assert_eq!(blood(&e, z), 5 - 1, "the 1R strike, committed");
        let req = offered_without(&mut e, ability(t.0, 0), uses_ability(t, 2));
        assert!(req.options.contains(&ability(t.0, 1)));
    }
}

/// [LSJ 20030502] "X can be zero." Reanimated Corpse: "[thn] Put X pathos
/// counters on this corpse when it enters play. During your unlock phase,
/// burn 1 pathos counter. Remove the corpse from the game if it has no
/// pathos counters. … [THN] As above, but put 2 additional pathos
/// counters." Recruited at X = 1: one counter, gone from the game (not the
/// ash heap) at its next unlock phase; at [THN] with X = 0: two.
#[test]
fn reanimated_corpse_counts_down_pathos_and_leaves_the_game() {
    for superior in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(
            SeatId(0),
            vampire_with(if superior { "THN" } else { "thn" }, 0),
            4,
            false,
        );
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Reanimated Corpse"]);
        pick(&mut e, play(0, superior as u8, a));
        pick_when(&mut e, set_x(if superior { 0 } else { 1 }));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let card = card_named(&e, "Reanimated Corpse");
        let corpse = e.state.cards[card.index()].body.expect("an ally");
        assert_eq!(
            e.state.cards[card.index()].counters,
            if superior { 2 } else { 1 }
        );
        assert_eq!(blood(&e, a), 4 - if superior { 0 } else { 1 }, "X blood");
        if superior {
            continue;
        }
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(!e.state.minions[corpse.index()].in_play, "out of the game");
        assert!(!e.state.seats[0]
            .ash
            .contains(&find_library("Reanimated Corpse")));
    }
}

/// [LSJ 20071015] "If it is brought back with X being zero … it is
/// burned immediately." [PIB 20111101] [LSJ 20100204] [LSJ 20040518-2]
/// [LSJ 20100302-1] Put in play other
/// than by recruiting, X is 0. [ANK 20170309-2] Its enter-play effect
/// applies however it comes into play. Put in play, the corpse has no
/// counter and leaves the game at once.
#[test]
fn reanimated_corpse_put_in_play_leaves_at_once() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let corpse = e.add_ally(SeatId(0), find_library("Reanimated Corpse"));
    assert!(!e.state.minions[corpse.index()].in_play);
    assert!(!e.state.seats[0]
        .ash
        .contains(&find_library("Reanimated Corpse")));
}

/// [ANK 20180111] [LSJ 20011217] [LSJ 20090508] [LSJ 19980303] Reanimated
/// Corpse: "The corpse can play
/// combat cards that require basic Fortitude [for] as a vampire." Soak and
/// Touch of Valeren's combat mode at [for]; never [FOR], Touch of
/// Valeren's action mode, Daring the Dawn (an action modifier) nor
/// Martyr's Resilience (Auspex and Fortitude); it does not have Fortitude.
#[test]
fn reanimated_corpse_plays_basic_fortitude_combat_cards_only() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("thn", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Reanimated Corpse"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, set_x(2));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let c = e.state.cards[card_named(&e, "Reanimated Corpse").index()]
        .body
        .expect("an ally");
    let has = |name: &str, mode: usize| {
        zvtes::cards::has_disciplines(&e.state, c, find_library(name), mode)
    };
    assert!(has("Soak", 0));
    assert!(!has("Soak", 1), "[FOR]");
    assert!(has("Touch of Valeren", 1), "its combat mode");
    assert!(!has("Touch of Valeren", 0), "its action mode");
    assert!(!has("Daring the Dawn", 0), "an action modifier");
    assert!(!has("Martyr's Resilience", 0), "one requirement only");
    assert!(!e.state.has_discipline(c, "for"));
}

/// [ANK 20180719-4] "Once each turn" means once during each player's turn.
/// Amulet of Temporal Perception: "This vampire can burn 1 blood to play a
/// card requiring Temporis [tem] as if they had basic Temporis. This
/// equipment can be used only once each turn." Blocking in its predator's
/// turn, its vampire plays Lapse [tem] for 2 + 1 blood, never [TEM], and
/// no second Lapse [tem] that turn; in its own turn, blocked, Lapse [tem]
/// is offered again.
#[test]
fn amulet_lets_a_vampire_play_basic_temporis_once_each_turn() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 5, false);
    let y = e.add_minion(SeatId(1), plain("Ventrue", 1), 5, false);
    on_with(
        &mut e,
        0,
        "Amulet of Temporal Perception",
        a,
        &[zvtes::cards::Modifier::BearerPaysToPlayAsBasic {
            discipline: "tem",
            blood: 1,
        }],
    );
    hand(&mut e, 0, &["Lapse", "Lapse"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(z));
    pick_when(&mut e, block(a));
    let req = offering(&mut e, play(0, 1, a));
    assert!(!req.options.contains(&play(0, 2, a)), "no [TEM]");
    pick(&mut e, play(0, 1, a));
    request(&mut e);
    assert_eq!(
        blood(&e, a),
        6 - 3 - 1,
        "its 2, the Amulet's 1, a hand strike"
    );
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.state.minions[a.index()].blood = 6;
    blocked(&mut e, a, y);
    offering(&mut e, play(0, 1, a));
}

/// [ANK 20230316] Amulet of Temporal Perception: "A second copy allows a
/// second use in the same turn." The first used, then gone with its
/// bearer, a second Amulet equipped on another vampire is usable that
/// same turn: once each turn is each copy's.
#[test]
fn amulet_second_copy_allows_a_second_use_in_the_same_turn() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 6, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 6, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 5, false);
    let y = e.add_minion(SeatId(1), plain("Ventrue", 1), 5, false);
    let amulet = zvtes::cards::Modifier::BearerPaysToPlayAsBasic {
        discipline: "tem",
        blood: 1,
    };
    let first = on_with(&mut e, 0, "Amulet of Temporal Perception", a, &[amulet]);
    hand(&mut e, 0, &["Lapse", "Lapse"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(z));
    pick_when(&mut e, block(a));
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[first.index()].spent_turn, e.state.turn);
    e.state.cards[first.index()].in_play = false;
    on_with(&mut e, 0, "Amulet of Temporal Perception", b, &[amulet]);
    pick(&mut e, bleed(y));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play(0, 1, b));
    request(&mut e);
    assert_eq!(
        blood(&e, b),
        6 - 3 - 1,
        "its 2, the second Amulet's 1, a hand strike"
    );
}

/// [ANK 20180111] [LSJ 19980303] The Amulet gives no superior and not the
/// Discipline: its vampire with basic Temporis is offered no [TEM], and a
/// vampire using it does not have Temporis; an ally bearer gets nothing.
#[test]
fn amulet_gives_no_superior_and_not_the_discipline() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 6, false);
    let t = e.add_minion(SeatId(0), plain("Brujah", 1), 6, false);
    e.state.minions[t.index()].gained_levels = vec!["tem"];
    let ally = e.add_ally(SeatId(0), find_library("47th Street Royals"));
    let amulet = zvtes::cards::Modifier::BearerPaysToPlayAsBasic {
        discipline: "tem",
        blood: 1,
    };
    for m in [a, t, ally] {
        on_with(&mut e, 0, "Amulet of Temporal Perception", m, &[amulet]);
    }
    let lapse = find_library("Lapse");
    let has = |m, mode| zvtes::cards::has_disciplines(&e.state, m, lapse, mode);
    assert!(has(a, 1) && !has(a, 2));
    assert!(!e.state.has_discipline(a, "tem"), "not the Discipline");
    assert!(has(t, 1) && !has(t, 2), "no superior");
    assert!(!has(ally, 1), "a vampire only");
}

/// [ANK 20190104] The Amulet's blood burns as the card is played (the red
/// reading R1), before its cost, and stays burned when the card is
/// canceled: Cheat the Fates [tem] canceled by Direct Intervention.
#[test]
fn amulet_burns_its_blood_as_the_card_is_played() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    on_with(
        &mut e,
        0,
        "Amulet of Temporal Perception",
        a,
        &[zvtes::cards::Modifier::BearerPaysToPlayAsBasic {
            discipline: "tem",
            blood: 1,
        }],
    );
    hand(&mut e, 0, &["Cheat the Fates"]);
    hand(&mut e, 1, &["Direct Intervention"]);
    pick(&mut e, play(0, 1, a));
    let req = expect(&mut e, SeatId(1), WindowLabel::CardAsPlayed);
    assert_eq!(blood(&e, a), 3, "burned as played");
    assert!(req.options.contains(&master_play(0)));
    pick(&mut e, master_play(0));
    request(&mut e);
    assert_eq!(blood(&e, a), 3, "not refunded");
}

/// [LSJ 20100421] "Still allows to fetch a second Dagger if equipped in a
/// non-standard way." Dagger: "You can put a second Dagger from your hand
/// or ash heap on this minion after they equip this Dagger from your hand."
/// Equipped from hand, a copy is offered from the hand and from the ash
/// heap; the ash heap's goes on too, put on, not played: the pool paid
/// once, the hand's copy kept.
#[test]
fn dagger_fetches_a_second_dagger_from_hand_or_ash_heap() {
    use zvtes::decision::Zone;
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let dagger = find_library("Dagger");
    hand(&mut e, 0, &["Dagger", "Dagger"]);
    e.state.seats[0].ash.push(dagger);
    zvtes::copies::number_unnumbered(&mut e);
    pick(&mut e, play(0, 0, a));
    let req = offering(
        &mut e,
        Choice::TakeFrom {
            zone: Zone::AshHeap,
            card: dagger,
        },
    );
    assert!(req.options.contains(&Choice::TakeFrom {
        zone: Zone::Hand,
        card: dagger
    }));
    pick(
        &mut e,
        Choice::TakeFrom {
            zone: Zone::AshHeap,
            card: dagger,
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let on: Vec<_> = e
        .state
        .cards
        .iter()
        .filter(|c| c.card == dagger && c.present() && c.host == Some(a))
        .collect();
    assert_eq!(on.len(), 2, "both on the minion");
    assert!(on.iter().all(|c| !c.effect_only));
    assert!(!e.state.seats[0].ash.contains(&dagger));
    assert_eq!(e.state.seats[0].pool, 10 - 1, "paid once");
    assert!(
        e.state.seats[0].hand.contains(&dagger),
        "the hand's copy kept"
    );
}

/// Dagger: "Damage inflicted on a Kiasyd by this weapon is aggravated. …
/// Alternatively, ranged strike: strength damage and, after strike
/// resolution, put this weapon out of play until the end of the action."
/// Its ranged strike at close range: out of play for the next round (no
/// strike of its own), back on its bearer as the action ends; on a Kiasyd,
/// the damage is aggravated (torpor).
#[test]
fn dagger_ranged_strike_leaves_play_until_the_action_ends() {
    for kiasyd in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 5, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = if kiasyd {
            e.add_minion(SeatId(1), of_clan("Kiasyd", 0), 5, false)
        } else {
            e.add_minion(SeatId(1), plain("Ventrue", 0), 5, false)
        };
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let d = on_with(&mut e, 0, "Dagger", a, &[]);
        on_with(
            &mut e,
            0,
            "Amulet of Temporal Perception",
            a,
            &[zvtes::cards::Modifier::HostCombatPress(1)],
        );
        blocked(&mut e, a, z);
        pick_when(&mut e, ability(d.0, 1));
        if kiasyd {
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert!(e.state.minions[z.index()].in_torpor, "aggravated");
            continue;
        }
        pick_when(&mut e, Choice::UsePress);
        assert!(e.state.cards[d.index()].out_of_play, "out of play");
        never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, ability(d.0, 0));
        let c = &e.state.cards[d.index()];
        assert!(c.present() && c.host == Some(a), "back as the action ends");
    }
}

/// Dagger: "Burn this weapon if any damage from it is prevented." Its
/// melee strike, 1 point prevented by Soak: burned; unprevented, it stays.
#[test]
fn dagger_burns_when_its_damage_is_prevented() {
    for soak in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 5, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), vampire_with("for", 0), 5, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let d = on_with(&mut e, 0, "Dagger", a, &[]);
        if soak {
            hand(&mut e, 1, &["Soak"]);
        }
        blocked(&mut e, a, z);
        pick_when(&mut e, ability(d.0, 0));
        if soak {
            pick_when(&mut e, play(0, 0, z));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.cards[d.index()].present(), !soak);
        assert_eq!(e.state.seats[0].ash.contains(&find_library("Dagger")), soak);
    }
}

/// [LSJ 20100211-2] "The weapon does not burn if it is temporarily out of
/// play (eg. Kerrie, Dagger). The active Methuselah chooses the order":
/// Cleave on John's Dagger, then its ranged strike; as the action ends,
/// burning first spares the Dagger (out of play), returning it first lets
/// Cleave burn it.
#[test]
fn cleave_does_not_burn_a_dagger_out_of_play() {
    for burn_first in [true, false] {
        let mut e = game(2);
        let john = e.add_minion(
            SeatId(0),
            find_card("John \"Cop90\" O'Malley"),
            CRYPT[find_card("John \"Cop90\" O'Malley") as usize].capacity,
            false,
        );
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 5, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let d = on_with(&mut e, 0, "Dagger", john, &[]);
        hand(&mut e, 0, &["Cleave"]);
        blocked(&mut e, john, z);
        pick_when(&mut e, at_card(0, 0, john, d));
        pick_when(&mut e, ability(d.0, 1));
        let order = Choice::ResolveRider { card: d.0 };
        let req = offering(&mut e, order);
        assert_eq!(req.seat, SeatId(0), "the active Methuselah");
        pick(&mut e, if burn_first { Choice::Pass } else { order });
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.cards[d.index()].present(), burn_first);
    }
}

// ---- Vampires ---------------------------------------------------------------

/// Whether `o` uses vampire `m`'s printed ability `ability`.
fn uses_vampire(o: &Choice, m: MinionId, ability: u8) -> bool {
    matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(v), ability: i, .. }
        if *v == m && *i == ability)
}

/// The first offer of vampire `m`'s ability `ability`, the fallback taken
/// until then.
fn vampire_use(e: &mut Engine, m: MinionId, ability: u8) -> Choice {
    for _ in 0..300 {
        let req = request(e);
        if let Some(o) = req.options.iter().find(|o| uses_vampire(o, m, ability)) {
            return *o;
        }
        e.choose(fallback(&req));
    }
    panic!("{m:?}'s ability {ability} never offered");
}

/// Vampire `m`'s printed action `ability` at `target`.
fn vampire_action(m: MinionId, ability: u8, target: Target) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Vampire(m),
        ability,
        actor: Some(m),
        target,
    }
}

/// Rulebook "Slave" (rules-qa 62.14): a crypt slave takes no directed
/// action while its controller controls no ready vampire of its master
/// clan (Rusticus; Chalcedony, whose bar was missing), and, as one of
/// them is blocked, may lock to fight the blocker instead.
#[test]
fn crypt_slaves_serve_their_master_clan() {
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), find_card("Rusticus"), 4, false);
    let c = e.add_minion(SeatId(0), find_card("Chalcedony"), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let req = request(&mut e);
    assert!(!req.options.contains(&bleed(r)) && !req.options.contains(&bleed(c)));
    assert!(
        req.options.contains(&action(r, ActionKind::Hunt)),
        "undirected"
    );
    let t = e.add_minion(SeatId(0), plain("Tremere", 0), 4, false);
    let req = request(&mut e);
    assert!(req.options.contains(&bleed(r)) && req.options.contains(&bleed(c)));
    pick(&mut e, bleed(t));
    pick_when(&mut e, block(z));
    let fight = vampire_use(&mut e, r, 0);
    pick(&mut e, fight);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[r.index()].locked, "the slave fought");
    assert!(!e.state.minions[t.index()].locked, "its master unlocked");
}

/// Toy: "Toy cannot act or block. … Malkavian antitribu slave." No action,
/// leaving torpor included, no block attempt; his slave's fight is neither
/// (rulebook "Slave"): he takes a blocked Malkavian antitribu's combat.
#[test]
fn toy_cannot_act_or_block_but_fights_for_his_master() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let toy = e.add_minion(SeatId(1), find_card("Toy"), 2, false);
    let m = e.add_minion(SeatId(1), plain("Malkavian antitribu", 0), 4, false);
    pick(&mut e, bleed(a));
    let req = offering(&mut e, block(m));
    assert!(!req.options.contains(&block(toy)), "cannot block");
    let mut e = game(2);
    let toy = e.add_minion(SeatId(0), find_card("Toy"), 2, false);
    let torpid = e.add_minion(SeatId(0), find_card("Toy"), 2, true);
    let m = e.add_minion(SeatId(0), plain("Malkavian antitribu", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let req = request(&mut e);
    assert!(
        !req.options.iter().any(|o| matches!(o,
            Choice::TakeAction { minion, .. } if *minion == toy || *minion == torpid)),
        "cannot act: {:?}",
        req.options
    );
    pick(&mut e, bleed(m));
    pick_when(&mut e, block(z));
    let fight = vampire_use(&mut e, toy, 0);
    pick(&mut e, fight);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[toy.index()].locked, "Toy fought");
}

/// [LSJ 20050322-2] Al-Muntathir "can use a multi-discipline card with a
/// potential (unused) Celerity effect": Line Brawl's [pot] mode, not its
/// [cel] one. "If you control 4 or more True Brujah, burn Al-Muntathir":
/// himself and a torpid one count; the fourth burns him at once.
#[test]
fn al_muntathir_plays_no_celerity_mode() {
    let mut e = game(2);
    let al = e.add_minion(
        SeatId(0),
        find_card("Al-Muntathir, God's Witness"),
        4,
        false,
    );
    e.add_minion(
        SeatId(0),
        any_vampire(
            |c| c.clan == "True Brujah" && c.name != "Al-Muntathir, God's Witness",
            0,
        ),
        4,
        false,
    );
    e.add_minion(
        SeatId(0),
        any_vampire(
            |c| c.clan == "True Brujah" && c.name != "Al-Muntathir, God's Witness",
            1,
        ),
        1,
        true,
    );
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let brawl = find_library("Line Brawl");
    let mode = |e: &Engine, m: u8| {
        e.state.printed_mod(
            al,
            Stat::CannotPlay,
            zvtes::state::PrintedCtx {
                card: Some((brawl, m)),
                ..Default::default()
            },
        )
    };
    assert_eq!(
        (mode(&e, 0), mode(&e, 1)),
        (1, 0),
        "[cel] barred, [pot] not"
    );
    request(&mut e);
    assert!(e.state.minions[al.index()].in_play, "three True Brujah");
    let fourth = e.add_minion(
        SeatId(0),
        any_vampire(
            |c| c.clan == "True Brujah" && c.name != "Al-Muntathir, God's Witness",
            2,
        ),
        4,
        false,
    );
    request(&mut e);
    assert!(!e.state.minions[al.index()].in_play, "burned");
    assert!(e.state.seats[0]
        .ash_crypt
        .contains(&find_card("Al-Muntathir, God's Witness")));
    assert!(e.state.minions[fourth.index()].in_play);
}

/// Obaluaye: "+1 stealth when hunting. If you control four or more
/// Salubri, burn him." His hunt's stealth only; three Salubri with him
/// (one in torpor), he burns.
#[test]
fn obaluaye_hunts_quietly_and_burns_among_four_salubri() {
    let mut e = game(2);
    let o = e.add_minion(SeatId(0), find_card("Obaluaye"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let st = |e: &Engine, kind| {
        e.state.printed_mod(
            o,
            Stat::Stealth,
            zvtes::state::PrintedCtx {
                kind: Some(kind),
                ..Default::default()
            },
        )
    };
    assert_eq!(
        (st(&e, ActionKind::Hunt), st(&e, ActionKind::Bleed)),
        (1, 0)
    );
    e.add_minion(
        SeatId(0),
        any_vampire(|c| c.clan == "Salubri" && c.name != "Obaluaye", 0),
        4,
        false,
    );
    e.add_minion(
        SeatId(0),
        any_vampire(|c| c.clan == "Salubri" && c.name != "Obaluaye", 1),
        1,
        true,
    );
    e.add_minion(
        SeatId(0),
        any_vampire(|c| c.clan == "Salubri antitribu", 0),
        4,
        false,
    );
    request(&mut e);
    assert!(
        e.state.minions[o.index()].in_play,
        "a Salubri antitribu is not one"
    );
    e.add_minion(
        SeatId(0),
        any_vampire(|c| c.clan == "Salubri" && c.name != "Obaluaye", 2),
        4,
        false,
    );
    request(&mut e);
    assert!(!e.state.minions[o.index()].in_play);
    assert!(e.state.seats[0].ash_crypt.contains(&find_card("Obaluaye")));
}

/// Alcoan: "If Alcoan is in combat with an ally or younger vampire, you can
/// look at the opposing minion's controller's hand before range is
/// determined during the first round." Against a younger blocker: the
/// look; against an older one: never offered.
#[test]
fn alcoan_looks_at_a_younger_opponents_hand_on_the_first_round() {
    for younger in [true, false] {
        let mut e = game(2);
        let al = e.add_minion(SeatId(0), find_card("Alcoan"), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        // Lachlan, Noddist: no printed text, his title inert in a combat.
        let z = if younger {
            textless(|c| c.sect.is_some() && c.capacity < 7, 0)
        } else {
            find_card("Lachlan, Noddist")
        };
        assert_eq!(CRYPT[z as usize].capacity < 7, younger);
        let z = e.add_minion(SeatId(1), z, 4, false);
        hand(&mut e, 1, &["Kevlar Vest"]);
        pick(&mut e, bleed(al));
        pick_when(&mut e, block(z));
        if younger {
            e.notes = Some(Vec::new());
            let look = vampire_use(&mut e, al, 0);
            pick(&mut e, look);
            request(&mut e);
            let looks = noted_looks(&mut e);
            assert!(
                looks
                    .iter()
                    .any(|l| l.viewer == SeatId(0) && l.seat == SeatId(1)),
                "{looks:?}"
            );
        } else {
            never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
                uses_vampire(o, al, 0)
            });
        }
    }
}

/// [ANK 20220615] Annazir "cannot perform the action provided by his
/// ability twice in the same turn": a location burned, the steal is not
/// offered that turn, unlocked or not.
#[test]
fn annazir_burns_a_location_or_steals_once_a_turn() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Annazir"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let loc = zvtes::library::LIBRARY
        .iter()
        .position(|l| l.labels.contains(&zvtes::library::Label::Location))
        .expect("a location") as u16;
    e.state.cards.push(CardInPlay::new(loc, SeatId(1), None, 0));
    let place = CardId(e.state.cards.len() as u32 - 1);
    let vest = in_play(&mut e, 1, "Kevlar Vest", Some(z));
    pick(&mut e, vampire_action(a, 0, Target::Card(place)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[place.index()].present(), "burned");
    e.state.minions[a.index()].locked = false;
    let req = request(&mut e);
    assert!(!req
        .options
        .contains(&vampire_action(a, 1, Target::Card(vest))));
}

/// [LSJ 20090324] Annazir's action "is directed against the Methuselah
/// controlling the equipment or location, not the minion it's on": the
/// steal is directed at seat 1, no minion targeted; the Vest is his.
#[test]
fn annazir_steals_at_the_bearers_methuselah_not_the_bearer() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Annazir"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let vest = in_play(&mut e, 1, "Kevlar Vest", Some(z));
    pick(&mut e, vampire_action(a, 1, Target::Card(vest)));
    request(&mut e);
    let act = e.action().expect("the action");
    assert_eq!(act.directed_seat(&e.state), Some(SeatId(1)));
    assert_eq!(act.target_minions().count(), 0, "not the bearer");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[vest.index()].host, Some(a));
}

/// Azrael: "Azrael can move 1 of the blood he would gain from a hunt to
/// another vampire you control instead." To his torpid vampire, or kept
/// (Pass); at capacity he still gains it to move (89.26).
#[test]
fn azrael_hunts_one_blood_into_another_vampire() {
    for (start, moved) in [(3, true), (3, false), (5, true)] {
        let mut e = game(2);
        let az = e.add_minion(SeatId(0), find_card("Azrael"), start, false);
        let t = e.add_minion(SeatId(0), plain("Brujah", 0), 1, true);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        pick(&mut e, action(az, ActionKind::Hunt));
        let to = Choice::ChooseMinion(t);
        let req = offering(&mut e, to);
        assert!(req.options.contains(&Choice::Pass));
        pick(&mut e, if moved { to } else { Choice::Pass });
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let (az_after, t_after) = if moved {
            (start, 2)
        } else {
            ((start + 1).min(5), 1)
        };
        assert_eq!((blood(&e, az), blood(&e, t)), (az_after, t_after));
        assert!(
            e.state.minions[az.index()].hunted_turn.is_some(),
            "a successful hunt"
        );
    }
}

/// Baroque: "can enter combat with a younger vampire who is not Giovanni
/// or Nosferatu as a Ⓓ action." A younger Toreador, not a younger Giovanni
/// or Nosferatu, nor an older vampire.
#[test]
fn baroque_rushes_a_younger_vampire_but_no_giovanni_or_nosferatu() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), find_card("Baroque"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let young = |clan: &str| any_vampire(|c| c.clan == clan && c.capacity < 7, 0);
    let tor = e.add_minion(SeatId(1), young("Toreador"), 4, false);
    let gio = e.add_minion(SeatId(1), young("Giovanni"), 4, false);
    let nos = e.add_minion(SeatId(1), young("Nosferatu"), 4, false);
    let old = e.add_minion(
        SeatId(1),
        any_vampire(|c| c.clan == "Toreador" && c.capacity > 7, 0),
        4,
        false,
    );
    let req = request(&mut e);
    let at = |t| vampire_action(b, 0, Target::Minion(t));
    assert!(req.options.contains(&at(tor)));
    for t in [gio, nos, old] {
        assert!(!req.options.contains(&at(t)));
    }
}

/// The `nth` base vampire matching `f`, defined or not (a count's fodder).
fn any_vampire(f: impl Fn(&zvtes::crypt::CryptCard) -> bool, nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(_, c)| !c.adv && !c.imbued && f(c))
        .nth(nth)
        .map(|(i, _)| i as u16)
        .expect("a vampire")
}

/// The burn counters on Byzar.
fn burn_counters(e: &Engine, m: MinionId) -> u8 {
    e.state.minions[m.index()].burn_counters.0
}

/// [LSJ 20091217] [LSJ 20091217-2] Byzar's ability "is a replacement
/// effect. It … still allows burn conditions to be met (eg. Judgment:
/// Camarilla Segregation)": burned to pay for the Judgment, Byzar goes to
/// torpor with a burn counter, and the Judgment burns.
#[test]
fn byzar_burned_goes_to_torpor_with_a_counter() {
    let mut e = game(2);
    let by = e.add_minion(SeatId(0), find_card("Byzar"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let j = in_play(&mut e, 1, "Judgment: Camarilla Segregation", None);
    e.at_master_phase();
    pick_when(&mut e, use_on(j, 0, Target::Minion(by)));
    request(&mut e);
    let v = &e.state.minions[by.index()];
    assert!(v.in_play && v.in_torpor);
    assert_eq!(burn_counters(&e, by), 1);
    assert!(!e.state.cards[j.index()].in_play, "the Judgment burned");
    assert!(!e.state.seats[0].ash_crypt.contains(&find_card("Byzar")));
}

/// [LSJ 20100228] Byzar's "ability will make a diablerie unsuccessful
/// (blood and equipment stay on him)": the diablerist gets nothing, no
/// blood hunt; Byzar torpid with a counter, his blood and Vest kept.
#[test]
fn byzar_survives_a_diablerie_unsuccessfully() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), plain("Brujah", 0), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let by = e.add_minion(SeatId(1), find_card("Byzar"), 3, true);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let vest = in_play(&mut e, 1, "Kevlar Vest", Some(by));
    pick(
        &mut e,
        Choice::TakeAction {
            minion: d,
            action: ActionKind::Diablerie,
            target: Some(by),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.referendum().is_none());
    let v = &e.state.minions[by.index()];
    assert!(v.in_play && v.in_torpor);
    assert_eq!((blood(&e, by), burn_counters(&e, by)), (3, 1));
    assert_eq!(e.state.cards[vest.index()].host, Some(by));
    assert_eq!(blood(&e, d), 2, "the diablerist gets nothing");
    assert!(e.state.minions[d.index()].diablerie_turn.is_none());
    assert!(e.state.minions[d.index()].in_play, "no blood hunt");
}

/// [LSJ 20100423] Sacrificial Lamb: "If the target does not burn (eg.
/// Byzar's ability) then no blood is gained and no equipment transferred."
/// [LSJ 20091217] [LSJ 20091217-2] Byzar's replacement "prevents any
/// effect triggering after the vampire burns (eg. Sacrificial Lamb)".
#[test]
fn byzar_spoils_a_sacrificial_lamb() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_cap("obf", 5, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let by = e.add_minion(SeatId(0), find_card("Byzar"), 1, true);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let vest = in_play(&mut e, 0, "Kevlar Vest", Some(by));
    hand(&mut e, 0, &["Sacrificial Lamb"]);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(a),
            target: Target::Minion(by),
        },
    );
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::ChooseCard(vest.0),
    );
    assert!(e.state.minions[by.index()].in_play);
    assert_eq!(burn_counters(&e, by), 1);
    assert_eq!(e.state.cards[vest.index()].host, Some(by));
    assert_eq!(blood(&e, a), 4 - 3, "its cost, no blood gained");
}

/// [ANK 20210424] Byzar: "When he burns from aggravated damage, additional
/// damage is lost": blocking the unnamed with no blood, his 2 aggravated
/// send Byzar to torpor and burn him once: one counter.
#[test]
fn byzar_burned_by_aggravated_damage_takes_one_counter() {
    let mut e = game(2);
    let u = e.add_minion(SeatId(0), find_card("unnamed, The"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let by = e.add_minion(SeatId(1), find_card("Byzar"), 0, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, bleed(u));
    pick_when(&mut e, block(by));
    let strike = vampire_use(&mut e, u, 0);
    pick(&mut e, strike);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[by.index()];
    assert!(v.in_play && v.in_torpor);
    assert_eq!(burn_counters(&e, by), 1);
}

/// Byzar: "If he has 3 burn counters, remove him from the game": a third
/// burn, counter then removal; removed, not burned, his Vest burned.
#[test]
fn byzar_leaves_the_game_at_his_third_burn_counter() {
    let mut e = game(2);
    let by = e.add_minion(SeatId(0), find_card("Byzar"), 3, false);
    e.state.minions[by.index()].burn_counters.0 = 2;
    let vest = in_play(&mut e, 0, "Kevlar Vest", Some(by));
    e.burn(by);
    let byzar = find_card("Byzar");
    assert!(!e.state.minions[by.index()].in_play);
    assert!(e.state.seats[0].removed_crypt.contains(&byzar));
    assert!(!e.state.seats[0].ash_crypt.contains(&byzar));
    assert!(!e.state.cards[vest.index()].in_play);
    assert!(e.state.seats[0].ash.contains(&find_library("Kevlar Vest")));
}

/// [LSJ 20100423] Byzar's "ability does not apply if he's not controlled
/// (ie. uncontrolled or yielded in a contest)": the yielded copy burns.
#[test]
fn byzar_yielded_in_a_contest_burns() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let mine = e.add_minion(SeatId(0), find_card("Byzar"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let theirs = e.add_minion(SeatId(1), find_card("Byzar"), 4, false);
    e.state.minions[mine.index()].contested = true;
    e.state.minions[theirs.index()].contested = true;
    pick_when(
        &mut e,
        Choice::YieldContest(zvtes::game::Contest::Card(theirs)),
    );
    request(&mut e);
    assert!(!e.state.minions[theirs.index()].in_play);
    assert_eq!(burn_counters(&e, theirs), 0);
    assert!(e.state.seats[1].ash_crypt.contains(&find_card("Byzar")));
}

/// [ANK 20210424] "If Amaranth burns him and he comes back in torpor,
/// another Amaranth can be played": Amaranth's diablerie resolves in the
/// combat (rules-qa 113.10), unsuccessful on Byzar, who goes to torpor
/// again: a second Amaranth is offered there, and its diablerie puts a
/// second counter on him; declined, he stays in torpor with one. No
/// blood hunt either way.
#[test]
fn byzar_back_in_torpor_after_amaranth_faces_a_second_one() {
    for second in [true, false] {
        let mut e = game(2);
        // Grimgroth: his +2 bleed and mage costs inert once blocked.
        let a = e.add_minion(SeatId(0), find_card("Grimgroth"), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let by = e.add_minion(SeatId(1), find_card("Byzar"), 0, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Amaranth", "Amaranth"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(by));
        pick_when(&mut e, play(0, 0, a));
        let req = request(&mut e);
        assert_eq!(req.window, WindowLabel::CombatPrevention, "{req:?}");
        assert!(e.combat().is_some(), "still in the combat");
        assert_eq!(burn_counters(&e, by), 1);
        if second {
            pick(&mut e, play(0, 0, a));
        } else {
            pick(&mut e, Choice::Pass);
        }
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        let v = &e.state.minions[by.index()];
        assert!(v.in_play && v.in_torpor, "second {second}");
        assert_eq!(burn_counters(&e, by), 1 + second as u8);
        assert!(e.state.minions[a.index()].diablerie_turn.is_none());
    }
}

/// rules-qa 113.10 (owner, 2026-11-11: "during combat yes"; advanced rules
/// §3.7.8.2): Amaranth's diablerie made in the combat, its blood hunt held
/// at once, the combat suspended, the diablerist (a combatant) casting its
/// votes in it; failed, the combat resumes and ends as its victim left it,
/// and Marie-Pierre enters combat with the diablerist (a combat of her
/// Samedi's), where a second Amaranth diablerizes her: a second blood hunt,
/// in that combat too.
#[test]
fn amaranth_blood_hunts_during_each_combat_marie_pierres_too() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Grimgroth"), 4, false);
    e.state.minions[a.index()].granted_title = Some(Title::Prince("Paris"));
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), of_clan("Samedi", 0), 0, false);
    let mp = e.add_minion(SeatId(1), find_card("Marie-Pierre"), 0, false);
    hand(&mut e, 0, &["Amaranth", "Amaranth"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    let mut hunts = Vec::new();
    for _ in 0..300 {
        let req = request(&mut e);
        if req.window == WindowLabel::BloodHuntPolling && hunts.len() < 2 {
            let c = e.combat().expect("the combat suspended");
            let victim = if hunts.is_empty() { z } else { mp };
            assert!(c.side_of_minion(victim).is_some(), "{req:?}");
            assert!(!e.state.minions[victim.index()].in_play, "diablerized");
            hunts.push(victim);
            if req.seat == SeatId(0) {
                assert!(req.options.contains(&votes(a, false)), "{req:?}");
            }
            while request(&mut e).window == WindowLabel::BloodHuntPolling {
                pick(&mut e, Choice::Pass);
            }
            assert!(e.referendum().is_none());
            continue;
        }
        if req.options.contains(&Choice::ChooseMinion(mp)) {
            assert_eq!(hunts, vec![z], "the first hunt before her offer");
            pick(&mut e, Choice::ChooseMinion(mp));
            continue;
        }
        if hunts.len() == 1 && req.options.contains(&play(0, 0, a)) {
            pick(&mut e, play(0, 0, a));
            continue;
        }
        if req.window == WindowLabel::MinionPhase {
            break;
        }
        e.choose(fallback(&req));
    }
    assert!(e.action().is_none() && e.combat().is_none());
    assert_eq!(hunts, vec![z, mp], "one blood hunt each");
    assert!(e.state.minions[a.index()].in_play, "both hunts failed");
}

/// rules-qa 113.10: the blocker's Amaranth diablerizes the acting vampire
/// its Coma sent to torpor, its blood hunt in the combat suspended, the acting Methuselah's prince
/// polled; the combat over, the action ends blocked, the acting vampire
/// gone.
#[test]
fn amaranth_by_the_blocker_hunts_in_combat_and_the_action_ends_blocked() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let p = e.add_minion(SeatId(0), plain("Ventrue", 2), 4, false);
    e.state.minions[p.index()].granted_title = Some(Title::Prince("Paris"));
    let b = e.add_minion(SeatId(1), find_card("Grimgroth"), 4, false);
    e.state.minions[b.index()].beast_disciplines[0] = Some("dem");
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 1, &["Coma", "Amaranth"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play(0, 0, b));
    pick_when(&mut e, play(0, 0, b));
    let req = until(&mut e, SeatId(0), WindowLabel::BloodHuntPolling);
    assert!(!e.state.minions[a.index()].in_play, "diablerized");
    assert!(e.combat().is_some(), "the hunt in the combat");
    assert!(req.options.contains(&votes(p, true)), "{req:?}");
    while request(&mut e).window == WindowLabel::BloodHuntPolling {
        pick(&mut e, Choice::Pass);
    }
    assert!(e.combat().is_none());
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
    assert!(e.state.minions[b.index()].in_play);
}

/// rules-qa 113.10: Sandra White blocking, her Amaranth diablerizes the
/// older acting vampire her hand strike sent to torpor; her text burns the
/// acting Methuselah's last pool, an oust read at once: no decision while
/// the ousted Methuselah's action stands (her Methuselah's Dreams of the
/// Sphinx keeps each window open), no window after the diablerie nor blood
/// hunt, the turn passes; at two seats, the game is over.
#[test]
fn sandra_whites_amaranth_ousts_the_acting_methuselah_at_once() {
    for n in [3, 2] {
        let mut e = game(n);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 1, false);
        let s = e.add_minion(SeatId(1), find_card("Sandra White"), 1, false);
        let p = e.add_minion(SeatId(1), plain("Ventrue", 2), 4, false);
        e.state.minions[p.index()].granted_title = Some(Title::Prince("Paris"));
        in_play(&mut e, 1, "Dreams of the Sphinx", None);
        assert!(e.state.younger_than(s, a), "an older victim");
        e.state.seats[0].pool = 1;
        hand(&mut e, 1, &["Amaranth"]);
        pick(&mut e, bleed(a));
        // Its last blood gone as it bleeds: a hand strike sends it down.
        e.state.minions[a.index()].blood = 0;
        pick_when(&mut e, block(s));
        pick_when(&mut e, play(0, 0, s));
        if n == 2 {
            let over = matches!(e.run(), zvtes::game::Step::Over(_));
            assert!(over, "the game over");
            assert!(e.state.seats[0].ousted && !e.state.minions[a.index()].in_play);
            continue;
        }
        let req = request(&mut e);
        assert!(e.state.seats[0].ousted && !e.state.minions[a.index()].in_play);
        assert!(e.action().is_none() && e.combat().is_none(), "{req:?}");
        assert_eq!(e.state.active, SeatId(1), "the turn passed");
    }
}

/// rules-qa 113.10: Sandra White acting, her Amaranth diablerizes the older
/// blocker her hand strike sent to torpor, its Methuselah's last pool burned
/// by her text; the oust waits on seat 2's Life Boon, read as it is declined:
/// the combat ends with the oust, its blood hunt held first with the combat
/// still set, then the combat over with no end-of-round window.
#[test]
fn amaranth_ousting_the_blockers_methuselah_hunts_before_the_combat_ends() {
    let mut e = game(3);
    let s = e.add_minion(SeatId(0), find_card("Sandra White"), 1, false);
    let p = e.add_minion(SeatId(0), plain("Ventrue", 2), 4, false);
    e.state.minions[p.index()].granted_title = Some(Title::Prince("Paris"));
    let b = e.add_minion(SeatId(1), plain("Brujah", 0), 0, false);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    assert!(e.state.younger_than(s, b), "an older victim");
    e.state.seats[1].pool = 1;
    hand(&mut e, 0, &["Amaranth"]);
    hand(&mut e, 2, &["Life Boon"]);
    pick(&mut e, bleed(s));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play(0, 0, s));
    let (mut declined, mut hunted) = (false, false);
    for _ in 0..200 {
        let req = request(&mut e);
        if (req.seat, req.window) == (SeatId(2), WindowLabel::ResolutionChoice) {
            assert!(!e.state.seats[1].ousted && !hunted, "the oust deferred");
            declined = true;
        }
        if req.window == WindowLabel::BloodHuntPolling {
            assert!(
                declined && e.state.seats[1].ousted,
                "ousted before the hunt"
            );
            assert!(!e.state.minions[b.index()].in_play, "diablerized");
            assert!(e.combat().is_some(), "the hunt in the combat");
            hunted = true;
        }
        assert_ne!(req.window, WindowLabel::CombatEndOfRound, "{req:?}");
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        e.choose(fallback(&req));
    }
    assert!(declined, "the Life Boon declined");
    assert!(hunted, "a blood hunt");
    assert!(e.combat().is_none() && e.state.minions[s.index()].in_play);
}

/// rules-qa 113.155: a vampire Liquefy [VIC]'s strike diablerizes is being
/// burned in Amaranth's sense (owner, 2026-11-11: "yes"): its hand strike
/// sending the Liquefy user to torpor, it is not offered Amaranth; the
/// Liquefy diablerie and its blood hunt follow, the user in torpor beside
/// them (the strikes land together).
#[test]
fn a_vampire_liquefy_diablerizes_cannot_play_amaranth() {
    let mut e = game(2);
    let big = (0..CRYPT.len() as u16)
        .find(|&i| {
            let c = &CRYPT[i as usize];
            c.capacity == 9 && !c.imbued && zvtes::cards::vampire(i).is_none()
        })
        .expect("a capacity 9 vampire");
    let a = e.add_minion(SeatId(0), big, 6, false);
    e.state.minions[a.index()].gained_levels = vec!["vic", "thn", "thn"];
    e.state.minions[a.index()].beast_disciplines[0] = Some("vic");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    let p = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[p.index()].granted_title = Some(Title::Prince("Paris"));
    hand(&mut e, 0, &["Liquefy the Mortal Coil", "Relentless Reaper"]);
    hand(&mut e, 1, &["Amaranth"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 1, a));
    let stand_in = card_named(&e, "Liquefy the Mortal Coil");
    pick_when(&mut e, ability(stand_in.0, 0));
    // Its last blood gone: the victim's hand strike sends it to torpor.
    e.state.minions[a.index()].blood = 0;
    let mut hunted = false;
    for _ in 0..100 {
        let req = request(&mut e);
        assert!(!req.options.contains(&play(0, 0, z)), "{req:?}");
        if req.window == WindowLabel::BloodHuntPolling {
            assert!(!e.state.minions[z.index()].in_play, "diablerized");
            hunted = true;
        }
        if (req.seat, req.window) == (SeatId(1), WindowLabel::MinionPhase) {
            break;
        }
        e.choose(fallback(&req));
    }
    assert!(hunted, "a blood hunt");
    assert!(
        e.state.minions[a.index()].in_torpor,
        "the hand strike landed too"
    );
}

/// rules-qa 113.155: the victim of the Liquefy [VIC] user's diablerie
/// strike, its Carrion Crows sending it toward torpor beside it, is being
/// diablerized, not going to torpor: neither the user's Amaranth nor its
/// Decapitate [POT] is offered against it; the Liquefy diablerie and its
/// blood hunt follow.
#[test]
fn a_liquefy_victim_is_not_going_to_torpor() {
    let mut e = game(2);
    let big = (0..CRYPT.len() as u16)
        .find(|&i| {
            let c = &CRYPT[i as usize];
            c.capacity == 9 && !c.imbued && zvtes::cards::vampire(i).is_none()
        })
        .expect("a capacity 9 vampire");
    let a = e.add_minion(SeatId(0), big, 6, false);
    e.state.minions[a.index()].gained_levels = vec!["vic", "thn", "thn", "ani", "pot", "pot"];
    e.state.minions[a.index()].beast_disciplines[0] = Some("vic");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    let p = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[p.index()].granted_title = Some(Title::Prince("Paris"));
    hand(
        &mut e,
        0,
        &[
            "Liquefy the Mortal Coil",
            "Carrion Crows",
            "Relentless Reaper",
            "Amaranth",
            "Decapitate",
        ],
    );
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 1, a));
    let stand_in = card_named(&e, "Liquefy the Mortal Coil");
    pick_when(&mut e, ability(stand_in.0, 0));
    // The crows' damage sends the victim down.
    e.state.minions[z.index()].blood = 0;
    let (amaranth, decapitate) = (play(0, 0, a), play(1, 2, a));
    let mut hunts = 0;
    for _ in 0..100 {
        let req = request(&mut e);
        if e.combat().is_some() {
            assert!(!req.options.contains(&amaranth), "{req:?}");
            assert!(!req.options.contains(&decapitate), "{req:?}");
        }
        if req.window == WindowLabel::BloodHuntPolling {
            hunts += 1;
            while request(&mut e).window == WindowLabel::BloodHuntPolling {
                pick(&mut e, Choice::Pass);
            }
            continue;
        }
        if (req.seat, req.window) == (SeatId(1), WindowLabel::MinionPhase) {
            break;
        }
        e.choose(fallback(&req));
    }
    assert_eq!(hunts, 1, "the Liquefy diablerie's hunt");
    assert!(!e.state.minions[z.index()].in_play && e.state.minions[a.index()].in_play);
    assert_eq!(e.state.seats[1].ash_crypt.copies().len(), 1, "burned once");
}

/// rules-qa 113.10: an oust while the combat is suspended for its blood
/// hunt waits for the combat to resume: the blocker diablerized by the
/// acting vampire's Amaranth, its Methuselah burns their last pool to
/// Ferraille in the hunt's polling, ousted at once or as seat 2 declines
/// its Life Boon; the hunt goes on in the combat, which ends as it resumes.
#[test]
fn an_oust_in_a_suspended_hunt_is_read_as_the_combat_resumes() {
    for boon in [false, true] {
        let mut e = game(3);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let p = e.add_minion(SeatId(0), plain("Ventrue", 2), 4, false);
        e.state.minions[p.index()].granted_title = Some(Title::Prince("Paris"));
        let b = e.add_minion(SeatId(1), plain("Brujah", 1), 0, false);
        e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
        let f = in_play(&mut e, 1, "Ferraille", None);
        e.state.seats[1].pool = 1;
        hand(&mut e, 0, &["Amaranth"]);
        if boon {
            hand(&mut e, 2, &["Life Boon"]);
        }
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(b));
        pick_when(&mut e, play(0, 0, a));
        pick_when(&mut e, use_on(f, 1, Target::None));
        let (mut declined, mut hunt_after) = (!boon, false);
        for _ in 0..200 {
            let req = request(&mut e);
            if (req.seat, req.window) == (SeatId(2), WindowLabel::ResolutionChoice) {
                assert!(e.combat().is_some() && !e.state.seats[1].ousted, "{req:?}");
                declined = true;
            } else if declined && e.referendum().is_some() {
                assert!(e.state.seats[1].ousted, "ousted in the hunt");
                assert!(e.combat().is_some(), "the combat still suspended");
                hunt_after = true;
            }
            if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
                break;
            }
            e.choose(fallback(&req));
        }
        assert!(declined && hunt_after, "the hunt goes on past the oust");
        assert!(e.combat().is_none() && e.referendum().is_none());
        assert!(!e.state.minions[b.index()].in_play && e.state.minions[a.index()].in_play);
    }
}

/// Blissful Agony [VAL] played by the blocker: the opposing vampire is the
/// acting one, whose Methuselah's minions are not offered; the blocker
/// itself and its Methuselah's others are (owner, 2026-11-10), and the
/// acting vampire enters combat with the one chosen.
#[test]
fn blissful_agony_superior_by_the_blocker_reads_its_own_opponent() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), find_card("Langa"), 4, false);
    let w = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 1, &["Blissful Agony"]);
    blocked(&mut e, a, z);
    let strike = |t: MinionId| Choice::Play {
        card: 0,
        mode: 2,
        actor: Some(z),
        target: Target::Minion(t),
    };
    let req = until(&mut e, SeatId(1), WindowLabel::CombatStrike);
    for t in [a, b] {
        assert!(!req.options.contains(&strike(t)), "the acting Methuselah's");
    }
    assert!(req.options.contains(&strike(z)), "the striker itself");
    pick(&mut e, strike(w));
    e.notes = Some(Vec::new());
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let begun: Vec<[MinionId; 2]> = noted_combats(&mut e)
        .into_iter()
        .filter_map(|n| match n {
            zvtes::combat::CombatNote::Begins { minions } => Some(minions),
            _ => None,
        })
        .collect();
    assert_eq!(begun, vec![[a, w]], "the acting vampire first");
}

/// Hector Trelane diablerized by Amaranth: burned in the combat, the
/// diablerist's controller gains 4 pool (LSJ 20091210 "in combat";
/// rules-qa 113.10).
#[test]
fn hector_trelane_diablerized_by_amaranth_pays_the_diablerist() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Grimgroth"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let h = e.add_minion(SeatId(1), find_card("Hector Trelane"), 0, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Amaranth"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(h));
    let before = e.state.seats[0].pool;
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    assert!(!e.state.minions[h.index()].in_play, "diablerized");
    assert_eq!(e.state.seats[0].pool, before + 4);
}

/// Cedric: "If Cedric successfully blocks a directed action, he may burn 1
/// blood when the action ends (after combat, if any) to unlock." After the
/// bleed's combat: unlocked for a blood; with none left, never offered.
#[test]
fn cedric_burns_a_blood_to_unlock_after_blocking_a_directed_action() {
    for start in [4, 1] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let c = e.add_minion(SeatId(1), find_card("Cedric"), start, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(c));
        if start == 1 {
            never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
                uses_vampire(o, c, 0)
            });
            assert!(e.state.minions[c.index()].locked);
            continue;
        }
        let unlock = vampire_use(&mut e, c, 0);
        let req = request(&mut e);
        assert_eq!(req.window, WindowLabel::ActionAfterCombat);
        let before = blood(&e, c);
        pick(&mut e, unlock);
        request(&mut e);
        assert!(!e.state.minions[c.index()].locked);
        assert_eq!(blood(&e, c), before - 1);
    }
}

/// [LSJ 20100708] Chaundice "can use her ability to prevent 4 damage on a
/// single strike": both uses on Hardestadt's 3-strength hand strike, none
/// taken.
#[test]
fn chaundice_prevents_twice_on_one_strike() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), find_card("Hardestadt"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let ch = e.add_minion(SeatId(1), find_card("Chaundice"), 5, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, bleed(h));
    pick_when(&mut e, block(ch));
    let first = vampire_use(&mut e, ch, 0);
    pick(&mut e, first);
    let second = vampire_use(&mut e, ch, 0);
    pick(&mut e, second);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, ch), 5, "3 damage prevented");
}

/// [LSJ 20040812-2] [PIB 20130327] Chaundice's prevention "cannot be used
/// to prevent aggravated damage": the unnamed's aggravated strike.
#[test]
fn chaundice_prevents_no_aggravated_damage() {
    let mut e = game(2);
    let u = e.add_minion(SeatId(0), find_card("unnamed, The"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let ch = e.add_minion(SeatId(1), find_card("Chaundice"), 5, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, bleed(u));
    pick_when(&mut e, block(ch));
    let strike = vampire_use(&mut e, u, 0);
    pick(&mut e, strike);
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        uses_vampire(o, ch, 0)
    });
    assert!(e.state.minions[ch.index()].in_torpor, "aggravated damage");
}

/// Darlene Killian: "Allies and retainers cost Darlene 1 additional
/// blood." A pool-cost ally: its pool and 1 of her blood.
#[test]
fn darlene_pays_a_blood_more_for_an_ally() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), find_card("Darlene Killian"), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let (ally, cost) = zvtes::library::LIBRARY
        .iter()
        .enumerate()
        .find_map(|(i, l)| match l.pool_cost {
            zvtes::library::Cost::Fixed(n)
                if l.types == [zvtes::library::CardType::Ally]
                    && l.clans.is_empty()
                    && l.disciplines.is_empty()
                    && l.blood_cost == zvtes::library::Cost::None
                    && zvtes::cards::lookup(i as u16).is_some() =>
            {
                Some((l.name, n as i32))
            }
            _ => None,
        })
        .expect("a plain ally");
    hand(&mut e, 0, &[ally]);
    pick(&mut e, play(0, 0, d));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 10 - cost, "{ally}");
    assert_eq!(blood(&e, d), 1, "a blood more");
}

/// Dovey Ebfwe: "+1 stealth when employing retainers."
#[test]
fn dovey_employs_with_a_stealth() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), find_card("Dovey Ebfwe"), 4, false);
    let st = |ctx| e.state.printed_mod(d, Stat::Stealth, ctx);
    let employ = zvtes::state::PrintedCtx {
        kind: Some(ActionKind::Employ),
        ..Default::default()
    };
    let bleeding = zvtes::state::PrintedCtx {
        kind: Some(ActionKind::Bleed),
        ..Default::default()
    };
    assert_eq!((st(employ), st(bleeding)), (1, 0));
}

/// Evil Jensen: "can search your crypt for a vampire and move them to your
/// uncontrolled region as a +1 stealth action that costs 2 pool." Any
/// vampire, the pool paid.
#[test]
fn evil_jensen_fetches_a_vampire_for_two_pool() {
    let mut e = game(2);
    let ej = e.add_minion(SeatId(0), find_card("Evil Jensen"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let (x, y) = (plain("Toreador", 0), plain("Nosferatu", 0));
    e.state.seats[0].crypt.push(x);
    e.state.seats[0].crypt.push(y);
    pick(&mut e, vampire_action(ej, 0, Target::None));
    let req = offering(&mut e, Choice::AshVampire(y));
    assert!(req.options.contains(&Choice::AshVampire(x)));
    pick(&mut e, Choice::AshVampire(y));
    request(&mut e);
    assert_eq!(
        e.state.seats[0].uncontrolled.last().map(|s| s.card),
        Some(y)
    );
    assert_eq!(e.state.seats[0].crypt.len(), 1);
    assert_eq!(e.state.seats[0].pool, 10 - 2);
}

/// Evil Jensen: "She must burn 1 blood to attempt to block older vampires."
/// An older acting vampire: the toll, paid as she attempts; a younger one:
/// none; with no blood, no attempt at an older one.
#[test]
fn evil_jensen_pays_to_block_an_older_vampire() {
    for (older, start) in [(true, 4), (false, 4), (true, 0)] {
        let mut e = game(2);
        let a = e.add_minion(
            SeatId(0),
            textless(
                |c| {
                    c.sect.is_some()
                        && if older {
                            c.capacity > 6
                        } else {
                            c.capacity < 6
                        }
                },
                0,
            ),
            4,
            false,
        );
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let ej = e.add_minion(SeatId(1), find_card("Evil Jensen"), start, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        pick(&mut e, bleed(a));
        if start == 0 {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(ej));
            continue;
        }
        pick_when(&mut e, block(ej));
        assert_eq!(blood(&e, ej), if older { 3 } else { 4 });
    }
}

/// Federico di Padua: "Federico gets 1 optional maneuver during the first
/// round of combat." Offered as range is determined in the first round,
/// once.
#[test]
fn federico_maneuvers_once_on_the_first_round() {
    let mut e = game(2);
    let f = e.add_minion(SeatId(0), find_card("Federico di Padua"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    pick(&mut e, bleed(f));
    pick_when(&mut e, block(z));
    let maneuver = vampire_use(&mut e, f, 0);
    assert_eq!(request(&mut e).window, WindowLabel::CombatDetermineRange);
    pick(&mut e, maneuver);
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        uses_vampire(o, f, 0)
    });
}

/// [LSJ 20011022] Fustuk: "You can lock Fustuk to unlock a Tremere
/// antitribu." From torpor, another Methuselah's locked Tremere antitribu;
/// an unlocked one is no target (rules-qa 48.7).
#[test]
fn fustuk_locks_from_torpor_to_unlock_a_tremere_antitribu() {
    let mut e = game(2);
    let fu = e.add_minion(SeatId(0), find_card("Fustuk"), 2, true);
    let u = e.add_minion(SeatId(0), plain("Tremere antitribu", 0), 4, false);
    let t = e.add_minion(SeatId(1), plain("Tremere antitribu", 1), 4, false);
    e.state.minions[t.index()].locked = true;
    let at = |o: &Choice, m: MinionId| {
        uses_vampire(o, fu, 0)
            && matches!(o, Choice::UseAbility { target: Target::Minion(x), .. } if *x == m)
    };
    let req = request(&mut e);
    assert!(!req.options.iter().any(|o| at(o, u)), "unlocked already");
    let unlock = *req
        .options
        .iter()
        .find(|o| at(o, t))
        .expect("the locked Tremere antitribu");
    pick(&mut e, unlock);
    request(&mut e);
    assert!(!e.state.minions[t.index()].locked);
    assert!(e.state.minions[fu.index()].locked);
}

/// Grotesque, from seat 0's uncontrolled region.
fn grotesque_to_influence(e: &mut Engine) {
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: find_card("Grotesque"),
            copy: Default::default(),
            counters: 3,
            banished: None,
        });
}

/// [LSJ 20091216] "If Grotesque enters play when you control no locations
/// (or no selectable locations), it immediately removes itself from the
/// game": influenced out with none, removed.
#[test]
fn grotesque_without_a_location_leaves_the_game() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    grotesque_to_influence(&mut e);
    let req = until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    e.choose(index_of(&req, Choice::InfluenceOut { slot: 0 }));
    request(&mut e);
    let g = MinionId(e.state.minions.len() as u32 - 1);
    assert!(!e.state.minions[g.index()].in_play);
    assert!(e.state.seats[0]
        .removed_crypt
        .contains(&find_card("Grotesque")));
}

/// Grotesque: "choose a location you control not chosen for another
/// Grotesque. Remove this Grotesque from the game if you no longer control
/// the location." The first one's location not offered to the second;
/// that location taken, the first leaves the game, the second stays.
#[test]
fn grotesque_leaves_the_game_with_its_location() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let first = e.add_minion(SeatId(0), find_card("Grotesque"), 3, false);
    let places: Vec<CardId> = zvtes::library::LIBRARY
        .iter()
        .enumerate()
        .filter(|(_, l)| l.labels.contains(&zvtes::library::Label::Location))
        .take(3)
        .map(|(i, _)| i as u16)
        .collect::<Vec<_>>()
        .into_iter()
        .map(|c| {
            e.state.cards.push(CardInPlay::new(c, SeatId(0), None, 0));
            CardId(e.state.cards.len() as u32 - 1)
        })
        .collect();
    e.state.minions[first.index()].bound_location = zvtes::state::SparseOption(Some(places[0]));
    grotesque_to_influence(&mut e);
    let req = until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    e.choose(index_of(&req, Choice::InfluenceOut { slot: 0 }));
    let req = request(&mut e);
    let second = MinionId(e.state.minions.len() as u32 - 1);
    assert_eq!(
        req.options,
        vec![
            Choice::ChooseCard(places[1].0),
            Choice::ChooseCard(places[2].0)
        ]
    );
    e.choose(index_of(&req, Choice::ChooseCard(places[2].0)));
    request(&mut e);
    assert_eq!(
        e.state.minions[second.index()].bound_location.0,
        Some(places[2])
    );
    e.state.cards[places[0].index()].controller = SeatId(1);
    request(&mut e);
    assert!(!e.state.minions[first.index()].in_play);
    assert!(e.state.seats[0]
        .removed_crypt
        .contains(&find_card("Grotesque")));
    assert!(e.state.minions[second.index()].in_play);
}

/// Harlan Graves: "If Harlan is diablerized, no blood hunt can be called."
/// Against a plain victim's diablerie, the blood hunt referendum opens.
#[test]
fn harlan_diablerized_calls_no_blood_hunt() {
    for harlan in [true, false] {
        let mut e = game(2);
        let d = e.add_minion(SeatId(0), plain("Brujah", 0), 2, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let victim = if harlan {
            find_card("Harlan Graves")
        } else {
            plain("Toreador", 0)
        };
        let h = e.add_minion(SeatId(1), victim, 3, true);
        let voter = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.minions[voter.index()].granted_title = Some(Title::Primogen);
        pick(
            &mut e,
            Choice::TakeAction {
                minion: d,
                action: ActionKind::Diablerie,
                target: Some(h),
            },
        );
        let mut hunted = false;
        loop {
            let req = request(&mut e);
            hunted |= e.referendum().is_some();
            if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
                break;
            }
            e.choose(fallback(&req));
        }
        assert_eq!(hunted, !harlan);
        assert!(!e.state.minions[h.index()].in_play, "diablerized");
        assert!(e.state.minions[d.index()].diablerie_turn.is_some());
    }
}

/// Harlan Graves: "He gets -1 intercept during non-bleed actions." A hunt
/// and a political action, not a bleed, nor outside an action.
#[test]
fn harlan_intercepts_less_off_bleeds() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), find_card("Harlan Graves"), 4, false);
    let during = |e: &Engine, kind| {
        e.state.printed_mod(
            h,
            Stat::Intercept,
            zvtes::state::PrintedCtx {
                kind,
                ..Default::default()
            },
        )
    };
    assert_eq!(
        [
            during(&e, Some(ActionKind::Hunt)),
            during(&e, Some(ActionKind::Political)),
            during(&e, Some(ActionKind::Bleed)),
            during(&e, None),
        ],
        [-1, -1, 0, 0]
    );
}

/// [LSJ 20050315] [LSJ 20010210] Helen Fairchild "cannot attempt an action
/// to equip nor strike to steal an equipment": a Kevlar Vest in hand
/// offers her no equip.
#[test]
fn helen_fairchild_equips_nothing() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), find_card("Helen Fairchild"), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Kevlar Vest"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, h)), "no equip");
    assert!(req.options.contains(&play(0, 0, b)));
    assert!(!e.state.can_hold(h, find_library("Kevlar Vest"), &[]));
}

/// [LSJ 20050315] Helen Fairchild "can equip with an equipment that is a
/// location when in play": Vast Wealth puts Palatial Estate on her (The
/// Beast's case).
#[test]
fn helen_fairchild_equips_a_location_through_vast_wealth() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), find_card("Helen Fairchild"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let wealth = in_play(&mut e, 0, "Vast Wealth", Some(h));
    hand(&mut e, 0, &["Palatial Estate"]);
    fill_hand(&mut e, 0);
    e.state.seats[0].library = vec![find_library("Palatial Estate")].into();
    pick(&mut e, used_by(wealth, 0, h));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let estate = find_library("Palatial Estate");
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.in_play && c.card == estate && c.host == Some(h)));
}

/// [ANK 20231124] [LSJ 19970718] Hillanvale: "During a bleed action,
/// Hillanvale can burn 1 blood and discard a card requiring Melpominee
/// [mel] to get +1 bleed." Before the bleed resolves, once: a blood and
/// Virtuosa for a bleed of 2.
#[test]
fn hillanvale_discards_melpominee_for_a_bleed_once() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), find_card("Hillanvale"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Virtuosa", "Virtuosa"]);
    pick(&mut e, bleed(h));
    let up = vampire_use(&mut e, h, 0);
    assert_eq!(request(&mut e).window, WindowLabel::ActionBlock);
    pick(&mut e, up);
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        uses_vampire(o, h, 0)
    });
    assert_eq!(blood(&e, h), 3);
    assert_eq!(e.state.seats[1].pool, 10 - 2);
    assert!(e.state.seats[0].ash.contains(&find_library("Virtuosa")));
    assert_eq!(e.state.seats[0].hand.len(), 1);
}

/// Hillanvale: "She gets -1 intercept against non-Independent, non-Anarch
/// vampires." Against a Camarilla vampire, not an Independent or Anarch
/// one, nor an ally.
#[test]
fn hillanvale_intercepts_less_against_camarilla() {
    use zvtes::crypt::Sect;
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), find_card("Hillanvale"), 4, false);
    let of = |e: &mut Engine, sect| {
        let card = textless(|c| c.sect == Some(sect), 0);
        e.add_minion(SeatId(1), card, 4, false)
    };
    let foes = [
        of(&mut e, Sect::Camarilla),
        of(&mut e, Sect::Independent),
        of(&mut e, Sect::Anarch),
        e.add_ally(SeatId(1), find_library("Political Ally")),
    ];
    assert_eq!(foes.map(|o| vs(&e, h, Stat::Intercept, o)), [-1, 0, 0, 0]);
}

/// [LSJ 20011022] [ANK 20231124] [LSJ 19970718] The Horde: "During a bleed
/// action, The Horde can lock to give another acting infernal minion +1
/// bleed." From torpor, another Methuselah's acting Annazir: +1 bleed
/// before it resolves; a non-infernal one gets nothing.
#[test]
fn the_horde_locks_to_bleed_an_infernal_minion_from_torpor() {
    for infernal in [true, false] {
        let mut e = game(3);
        let a = if infernal {
            e.add_minion(SeatId(0), find_card("Annazir"), 6, false)
        } else {
            e.add_minion(SeatId(0), plain("Brujah", 0), 6, false)
        };
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let horde = e.add_minion(SeatId(2), find_card("Horde, The"), 2, true);
        e.add_minion(SeatId(2), plain("Toreador", 0), 4, false);
        pick(&mut e, bleed(a));
        if !infernal {
            never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
                uses_vampire(o, horde, 0)
            });
            continue;
        }
        let up = vampire_use(&mut e, horde, 0);
        pick(&mut e, up);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(e.state.minions[horde.index()].locked);
        assert_eq!(
            e.state.seats[1].pool,
            10 - 3,
            "1, Annazir's +1, the Horde's"
        );
    }
}

/// Janet Langer: "Janet must burn 1 blood to attempt to block." Paid as she
/// attempts; with no blood, no attempt.
#[test]
fn janet_langer_pays_a_blood_to_block() {
    for start in [3, 0] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let j = e.add_minion(SeatId(1), find_card("Janet Langer"), start, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        pick(&mut e, bleed(a));
        if start == 0 {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(j));
            continue;
        }
        pick_when(&mut e, block(j));
        assert_eq!(blood(&e, j), 2);
    }
}

/// [ANK 20210124] Josette: "Josette can steal a vampire in torpor as a Ⓓ
/// action that costs 2 blood (the stolen vampire stays in torpor)." Not at
/// a ready vampire; the torpid one hers, still in torpor, for 2 blood.
#[test]
fn josette_steals_a_torpid_vampire_for_two_blood() {
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), find_card("Josette"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let t = e.add_minion(SeatId(1), plain("Toreador", 0), 2, true);
    let req = request(&mut e);
    assert!(!req
        .options
        .contains(&vampire_action(j, 0, Target::Minion(z))));
    pick(&mut e, vampire_action(j, 0, Target::Minion(t)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[t.index()];
    assert_eq!(v.owner, SeatId(0));
    assert!(v.in_play && v.in_torpor);
    assert_eq!(blood(&e, j), 2);
}

/// [LSJ 20011015] Jozz: "Jozz can enter combat with a vampire no more than
/// twice his capacity or with an ally as a Ⓓ action. Scarce." Capacity 2:
/// a capacity-4 vampire and an ally, not a capacity-5 one.
#[test]
fn jozz_rushes_a_vampire_up_to_twice_his_capacity_or_an_ally() {
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), find_card("Jozz"), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let of_cap = |n: u8| textless(|c| c.capacity == n && c.sect.is_some(), 0);
    let four = e.add_minion(SeatId(1), of_cap(4), 4, false);
    let five = e.add_minion(SeatId(1), of_cap(5), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Political Ally"));
    let req = request(&mut e);
    let at = |t| vampire_action(j, 0, Target::Minion(t));
    assert!(req.options.contains(&at(four)) && req.options.contains(&at(ally)));
    assert!(!req.options.contains(&at(five)));
    assert!(CRYPT[find_card("Jozz") as usize].scarce);
}

/// La Viuda Blanca: "La Viuda can draw up to 2 cards as a +1 stealth action
/// (discard down afterward)." The number picked as it resolves, 1 or 2:
/// two drawn, the hand discarded down to its size.
#[test]
fn la_viuda_blanca_draws_two_and_discards_down() {
    let mut e = game(2);
    let lv = e.add_minion(SeatId(0), find_card("La Viuda Blanca"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    fill_hand(&mut e, 0);
    let size = e.state.seats[0].hand.len();
    e.state.seats[0].library = vec![find_library("Bonding"); 5].into();
    pick(&mut e, vampire_action(lv, 0, Target::None));
    let req = offering(&mut e, Choice::SetX(2));
    assert_eq!(req.options, vec![Choice::SetX(1), Choice::SetX(2)]);
    pick(&mut e, Choice::SetX(2));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].library.len(), 3);
    assert_eq!(e.state.seats[0].hand.len(), size);
    assert_eq!(e.state.seats[0].ash.len(), 2);
}

/// Langa: "Langa may enter combat with a vampire controlled by another
/// Methuselah as a Ⓓ action." Another's vampire, not an ally nor his own
/// vampire.
#[test]
fn langa_rushes_another_methuselahs_vampire_not_an_ally() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), find_card("Langa"), 4, false);
    let own = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Political Ally"));
    let req = request(&mut e);
    let at = |t| vampire_action(l, 0, Target::Minion(t));
    assert!(req.options.contains(&at(z)));
    assert!(!req.options.contains(&at(ally)) && !req.options.contains(&at(own)));
    pick(&mut e, at(z));
    request(&mut e);
    assert!(e.combat().is_some() || e.action().is_some());
}

/// Take vampire `m`'s printed ability `ability` at its first offer.
fn use_vamp(e: &mut Engine, m: MinionId, ability: u8) {
    let c = vampire_use(e, m, ability);
    pick(e, c);
}

/// Copies of `name` in `seat`'s ash heap, hand and library.
fn copies(e: &Engine, seat: usize, name: &str) -> (usize, usize, usize) {
    let c = find_library(name);
    let s = &e.state.seats[seat];
    let n = |p: &[u16]| p.iter().filter(|&&k| k == c).count();
    (n(&s.ash), n(&s.hand), n(&s.library))
}

/// `seat`'s library, bottom first.
fn library(e: &mut Engine, seat: usize, names: &[&str]) {
    for n in names {
        e.state.seats[seat].library.push(find_library(n));
    }
}

/// Lord Ephraim Wainwright: "You can search your library for another copy
/// of an action card Ephraim plays after it is played and move that copy to
/// your ash heap (shuffle afterward). If the action succeeds, move the card
/// to your hand." Computer Hacking bled unblocked: the copy milled comes
/// back to hand; blocked, it stays in the ash heap.
#[test]
fn lord_ephraim_mills_a_copy_back_to_hand_if_the_action_succeeds() {
    for blocked in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), find_card("Lord Ephraim Wainwright"), 6, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Computer Hacking"]);
        fill_hand(&mut e, 0);
        library(&mut e, 0, &["Computer Hacking", "Conceal", "Conceal"]);
        pick(&mut e, play(0, 0, a));
        pick_when(&mut e, Choice::ResolvePrinted { minion: a });
        assert_eq!(copies(&e, 0, "Computer Hacking"), (1, 0, 0), "milled");
        assert_eq!(e.state.seats[0].library.len(), 1, "one replacement drawn");
        if blocked {
            pick_when(&mut e, block(z));
        }
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        if blocked {
            assert_eq!(copies(&e, 0, "Computer Hacking"), (2, 0, 0));
        } else {
            assert_eq!(e.state.seats[1].pool, 10 - 2);
            assert_eq!(
                copies(&e, 0, "Computer Hacking"),
                (1, 1, 0),
                "the copy back, the played card to the ash heap"
            );
        }
    }
}

/// Lord Ephraim Wainwright with no other copy in the library: the search
/// is still offered (a search may find nothing), the action's success
/// brings nothing back.
#[test]
fn lord_ephraim_searching_without_a_copy_brings_nothing_back() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Lord Ephraim Wainwright"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Computer Hacking"]);
    fill_hand(&mut e, 0);
    library(&mut e, 0, &["Conceal", "Conceal"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::ResolvePrinted { minion: a });
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(copies(&e, 0, "Computer Hacking"), (1, 0, 0));
    assert_eq!(e.state.seats[1].pool, 10 - 2);
}

/// Lord Ephraim Wainwright: "the card" is any copy of the milled card still
/// in the ash heap (rules-qa 113.25, the owner: copies there are not told
/// apart). The milled copy taken out of the ash heap during the action:
/// another copy there returns; with none, nothing, the played card (not
/// yet in the ash heap) staying there.
#[test]
fn lord_ephraim_returns_any_copy_of_the_milled_card() {
    use zvtes::copies::CopyId;
    let hacking = find_library("Computer Hacking");
    for other in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), find_card("Lord Ephraim Wainwright"), 6, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Computer Hacking"]);
        fill_hand(&mut e, 0);
        let s = &mut e.state.seats[0];
        s.library.put(hacking, CopyId(900));
        library(&mut e, 0, &["Conceal", "Conceal"]);
        if other {
            e.state.seats[0].ash.put(hacking, CopyId(901));
        }
        pick(&mut e, play(0, 0, a));
        pick_when(&mut e, Choice::ResolvePrinted { minion: a });
        // Another effect takes the milled copy.
        let ash = &mut e.state.seats[0].ash;
        let i = ash.copies().iter().position(|&c| c == CopyId(900));
        ash.take(i.expect("milled"));
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[1].pool, 10 - 2);
        assert_eq!(
            copies(&e, 0, "Computer Hacking"),
            (1, other as usize, 0),
            "other {other}"
        );
    }
}

/// Lydia, Grand Praetor: "During an action, Lydia can burn 1 blood to get +1
/// stealth. She cannot use cards requiring Celerity [cel]." Mordechai
/// Ben-Nun ("+1 intercept") attempts to block her bleed: she buys 1 stealth
/// once, not again in that action though still short. Line Brawl's [cel]
/// mode is barred, its [pot] mode not.
/// [LSJ 20050322-2] a multi-discipline card's unused [cel] mode is no bar.
#[test]
fn lydia_buys_stealth_once_an_action_and_shuns_celerity() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), find_card("Lydia, Grand Praetor"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), find_card("Mordechai Ben-Nun"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let brawl = find_library("Line Brawl");
    let barred = |e: &Engine, m: MinionId, mode: u8| {
        e.state.printed_mod(
            m,
            Stat::CannotPlay,
            PrintedCtx {
                card: Some((brawl, mode)),
                ..PrintedCtx::default()
            },
        )
    };
    let mikael = e.add_minion(SeatId(0), find_card("Mikael Birkholm"), 4, false);
    for m in [l, mikael] {
        assert_eq!((barred(&e, m, 0), barred(&e, m, 1)), (1, 0), "[cel] only");
    }
    e.state.minions[mikael.index()].in_play = false;
    pick(&mut e, bleed(l));
    pick_when(&mut e, block(z));
    use_vamp(&mut e, l, 0);
    assert_eq!(blood(&e, l), 3);
    // Still short of Mordechai's intercept: not offered again.
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
        uses_vampire(o, l, 0)
    });
}

/// Matthew: "If Matthew is ready during your master phase, he can reduce the
/// cost of a master card you play by 1 pool." The first master is 1 pool
/// less, the second of that master phase full price; torpid, nothing.
#[test]
fn matthew_takes_a_pool_off_one_master_each_master_phase() {
    for torpid in [false, true] {
        let mut e = game(2);
        let m = e.add_minion(SeatId(0), find_card("Matthew"), 4, torpid);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Carfax Abbey", "Channel 10"]);
        e.at_master_phase();
        e.state.seats[0].mpa = 2;
        pick(&mut e, master_play(0));
        let off = Choice::ResolvePrinted { minion: m };
        if torpid {
            assert!(!request(&mut e).options.contains(&off));
            until(&mut e, SeatId(0), WindowLabel::MasterPhase);
            assert_eq!(e.state.seats[0].pool, 10 - 2, "torpid: full price");
            continue;
        }
        pick(&mut e, off);
        until(&mut e, SeatId(0), WindowLabel::MasterPhase);
        assert_eq!(e.state.seats[0].pool, 10 - 1, "2 pool, 1 off");
        pick(&mut e, master_play(0));
        assert!(!request(&mut e).options.contains(&off), "once");
    }
}

/// Mikael Birkholm: "As Mikael chooses a gun strike, he can use the first
/// aim card you find in your library (shuffle afterward)." His .44 Magnum
/// strike chosen, the library's Target Vitals aims it: 2R and +2, the aim
/// out of the library and in the ash heap. A hand strike gets no offer.
/// [LSJ 20100511] the first from the top; its requirement met, no cost.
#[test]
fn mikael_birkholm_aims_with_the_first_aim_in_his_library() {
    for gun in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), find_card("Mikael Birkholm"), 6, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let magnum = in_play(&mut e, 0, ".44 Magnum", Some(a));
        assert_eq!(e.state.hand_size(SeatId(0)), 7 + 1, "+1 hand size");
        fill_hand(&mut e, 0);
        library(
            &mut e,
            0,
            &["Target Vitals", "Conceal", "Target Vitals", "Conceal"],
        );
        let hand_before = e.state.seats[0].hand.len();
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        if !gun {
            pick_when(&mut e, Choice::HandStrike);
            never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
                uses_vampire(o, a, 0)
            });
            assert_eq!(copies(&e, 0, "Target Vitals").2, 2);
            continue;
        }
        pick_when(&mut e, ability(magnum.0, 0));
        use_vamp(&mut e, a, 0);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, z), 6 - 4, "2R and the aim's 2");
        assert_eq!(copies(&e, 0, "Target Vitals"), (1, 0, 1));
        assert_eq!(e.state.seats[0].library.len(), 3);
        assert_eq!(e.state.seats[0].hand.len(), hand_before, "not from hand");
    }
}

/// Mikael Birkholm, his Target Vitals from hand canceled by the opponent's
/// two combat cards: neither his library aim nor another Target Vitals from
/// hand is offered on that strike (one aim each strike).
/// [LSJ 20100511] not after another aim, even canceled.
#[test]
fn mikael_birkholm_cannot_aim_after_a_canceled_aim() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Mikael Birkholm"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let magnum = in_play(&mut e, 0, ".44 Magnum", Some(a));
    hand(&mut e, 0, &["Target Vitals", "Target Vitals"]);
    fill_hand(&mut e, 0);
    hand(&mut e, 1, &["Form of Mist", "Form of Mist"]);
    library(&mut e, 0, &["Target Vitals", "Conceal"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, ability(magnum.0, 0));
    let req = request(&mut e);
    assert!(
        req.options.iter().any(|o| uses_vampire(o, a, 0)),
        "{:?}",
        req.options
    );
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::PayToCancel);
    let aim = |o: &Choice| uses_vampire(o, a, 0) || *o == play(0, 0, a);
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, aim);
    assert_eq!(blood(&e, z), 6 - 2, "2R, no aim");
    assert_eq!(copies(&e, 0, "Target Vitals").2, 1, "never searched");
}

/// Mikael Birkholm bearing Light Intensifying Goggles: the aim his library
/// gave may go onto the goggles.
/// [LSJ 20100209] aims used with his ability can be moved to the goggles.
#[test]
fn mikael_birkholm_stores_a_library_aim_on_his_goggles() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Mikael Birkholm"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let magnum = in_play(&mut e, 0, ".44 Magnum", Some(a));
    let goggles = in_play(&mut e, 0, "Light Intensifying Goggles", Some(a));
    fill_hand(&mut e, 0);
    library(&mut e, 0, &["Target Vitals"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, ability(magnum.0, 0));
    use_vamp(&mut e, a, 0);
    pick_when(&mut e, Choice::ResolveRider { card: goggles.0 });
    request(&mut e);
    assert_eq!(
        e.state.cards[goggles.index()].stored.to_cards(),
        vec![find_library("Target Vitals")]
    );
    assert_eq!(copies(&e, 0, "Target Vitals"), (0, 0, 0));
}

/// Morlock, his vampire and the blocker's, both seats with a second
/// vampire.
fn morlock_game() -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Morlock"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    (e, m, z)
}

/// Morlock's strike burning `card`.
fn morlock_burn(m: MinionId, card: CardId) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Vampire(m),
        ability: 0,
        actor: None,
        target: Target::Card(card),
    }
}

/// Morlock: "Morlock can strike, ranged: burn a retainer. +1 bleed."
/// Unblocked, he bleeds for 2.
#[test]
fn morlock_bleeds_for_two() {
    let (mut e, m, _) = morlock_game();
    pick(&mut e, bleed(m));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 2, "+1 bleed");
}

/// Morlock's strike: the blocker's Raven Spy is burned, no damage dealt.
#[test]
fn morlock_burns_an_opposing_retainer_with_a_strike() {
    let (mut e, m, z) = morlock_game();
    let raven = in_play(&mut e, 1, "Raven Spy", Some(z));
    pick(&mut e, bleed(m));
    pick_when(&mut e, block(z));
    let burn = morlock_burn(m, raven);
    let req = offering(&mut e, burn);
    assert!(req.options.contains(&burn));
    pick(&mut e, burn);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[raven.index()].present(), "burned");
    assert_eq!(blood(&e, z), 6, "no damage");
}

/// Morlock's strike with no retainer on the blocker: not offered (D14).
#[test]
fn morlock_strike_needs_a_retainer() {
    let (mut e, m, z) = morlock_game();
    pick(&mut e, bleed(m));
    pick_when(&mut e, block(z));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        uses_vampire(o, m, 0)
    });
}

/// Aye burns as its Laibon burns "a minion, equipment, or location": a
/// retainer burned by Morlock's strike (Morlock made Laibon) is none, and
/// his Aye stays.
#[test]
fn morlock_retainer_burn_burns_no_aye() {
    let (mut e, m, z) = morlock_game();
    e.state.minions[m.index()].sect_override = Some(Sect::Laibon);
    let raven = in_play(&mut e, 1, "Raven Spy", Some(z));
    let mods: Vec<zvtes::cards::Modifier> = zvtes::cards::lookup(find_library("Aye"))
        .expect("Aye")
        .modes[0]
        .effects
        .iter()
        .filter_map(|effect| match effect {
            zvtes::cards::Effect::Layer(modifier) => Some(*modifier),
            _ => None,
        })
        .collect();
    let aye = on_with(&mut e, 0, "Aye", m, &mods);
    pick(&mut e, bleed(m));
    pick_when(&mut e, block(z));
    pick_when(&mut e, morlock_burn(m, raven));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[raven.index()].present(), "burned");
    assert!(e.state.cards[aye.index()].in_play, "the Aye stays");
}

/// Myrna Goldman: "During your unlock phase, you can look at the top 3
/// cards of your library." Her look, noted to her seat only.
#[test]
fn myrna_goldman_looks_at_her_top_three() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Myrna Goldman"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    fill_hand(&mut e, 0);
    library(
        &mut e,
        0,
        &[
            "Conceal",
            "Conceal",
            "Deflection",
            "Bonding",
            "Govern the Unaligned",
        ],
    );
    until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    e.notes = Some(Vec::new());
    use_vamp(&mut e, m, 0);
    request(&mut e);
    let looks = noted_looks(&mut e);
    assert_eq!(looks.len(), 1);
    let l = &looks[0];
    assert_eq!((l.viewer, l.seat, l.library), (SeatId(0), SeatId(0), true));
    assert_eq!(l.cards.len(), 3);
    assert_eq!(
        l.cards[0],
        find_library("Govern the Unaligned"),
        "top first"
    );
}

/// Nahum Enosh: "If Nahum is ready during your unlock phase, another ready
/// vampire you control can gain 1 blood." Once that unlock phase, never
/// himself; torpid, not offered.
#[test]
fn nahum_enosh_feeds_another_ready_vampire_once() {
    for torpid in [false, true] {
        let mut e = game(2);
        let n = e.add_minion(SeatId(0), find_card("Nahum Enosh"), 6, torpid);
        let b = e.add_minion(SeatId(0), plain("Brujah", 0), 2, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        fill_hand(&mut e, 0);
        library(&mut e, 0, &["Conceal", "Conceal"]);
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        if torpid {
            never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
                uses_vampire(o, n, 0)
            });
            continue;
        }
        until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
        let feed = Choice::UseAbility {
            source: AbilitySource::Vampire(n),
            ability: 0,
            actor: None,
            target: Target::Minion(b),
        };
        let req = request(&mut e);
        assert!(req.options.contains(&feed), "{:?}", req.options);
        assert!(!req.options.contains(&Choice::UseAbility {
            source: AbilitySource::Vampire(n),
            ability: 0,
            actor: None,
            target: Target::Minion(n),
        }));
        pick(&mut e, feed);
        never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
            uses_vampire(o, n, 0)
        });
        assert_eq!(blood(&e, b), 3);
    }
}

/// Nitidas: "You can lock Nitidas to get +1 hand size until the end of the
/// turn." In torpor too, back to the hand size as the turn ends.
/// [LSJ 20011022] usable in torpor.
#[test]
fn nitidas_locks_for_hand_size_even_in_torpor() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), find_card("Nitidas"), 4, true);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let size = e.state.hand_size(SeatId(0));
    use_vamp(&mut e, n, 0);
    request(&mut e);
    assert!(e.state.minions[n.index()].locked);
    assert_eq!(e.state.hand_size(SeatId(0)), size + 1);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.hand_size(SeatId(0)),
        size,
        "until the end of the turn"
    );
}

/// Nkechi: "When Nkechi is in combat, she may burn a blood before range is
/// determined on the first round to allow you to look at the opposing
/// minion's controller's hand." Her blood burned, the hand seen; not in the
/// second round.
#[test]
fn nkechi_pays_a_blood_to_see_the_opposing_hand() {
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), find_card("Nkechi"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 1, &["Form of Mist"]);
    hand(&mut e, 0, &["Pursuit"]);
    pick(&mut e, bleed(k));
    pick_when(&mut e, block(z));
    e.notes = Some(Vec::new());
    use_vamp(&mut e, k, 0);
    assert_eq!(blood(&e, k), 3);
    let looks = noted_looks(&mut e);
    assert_eq!(looks.len(), 1);
    assert_eq!((looks[0].viewer, looks[0].seat), (SeatId(0), SeatId(1)));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        uses_vampire(o, k, 0)
    });
}

/// The text-only vampires: no definition, each its crypt card with its
/// printed sect and Black Hand trait (Malachai's Flight among his
/// Disciplines); the others' printed traits, generated too.
#[test]
fn the_round_28_text_only_vampires_carry_their_printed_traits() {
    for (name, sect, black_hand) in [
        ("Abebe", Sect::Independent, false),
        ("Angel Chavarria", Sect::Independent, false),
        ("Brigitte Mandisa", Sect::Independent, false),
        ("Ariel", Sect::Sabbat, false),
        ("Black Wallace", Sect::Sabbat, false),
        ("Aredhel", Sect::Sabbat, true),
        ("Greer Worder", Sect::Sabbat, false),
        ("Frank Weisshadel", Sect::Camarilla, false),
        ("Lord Vauxhall", Sect::Sabbat, false),
        ("Macoute", Sect::Independent, false),
        ("Malachai", Sect::Independent, false),
        ("Mariel St. John", Sect::Sabbat, false),
        ("Rashiel", Sect::Sabbat, false),
        ("Tarrence Moore", Sect::Camarilla, false),
    ] {
        let card = find_card(name);
        let c = &CRYPT[card as usize];
        assert!(zvtes::cards::vampire(card).is_none(), "{name}");
        assert_eq!((c.sect, c.black_hand), (Some(sect), black_hand), "{name}");
        assert_eq!(c.title, Title::None, "{name}");
    }
    assert!(CRYPT[find_card("Malachai") as usize]
        .disciplines
        .contains(&"FLIGHT"));
    assert!(CRYPT[find_card("Morlock") as usize].black_hand);
    let lydia = &CRYPT[find_card("Lydia, Grand Praetor") as usize];
    assert_eq!(lydia.title, Title::Votes(1));
    for name in [
        "Lydia, Grand Praetor",
        "Matthew",
        "Mikael Birkholm",
        "Nahum Enosh",
        "Serenna the White",
        "Veejay Vinod",
    ] {
        assert!(CRYPT[find_card(name) as usize].scarce, "{name}");
    }
    assert!(CRYPT[find_card("Sylvie Helgon") as usize].sterile);
    assert!(CRYPT[find_card("Xeper, Sultan of Lepers") as usize].infernal);
    assert!(CRYPT[find_card("Rocia") as usize]
        .disciplines
        .contains(&"FLIGHT"));
    for name in ["Uriel", "Virginie, Prodigy"] {
        assert_eq!(CRYPT[find_card(name) as usize].title, Title::Bishop);
    }
}

/// A printed stat of `m` against `opponent`.
fn vs(e: &Engine, m: MinionId, stat: Stat, opponent: MinionId) -> i32 {
    e.state.printed_mod(
        m,
        stat,
        PrintedCtx {
            opponent: Some(opponent),
            ..PrintedCtx::default()
        },
    )
}

/// Rocia: "Rocia gets +1 intercept when attempting to block Tremere or
/// Tremere antitribu. She gets an optional press each combat." The acting
/// vampire's clan: +1 against either Tremere clan, none against a Brujah;
/// her press in any combat.
#[test]
fn rocia_intercepts_tremere_and_presses() {
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), find_card("Rocia"), 4, false);
    let tremere = e.add_minion(SeatId(1), plain("Tremere", 0), 4, false);
    let anti = e.add_minion(SeatId(1), plain("Tremere antitribu", 0), 4, false);
    let brujah = e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    assert_eq!(vs(&e, r, Stat::Intercept, tremere), 1);
    assert_eq!(vs(&e, r, Stat::Intercept, anti), 1);
    assert_eq!(vs(&e, r, Stat::Intercept, brujah), 0);
    assert_eq!(vs(&e, r, Stat::CombatPress, brujah), 1);
}

/// Serenna the White: "Once each turn, if Serenna is ready, she can burn 1
/// blood to reduce a bleed against you by 2." The first bleed of the turn
/// at her controller reduced to nothing, the second not offered; offered
/// again in the next turn.
/// [ANK 20180719-4] once in each Methuselah's turn.
#[test]
fn serenna_reduces_one_bleed_each_turn() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), find_card("Serenna the White"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let a = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(a));
    use_vamp(&mut e, s, 0);
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!((e.state.seats[0].pool, blood(&e, s)), (10, 3));
    pick(&mut e, bleed(b));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        uses_vampire(o, s, 0)
    });
    assert_eq!(e.state.seats[0].pool, 10 - 1, "the second bleed in full");
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(a));
    use_vamp(&mut e, s, 0);
    assert_eq!(blood(&e, s), 2, "the next turn");
}

/// Silas: "If Silas is ready during your discard phase, he can burn 1 blood
/// to give you +1 discard phase action." One blood for a second discard
/// phase action; torpid, not offered.
#[test]
fn silas_buys_a_discard_phase_action() {
    for torpid in [false, true] {
        let mut e = game(2);
        let s = e.add_minion(SeatId(0), find_card("Silas"), 4, torpid);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        fill_hand(&mut e, 0);
        library(&mut e, 0, &["Conceal", "Conceal", "Conceal"]);
        if torpid {
            never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
                uses_vampire(o, s, 0)
            });
            continue;
        }
        until(&mut e, SeatId(0), WindowLabel::DiscardPhase);
        let dpa = e.state.seats[0].dpa;
        use_vamp(&mut e, s, 0);
        request(&mut e);
        assert_eq!(blood(&e, s), 3);
        assert_eq!(e.state.seats[0].dpa, dpa + 1);
    }
}

/// Slag: "While Slag is ready, Methuselahs playing or discarding cards in a
/// referendum shows you their replacements." Another Methuselah burns a
/// political action card for a vote: Slag's controller is shown the card it
/// drew; torpid Slag, nothing.
#[test]
fn slag_sees_the_replacements_drawn_in_a_referendum() {
    for torpid in [false, true] {
        let mut e = game(2);
        let caller = e.add_minion(SeatId(0), find_card("Alexa Draper"), 5, false);
        e.add_minion(SeatId(0), find_card("Slag"), 4, torpid);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Ancilla Empowerment"]);
        hand(&mut e, 1, &["Neonate Breach"]);
        fill_hand(&mut e, 1);
        library(&mut e, 1, &["Villein"]);
        pick(&mut e, play(0, 0, caller));
        to_window(&mut e, SeatId(1), WindowLabel::ReferendumPolling);
        e.notes = Some(Vec::new());
        pick(
            &mut e,
            Choice::BurnForVote {
                card: 0,
                favor: true,
            },
        );
        request(&mut e);
        let looks = noted_looks(&mut e);
        if torpid {
            assert!(looks.is_empty(), "{looks:?}");
            continue;
        }
        assert_eq!(looks.len(), 1);
        let l = &looks[0];
        assert_eq!((l.viewer, l.seat, l.library), (SeatId(0), SeatId(1), false));
        assert_eq!(l.cards, vec![find_library("Villein")]);
    }
}

/// Slag: the replacement taken through a draw alternative (Black Market
/// Cache's Kevlar Vest, not the library's Villein) is shown too: whatever
/// the replacement put into the hand.
#[test]
fn slag_sees_a_replacement_taken_from_a_draw_alternative() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), find_card("Alexa Draper"), 5, false);
    e.add_minion(SeatId(0), find_card("Slag"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let cache = in_play(&mut e, 1, "Black Market Cache", None);
    e.state.cards[cache.index()]
        .stored
        .push(find_library("Kevlar Vest"));
    hand(&mut e, 0, &["Ancilla Empowerment"]);
    hand(&mut e, 1, &["Neonate Breach"]);
    fill_hand(&mut e, 1);
    library(&mut e, 1, &["Villein"]);
    pick(&mut e, play(0, 0, caller));
    to_window(&mut e, SeatId(1), WindowLabel::ReferendumPolling);
    e.notes = Some(Vec::new());
    pick(
        &mut e,
        Choice::BurnForVote {
            card: 0,
            favor: true,
        },
    );
    pick(
        &mut e,
        Choice::DrawStored {
            card: cache.0,
            stored: find_library("Kevlar Vest"),
        },
    );
    request(&mut e);
    let looks = noted_looks(&mut e);
    assert_eq!(looks.len(), 1, "{looks:?}");
    let l = &looks[0];
    assert_eq!((l.viewer, l.seat, l.library), (SeatId(0), SeatId(1), false));
    assert_eq!(l.cards, vec![find_library("Kevlar Vest")]);
}

/// Sylvie Helgon: "During an action, Sylvie can discard a political action
/// card to get +1 stealth or +1 intercept." Blocked by Mordechai Ben-Nun
/// (+1 intercept), she discards one for +1 stealth, not a second in that
/// action (still short); attempting to block a hunt (+1 stealth), one for +1 intercept.
#[test]
fn sylvie_helgon_discards_politics_for_stealth_or_intercept() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), find_card("Sylvie Helgon"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), find_card("Mordechai Ben-Nun"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Neonate Breach", "Neonate Breach"]);
    fill_hand(&mut e, 0);
    library(&mut e, 0, &["Conceal", "Conceal"]);
    pick(&mut e, bleed(s));
    pick_when(&mut e, block(z));
    use_vamp(&mut e, s, 0);
    request(&mut e);
    assert_eq!(copies(&e, 0, "Neonate Breach"), (1, 1, 0));
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
        uses_vampire(o, s, 0)
    });

    let mut e = game(2);
    let h = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let s = e.add_minion(SeatId(1), find_card("Sylvie Helgon"), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 1, &["Neonate Breach"]);
    fill_hand(&mut e, 1);
    library(&mut e, 1, &["Conceal"]);
    e.state.minions[h.index()].blood = 1;
    pick(&mut e, action(h, ActionKind::Hunt));
    pick_when(&mut e, block(s));
    use_vamp(&mut e, s, 1);
    request(&mut e);
    assert_eq!(copies(&e, 1, "Neonate Breach"), (1, 0, 0));
    assert!(e.combat().is_some(), "the hunt blocked");
}

/// Tanginé: "When in combat with a Salubri, Tanginé cannot strike except to
/// dodge or end combat." Against a Salubri her hand strike is not offered,
/// a Dodge and not striking are; against a Brujah, her hand strike is.
#[test]
fn tangine_only_dodges_or_ends_against_a_salubri() {
    for salubri in [true, false] {
        let mut e = game(2);
        let t = e.add_minion(SeatId(0), find_card("Tanginé"), 6, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let clan = if salubri { "Salubri" } else { "Brujah" };
        let z = e.add_minion(SeatId(1), plain(clan, 1), 6, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Dodge"]);
        fill_hand(&mut e, 0);
        pick(&mut e, bleed(t));
        pick_when(&mut e, block(z));
        let req = offering(&mut e, play(0, 0, t));
        assert_eq!(req.window, WindowLabel::CombatStrike);
        assert_eq!(req.options.contains(&Choice::HandStrike), !salubri);
        assert_eq!(req.options.contains(&Choice::Pass), salubri);
    }
}

/// Tanginé: "She may inflict 1 damage on any non-Salubri minion or retainer
/// as a Ⓓ action." Another Methuselah's ally loses 1 life, a retainer 1; a
/// Salubri is no target.
#[test]
fn tangine_wounds_a_non_salubri_minion_or_a_retainer() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), find_card("Tanginé"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let salubri = e.add_minion(SeatId(1), plain("Salubri", 1), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Political Ally"));
    for m in [salubri, ally] {
        e.state.minions[m.index()].locked = true;
    }
    let raven = in_play(&mut e, 1, "Raven Spy", Some(salubri));
    e.state.cards[raven.index()].counters = 2;
    let hit = |m| Choice::UseAbility {
        source: AbilitySource::Vampire(t),
        ability: 0,
        actor: Some(t),
        target: Target::Minion(m),
    };
    let req = request(&mut e);
    assert!(req.options.contains(&hit(ally)), "{:?}", req.options);
    assert!(!req.options.contains(&hit(salubri)));
    let life = blood(&e, ally);
    pick(&mut e, hit(ally));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, ally), life - 1);

    let mut e = game(2);
    let t = e.add_minion(SeatId(0), find_card("Tanginé"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[z.index()].locked = true;
    let raven = in_play(&mut e, 1, "Raven Spy", Some(z));
    e.state.cards[raven.index()].counters = 2;
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Vampire(t),
            ability: 1,
            actor: Some(t),
            target: Target::Card(raven),
        },
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[raven.index()].counters, 1);
}

/// Titus Camille: "Titus can steal equipment from a minion controlled by
/// your predator or prey as a Ⓓ action. +1 strength." At four seats, his
/// prey's .44 Magnum is offered and taken, the cross-table seat's is not.
#[test]
fn titus_camille_steals_from_his_predator_or_prey() {
    let mut e = game(4);
    let t = e.add_minion(SeatId(0), find_card("Titus Camille"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let mut guns = Vec::new();
    for s in 1..4u8 {
        let v = e.add_minion(SeatId(s), plain("Ventrue", s as usize), 4, false);
        e.state.minions[v.index()].locked = true;
        guns.push(in_play(&mut e, s, ".44 Magnum", Some(v)));
    }
    let steal = |c| Choice::UseAbility {
        source: AbilitySource::Vampire(t),
        ability: 0,
        actor: Some(t),
        target: Target::Card(c),
    };
    let req = request(&mut e);
    assert!(req.options.contains(&steal(guns[0])), "prey");
    assert!(req.options.contains(&steal(guns[2])), "predator");
    assert!(!req.options.contains(&steal(guns[1])), "across the table");
    pick(&mut e, steal(guns[0]));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[guns[0].index()].host, Some(t));
}

/// Troglodytia: "After any Methuselah plays a master card, you may look at
/// that Methuselah's hand (after the replacement card is drawn)." Seat 0's
/// master, canceled by seat 1's Wash: Troglodytia's controller looks at seat
/// 1's hand without Wash's replacement (not replaced until its unlock
/// phase), and at seat 0's with its replacement.
/// [ANK 20180512] a master not replaced at once: the hand without it.
#[test]
fn troglodytia_looks_after_each_master_card() {
    let mut e = game(3);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), find_card("Troglodytia"), 4, true);
    hand(&mut e, 0, &["Channel 10"]);
    fill_hand(&mut e, 0);
    library(&mut e, 0, &["Conceal"]);
    hand(&mut e, 1, &["Wash"]);
    fill_hand(&mut e, 1);
    library(&mut e, 1, &["Conceal"]);
    let size = e.state.hand_size(SeatId(1));
    e.at_master_phase();
    e.notes = Some(Vec::new());
    pick(&mut e, master_play(0));
    for _ in 0..20 {
        let req = request(&mut e);
        if req.seat == SeatId(1) && req.options.contains(&master_play(0)) {
            pick(&mut e, master_play(0));
            break;
        }
        e.choose(fallback(&req));
    }
    request(&mut e);
    let looks = noted_looks(&mut e);
    let at = |s: u8| {
        looks
            .iter()
            .find(|l| (l.viewer, l.seat) == (SeatId(2), SeatId(s)))
            .unwrap_or_else(|| panic!("no look at {s}: {looks:?}"))
    };
    assert_eq!(at(1).cards.len(), size - 1, "Wash not replaced yet");
    assert!(!at(1).cards.contains(&find_library("Wash")));
    assert!(looks.iter().all(|l| l.viewer == SeatId(2)));
    assert!(
        at(0).cards.contains(&find_library("Conceal")),
        "the canceled master's replacement drawn"
    );
}

/// Uriel: "In combat, the controller of the minion opposing Uriel plays
/// with an open hand." Seat 1 sees seat 0's hand while Uriel blocks, not
/// before.
#[test]
fn uriel_opens_his_opponents_hand_in_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let u = e.add_minion(SeatId(1), find_card("Uriel"), 4, false);
    hand(&mut e, 0, &["Immortal Grapple"]);
    assert!(!e.state.hand_visible_to(SeatId(0), SeatId(1)));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(u));
    request(&mut e);
    assert!(e.combat().is_some());
    assert!(e.state.hand_visible_to(SeatId(0), SeatId(1)));
}

/// Veejay Vinod: "During an action, Veejay can burn 2 blood to get +1
/// stealth." Blocked by Mordechai Ben-Nun, 2 blood for 1 stealth, once in
/// that action though still short.
#[test]
fn veejay_vinod_buys_one_stealth_for_two_blood() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), find_card("Veejay Vinod"), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), find_card("Mordechai Ben-Nun"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    pick(&mut e, bleed(v));
    pick_when(&mut e, block(z));
    use_vamp(&mut e, v, 0);
    assert_eq!(blood(&e, v), 1);
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
        uses_vampire(o, v, 0)
    });
}

/// Virginie, Prodigy: "During an action, Virginie can burn 1 blood to get
/// +1 bleed." Once in her bleed, for 2; not offered in a hunt.
/// [ANK 20231124] [LSJ 19970718] during a bleed, before resolution.
#[test]
fn virginie_bleeds_harder_once_before_resolution() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), find_card("Virginie, Prodigy"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    pick(&mut e, bleed(v));
    use_vamp(&mut e, v, 0);
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
        uses_vampire(o, v, 0)
    });
    assert_eq!((e.state.seats[1].pool, blood(&e, v)), (10 - 2, 3));

    let mut e = game(2);
    let v = e.add_minion(SeatId(0), find_card("Virginie, Prodigy"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    pick(&mut e, action(v, ActionKind::Hunt));
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
        uses_vampire(o, v, 0)
    });
}

/// Xeper, Sultan of Lepers: "Xeper gets +1 vote for each ready unique
/// infernal vampire you control." Himself and Giotto Verducci: 2, Giotto
/// locked still; Giotto in torpor: 1. Untitled; granted a title of 2
/// votes, he casts 2 + 2.
/// [LSJ 20100601] not titled, his votes added to a title's.
#[test]
fn xeper_votes_for_each_ready_unique_infernal() {
    let mut e = game(2);
    let x = e.add_minion(SeatId(0), find_card("Xeper, Sultan of Lepers"), 5, false);
    let g = e.add_minion(SeatId(0), find_card("Giotto Verducci"), 4, false);
    e.add_minion(SeatId(1), find_card("Arishat"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let printed = |e: &Engine| e.state.printed_mod(x, Stat::Votes, PrintedCtx::default());
    assert_eq!(
        printed(&e),
        2,
        "the other Methuselah's infernal not counted"
    );
    assert_eq!(e.state.title_of(x), Title::None);
    e.state.minions[g.index()].locked = true;
    assert_eq!(printed(&e), 2, "ready, not unlocked");
    e.state.minions[g.index()].in_torpor = true;
    assert_eq!(printed(&e), 1);
    e.state.minions[g.index()].in_torpor = false;
    e.state.minions[g.index()].locked = false;
    e.state.minions[x.index()].granted_title = Some(Title::Votes(2));
    hand(&mut e, 0, &["Kine Resources Contested"]);
    pick(&mut e, play(0, 0, x));
    pick(&mut e, alloc(0, 3));
    to_window(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
    let r = e.referendum().expect("referendum");
    assert_eq!(r.votes_of(&e.state, x), 4);
}

/// Xeper, Sultan of Lepers: "He can steal an ally controlled by another
/// Methuselah as a Ⓓ action." The ally changes controller.
#[test]
fn xeper_steals_an_ally_of_another_methuselah() {
    let mut e = game(2);
    let x = e.add_minion(SeatId(0), find_card("Xeper, Sultan of Lepers"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Political Ally"));
    for m in [z, ally] {
        e.state.minions[m.index()].locked = true;
    }
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Vampire(x),
            ability: 0,
            actor: Some(x),
            target: Target::Minion(ally),
        },
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[ally.index()].owner, SeatId(0));
}

// ---- Rulings across the round -----------------------------------------------

/// Papillon: "Requires a ready vampire with a city title": a prince, a
/// baron or an archbishop (rules-qa 112.21). Tereza Rossi, a Sabbat
/// archbishop (+1 strength, inert here), alone lets it be played.
#[test]
fn papillon_plays_with_only_a_ready_archbishop() {
    let table = |torpor: bool| {
        let mut e = game(2);
        e.add_minion(SeatId(0), find_card("Tereza Rossi"), 4, torpor);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Papillon"]);
        e.at_master_phase();
        e
    };
    let mut e = table(true);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, master_play(0));
    let mut e = table(false);
    pick(&mut e, master_play(0));
    request(&mut e);
    let c = card_named(&e, "Papillon");
    assert!(!e.state.cards[c.index()].effect_only);
    assert_eq!(e.state.seats[0].pool, 7);
}

/// Blissful Agony [VAL]: "Choose a minion controlled by a Methuselah other
/// than the opposing vampire's controller." The striker itself and its
/// Methuselah's other minions are offered (owner, 2026-11-10), the
/// opposing controller's are not; the striker chosen, the opposing vampire
/// enters a new combat with it, going first.
#[test]
fn blissful_agony_superior_may_choose_the_striker_itself() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), find_card("Langa"), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let y = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.add_minion(SeatId(2), plain("Toreador", 0), 4, false);
    hand(&mut e, 0, &["Blissful Agony"]);
    blocked(&mut e, a, z);
    let strike = |t: MinionId| Choice::Play {
        card: 0,
        mode: 2,
        actor: Some(a),
        target: Target::Minion(t),
    };
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert!(req.options.contains(&strike(b)), "the striker's Methuselah");
    assert!(
        !req.options.contains(&strike(y)),
        "the blocker's Methuselah"
    );
    pick(&mut e, strike(a));
    e.notes = Some(Vec::new());
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let begun: Vec<[MinionId; 2]> = noted_combats(&mut e)
        .into_iter()
        .filter_map(|n| match n {
            zvtes::combat::CombatNote::Begins { minions } => Some(minions),
            _ => None,
        })
        .collect();
    assert_eq!(begun, vec![[z, a]], "the blocker first");
}

/// Eye of Unforgiving Heaven against a dodge: the dodge protects the dodger
/// only (rulebook "Dodge"; owner, 2026-11-10): the blocker is untouched,
/// its demon retainer (Charnas the Imp, 1 life) takes the 2 and burns.
#[test]
fn eye_of_unforgiving_heaven_reaches_a_dodgers_demon_retainer() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Nkechi"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let imp = in_play(&mut e, 1, "Charnas the Imp", Some(z));
    e.state.cards[imp.index()].counters = 1;
    hand(&mut e, 0, &["Eye of Unforgiving Heaven"]);
    hand(&mut e, 1, &["Dodge"]);
    blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 4, "dodged");
    assert!(!e.state.cards[imp.index()].present(), "its retainer is not");
}

/// [LSJ 20100211] Pressing Flesh "can be used on an Imbued, it comes back
/// as a zombie ally and is not an Imbued anymore. It has no Creed and
/// cannot use Virtues, nor does it get Convictions." The prey's burned
/// imbued (Jennifer: her text inert here) comes to the player's ready
/// region with this card on it: the same crypt card, an ally, no creed, no
/// virtue, its starting life and 1 more; burned again, to its owner's ash
/// heap.
#[test]
fn pressing_flesh_raises_a_burned_imbued_as_a_zombie_ally() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("THN", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let card = find_card("Jennifer \"Flame61\" Vidisania");
    let life = CRYPT[card as usize].capacity;
    let virtue = CRYPT[card as usize].disciplines[0];
    let imbued = e.add_minion(SeatId(1), card, life, false);
    hand(&mut e, 0, &["Pressing Flesh"]);
    zvtes::copies::number_unnumbered(&mut e);
    e.burn(imbued);
    assert!(e.state.seats[1].ash_crypt.contains(&card));
    pick(&mut e, act(0, 1, a, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let z = zombie(&e);
    assert_ne!(z, imbued, "a new minion");
    assert!(!e.state.seats[1].ash_crypt.contains(&card));
    let v = &e.state.minions[z.index()];
    assert_eq!((v.owner, v.crypt_card()), (SeatId(0), Some(card)));
    assert!(v.is_ally() && !v.is_imbued(), "not an imbued anymore");
    assert_eq!(e.state.creed(z), None, "no creed");
    assert_eq!(e.state.discipline_level(z, virtue), 0, "no virtue");
    assert!(e.state.convictions(z).is_empty(), "no conviction");
    assert!(e.state.has_subtype(z, zvtes::library::Label::Zombie));
    assert_eq!(blood(&e, z), life + 1, "its starting life and 1 more");
    e.burn(z);
    assert!(e.state.seats[1].ash_crypt.contains(&card), "its owner's");
}

/// Din of the Damned: equipment "cannot grant intercept" — a minus is no
/// grant: the hunter's Lambreta still gives the blocker -1 intercept.
#[test]
fn din_of_the_damned_keeps_an_equipments_minus_intercept() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), harbinger(), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let bike = in_play(&mut e, 0, "Lambreta", Some(a));
    hand(&mut e, 0, &["Din of the Damned"]);
    pick(&mut e, action(a, ActionKind::Hunt));
    pick_when(&mut e, act(0, 0, a, Target::None));
    pick_when(&mut e, block(z));
    pick_when(&mut e, use_on(bike, 0, Target::None));
    let have = e.action().expect("hunt").intercept_of(&e.state, z);
    assert_eq!(have, -1, "Lambreta's -1 kept");
}

/// Din of the Damned's toll, "an additional blood or life": an ally playing
/// a reaction granting intercept pays it in life. Herald of Topheth (5
/// life, flight) blocks a hunt with Patrol ("+1 intercept", Flight), then
/// fights: 1 life fewer with Din than without.
#[test]
fn din_of_the_damned_toll_is_paid_in_life_by_an_ally() {
    let left = [true, false].map(|din| {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), harbinger(), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let h = e.add_ally(SeatId(1), find_library("Herald of Topheth"));
        if din {
            hand(&mut e, 0, &["Din of the Damned"]);
        }
        hand(&mut e, 1, &["Patrol"]);
        pick(&mut e, action(a, ActionKind::Hunt));
        if din {
            pick_when(&mut e, act(0, 0, a, Target::None));
        }
        pick_when(&mut e, block(h));
        pick_when(&mut e, act(0, 0, h, Target::None));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(e.state.seats[1].ash.contains(&find_library("Patrol")));
        blood(&e, h)
    });
    assert_eq!(left[0] + 1, left[1], "the toll in life");
}

/// Dagger out of play after its ranged strike, its bearer burned meanwhile:
/// it burns as it would return (a minion's cards burn with it).
#[test]
fn dagger_out_of_play_burns_with_its_burned_bearer() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 5, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let d = on_with(&mut e, 0, "Dagger", a, &[]);
    on_with(
        &mut e,
        0,
        "Amulet of Temporal Perception",
        a,
        &[zvtes::cards::Modifier::HostCombatPress(1)],
    );
    blocked(&mut e, a, z);
    pick_when(&mut e, ability(d.0, 1));
    pick_when(&mut e, Choice::UsePress);
    assert!(e.state.cards[d.index()].out_of_play);
    e.burn(a);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[d.index()].present());
    assert!(e.state.seats[0].ash.contains(&find_library("Dagger")));
}

/// Draeven Softfoot: "Cards requiring Chimerstry [chi] can neither be
/// played while he is acting, blocking, or in combat, nor target him."
/// Netwar [chi] ("choose another ready minion you control. The chosen
/// minion unlocks and attempts to block"), his own Methuselah's: offered on
/// a locked vampire and a locked mortal ally, never on the locked Draeven.
#[test]
fn draeven_softfoot_is_no_target_of_chimerstry() {
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[r.index()].sect_override = Some(Sect::Anarch);
    e.state.minions[r.index()].gained_levels = vec!["chi"];
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let d = e.add_ally(SeatId(0), find_library("Draeven Softfoot"));
    let p = e.add_ally(SeatId(0), find_library("Political Ally"));
    for m in [b, d, p] {
        e.state.minions[m.index()].locked = true;
    }
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Netwar"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(z));
    let req = offering(&mut e, act(0, 1, r, Target::Minion(b)));
    assert!(
        req.options.contains(&act(0, 1, r, Target::Minion(p))),
        "an ally"
    );
    assert!(!req.options.contains(&act(0, 1, r, Target::Minion(d))));
}

/// Draeven Softfoot: "Cards requiring Chimerstry [chi] can neither be
/// played while he is acting, blocking, or in combat, nor target him." A
/// card printing [chi] targets him at none of its modes (rules-digest D29,
/// rules-qa 17.4): Shattering [pro] ("inflict 2 unpreventable damage on a
/// ready minion") is offered on another ally, never on Draeven.
#[test]
fn draeven_softfoot_is_no_target_of_a_chimerstry_card_at_another_mode() {
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[r.index()].sect_override = Some(Sect::Anarch);
    e.state.minions[r.index()].gained_levels = vec!["pro"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let d = e.add_ally(SeatId(1), find_library("Draeven Softfoot"));
    let p = e.add_ally(SeatId(1), find_library("Political Ally"));
    hand(&mut e, 0, &["Shattering"]);
    let req = offering(&mut e, act(0, 2, r, Target::Minion(p)));
    assert!(!req.options.contains(&act(0, 2, r, Target::Minion(d))));
}

/// Draeven Softfoot: "Draeven can lock to prevent 1 damage inflicted in
/// combat on a Kiasyd." The text sets no bar on his own combat: rushed by a
/// Kiasyd (Bum's Rush), still unlocked, he may lock to spare it 1 of the
/// damage he dealt.
#[test]
fn draeven_softfoot_prevents_in_his_own_combat() {
    let mut lost = Vec::new();
    for prevents in [false, true] {
        let mut e = game(2);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let d = e.add_ally(SeatId(0), find_library("Draeven Softfoot"));
        let k = e.add_minion(SeatId(1), of_clan("Kiasyd", 0), 5, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 1, &["Bum's Rush"]);
        let card = card_named(&e, "Draeven Softfoot");
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        pick(&mut e, act(0, 0, k, Target::Minion(d)));
        if prevents {
            pick_when(&mut e, use_on(card, 0, Target::Minion(k)));
            assert!(e.state.minions[d.index()].locked);
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        lost.push(5 - blood(&e, k));
    }
    assert_eq!(lost[1] + 1, lost[0], "1 prevented");
}

/// Jar of Skin Eaters' action fizzling (its target burned before
/// resolution): no effect, the Jar and its blood kept ("burn after use"
/// needs a use).
#[test]
fn jar_of_skin_eaters_fizzled_action_keeps_the_jar() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), of_clan("Tremere", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 3, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let jar = on_with(&mut e, 0, "Jar of Skin Eaters", t, &[]);
    e.state.cards[jar.index()].counters = 1;
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Card(jar),
            ability: 1,
            actor: Some(t),
            target: Target::Minion(z),
        },
    );
    request(&mut e);
    e.burn(z);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let c = &e.state.cards[jar.index()];
    assert!(c.present() && c.counters == 1, "kept, its blood too");
}

/// Journal of Hrorsh: a fizzled action counts as a successful one for its
/// counter (rules-digest 3.2): its Salubri's Bum's Rush, the target burned
/// before resolution, adds 1.
#[test]
fn journal_of_hrorsh_counts_a_fizzled_action() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), of_clan("Salubri", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let j = on_with(&mut e, 0, "Journal of Hrorsh", s, &[]);
    hand(&mut e, 0, &["Bum's Rush"]);
    pick(&mut e, act(0, 0, s, Target::Minion(z)));
    request(&mut e);
    e.burn(z);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[j.index()].counters, 1);
}

/// Journal of Hrorsh taken by another Salubri: "burn these counters if
/// this card is moved", then the take, a successful action of its new
/// Salubri, adds 1 (the take is no equip; rules-qa 3.7, 80.44).
#[test]
fn journal_of_hrorsh_taken_by_a_salubri_restarts_at_one() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), of_clan("Salubri", 0), 4, false);
    let t = e.add_minion(SeatId(0), of_clan("Salubri", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let j = on_with(&mut e, 0, "Journal of Hrorsh", s, &[]);
    e.state.cards[j.index()].counters = 2;
    pick(&mut e, take_from(t, s));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[j.index()].host, Some(t));
    assert_eq!(e.state.cards[j.index()].counters, 1, "2 burned, 1 added");
}

/// Tinglestripe: "This weapon can be used to strike only once each round":
/// either of its strikes, an additional strike included. At [MYT], its 2R
/// strike used, Blur's additional strike this round offers neither: a hand
/// strike, elided.
#[test]
fn tinglestripe_strikes_once_a_round_across_both_strikes() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("cel", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let t = on_with(&mut e, 0, "Tinglestripe", a, &[]);
    e.state.cards[t.index()].mode = 1;
    hand(&mut e, 0, &["Blur"]);
    blocked(&mut e, a, z);
    pick_when(&mut e, ability(t.0, 0));
    pick_when(&mut e, play(0, 0, a));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        uses_ability(t, 0)(o) || uses_ability(t, 1)(o)
    });
    assert_eq!(blood(&e, z), 6 - 2 - 1, "2R, then a hand strike");
}

/// Ritual Scalpel's cut: "inflict 1 unpreventable damage on a mortal …
/// to gain 2 blood": the 2 blood come even if the damage burns the mortal
/// (a 1-life Political Ally).
#[test]
fn ritual_scalpel_feeds_even_when_the_mortal_burns() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), nagaraja(), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Political Ally"));
    assert_eq!(blood(&e, ally), 1);
    let s = on_with(&mut e, 0, "Ritual Scalpel", n, &[]);
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Card(s),
            ability: 0,
            actor: Some(n),
            target: Target::Minion(ally),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[ally.index()].in_play, "burned");
    assert_eq!(blood(&e, n), 1 + 2);
}

/// Skaald: "Any older non-titled Gangrel can steal this card as a +1
/// stealth Ⓓ action": the stolen card moves onto the stealing Gangrel,
/// under its Methuselah (as Saankaláxt's steal, rules-qa 6.2), not only
/// its control.
#[test]
fn skaald_stolen_moves_to_the_stealing_gangrel() {
    let mut e = game(2);
    let young = textless(|c| c.clan == "Gangrel" && c.capacity == 5, 0);
    let old = textless(|c| c.clan == "Gangrel" && c.capacity > 5, 0);
    let g = e.add_minion(SeatId(0), young, 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let thief = e.add_minion(SeatId(1), old, 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let skaald = in_play(&mut e, 0, "Skaald", Some(g));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, used_by(skaald, 0, thief));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let c = &e.state.cards[skaald.index()];
    assert_eq!(c.host, Some(thief), "on the stealing Gangrel");
    assert_eq!(e.state.card_controller(skaald), SeatId(1));
}

/// Azrael: "He cannot press to end combat." With a press each, he may
/// press to continue; once the blocker pressed to continue, his press to
/// end it is not offered (the step elided): the next press step is round
/// 2's.
#[test]
fn azrael_cannot_press_to_end_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Azrael"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    for m in [a, z] {
        on_with(
            &mut e,
            if m == a { 0 } else { 1 },
            "Amulet of Temporal Perception",
            m,
            &[zvtes::cards::Modifier::HostCombatPress(1)],
        );
    }
    blocked(&mut e, a, z);
    let req = until(&mut e, SeatId(0), WindowLabel::CombatPress);
    assert!(req.options.contains(&Choice::UsePress), "to continue");
    pick(&mut e, Choice::Pass);
    let req = expect(&mut e, SeatId(1), WindowLabel::CombatPress);
    assert!(req.options.contains(&Choice::UsePress));
    pick(&mut e, Choice::UsePress);
    let req = request(&mut e);
    assert_eq!(
        e.combat().expect("combat").round,
        2,
        "the press step elided"
    );
    assert_eq!(req.window, WindowLabel::CombatPress, "round 2's");
}

/// Grotesque put in play without entering (no location chosen "as this
/// Grotesque enters play"): its controller controls no location, yet the
/// removal clause never fires — it has no location to lose.
#[test]
fn grotesque_put_in_play_without_a_binding_stays() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let g = e.add_minion(SeatId(0), find_card("Grotesque"), 3, false);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[g.index()].in_play);
    assert!(!e.state.seats[0]
        .removed_crypt
        .contains(&find_card("Grotesque")));
}
