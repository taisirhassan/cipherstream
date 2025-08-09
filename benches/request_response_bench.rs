use cipherstream::file_transfer::{FileTransferCodec, FileTransferProtocol, ProtocolResponse};
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use futures::io::Cursor;
use libp2p::request_response::Codec;

fn bench_codec_response_roundtrip(c: &mut Criterion) {
    let protocol = FileTransferProtocol::new();
    let mut codec = FileTransferCodec;
    let response = ProtocolResponse::ChunkResponse {
        transfer_id: "bench".to_string(),
        chunk_index: 1,
        success: true,
        error: None,
    };

    c.bench_function("codec_response_roundtrip_small", |b| {
        b.iter(|| {
            let mut buf = Vec::new();
            // write phase
            futures::executor::block_on(async {
                let mut w = Cursor::new(&mut buf);
                codec
                    .write_response(&protocol, &mut w, response.clone())
                    .await
                    .unwrap();
            });
            // read phase
            futures::executor::block_on(async {
                let mut r = Cursor::new(&buf[..]);
                let _decoded = codec.read_response(&protocol, &mut r).await.unwrap();
                black_box(());
            });
        })
    });
}

criterion_group!(benches, bench_codec_response_roundtrip);
criterion_main!(benches);
