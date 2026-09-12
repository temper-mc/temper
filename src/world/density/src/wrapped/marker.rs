use crate::wrapped::WrappedDensityFunction;
use temper_core::pos::BlockPos;

pub struct CacheData {
    last_pos: BlockPos,
    last_value: f64,
}

impl Default for CacheData {
    fn default() -> Self {
        Self {
            last_pos: BlockPos::of(i32::MAX, i32::MAX, i32::MAX),
            last_value: 0.0,
        }
    }
}

pub enum MarkerDensityFunction {
    CacheAllInCell,
    Interpolated,
    CacheOnce(CacheData),
    Cache2d(CacheData),
    FlatCache(CacheData),
}

impl MarkerDensityFunction {
    pub fn execute(&mut self, inner: usize, func: &WrappedDensityFunction) -> f64 {
        match self {
            Self::CacheAllInCell => func.execute_inner(inner),
            Self::Interpolated => func.execute_inner(inner),
            Self::CacheOnce(data) => {
                if func.pos != data.last_pos {
                    data.last_pos = func.pos;
                    data.last_value = func.execute_inner(inner);
                }

                data.last_value
            }
            Self::Cache2d(data) => {
                let pos = BlockPos::of(func.pos.pos.x, 0, func.pos.pos.z);

                if pos != data.last_pos {
                    data.last_pos = pos;
                    data.last_value = func.execute_inner(inner);
                }

                data.last_value
            }
            Self::FlatCache(data) => {
                let pos = BlockPos::of(func.pos.pos.x >> 2, 0, func.pos.pos.z >> 2);

                if pos != data.last_pos {
                    data.last_pos = pos;
                    data.last_value = func
                        .execute_inner_at(inner, BlockPos::of(pos.pos.x << 2, 0, pos.pos.z << 2))
                }

                data.last_value
                // if let None = data.values {
                //     let mut values = vec![0.0; (data.size_xz * data.size_xz) as usize].into_boxed_slice();
                //
                //     for x in 0..data.size_xz {
                //         let quart_x = data.first_x + x;
                //         let block_x = quart_x << 2;
                //
                //         for z in 0..data.size_xz {
                //             let quart_z = data.first_z + z;
                //             let block_z = quart_z << 2;
                //
                //             values[(x + z * data.size_xz) as usize] = func.execute_inner_at(inner, BlockPos::of(block_x, 0, block_z));
                //         }
                //     }
                //
                //     data.values = Some(values);
                // }
                //
                // let Some(values) = &mut data.values else {
                //     unreachable!()
                // };
                //
                // let quart_x = func.pos.pos.x >> 2;
                // let quart_z = func.pos.pos.z >> 2;
                // let x = quart_x - data.first_x;
                // let z = quart_z - data.first_z;
                //
                // if x >= 0 && z >= 0 && x < data.size_xz && z < data.size_xz {
                //     values[(x + z * data.size_xz) as usize]
                // } else {
                //     func.execute_inner(inner)
                // }
            }
        }
    }
}
