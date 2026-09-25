#!/usr/bin/env node
// Vérifie que la version est la même dans package.json, Cargo.toml (workspace) et
// src-tauri/tauri.conf.json, et, si un tag est passé (`v0.2.0`), qu'elle lui correspond.
// Usage : node scripts/check-version.mjs [tag]
import { readFileSync } from "node:fs";

const pkg = JSON.parse(readFileSync("package.json", "utf8")).version;
const tauri = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8")).version;
const cargo = readFileSync("Cargo.toml", "utf8").match(/\[workspace\.package\][^[]*?\nversion\s*=\s*"([^"]+)"/)?.[1];

const found = { "package.json": pkg, "Cargo.toml": cargo, "src-tauri/tauri.conf.json": tauri };
const versions = new Set(Object.values(found));
let ok = true;
if (versions.size !== 1 || versions.has(undefined)) {
  console.error("Versions différentes :", found);
  ok = false;
}
const tag = process.argv[2];
if (tag) {
  const expected = tag.replace(/^v/, "");
  if (pkg !== expected) {
    console.error(`Le tag ${tag} ne correspond pas à la version ${pkg}. Mettez les trois fichiers à ${expected} avant de taguer.`);
    ok = false;
  }
}
if (!ok) process.exit(1);
console.log(`Version ${pkg} cohérente${tag ? ` avec le tag ${tag}` : ""}.`);
