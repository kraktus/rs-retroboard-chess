use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use retroboard::{perft, shakmaty::Chess, RetroBoard};

pub fn criterion_benchmark(c: &mut Criterion) {
    let fen = "q4N2/1p5k/8/8/6P1/4Q3/1K1PB3/7r b - - 0 1";
    let white_p = "2PNBRQ";
    let black_p = "3NBRQP";
    let rboard = RetroBoard::new(fen, white_p, black_p).unwrap();

    c.bench_function("rboard clone", |b| b.iter(|| black_box(rboard.clone())));
    c.bench_function("perft", |b| {
        b.iter(|| assert_eq!(perft(black_box(&rboard), 2), 3951))
    });
    c.bench_function("chess from rboard", move |b| {
        b.iter_batched(|| rboard.clone(), Chess::from, BatchSize::SmallInput)
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
