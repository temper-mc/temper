use temper_core::block_state_id::BlockStateId;

/// Sentinel value meaning the block cannot be traversed.
pub const IMPASSABLE: i32 = i32::MIN;

/// Returns the pathfinding penalty for a block, following the Minecraft wiki penalty system:
/// - IMPASSABLE: solid blocks, fences, walls, closed doors, cactus, lava, etc.
/// - 0  : air, open trapdoors, lily pads, vegetation
/// - 8  : water, honey blocks, danger zones (near fire/cactus)
/// - 16 : fire, lava, magma, lit campfire
pub fn block_penalty(id: BlockStateId) -> i32 {
    if id.raw() == 0 {
        return 0; // air
    }

    let Some(data) = id.to_block_data() else {
        return IMPASSABLE;
    };

    let name = data.name.trim_start_matches("minecraft:");

    if name.ends_with("air") {
        return 0;
    }

    // Damage blocks (penalty: 16)
    if matches!(name, "fire" | "soul_fire" | "magma_block") {
        return 16;
    }
    if name.ends_with("_campfire") {
        return 16;
    }

    // Liquids
    if name == "lava" {
        return IMPASSABLE;
    }
    if name == "water" || name == "bubble_column" {
        return 8;
    }

    // Impassable hazards
    if matches!(
        name,
        "cactus" | "sweet_berry_bush" | "cobweb" | "powder_snow"
    ) {
        return IMPASSABLE;
    }

    // Fences, walls
    if name.ends_with("_fence") || name.ends_with("_wall") {
        return IMPASSABLE;
    }

    // Doors and fence gates: passable only when open
    if name.ends_with("_door") || name.ends_with("_fence_gate") {
        let open = data
            .properties
            .as_ref()
            .and_then(|p| p.get("open"))
            .map(|v| v == "true")
            .unwrap_or(false);
        return if open { 0 } else { IMPASSABLE };
    }

    // Trapdoors: passable only when open
    if name.ends_with("_trapdoor") {
        let open = data
            .properties
            .as_ref()
            .and_then(|p| p.get("open"))
            .map(|v| v == "true")
            .unwrap_or(false);
        return if open { 0 } else { IMPASSABLE };
    }

    // Known non-solid blocks
    if is_non_solid(name) {
        return 0;
    }

    // Default: solid/impassable
    IMPASSABLE
}

fn is_non_solid(name: &str) -> bool {
    if matches!(
        name,
        "grass"
            | "short_grass"
            | "tall_grass"
            | "fern"
            | "large_fern"
            | "dead_bush"
            | "lily_pad"
            | "big_dripleaf"
            | "small_dripleaf"
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
