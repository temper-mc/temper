use crate::math::{TemperMathExt, TemperMathExtUnsafe};

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

impl TemperMathExt for f32 {
    fn smooth_step(self) -> Self {
        self * self * self * (self * (self * 6.0 - 15.0) + 10.0)
    }

    fn lerp(self, p0: Self, p1: Self) -> Self {
        p0 + self * (p1 - p0)
    }

    fn clamped_lerp(self, p0: Self, p1: Self) -> Self {
        self.clamp(0.0, 1.0).lerp(p0, p1)
    }

    fn inverse_lerp(self, p0: Self, p1: Self) -> Self {
        (self - p0) / (p1 - p0)
    }

    fn lerp2(t0: Self, t1: Self, p00: Self, p01: Self, p10: Self, p11: Self) -> Self {
        t1.lerp(t0.lerp(p00, p01), t0.lerp(p10, p11))
    }

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
    ) -> Self {
        t2.lerp(
            f32::lerp2(t0, t1, p000, p001, p010, p011),
            f32::lerp2(t0, t1, p100, p101, p110, p111),
        )
    }
}

#[cfg(target_arch = "x86_64")]
impl TemperMathExtUnsafe for __m256 {
    #[target_feature(enable = "avx2")]
    unsafe fn square(self) -> Self {
        _mm256_mul_ps(self, self)
    }

    #[target_feature(enable = "avx2")]
    unsafe fn cube(self) -> Self {
        _mm256_mul_ps(_mm256_mul_ps(self, self), self)
    }

    #[target_feature(enable = "avx2")]
    unsafe fn inverse(self) -> Self {
        _mm256_div_ps(_mm256_set1_ps(1.0), self)
    }

    #[target_feature(enable = "avx2")]
    unsafe fn smooth_step(self) -> Self {
        let a = _mm256_fma_fallback_ps(self, _mm256_set1_ps(6.0), _mm256_set1_ps(-15.0));

        let b = _mm256_fma_fallback_ps(self, a, _mm256_set1_ps(10.0));

        _mm256_mul_ps(_mm256_mul_ps(_mm256_mul_ps(self, self), self), b)
    }

    #[target_feature(enable = "avx2")]
    unsafe fn lerp(self, p0: Self, p1: Self) -> Self {
        let a = _mm256_sub_ps(p1, p0);
        _mm256_fma_fallback_ps(self, a, p0)
    }

    #[target_feature(enable = "avx2")]
    unsafe fn lerp2(t0: Self, t1: Self, p00: Self, p01: Self, p10: Self, p11: Self) -> Self {
        t1.lerp(t0.lerp(p00, p01), t0.lerp(p10, p11))
    }

    #[target_feature(enable = "avx2")]
    unsafe fn lerp3(
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
    ) -> Self {
        t2.lerp(
            __m256::lerp2(t0, t1, p000, p001, p010, p011),
            __m256::lerp2(t0, t1, p100, p101, p110, p111),
        )
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub fn _mm256_fma_fallback_ps(a: __m256, b: __m256, c: __m256) -> __m256 {
    if is_x86_feature_detected!("fma") {
        // SAFETY: required features are present if we made it here
        unsafe { _mm256_fmadd_ps(a, b, c) }
    } else {
        _mm256_add_ps(_mm256_mul_ps(a, b), c)
    }
}
