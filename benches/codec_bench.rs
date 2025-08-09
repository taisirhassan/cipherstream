use cipherstream::file_transfer::{FileTransferCodec, FileTransferProtocol, ProtocolRequest};
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use futures::io::Cursor;
use libp2p::request_response::Codec;

fn bench_codec_request_roundtrip(c: &mut Criterion) {
    let protocol = FileTransferProtocol::new();
    let mut codec = FileTransferCodec;
    let request = ProtocolRequest::FileChunk {
        transfer_id: "bench".to_string(),
        chunk_index: 1,
        total_chunks: 10,
        data: vec![0x55; 1024 * 64],
        is_last: false,
    };

    c.bench_function("codec_request_roundtrip_64KB", |b| {
        b.iter(|| {
            let mut buf = Vec::new();
            // write phase
            futures::executor::block_on(async {
                let mut w = Cursor::new(&mut buf);
                codec
                    .write_request(&protocol, &mut w, request.clone())
                    .await
                    .unwrap();
            });
            // read phase
            futures::executor::block_on(async {
                let mut r = Cursor::new(&buf[..]);
                let _decoded = codec.read_request(&protocol, &mut r).await.unwrap();
                black_box(())
            });
        })
    });
}

criterion_group!(benches, bench_codec_request_roundtrip);
criterion_main!(benches);
