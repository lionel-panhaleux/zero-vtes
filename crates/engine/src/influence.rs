//! Influence phase (rulebook "Influence Phase"): transfers, withdrawals,
//! the 4-transfer + 1-pool crypt draw, and influencing out (no voluntary
//! self-contest).

use crate::cards::Window;
use crate::crypt::CRYPT;
use crate::decision::{Choice, DecisionRequest, WindowLabel};
use crate::game::{Engine, Flow};
use crate::ids::{MinionId, SeatId};
use crate::notes::{Cause, Source};
use crate::state::UncontrolledSlot;

/// Transfers granted at the start of an influence phase: 1/2/3 on the first
/// three turns of the game, 4 thereafter (rulebook "Influence Phase"); in
/// the two-player variant, 3 on the first turn, 4 thereafter.
pub fn transfers_for_turn(turn: u32, format: crate::state::Format) -> u8 {
    match format {
        crate::state::Format::Standard => turn.min(4) as u8,
        crate::state::Format::TwoPlayer if turn == 1 => 3,
        crate::state::Format::TwoPlayer => 4,
    }
}

impl Engine {
    /// The active seat's transfers left, during its influence phase (a
    /// window opened in it too).
    pub fn transfers_left(&self) -> Option<u8> {
        match *self.flow.resumed() {
            Flow::Influence { left } => Some(left),
            _ => None,
        }
    }

    /// The transfers left at the influence phase's own decision only, not
    /// in a window opened in it (the observation's transfers feature,
    /// trained on that reading).
    pub fn influence_transfers_left(&self) -> Option<u8> {
        match self.flow {
            Flow::Influence { left } => Some(left),
            _ => None,
        }
    }

    /// The transfers an ability use gains ("lock to get +1 transfer").
    fn transfer_gain(&self, choice: &Choice) -> u8 {
        let Choice::UseAbility {
            source, ability, ..
        } = choice
        else {
            return 0;
        };
        source
            .mode(&self.state, *ability)
            .effects
            .iter()
            .map(|e| match e {
                crate::cards::Effect::GainTransfers(n) => *n,
                _ => 0,
            })
            .sum()
    }

    /// The transfers an ability use costs ("you can use 1 transfer to …").
    fn transfer_cost(&self, choice: &Choice) -> u8 {
        let Choice::UseAbility {
            source, ability, ..
        } = choice
        else {
            return 0;
        };
        source
            .mode(&self.state, *ability)
            .effects
            .iter()
            .map(|e| match e {
                crate::cards::Effect::SpendTransfers(n) => *n,
                _ => 0,
            })
            .sum()
    }

    pub(crate) fn influence_request(&self, left: u8) -> DecisionRequest {
        let st = &self.state;
        let s = st.active;
        let seat = &st.seats[s.index()];
        let mut options = vec![Choice::Pass];
        let moves = !st
            .controlled_layers(s)
            .any(|l| l.modifier == crate::cards::Modifier::NoTransferCounterMoves);
        for (i, slot) in seat.uncontrolled.iter().enumerate() {
            let i = i as u8;
            let (capacity, _) = st.slot_vampire(s, i as usize);
            // Any amount, over capacity too (rules-qa 28.3).
            if moves && left >= 1 && seat.pool > 0 {
                options.push(Choice::Transfer { slot: i });
            }
            if moves && left >= 2 && slot.counters > 0 {
                options.push(Choice::Withdraw { slot: i });
            }
            // No voluntary self-contest, but Anarch Convert: he leaves as
            // he enters, before any contest.
            if slot.counters >= capacity
                && (slot.card == crate::state::CREATED
                    || !st.controls_copy(s, slot.card)
                    || is_anarch_convert(slot.card))
            {
                options.push(Choice::InfluenceOut { slot: i });
            }
        }
        if left >= 4 && seat.pool > 0 && !seat.crypt.is_empty() {
            options.push(Choice::DrawCrypt);
        }
        if left >= 4 && seat.pool > 0 {
            options.extend(
                (0..seat.uncontrolled.len())
                    .filter(|&i| self.merge_partner(s, i).is_some())
                    .map(|i| Choice::Merge { slot: i as u8 }),
            );
        }
        options.extend(
            st.minions_of(s)
                .filter(|&m| st.may_follow_path(m))
                .flat_map(|minion| {
                    (0..crate::state::SABBAT_PATHS.len() as u8)
                        .map(move |path| Choice::FollowPath { minion, path })
                }),
        );
        options.extend(self.any_impulse_options(s));
        options.extend(self.ability_options(s, Window::InfluencePhaseOwn));
        // Abilities paid in transfers: while enough are left.
        options.extend(
            self.ability_options(s, Window::Transfers)
                .into_iter()
                .filter(|o| self.transfer_cost(o) <= left),
        );
        // Frontal Assault's burn: in the order the player chooses, before
        // they pass (rules-qa 28.9).
        let due = self.influence_burns_due();
        if !due.is_empty() {
            options.retain(|o| *o != Choice::Pass);
            options.extend(due.iter().map(|c| Choice::ResolveRider { card: c.0 }));
        }
        DecisionRequest::new(s, WindowLabel::InfluenceTransfers, options)
    }

    pub(crate) fn influence_apply(&mut self, left: u8, choice: &Choice) {
        let s = self.state.active;
        match choice {
            Choice::UseAbility {
                source,
                ability,
                actor,
                target,
            } => {
                let left = left - self.transfer_cost(choice) + self.transfer_gain(choice);
                if !self.use_ability(
                    s,
                    *source,
                    *ability,
                    *actor,
                    *target,
                    Flow::Influence { left },
                ) {
                    self.flow = Flow::Influence { left };
                }
            }
            Choice::Transfer { slot } => {
                let seat = &mut self.state.seats[s.index()];
                seat.lose_pool(1);
                seat.uncontrolled[*slot as usize].counters += 1;
                self.flow = Flow::Influence { left: left - 1 };
                if seat.pool == 0 {
                    self.oust_and_pass(s);
                }
            }
            Choice::Withdraw { slot } => {
                self.state.seats[s.index()].uncontrolled[*slot as usize].counters -= 1;
                self.state.gain_pool(s, 1);
                self.flow = Flow::Influence { left: left - 2 };
            }
            Choice::FollowPath { minion, path } => {
                let cost = Cause::Cost(Source::Minion(*minion));
                self.caused(cost, |e| e.state.lose_blood(*minion, 1));
                self.state.minions[minion.index()].path_override =
                    Some(crate::state::SABBAT_PATHS[*path as usize]);
                self.flow = Flow::Influence { left };
            }
            Choice::DrawCrypt => {
                let seat = &mut self.state.seats[s.index()];
                seat.lose_pool(1);
                let (card, copy) = seat.crypt.take_top().expect("draw from empty crypt");
                seat.uncontrolled.push(UncontrolledSlot {
                    card,
                    copy: crate::copies::Untracked(copy),
                    counters: 0,
                    banished: None,
                });
                self.flow = Flow::Influence { left: left - 4 };
                if seat.pool == 0 {
                    self.oust_and_pass(s);
                }
            }
            Choice::InfluenceOut { slot } => {
                let m = self.slot_to_ready(s, *slot as usize);
                self.flow = Flow::Influence { left };
                // Twilight Camp: a non-titled non-Anarch vampire put in play
                // from the uncontrolled region may be made Anarch.
                if self.state.title_of(m) == crate::crypt::Title::None
                    && self.state.sect_of(m) != Some(crate::crypt::Sect::Anarch)
                {
                    let hook = crate::cards::Hook::OwnVampireInfluenced;
                    let on = |c: &crate::state::CardInPlay| c.counters > 0;
                    let at = crate::plays::HookAt {
                        seat: Some(s),
                        ..crate::plays::HookAt::on(&on)
                    };
                    for (id, r) in self.fired_riders(hook, &at) {
                        for e in r.effects {
                            match *e {
                                crate::cards::Effect::MayMakeAnarchForCounter => {
                                    self.interrupts.push(crate::draws::Interrupt::MakeAnarch {
                                        seat: s,
                                        card: id,
                                        minion: m,
                                    })
                                }
                                other => unreachable!("not an influence rider effect: {other:?}"),
                            }
                        }
                    }
                }
                // Alvusia: a Tremere or Gargoyle entering in its
                // controller's influence phase, 1 counter.
                let hook = crate::cards::Hook::OwnVampireEntersInInfluence;
                let entering = self.state.minions[m.index()].clan();
                for (id, r) in self.fired_riders(hook, &crate::plays::HookAt::of(s)) {
                    for e in r.effects {
                        match *e {
                            crate::cards::Effect::AddCounterForEnteringClan(clans) => {
                                if entering.is_some_and(|c| clans.contains(&c)) {
                                    self.state.cards[id.index()].counters += 1;
                                }
                            }
                            other => unreachable!("not an influence rider effect: {other:?}"),
                        }
                    }
                }
                if self.scarce_toll(s, m) {
                    self.oust_and_pass(s);
                }
            }
            Choice::ResolveRider { card } => {
                self.flow = Flow::Influence { left };
                let id = crate::state::CardId(*card);
                self.caused(Cause::Effect(Source::Card(id)), |e| e.influence_burn(id));
            }
            // The counters on the card coming from the uncontrolled region
            // burn; not entering play (advanced rules 5.1.2).
            Choice::Merge { slot } => {
                let m = self
                    .merge_partner(s, *slot as usize)
                    .expect("merge partner");
                let (slot, on) = self.state.take_slot(s, *slot as usize);
                for c in on {
                    self.state.cards[c.index()].host = Some(m);
                }
                self.merge(m, slot.card, *slot.copy);
                self.flow = Flow::Influence { left: left - 4 };
                let merged = Cause::Cost(Source::Minion(m));
                self.caused(merged, |e| e.state.seats[s.index()].lose_pool(1));
                if self.state.seats[s.index()].pool == 0 {
                    self.oust_and_pass(s);
                }
            }
            Choice::Pass => self.open_phase_rotation(Flow::Influence { left }),
            other => unreachable!("illegal influence choice {other:?}"),
        }
    }
}

impl Engine {
    /// `s`'s uncontrolled vampire at `slot` to the ready region with its
    /// counters (the influence phase's move; Gather's): its entry effects
    /// and contest.
    pub(crate) fn slot_to_ready(&mut self, s: SeatId, slot: usize) -> crate::ids::MinionId {
        let (out, on) = self.state.take_slot(s, slot);
        let forced = out.card != crate::state::CREATED && self.state.controls_copy(s, out.card);
        // Excess over capacity drains, a returning vampire's
        // capacity counting its cards (advanced rules 5.1.1).
        let m = match out.banished.map(|b| (b.minion, b.cards)) {
            // A banished vampire returns as it left, its cards
            // with it, unlocked (rules-qa 5.7; LSJ 20060213).
            Some((m, on)) => {
                let v = &mut self.state.minions[m.index()];
                v.in_play = true;
                v.locked = false;
                v.in_torpor = false;
                v.no_unlock_in_torpor = crate::state::Sparse(0);
                for c in on {
                    let c = &mut self.state.cards[c.index()];
                    c.in_play = true;
                    c.locked = false;
                }
                let capacity = self.state.capacity_of(m).unwrap_or(1);
                self.state.minions[m.index()].raise_blood(out.counters.min(capacity));
                m
            }
            // Its counters (all of its capacity for an influence; Gather's
            // move takes what it has), the excess draining.
            None => {
                let capacity = CRYPT[out.card as usize].capacity;
                let m = self.add_minion(s, out.card, out.counters.min(capacity), false);
                self.state.minions[m.index()].copy = out.copy;
                m
            }
        };
        for c in on {
            self.state.cards[c.index()].host = Some(m);
        }
        self.vampire_entered(s, m, out.card, forced);
        m
    }

    /// Scarce: 3 pool for each vampire of its clan already controlled —
    /// the apparent clan (LSJ 20011015) — as it leaves the uncontrolled
    /// region; whether the pool ran out.
    pub(crate) fn scarce_toll(&mut self, s: SeatId, m: crate::ids::MinionId) -> bool {
        let card = self.state.minions[m.index()].crypt_card();
        if !card.is_some_and(|c| c != crate::state::CREATED && CRYPT[c as usize].scarce) {
            return false;
        }
        let clan = self.state.minions[m.index()].clan();
        let k = self
            .state
            .minions_of(s)
            .filter(|&o| {
                o != m
                    && self.state.minions[o.index()].is_vampire()
                    && self.state.minions[o.index()].clan() == clan
            })
            .count() as i32;
        if k == 0 {
            return false;
        }
        let scarce = Cause::Effect(Source::Minion(m));
        self.caused(scarce, |e| e.state.seats[s.index()].lose_pool(3 * k));
        self.state.seats[s.index()].pool <= 0
    }

    /// A crypt card just put in play as `s`'s vampire `m`: its entry
    /// effects, then any contest (influenced out, or put in play: Summon
    /// History); `forced`: another copy already controlled.
    pub(crate) fn vampire_entered(&mut self, s: SeatId, m: MinionId, card: u16, forced: bool) {
        self.entered_play_riders(m);
        // Jack Drake, whichever way it came out of the uncontrolled region.
        let turned = (0..self.state.minions.len())
            .map(|i| MinionId(i as u32))
            .any(|o| {
                let v = &self.state.minions[o.index()];
                v.owner == s
                    && v.present()
                    && self.state.younger_than(m, o)
                    && self.state.printed_mod(
                        o,
                        crate::cards::Stat::YoungerEntrantsMayTurnAnarch,
                        crate::state::PrintedCtx::default(),
                    ) > 0
            });
        if turned {
            self.interrupts
                .push(crate::draws::Interrupt::BurnBloodToAnarch { seat: s, minion: m });
        }
        // An Anarch entering play: counted first, the Convert's
        // ability next (Hackerspace, ANK 20210410).
        if self.state.sect_of(m) == Some(crate::crypt::Sect::Anarch) {
            self.anarch_arrives(s);
        }
        // Inez: a power requiring her virtue, from library or hand.
        if let Some(virtue) = entry_effects(card).find_map(|e| match *e {
            crate::cards::Effect::SearchPowerOnEntry(v) => Some(v),
            _ => None,
        }) {
            self.interrupts.push(crate::draws::Interrupt::PowerSearch {
                seat: s,
                minion: m,
                virtue,
            });
        }
        // An imbued entering play with no conviction may gain one (rulebook
        // "Appendix: Imbued Rules" 2), but Liz.
        if self.state.minions[m.index()].is_imbued()
            && self.state.convictions(m).is_empty()
            && !entry_has(card, crate::cards::Effect::NoEntryConviction)
        {
            self.interrupts
                .push(crate::draws::Interrupt::GainConviction {
                    seat: s,
                    minion: m,
                    library: true,
                });
        }
        // Grotesque: its location chosen as it enters, before any contest
        // (LSJ 20080526).
        if entry_has(card, crate::cards::Effect::BindLocationOnEntry) {
            self.interrupts
                .push(crate::draws::Interrupt::BindLocation { seat: s, minion: m });
        }
        // New Blood: "choose any circle and draw 1 card from your
        // crypt" as he enters.
        if entry_has(card, crate::cards::Effect::ChooseCircleOnEntry) {
            self.interrupts
                .push(crate::draws::Interrupt::ChooseCircle { seat: s, minion: m });
        }
        // Hermana Hambrienta: "burns 2 blood or is burned", after
        // the excess drained (LSJ 20051126).
        if let Some(n) = entry_burn(card) {
            self.interrupts
                .push(crate::draws::Interrupt::BloodOrBurned {
                    seat: s,
                    minion: m,
                    n,
                    card: None,
                });
        }
        if entry_has(card, crate::cards::Effect::DrawCryptOnEntry) {
            let st = &mut self.state.seats[s.index()];
            if let Some((card, copy)) = st.crypt.take_top() {
                st.uncontrolled.push(crate::state::UncontrolledSlot {
                    card,
                    copy: crate::copies::Untracked(copy),
                    counters: 0,
                    banished: None,
                });
            }
        }
        // Dr. Solomon Grey: before any contest (LSJ 20080526); the last
        // pool ousts, and nothing more of his entry follows.
        if let Some(n) = entry_pool(card) {
            let mut loss = vec![0; self.state.seats.len()];
            loss[s.index()] = n as i32;
            self.caused(Cause::Effect(Source::Minion(m)), |e| {
                e.burn_pool_batch(&loss)
            });
            if self.state.seats[s.index()].ousted {
                return;
            }
        }
        // Verbruch: "as he enters play, you can search your crypt", before
        // any contest (LSJ 20080526); the search reads nothing of him.
        if let Some(clan) = entry_crypt_search(card) {
            self.interrupts.push(crate::draws::Interrupt::CryptSearch {
                seat: s,
                clan: Some(clan),
                blood: 0,
                below: None,
                max_level: None,
            });
        }
        // Anarch Convert: his entry ability before any contest.
        if is_anarch_convert(card) {
            self.interrupts
                .push(crate::draws::Interrupt::AnarchConvert {
                    seat: s,
                    convert: m,
                    forced,
                    chosen: None,
                });
        } else if let Some(offers) = entry_title_offers(card) {
            // Victor Pelletier: the title bought before any contest (LSJ
            // 20080526, LSJ 20100902).
            self.interrupts.push(crate::draws::Interrupt::BuyTitle {
                seat: s,
                minion: m,
                offers,
            });
        } else if let Some((n, clan)) = entry_distribution(card) {
            // Erlik: before any contest (LSJ 20080526).
            self.interrupts
                .push(crate::draws::Interrupt::DistributeFrom {
                    seat: s,
                    from: m,
                    clan,
                    left: n,
                });
        } else {
            self.stage_contests(m);
        }
    }
}

impl Engine {
    /// The vampire `s` controls that uncontrolled slot `i` may merge with:
    /// the other version of its name, unmerged, in torpor too.
    pub(crate) fn merge_partner(
        &self,
        s: crate::ids::SeatId,
        i: usize,
    ) -> Option<crate::ids::MinionId> {
        let slot = &self.state.seats[s.index()].uncontrolled[i];
        if slot.card == crate::state::CREATED || slot.banished.is_some() {
            return None;
        }
        let c = &CRYPT[slot.card as usize];
        self.state.minions_of(s).find(|&m| {
            let v = &self.state.minions[m.index()];
            v.merged.is_none()
                && v.vampire()
                    .is_some_and(|o| o.name == c.name && o.adv != c.adv)
        })
    }

    /// Epiphany: `m`'s advanced version, from `s`'s crypt, uncontrolled
    /// region or ash heap in that order, merged with it; none, no effect.
    pub(crate) fn merge_with_advanced(
        &mut self,
        s: SeatId,
        m: crate::ids::MinionId,
        crypt_only: bool,
    ) {
        let name = CRYPT[self.state.minions[m.index()]
            .crypt_card()
            .expect("a vampire") as usize]
            .name;
        let advanced = |c: u16| CRYPT[c as usize].adv && CRYPT[c as usize].name == name;
        let mut blood = 0;
        let seat = &mut self.state.seats[s.index()];
        let found = if let Some(i) = seat.crypt.iter().position(|&c| advanced(c)) {
            Some(seat.crypt.take(i))
        } else if crypt_only {
            None
        } else if let Some(i) = seat
            .uncontrolled
            .iter()
            .position(|u| advanced(u.card) && u.banished.is_none())
        {
            let (slot, on) = self.state.take_slot(s, i);
            blood = slot.counters;
            for c in on {
                self.state.cards[c.index()].host = Some(m);
            }
            Some((slot.card, *slot.copy))
        } else {
            seat.ash_crypt
                .iter()
                .position(|&c| advanced(c))
                .map(|i| seat.ash_crypt.take(i))
        };
        self.state.seats[s.index()].crypt.shuffle(&mut self.rng);
        if let Some((card, copy)) = found {
            self.merge(m, card, copy);
            let merge = Cause::Effect(Source::Minion(m));
            self.caused(merge, |e| {
                let b = e.state.minions[m.index()].blood + blood;
                e.state.set_blood(m, b);
            });
        }
    }

    /// Merge `m` with crypt card `card` (rulebook "Advanced"): the advanced
    /// card on top, its sect taking over any other (advanced rules 5.1.2).
    pub(crate) fn merge(
        &mut self,
        m: crate::ids::MinionId,
        card: u16,
        copy: crate::copies::CopyId,
    ) {
        let v = &mut self.state.minions[m.index()];
        let own = (v.crypt_card().expect("a crypt vampire"), *v.copy);
        let ((adv, adv_copy), (base, base_copy)) = if CRYPT[card as usize].adv {
            ((card, copy), own)
        } else {
            (own, (card, copy))
        };
        v.kind = crate::state::MinionKind::Vampire(adv);
        v.copy = crate::copies::Untracked(adv_copy);
        v.merged = Some(base);
        v.merged_copy = crate::copies::Untracked(base_copy);
        v.sect_override = None;
    }
}

/// Anarch Convert's crypt card ("as the convert enters play, you can remove
/// him from the game …").
fn is_anarch_convert(card: u16) -> bool {
    card != crate::state::CREATED && CRYPT[card as usize].name == "Anarch Convert"
}

/// The effects of a crypt card's entry riders (none for a created
/// vampire).
fn entry_effects(card: u16) -> impl Iterator<Item = &'static crate::cards::Effect> {
    (card != crate::state::CREATED)
        .then(|| crate::cards::vampire(card))
        .flatten()
        .into_iter()
        .flat_map(|d| d.riders)
        .filter(|r| r.hook == crate::cards::Hook::EntersPlay)
        .flat_map(|r| r.effects)
}

/// A vampire's "as he enters play, you can distribute up to n blood from
/// him to your uncontrolled <clan>" (Erlik): its entry rider.
fn entry_distribution(card: u16) -> Option<(u8, &'static str)> {
    entry_effects(card).find_map(|e| match *e {
        crate::cards::Effect::DistributeToUncontrolled { n, clan } => Some((n, clan)),
        _ => None,
    })
}

/// A vampire's "when moved to the ready region in your influence phase,
/// this vampire burns n blood or is burned" (Hermana Hambrienta).
fn entry_burn(card: u16) -> Option<u8> {
    entry_effects(card).find_map(|e| match *e {
        crate::cards::Effect::BurnsBloodOrIsBurnedOnEntry(n) => Some(n),
        _ => None,
    })
}

/// A vampire's "burn n pool as he enters play" (Dr. Solomon Grey).
fn entry_pool(card: u16) -> Option<u8> {
    entry_effects(card).find_map(|e| match *e {
        crate::cards::Effect::BurnPool(n) => Some(n),
        _ => None,
    })
}

/// A vampire's "when he enters play, you may pay n pool to make him
/// <title>" (Victor Pelletier): its entry rider's offers.
fn entry_title_offers(card: u16) -> Option<&'static [(u8, crate::crypt::Title)]> {
    entry_effects(card).find_map(|e| match *e {
        crate::cards::Effect::BuyTitleOnEntry(offers) => Some(offers),
        _ => None,
    })
}

/// A vampire's "as he enters play, you can search your crypt for a <clan>
/// and move it to your uncontrolled region" (Verbruch): its entry rider.
fn entry_crypt_search(card: u16) -> Option<&'static str> {
    entry_effects(card).find_map(|e| match *e {
        crate::cards::Effect::CryptSearchOnEntry(clan) => Some(clan),
        _ => None,
    })
}

/// Does `effect` figure among the vampire's entry riders (New Blood)?
fn entry_has(card: u16, effect: crate::cards::Effect) -> bool {
    entry_effects(card).any(|e| *e == effect)
}
