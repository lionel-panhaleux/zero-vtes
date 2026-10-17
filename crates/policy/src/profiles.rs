//! The bot profiles a host offers by name (`zvtes_host::Bots`): a network
//! profile is the served weights at a temperature with its nudges. The
//! server and the tools that play as it does (`examples/humanset.rs`) read
//! them here.

use std::sync::Arc;

use zvtes::agent::Agent;
use zvtes_api::record::BotRecord;

use crate::{Net, NetAgent, Nudges};

/// A network bot as a host plays it.
#[derive(Clone, Copy, Debug)]
pub struct Profile {
    pub name: &'static str,
    /// `NetAgent::temperature`.
    pub temperature: f32,
    pub nudges: Nudges,
}

/// The served network profile. Temperature 0.5, measured on
/// `p7-politics@1400` (`examples/strength.rs`, 512 games): 1.16 VP a seat
/// against 0.82 sampling as training does (0.3: 1.18 / 0.80; 0.7: 1.06 /
/// 0.92) — the network's favourites more often, a card it rates low played
/// less "for the sake of it". The nudges the served weights trained with,
/// never faded (`nudge_until` 0): off them it plays off its training (a
/// transfer onto a second copy of a unique: 35% of the offers instead of
/// 4%; Z-train, 2026-10-12). The rescue bonus (owner, 2026-10-05: rescuing
/// one's own vampire encouraged, not forced): +6 takes 7% of the rescues
/// offered at temperature 1 (0% without), at no measured cost (512 games:
/// 0.94 / 1.05 VP a seat, within the control's spread). Nightmare 6: p10's.
pub const NETWORK: Profile = Profile {
    name: "network",
    temperature: 0.5,
    nudges: Nudges {
        rescue: 6.0,
        pass: 0.0,
        reaction: 0.0,
        bleed: 4.0,
        idle: 3.0,
        draw: 7.0,
        stuck: 6.0,
        nightmare: 6.0,
    },
};

/// The network profiles offered.
pub const PROFILES: [Profile; 1] = [NETWORK];

impl Profile {
    /// Its agent over `net`, sampling from `seed`.
    pub fn agent(&self, net: Arc<Net>, seed: u64) -> NetAgent {
        NetAgent::new(net, seed)
            .temperature(self.temperature)
            .nudges(self.nudges)
    }

    /// How a record names it: the weights' hash, its temperature and its
    /// nudges not zero.
    pub fn record(&self, weights: &str, seed: u32) -> BotRecord {
        BotRecord {
            seat: 0,
            profile: self.name.into(),
            kind: "network".into(),
            weights: Some(weights.into()),
            temperature: Some(self.temperature),
            seed,
            nudges: self
                .nudges
                .named()
                .into_iter()
                .filter(|&(_, v)| v != 0.0)
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
        }
    }
}

/// The served weights, shared by every game.
pub struct Served {
    pub net: Arc<Net>,
    /// The weights' SHA-256.
    pub hash: String,
}

/// The served weights, or none: then no network profile is offered, the
/// scripted ones only (`zvtes_host::BUILT_IN`).
pub struct Profiles(pub Option<Arc<Served>>);

impl zvtes_host::Bots for Profiles {
    fn profiles(&self) -> Vec<String> {
        match &self.0 {
            Some(_) => PROFILES.iter().map(|p| p.name.to_string()).collect(),
            None => vec![],
        }
    }

    fn make(&self, profile: &str, seed: u32) -> Option<(Box<dyn Agent>, BotRecord)> {
        let served = self.0.as_ref()?;
        let p = PROFILES.iter().find(|p| p.name == profile)?;
        Some((
            Box::new(p.agent(served.net.clone(), seed as u64)),
            p.record(&served.hash, seed),
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use zvtes_host::Bots;

    use super::{Profiles, Served};

    // No weights served: no network profile offered or made. Served: the
    // network profile offered and made, its record naming the weights,
    // its temperature, its nudges not zero; another name none.
    #[test]
    fn a_profile_needs_weights_and_is_recorded_whole() {
        let none = Profiles(None);
        assert!(none.profiles().is_empty());
        assert!(none.make("network", 1).is_none());
        let served = Profiles(Some(Arc::new(Served {
            net: Arc::new(crate::tests::uniform_net()),
            hash: "abc".into(),
        })));
        assert_eq!(served.profiles(), ["network"]);
        assert!(served.make("wall", 7).is_none());
        let (_, r) = served.make("network", 7).expect("the network made");
        assert_eq!(
            (r.profile.as_str(), r.kind.as_str()),
            ("network", "network")
        );
        assert_eq!(
            (r.weights.as_deref(), r.temperature, r.seed),
            (Some("abc"), Some(0.5), 7)
        );
        let named: Vec<&str> = r.nudges.keys().map(String::as_str).collect();
        assert_eq!(
            named,
            ["bleed", "draw", "idle", "nightmare", "rescue", "stuck"]
        );
    }
}
