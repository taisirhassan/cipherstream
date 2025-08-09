use cipherstream::crypto;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::io::Write;
use tempfile::NamedTempFile;

fn bench_file_hashing(c: &mut Criterion) {
    let sizes = [1024usize, 1024 * 64, 1024 * 1024];
    for &size in &sizes {
        let mut tmp = NamedTempFile::new().unwrap();
        tmp.write_all(&vec![0x42; size]).unwrap();
        let path = tmp.path().to_path_buf();
        c.bench_function(&format!("hash_file_{}KB", size / 1024), |b| {
            b.iter(|| {
                let _ = futures::executor::block_on(async {
                    crypto::compute_file_hash(&path).await.unwrap()
                });
                black_box(());
            })
        });
    }
}

criterion_group!(benches, bench_file_hashing);
criterion_main!(benches);
