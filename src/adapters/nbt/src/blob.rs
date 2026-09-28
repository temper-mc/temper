use crate::de::borrow::NbtTag;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use temper_codec::decode::errors::NetDecodeError;
use temper_codec::decode::{NetDecode, NetDecodeOpts};
use temper_codec::encode::errors::NetEncodeError;
use temper_codec::encode::{NetEncode, NetEncodeOpts};
use type_hash::TypeHash;

const INITIAL_BLOB_CAPACITY: usize = 256;

/// A lump of NBT data as bytes. Useful for when you need to read
/// some NBT but don't actually care what's in there.
#[derive(Debug, Clone, Hash, Default, PartialEq, TypeHash, Serialize, Deserialize)]
pub struct NbtBlob(pub Vec<u8>);

impl NetDecode for NbtBlob {
    fn decode<R: Read>(reader: &mut R, _opts: &NetDecodeOpts) -> Result<Self, NetDecodeError> {
        let mut bytes = Vec::with_capacity(INITIAL_BLOB_CAPACITY);
        let tag = read_tag(reader, &mut bytes)?;
        read_payload(reader, &mut bytes, tag)?;

        Ok(Self(bytes))
    }
}

impl NetEncode for NbtBlob {
    fn encode<W: Write>(
        &self,
        writer: &mut W,
        _opts: &NetEncodeOpts,
    ) -> Result<(), NetEncodeError> {
        writer.write_all(&self.0).map_err(NetEncodeError::Io)
    }
}

fn read_payload<R: Read>(
    reader: &mut R,
    bytes: &mut Vec<u8>,
    tag: NbtTag,
) -> Result<(), NetDecodeError> {
    match tag {
        NbtTag::End => {}
        NbtTag::Byte => read_bytes(reader, bytes, 1)?,
        NbtTag::Short => read_bytes(reader, bytes, 2)?,
        NbtTag::Int | NbtTag::Float => read_bytes(reader, bytes, 4)?,
        NbtTag::Long | NbtTag::Double => read_bytes(reader, bytes, 8)?,
        NbtTag::ByteArray => {
            let len = read_i32(reader, bytes)?;
            read_bytes(reader, bytes, checked_len(len, 1)?)?;
        }
        NbtTag::String => {
            let len = read_u16(reader, bytes)? as usize;
            read_bytes(reader, bytes, len)?;
        }
        NbtTag::List => {
            let el_type = read_tag(reader, bytes)?;
            let len = checked_len(read_i32(reader, bytes)?, 1)?;

            for _ in 0..len {
                read_payload(reader, bytes, el_type)?;
            }
        }
        NbtTag::Compound => loop {
            let tag = read_tag(reader, bytes)?;
            if tag == NbtTag::End {
                break;
            }

            read_string(reader, bytes)?;
            read_payload(reader, bytes, tag)?;
        },
        NbtTag::IntArray => {
            let len = read_i32(reader, bytes)?;
            read_bytes(reader, bytes, checked_len(len, 4)?)?;
        }
        NbtTag::LongArray => {
            let len = read_i32(reader, bytes)?;
            read_bytes(reader, bytes, checked_len(len, 8)?)?;
        }
    }

    Ok(())
}

fn read_string<R: Read>(reader: &mut R, bytes: &mut Vec<u8>) -> Result<(), NetDecodeError> {
    let len = read_u16(reader, bytes)? as usize;
    read_bytes(reader, bytes, len)
}

fn read_i32<R: Read>(reader: &mut R, bytes: &mut Vec<u8>) -> Result<i32, NetDecodeError> {
    let mut buf = [0; size_of::<i32>()];
    reader.read_exact(&mut buf)?;
    bytes.extend_from_slice(&buf);

    Ok(i32::from_be_bytes(buf))
}

fn read_u16<R: Read>(reader: &mut R, bytes: &mut Vec<u8>) -> Result<u16, NetDecodeError> {
    let mut buf = [0; size_of::<u16>()];
    reader.read_exact(&mut buf)?;
    bytes.extend_from_slice(&buf);

    Ok(u16::from_be_bytes(buf))
}

fn read_u8<R: Read>(reader: &mut R, bytes: &mut Vec<u8>) -> Result<u8, NetDecodeError> {
    let mut byte = [0; 1];
    reader.read_exact(&mut byte)?;
    bytes.push(byte[0]);

    Ok(byte[0])
}

fn read_tag<R: Read>(reader: &mut R, bytes: &mut Vec<u8>) -> Result<NbtTag, NetDecodeError> {
    let tag = read_u8(reader, bytes)?;

    NbtTag::from_byte(tag)
        .ok_or_else(|| NetDecodeError::ExternalError(format!("invalid NBT tag: {tag}").into()))
}

fn read_bytes<R: Read>(
    reader: &mut R,
    bytes: &mut Vec<u8>,
    len: usize,
) -> Result<(), NetDecodeError> {
    if len == 0 {
        return Ok(());
    }

    let start = bytes.len();
    bytes.resize(start + len, 0);
    reader.read_exact(&mut bytes[start..])?;

    Ok(())
}

fn checked_len(len: i32, width: usize) -> Result<usize, NetDecodeError> {
    let len = usize::try_from(len)
        .map_err(|_| NetDecodeError::ExternalError("negative NBT length".into()))?;

    len.checked_mul(width)
        .ok_or_else(|| NetDecodeError::ExternalError("NBT length overflow".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn blob_decode_reads_exactly_one_compound() {
        let mut bytes = vec![
            NbtTag::Compound as u8,
            NbtTag::String as u8,
            0,
            4,
            b'n',
            b'a',
            b'm',
            b'e',
            0,
            3,
            b'b',
            b'o',
            b'b',
            NbtTag::End as u8,
            99,
        ];
        let trailing = bytes.pop().unwrap();
        let mut reader = Cursor::new({
            let mut stream = bytes.clone();
            stream.push(trailing);
            stream
        });

        let blob = NbtBlob::decode(&mut reader, &NetDecodeOpts::None).unwrap();

        assert_eq!(blob.0, bytes);
        assert_eq!(reader.position(), blob.0.len() as u64);
    }

    #[test]
    fn blob_decode_reads_nested_lists_and_arrays() {
        let bytes = vec![
            NbtTag::Compound as u8,
            NbtTag::List as u8,
            0,
            6,
            b'v',
            b'a',
            b'l',
            b'u',
            b'e',
            b's',
            NbtTag::IntArray as u8,
            0,
            0,
            0,
            1,
            0,
            0,
            0,
            3,
            0,
            0,
            0,
            4,
            0,
            0,
            0,
            5,
            0,
            0,
            0,
            6,
            NbtTag::End as u8,
        ];
        let mut reader = Cursor::new(bytes.clone());

        let blob = NbtBlob::decode(&mut reader, &NetDecodeOpts::None).unwrap();

        assert_eq!(blob.0, bytes);
    }

    #[test]
    fn blob_decode_rejects_invalid_tags() {
        let mut reader = Cursor::new([13]);

        assert!(NbtBlob::decode(&mut reader, &NetDecodeOpts::None).is_err());
    }
}
