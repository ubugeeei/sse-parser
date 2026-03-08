use divan::{Bencher, black_box};
use sse_parser::Parser;
use std::sync::OnceLock;

const SINGLE_EVENT: &[u8] = b"event: ping\ndata: hello\n\n";
const STREAMED_EVENT_A: &[u8] = b"id: 42\nevent: update\ndata: alpha";
const STREAMED_EVENT_B: &[u8] = b"\ndata: beta\nretry: 1500\n\n";

fn main() {
    divan::main();
}

#[divan::bench]
fn parse_single_event(bencher: Bencher) {
    bencher.bench_local(|| {
        let mut parser = Parser::new();
        let events = parser.feed(black_box(SINGLE_EVENT)).unwrap();
        black_box(events);
    });
}

#[divan::bench]
fn parse_streamed_event(bencher: Bencher) {
    bencher.bench_local(|| {
        let mut parser = Parser::new();
        let first = parser.feed(black_box(STREAMED_EVENT_A)).unwrap();
        let second = parser.feed(black_box(STREAMED_EVENT_B)).unwrap();
        let final_events = parser.finish().unwrap();

        black_box((first, second, final_events));
    });
}

#[divan::bench]
fn parse_large_batch(bencher: Bencher) {
    let payload = large_batch_payload();

    bencher.bench_local(|| {
        let mut parser = Parser::new();
        let events = parser.feed(black_box(payload)).unwrap();
        black_box(events);
    });
}

#[divan::bench]
fn parse_fragmented_large_batch(bencher: Bencher) {
    let payload = large_batch_payload();

    bencher.bench_local(|| {
        let mut parser = Parser::new();
        let mut events = Vec::new();

        for chunk in payload.chunks(32) {
            parser.feed_into(black_box(chunk), &mut events).unwrap();
        }

        parser.finish_into(&mut events).unwrap();
        black_box(events);
    });
}

fn large_batch_payload() -> &'static [u8] {
    static PAYLOAD: OnceLock<Vec<u8>> = OnceLock::new();

    PAYLOAD
        .get_or_init(|| {
            let mut payload = Vec::with_capacity(64 * 1024);

            for index in 0..1024 {
                payload.extend_from_slice(b"id: ");
                payload.extend_from_slice(index.to_string().as_bytes());
                payload.extend_from_slice(b"\nevent: update\ndata: hello\ndata: world\n\n");
            }

            payload
        })
        .as_slice()
}
