use crate::{blob::NbtBlob, FromNbt, NBTSerializable, NBTSerializeOptions, NbtTape};
use std::fmt::Debug;
use std::io::{Read, Write};
use std::ops::{Deref, DerefMut};
use temper_codec::decode::errors::NetDecodeError;
use temper_codec::decode::{NetDecode, NetDecodeOpts};
use temper_codec::encode::errors::NetEncodeError;
use temper_codec::encode::{NetEncode, NetEncodeOpts};

pub struct NBT<T> {
    inner: T,
}

impl<T> NBT<T> {
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<T: NBTSerializable> NetEncode for NBT<T> {
    fn encode<W: Write>(
        &self,
        writer: &mut W,
        _opts: &NetEncodeOpts,
    ) -> Result<(), NetEncodeError> {
        self.inner.serialize(writer, &NBTSerializeOptions::Network);
        Ok(())
    }
}

impl<T: for<'a> FromNbt<'a>> NetDecode for NBT<T> {
    fn decode<R: Read>(reader: &mut R, _opts: &NetDecodeOpts) -> Result<Self, NetDecodeError> {
        let bytes = NbtBlob::decode(reader, &NetDecodeOpts::None)?;
        let mut tape = NbtTape::new(&bytes.0);
        tape.parse_network_root()
            .map_err(|_| NetDecodeError::ExternalError("NBT Parse Error".into()))?;
        let root = tape.take_root().map_err(|_| {
            NetDecodeError::ExternalError("NBT did not contain a root compound".into())
        })?;

        Ok(NBT {
            inner: T::from_nbt(&tape, root)
                .map_err(|_| NetDecodeError::ExternalError("NBT Parse Error".into()))?,
        })
    }
}

impl<T> From<T> for NBT<T> {
    fn from(inner: T) -> Self {
        Self { inner }
    }
}

impl<T: Default> Default for NBT<T> {
    fn default() -> Self {
        Self {
            inner: T::default(),
        }
    }
}

impl<T: Clone> Clone for NBT<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T: PartialEq> PartialEq for NBT<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner.eq(&other.inner)
    }
}

impl<T: Debug> Debug for NBT<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl<T> Deref for NBT<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for NBT<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NBTError, NbtTapeElement};
    use std::io::Cursor;
    use temper_codec::encode::NetEncodeOpts;
    use tokio::io::{AsyncWrite, AsyncWriteExt};

    #[derive(Clone, Debug, PartialEq)]
    struct NetworkFixture {
        name: String,
        health: i32,
        heights: Vec<i64>,
    }

    impl NetworkFixture {
        fn sample() -> Self {
            Self {
                name: "temper".to_string(),
                health: 20,
                heights: vec![64, 72, 80],
            }
        }
    }

    impl NBTSerializable for NetworkFixture {
        fn serialize<W: Write>(&self, writer: &mut W, options: &NBTSerializeOptions<'_>) {
            match options {
                NBTSerializeOptions::None | NBTSerializeOptions::Flatten => {}
                NBTSerializeOptions::Network => {
                    Self::id().serialize(writer, &NBTSerializeOptions::None);
                }
                NBTSerializeOptions::WithHeader(name) => {
                    Self::id().serialize(writer, &NBTSerializeOptions::None);
                    name.serialize(writer, &NBTSerializeOptions::None);
                }
            }

            self.name
                .serialize(writer, &NBTSerializeOptions::WithHeader("name"));
            self.health
                .serialize(writer, &NBTSerializeOptions::WithHeader("health"));
            self.heights
                .serialize(writer, &NBTSerializeOptions::WithHeader("heights"));

            if options != &NBTSerializeOptions::Flatten {
                0u8.serialize(writer, &NBTSerializeOptions::None);
            }
        }

        async fn serialize_async<W: AsyncWrite + Unpin>(
            &self,
            writer: &mut W,
            options: &NBTSerializeOptions<'_>,
        ) {
            let mut buf = Vec::new();
            self.serialize(&mut buf, options);
            writer
                .write_all(&buf)
                .await
                .expect("failed to write fixture bytes");
        }

        fn id() -> u8 {
            10
        }
    }

    impl<'a> FromNbt<'a> for NetworkFixture {
        fn from_nbt<'tape>(
            tapes: &'tape NbtTape<'a>,
            mut element: NbtTapeElement<'a, 'tape>,
        ) -> crate::Result<Self>
        where
            'a: 'tape,
        {
            if !matches!(element, NbtTapeElement::Compound(_)) {
                return Err(NBTError::TypeMismatch {
                    expected: "Compound",
                    found: element.nbt_type(),
                });
            }

            Ok(Self {
                name: String::from_nbt(
                    tapes,
                    element.take("name").ok_or(NBTError::ElementNotFound("name"))?,
                )?,
                health: i32::from_nbt(
                    tapes,
                    element
                        .take("health")
                        .ok_or(NBTError::ElementNotFound("health"))?,
                )?,
                heights: Vec::<i64>::from_nbt(
                    tapes,
                    element
                        .take("heights")
                        .ok_or(NBTError::ElementNotFound("heights"))?,
                )?,
            })
        }
    }

    fn encode_network_fixture() -> Vec<u8> {
        let mut bytes = Vec::new();
        NBT::new(NetworkFixture::sample())
            .encode(&mut bytes, &NetEncodeOpts::None)
            .expect("failed to encode fixture");
        bytes
    }

    #[test]
    fn net_decode_round_trips_network_payload() {
        let mut reader = Cursor::new(encode_network_fixture());

        let decoded = NBT::<NetworkFixture>::decode(&mut reader, &NetDecodeOpts::None)
            .expect("failed to decode network NBT");

        assert_eq!(*decoded, NetworkFixture::sample());
    }

    #[test]
    fn net_decode_leaves_next_byte_in_reader() {
        let marker = 0x2a;
        let mut bytes = encode_network_fixture();
        bytes.push(marker);
        let mut reader = Cursor::new(bytes);

        let decoded = NBT::<NetworkFixture>::decode(&mut reader, &NetDecodeOpts::None)
            .expect("failed to decode network NBT");

        assert_eq!(*decoded, NetworkFixture::sample());
        assert_eq!(
            u8::decode(&mut reader, &NetDecodeOpts::None).expect("failed to read marker byte"),
            marker
        );
    }

    #[test]
    fn net_decode_rejects_non_compound_root() {
        let mut reader = Cursor::new([3, 0, 0, 0, 42]);

        let err = NBT::<NetworkFixture>::decode(&mut reader, &NetDecodeOpts::None)
            .expect_err("decoded invalid root tag");

        assert!(matches!(err, NetDecodeError::ExternalError(_)));
    }
}
