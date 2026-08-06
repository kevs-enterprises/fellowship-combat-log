import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { test } from "node:test";

import { version } from "../dist/index.js";

test("version() reports the underlying crate's Cargo.toml version", async () => {
  const cargoToml = readFileSync(
    fileURLToPath(new URL("../../rust/Cargo.toml", import.meta.url)),
    "utf8",
  );
  const match = cargoToml.match(/^version\s*=\s*"([^"]+)"/m);
  assert.ok(match, "expected to find a version field in rust/Cargo.toml");

  assert.equal(await version(), match[1]);
});
