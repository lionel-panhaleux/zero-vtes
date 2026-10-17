//! A decision's prompt in English (docs/protocol.md §5): what is chosen,
//! when its window and options alone do not say — the terms of a play,
//! an ability or a referendum, the choice an effect asks. Told from the
//! decision's `purpose`, `source` and `about`; `None` where none of them
//! says more than the options.

use zvtes_api::ids::{Purpose, Window};
use zvtes_api::message::{About, Decision};
use zvtes_api::option::CardRef;

use crate::Names;

/// The prompt of a decision.
pub fn prompt(d: &Decision, n: &Names) -> Option<String> {
    let none = About::default();
    let a = d.about.as_ref().unwrap_or(&none);
    let p = Prompt { d, a, n };
    match d.purpose {
        Some(purpose) => p.asked(purpose),
        None => p.terms(),
    }
}

struct Prompt<'a> {
    d: &'a Decision,
    a: &'a About,
    n: &'a Names<'a>,
}

impl Prompt<'_> {
    fn minion(&self) -> Option<String> {
        self.a.minion.map(|c| self.n.minion_bare(c))
    }

    fn to(&self) -> Option<String> {
        self.a.to.map(|c| self.n.minion_bare(c))
    }

    fn card(&self) -> Option<String> {
        self.a.card.as_ref().map(|c| card(c, self.n))
    }

    fn clan(&self) -> Option<String> {
        self.a.clan.as_deref().map(|c| self.n.value(c))
    }

    fn seat(&self) -> Option<String> {
        self.a.seat.map(|s| self.n.seat(s))
    }

    /// "your" or "Seat's".
    fn whose(&self) -> Option<String> {
        let s = self.a.seat?;
        Some(match s == self.d.seat {
            true => "your".into(),
            false => format!("{}'s", self.n.seat(s)),
        })
    }

    /// "of yours" or "of Seat".
    fn of(&self) -> Option<String> {
        let s = self.a.seat?;
        Some(match s == self.d.seat {
            true => "of yours".into(),
            false => format!("of {}", self.n.seat(s)),
        })
    }

    fn detail(&self) -> Option<&str> {
        self.a.detail.as_deref()
    }

    /// The prompt of a choice an effect asks.
    fn asked(&self, p: Purpose) -> Option<String> {
        use Purpose as P;
        let a = self.a;
        Some(match p {
            P::GiveBlood => format!(
                "Choose the vampire who receives 1 blood from {}",
                self.minion()?
            ),
            P::AnarchConvert => match self.detail() {
                Some("chosen") => "Gain 1 pool or draw 1 card from your crypt".into(),
                _ => format!(
                    "{} enters play: you can remove him from the game to make a non-titled \
                     vampire you control Anarch",
                    self.minion()?
                ),
            },
            P::DiablerieEquipment => format!(
                "Choose the equipment {} takes from {}",
                self.to()?,
                self.minion()?
            ),
            P::MoveBetween => match self.detail() {
                None => format!(
                    "Choose the vampire who receives blood, equipment and retainers from {}",
                    self.minion()?
                ),
                Some("blood") => format!(
                    "How much blood moves from {} to {}?",
                    self.minion()?,
                    self.to()?
                ),
                _ => format!(
                    "Move equipment or retainers from {} to {}",
                    self.minion()?,
                    self.to()?
                ),
            },
            P::PayOrBurn => format!("{} burns unless you burn {} pool", self.card()?, a.n?),
            P::LockOrDiscard => format!("{} is locked unless you discard a card", self.minion()?),
            P::BloodOrBurned => match self.card() {
                Some(c) => format!("{} burns {} blood or {c} is burned", self.minion()?, a.n?),
                None => format!("{} burns {} blood or is burned", self.minion()?, a.n?),
            },
            P::StoreSearch => format!(
                "Search your library for non-unique equipment to put on {} ({} more at most)",
                self.card()?,
                a.n?
            ),
            P::EntryBurn => format!(
                "{} entered play: burn an ally or retainer you control",
                self.minion()?
            ),
            P::MayPutOn => format!("You can put {} on {}", self.card()?, self.minion()?),
            P::MayAddCounter => format!("You can add 1 counter to {}", self.card()?),
            P::MayGainPool => format!("{}: you can gain {} pool", self.minion()?, a.n?),
            P::EntryRemoveOrBurn => format!(
                "{} burns unless you remove an ally or vampire in your ash heap from the game",
                self.minion()?
            ),
            P::BlockToll => format!(
                "{}'s block attempt has a toll: pay it or resign the attempt",
                self.minion()?
            ),
            P::CancelAttempt => format!("{} can cancel its block attempt", self.minion()?),
            P::Reorder => format!(
                "Reorder the top {} cards of your library: choose card {} from the top",
                a.n?,
                a.m? + 1
            ),
            P::MinionOf => {
                let of = self.of()?;
                match self.detail()? {
                    "lock" => format!("You can lock a minion {of}"),
                    "corrupt" => format!("Choose a minion {of} to get a corruption counter"),
                    "lock_vampires" => format!("Lock up to two vampires {of}"),
                    "unlock_anarch" => "You can unlock a ready Anarch of yours".into(),
                    "burn_blood" => {
                        format!("Choose a ready {} of yours to burn 1 blood", self.clan()?)
                    }
                    "host_card" => format!("Choose the minion {of} who gets {}", self.card()?),
                    "lock_oldest" => {
                        format!("Choose one of the oldest {} {of} to lock", self.clan()?)
                    }
                    "oldest_gain_blood" => {
                        "Choose one of your two oldest ready vampires to gain 1 blood".into()
                    }
                    "burn_crowded_clan" => {
                        "Choose a vampire to burn, of a clan you control 3 or more of".into()
                    }
                    "damage" => format!(
                        "Choose a ready minion of yours to take {} unpreventable damage",
                        a.n?
                    ),
                    _ => return None,
                }
            }
            P::CorruptionOwner => format!(
                "Choose whose corruption counter on {} burns",
                self.minion()?
            ),
            P::AvoidDamage => format!(
                "{} takes {} unpreventable aggravated damage unless it burns {} blood",
                self.minion()?,
                a.m?,
                a.n?
            ),
            P::DiscardFromHandOf => format!(
                "Discard a card from {}'s hand (they replace it)",
                self.seat()?
            ),
            P::DiscardChosen => match self.detail() {
                Some("combat") => format!("Discard {} combat card(s) of your choice", a.n?),
                _ => format!("Discard {} card(s) of your choice", a.n?),
            },
            P::RemoveFromAsh => format!(
                "Remove {} card(s) in {} ash heap from the game",
                a.n?,
                self.whose()?
            ),
            P::DisciplineSearch => format!(
                "You can search for a Discipline master card to put on {}",
                self.minion()?
            ),
            P::MayDrawExtra => format!(
                "{}: you can draw an additional card (discard afterward)",
                self.card()?
            ),
            P::MayTorture => format!(
                "The Path of Lilith: you can lock it to put a torture counter on {}",
                self.minion()?
            ),
            P::PursuitDraw => format!("{}: replace a card and draw more", self.card()?),
            P::PursuitDiscard => format!("{}: discard {} card(s)", self.card()?, a.n?),
            P::PurgePick => format!(
                "Choose a ready vampire of {} not older than {} to go to torpor",
                self.seat()?,
                self.minion()?
            ),
            P::MoveBloodUpTo => format!(
                "How much blood does {} move to {}?",
                self.minion()?,
                self.to()?
            ),
            P::StealUpTo => format!(
                "How much blood does {} steal from {}?",
                self.to()?,
                self.minion()?
            ),
            P::AllySearch => format!(
                "{}: you can search for a mage ally to put in play",
                self.minion()?
            ),
            P::Redraw => format!(
                "{}: you can put {} on the bottom of your library and draw a new card",
                self.minion()?,
                self.card()?
            ),
            P::RemoveUncontrolled => {
                "Remove a crypt card in your uncontrolled region from the game".into()
            }
            P::ChooseForCard => {
                let clan = self.clan().unwrap_or_else(|| "vampire".into());
                let who = match self.detail()? {
                    "unblockable_non_bleed" => {
                        format!("the {clan} whose non-bleed actions cannot be blocked")
                    }
                    "does_not_unlock" => format!("the {clan} who does not unlock as normal"),
                    _ => format!("the card's chosen {clan}"),
                };
                format!("{}: choose {who}", self.card()?)
            }
            P::ExchangeAnarch => match self.card() {
                Some(c) => format!("Choose the card in your hand to exchange for {c}"),
                None => "Choose a card requiring an Anarch in your ash heap, to exchange with \
                         a card in your hand"
                    .into(),
            },
            P::PoolForCard => format!("You can burn {} to gain {} pool", self.card()?, a.n?),
            P::LockForPool => format!("You can lock {} to gain {} pool", self.card()?, a.n?),
            P::StealForCorruption => format!(
                "You can burn the corruption counters on {} to steal it",
                self.minion()?
            ),
            P::MakeAnarch => format!(
                "You can burn 1 blood from {} to make {} Anarch",
                self.card()?,
                self.minion()?
            ),
            P::AshurPicks => match self.detail() {
                Some("to_library") => format!(
                    "Choose up to {} more ash heap cards to shuffle into your library",
                    a.n?
                ),
                _ => "Choose the card in your ash heap to put in your hand".into(),
            },
            P::BecomesAnarch => "Choose a non-titled vampire of yours to become Anarch".into(),
            P::LockOrPay => format!("Lock {} or burn {} pool", self.card()?, a.n?),
            P::LibraryToHand => "Choose a card in your library to put in your hand".into(),
            P::BloodOrCamarilla => format!("{} burns 1 blood or becomes Camarilla", self.minion()?),
            P::PoolOrNoTransfers => format!(
                "Burn {} pool, or lose all transfers in your next influence phase",
                a.n?
            ),
            P::PoolOrVampire => format!(
                "Burn {} pool, or burn a vampire of yours with capacity {} or more to burn {}",
                a.n?,
                a.cap?,
                self.card()?
            ),
            P::CryptSearch => format!(
                "Choose a {} in your crypt to move to your uncontrolled region",
                self.clan()?
            ),
            P::PreySlotToPool => format!(
                "Choose the uncontrolled vampire of {} whose blood goes to their pool",
                self.seat()?
            ),
            P::NameSlotPair => match self.detail() {
                Some(_) => "Choose the vampire in your uncontrolled region to get it".into(),
                None => format!(
                    "Choose the uncontrolled vampire of {} to move 1 blood from",
                    self.seat()?
                ),
            },
            P::CircleBlood => match (self.minion(), self.to(), self.card()) {
                (Some(f), Some(_), _) => format!("How much blood {f} moves"),
                (Some(f), None, _) => format!("Choose the Blood Brother {f} moves blood to"),
                (None, _, Some(c)) => format!("Choose the Blood Brother {c} moves to"),
                (None, _, None) => format!(
                    "Choose a Blood Brother of the {} Circle to move blood or equipment from \
                     (or stop)",
                    self.clan()?
                ),
            },
            P::ChooseCircle => format!("Choose the circle {} belongs to", self.minion()?),
            P::BaseSearch => format!(
                "Choose the base version of {} to merge with (crypt, uncontrolled region or \
                 ash heap)",
                self.minion()?
            ),
            P::PoolToVampires => match self.to() {
                Some(t) => format!("How much blood moves from your pool to {t}"),
                None => {
                    "Choose a vampire of yours to move blood to from your pool (or stop)".into()
                }
            },
            P::DistributeFrom => format!(
                "You can move 1 blood from {} to a {} in your uncontrolled region ({} more at \
                 most)",
                self.minion()?,
                self.clan()?,
                a.n?
            ),
            P::UncontrolledBlood => match self.clan() {
                Some(c) => format!(
                    "Add {} blood to a {c} vampire in your uncontrolled region",
                    a.n?
                ),
                None => format!(
                    "You can add {} blood to a vampire in your uncontrolled region",
                    a.n?
                ),
            },
            P::AllyFromAsh => {
                "Choose the ally to move from your ash heap to your ready region".into()
            }
            P::MayRecycleCanceled => format!(
                "You can move the canceled {} from your ash heap to your library",
                self.card()?
            ),
            P::BeastDisciplines => "Choose a Discipline for the Great Beast, at superior".into(),
            P::StorePreyAsh => {
                "Choose the library card of your prey's ash heap to move onto the Fragments".into()
            }
            P::NameOwnAshCard => "Name the library card in your ash heap to take back".into(),
            P::NameAshVampire => {
                "Name the vampire in your ash heap to move to your uncontrolled region".into()
            }
            P::StoreOwnAsh => format!(
                "Choose the library card of your ash heap to move onto {}",
                self.card()?
            ),
            P::StoredToLibraryTop => format!(
                "Choose the card on {} to put on top of your library",
                self.card()?
            ),
            P::NameUncontrolled => {
                "Choose the vampire in your uncontrolled region to remove from the game".into()
            }
            P::NameAshPair => {
                "Name the card two copies of which in your prey's ash heap to remove".into()
            }
            P::MoveForBlood => format!(
                "{} can burn {} blood to move {} to your ready region",
                self.minion()?,
                a.n?,
                self.to()?
            ),
            P::MoveCardToTitled => format!(
                "Move {} to another justicar or Inner Circle member",
                self.card()?
            ),
            P::SecretNumber => "Secretly choose a number".into(),
            P::SearchEquipFor => format!(
                "Search your library for an equipment card to put on {}",
                self.minion()?
            ),
            P::PutInPlaySearch => format!(
                "Search for a card of cost or capacity {} or less to put in play for {}",
                a.n?,
                self.minion()?
            ),
            P::MoveCardToOldest => format!(
                "Move {} to one of the oldest ready untitled vampires of its clan and sect",
                self.card()?
            ),
            P::MayPreventForBlood => format!(
                "{} can burn {} blood to prevent the opposing strike's damage",
                self.minion()?,
                a.n?
            ),
            P::ShortfallPicks => format!(
                "{} is short of blood: choose who receives the next one",
                self.minion()?
            ),
            P::MayUnlock => format!("{} can unlock", self.minion()?),
            P::GiveCardTo => format!("Choose who takes control of {}", self.card()?),
            P::MoveOffBurnedBearer => format!(
                "{} is about to be burned: put {} on another minion?",
                self.minion()?,
                self.card()?
            ),
            _ => return None,
        })
    }

    /// The prompt of a play's, a cost's, an ability's, a rider's or a
    /// referendum's terms: the card, then what is chosen.
    fn terms(&self) -> Option<String> {
        let a = self.a;
        let source = self.d.source.as_ref().map(|c| card(c, self.n));
        let what = match self.d.window {
            Window::PlayTerms => match self.detail()? {
                "second_unblockable_non_bleed" => format!(
                    "choose the {} whose non-bleed actions cannot be blocked",
                    self.clan()?
                ),
                "second_chosen" => format!("choose the card's chosen {}", self.clan()?),
                "blood_range" => "how much blood".into(),
                "up_to_target_blood" => format!("how much blood to take from {}", self.minion()?),
                "x_blood" => "X, paid in blood".into(),
                "up_to_margin" => "how much blood moves to your pool".into(),
                "boon_pool" => "how much pool to give".into(),
                "burn_named" => format!("which {} to burn", self.card()?),
                "ash_heap_ally" => "the ash heap to take an ally from".into(),
                "ally" => "the ally".into(),
                "equipment" => "the equipment".into(),
                "equipment_cost" => format!(
                    "how much of the cost {} pays in blood",
                    self.minion().unwrap_or_else(|| "the vampire".into())
                ),
                "put_in_play" => "the card to put in play".into(),
                "cost_reductions" => "the cost reductions to use".into(),
                "discard" => format!("discard {} card(s) to use it", a.n?),
                "count" => "how many cards".into(),
                "x_pool" => "how much pool to pay".into(),
                "x_counters" => "how many counters to use".into(),
                "life_from_target" => format!("how much blood to move from {}", self.minion()?),
                "ash_card" => "a card in your ash heap".into(),
                "hand_card" => "a card from the hand".into(),
                "hold_target" => match self.card() {
                    Some(c) => format!("the vampire who gets {c}"),
                    None => "the vampire who gets the card".into(),
                },
                _ => return None,
            },
            Window::RiderTerms => format!("{} still to pay", a.n?),
            Window::ReferendumTerms => {
                let chosen = self
                    .seat()
                    .unwrap_or_else(|| "the chosen Methuselah".into());
                match self.detail()? {
                    "gains_pool" => format!("choose the Methuselah who gains {} pool", a.n?),
                    "gives_pool" => "choose the Methuselah whose pool is given away".into(),
                    "takes_card" => "choose the Methuselah who takes control of the card".into(),
                    "card_changes_control" => "choose the card that changes control".into(),
                    "burn_chosen" => format!(
                        "choose the Methuselahs who burn {} pool ({} if they control a ready \
                         vampire of capacity {} or less)",
                        a.n?, a.m?, a.cap?
                    ),
                    "burn_allocated" => {
                        "allocate the points: each Methuselah burns 1 pool per point".into()
                    }
                    "move_allocated" => format!(
                        "allocate {chosen}'s pool: each Methuselah gains 1 pool per point, from \
                         {chosen}"
                    ),
                    "lock_burn_allocated" => "allocate the points: each Methuselah burns 1 pool \
                                              per point, each card given a point is burned"
                        .into(),
                    "banish" => "choose the vampire to banish".into(),
                    "expel" => format!(
                        "choose up to {} minions that cannot play reaction cards, block or cast \
                         votes this turn",
                        a.n?
                    ),
                    "put_on_chosen" => "choose the vampire who gets the card".into(),
                    "lock_chosen" => "choose the minions to lock".into(),
                    "gain_per_clan" => "choose a clan: each Methuselah gains 1 pool per vampire \
                                        of that clan they control"
                        .into(),
                    _ => return None,
                }
            }
            _ => return None,
        };
        Some(match source {
            Some(s) => format!("{s}: {what}"),
            None => capitalize(&what),
        })
    }
}

/// A card a prompt names: by its id, else as the table names its copy.
fn card(c: &CardRef, n: &Names) -> String {
    match (c.id, c.copy) {
        (Some(id), _) => n.card_id(Some(id)),
        (None, Some(copy)) => n.own(copy),
        (None, None) => "a card".into(),
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().chain(c).collect())
        .unwrap_or_default()
}
