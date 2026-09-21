use crate::de::converter::FromNbt;
use crate::{NBTError, NBTSerializable, NBTSerializeOptions};
use simdnbt::owned::{Nbt, NbtCompound, NbtList, NbtTag as SimdNbtTag};
use std::io::{Cursor, Write};
use std::marker::PhantomData;
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
pub enum NbtTapeElement<'a> {
    End,
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<u8>),
    String(String),
    List(NbtList),
    Compound(NbtCompound),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
    #[doc(hidden)]
    __Lifetime(PhantomData<&'a ()>),
}

impl NbtTapeElement<'_> {
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
            NbtTapeElement::__Lifetime(_) => "Internal",
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
            NbtTapeElement::__Lifetime(_) => NbtTag::End as u8,
        }
    }
}

pub struct NbtTape<'a> {
    data: &'a [u8],
    pub root: Option<(String, NbtTapeElement<'a>)>,
}

impl<'a> NbtTapeElement<'a> {
    pub fn get(&self, key: &str) -> Option<NbtTapeElement<'a>> {
        match self {
            NbtTapeElement::Compound(compound) => compound.get(key).cloned().map(convert_tag),
            _ => None,
        }
    }

    pub fn take(&mut self, key: &str) -> Option<NbtTapeElement<'a>> {
        match self {
            NbtTapeElement::Compound(compound) => compound.take(key).map(convert_tag),
            _ => None,
        }
    }

    pub fn as_compound(&self) -> Option<&NbtCompound> {
        match self {
            NbtTapeElement::Compound(compound) => Some(compound),
            _ => None,
        }
    }

    pub fn as_list<T: FromNbt<'a>>(self, tape: &NbtTape<'a>) -> Option<Vec<T>> {
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
        let nbt = simdnbt::owned::read_unnamed(&mut Cursor::new(self.data))
            .map_err(|_| NBTError::InvalidNBTData)?;
        self.root = Some(convert_root(nbt)?);
        Ok(())
    }

    fn parse_result(&mut self) -> crate::Result<()> {
        let nbt = simdnbt::owned::read(&mut Cursor::new(self.data))
            .map_err(|_| NBTError::InvalidNBTData)?;
        self.root = Some(convert_root(nbt)?);
        Ok(())
    }

    pub fn take_root(&mut self) -> crate::Result<NbtTapeElement<'a>> {
        self.root
            .take()
            .map(|(_, element)| element)
            .ok_or(NBTError::NoRootTag)
    }

    pub fn get(&self, key: &str) -> Option<NbtTapeElement<'a>> {
        self.root.as_ref().and_then(|(_, element)| element.get(key))
    }

    pub fn unpack_list<T: FromNbt<'a>>(&self, element: NbtTapeElement<'a>) -> Option<Vec<T>> {
        match element {
            NbtTapeElement::List(list) => unpack_simd_list(self, list),
            NbtTapeElement::ByteArray(data) => data
                .into_iter()
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
        _element: &NbtTapeElement<'a>,
    ) -> Option<&'a [T]> {
        None
    }
}

fn convert_root<'a>(nbt: Nbt) -> crate::Result<(String, NbtTapeElement<'a>)> {
    let Nbt::Some(root) = nbt else {
        return Err(NBTError::NoRootTag);
    };

    let name = root.name().to_str().into_owned();
    Ok((name, NbtTapeElement::Compound(root.as_compound())))
}

pub(crate) fn convert_tag<'a>(tag: SimdNbtTag) -> NbtTapeElement<'a> {
    match tag {
        SimdNbtTag::Byte(value) => NbtTapeElement::Byte(value),
        SimdNbtTag::Short(value) => NbtTapeElement::Short(value),
        SimdNbtTag::Int(value) => NbtTapeElement::Int(value),
        SimdNbtTag::Long(value) => NbtTapeElement::Long(value),
        SimdNbtTag::Float(value) => NbtTapeElement::Float(value),
        SimdNbtTag::Double(value) => NbtTapeElement::Double(value),
        SimdNbtTag::ByteArray(values) => NbtTapeElement::ByteArray(values),
        SimdNbtTag::String(value) => NbtTapeElement::String(value.into_string()),
        SimdNbtTag::List(list) => NbtTapeElement::List(list),
        SimdNbtTag::Compound(compound) => NbtTapeElement::Compound(compound),
        SimdNbtTag::IntArray(values) => NbtTapeElement::IntArray(values),
        SimdNbtTag::LongArray(values) => NbtTapeElement::LongArray(values),
    }
}

fn unpack_simd_list<'a, T: FromNbt<'a>>(tape: &NbtTape<'a>, list: NbtList) -> Option<Vec<T>> {
    match list {
        NbtList::Empty => Some(Vec::new()),
        NbtList::Byte(values) => values
            .into_iter()
            .map(|value| T::from_nbt(tape, NbtTapeElement::Byte(value)).ok())
            .collect(),
        NbtList::Short(values) => values
            .into_iter()
            .map(|value| T::from_nbt(tape, NbtTapeElement::Short(value)).ok())
            .collect(),
        NbtList::Int(values) => values
            .into_iter()
            .map(|value| T::from_nbt(tape, NbtTapeElement::Int(value)).ok())
            .collect(),
        NbtList::Long(values) => values
            .into_iter()
            .map(|value| T::from_nbt(tape, NbtTapeElement::Long(value)).ok())
            .collect(),
        NbtList::Float(values) => values
            .into_iter()
            .map(|value| T::from_nbt(tape, NbtTapeElement::Float(value)).ok())
            .collect(),
        NbtList::Double(values) => values
            .into_iter()
            .map(|value| T::from_nbt(tape, NbtTapeElement::Double(value)).ok())
            .collect(),
        NbtList::ByteArray(values) => values
            .into_iter()
            .map(|value| T::from_nbt(tape, NbtTapeElement::ByteArray(value)).ok())
            .collect(),
        NbtList::String(values) => values
            .into_iter()
            .map(|value| T::from_nbt(tape, NbtTapeElement::String(value.into_string())).ok())
            .collect(),
        NbtList::List(values) => values
            .into_iter()
            .map(|value| T::from_nbt(tape, NbtTapeElement::List(value)).ok())
            .collect(),
        NbtList::Compound(values) => values
            .into_iter()
            .map(|value| T::from_nbt(tape, NbtTapeElement::Compound(value)).ok())
            .collect(),
        NbtList::IntArray(values) => values
            .into_iter()
            .map(|value| T::from_nbt(tape, NbtTapeElement::IntArray(value)).ok())
            .collect(),
        NbtList::LongArray(values) => values
            .into_iter()
            .map(|value| T::from_nbt(tape, NbtTapeElement::LongArray(value)).ok())
            .collect(),
    }
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

impl NbtTapeElement<'_> {
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

fn write_payload(element: &NbtTapeElement<'_>, writer: &mut Vec<u8>) -> Result<(), NetEncodeError> {
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
            value.serialize(writer, &NBTSerializeOptions::None);
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
        NbtTapeElement::__Lifetime(_) => {}
    }

    Ok(())
}
