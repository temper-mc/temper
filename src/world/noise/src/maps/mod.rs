use bevy_math::DVec3;

mod blended;
mod improved;
mod normal;
mod perlin;

pub use blended::BlendedNoise;
pub use improved::ImprovedNoise;
pub use normal::NormalNoise;
pub use perlin::PerlinNoise;

const GRADIENT: [DVec3; 16] = [
    DVec3::new(1.0, 1.0, 0.0),
    DVec3::new(-1.0, 1.0, 0.0),
    DVec3::new(1.0, -1.0, 0.0),
    DVec3::new(-1.0, -1.0, 0.0),
    DVec3::new(1.0, 0.0, 1.0),
    DVec3::new(-1.0, 0.0, 1.0),
    DVec3::new(1.0, 0.0, -1.0),
    DVec3::new(-1.0, 0.0, -1.0),
    DVec3::new(0.0, 1.0, 1.0),
    DVec3::new(0.0, -1.0, 1.0),
    DVec3::new(0.0, 1.0, -1.0),
    DVec3::new(0.0, -1.0, -1.0),
    DVec3::new(1.0, 1.0, 0.0),
    DVec3::new(0.0, -1.0, 1.0),
    DVec3::new(-1.0, 1.0, 0.0),
    DVec3::new(0.0, -1.0, -1.0),
];

#[cfg(test)]
mod tests {
    use bevy_math::DVec3;
    use temper_core::random::XoroshiroRandomSource;

    pub mod data;

    pub type NoiseMapTest<T> = &'static [((u64, T), &'static [([f64; 3], f64)])];

    pub fn run_test<T, D, FN, FG>(data: &NoiseMapTest<D>, new_func: FN, get_func: FG)
    where
        FN: Fn(&mut XoroshiroRandomSource, &D) -> T,
        FG: Fn(&T, DVec3) -> f64,
    {
        for (i, ((seed, custom_data), data)) in data.iter().enumerate() {
            let mut random = XoroshiroRandomSource::new(*seed);
            let map = new_func(&mut random, custom_data);

            for (pos, data) in data.iter() {
                let pos = DVec3::from_array(*pos);
                let val = get_func(&map, pos);

                assert_eq!(val, *data, "Value mismatch on pos {} and seed #{i}", pos)
            }
        }
    }
}
