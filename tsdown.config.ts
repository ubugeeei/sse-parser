import { defineConfig } from "tsdown";

export default defineConfig({
  entry: ["index.ts"],
  format: "esm",
  platform: "neutral",
  target: "esnext",
  clean: true,
  sourcemap: true,
  dts: true,
  external: ["node:fs/promises"],
});
