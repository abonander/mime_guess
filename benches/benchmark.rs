#[macro_use]
extern crate criterion;
extern crate mime_guess;
extern crate rand_pcg;
extern crate rand;

use std::env;
use std::sync::LazyLock;
use rand::seq::IndexedRandom;
use rand_pcg::Pcg64Mcg;
use self::criterion::Criterion;

#[path = "../src/mime_types.rs"]
mod mime_types;

/// Get a reproducible, seeded RNG.
fn rng_seeded() -> Pcg64Mcg {
    static SEED: LazyLock<u128> = LazyLock::new(|| {
        if let Ok(seed) = env::var("BENCH_RNG_SEED") {
            return seed.parse().expect("failed to parse BENCH_RNG_SEED");
        }

        let seed = rand::random();

        eprintln!("Generated random seed; set BENCH_RNG_SEED={seed} to reproduce results");

        seed
    });

    Pcg64Mcg::new(*SEED)
}

/// WARNING: this may take a while!
fn bench_mime_str(c: &mut Criterion) {
    let mut rng = rng_seeded();

    c.bench_function("from_ext", |b| {

        b.iter(|| {
            let (mime_ext, _) = mime_types::MIME_TYPES.choose(&mut rng).unwrap();

            mime_guess::from_ext(mime_ext).first_raw()
        });
    });
}

fn bench_mime_str_uppercase(c: &mut Criterion) {
    let mut rng = rng_seeded();

    c.bench_function("from_ext uppercased", |b| {
        b.iter(|| {
            let (mime_ext, _) = mime_types::MIME_TYPES.choose(&mut rng).unwrap();

            let mime_ext = mime_ext.to_uppercase();

            mime_guess::from_ext(&mime_ext).first_raw()
        });
    });
}

criterion_group!(benches, bench_mime_str, bench_mime_str_uppercase);
criterion_main!(benches);
