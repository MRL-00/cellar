#!/usr/bin/env node
import { spawnSync } from "node:child_process";

const entries = process.argv.slice(2);

if (entries.length === 0) {
  console.error("usage: node scripts/check-ts-entry.mjs <file.ts> [...file.ts]");
  process.exit(2);
}

const tsc = process.platform === "win32" ? "tsc.cmd" : "tsc";
const result = spawnSync(
  tsc,
  [
    "--noEmit",
    "--target",
    "ES2022",
    "--module",
    "ESNext",
    "--moduleResolution",
    "bundler",
    "--lib",
    "ES2022,DOM,DOM.Iterable",
    "--jsx",
    "react-jsx",
    "--strict",
    "--noUncheckedIndexedAccess",
    "--noImplicitOverride",
    "--isolatedModules",
    "--resolveJsonModule",
    "--skipLibCheck",
    "--forceConsistentCasingInFileNames",
    ...entries,
  ],
  {
    cwd: process.cwd(),
    stdio: "inherit",
  },
);

if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}

process.exit(result.status ?? 1);
