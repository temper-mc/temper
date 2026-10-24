use std::hint::black_box;
use std::path::Path;
use std::time::{Duration, Instant};

use temper_config::server_config::create_dummy_config;
use temper_core::dimension::Dimension;
use temper_core::pos::ChunkPos;
use temper_world::World;

pub fn bench_gen(c: &mut criterion::Criterion) {
    let mut group = c.benchmark_group("world_gen");
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    for size in [1, 8, 16] {
        let mut world_index = 0;

        group.throughput(criterion::Throughput::Elements((size * size) as u64));
        group.bench_function(format!("generate to {}, {}", size, size), |b| {
            b.iter_custom(|iterations| {
                let mut elapsed = Duration::ZERO;

                for _ in 0..iterations {
                    let world = new_bench_world(temp_dir.path(), format!("{size}-{world_index}"));
                    world_index += 1;

                    let start = Instant::now();
                    generate_region(&world, size);
                    elapsed += start.elapsed();
                }

                elapsed
            })
        });
    }

    group.finish();
}

fn new_bench_world(root: &Path, name: String) -> World {
    let db_path = root.join(name);
    let mut config = create_dummy_config();
    config.world_gen.generator = "normal".to_string();
    config.database.db_path = db_path.to_string_lossy().to_string();

    World::new(db_path, &config).unwrap()
}

fn generate_region(world: &World, size: i32) {
    let store = &world.chunks;

    for x in 0..size {
        for z in 0..size {
            let pos = black_box(ChunkPos::new(x, z));
            black_box(
                world
                    .chunk_generators
                    .get(&Dimension::Overworld)
                    .expect("overworld generator didnt exist")
                    .generate(store, Dimension::Overworld, pos),
            )
            .unwrap();
        }
    }
}
