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

/// A trait to provide math extensions to various unsafe primitives (like SIMD
/// primitives).
pub trait TemperMathExtUnsafe: Sized {
    /// # Safety
    /// This function requires the avx2 feature set.
    unsafe fn square(self) -> Self;

    /// # Safety
    /// This function requires the avx2 feature set.
    unsafe fn cube(self) -> Self;

    /// # Safety
    /// This function requires the avx2 feature set.
    unsafe fn inverse(self) -> Self;

    /// # Safety
    /// This function requires the avx2 feature set.
    unsafe fn smooth_step(self) -> Self;

    /// # Safety
    /// This function requires the avx2 feature set.
    unsafe fn lerp(self, p0: Self, p1: Self) -> Self;

    /// # Safety
    /// This function requires the avx2 feature set.
    unsafe fn lerp2(t0: Self, t1: Self, p00: Self, p01: Self, p10: Self, p11: Self) -> Self;

    /// # Safety
    /// This function requires the avx2 feature set.
    #[expect(clippy::too_many_arguments)]
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
    ) -> Self;
}
