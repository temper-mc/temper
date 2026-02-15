use criterion::{Criterion, criterion_group, criterion_main};
use std::time::Duration;
use temper_threadpool::ThreadPool;

fn bench(c: &mut Criterion) {
    let pool = ThreadPool::new();
    c.bench_function("bench_threadpool", |b| {
        b.iter(|| {
            let mut batch = pool.batch();
            for _ in 0..100 {
                batch.execute(|| std::thread::sleep(Duration::from_millis(10)));
            }
            batch.wait();
        })
    });
}

criterion_group!(threadpool_bench, bench);
criterion_main!(threadpool_bench);
