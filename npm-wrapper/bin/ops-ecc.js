#!/usr/bin/env node
/**
 * Thin zero-overhead CLI launcher for `npx ops-ecc`.
 */
const fs = require("fs");
const os = require("os");
const path = require("path");
const { spawnSync } = require("child_process");

const binName = os.platform() === "win32" ? "ops-ecc.exe" : "ops-ecc";
const candidates = [
  path.join(__dirname, "..", "vendor", binName),
  path.resolve(__dirname, "..", "..", "target", "release", binName),
  path.join(os.homedir(), ".cargo", "bin", binName),
  binName
];

let resolved = binName;
for (const candidate of candidates) {
  if (candidate !== binName && fs.existsSync(candidate)) {
    resolved = candidate;
    break;
  }
}

const result = spawnSync(resolved, process.argv.slice(2), {
  stdio: "inherit",
  env: process.env
});

if (result.error) {
  console.error(
    `[ops-ecc] Failed to execute binary (${resolved}): ${result.error.message}\n` +
      `Build or install via Rust Cargo: cargo install --path .`
  );
  process.exit(1);
}

process.exit(result.status ?? 0);
