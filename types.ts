export interface SseEvent {
  event: string;
  data: string;
  id?: string;
}

export type BinaryChunk = Uint8Array | ArrayBuffer | ArrayBufferView;
export type InputChunk = string | BinaryChunk;
export type WasmSource = BufferSource | Request | Response | URL | string | WebAssembly.Module;

export interface InitOptions {
  wasm?: WasmSource;
}
