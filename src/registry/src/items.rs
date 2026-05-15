use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawItemData<'a> {
    id: u32,
    name: &'a str,
    stack_size: u8,
    max_durability: Option<u16>,
}

struct ItemRegistry {
    names: Vec<String>,
    stack_size: Vec<u8>,
    // For max_durability, 0 will mean "unbreakable" or "does not use durability"
    max_durability: Vec<u16>,
}
