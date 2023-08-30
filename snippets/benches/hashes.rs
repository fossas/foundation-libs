use criterion::{black_box, criterion_group, criterion_main, Criterion};
use sha2::{Digest, Sha256};
use snippets::text::ConvertCRLFToLF;

const INPUT: &[u8] = b"Iste nam laboriosam \r\n voluptatem \n distinctio.";

fn hash_slice_update() {
    let mut hasher = Sha256::new();
    hasher.update(INPUT);
    let digest = hasher.finalize().as_slice().to_vec();

    black_box(digest);
}

fn hash_slice_digest() {
    let digest = Sha256::digest(INPUT);

    black_box(digest);
}

fn hash_iter() {
    let mut hasher = Sha256::new();
    for c in INPUT.iter().copied() {
        hasher.update([c]);
    }
    let digest = hasher.finalize().as_slice().to_vec();

    black_box(digest);
}

fn hash_vec() {
    let mut hasher = Sha256::new();
    let input = INPUT.to_vec();
    hasher.update(&input);
    let digest = hasher.finalize().as_slice().to_vec();

    black_box(digest);
}

fn hash_transform_iter() {
    let mut hasher = Sha256::new();
    for c in INPUT.iter().copied().convert_crlf_lf() {
        hasher.update([c]);
    }
    let digest = hasher.finalize().as_slice().to_vec();

    black_box(digest);
}

fn hash_transform_vec() {
    let mut hasher = Sha256::new();
    let input = INPUT.iter().copied().convert_crlf_lf().collect::<Vec<_>>();
    hasher.update(&input);
    let digest = hasher.finalize().as_slice().to_vec();

    black_box(digest);
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("hash_slice_update", |b| b.iter(hash_slice_update));
    c.bench_function("hash_slice_digest", |b| b.iter(hash_slice_digest));
    c.bench_function("hash_iter", |b| b.iter(hash_iter));
    c.bench_function("hash_vec", |b| b.iter(hash_vec));
    c.bench_function("hash_transform_iter", |b| b.iter(hash_transform_iter));
    c.bench_function("hash_transform_vec", |b| b.iter(hash_transform_vec));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
