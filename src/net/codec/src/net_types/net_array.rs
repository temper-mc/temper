use crate::encode::errors::NetEncodeError;
use crate::encode::{NetEncode, NetEncodeOpts};
use crate::net_types::var_int::VarInt;
use std::borrow::Cow;
use std::io::Write;

pub struct NetworkArray<'data, T: NetEncode + ToOwned + Clone>(Cow<'data, [T]>);

impl<'data, T: NetEncode + ToOwned + Clone> NetworkArray<'data, T> {
    pub fn new_borrowed(data: &'data [T]) -> Self {
        NetworkArray(Cow::Borrowed(data))
    }

    pub fn new_owned(data: <[T] as ToOwned>::Owned) -> Self {
        NetworkArray(Cow::Owned(data))
    }

    pub fn into_inner(self) -> <[T] as ToOwned>::Owned {
        self.0.into_owned()
    }
}

impl<'data, T: NetEncode + ToOwned + Clone> NetEncode for NetworkArray<'data, T> {
    fn encode<W: Write>(&self, writer: &mut W, opts: &NetEncodeOpts) -> Result<(), NetEncodeError> {
        if matches!(opts, NetEncodeOpts::SizePrefixed) {
            let len = VarInt::new(self.0.len() as i32);
            len.encode(writer, opts)?;
        }

        for item in self.0.iter() {
            item.encode(writer, opts)?;
        }

        Ok(())
    }
}
