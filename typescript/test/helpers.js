// Shared test helpers: fixture-file access.
//
// Fixtures live in the sibling `rust` crate's own test suite
// (`public/rust/tests/fixtures/*.log`) rather than being duplicated here, so this package's
// tests always decode exactly what the Rust crate's own tests do.

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const FIXTURES_DIR = fileURLToPath(
  new URL("../../rust/tests/fixtures/", import.meta.url),
);

export function fixtureText(name) {
  return readFileSync(FIXTURES_DIR + name, "utf8");
}

/** The fixture's non-empty lines, 1-based `seq` paired with each line's text. */
export function fixtureLines(name) {
  return fixtureText(name)
    .split("\n")
    .map((line, index) => ({ seq: index + 1, line }))
    .filter(({ line }) => line.length > 0);
}
