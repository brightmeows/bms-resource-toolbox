//! Benchmark for async filesystem operations.

use bms_resource_toolbox::infra::fs::pack_move::is_dir_having_file;
use criterion::{Criterion, criterion_group, criterion_main};
use tokio::runtime::Runtime;

fn benchmark_async_is_dir_having_file(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("async_is_dir_having_file", |b| {
        let temp_dir = tempfile::tempdir().unwrap();
        b.iter(|| rt.block_on(is_dir_having_file(temp_dir.path())));
    });
}

criterion_group!(benches, benchmark_async_is_dir_having_file);
criterion_main!(benches);
