#!/usr/bin/env node
import fs from "node:fs";

const pkg = JSON.parse(fs.readFileSync("package.json", "utf8"));
const cargo = fs.readFileSync("Cargo.toml", "utf8");
const m = cargo.match(/^version = "([^"]+)"/m);
if (!m) {
  console.error("Cargo.toml: could not parse version");
  process.exit(1);
}
if (m[1] !== pkg.version) {
  console.error(
    `Version mismatch: package.json "${pkg.version}" vs Cargo.toml "${m[1]}". Run \`node scripts/sync-cargo-version.mjs\` or align manually.`,
  );
  process.exit(1);
}
console.log(`OK: ${pkg.version}`);
