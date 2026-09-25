#!/usr/bin/env node
// Vérifie que les archives de mise à jour ont été signées avec la clé dont la clé publique est
// embarquée dans l'application (plugins.updater.pubkey de src-tauri/tauri.conf.json). Sinon, les
// applications installées refuseraient la mise à jour : autant échouer ici.
//
// Signature et clé publique minisign : ligne 2 en base64, octets 2 à 9 = identifiant de la clé.
// Les fichiers .sig et la pubkey de Tauri sont eux-mêmes encodés en base64.
//
// Usage : node scripts/check-updater-key.mjs [dossier]   (défaut : target)

import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

const keyId = (b64) => {
  const text = Buffer.from(b64.trim(), "base64").toString("utf8");
  const line = text.split(/\r?\n/)[1];
  if (!line) throw new Error("format minisign inattendu");
  // Stocké en petit-boutiste ; minisign l'affiche octets inversés (« public key: 3D25… »).
  return Buffer.from(Buffer.from(line.trim(), "base64").subarray(2, 10)).reverse().toString("hex").toUpperCase();
};

const conf = JSON.parse(readFileSync(new URL("../src-tauri/tauri.conf.json", import.meta.url), "utf8"));
const expected = keyId(conf.plugins.updater.pubkey);

function* sigs(dir) {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) yield* sigs(p);
    else if (name.endsWith(".sig") && p.includes("bundle")) yield p;
  }
}

const root = process.argv[2] ?? "target";
let n = 0;
let bad = 0;
for (const p of sigs(root)) {
  n++;
  const got = keyId(readFileSync(p, "utf8"));
  if (got === expected) console.log(`ok  ${p} (clé ${got})`);
  else {
    bad++;
    console.log(`::error title=Clé de mise à jour::${p} est signé avec la clé ${got}, l'application attend ${expected} (voir docs/PACKAGING.md §9.1)`);
  }
}
if (!n) {
  console.log(`::error title=Clé de mise à jour::aucune signature .sig trouvée sous ${root}`);
  process.exit(1);
}
process.exit(bad ? 1 : 0);
