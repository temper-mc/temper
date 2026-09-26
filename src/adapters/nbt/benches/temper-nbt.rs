use crate::structs::Chunk;
use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use fastnbt::Value;
use nbt as hematite_nbt;
use std::hint::black_box;
use std::io::Cursor;
use temper_codec::decode::{NetDecode, NetDecodeOpts};
use temper_codec::encode::{NetEncode, NetEncodeOpts};
use temper_macros::{NBTDeserialize, NBTSerialize};
use temper_nbt::NBT;

mod structs {
    use super::*;
    #[derive(NBTDeserialize)]
    pub(super) struct Chunk {
        #[nbt(rename = "xPos")]
        pub(crate) x_pos: i32,
        #[nbt(rename = "zPos")]
        pub(crate) z_pos: i32,
        #[nbt(rename = "Heightmaps")]
        pub(crate) heightmaps: Heightmaps,
        #[nbt(rename = "sections")]
        _sections: Vec<Section>,
    }

    #[derive(NBTDeserialize)]
    pub(super) struct Heightmaps {
        #[nbt(rename = "MOTION_BLOCKING")]
        pub(crate) motion_blocking: Vec<i64>,
    }

    #[derive(NBTDeserialize)]
    pub(super) struct Section {
        #[nbt(rename = "Y")]
        _y: i8,
        _block_states: Option<BlockState>,
    }

    #[derive(NBTDeserialize)]
    pub(super) struct BlockState {
        pub(crate) _data: Option<Vec<i64>>,
        pub(crate) _palette: Vec<Palette>,
    }

    #[derive(NBTDeserialize)]
    pub(super) struct Palette {
        #[nbt(rename = "Name")]
        pub(crate) _name: String,
    }

    #[derive(Clone, NBTSerialize, NBTDeserialize)]
    pub(super) struct NetworkFixture {
        pub(crate) name: String,
        pub(crate) health: i32,
        pub(crate) heights: Vec<i64>,
    }

    impl NetworkFixture {
        pub(crate) fn sample() -> Self {
            Self {
                name: "temper".to_string(),
                health: 20,
                heights: vec![64, 72, 80],
            }
        }
    }
}

fn network_fixture_data() -> Vec<u8> {
    let mut bytes = Vec::new();
    NBT::new(structs::NetworkFixture::sample())
        .encode(&mut bytes, &NetEncodeOpts::None)
        .unwrap();
    bytes
}

fn bench_temper_nbt(data: &[u8]) {
    let chunk = Chunk::from_bytes(data).unwrap();
    assert_eq!(chunk.x_pos, 0);
    assert_eq!(chunk.z_pos, 32);
    assert_eq!(chunk.heightmaps.motion_blocking.len(), 37);
}

fn fastnbt(data: &[u8]) {
    let nbt: Value = black_box(fastnbt::from_reader(&mut Cursor::new(data)).unwrap());
    black_box(nbt);
}

fn crab_nbt(data: &[u8]) {
    let nbt = crab_nbt::Nbt::read(&mut Cursor::new(data)).unwrap();
    black_box(nbt);
}

fn hematite_nbt(data: &[u8]) {
    let nbt = hematite_nbt::Blob::from_reader(&mut Cursor::new(data)).unwrap();
    black_box(nbt);
}

fn temper_netdecode(data: &[u8]) {
    let nbt = NBT::<structs::NetworkFixture>::decode(&mut Cursor::new(data), &NetDecodeOpts::None)
        .unwrap();
    black_box(nbt);
}

fn criterion_benchmark(c: &mut Criterion) {
    let data = include_bytes!("../../../../.etc/benches/chunk_0-0.nbt");

    let mut group = c.benchmark_group("Chunk Data NBT Parsing");
    group.throughput(Throughput::Bytes(data.len() as u64));
    group.bench_function("temper NBT", |b| {
        b.iter(|| bench_temper_nbt(black_box(data)))
    });
    group.bench_function("fastnbt", |b| b.iter(|| fastnbt(black_box(data))));
    group.bench_function("crab_nbt", |b| b.iter(|| crab_nbt(black_box(data))));
    group.bench_function("hematite_nbt", |b| b.iter(|| hematite_nbt(black_box(data))));
    group.finish();

    let network_data = network_fixture_data();
    let mut group = c.benchmark_group("Network NBT Decode");
    group.throughput(Throughput::Bytes(network_data.len() as u64));
    group.bench_function("temper NetDecode", |b| {
        b.iter(|| temper_netdecode(black_box(&network_data)))
    });
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
