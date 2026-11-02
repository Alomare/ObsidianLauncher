//! Resource name hashing (MurmurHash64A, seed 0) and the `#hex` notation.

use crate::{ObsidianError, Result};

const M: u64 = 0xc6a4a7935bd1e995;
const R: u32 = 47;

/// Bitsquid resource type names, enough to name every type HD1 bundles use.
const TYPE_NAMES: &[&str] = &[
    "animation", "animation_curves", "apb", "bik", "bones", "crypto", "data", "entity", "flow",
    "font", "ivf", "level", "lua", "material", "mod", "mouse_cursor", "network_config", "package",
    "particles", "physics_properties", "render_config", "rt_pipeline", "scene", "shader",
    "shader_library", "shader_library_group", "shading_environment", "shading_environment_mapping",
    "slug", "slug_album", "sound_environment", "state_machine", "strings", "surface_properties",
    "texture", "timpani_bank", "timpani_master", "tome", "unit", "vector_field", "wav",
    "wwise_bank", "wwise_dep", "wwise_metadata", "wwise_stream", "config", "script_flow_nodes",
];

/// MurmurHash64A with seed 0, as the engine hashes resource names.
pub fn murmur64(data: &[u8]) -> u64 {
    let mut h = (data.len() as u64).wrapping_mul(M);
    let (blocks, tail) = data.as_chunks::<8>();
    for block in blocks {
        let mut k = u64::from_le_bytes(*block);
        k = k.wrapping_mul(M);
        k ^= k >> R;
        k = k.wrapping_mul(M);
        h ^= k;
        h = h.wrapping_mul(M);
    }
    if !tail.is_empty() {
        let mut buf = [0u8; 8];
        buf[..tail.len()].copy_from_slice(tail);
        h ^= u64::from_le_bytes(buf);
        h = h.wrapping_mul(M);
    }
    h ^= h >> R;
    h = h.wrapping_mul(M);
    h ^= h >> R;
    h
}

pub fn hash_name(name: &str) -> u64 {
    murmur64(name.as_bytes())
}

pub fn type_name(hash: u64) -> Option<&'static str> {
    TYPE_NAMES.iter().copied().find(|t| hash_name(t) == hash)
}

/// A game path such as `lua/boot/boot`, or `#` plus 16 lowercase hex digits.
pub fn parse_ref(s: &str) -> Result<u64> {
    match s.strip_prefix('#') {
        Some(hex) if hex.len() == 16 && hex.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) => {
            Ok(u64::from_str_radix(hex, 16).unwrap())
        }
        Some(_) => Err(ObsidianError::BadRef(s.to_string())),
        None if s.is_empty() => Err(ObsidianError::BadRef(s.to_string())),
        None => Ok(hash_name(s)),
    }
}

pub fn format_type(hash: u64) -> String {
    type_name(hash).map_or_else(|| format_name(hash), str::to_string)
}

pub fn format_name(hash: u64) -> String {
    format!("#{hash:016x}")
}

/// Resource names found in the game's bundles (tools/make-names.py), `<hash> <name>` per line.
const NAMES: &str = include_str!("../data/names.txt");
/// Names the bundles don't spell out but Obsidian knows.
const EXTRA_NAMES: &[&str] = &[
    "resource_packages/boot",
    "foundation/lua/build_date_information",
    "lua/boot/boot",
    "lua/components/inventory_component",
    "lua/menu/menu_screen_campaign_multiplayer",
    "lua/generated/unit_path_to_network_object",
];

fn known_names() -> &'static std::collections::HashMap<u64, &'static str> {
    static MAP: std::sync::OnceLock<std::collections::HashMap<u64, &'static str>> = std::sync::OnceLock::new();
    MAP.get_or_init(|| {
        let mut m: std::collections::HashMap<u64, &'static str> = NAMES
            .lines()
            .filter_map(|l| l.split_once(' '))
            .filter_map(|(h, n)| u64::from_str_radix(h, 16).ok().map(|h| (h, n)))
            .collect();
        m.extend(EXTRA_NAMES.iter().map(|n| (hash_name(n), *n)));
        m
    })
}

/// A resource or bundle name for people: the name when Obsidian knows it, else `#hash`.
pub fn display_name(hash: u64) -> String {
    known_names().get(&hash).map_or_else(|| format_name(hash), |n| n.to_string())
}

/// `display_name` for a name as manifests and bundles write it: `#hex`, bare hex, or a name.
pub fn display_text(text: &str) -> String {
    let hex = text.strip_prefix('#').unwrap_or(text);
    match (hex.len(), u64::from_str_radix(hex, 16)) {
        (16, Ok(h)) => display_name(h),
        _ => text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scripts_the_bundles_dont_spell_out_have_names_too() {
        assert_eq!(display_name(0x520e79e14b304a44), "lua/components/inventory_component");
        assert_eq!(display_name(0x54ca95ed20705469), "lua/menu/menu_screen_campaign_multiplayer");
        assert_eq!(display_name(0xb04000fc3086bd36), "lua/generated/unit_path_to_network_object");
    }

    #[test]
    fn known_names_replace_hashes_for_people() {
        assert_eq!(display_name(0x15a03cea0ee39970), "lua/states/mission_end_states/mission_end_state_planet_completed");
        assert_eq!(display_text("9e13b2414b41b842"), "resource_packages/boot");
        assert_eq!(display_text("#15a03cea0ee39970"), display_name(0x15a03cea0ee39970));
        assert_eq!(display_name(0x0123456789abcdef), "#0123456789abcdef");
        assert_eq!(display_text("lua/my_script"), "lua/my_script");
    }

    #[test]
    fn murmur_matches_known_game_names() {
        assert_eq!(hash_name("lua"), 0xa14e8dfa2cd117e2);
        assert_eq!(hash_name("foundation/lua/build_date_information"), 0x35546153a5bb59d2);
        assert_eq!(hash_name("lua/boot/boot"), 0x37048a618abddc81);
        assert_eq!(hash_name("resource_packages/boot"), 0x9e13b2414b41b842);
    }

    #[test]
    fn parses_paths_and_hex_refs() {
        assert_eq!(parse_ref("lua").unwrap(), 0xa14e8dfa2cd117e2);
        assert_eq!(parse_ref("#35546153a5bb59d2").unwrap(), 0x35546153a5bb59d2);
        assert!(parse_ref("#123").is_err());
        assert_eq!(format_type(0xa14e8dfa2cd117e2), "lua");
        assert_eq!(format_name(0x35546153a5bb59d2), "#35546153a5bb59d2");
    }
}
