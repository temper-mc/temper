mod double;
mod float;

/// A trait to provide math extensions to various primitives.
pub trait TemperMathExt: Sized {
    fn smooth_step(self) -> Self;

    fn lerp(self, p0: Self, p1: Self) -> Self;

    fn clamped_lerp(self, p0: Self, p1: Self) -> Self;

    fn inverse_lerp(self, p0: Self, p1: Self) -> Self;

    fn lerp2(t0: Self, t1: Self, p00: Self, p01: Self, p10: Self, p11: Self) -> Self;

    #[expect(clippy::too_many_arguments)]
    fn lerp3(
        t0: Self,
        t1: Self,
        t2: Self,
        p000: Self,
        p001: Self,
        p010: Self,
        p011: Self,
        p100: Self,
        p101: Self,
        p110: Self,
        p111: Self,
    ) -> Self;

    fn clamped_map(self, from_min: Self, from_max: Self, to_min: Self, to_max: Self) -> Self {
        self.inverse_lerp(from_min, from_max)
            .clamped_lerp(to_min, to_max)
    }
}

pub fn java_hash_code(str: &str) -> u32 {
    let mut output = 0u32;

    for (i, char) in str.chars().enumerate() {
        output = output.wrapping_add(
            (char as u32).wrapping_mul(31u32.wrapping_pow(str.len() as u32 - i as u32 - 1)),
        )
    }

    output
}

#[cfg(test)]
mod tests {
    use crate::math::java_hash_code;

    #[test]
    fn test_java_hash_code() {
        assert_eq!(java_hash_code("Hello, world!"), 2414922741);
        assert_eq!(java_hash_code("temper rox"), 1116817596);
        assert_eq!(java_hash_code("my custom seed"), 1881557324);
        assert_eq!(java_hash_code("abcghimnostuyz123789"), 1899513971);
        assert_eq!(java_hash_code("TEMPER ROX!"), 2444120837);
    }
}
