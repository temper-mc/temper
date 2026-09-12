use crate::item_to_block_mapping::write_item_to_block_mapping;
use crate::registry_packets::write_registry_packets;
use crate::tag_packets::write_tag_packets;
use crate::write_if_changed;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub fn generate_source(assets_path: PathBuf) {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR missing"));
    let reports_dir = assets_path.join("generated").join("reports");
    let data_dir = assets_path.join("generated").join("data");
    let blockstates_path = write_blockstates(&out_dir, &reports_dir);
    let item_to_block_mapping_path = write_item_to_block_mapping(&out_dir, &reports_dir);
    let registry_packets_path = write_registry_packets(&out_dir, &assets_path);
    let tag_packets_path = write_tag_packets(&out_dir, &assets_path, &data_dir);

    let mut content = String::new();
    content.push_str("pub const BLOCKSTATES: &str = include_str!(");
    content.push_str(&format!("{:?}", blockstates_path.to_string_lossy()));
    content.push_str(");\n\n");
    content.push_str("pub const ITEM_TO_BLOCK_MAPPING: &str = include_str!(");
    content.push_str(&format!(
        "{:?}",
        item_to_block_mapping_path.to_string_lossy()
    ));
    content.push_str(");\n\n");
    content.push_str("pub const REGISTRY_PACKETS: &str = include_str!(");
    content.push_str(&format!("{:?}", registry_packets_path.to_string_lossy()));
    content.push_str(");\n\n");
    content.push_str("pub const TAG_PACKETS: &str = include_str!(");
    content.push_str(&format!("{:?}", tag_packets_path.to_string_lossy()));
    content.push_str(");\n\n");
    content.push_str("pub mod reports {\n");
    write_dir(&mut content, &reports_dir, 1);
    content.push_str("}\n");

    content.push_str(
        &format!(
            "#[doc = include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/src/asset_path_macro.md\"))]\n\
            #[macro_export]\n\
            macro_rules! asset_path {{\n\
                ($($path:expr),* $(,)?) => {{\n\
                    concat!({:?}, $({:?}, $path),*)\
                }};\
            }}",
            assets_path.join("generated").to_string_lossy(),
            std::path::MAIN_SEPARATOR_STR,
        )
    );

    write_if_changed(out_dir.join("generated.rs"), content)
        .expect("Failed to write generated source");
}

fn write_blockstates(out_dir: &Path, reports_dir: &Path) -> PathBuf {
    let blocks_path = reports_dir.join("blocks.json");
    let blocks = fs::read_to_string(&blocks_path).expect("Failed to read generated blocks report");
    let blocks: serde_json::Value =
        serde_json::from_str(&blocks).expect("Failed to parse generated blocks report");
    let blocks = blocks
        .as_object()
        .expect("Generated blocks report should be an object");

    let mut blockstates = BTreeMap::new();

    for (name, block) in blocks {
        let states = block
            .get("states")
            .and_then(serde_json::Value::as_array)
            .expect("Generated block should have states");

        for state in states {
            let id = state
                .get("id")
                .and_then(serde_json::Value::as_u64)
                .expect("Generated block state should have an id");
            let mut blockstate = serde_json::Map::new();

            blockstate.insert("name".to_string(), serde_json::Value::String(name.clone()));

            if let Some(properties) = state.get("properties") {
                blockstate.insert("properties".to_string(), properties.clone());
            } else {
                blockstate.insert("default".to_string(), serde_json::Value::Bool(true));
            }

            blockstates.insert(id, serde_json::Value::Object(blockstate));
        }
    }

    let blockstates_path = out_dir.join("blockstates.json");
    let blockstates =
        serde_json::to_string(&blockstates).expect("Failed to serialize generated blockstates");
    write_if_changed(&blockstates_path, blockstates)
        .expect("Failed to write generated blockstates");
    blockstates_path
}

fn write_dir(content: &mut String, dir: &Path, depth: usize) {
    let mut entries = fs::read_dir(dir)
        .expect("Failed to read generated reports directory")
        .collect::<Result<Vec<_>, _>>()
        .expect("Failed to read generated reports entry");

    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();

        if path.is_dir() {
            write_indent(content, depth);
            content.push_str("pub mod ");
            content.push_str(&to_ident(&name, false));
            content.push_str(" {\n");

            write_dir(content, &path, depth + 1);

            write_indent(content, depth);
            content.push_str("}\n");
        } else {
            let stem = path
                .file_stem()
                .expect("Generated report file missing stem")
                .to_string_lossy();

            write_indent(content, depth);
            content.push_str("pub const ");
            content.push_str(&to_ident(&stem, true));
            content.push_str(": &str = include_str!(");
            content.push_str(&format!("{:?}", path.to_string_lossy()));
            content.push_str(");\n");
        }
    }
}

fn write_indent(content: &mut String, depth: usize) {
    for _ in 0..depth {
        content.push_str("    ");
    }
}

fn to_ident(name: &str, uppercase: bool) -> String {
    let mut ident = String::new();

    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            if uppercase {
                ident.push(ch.to_ascii_uppercase());
            } else {
                ident.push(ch.to_ascii_lowercase());
            }
        } else {
            ident.push('_');
        }
    }

    if ident
        .as_bytes()
        .first()
        .is_some_and(|ch| ch.is_ascii_digit())
    {
        ident.insert(0, '_');
    }

    ident
}
