#!/usr/bin/env node
import { spawnSync } from "node:child_process";

const [task, ...extraArgs] = process.argv.slice(2);

if (!task) {
  console.error("usage: node scripts/turbo-guard.mjs <task> [...turbo args]");
  process.exit(2);
}

const pnpm = process.platform === "win32" ? "pnpm.cmd" : "pnpm";
const result = spawnSync(
  pnpm,
  ["exec", "turbo", "run", task, ...extraArgs],
  {
    cwd: process.cwd(),
    encoding: "utf8",
    stdio: ["inherit", "pipe", "pipe"],
  },
);

if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);

if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}

if (result.status !== 0) {
  process.exit(result.status ?? 1);
}

const output = `${result.stdout ?? ""}\n${result.stderr ?? ""}`;
const ranZeroTasks =
  /\bTasks:\s+0 successful,\s+0 total\b/.test(output) ||
  /\bNo tasks were executed as part of this run\./.test(output);

if (ranZeroTasks) {
  console.error(
    `turbo run ${task} completed without executing any tasks; add workspace scripts or remove this release gate.`,
  );
  process.exit(1);
}
