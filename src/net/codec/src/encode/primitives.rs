use crate::encode::errors::NetEncodeError;
use crate::encode::{NetEncode, NetEncodeOpts};
use crate::net_types::var_int::VarInt;
use std::borrow::Cow;
use std::collections::HashMap;
use std::io::Write;
use std::ops::Deref;

macro_rules! impl_for_primitives {
    ($($primitive_type:ty $(| $alt:ty)?),*) => {
        $(
            impl NetEncode for $primitive_type {
                fn encode<W: Write>(&self, writer: &mut W, _: &NetEncodeOpts) -> Result<(), NetEncodeError> {
                    writer.write_all(&self.to_be_bytes())?;
                    Ok(())
                }
            }

            $(
                impl NetEncode for $alt {
                    fn encode<W: Write>(&self, writer: &mut W, opts: &NetEncodeOpts) -> Result<(), NetEncodeError> {
                        (*self as $primitive_type).encode(writer, opts)
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

impl NetEncode for bool {
    fn encode<W: Write>(&self, writer: &mut W, _: &NetEncodeOpts) -> Result<(), NetEncodeError> {
        u8::from(*self).encode(writer, &NetEncodeOpts::None)
    }
}

impl NetEncode for String {
    fn encode<W: Write>(&self, writer: &mut W, _: &NetEncodeOpts) -> Result<(), NetEncodeError> {
        self.as_str().encode(writer, &NetEncodeOpts::None)
    }
}

impl NetEncode for &str {
    fn encode<W: Write>(&self, writer: &mut W, _: &NetEncodeOpts) -> Result<(), NetEncodeError> {
        let len = VarInt::new(self.len() as i32);
        len.encode(writer, &NetEncodeOpts::None)?;
        writer.write_all(self.as_bytes())?;
        Ok(())
    }
}

impl<T> NetEncode for Vec<T>
where
    T: NetEncode,
{
    fn encode<W: Write>(&self, writer: &mut W, opts: &NetEncodeOpts) -> Result<(), NetEncodeError> {
        if matches!(opts, NetEncodeOpts::SizePrefixed) {
            let len = VarInt::new(self.len() as i32);
            len.encode(writer, opts)?;
        }

        for item in self {
            item.encode(writer, opts)?;
        }
        Ok(())
    }
}

impl NetEncode for &[u8] {
    fn encode<W: Write>(&self, writer: &mut W, opts: &NetEncodeOpts) -> Result<(), NetEncodeError> {
        if matches!(opts, NetEncodeOpts::SizePrefixed) {
            let len = VarInt::new(self.len() as i32);
            len.encode(writer, opts)?;
        }

        writer.write_all(self)?;

        Ok(())
    }
}

impl NetEncode for [u8] {
    fn encode<W: Write>(&self, writer: &mut W, opts: &NetEncodeOpts) -> Result<(), NetEncodeError> {
        if matches!(opts, NetEncodeOpts::SizePrefixed) {
            let len = VarInt::new(self.len() as i32);
            len.encode(writer, opts)?;
        }

        writer.write_all(self)?;

        Ok(())
    }
}

impl<T: NetEncode + ?Sized + ToOwned> NetEncode for Cow<'_, T> {
    fn encode<W: Write>(&self, writer: &mut W, opts: &NetEncodeOpts) -> Result<(), NetEncodeError> {
        self.deref().encode(writer, opts)
    }
}

impl<T: NetEncode> NetEncode for Option<T> {
    fn encode<W: Write>(&self, writer: &mut W, opts: &NetEncodeOpts) -> Result<(), NetEncodeError> {
        match self {
            Some(value) => value.encode(writer, opts),
            None => Ok(()),
        }
    }
}

impl<K, V> NetEncode for HashMap<K, V>
where
    K: NetEncode,
    V: NetEncode,
{
    fn encode<W: Write>(&self, writer: &mut W, opts: &NetEncodeOpts) -> Result<(), NetEncodeError> {
        let len = VarInt::new(self.len() as i32);
        len.encode(writer, opts)?;

        for (key, value) in self {
            key.encode(writer, opts)?;
            value.encode(writer, opts)?;
        }
        Ok(())
    }
}
