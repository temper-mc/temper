use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::OnceLock;

static REGISTRY: OnceLock<BlockRegistry> = OnceLock::new();

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawBlockData<'a> {
    id: u32,
    name: &'a str,
    hardness: Option<f32>,
    resistance: f32,
    material: Option<&'a str>,
    transparent: bool,
    min_state_id: u32,
    max_state_id: u32,
    states: Vec<RawState>,
}

#[derive(Deserialize)]
struct RawState {
    id: u32,
    properties: Option<HashMap<String, Value>>,
}

struct BlockRegistry {
    hardness: Vec<f32>,
    resistance: Vec<f32>,
    is_transparent: Vec<bool>,
    material: Vec<BlockMaterial>,
    state_to_block: Vec<u32>,
    is_solid: Vec<bool>,
    names: Vec<String>,
}

// --- Enums ---

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum BlockMaterial {
    Default = 0,
    Cobweb = 1,
    GourdAxe = 2,
    IncorrectForWoodenTool = 3,
    LeavesHoe = 4,
    Axe = 5,
    Hoe = 6,
    Pickaxe = 7,
    Shovel = 8,
    PlantAxe = 9,
    SwordInstantlyMines = 10,
    VinePlantAxe = 11,
    Wool = 12,
    Unknown = 255,
}

impl BlockMaterial {
    pub fn from_str(s: &str) -> Self {
        match s {
            "default" => Self::Default,
            "coweb" => Self::Cobweb,
            "gourd;mineable/axe" => Self::GourdAxe,
            "incorrect_for_wooden_tool" => Self::IncorrectForWoodenTool,
            "leaves;mineable/hoe" => Self::LeavesHoe,
            "mineable/axe" => Self::Axe,
            "mineable/hoe" => Self::Hoe,
            "mineable/pickaxe" => Self::Pickaxe,
            "mineable/shovel" => Self::Shovel,
            "plant;mineable/axe" => Self::PlantAxe,
            "sword_instantly_mines" => Self::SwordInstantlyMines,
            "vine_or_glow_lichen;plant;mineable/axe" => Self::VinePlantAxe,
            "wool" => Self::Wool,
            _ => Self::Unknown,
        }
    }
}

// --- Getters (Using State IDs) ---

#[inline(always)]
fn base_id(state_id: u32) -> u32 {
    let reg = REGISTRY.get().expect("Registry not initialized");
    reg.state_to_block
        .get(state_id as usize)
        .copied()
        .unwrap_or(0)
}

#[inline(always)]
pub fn hardness(state_id: u32) -> f32 {
    let reg = REGISTRY.get().expect("Registry not initialized");
    reg.hardness
        .get(base_id(state_id) as usize)
        .copied()
        .unwrap_or(0.0)
}

#[inline(always)]
pub fn material(state_id: u32) -> BlockMaterial {
    let reg = REGISTRY.get().expect("Registry not initialized");
    reg.material
        .get(base_id(state_id) as usize)
        .copied()
        .unwrap_or(BlockMaterial::Default)
}

#[inline(always)]
pub fn is_transparent(state_id: u32) -> bool {
    let reg = REGISTRY.get().expect("Registry not initialized");
    reg.is_transparent
        .get(base_id(state_id) as usize)
        .copied()
        .unwrap_or(false)
}

#[inline(always)]
pub fn is_solid(state_id: u32) -> bool {
    let reg = REGISTRY.get().expect("Registry not initialized");
    reg.is_solid
        .get(state_id as usize)
        .copied()
        .unwrap_or(false)
}

#[inline(always)]
pub fn name(state_id: u32) -> &'static str {
    let reg = REGISTRY.get().expect("Registry not initialized");
    reg.names
        .get(base_id(state_id) as usize)
        .map(|s| s.as_str())
        .unwrap_or("air") // Fallback to air if out of bounds
}

fn is_non_solid_decoration(name: &str) -> bool {
    if matches!(
        name,
        "grass"
            | "short_grass"
            | "tall_grass"
            | "fern"
            | "large_fern"
            | "dead_bush"
            | "snow"
            | "string"
            | "nether_portal"
            | "spore_blossom"
            | "glow_lichen"
            | "dandelion"
            | "poppy"
            | "blue_orchid"
            | "allium"
            | "azure_bluet"
            | "oxeye_daisy"
            | "cornflower"
            | "lily_of_the_valley"
            | "wither_rose"
            | "sunflower"
            | "lilac"
            | "rose_bush"
            | "peony"
            | "torchflower"
            | "pitcher_plant"
            | "pitcher_pod"
            | "sweet_berry_bush"
            | "cobweb"
            | "powder_snow"
            | "redstone_wire"
            | "rail"
            | "powered_rail"
            | "detector_rail"
            | "activator_rail"
            | "tripwire"
            | "tripwire_hook"
            | "structure_void"
    ) {
        return true;
    }

    name.ends_with("_button")
        || name.ends_with("_pressure_plate")
        || name.ends_with("_sign")
        || name.ends_with("_banner")
        || name.ends_with("_carpet")
        || name.ends_with("_torch")
        || name.ends_with("_sapling")
        || name.ends_with("_mushroom")
        || name.ends_with("_flower")
        || name.ends_with("_vine")
        || name.ends_with("_roots")
}

pub fn init(json_string: &str) {
    let raw_blocks: Vec<RawBlockData> =
        serde_json::from_str(json_string).expect("Failed to parse blocks.json");

    let max_id = raw_blocks.iter().map(|b| b.id).max().unwrap_or(0) as usize;
    let highest_state = raw_blocks.iter().map(|b| b.max_state_id).max().unwrap_or(0) as usize;

    let mut hardness = vec![0.0; max_id + 1];
    let mut resistance = vec![0.0; max_id + 1];
    let mut is_transparent = vec![false; max_id + 1];
    let mut material = vec![BlockMaterial::Default; max_id + 1];
    let mut state_to_block = vec![0; highest_state + 1];
    let mut is_solid = vec![false; highest_state + 1];
    let mut names = vec![String::new(); max_id + 1];

    for block in raw_blocks {
        let idx = block.id as usize;
        names[idx] = block.name.to_string();
        hardness[idx] = block.hardness.unwrap_or(0.0);
        resistance[idx] = block.resistance;
        material[idx] = block
            .material
            .map(BlockMaterial::from_str)
            .unwrap_or(BlockMaterial::Default);
        is_transparent[idx] = block.transparent;

        // Fallback mapping for all states
        for state_id in block.min_state_id..=block.max_state_id {
            state_to_block[state_id as usize] = block.id;
        }

        let name = block.name;

        // Process specific properties and overwrite the solidity map
        for state in block.states {
            let state_id = state.id as usize;
            state_to_block[state_id] = block.id;

            let mut solid = true;

            if name.ends_with("air")
                || matches!(
                    name,
                    "water" | "lava" | "bubble_column" | "fire" | "soul_fire"
                )
            {
                solid = false;
            } else if name.ends_with("_campfire") {
                solid = true;
            } else if name.ends_with("_door")
                || name.ends_with("_fence_gate")
                || name.ends_with("_trapdoor")
            {
                // Prismarine handles booleans inconsistently. Check for native bool OR string "true".
                let is_open = state
                    .properties
                    .as_ref()
                    .and_then(|p| p.get("open"))
                    .map(|v| v.as_bool().unwrap_or_else(|| v.as_str() == Some("true")))
                    .unwrap_or(false);
                solid = !is_open;
            } else if is_non_solid_decoration(name) {
                solid = false;
            }

            is_solid[state_id] = solid;
        }
    }

    REGISTRY
        .set(BlockRegistry {
            hardness,
            resistance,
            is_transparent,
            material,
            state_to_block,
            is_solid,
            names,
        })
        .map_err(|_| "Registry already initialized")
        .unwrap();
}
