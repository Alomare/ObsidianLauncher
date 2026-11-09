//! The editor over a Sandbox save: a typed state to show, and edits that change the save. Paid DLC the
//! account does not own is never written, names the catalog doesn't know stay in the save untouched,
//! and `new_unlocks` is left alone.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

use super::catalog::{self, Catalog, Item};
use super::codec::{Table, Value};
use super::mod_data::{self, ModData, ModValue};
use super::{Pair, Saves};
use crate::library::{Library, Profile};
use crate::manifest::{SaveEditorSpec, SaveFieldSpec, load_manifest};
use crate::{ObsidianError, Result};

const UNLOCKS: &str = "item_unlocks";
const UPGRADES: &str = "item_upgrades_02";
const COSMETICS: &str = "customization_02";
const BOSS: &str = "boss_influence";
const LIFETIME: &str = "lifetime_stats";
const LIFETIME_KEYS: [&str; 11] = ["kills", "deaths", "team_kills", "shots", "time_played", "missions_fought", "missions_won", "planets_completed", "seasons_fought", "seasons_won", "last_season_fought"];
const FLAGS: [&str; 8] = [
    "tutorial_finished",
    "galcamp_tutorial_completed_attack_event",
    "galcamp_tutorial_completed_defend_event",
    "galcamp_tutorial_completed_defend_superearth_event",
    "galcamp_tutorial_completed_mission",
    "galcamp_tutorial_completed_planet",
    "galcamp_tutorial_completed_recap",
    "galcamp_tutorial_completed_region",
];
/// Where a manual unlock of a mod's item is kept: one comma list under Obsidian API, the library
/// that reads the list in the game.
const API_MOD: &str = "obsidian.api";
const API_UNLOCKS: &str = "stratagem_unlocks";

fn bad(field: &str, value: impl std::fmt::Display) -> ObsidianError {
    ObsidianError::BadSaveEdit { field: field.to_string(), value: value.to_string() }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemState {
    pub name: String,
    pub display: String,
    pub description: String,
    pub category: Option<String>,
    /// Paid DLC.
    pub dlc: bool,
    /// Paid DLC the account does not own: shown locked, never written.
    pub locked: bool,
    /// Owned by any route: rank, unlock, entitlement or fixed.
    pub owned: bool,
    /// In `item_unlocks`, the part this editor changes.
    pub in_unlocks: bool,
    pub granted_rank: Option<u32>,
    /// `bitmask` or `level`.
    pub upgrade_kind: String,
    pub upgrade: u32,
    pub max_upgrade: u32,
    pub upgrades: Vec<catalog::Upgrade>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CosmeticState {
    pub name: String,
    pub display: String,
    pub dlc: bool,
    pub locked: bool,
    pub owned: bool,
    pub in_unlocks: bool,
    pub granted_rank: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RankRow {
    pub level: u32,
    pub required_xp: u64,
    pub title: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlagState {
    pub key: String,
    pub value: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModFieldState {
    pub id: String,
    pub kind: String,
    pub label: Option<Json>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub choices: Option<Vec<Json>>,
    pub when: Option<Json>,
    pub value: Option<Json>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModItemState {
    pub kind: String,
    pub name: String,
    pub label: Option<Json>,
    pub unlock: Option<Json>,
    /// For a `manual` item: whether the save has opened it.
    pub owned: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModDataRow {
    pub key: String,
    pub value: Json,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModSection {
    pub id: String,
    pub name: Option<Json>,
    /// `enabled`, `disabled` or `removed`.
    pub state: String,
    pub items: Vec<ModItemState>,
    pub fields: Vec<ModFieldState>,
    /// What the file holds for this mod; for a removed mod, all there is to show.
    pub data: Vec<ModDataRow>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorState {
    pub id: String,
    pub name: String,
    pub xp: f64,
    pub rank: u32,
    pub samples: f64,
    pub research_points: u32,
    pub max_samples: u32,
    pub samples_per_point: u32,
    pub ranks: Vec<RankRow>,
    pub customization: BTreeMap<String, u32>,
    pub cosmetics: BTreeMap<String, Vec<CosmeticState>>,
    pub primary_weapons: Vec<ItemState>,
    pub sidearm_weapons: Vec<ItemState>,
    pub stratagems: Vec<ItemState>,
    pub perks: Vec<ItemState>,
    /// `item_unlocks` names the catalog doesn't know. They stay in the save.
    pub unknown_unlocks: Vec<String>,
    pub boss_influence: Vec<f64>,
    pub boss_influence_required: u32,
    pub flags: Vec<FlagState>,
    pub tutorial_stage: Option<f64>,
    pub lifetime: BTreeMap<String, f64>,
    pub mods: Vec<ModSection>,
}

/// One change to a Sandbox save.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Edit {
    SetXp { xp: f64 },
    SetRank { rank: u32 },
    SetSamples { samples: f64 },
    SetOwned { item: String, owned: bool },
    SetUpgrade { item: String, value: u32 },
    SetCustomization { slot: String, index: u32 },
    SetBossInfluence { index: usize, value: f64 },
    SetFlag { key: String, value: bool },
    SetTutorialStage { value: f64 },
    SetLifetime { key: String, value: f64 },
    MaxEverything,
    ResetProgression,
    ModSet { mod_id: String, key: String, value: ModValue },
    ModDelete { mod_id: String, key: String },
    ModClear { mod_id: String },
    ModItemOwned { mod_id: String, name: String, owned: bool },
}

fn owned_set(p: &Pair) -> BTreeSet<String> {
    p.user.root.table(UNLOCKS).map(|t| t.strings().into_iter().collect()).unwrap_or_default()
}

fn item_state(cat: &Catalog, item: &Item, rank: u32, unlocks: &BTreeSet<String>, ents: &BTreeSet<String>, upgrades: Option<&Table>) -> ItemState {
    let granted_rank = cat.rank_granting(&item.name);
    let in_unlocks = unlocks.contains(&item.name);
    let entitled = item.dlc && !cat.is_locked(&item.name, ents);
    let owned = in_unlocks || entitled || cat.is_static(&item.name) || granted_rank.is_some_and(|r| r <= rank);
    let upgrade = upgrades.and_then(|t| t.number(&item.name)).map_or(0, |n| n as u32);
    ItemState {
        name: item.name.clone(),
        display: item.display.clone(),
        description: item.description.clone(),
        category: item.category.clone(),
        dlc: item.dlc,
        locked: item.dlc && !entitled,
        owned,
        in_unlocks,
        granted_rank,
        upgrade_kind: item.upgrade_kind.clone(),
        upgrade,
        max_upgrade: item.max_upgrade,
        upgrades: item.upgrades.clone(),
    }
}

impl Saves {
    /// The save, as the editor shows it. Sandbox only.
    pub fn editor_state(&self, id: &str) -> Result<EditorState> {
        self.require_sandbox(id, "edit")?;
        let pair = self.read_sandbox(id)?;
        let data = mod_data::read(&self.sandbox_mod(id))?;
        let name = self.save_name(id)?;
        self.state_of(id, name, &pair, &data)
    }

    fn save_name(&self, id: &str) -> Result<String> {
        Ok(self.read_entry_name(id)?.unwrap_or_default())
    }

    fn state_of(&self, id: &str, name: String, pair: &Pair, data: &ModData) -> Result<EditorState> {
        let cat = catalog::get();
        let user = &pair.user.root;
        let common = &pair.common.root;
        let xp = user.number("player_xp").unwrap_or(0.0);
        let rank = cat.rank_of(xp);
        let unlocks = owned_set(pair);
        let ents = self.owned_entitlements()?;
        let upgrades = user.table(UPGRADES);
        let samples = user.number("research_samples").unwrap_or(0.0);

        let items = |list: &Vec<Item>| list.iter().map(|i| item_state(cat, i, rank, &unlocks, &ents, upgrades)).collect::<Vec<_>>();

        let mut cosmetics = BTreeMap::new();
        for slot in ["helmet", "body", "cape", "gender"] {
            let list = cat.customization.slot(slot).expect("known slot");
            let rows = list
                .iter()
                .map(|c| {
                    let granted_rank = cat.rank_granting(&c.name);
                    let in_unlocks = unlocks.contains(&c.name);
                    let locked = cat.is_locked(&c.name, &ents);
                    let owned = in_unlocks || (c.dlc && !locked) || granted_rank.is_some_and(|r| r <= rank);
                    CosmeticState { name: c.name.clone(), display: c.display.clone(), dlc: c.dlc, locked, owned, in_unlocks, granted_rank }
                })
                .collect();
            cosmetics.insert(slot.to_string(), rows);
        }

        let customization = user
            .table(COSMETICS)
            .map(|t| t.0.iter().filter_map(|(k, v)| Some((k.as_str()?, v.as_number()? as u32))).collect())
            .unwrap_or_default();

        let known: BTreeSet<&str> = cat.items().map(|i| i.name.as_str()).chain(cat.cosmetics().map(|c| c.name.as_str())).collect();
        let unknown_unlocks = user.table(UNLOCKS).map(|t| t.strings().into_iter().filter(|n| !known.contains(n.as_str())).collect()).unwrap_or_default();

        let boss = user.table(BOSS);
        let boss_influence = (1..=3).map(|i| boss.and_then(|t| t.at(i)).and_then(Value::as_number).unwrap_or(0.0)).collect();

        let flags = FLAGS.iter().map(|k| FlagState { key: k.to_string(), value: common.get(k).and_then(Value::as_bool).unwrap_or(false) }).collect();

        let life = user.table(LIFETIME);
        let lifetime = LIFETIME_KEYS.iter().map(|k| (k.to_string(), life.and_then(|t| t.number(k)).unwrap_or(0.0))).collect();

        let mods = self.mod_sections(data)?;
        Ok(EditorState {
            id: id.to_string(),
            name,
            xp,
            rank,
            samples,
            research_points: (samples / cat.limits.samples_per_point as f64).floor() as u32,
            max_samples: cat.limits.max_samples,
            samples_per_point: cat.limits.samples_per_point,
            ranks: cat.ranks.iter().map(|r| RankRow { level: r.level, required_xp: r.required_xp, title: r.title.clone() }).collect(),
            customization,
            cosmetics,
            primary_weapons: items(&cat.primary_weapons),
            sidearm_weapons: items(&cat.sidearm_weapons),
            stratagems: items(&cat.stratagems),
            perks: items(&cat.perks),
            unknown_unlocks,
            boss_influence,
            boss_influence_required: cat.boss_influence_required,
            flags,
            tutorial_stage: common.number("tutorial_stage"),
            lifetime,
            mods,
        })
    }

    fn mod_sections(&self, data: &ModData) -> Result<Vec<ModSection>> {
        let lib = Library::open(&self.data)?;
        let profile = Profile::load(&self.data)?;
        let manifests = lib.list()?;
        let unlocks: BTreeSet<String> = api_unlock_names(data);
        let mut out = Vec::new();
        let mut known = BTreeSet::new();
        for m in &manifests {
            known.insert(m.id.clone());
            let Some(spec) = m.save_editor()? else { continue };
            let state = if profile.mods.iter().any(|e| e.id == m.id && e.enabled) { "enabled" } else { "disabled" };
            let values = data.0.get(&m.id);
            out.push(ModSection {
                id: m.id.clone(),
                name: serde_json::to_value(&m.name).ok(),
                state: state.to_string(),
                items: spec
                    .items
                    .iter()
                    .map(|i| {
                        let owned = i.unlock.as_ref().and_then(Json::as_str).filter(|u| *u == "manual").map(|_| unlocks.contains(&i.name));
                        ModItemState { kind: i.kind.clone(), name: i.name.clone(), label: i.label.clone(), unlock: i.unlock.clone(), owned }
                    })
                    .collect(),
                fields: spec.fields.iter().filter(|f| f.kind != "tree").map(|f| field_state(f, values)).collect(),
                data: data_rows(values),
            });
        }
        for (id, values) in &data.0 {
            if !known.contains(id) {
                out.push(ModSection { id: id.clone(), name: None, state: "removed".to_string(), items: Vec::new(), fields: Vec::new(), data: data_rows(Some(values)) });
            }
        }
        Ok(out)
    }

    /// Applies `edits` in order and writes the save. Sandbox only, game closed.
    pub fn apply_edits(&self, id: &str, edits: &[Edit]) -> Result<EditorState> {
        self.closed()?;
        self.require_sandbox(id, "edit")?;
        let mut pair = self.read_sandbox(id)?;
        let mut data = mod_data::read(&self.sandbox_mod(id))?;
        let lib = Library::open(&self.data)?;
        let ents = self.owned_entitlements()?;
        for e in edits {
            self.apply_one(&mut pair, &mut data, &lib, &ents, e)?;
        }
        self.write_sandbox(id, &pair)?;
        mod_data::write(&self.sandbox_mod(id), &data)?;
        let name = self.save_name(id)?;
        self.state_of(id, name, &pair, &data)
    }

    fn apply_one(&self, pair: &mut Pair, data: &mut ModData, lib: &Library, ents: &BTreeSet<String>, edit: &Edit) -> Result<()> {
        let cat = catalog::get();
        let user = &mut pair.user.root;
        let common = &mut pair.common.root;
        match edit {
            Edit::SetXp { xp } => {
                let top = cat.ranks.last().map_or(0, |r| r.required_xp) as f64;
                if !xp.is_finite() || *xp < 0.0 || *xp > top {
                    return Err(bad("xp", xp));
                }
                user.set("player_xp", Value::Number(*xp));
            }
            Edit::SetRank { rank } => {
                let r = cat.ranks.iter().find(|r| r.level == *rank).ok_or_else(|| bad("rank", rank))?;
                user.set("player_xp", Value::Number(r.required_xp as f64));
            }
            Edit::SetSamples { samples } => {
                if !samples.is_finite() || *samples < 0.0 || *samples > cat.limits.max_samples as f64 {
                    return Err(bad("samples", samples));
                }
                user.set("research_samples", Value::Number(*samples));
            }
            Edit::SetOwned { item, owned } => set_owned(cat, user, ents, item, *owned)?,
            Edit::SetUpgrade { item, value } => set_upgrade(cat, user, ents, item, *value)?,
            Edit::SetCustomization { slot, index } => {
                let list = cat.customization.slot(slot).ok_or_else(|| bad("customization slot", slot))?;
                let chosen = list.get((*index as usize).wrapping_sub(1)).ok_or_else(|| bad("customization index", index))?;
                if cat.is_locked(&chosen.name, ents) {
                    return Err(ObsidianError::PaidDlc(chosen.name.clone()));
                }
                user.table_mut(COSMETICS).set(slot, Value::Number(*index as f64));
            }
            Edit::SetBossInfluence { index, value } => {
                if !(1..=3).contains(index) {
                    return Err(bad("boss influence index", index));
                }
                if !value.is_finite() || *value < 0.0 || *value > cat.boss_influence_required as f64 {
                    return Err(bad("boss influence", value));
                }
                user.table_mut(BOSS).set_at(*index, Value::Number(*value));
            }
            Edit::SetFlag { key, value } => {
                if !FLAGS.contains(&key.as_str()) {
                    return Err(bad("flag", key));
                }
                common.set(key, Value::Bool(*value as u32));
            }
            Edit::SetTutorialStage { value } => {
                if !value.is_finite() || *value < 0.0 {
                    return Err(bad("tutorial stage", value));
                }
                common.set("tutorial_stage", Value::Number(*value));
            }
            Edit::SetLifetime { key, value } => {
                if !LIFETIME_KEYS.contains(&key.as_str()) || !value.is_finite() || *value < 0.0 {
                    return Err(bad("lifetime stat", key));
                }
                user.table_mut(LIFETIME).set(key, Value::Number(*value));
            }
            Edit::MaxEverything => max_everything(cat, user, ents),
            Edit::ResetProgression => reset_progression(cat, user, ents),
            Edit::ModSet { mod_id, key, value } => {
                let m = load_manifest(&lib.mod_dir(mod_id)).map_err(|_| ObsidianError::ModMissing(mod_id.clone()))?;
                let spec = m.save_editor()?.ok_or_else(|| bad("mod data", mod_id))?;
                let f = spec.fields.iter().find(|f| f.id == *key).ok_or_else(|| bad("mod field", key))?;
                check_field(f, value)?;
                data.0.entry(mod_id.clone()).or_default().insert(key.clone(), value.clone());
            }
            Edit::ModDelete { mod_id, key } => {
                if let Some(values) = data.0.get_mut(mod_id) {
                    values.remove(key);
                }
                data.0.retain(|_, values| !values.is_empty());
            }
            Edit::ModClear { mod_id } => {
                data.0.remove(mod_id);
            }
            Edit::ModItemOwned { mod_id, name, owned } => {
                let m = load_manifest(&lib.mod_dir(mod_id)).map_err(|_| ObsidianError::ModMissing(mod_id.clone()))?;
                let spec: SaveEditorSpec = m.save_editor()?.ok_or_else(|| bad("mod data", mod_id))?;
                let item = spec.items.iter().find(|i| i.name == *name).ok_or_else(|| bad("mod item", name))?;
                if item.unlock.as_ref().and_then(Json::as_str) != Some("manual") {
                    return Err(bad("mod item unlock", name));
                }
                let mut names = api_unlock_names(data);
                if *owned {
                    names.insert(name.clone());
                } else {
                    names.remove(name);
                }
                let values = data.0.entry(API_MOD.to_string()).or_default();
                if names.is_empty() {
                    values.remove(API_UNLOCKS);
                } else {
                    values.insert(API_UNLOCKS.to_string(), ModValue::Text(names.iter().cloned().collect::<Vec<_>>().join(",")));
                }
                data.0.retain(|_, values| !values.is_empty());
            }
        }
        Ok(())
    }
}

fn set_owned(cat: &Catalog, user: &mut Table, ents: &BTreeSet<String>, item: &str, owned: bool) -> Result<()> {
    if cat.is_locked(item, ents) {
        return Err(ObsidianError::PaidDlc(item.to_string()));
    }
    let known = cat.item(item).is_some() || cat.cosmetics().any(|c| c.name == item);
    if owned && !known {
        return Err(bad("item", item));
    }
    let list = user.table_mut(UNLOCKS);
    let mut names = list.strings();
    let has = names.iter().any(|n| n == item);
    if owned && !has {
        names.push(item.to_string());
        list.set_strings(&names);
    } else if !owned && has {
        names.retain(|n| n != item);
        list.set_strings(&names);
    }
    Ok(())
}

fn set_upgrade(cat: &Catalog, user: &mut Table, ents: &BTreeSet<String>, item: &str, value: u32) -> Result<()> {
    let found = cat.item(item).ok_or_else(|| bad("item", item))?;
    if cat.is_locked(item, ents) {
        return Err(ObsidianError::PaidDlc(item.to_string()));
    }
    let fine = if found.upgrade_kind == "bitmask" { value & !found.max_upgrade == 0 } else { value <= found.max_upgrade };
    if !fine {
        return Err(bad("upgrade", value));
    }
    let upgrades = user.table_mut(UPGRADES);
    if value == 0 {
        upgrades.remove(item);
    } else {
        upgrades.set(item, Value::Number(value as f64));
    }
    Ok(())
}

/// Top rank, samples at the cap, every item the game can give and full upgrades, with the paid DLC the
/// account owns and none it does not.
fn max_everything(cat: &Catalog, user: &mut Table, ents: &BTreeSet<String>) {
    if let Some(top) = cat.ranks.last() {
        user.set("player_xp", Value::Number(top.required_xp as f64));
    }
    user.set("research_samples", Value::Number(cat.limits.max_samples as f64));
    let list = user.table_mut(UNLOCKS);
    let mut names = list.strings();
    let wanted = cat.items().map(|i| (i.name.clone(), i.dlc)).chain(cat.cosmetics().map(|c| (c.name.clone(), c.dlc)));
    for (name, dlc) in wanted {
        // A rank or an entitlement grants it, or it is always there: a name in the save adds nothing.
        if !dlc && cat.rank_granting(&name).is_none() && !cat.is_static(&name) && !names.contains(&name) {
            names.push(name);
        }
    }
    list.set_strings(&names);
    let upgrades = user.table_mut(UPGRADES);
    for i in cat.items().filter(|i| !cat.is_locked(&i.name, ents) && i.max_upgrade > 0) {
        upgrades.set(&i.name, Value::Number(i.max_upgrade as f64));
    }
}

/// Back to the first rank with no samples, nothing unlocked and no upgrades. Paid DLC the account does
/// not own and names the catalog doesn't know stay; lifetime stats, cosmetics and the campaign are
/// left as they are.
fn reset_progression(cat: &Catalog, user: &mut Table, ents: &BTreeSet<String>) {
    user.set("player_xp", Value::Number(0.0));
    user.set("research_samples", Value::Number(0.0));
    let is_known = |n: &str| cat.item(n).is_some() || cat.cosmetics().any(|c| c.name == n);
    if user.contains(UNLOCKS) {
        let list = user.table_mut(UNLOCKS);
        let keep: Vec<String> = list.strings().into_iter().filter(|n| !is_known(n) || cat.is_locked(n, ents)).collect();
        list.set_strings(&keep);
    }
    if user.contains(UPGRADES) {
        let upgrades = user.table_mut(UPGRADES);
        upgrades.0.retain(|(k, _)| k.as_str().is_none_or(|n| !is_known(&n) || cat.is_locked(&n, ents)));
    }
}

fn api_unlock_names(data: &ModData) -> BTreeSet<String> {
    match data.0.get(API_MOD).and_then(|v| v.get(API_UNLOCKS)) {
        Some(ModValue::Text(t)) => t.split(',').filter(|s| !s.is_empty()).map(str::to_string).collect(),
        _ => BTreeSet::new(),
    }
}

fn data_rows(values: Option<&BTreeMap<String, ModValue>>) -> Vec<ModDataRow> {
    values.map(|v| v.iter().map(|(k, val)| ModDataRow { key: k.clone(), value: val.to_json() }).collect()).unwrap_or_default()
}

fn field_state(f: &SaveFieldSpec, values: Option<&BTreeMap<String, ModValue>>) -> ModFieldState {
    ModFieldState {
        id: f.id.clone(),
        kind: f.kind.clone(),
        label: f.label.clone(),
        min: f.min,
        max: f.max,
        choices: f.choices.clone(),
        when: f.when.clone(),
        value: values.and_then(|v| v.get(&f.id)).map(ModValue::to_json),
    }
}

fn choice_id(c: &Json) -> Option<&str> {
    c.as_str().or_else(|| c.get("id").and_then(Json::as_str))
}

fn check_field(f: &SaveFieldSpec, v: &ModValue) -> Result<()> {
    let fine = match (f.kind.as_str(), v) {
        ("number", ModValue::Number(n)) => n.is_finite() && f.min.is_none_or(|m| *n >= m) && f.max.is_none_or(|m| *n <= m),
        ("toggle", ModValue::Toggle(_)) => true,
        ("choice", ModValue::Text(t)) => f.choices.as_ref().is_some_and(|c| c.iter().any(|c| choice_id(c) == Some(t.as_str()))),
        _ => false,
    };
    if fine { Ok(()) } else { Err(bad(&f.id, v.to_json())) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::saves::test_support::*;
    use crate::testutil::put;

    fn world_with_sandbox() -> (World, String) {
        let w = world();
        let id = {
            let s = w.saves();
            s.list().unwrap();
            s.create_sandbox("E").unwrap()
        };
        (w, id)
    }

    fn apply(w: &World, id: &str, edits: Vec<Edit>) -> EditorState {
        w.saves().apply_edits(id, &edits).unwrap()
    }

    #[test]
    fn a_rank_sets_xp_and_the_editor_is_refused_on_official() {
        let (w, id) = world_with_sandbox();
        let s = w.saves();
        let st = apply(&w, &id, vec![Edit::SetRank { rank: 5 }, Edit::SetSamples { samples: 25.0 }]);
        assert_eq!(st.rank, 5);
        assert_eq!(st.research_points, 2);
        let official = s.list().unwrap().saves.iter().find(|r| r.kind == crate::saves::SaveKind::Official).unwrap().id.clone();
        assert!(matches!(s.editor_state(&official), Err(ObsidianError::OfficialSave { .. })));
        assert!(matches!(s.apply_edits(&official, &[Edit::MaxEverything]), Err(ObsidianError::OfficialSave { .. })));
    }

    #[test]
    fn paid_dlc_is_refused_and_unknown_names_are_kept() {
        let (w, id) = world_with_sandbox();
        let s = w.saves();
        let mut p = s.read_sandbox(&id).unwrap();
        p.user.root.table_mut(UNLOCKS).set_strings(&["mod_thing".to_string()]);
        s.write_sandbox(&id, &p).unwrap();
        assert!(matches!(s.apply_edits(&id, &[Edit::SetOwned { item: "weapon_plasmarifle".into(), owned: true }]), Err(ObsidianError::PaidDlc(_))));
        assert!(matches!(s.apply_edits(&id, &[Edit::SetUpgrade { item: "weapon_plasmarifle".into(), value: 1 }]), Err(ObsidianError::PaidDlc(_))));
        let st = apply(&w, &id, vec![Edit::SetOwned { item: "stratagem_napalmstrike".into(), owned: true }]);
        assert!(st.unknown_unlocks.contains(&"mod_thing".to_string()));
        assert!(!st.unknown_unlocks.contains(&"stratagem_napalmstrike".to_string()));
        assert!(st.stratagems.iter().any(|i| i.name == "stratagem_napalmstrike" && i.in_unlocks));
    }

    #[test]
    fn new_unlocks_stay_and_max_everything_skips_dlc_and_ranks() {
        let (w, id) = world_with_sandbox();
        let s = w.saves();
        let mut p = s.read_sandbox(&id).unwrap();
        p.user.root.table_mut("new_unlocks").set_strings(&["x".to_string()]);
        s.write_sandbox(&id, &p).unwrap();
        let st = apply(&w, &id, vec![Edit::MaxEverything]);
        assert_eq!(st.rank, 50);
        assert_eq!(st.samples, 3168.0);
        let p = s.read_sandbox(&id).unwrap();
        let unlocks = p.user.root.table(UNLOCKS).unwrap().strings();
        assert!(!unlocks.iter().any(|n| n == "weapon_plasmarifle"), "paid DLC stays out");
        assert!(!unlocks.iter().any(|n| n == "weapon_assaultrifle"), "a rank already grants it");
        assert!(unlocks.iter().any(|n| n == "stratagem_tesla_tower"), "a reward item no rank gives");
        let upgrades = p.user.root.table(UPGRADES).unwrap();
        assert_eq!(upgrades.number("weapon_assaultrifle"), Some(7.0));
        assert_eq!(upgrades.number("weapon_plasmarifle"), None);
        assert_eq!(p.user.root.table("new_unlocks").unwrap().strings(), ["x"], "untouched");
    }

    #[test]
    fn reset_clears_progress_and_keeps_unknown_and_dlc() {
        let (w, id) = world_with_sandbox();
        let s = w.saves();
        let mut p = s.read_sandbox(&id).unwrap();
        p.user.root.set("player_xp", Value::Number(50000.0));
        p.user.root.table_mut(UNLOCKS).set_strings(&["weapon_plasmarifle".to_string(), "mod_thing".to_string(), "stratagem_napalmstrike".to_string()]);
        p.user.root.table_mut(UPGRADES).set("weapon_assaultrifle", Value::Number(7.0));
        p.user.root.table_mut(UPGRADES).set("mod_weapon", Value::Number(1.0));
        s.write_sandbox(&id, &p).unwrap();
        let st = apply(&w, &id, vec![Edit::ResetProgression]);
        assert_eq!((st.xp, st.rank, st.samples), (0.0, 1, 0.0));
        let p = s.read_sandbox(&id).unwrap();
        assert_eq!(p.user.root.table(UNLOCKS).unwrap().strings(), ["weapon_plasmarifle", "mod_thing"]);
        let up = p.user.root.table(UPGRADES).unwrap();
        assert_eq!(up.number("weapon_assaultrifle"), None);
        assert_eq!(up.number("mod_weapon"), Some(1.0));
    }

    #[test]
    fn owned_dlc_can_be_edited_and_the_rest_stays_locked() {
        let w = world();
        own_dlc(&w, &["ENT011"]);
        let s = w.saves();
        s.list().unwrap();
        let id = s.create_sandbox("E").unwrap();
        let st = s.editor_state(&id).unwrap();
        let item = |name: &str, st: &EditorState| st.primary_weapons.iter().chain(&st.stratagems).find(|i| i.name == name).unwrap().clone();
        let owned = item("weapon_plasmarifle", &st);
        assert!(owned.dlc && !owned.locked && owned.owned, "ENT011 grants the plasma rifle before any login");
        let other = item("stratagem_uav_drone", &st);
        assert!(other.dlc && other.locked && !other.owned, "ENT001 is not owned");

        let st = apply(&w, &id, vec![Edit::SetUpgrade { item: "weapon_plasmarifle".into(), value: 1 }]);
        assert_eq!(item("weapon_plasmarifle", &st).upgrade, 1);
        assert!(matches!(s.apply_edits(&id, &[Edit::SetOwned { item: "stratagem_uav_drone".into(), owned: true }]), Err(ObsidianError::PaidDlc(_))));
        assert!(matches!(s.apply_edits(&id, &[Edit::SetUpgrade { item: "stratagem_uav_drone".into(), value: 1 }]), Err(ObsidianError::PaidDlc(_))));

        let cat = catalog::get();
        let index = |name: &str| cat.customization.helmet.iter().position(|c| c.name == name).unwrap() as u32 + 1;
        // ENT011 has no cosmetics, so a helmet of ENT008 stays refused.
        assert!(matches!(s.apply_edits(&id, &[Edit::SetCustomization { slot: "helmet".into(), index: index("cus_helmet_pilot") }]), Err(ObsidianError::PaidDlc(_))));
        assert!(st.cosmetics["helmet"].iter().find(|c| c.name == "cus_helmet_pilot").unwrap().locked);
    }

    #[test]
    fn owned_dlc_cosmetics_can_be_worn() {
        let w = world();
        own_dlc(&w, &["ENT008"]);
        let s = w.saves();
        s.list().unwrap();
        let id = s.create_sandbox("E").unwrap();
        let cat = catalog::get();
        let helmet = cat.customization.helmet.iter().position(|c| c.name == "cus_helmet_pilot").unwrap() as u32 + 1;
        let st = apply(&w, &id, vec![Edit::SetCustomization { slot: "helmet".into(), index: helmet }]);
        assert_eq!(st.customization["helmet"], helmet);
        let c = st.cosmetics["helmet"].iter().find(|c| c.name == "cus_helmet_pilot").unwrap();
        assert!(c.dlc && !c.locked && c.owned);
    }

    #[test]
    fn a_save_carrying_other_entitlements_does_not_count() {
        // Only the account's Official save says what is owned, so an imported Sandbox can't bring DLC along.
        let w = world();
        let s = w.saves();
        s.list().unwrap();
        let id = s.create_sandbox("E").unwrap();
        let mut p = s.read_sandbox(&id).unwrap();
        p.user.root.table_mut("entitlements").table_mut("ENT011").set("id", Value::Text("ENT011".into()));
        s.write_sandbox(&id, &p).unwrap();
        let st = s.editor_state(&id).unwrap();
        assert!(st.primary_weapons.iter().find(|i| i.name == "weapon_plasmarifle").unwrap().locked);
    }

    #[test]
    fn max_everything_includes_owned_dlc_and_skips_the_rest() {
        let w = world();
        own_dlc(&w, &["ENT011"]);
        let s = w.saves();
        s.list().unwrap();
        let id = s.create_sandbox("E").unwrap();
        apply(&w, &id, vec![Edit::MaxEverything]);
        let p = s.read_sandbox(&id).unwrap();
        let unlocks = p.user.root.table(UNLOCKS).unwrap().strings();
        let upgrades = p.user.root.table(UPGRADES).unwrap();
        assert!(!unlocks.iter().any(|n| n == "weapon_plasmarifle"), "the entitlement already grants it");
        assert_eq!(upgrades.number("weapon_plasmarifle"), Some(catalog::get().item("weapon_plasmarifle").unwrap().max_upgrade as f64));
        assert_eq!(upgrades.number("stratagem_uav_drone"), None, "ENT001 is not owned");
    }

    #[test]
    fn reset_clears_owned_dlc_upgrades_and_keeps_the_rest() {
        let w = world();
        own_dlc(&w, &["ENT011"]);
        let s = w.saves();
        s.list().unwrap();
        let id = s.create_sandbox("E").unwrap();
        let mut p = s.read_sandbox(&id).unwrap();
        p.user.root.table_mut(UPGRADES).set("weapon_plasmarifle", Value::Number(1.0));
        p.user.root.table_mut(UPGRADES).set("stratagem_uav_drone", Value::Number(1.0));
        s.write_sandbox(&id, &p).unwrap();
        apply(&w, &id, vec![Edit::ResetProgression]);
        let up = s.read_sandbox(&id).unwrap();
        let up = up.user.root.table(UPGRADES).unwrap();
        assert_eq!(up.number("weapon_plasmarifle"), None);
        assert_eq!(up.number("stratagem_uav_drone"), Some(1.0));
    }

    #[test]
    fn upgrades_cosmetics_boss_flags_and_lifetime_stats() {
        let (w, id) = world_with_sandbox();
        let s = w.saves();
        assert!(s.apply_edits(&id, &[Edit::SetUpgrade { item: "weapon_assaultrifle".into(), value: 8 }]).is_err());
        assert!(s.apply_edits(&id, &[Edit::SetUpgrade { item: "stratagem_turret".into(), value: 3 }]).is_err());
        let helmet_dlc = catalog::get().customization.helmet.iter().position(|c| c.dlc).unwrap() as u32 + 1;
        assert!(matches!(s.apply_edits(&id, &[Edit::SetCustomization { slot: "helmet".into(), index: helmet_dlc }]), Err(ObsidianError::PaidDlc(_))));
        let st = apply(
            &w,
            &id,
            vec![
                Edit::SetUpgrade { item: "weapon_assaultrifle".into(), value: 5 },
                Edit::SetCustomization { slot: "cape".into(), index: 2 },
                Edit::SetBossInfluence { index: 2, value: 50.0 },
                Edit::SetFlag { key: "tutorial_finished".into(), value: true },
                Edit::SetLifetime { key: "kills".into(), value: 99.0 },
            ],
        );
        assert_eq!(st.primary_weapons.iter().find(|i| i.name == "weapon_assaultrifle").unwrap().upgrade, 5);
        assert_eq!(st.customization["cape"], 2);
        assert_eq!(st.boss_influence, [0.0, 50.0, 0.0]);
        assert!(st.flags.iter().any(|f| f.key == "tutorial_finished" && f.value));
        assert_eq!(st.lifetime["kills"], 99.0);
        assert!(s.apply_edits(&id, &[Edit::SetFlag { key: "show_hints".into(), value: true }]).is_err(), "only tutorial flags");
    }

    fn add_mod(w: &World, id: &str, editor: &str) {
        let dir = w.data.join("stage").join(id);
        put(&dir, "obsidian.json", format!(r#"{{"obsidian":1,"id":"{id}","version":"1.0.0","name":{{"en":"M"}},"save_editor":{editor}}}"#).as_bytes());
        Library::open(&w.data).unwrap().install(&dir).unwrap();
    }

    #[test]
    fn mod_sections_show_state_and_validate_values() {
        let (w, id) = world_with_sandbox();
        add_mod(&w, "a.on", r#"{"fields":[{"id":"power","type":"number","min":0,"max":5,"label":{"en":"P"}},{"id":"on","type":"toggle"}],"items":[{"type":"stratagem","name":"s_one","unlock":"manual","label":{"en":"S"}}]}"#);
        add_mod(&w, "b.off", r#"{"fields":[{"id":"mode","type":"choice","choices":["x","y"]}]}"#);
        let lib = Library::open(&w.data).unwrap();
        let mut p = Profile::load(&w.data).unwrap();
        p.sync(&lib).unwrap();
        p.set_enabled("a.on", true).unwrap();
        p.save(&w.data).unwrap();

        let s = w.saves();
        let st = apply(
            &w,
            &id,
            vec![
                Edit::ModSet { mod_id: "a.on".into(), key: "power".into(), value: ModValue::Number(3.0) },
                Edit::ModSet { mod_id: "b.off".into(), key: "mode".into(), value: ModValue::Text("y".into()) },
                Edit::ModItemOwned { mod_id: "a.on".into(), name: "s_one".into(), owned: true },
            ],
        );
        let a = st.mods.iter().find(|m| m.id == "a.on").unwrap();
        assert_eq!(a.state, "enabled");
        assert_eq!(a.fields.iter().find(|f| f.id == "power").unwrap().value, Some(serde_json::json!(3.0)));
        assert_eq!(a.items[0].owned, Some(true));
        assert_eq!(st.mods.iter().find(|m| m.id == "b.off").unwrap().state, "disabled");
        let bad_value = Edit::ModSet { mod_id: "a.on".into(), key: "power".into(), value: ModValue::Number(9.0) };
        assert!(s.apply_edits(&id, &[bad_value]).is_err());
        let unknown_mod = Edit::ModSet { mod_id: "none".into(), key: "x".into(), value: ModValue::Toggle(true) };
        assert!(matches!(s.apply_edits(&id, &[unknown_mod]), Err(ObsidianError::ModMissing(_))));

        // Data of a mod no longer installed shows by id and can be cleared.
        let file = s.sandbox_mod(&id);
        std::fs::write(&file, std::fs::read_to_string(&file).unwrap() + "gone.mod\tk\tb\t1\n").unwrap();
        let st = s.editor_state(&id).unwrap();
        let gone = st.mods.iter().find(|m| m.id == "gone.mod").unwrap();
        assert_eq!(gone.state, "removed");
        assert_eq!(gone.data[0].key, "k");
        let st = apply(&w, &id, vec![Edit::ModClear { mod_id: "gone.mod".into() }]);
        assert!(!st.mods.iter().any(|m| m.id == "gone.mod"));
    }
}
