use sse_parser::Parser;

#[test]
fn snapshot_complex_frame() {
    let mut parser = Parser::new();
    let events = parser
        .feed(b": heartbeat\nevent: status\nid: 7\ndata: hello\ndata: world\nretry: 1500\n\n")
        .unwrap();

    insta::assert_debug_snapshot!((events, parser.last_event_id(), parser.retry()));
}
