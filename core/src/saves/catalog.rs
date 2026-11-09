//! The game's item names, ranks and limits, from `core/assets/save-catalog.json`.
//! `tools/make-save-catalog.py` writes that file from a dump taken from the running game.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

const JSON: &str = include_str!("../../assets/save-catalog.json");

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Limits {
    pub points_per_level: u32,
    pub samples_per_point: u32,
    pub max_samples: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Rank {
    pub level: u32,
    pub required_xp: u64,
    pub title: String,
    pub unlocks: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Upgrade {
    pub name: String,
    pub display: String,
    /// A bit for a weapon, a level for a stratagem.
    pub value: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Item {
    pub name: String,
    pub display: String,
    pub description: String,
    pub dlc: bool,
    pub upgrade_kind: String,
    pub max_upgrade: u32,
    pub upgrades: Vec<Upgrade>,
    #[serde(default)]
    pub category: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Cosmetic {
    pub name: String,
    pub display: String,
    pub dlc: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Customization {
    pub helmet: Vec<Cosmetic>,
    pub body: Vec<Cosmetic>,
    pub cape: Vec<Cosmetic>,
    pub gender: Vec<Cosmetic>,
}

impl Customization {
    pub fn slot(&self, slot: &str) -> Option<&Vec<Cosmetic>> {
        match slot {
            "helmet" => Some(&self.helmet),
            "body" => Some(&self.body),
            "cape" => Some(&self.cape),
            "gender" => Some(&self.gender),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DefaultLoadout {
    pub sidearm_weapon: String,
    pub primary_weapon: String,
    pub perk: String,
    pub stratagems: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Catalog {
    pub version: u32,
    pub limits: Limits,
    pub boss_influence_required: u32,
    pub default_loadout: DefaultLoadout,
    pub static_stratagems: Vec<String>,
    pub ranks: Vec<Rank>,
    pub primary_weapons: Vec<Item>,
    pub sidearm_weapons: Vec<Item>,
    pub stratagems: Vec<Item>,
    pub perks: Vec<Item>,
    pub customization: Customization,
    pub hidden: Vec<String>,
}

pub fn get() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| serde_json::from_str(JSON).expect("the save catalog is built with the app"))
}

impl Catalog {
    /// The highest rank `xp` reaches, from 1.
    pub fn rank_of(&self, xp: f64) -> u32 {
        self.ranks.iter().filter(|r| (r.required_xp as f64) <= xp).map(|r| r.level).max().unwrap_or(1)
    }

    /// Every item that goes in `item_unlocks` or has an upgrade: weapons, stratagems, perks.
    pub fn items(&self) -> impl Iterator<Item = &Item> {
        self.primary_weapons.iter().chain(&self.sidearm_weapons).chain(&self.stratagems).chain(&self.perks)
    }

    pub fn item(&self, name: &str) -> Option<&Item> {
        self.items().find(|i| i.name == name)
    }

    pub fn cosmetics(&self) -> impl Iterator<Item = &Cosmetic> {
        let c = &self.customization;
        c.helmet.iter().chain(&c.body).chain(&c.cape)
    }

    /// Whether an item name is paid DLC.
    pub fn is_dlc(&self, name: &str) -> bool {
        self.item(name).map(|i| i.dlc).or_else(|| self.cosmetics().find(|c| c.name == name).map(|c| c.dlc)).unwrap_or(false)
    }

    pub fn is_static(&self, name: &str) -> bool {
        self.static_stratagems.iter().any(|s| s == name)
    }

    /// The rank that grants an item, when one does.
    pub fn rank_granting(&self, name: &str) -> Option<u32> {
        self.ranks.iter().filter(|r| r.unlocks.iter().any(|u| u == name)).map(|r| r.level).min()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_catalog_is_read_and_has_what_the_editor_needs() {
        let c = get();
        assert_eq!(c.ranks.len(), 50);
        assert_eq!(c.ranks[0].required_xp, 0);
        assert_eq!(c.limits.samples_per_point, 10);
        assert!(c.is_dlc("weapon_plasmarifle"));
        assert!(!c.is_dlc("weapon_assaultrifle"));
        assert!(c.is_static("stratagem_reinforce"));
        assert_eq!(c.rank_granting("weapon_assaultrifle"), Some(1));
        assert_eq!(c.item("weapon_assaultrifle").unwrap().max_upgrade, 7);
        assert!(!c.item("weapon_assaultrifle").unwrap().upgrades.is_empty());
        assert!(c.item("weapon_saber").is_none(), "a hidden item stays out of the owned lists");
        assert!(c.hidden.iter().any(|h| h == "weapon_saber"));
        assert_eq!(c.rank_of(0.0), 1);
        assert_eq!(c.rank_of(7_094_381.0), 50);
        assert_eq!(c.rank_of(17_556.0), 9);
    }
}
