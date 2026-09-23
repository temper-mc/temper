use crate::de::converter::FromNbt;
use crate::{NBTError, NBTSerializable, NBTSerializeOptions};
use simdnbt::borrow::{
    BaseNbt as SimdBaseNbt, Nbt as SimdNbt, NbtCompound, NbtCompoundList, NbtList, NbtListList,
    NbtTag as SimdNbtTag,
};
use simdnbt::Mutf8Str;
use std::io::{Cursor, Write};
use temper_codec::encode::errors::NetEncodeError;
use temper_codec::encode::{NetEncode, NetEncodeOpts};

#[repr(u8)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum NbtTag {
    End = 0,
    Byte = 1,
    Short = 2,
    Int = 3,
    Long = 4,
    Float = 5,
    Double = 6,
    ByteArray = 7,
    String = 8,
    List = 9,
    Compound = 10,
    IntArray = 11,
    LongArray = 12,
}

impl NbtTag {
    pub const fn from_byte(tag: u8) -> Option<Self> {
        match tag {
            0 => Some(NbtTag::End),
            1 => Some(NbtTag::Byte),
            2 => Some(NbtTag::Short),
            3 => Some(NbtTag::Int),
            4 => Some(NbtTag::Long),
            5 => Some(NbtTag::Float),
            6 => Some(NbtTag::Double),
            7 => Some(NbtTag::ByteArray),
            8 => Some(NbtTag::String),
            9 => Some(NbtTag::List),
            10 => Some(NbtTag::Compound),
            11 => Some(NbtTag::IntArray),
            12 => Some(NbtTag::LongArray),
            _ => None,
        }
    }
}

impl From<u8> for NbtTag {
    fn from(tag: u8) -> Self {
        Self::from_byte(tag).unwrap_or_else(|| panic!("Invalid NbtTag: {tag}"))
    }
}

#[derive(Clone, Debug)]
pub enum NbtTapeElement<'a, 'tape>
where
    'a: 'tape,
{
    End,
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(&'a [u8]),
    String(&'a Mutf8Str),
    List(NbtList<'a, 'tape>),
    Compound(NbtCompound<'a, 'tape>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

impl NbtTapeElement<'_, '_> {
    pub const fn nbt_type(&self) -> &'static str {
        match self {
            NbtTapeElement::End => "End",
            NbtTapeElement::Byte(_) => "Byte",
            NbtTapeElement::Short(_) => "Short",
            NbtTapeElement::Int(_) => "Int",
            NbtTapeElement::Long(_) => "Long",
            NbtTapeElement::Float(_) => "Float",
            NbtTapeElement::Double(_) => "Double",
            NbtTapeElement::ByteArray(_) => "ByteArray",
            NbtTapeElement::String(_) => "String",
            NbtTapeElement::List(_) => "List",
            NbtTapeElement::Compound(_) => "Compound",
            NbtTapeElement::IntArray(_) => "IntArray",
            NbtTapeElement::LongArray(_) => "LongArray",
        }
    }

    pub const fn nbt_id(&self) -> u8 {
        match self {
            NbtTapeElement::End => NbtTag::End as u8,
            NbtTapeElement::Byte(_) => NbtTag::Byte as u8,
            NbtTapeElement::Short(_) => NbtTag::Short as u8,
            NbtTapeElement::Int(_) => NbtTag::Int as u8,
            NbtTapeElement::Long(_) => NbtTag::Long as u8,
            NbtTapeElement::Float(_) => NbtTag::Float as u8,
            NbtTapeElement::Double(_) => NbtTag::Double as u8,
            NbtTapeElement::ByteArray(_) => NbtTag::ByteArray as u8,
            NbtTapeElement::String(_) => NbtTag::String as u8,
            NbtTapeElement::List(_) => NbtTag::List as u8,
            NbtTapeElement::Compound(_) => NbtTag::Compound as u8,
            NbtTapeElement::IntArray(_) => NbtTag::IntArray as u8,
            NbtTapeElement::LongArray(_) => NbtTag::LongArray as u8,
        }
    }
}

pub struct NbtTape<'a> {
    data: &'a [u8],
    root: Option<SimdNbt<'a>>,
}

impl<'a, 'tape> NbtTapeElement<'a, 'tape>
where
    'a: 'tape,
{
    pub fn get(&self, key: &str) -> Option<NbtTapeElement<'a, 'tape>> {
        match self {
            NbtTapeElement::Compound(compound) => compound.get(key).and_then(convert_tag),
            _ => None,
        }
    }

    pub fn take(&mut self, key: &str) -> Option<NbtTapeElement<'a, 'tape>> {
        self.get(key)
    }

    pub fn as_compound(&self) -> Option<NbtCompound<'a, 'tape>> {
        match self {
            NbtTapeElement::Compound(compound) => Some(*compound),
            _ => None,
        }
    }

    pub fn as_list<T: FromNbt<'a>>(self, tape: &'tape NbtTape<'a>) -> Option<Vec<T>> {
        tape.unpack_list(self)
    }
}

impl<'a> NbtTape<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, root: None }
    }

    pub fn parse(&mut self) {
        self.parse_result().expect("failed to parse NBT root");
    }

    pub fn parse_network_root(&mut self) -> crate::Result<()> {
        let nbt = simdnbt::borrow::read_unnamed(&mut Cursor::new(self.data))
            .map_err(|_| NBTError::InvalidNBTData)?;
        self.root = Some(nbt);
        Ok(())
    }

    fn parse_result(&mut self) -> crate::Result<()> {
        let nbt = simdnbt::borrow::read(&mut Cursor::new(self.data))
            .map_err(|_| NBTError::InvalidNBTData)?;
        self.root = Some(nbt);
        Ok(())
    }

    pub fn take_root<'tape>(&'tape self) -> crate::Result<NbtTapeElement<'a, 'tape>> {
        self.root_element()
    }

    pub fn root_element<'tape>(&'tape self) -> crate::Result<NbtTapeElement<'a, 'tape>> {
        let SimdNbt::Some(root) = self.root.as_ref().ok_or(NBTError::NoRootTag)? else {
            return Err(NBTError::NoRootTag);
        };

        Ok(NbtTapeElement::Compound(root_compound(root)))
    }

    pub fn get<'tape>(&'tape self, key: &str) -> Option<NbtTapeElement<'a, 'tape>> {
        self.root_element().ok().and_then(|element| element.get(key))
    }

    pub fn unpack_list<'tape, T: FromNbt<'a>>(
        &'tape self,
        element: NbtTapeElement<'a, 'tape>,
    ) -> Option<Vec<T>>
    where
        'a: 'tape,
    {
        match element {
            NbtTapeElement::List(list) => unpack_simd_list(self, list),
            NbtTapeElement::ByteArray(data) => data
                .iter()
                .copied()
                .map(|value| T::from_nbt(self, NbtTapeElement::Byte(value as i8)).ok())
                .collect(),
            NbtTapeElement::IntArray(data) => data
                .into_iter()
                .map(|value| T::from_nbt(self, NbtTapeElement::Int(value)).ok())
                .collect(),
            NbtTapeElement::LongArray(data) => data
                .into_iter()
                .map(|value| T::from_nbt(self, NbtTapeElement::Long(value)).ok())
                .collect(),
            _ => None,
        }
    }

    pub fn unpack_list_sliced<T: NbtDeserializable<'a>>(
        &self,
        _element: &NbtTapeElement<'a, '_>,
    ) -> Option<&'a [T]> {
        None
    }
}

fn root_compound<'a, 'tape>(root: &'tape SimdBaseNbt<'a>) -> NbtCompound<'a, 'tape>
where
    'a: 'tape,
{
    // simdnbt's borrowed root exposes `as_compound` through `&'a self`, but the
    // returned compound is tied to the tape borrow through its internal
    // `extra_tapes` reference. Keep the lifetime widening boxed in here so the
    // public wrapper still only hands out elements that live as long as `self`.
    let root: &'a SimdBaseNbt<'a> = unsafe { &*(root as *const SimdBaseNbt<'a>) };
    root.as_compound()
}

pub(crate) fn convert_tag<'a, 'tape>(tag: SimdNbtTag<'a, 'tape>) -> Option<NbtTapeElement<'a, 'tape>>
where
    'a: 'tape,
{
    Some(if let Some(value) = tag.byte() {
        NbtTapeElement::Byte(value)
    } else if let Some(value) = tag.short() {
        NbtTapeElement::Short(value)
    } else if let Some(value) = tag.int() {
        NbtTapeElement::Int(value)
    } else if let Some(value) = tag.long() {
        NbtTapeElement::Long(value)
    } else if let Some(value) = tag.float() {
        NbtTapeElement::Float(value)
    } else if let Some(value) = tag.double() {
        NbtTapeElement::Double(value)
    } else if let Some(values) = tag.byte_array() {
        NbtTapeElement::ByteArray(values)
    } else if let Some(value) = tag.string() {
        NbtTapeElement::String(value)
    } else if let Some(list) = tag.list() {
        NbtTapeElement::List(list)
    } else if let Some(compound) = tag.compound() {
        NbtTapeElement::Compound(compound)
    } else if let Some(values) = tag.int_array() {
        NbtTapeElement::IntArray(values)
    } else if let Some(values) = tag.long_array() {
        NbtTapeElement::LongArray(values)
    } else {
        return None;
    })
}

fn unpack_simd_list<'a, 'tape, T: FromNbt<'a>>(
    tape: &'tape NbtTape<'a>,
    list: NbtList<'a, 'tape>,
) -> Option<Vec<T>>
where
    'a: 'tape,
{
    if list.empty() {
        return Some(Vec::new());
    }

    if let Some(values) = list.bytes() {
        return values
            .iter()
            .copied()
            .map(|value| T::from_nbt(tape, NbtTapeElement::Byte(value)).ok())
            .collect();
    }

    if let Some(values) = list.shorts() {
        return values
            .into_iter()
            .map(|value| T::from_nbt(tape, NbtTapeElement::Short(value)).ok())
            .collect();
    }

    if let Some(values) = list.ints() {
        return values
            .into_iter()
            .map(|value| T::from_nbt(tape, NbtTapeElement::Int(value)).ok())
            .collect();
    }

    if let Some(values) = list.longs() {
        return values
            .into_iter()
            .map(|value| T::from_nbt(tape, NbtTapeElement::Long(value)).ok())
            .collect();
    }

    if let Some(values) = list.floats() {
        return values
            .into_iter()
            .map(|value| T::from_nbt(tape, NbtTapeElement::Float(value)).ok())
            .collect();
    }

    if let Some(values) = list.doubles() {
        return values
            .into_iter()
            .map(|value| T::from_nbt(tape, NbtTapeElement::Double(value)).ok())
            .collect();
    }

    if let Some(values) = list.byte_arrays() {
        return values
            .iter()
            .copied()
            .map(|value| T::from_nbt(tape, NbtTapeElement::ByteArray(value)).ok())
            .collect();
    }

    if let Some(values) = list.strings() {
        return values
            .iter()
            .copied()
            .map(|value| T::from_nbt(tape, NbtTapeElement::String(value)).ok())
            .collect();
    }

    if let Some(values) = list.lists() {
        return collect_list_list(tape, values);
    }

    if let Some(values) = list.compounds() {
        return collect_compound_list(tape, values);
    }

    if let Some(values) = list.int_arrays() {
        return values
            .iter()
            .copied()
            .map(|value| T::from_nbt(tape, NbtTapeElement::IntArray(value.to_vec())).ok())
            .collect();
    }

    if let Some(values) = list.long_arrays() {
        return values
            .iter()
            .copied()
            .map(|value| T::from_nbt(tape, NbtTapeElement::LongArray(value.to_vec())).ok())
            .collect();
    }

    None
}

fn collect_list_list<'a, 'tape, T: FromNbt<'a>>(
    tape: &'tape NbtTape<'a>,
    values: NbtListList<'a, 'tape>,
) -> Option<Vec<T>>
where
    'a: 'tape,
{
    values
        .into_iter()
        .map(|value| T::from_nbt(tape, NbtTapeElement::List(value)).ok())
        .collect()
}

fn collect_compound_list<'a, 'tape, T: FromNbt<'a>>(
    tape: &'tape NbtTape<'a>,
    values: NbtCompoundList<'a, 'tape>,
) -> Option<Vec<T>>
where
    'a: 'tape,
{
    values
        .into_iter()
        .map(|value| T::from_nbt(tape, NbtTapeElement::Compound(value)).ok())
        .collect()
}

pub enum NbtDeserializableOptions {
    None,
    TagType(NbtTag),
}

pub trait NbtDeserializable<'a>: Sized {
    fn parse_from_bytes(_data: &'a [u8]) -> Self;

    fn parse_from_nbt(_tape: &mut NbtTape<'a>, _opts: NbtDeserializableOptions) -> Self {
        panic!("raw NBT tape parsing is no longer exposed")
    }
}

mod primitives {
    use super::NbtDeserializable;

    impl NbtDeserializable<'_> for i8 {
        fn parse_from_bytes(data: &[u8]) -> Self {
            data[0] as i8
        }
    }

    impl NbtDeserializable<'_> for u8 {
        fn parse_from_bytes(data: &[u8]) -> Self {
            data[0]
        }
    }

    impl NbtDeserializable<'_> for i16 {
        fn parse_from_bytes(data: &[u8]) -> Self {
            i16::from_be_bytes(data.try_into().expect("invalid i16 byte length"))
        }
    }

    impl NbtDeserializable<'_> for u16 {
        fn parse_from_bytes(data: &[u8]) -> Self {
            u16::from_be_bytes(data.try_into().expect("invalid u16 byte length"))
        }
    }

    impl NbtDeserializable<'_> for i32 {
        fn parse_from_bytes(data: &[u8]) -> Self {
            i32::from_be_bytes(data.try_into().expect("invalid i32 byte length"))
        }
    }

    impl NbtDeserializable<'_> for u32 {
        fn parse_from_bytes(data: &[u8]) -> Self {
            u32::from_be_bytes(data.try_into().expect("invalid u32 byte length"))
        }
    }

    impl NbtDeserializable<'_> for i64 {
        fn parse_from_bytes(data: &[u8]) -> Self {
            i64::from_be_bytes(data.try_into().expect("invalid i64 byte length"))
        }
    }

    impl NbtDeserializable<'_> for u64 {
        fn parse_from_bytes(data: &[u8]) -> Self {
            u64::from_be_bytes(data.try_into().expect("invalid u64 byte length"))
        }
    }

    impl NbtDeserializable<'_> for f32 {
        fn parse_from_bytes(data: &[u8]) -> Self {
            f32::from_be_bytes(data.try_into().expect("invalid f32 byte length"))
        }
    }

    impl NbtDeserializable<'_> for f64 {
        fn parse_from_bytes(data: &[u8]) -> Self {
            f64::from_be_bytes(data.try_into().expect("invalid f64 byte length"))
        }
    }

    impl NbtDeserializable<'_> for bool {
        fn parse_from_bytes(data: &[u8]) -> Self {
            data[0] != 0
        }
    }
}

impl NetEncode for NbtTape<'_> {
    fn encode<W: Write>(
        &self,
        writer: &mut W,
        _opts: &NetEncodeOpts,
    ) -> Result<(), NetEncodeError> {
        writer.write_all(self.data)?;
        Ok(())
    }
}

impl NbtTapeElement<'_, '_> {
    pub fn serialize_as_network(
        &self,
        _tape: &mut NbtTape,
        writer: &mut Vec<u8>,
        opts: &NBTSerializeOptions,
    ) -> Result<(), NetEncodeError> {
        match opts {
            NBTSerializeOptions::None => {}
            NBTSerializeOptions::WithHeader(name) => {
                writer.write_all(&[self.nbt_id()])?;
                name.serialize(writer, &NBTSerializeOptions::None);
            }
            NBTSerializeOptions::Network | NBTSerializeOptions::Flatten => {
                writer.write_all(&[self.nbt_id()])?;
            }
        }

        write_payload(self, writer)
    }
}

fn write_payload(
    element: &NbtTapeElement<'_, '_>,
    writer: &mut Vec<u8>,
) -> Result<(), NetEncodeError> {
    match element {
        NbtTapeElement::End => {}
        NbtTapeElement::Byte(value) => writer.write_all(&[*value as u8])?,
        NbtTapeElement::Short(value) => writer.write_all(&value.to_be_bytes())?,
        NbtTapeElement::Int(value) => writer.write_all(&value.to_be_bytes())?,
        NbtTapeElement::Long(value) => writer.write_all(&value.to_be_bytes())?,
        NbtTapeElement::Float(value) => writer.write_all(&value.to_be_bytes())?,
        NbtTapeElement::Double(value) => writer.write_all(&value.to_be_bytes())?,
        NbtTapeElement::ByteArray(values) => {
            (values.len() as i32).serialize(writer, &NBTSerializeOptions::None);
            writer.write_all(values)?;
        }
        NbtTapeElement::String(value) => {
            value.to_str()
                .as_ref()
                .serialize(writer, &NBTSerializeOptions::None);
        }
        NbtTapeElement::List(list) => {
            list.write(writer);
        }
        NbtTapeElement::Compound(elements) => {
            elements.write(writer);
        }
        NbtTapeElement::IntArray(values) => {
            (values.len() as i32).serialize(writer, &NBTSerializeOptions::None);
            for value in values {
                writer.write_all(&value.to_be_bytes())?;
            }
        }
        NbtTapeElement::LongArray(values) => {
            (values.len() as i32).serialize(writer, &NBTSerializeOptions::None);
            for value in values {
                writer.write_all(&value.to_be_bytes())?;
            }
        }
    }

    Ok(())
}
