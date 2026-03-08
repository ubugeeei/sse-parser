# sse-parser

A streaming SSE parser written in Rust. It builds for `wasm32-unknown-unknown` and works in both browsers and Node.js 24+.

## Features

- Incremental parsing directly from byte streams
- Correct handling of `\n`, `\r`, and `\r\n`
- Spec-compliant parsing of `id`, `event`, `data`, and `retry`
- WebAssembly bindings built with `wasm-bindgen`
- TypeScript wrapper and Vitest coverage
- `insta` snapshot tests and Rust benchmarks

## Requirements

- Rust
- `wasm32-unknown-unknown` target
- `wasm-bindgen-cli`
- Node.js 24+
- pnpm

## Build

```bash
pnpm install
pnpm build
```

## Test

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo bench
pnpm test
```

## Benchmarks

```bash
cargo bench --bench parser
pnpm run bench:wasm
```

Current benchmark coverage:

- Rust core: single event in one chunk
- Rust core: streamed event across chunks
- Rust core: large batch in one chunk
- Rust core: large batch fragmented into small chunks
- wasm wrapper: single event
- wasm wrapper: large batch
- wasm wrapper: fragmented large batch

## Performance Notes

- For the Rust API, prefer [`Parser::feed_into`](./src/parser.rs) and reuse the output `Vec<Event>` when parsing many chunks.
- The Rust hot path uses `memchr`-based line scanning to avoid byte-by-byte delimiter checks.
- The wasm benchmark measures parser reuse with `reset()` to isolate parsing throughput from module initialization cost.

## TypeScript Usage

```ts
import { SseParser } from "sse-parser";

const parser = await SseParser.create();
const events = parser.feed(new TextEncoder().encode("data: hello\n\n"));

console.log(events);
```
