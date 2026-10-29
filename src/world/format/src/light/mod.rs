use deepsize::DeepSizeOf;
use serde_derive::{Deserialize, Serialize};
use type_hash::TypeHash;

pub mod network;

#[derive(Default, Clone, DeepSizeOf, Serialize, Deserialize, TypeHash)]
pub(crate) enum LightStorage {
    #[default]
    Empty,
    Full,
    Mixed {
        light_data: Box<[u8]>,
    },
}

#[derive(Clone, DeepSizeOf, Serialize, Deserialize, TypeHash)]
pub struct SectionLightData {
    sky_light: LightStorage,
    block_light: LightStorage,
}

impl Default for SectionLightData {
    fn default() -> Self {
        Self {
            sky_light: LightStorage::Full,
            block_light: LightStorage::default(),
        }
    }
}

impl From<Vec<i8>> for LightStorage {
    fn from(data: Vec<i8>) -> Self {
        if data.len() != 2048 {
            Self::Empty
        } else {
            let mut all_on = true;
            let mut all_off = true;

            for b in data.iter() {
                if *b != -1 {
                    // Full light is 0xFF, which is -1 as i8 (Two's Complement)
                    all_on = false
                };
                if *b != 0 {
                    // No light is 0x00, which is 0 as i8
                    all_off = false
                };
            }

            if all_on {
                Self::Full
            } else if all_off {
                Self::Empty
            } else {
                Self::Mixed {
                    light_data: data.into_iter().map(|v| v as u8).collect(),
                }
            }
        }
    }
}

impl SectionLightData {
    pub(crate) fn with_data(sky_light: LightStorage, block_light: LightStorage) -> Self {
        Self {
            sky_light,
            block_light,
        }
    }

    #[inline]
    pub fn contains_sky_light(&self) -> bool {
        self.sky_light.contains_light()
    }

    #[inline]
    pub fn contains_block_light(&self) -> bool {
        self.block_light.contains_light()
    }
}

impl LightStorage {
    #[inline]
    pub fn contains_light(&self) -> bool {
        match self {
            LightStorage::Empty => false,
            LightStorage::Full => true,
            LightStorage::Mixed { .. } => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_light_arrays_compress() {
        assert!(matches!(
            LightStorage::from(vec![-1i8; 2048]),
            LightStorage::Full
        ));
        assert!(matches!(
            LightStorage::from(vec![0i8; 2048]),
            LightStorage::Empty
        ));

        let mut mixed = vec![0i8; 2048];
        mixed[100] = 0x0F;
        assert!(matches!(
            LightStorage::from(mixed),
            LightStorage::Mixed { .. }
        ));
    }
}
