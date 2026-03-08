import type { BinaryChunk, InitOptions, InputChunk, SseEvent } from "./types.ts";
import { loadBindings, type WasmParserHandle } from "./wasm-loader.ts";

function toUint8Array(chunk: BinaryChunk): Uint8Array {
  if (chunk instanceof Uint8Array) {
    return chunk;
  }

  if (ArrayBuffer.isView(chunk)) {
    return new Uint8Array(chunk.buffer, chunk.byteOffset, chunk.byteLength);
  }

  return new Uint8Array(chunk);
}

function toEvents(value: unknown): SseEvent[] {
  return value as SseEvent[];
}

export async function init(options?: InitOptions): Promise<void> {
  await loadBindings(options);
}

export class SseParser {
  private readonly inner: WasmParserHandle;

  private constructor(inner: WasmParserHandle) {
    this.inner = inner;
  }

  static async create(options?: InitOptions): Promise<SseParser> {
    const bindings = await loadBindings(options);
    return new SseParser(new bindings.SseParser());
  }

  feed(chunk: InputChunk): SseEvent[] {
    if (typeof chunk === "string") {
      return toEvents(this.inner.feedText(chunk));
    }

    return toEvents(this.inner.feedBytes(toUint8Array(chunk)));
  }

  finish(): SseEvent[] {
    return toEvents(this.inner.finish());
  }

  reset(): void {
    this.inner.reset();
  }

  get lastEventId(): string | undefined {
    return this.inner.lastEventId;
  }

  get retry(): number | undefined {
    return this.inner.retry;
  }
}

export async function parse(chunk: InputChunk, options?: InitOptions): Promise<SseEvent[]> {
  const parser = await SseParser.create(options);
  const events = parser.feed(chunk);
  return [...events, ...parser.finish()];
}
