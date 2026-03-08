import { hrtime } from "node:process";

import { SseParser } from "../dist/index.js";

interface BenchCase {
  name: string;
  bytes: number;
  iterations: number;
  run(parser: SseParser): number;
}

const encoder = new TextEncoder();

const singleEvent = encoder.encode("event: ping\ndata: hello\n\n");
const largeBatch = buildLargeBatchPayload();
const fragmentedLargeBatch = chunkBytes(largeBatch, 32);

const benchCases: BenchCase[] = [
  {
    name: "wasm_single_event",
    bytes: singleEvent.byteLength,
    iterations: 200_000,
    run(parser) {
      return parser.feed(singleEvent).length + parser.finish().length;
    },
  },
  {
    name: "wasm_large_batch",
    bytes: largeBatch.byteLength,
    iterations: 5_000,
    run(parser) {
      return parser.feed(largeBatch).length + parser.finish().length;
    },
  },
  {
    name: "wasm_fragmented_large_batch",
    bytes: largeBatch.byteLength,
    iterations: 2_000,
    run(parser) {
      let events = 0;

      for (const chunk of fragmentedLargeBatch) {
        events += parser.feed(chunk).length;
      }

      return events + parser.finish().length;
    },
  },
];

const parser = await SseParser.create();

for (const benchCase of benchCases) {
  runWarmup(parser, benchCase);
  const result = runBench(parser, benchCase);

  console.log(
    [
      benchCase.name.padEnd(28),
      `ops/s=${formatNumber(result.opsPerSecond).padStart(12)}`,
      `MiB/s=${formatNumber(result.mebibytesPerSecond).padStart(12)}`,
      `ns/op=${formatNumber(result.nanosecondsPerOperation).padStart(12)}`,
      `events=${String(result.totalEvents).padStart(8)}`,
    ].join("  ")
  );
}

function runWarmup(parser: SseParser, benchCase: BenchCase): void {
  for (let index = 0; index < 200; index += 1) {
    parser.reset();
    benchCase.run(parser);
  }
}

function runBench(
  parser: SseParser,
  benchCase: BenchCase
): {
  opsPerSecond: number;
  mebibytesPerSecond: number;
  nanosecondsPerOperation: number;
  totalEvents: number;
} {
  let totalEvents = 0;
  const start = hrtime.bigint();

  for (let index = 0; index < benchCase.iterations; index += 1) {
    parser.reset();
    totalEvents += benchCase.run(parser);
  }

  const end = hrtime.bigint();
  const totalNanoseconds = Number(end - start);
  const totalSeconds = totalNanoseconds / 1_000_000_000;
  const opsPerSecond = benchCase.iterations / totalSeconds;
  const mebibytesPerSecond =
    (benchCase.bytes * benchCase.iterations) / (1024 * 1024) / totalSeconds;
  const nanosecondsPerOperation = totalNanoseconds / benchCase.iterations;

  return {
    opsPerSecond,
    mebibytesPerSecond,
    nanosecondsPerOperation,
    totalEvents,
  };
}

function buildLargeBatchPayload(): Uint8Array {
  const chunks: Uint8Array[] = [];
  let totalLength = 0;

  for (let index = 0; index < 1024; index += 1) {
    const chunk = encoder.encode(`id: ${index}\nevent: update\ndata: hello\ndata: world\n\n`);
    chunks.push(chunk);
    totalLength += chunk.byteLength;
  }

  const payload = new Uint8Array(totalLength);
  let offset = 0;

  for (const chunk of chunks) {
    payload.set(chunk, offset);
    offset += chunk.byteLength;
  }

  return payload;
}

function chunkBytes(source: Uint8Array, chunkSize: number): Uint8Array[] {
  const chunks: Uint8Array[] = [];

  for (let offset = 0; offset < source.byteLength; offset += chunkSize) {
    chunks.push(source.subarray(offset, offset + chunkSize));
  }

  return chunks;
}

function formatNumber(value: number): string {
  return new Intl.NumberFormat("en-US", {
    maximumFractionDigits: 2,
    minimumFractionDigits: 2,
  }).format(value);
}
