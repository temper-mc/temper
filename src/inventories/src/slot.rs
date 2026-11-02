use crate::components::{ItemComponent, decode_component_value};
use crate::item::ItemID;
use bon::Builder;
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt::Display;
use std::io::{Cursor, Read, Write};
use temper_codec::decode::errors::NetDecodeError;
use temper_codec::decode::{NetDecode, NetDecodeOpts};
use temper_codec::encode::errors::NetEncodeError;
use temper_codec::encode::{NetEncode, NetEncodeOpts};
use temper_codec::net_types::var_int::VarInt;
use type_hash::TypeHash;

fn unique_id_deser<'de, D: Deserializer<'de>>(_: D) -> Result<u64, D::Error> {
    Ok(rand::random())
}

/// Represents a single slot in an inventory, which can hold an item and its associated data.
///
/// Use the by_unique_id methods to fetch a slot by its unique_id to avoid TOC/TOU bugs regarding
/// moving items to different slots.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TypeHash, Builder)]
pub struct InventorySlot {
    pub count: VarInt,
    pub item_id: Option<ItemID>,
    #[type_hash(foreign_type)]
    pub components_to_add: Option<Vec<ItemComponent>>,
    pub components_to_remove: Option<Vec<VarInt>>,
    #[serde(skip_serializing)]
    #[serde(deserialize_with = "unique_id_deser")]
    #[builder(skip = rand::random())]
    unique_id: u64,
}

impl InventorySlot {
    pub fn empty() -> Self {
        Self {
            count: VarInt(0),
            item_id: None,
            components_to_add: None,
            components_to_remove: None,
            unique_id: rand::random(),
        }
    }

    /// Decodes the creative inventory slot format, where component values are length-prefixed.
    pub fn decode_creative_mode_slot<R: Read>(
        reader: &mut R,
        opts: &NetDecodeOpts,
    ) -> Result<Self, NetDecodeError> {
        let count = VarInt::decode(reader, opts)?;
        if count.0 <= 0 {
            Ok(Self {
                count,
                ..Default::default()
            })
        } else {
            let item_id = VarInt::decode(reader, opts)?;
            let (components_to_add, components_to_remove) =
                decode_data_component_patch(reader, opts, true)?;

            Ok(Self {
                count,
                item_id: Some(ItemID(item_id)),
                components_to_add: Some(components_to_add),
                components_to_remove: Some(components_to_remove),
                unique_id: rand::random(),
            })
        }
    }

    /// Decodes the non-optional item stack format used inside item components because it just has
    /// to be special. Mojang count ur fuckin days.
    pub fn decode_template<R: Read>(
        reader: &mut R,
        opts: &NetDecodeOpts,
    ) -> Result<Self, NetDecodeError> {
        let item_id = ItemID::decode(reader, opts)?;
        let count = VarInt::decode(reader, opts)?;
        let (components_to_add, components_to_remove) =
            decode_data_component_patch(reader, opts, false)?;

        if count.0 <= 0 {
            return Ok(Self {
                count: VarInt::new(0),
                ..Default::default()
            });
        }

        Ok(Self {
            count,
            item_id: Some(item_id),
            components_to_add: Some(components_to_add),
            components_to_remove: Some(components_to_remove),
            unique_id: rand::random(),
        })
    }

    /// Encodes the non-optional item stack format used inside item components. See above for
    /// threats to mojang
    pub fn encode_template<W: Write>(
        &self,
        writer: &mut W,
        opts: &NetEncodeOpts,
    ) -> Result<(), NetEncodeError> {
        match &self.item_id {
            Some(item_id) => item_id.encode(writer, opts)?,
            None => ItemID::new(0).encode(writer, opts)?,
        }

        self.count.encode(writer, opts)?;
        encode_data_component_patch(
            writer,
            opts,
            if self.count.0 <= 0 {
                None
            } else {
                self.components_to_add.as_deref()
            },
            if self.count.0 <= 0 {
                None
            } else {
                self.components_to_remove.as_deref()
            },
        )
    }

    pub fn get_unique_id(&self) -> u64 {
        self.unique_id
    }
}

impl Display for InventorySlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "InventorySlot {{ count: {}, item_id: {:?}, components_to_add: {:?}, components_to_remove: {:?} }}",
            self.count.0, self.item_id, self.components_to_add, self.components_to_remove,
        )
    }
}

impl NetDecode for InventorySlot {
    fn decode<R: Read>(reader: &mut R, opts: &NetDecodeOpts) -> Result<Self, NetDecodeError> {
        let count = VarInt::decode(reader, opts)?;
        if count.0 <= 0 {
            Ok(Self {
                count,
                ..Default::default()
            })
        } else {
            let item_id = VarInt::decode(reader, opts)?;
            let (components_to_add, components_to_remove) =
                decode_data_component_patch(reader, opts, false)?;

            Ok(Self {
                count,
                item_id: Some(ItemID(item_id)),
                components_to_add: Some(components_to_add),
                components_to_remove: Some(components_to_remove),
                unique_id: rand::random(),
            })
        }
    }
}

impl NetEncode for InventorySlot {
    fn encode<W: Write>(&self, writer: &mut W, opts: &NetEncodeOpts) -> Result<(), NetEncodeError> {
        // 1. Always encode the count
        self.count.encode(writer, opts)?;

        // If the slot is empty, stop immediately
        if self.count.0 <= 0 {
            return Ok(());
        }

        // 2. Encode ItemID
        match &self.item_id {
            Some(item_id) => item_id.0.encode(writer, opts)?,
            None => VarInt::new(0).encode(writer, opts)?,
        }

        // 3. Encode the data component patch.
        encode_data_component_patch(
            writer,
            opts,
            self.components_to_add.as_deref(),
            self.components_to_remove.as_deref(),
        )
    }
}

fn decode_data_component_patch<R: Read>(
    reader: &mut R,
    opts: &NetDecodeOpts,
    delimited_values: bool,
) -> Result<(Vec<ItemComponent>, Vec<VarInt>), NetDecodeError> {
    let components_to_add_count = VarInt::decode(reader, opts)?;
    if components_to_add_count.0 < 0 {
        return Err(NetDecodeError::ExternalError(
            format!(
                "negative data component add count: {}",
                components_to_add_count.0
            )
            .into(),
        ));
    }

    let components_to_remove_count = VarInt::decode(reader, opts)?;
    if components_to_remove_count.0 < 0 {
        return Err(NetDecodeError::ExternalError(
            format!(
                "negative data component remove count: {}",
                components_to_remove_count.0
            )
            .into(),
        ));
    }

    let mut components_to_add = Vec::with_capacity(components_to_add_count.0 as usize);
    for _ in 0..components_to_add_count.0 {
        let component = if delimited_values {
            decode_delimited_data_component(reader, opts)?
        } else {
            ItemComponent::decode(reader, opts)?
        };
        components_to_add.push(component);
    }

    let mut components_to_remove = Vec::with_capacity(components_to_remove_count.0 as usize);
    for _ in 0..components_to_remove_count.0 {
        components_to_remove.push(VarInt::decode(reader, opts)?);
    }

    Ok((components_to_add, components_to_remove))
}

fn decode_delimited_data_component<R: Read>(
    reader: &mut R,
    opts: &NetDecodeOpts,
) -> Result<ItemComponent, NetDecodeError> {
    let component_id = VarInt::decode(reader, opts)?;
    let length = VarInt::decode(reader, opts)?.0;
    if length < 0 {
        return Err(NetDecodeError::ExternalError(
            format!("negative data component value length: {length}").into(),
        ));
    }

    let mut buf = vec![0; length as usize];
    reader.read_exact(&mut buf)?;
    let mut cursor = Cursor::new(buf);
    let component = decode_component_value(component_id.0, &mut cursor)?;
    let decoded_len = cursor.position();
    let expected_len = cursor.get_ref().len() as u64;
    if decoded_len != expected_len {
        return Err(NetDecodeError::ExternalError(
            format!(
                "data component {} consumed {decoded_len} bytes from {expected_len} byte value",
                component_id.0
            )
            .into(),
        ));
    }

    Ok(component)
}

fn encode_data_component_patch<W: Write>(
    writer: &mut W,
    opts: &NetEncodeOpts,
    components_to_add: Option<&[ItemComponent]>,
    components_to_remove: Option<&[VarInt]>,
) -> Result<(), NetEncodeError> {
    let add_count = components_to_add.map_or(0, <[ItemComponent]>::len);
    let remove_count = components_to_remove.map_or(0, <[VarInt]>::len);

    VarInt::new(add_count as i32).encode(writer, opts)?;
    VarInt::new(remove_count as i32).encode(writer, opts)?;
    encode_data_component_patch_entries(writer, opts, components_to_add)?;
    encode_removed_data_component_entries(writer, opts, components_to_remove)
}

fn encode_data_component_patch_entries<W: Write>(
    writer: &mut W,
    opts: &NetEncodeOpts,
    components: Option<&[ItemComponent]>,
) -> Result<(), NetEncodeError> {
    if let Some(components) = components {
        for component in components {
            component.encode(writer, opts)?;
        }
    }

    Ok(())
}

fn encode_removed_data_component_entries<W: Write>(
    writer: &mut W,
    opts: &NetEncodeOpts,
    components: Option<&[VarInt]>,
) -> Result<(), NetEncodeError> {
    if let Some(components) = components {
        for component in components {
            component.encode(writer, opts)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use temper_codec::encode::NetEncodeOpts;
    use temper_codec::net_types::var_int::VarInt;

    // This helper function runs the encode/decode cycle
    fn run_roundtrip_test(slot_in: &InventorySlot) -> InventorySlot {
        let mut buffer = Vec::new();

        // Create both types of options explicitly.
        let encode_opts = NetEncodeOpts::default();
        let decode_opts = NetDecodeOpts::default();

        // 1. Encode
        slot_in
            .encode(&mut buffer, &encode_opts)
            .expect("Encode failed");

        // 2. Decode
        let mut reader = Cursor::new(&buffer);
        let slot_out = InventorySlot::decode(&mut reader, &decode_opts).expect("Decode failed");

        // 3. Check that all bytes were read
        assert_eq!(
            reader.position() as usize,
            buffer.len(),
            "Decoder did not read the entire buffer"
        );

        slot_out
    }

    #[test]
    fn test_slot_encode_decode_roundtrip() {
        // --- Test Case 1: The Empty Slot ---

        let simple_slot = InventorySlot {
            count: VarInt::new(10),
            item_id: Some(ItemID::new(1)),
            components_to_add: Some(vec![]),
            components_to_remove: Some(vec![]),
            unique_id: 0,
        };

        let mut decoded_simple = run_roundtrip_test(&simple_slot);
        decoded_simple.unique_id = 0; // Reset unique_id for comparison
        assert_eq!(simple_slot, decoded_simple, "Simple slot roundtrip failed");

        // --- Test Case 2: The Full NBT/Component Slot ---
        let complex_slot = InventorySlot {
            count: VarInt::new(1),
            item_id: Some(ItemID::new(872)),
            components_to_add: Some(vec![
                ItemComponent::MaxStackSize(VarInt::new(10)),
                ItemComponent::MaxDamage(VarInt::new(11)),
            ]),
            components_to_remove: Some(vec![VarInt::new(20), VarInt::new(21)]),
            unique_id: 0,
        };
        let mut decoded_complex = run_roundtrip_test(&complex_slot);
        decoded_complex.unique_id = 0; // Reset unique_id for comparison
        assert_eq!(
            complex_slot, decoded_complex,
            "Complex slot roundtrip failed"
        );
    }
}
