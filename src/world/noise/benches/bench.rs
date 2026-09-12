use bevy_math::DVec3;
use criterion::{Criterion, criterion_group, criterion_main};
use noise::{NoiseFn, Perlin};
use temper_core::random::XoroshiroRandomSource;
use temper_noise::{ImprovedNoise, NormalNoise, PerlinNoise};

fn bench_improved(c: &mut Criterion) {
    let mut random = XoroshiroRandomSource::new(0);
    let map = ImprovedNoise::new(&mut random);

    c.bench_function("improved noise - scalar", |b| {
        b.iter(|| {
            std::hint::black_box([map.noise(DVec3::splat(0.0))]);
        })
    });

    let map = Perlin::new(0);
    c.bench_function("improved (perlin) noise - scalar - noise crate", |b| {
        b.iter(|| {
            std::hint::black_box([map.get([0.0, 0.0, 0.0])]);
        })
    });
}

fn bench_perlin(c: &mut Criterion) {
    let mut random = XoroshiroRandomSource::new(0);
    let map = PerlinNoise::new(&mut random, -1, &[1.0, 1.0, 0.0, 2.0]);

    c.bench_function("perlin noise - scalar", |b| {
        b.iter(|| {
            std::hint::black_box([map.noise(DVec3::splat(0.0))]);
        })
    });
}

fn bench_normal(c: &mut Criterion) {
    let mut random = XoroshiroRandomSource::new(0);
    let map = NormalNoise::new_custom(&mut random, -1, &[1.0, 1.0, 0.0, 2.0]);

    c.bench_function("normal noise - scalar", |b| {
        b.iter(|| {
            std::hint::black_box([map.noise(DVec3::splat(0.0))]);
        })
    });
}

fn bench_blended(c: &mut Criterion) {
    let mut random = XoroshiroRandomSource::new(0);
    let map = NormalNoise::new_custom(&mut random, -1, &[1.0, 1.0, 0.0, 2.0]);

    c.bench_function("blended noise - scalar", |b| {
        b.iter(|| {
            std::hint::black_box([map.noise(DVec3::splat(0.0))]);
        })
    });
}

criterion_group!(
    benches,
    bench_improved,
    bench_perlin,
    bench_normal,
    bench_blended,
);
criterion_main!(benches);
