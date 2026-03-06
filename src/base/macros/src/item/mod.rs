use quote::quote;
use simd_json::prelude::*;

const REGISTRY_FILE: &[u8] = include_bytes!("../../../../../assets/data/registries.json");

pub(super) fn item(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut buf = REGISTRY_FILE.to_vec();

    let parsed = simd_json::to_owned_value(&mut buf).unwrap();

    let target_name = syn::parse_macro_input!(input as syn::LitStr).value();

    let prefixed_name = if target_name.starts_with("minecraft:") {
        target_name.clone()
    } else {
        format!("minecraft:{}", target_name)
    };

    let id = parsed
        .get("minecraft:item")
        .expect("Failed to get 'minecraft:item' from registries.json")
        .get("entries")
        .expect("Failed to get 'entries' from 'minecraft:item' in registries.json")
        .get(&prefixed_name)
        .unwrap_or_else(|| {
            panic!(
                "Failed to find item 'minecraft:{}' in registries.json",
                prefixed_name
            )
        })
        .get_i32("protocol_id")
        .unwrap_or_else(|| {
            panic!(
                "Failed to get 'protocol_id' for item 'minecraft:{}' in registries.json",
                prefixed_name
            )
        });

    quote! {
        temper_inventories::item::ItemID(temper_codec::net_types::var_int::VarInt(#id))
    }
    .into()
}
