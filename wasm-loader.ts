import type { InitOptions, WasmSource } from "./types.ts";

export interface WasmParserHandle {
  feedBytes(chunk: Uint8Array): unknown;
  feedText(chunk: string): unknown;
  finish(): unknown;
  reset(): void;
  readonly lastEventId?: string;
  readonly retry?: number;
}

interface WasmBindings {
  default(input?: { module_or_path?: WasmSource | Promise<WasmSource> }): Promise<unknown>;
  SseParser: new () => WasmParserHandle;
}

const WASM_MODULE_URL = new URL("./wasm/pkg/sse_parser.js", import.meta.url);
const WASM_BINARY_URL = new URL("./wasm/pkg/sse_parser_bg.wasm", import.meta.url);

let wasmBindingsPromise: Promise<WasmBindings> | undefined;

function isNodeRuntime(): boolean {
  return (
    typeof process !== "undefined" &&
    typeof process.versions === "object" &&
    typeof process.versions.node === "string"
  );
}

async function defaultWasmSource(): Promise<WasmSource> {
  if (!isNodeRuntime()) {
    return WASM_BINARY_URL;
  }

  const { readFile } = await import("node:fs/promises");
  return readFile(WASM_BINARY_URL);
}

export async function loadBindings(options: InitOptions = {}): Promise<WasmBindings> {
  if (wasmBindingsPromise) {
    return wasmBindingsPromise;
  }

  wasmBindingsPromise = (async (): Promise<WasmBindings> => {
    const bindings = (await import(WASM_MODULE_URL.href)) as WasmBindings;
    await bindings.default({
      module_or_path: options.wasm ?? (await defaultWasmSource()),
    });
    return bindings;
  })();

  return wasmBindingsPromise;
}
