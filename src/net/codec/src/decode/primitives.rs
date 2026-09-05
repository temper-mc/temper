use crate::decode::errors::NetDecodeError;
use crate::decode::{NetDecode, NetDecodeOpts};
use crate::net_types::var_int::VarInt;
use std::collections::HashMap;
use std::hash::Hash;
use std::io::Read;

macro_rules! impl_for_primitives {
    ($($primitive_type:ty $(| $alt:ty)?),*) => {
        $(
            impl NetDecode for $primitive_type {
                fn decode<R: Read>(reader: &mut R, _: &NetDecodeOpts) -> Result<Self, NetDecodeError> {
                    let mut buf = [0; std::mem::size_of::<Self>()];
                    reader.read_exact(&mut buf)?;
                    Ok(Self::from_be_bytes(buf))
                }
            }

            $(
                impl NetDecode for $alt {
                    fn decode<R: Read>(reader: &mut R, opts: &NetDecodeOpts) -> Result<Self, NetDecodeError> {
                        // Basically use the decode method of the primitive type,
                        // and then convert it to the alternative type.
                        <$primitive_type as NetDecode>::decode(reader, opts)
                        .map(|x| x as Self)
                    }
                }
            )?
        )*
    };
}

impl_for_primitives!(
    u8 | i8,
    u16 | i16,
    u32 | i32,
    u64 | i64,
    u128 | i128,
    usize | isize,
    f32,
    f64
);

macro_rules! impl_for_tuples {
    ($(($($name:ident),+)),+ $(,)?) => {
        $(
            impl<$($name),+> NetDecode for ($($name,)+)
            where
                $($name: NetDecode),+
            {
                fn decode<R: Read>(reader: &mut R, opts: &NetDecodeOpts) -> Result<Self, NetDecodeError> {
                    Ok((
                        $(
                            <$name as NetDecode>::decode(reader, opts)?,
                        )+
                    ))
                }
            }
        )+
    };
}

impl_for_tuples!(
    (A),
    (A, B),
    (A, B, C),
    (A, B, C, D),
    (A, B, C, D, E),
    (A, B, C, D, E, F),
    (A, B, C, D, E, F, G),
    (A, B, C, D, E, F, G, H),
    (A, B, C, D, E, F, G, H, I),
    (A, B, C, D, E, F, G, H, I, J),
    (A, B, C, D, E, F, G, H, I, J, K),
    (A, B, C, D, E, F, G, H, I, J, K, L),
);

impl NetDecode for bool {
    fn decode<R: Read>(reader: &mut R, _: &NetDecodeOpts) -> Result<Self, NetDecodeError> {
        Ok(<u8 as NetDecode>::decode(reader, &NetDecodeOpts::None)? != 0)
    }
}

impl NetDecode for String {
    fn decode<R: Read>(reader: &mut R, _: &NetDecodeOpts) -> Result<Self, NetDecodeError> {
        let len = <VarInt as NetDecode>::decode(reader, &NetDecodeOpts::None)?.0 as usize;
        let mut buf = vec![0; len];
        reader.read_exact(&mut buf)?;
        Ok(String::from_utf8(buf)?)
    }
}

impl<T> NetDecode for Vec<T>
where
    T: NetDecode,
{
    fn decode<R: Read>(reader: &mut R, opts: &NetDecodeOpts) -> Result<Self, NetDecodeError> {
        if matches!(opts, NetDecodeOpts::IsSizePrefixed) {
            let len = <VarInt as NetDecode>::decode(reader, opts)?.0 as usize;
            let mut vec = Vec::with_capacity(len);
            for _ in 0..len {
                vec.push(T::decode(reader, opts)?);
            }
            return Ok(vec);
        }

        // read to end
        let mut data = Vec::new();
        R::read_to_end(reader, &mut data)?;

        let mut cursor = std::io::Cursor::new(data);

        let mut vec = Vec::new();
        while cursor.position() < cursor.get_ref().len() as u64 {
            vec.push(T::decode(&mut cursor, opts)?);
        }

        Ok(vec)
    }
}

/// This isn't actually a type in the Minecraft Protocol. This is just for saving data/ or for general use.
/// It was created for saving/reading chunks!
impl<K, V> NetDecode for HashMap<K, V>
where
    K: NetDecode + Eq + Hash,
    V: NetDecode,
{
    fn decode<R: Read>(reader: &mut R, opts: &NetDecodeOpts) -> Result<Self, NetDecodeError> {
        let len = <VarInt as NetDecode>::decode(reader, opts)?.0 as usize;
        let mut map = HashMap::with_capacity(len);
        for _ in 0..len {
            let key = K::decode(reader, opts)?;
            let value = V::decode(reader, opts)?;
            map.insert(key, value);
        }
        Ok(map)
    }
}

impl<const N: usize> NetDecode for [u8; N] {
    fn decode<R: Read>(reader: &mut R, _opts: &NetDecodeOpts) -> Result<Self, NetDecodeError> {
        let mut buf = [0; N];
        reader.read_exact(&mut buf)?;
        Ok(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encode::{NetEncode, NetEncodeOpts};
    use crate::net_types::prefixed_optional::PrefixedOptional;
    use std::io::Cursor;

    #[test]
    fn tuple_values_encode_and_decode_in_order() {
        let value = (1u8, -2i16, 3.5f64);
        let mut encoded = Vec::new();
        value.encode(&mut encoded, &NetEncodeOpts::None).unwrap();

        let mut expected = vec![1];
        expected.extend_from_slice(&(-2i16).to_be_bytes());
        expected.extend_from_slice(&3.5f64.to_be_bytes());
        assert_eq!(encoded, expected);

        let mut cursor = Cursor::new(encoded);
        let decoded =
            <(u8, i16, f64) as NetDecode>::decode(&mut cursor, &NetDecodeOpts::None).unwrap();

        assert_eq!(decoded, value);
    }

    #[test]
    fn prefixed_optional_tuple_round_trips() {
        let value = PrefixedOptional::Some((1.25f64, 2.5f64, 3.75f64));
        let mut encoded = Vec::new();
        value.encode(&mut encoded, &NetEncodeOpts::None).unwrap();

        let mut cursor = Cursor::new(encoded);
        let decoded =
            PrefixedOptional::<(f64, f64, f64)>::decode(&mut cursor, &NetDecodeOpts::None).unwrap();

        assert_eq!(decoded, value);
    }
}
