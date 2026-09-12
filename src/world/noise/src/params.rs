#[derive(Debug, Clone)]
pub struct NoiseParameter {
    pub name: &'static str,
    pub amplitudes: &'static [f64],
    pub first_octave: i32,
}

include!(concat!(env!("OUT_DIR"), "/parameters_impl.rs"));
