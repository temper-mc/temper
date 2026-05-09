use crate::decode::errors::NetDecodeError;
use std::io::Read;

pub mod errors;
mod primitives;

/// Sole purpose is for compression compatibility.
/// And possibly other stuff in the future.
#[derive(Debug, Clone, Copy, Default)]
pub enum NetDecodeOpts {
    #[default]
    None,
    IsSizePrefixed,
}

pub trait NetDecode: Sized {
    fn decode<R: Read>(reader: &mut R, opts: &NetDecodeOpts) -> Result<Self, NetDecodeError>;
}
