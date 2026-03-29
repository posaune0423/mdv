#!/usr/bin/env node
import fs from "node:fs";

const pkg = JSON.parse(fs.readFileSync("package.json", "utf8"));
let cargo = fs.readFileSync("Cargo.toml", "utf8");
const re = /^version = "[^"]+"/m;
if (!re.test(cargo)) {
  throw new Error("Cargo.toml: missing version = \"...\" line");
}
const next = cargo.replace(re, `version = "${pkg.version}"`);
if (next === cargo) {
  process.stdout.write(`Cargo.toml already at ${pkg.version}\n`);
  process.exit(0);
}
fs.writeFileSync("Cargo.toml", next);
process.stdout.write(`Cargo.toml version -> ${pkg.version}\n`);
