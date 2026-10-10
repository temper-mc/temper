#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Attribute {
    pub id: u16,
    pub name: &'static str,
    pub default_value: f64,
}
impl Attribute {
    pub const AIR_DRAG_MODIFIER: Attribute = Attribute {
        id: 0,
        name: "air_drag_modifier",
        default_value: 1.0,
    };
    pub const ARMOR: Attribute = Attribute {
        id: 1,
        name: "armor",
        default_value: 0.0,
    };
    pub const ARMOR_TOUGHNESS: Attribute = Attribute {
        id: 2,
        name: "armor_toughness",
        default_value: 0.0,
    };
    pub const ATTACK_DAMAGE: Attribute = Attribute {
        id: 3,
        name: "attack_damage",
        default_value: 2.0,
    };
    pub const ATTACK_KNOCKBACK: Attribute = Attribute {
        id: 4,
        name: "attack_knockback",
        default_value: 0.0,
    };
    pub const ATTACK_SPEED: Attribute = Attribute {
        id: 5,
        name: "attack_speed",
        default_value: 4.0,
    };
    pub const BELOW_NAME_DISTANCE: Attribute = Attribute {
        id: 6,
        name: "below_name_distance",
        default_value: 10.0,
    };
    pub const BLOCK_BREAK_SPEED: Attribute = Attribute {
        id: 7,
        name: "block_break_speed",
        default_value: 1.0,
    };
    pub const BLOCK_INTERACTION_RANGE: Attribute = Attribute {
        id: 8,
        name: "block_interaction_range",
        default_value: 4.5,
    };
    pub const BOUNCINESS: Attribute = Attribute {
        id: 9,
        name: "bounciness",
        default_value: 0.0,
    };
    pub const BURNING_TIME: Attribute = Attribute {
        id: 10,
        name: "burning_time",
        default_value: 1.0,
    };
    pub const CAMERA_DISTANCE: Attribute = Attribute {
        id: 11,
        name: "camera_distance",
        default_value: 4.0,
    };
    pub const ENTITY_INTERACTION_RANGE: Attribute = Attribute {
        id: 13,
        name: "entity_interaction_range",
        default_value: 3.0,
    };
    pub const EXPLOSION_KNOCKBACK_RESISTANCE: Attribute = Attribute {
        id: 12,
        name: "explosion_knockback_resistance",
        default_value: 0.0,
    };
    pub const FALL_DAMAGE_MULTIPLIER: Attribute = Attribute {
        id: 14,
        name: "fall_damage_multiplier",
        default_value: 1.0,
    };
    pub const FLYING_SPEED: Attribute = Attribute {
        id: 15,
        name: "flying_speed",
        default_value: 0.4,
    };
    pub const FOLLOW_RANGE: Attribute = Attribute {
        id: 16,
        name: "follow_range",
        default_value: 32.0,
    };
    pub const FRICTION_MODIFIER: Attribute = Attribute {
        id: 17,
        name: "friction_modifier",
        default_value: 1.0,
    };
    pub const GRAVITY: Attribute = Attribute {
        id: 18,
        name: "gravity",
        default_value: 0.1,
    };
    pub const JUMP_STRENGTH: Attribute = Attribute {
        id: 19,
        name: "jump_strength",
        default_value: 0.4,
    };
    pub const KNOCKBACK_RESISTANCE: Attribute = Attribute {
        id: 20,
        name: "knockback_resistance",
        default_value: 0.0,
    };
    pub const LUCK: Attribute = Attribute {
        id: 21,
        name: "luck",
        default_value: 0.0,
    };
    pub const MAX_ABSORPTION: Attribute = Attribute {
        id: 22,
        name: "max_absorption",
        default_value: 0.0,
    };
    pub const MAX_HEALTH: Attribute = Attribute {
        id: 23,
        name: "max_health",
        default_value: 20.0,
    };
    pub const MINING_EFFICIENCY: Attribute = Attribute {
        id: 24,
        name: "mining_efficiency",
        default_value: 0.0,
    };
    pub const MOVEMENT_EFFICIENCY: Attribute = Attribute {
        id: 25,
        name: "movement_efficiency",
        default_value: 0.0,
    };
    pub const MOVEMENT_SPEED: Attribute = Attribute {
        id: 26,
        name: "movement_speed",
        default_value: 0.7,
    };
    pub const NAME_TAG_DISTANCE: Attribute = Attribute {
        id: 27,
        name: "name_tag_distance",
        default_value: 64.0,
    };
    pub const OXYGEN_BONUS: Attribute = Attribute {
        id: 28,
        name: "oxygen_bonus",
        default_value: 0.0,
    };
    pub const SAFE_FALL_DISTANCE: Attribute = Attribute {
        id: 29,
        name: "safe_fall_distance",
        default_value: 3.0,
    };
    pub const SCALE: Attribute = Attribute {
        id: 30,
        name: "scale",
        default_value: 1.0,
    };
    pub const SNEAKING_SPEED: Attribute = Attribute {
        id: 31,
        name: "sneaking_speed",
        default_value: 0.3,
    };
    pub const SPAWN_REINFORCEMENTS: Attribute = Attribute {
        id: 32,
        name: "spawn_reinforcements",
        default_value: 0.0,
    };
    pub const STEP_HEIGHT: Attribute = Attribute {
        id: 33,
        name: "step_height",
        default_value: 0.6,
    };
    pub const SUBMERGED_MINING_SPEED: Attribute = Attribute {
        id: 34,
        name: "submerged_mining_speed",
        default_value: 0.2,
    };
    pub const SWEEPING_DAMAGE_RATIO: Attribute = Attribute {
        id: 35,
        name: "sweeping_damage_ratio",
        default_value: 0.0,
    };
    pub const TEMPT_RANGE: Attribute = Attribute {
        id: 36,
        name: "tempt_range",
        default_value: 10.0,
    };
    pub const WATER_MOVEMENT_EFFICIENCY: Attribute = Attribute {
        id: 37,
        name: "water_movement_efficiency",
        default_value: 0.0,
    };
    pub const WAYPOINT_RECEIVE_RANGE: Attribute = Attribute {
        id: 39,
        name: "waypoint_receive_range",
        default_value: 0.0,
    };
    pub const WAYPOINT_TRANSMIT_RANGE: Attribute = Attribute {
        id: 38,
        name: "waypoint_transmit_range",
        default_value: 0.0,
    };
    pub const ATTRIBUTES_BY_NAME: &'static [(&'static str, &'static Self)] = &[
        ("air_drag_modifier", &Self::AIR_DRAG_MODIFIER),
        ("armor", &Self::ARMOR),
        ("armor_toughness", &Self::ARMOR_TOUGHNESS),
        ("attack_damage", &Self::ATTACK_DAMAGE),
        ("attack_knockback", &Self::ATTACK_KNOCKBACK),
        ("attack_speed", &Self::ATTACK_SPEED),
        ("below_name_distance", &Self::BELOW_NAME_DISTANCE),
        ("block_break_speed", &Self::BLOCK_BREAK_SPEED),
        ("block_interaction_range", &Self::BLOCK_INTERACTION_RANGE),
        ("bounciness", &Self::BOUNCINESS),
        ("burning_time", &Self::BURNING_TIME),
        ("camera_distance", &Self::CAMERA_DISTANCE),
        ("entity_interaction_range", &Self::ENTITY_INTERACTION_RANGE),
        (
            "explosion_knockback_resistance",
            &Self::EXPLOSION_KNOCKBACK_RESISTANCE,
        ),
        ("fall_damage_multiplier", &Self::FALL_DAMAGE_MULTIPLIER),
        ("flying_speed", &Self::FLYING_SPEED),
        ("follow_range", &Self::FOLLOW_RANGE),
        ("friction_modifier", &Self::FRICTION_MODIFIER),
        ("gravity", &Self::GRAVITY),
        ("jump_strength", &Self::JUMP_STRENGTH),
        ("knockback_resistance", &Self::KNOCKBACK_RESISTANCE),
        ("luck", &Self::LUCK),
        ("max_absorption", &Self::MAX_ABSORPTION),
        ("max_health", &Self::MAX_HEALTH),
        ("mining_efficiency", &Self::MINING_EFFICIENCY),
        ("movement_efficiency", &Self::MOVEMENT_EFFICIENCY),
        ("movement_speed", &Self::MOVEMENT_SPEED),
        ("name_tag_distance", &Self::NAME_TAG_DISTANCE),
        ("oxygen_bonus", &Self::OXYGEN_BONUS),
        ("safe_fall_distance", &Self::SAFE_FALL_DISTANCE),
        ("scale", &Self::SCALE),
        ("sneaking_speed", &Self::SNEAKING_SPEED),
        ("spawn_reinforcements", &Self::SPAWN_REINFORCEMENTS),
        ("step_height", &Self::STEP_HEIGHT),
        ("submerged_mining_speed", &Self::SUBMERGED_MINING_SPEED),
        ("sweeping_damage_ratio", &Self::SWEEPING_DAMAGE_RATIO),
        ("tempt_range", &Self::TEMPT_RANGE),
        (
            "water_movement_efficiency",
            &Self::WATER_MOVEMENT_EFFICIENCY,
        ),
        ("waypoint_receive_range", &Self::WAYPOINT_RECEIVE_RANGE),
        ("waypoint_transmit_range", &Self::WAYPOINT_TRANSMIT_RANGE),
    ];
    #[doc = r" Try to parse an `Attribute` from a resource location string."]
    pub fn from_name(name: &str) -> Option<&'static Self> {
        let name = name.strip_prefix("minecraft:").unwrap_or(name);
        Self::ATTRIBUTES_BY_NAME
            .binary_search_by_key(&name, |&(k, _)| k)
            .ok()
            .map(|idx| Self::ATTRIBUTES_BY_NAME[idx].1)
    }
    #[doc = r" Try to get an `Attribute` from its ID."]
    pub const fn from_id(id: u16) -> Option<&'static Self> {
        match id {
            0 => Some(&Self::AIR_DRAG_MODIFIER),
            1 => Some(&Self::ARMOR),
            2 => Some(&Self::ARMOR_TOUGHNESS),
            3 => Some(&Self::ATTACK_DAMAGE),
            4 => Some(&Self::ATTACK_KNOCKBACK),
            5 => Some(&Self::ATTACK_SPEED),
            6 => Some(&Self::BELOW_NAME_DISTANCE),
            7 => Some(&Self::BLOCK_BREAK_SPEED),
            8 => Some(&Self::BLOCK_INTERACTION_RANGE),
            9 => Some(&Self::BOUNCINESS),
            10 => Some(&Self::BURNING_TIME),
            11 => Some(&Self::CAMERA_DISTANCE),
            13 => Some(&Self::ENTITY_INTERACTION_RANGE),
            12 => Some(&Self::EXPLOSION_KNOCKBACK_RESISTANCE),
            14 => Some(&Self::FALL_DAMAGE_MULTIPLIER),
            15 => Some(&Self::FLYING_SPEED),
            16 => Some(&Self::FOLLOW_RANGE),
            17 => Some(&Self::FRICTION_MODIFIER),
            18 => Some(&Self::GRAVITY),
            19 => Some(&Self::JUMP_STRENGTH),
            20 => Some(&Self::KNOCKBACK_RESISTANCE),
            21 => Some(&Self::LUCK),
            22 => Some(&Self::MAX_ABSORPTION),
            23 => Some(&Self::MAX_HEALTH),
            24 => Some(&Self::MINING_EFFICIENCY),
            25 => Some(&Self::MOVEMENT_EFFICIENCY),
            26 => Some(&Self::MOVEMENT_SPEED),
            27 => Some(&Self::NAME_TAG_DISTANCE),
            28 => Some(&Self::OXYGEN_BONUS),
            29 => Some(&Self::SAFE_FALL_DISTANCE),
            30 => Some(&Self::SCALE),
            31 => Some(&Self::SNEAKING_SPEED),
            32 => Some(&Self::SPAWN_REINFORCEMENTS),
            33 => Some(&Self::STEP_HEIGHT),
            34 => Some(&Self::SUBMERGED_MINING_SPEED),
            35 => Some(&Self::SWEEPING_DAMAGE_RATIO),
            36 => Some(&Self::TEMPT_RANGE),
            37 => Some(&Self::WATER_MOVEMENT_EFFICIENCY),
            39 => Some(&Self::WAYPOINT_RECEIVE_RANGE),
            38 => Some(&Self::WAYPOINT_TRANSMIT_RANGE),
            _ => None,
        }
    }
    #[doc = r" Get all attributes as a slice."]
    pub fn all() -> &'static [&'static Self] {
        &[
            &Self::AIR_DRAG_MODIFIER,
            &Self::ARMOR,
            &Self::ARMOR_TOUGHNESS,
            &Self::ATTACK_DAMAGE,
            &Self::ATTACK_KNOCKBACK,
            &Self::ATTACK_SPEED,
            &Self::BELOW_NAME_DISTANCE,
            &Self::BLOCK_BREAK_SPEED,
            &Self::BLOCK_INTERACTION_RANGE,
            &Self::BOUNCINESS,
            &Self::BURNING_TIME,
            &Self::CAMERA_DISTANCE,
            &Self::ENTITY_INTERACTION_RANGE,
            &Self::EXPLOSION_KNOCKBACK_RESISTANCE,
            &Self::FALL_DAMAGE_MULTIPLIER,
            &Self::FLYING_SPEED,
            &Self::FOLLOW_RANGE,
            &Self::FRICTION_MODIFIER,
            &Self::GRAVITY,
            &Self::JUMP_STRENGTH,
            &Self::KNOCKBACK_RESISTANCE,
            &Self::LUCK,
            &Self::MAX_ABSORPTION,
            &Self::MAX_HEALTH,
            &Self::MINING_EFFICIENCY,
            &Self::MOVEMENT_EFFICIENCY,
            &Self::MOVEMENT_SPEED,
            &Self::NAME_TAG_DISTANCE,
            &Self::OXYGEN_BONUS,
            &Self::SAFE_FALL_DISTANCE,
            &Self::SCALE,
            &Self::SNEAKING_SPEED,
            &Self::SPAWN_REINFORCEMENTS,
            &Self::STEP_HEIGHT,
            &Self::SUBMERGED_MINING_SPEED,
            &Self::SWEEPING_DAMAGE_RATIO,
            &Self::TEMPT_RANGE,
            &Self::WATER_MOVEMENT_EFFICIENCY,
            &Self::WAYPOINT_RECEIVE_RANGE,
            &Self::WAYPOINT_TRANSMIT_RANGE,
        ]
    }
}
