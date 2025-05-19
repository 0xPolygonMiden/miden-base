use std::time::Duration;

use bench_prover::{
    bench_functions::{prove_consume_multiple_notes, prove_consume_note_with_new_account},
    benchmark_names::{BENCH_CONSUME_MULTIPLE_NOTES, BENCH_CONSUME_NOTE_NEW_ACCOUNT, BENCH_GROUP},
};
use criterion::{Criterion, SamplingMode, black_box, criterion_group, criterion_main};

fn core_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group(BENCH_GROUP);

    group
        .sampling_mode(SamplingMode::Flat)
        .sample_size(10)
        .warm_up_time(Duration::from_millis(1000));

    group.bench_function(BENCH_CONSUME_NOTE_NEW_ACCOUNT, |b| {
        b.iter(|| black_box(prove_consume_note_with_new_account()))
    });

    group.bench_function(BENCH_CONSUME_MULTIPLE_NOTES, |b| {
        b.iter(|| black_box(prove_consume_multiple_notes()))
    });

    group.finish();
}
criterion_group!(benches, core_benchmarks);
criterion_main!(benches);
