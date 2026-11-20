#!/usr/bin/env node
/**
 * Post-install script for `ops-ecc` npm wrapper.
 * Resolves platform/architecture and either locates an existing local/cargo build
 * or downloads the prebuilt release binary from GitHub Releases.
 */
const fs = require("fs");
const path = require("path");
const os = require("os");
const https = require("https");
const { execSync } = require("child_process");

const pkg = require("./package.json");
const VERSION = process.env.OPS_ECC_VERSION || `v${pkg.version}`;
const REPO = process.env.OPS_ECC_REPO || "bilal-talha-dev/ops-ecc";

function getPlatformArchive() {
  const platform = os.platform();
  const arch = os.arch();

  if (platform === "win32" && arch === "x64") {
    return { archive: "ops-ecc-windows-x86_64.zip", bin: "ops-ecc.exe" };
  }
  if (platform === "darwin" && arch === "arm64") {
    return { archive: "ops-ecc-macos-arm64.tar.gz", bin: "ops-ecc" };
  }
  if (platform === "darwin" && arch === "x64") {
    return { archive: "ops-ecc-macos-x86_64.tar.gz", bin: "ops-ecc" };
  }
  if (platform === "linux" && arch === "arm64") {
    return { archive: "ops-ecc-linux-arm64.tar.gz", bin: "ops-ecc" };
  }
  if (platform === "linux" && arch === "x64") {
    return { archive: "ops-ecc-linux-x86_64.tar.gz", bin: "ops-ecc" };
  }
  throw new Error(`Unsupported platform/arch combination: ${platform}/${arch}`);
}

function findLocalBinary(binName) {
  const cargoBin = path.join(os.homedir(), ".cargo", "bin", binName);
  if (fs.existsSync(cargoBin)) return cargoBin;

  const localRelease = path.resolve(__dirname, "..", "target", "release", binName);
  if (fs.existsSync(localRelease)) return localRelease;

  return null;
}

function downloadFile(url, dest) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);
    https
      .get(url, (res) => {
        if (res.statusCode === 301 || res.statusCode === 302) {
          file.close();
          return downloadFile(res.headers.location, dest).then(resolve).catch(reject);
        }
        if (res.statusCode !== 200) {
          file.close();
          return reject(new Error(`HTTP ${res.statusCode} when fetching ${url}`));
        }
        res.pipe(file);
        file.on("finish", () => file.close(resolve));
      })
      .on("error", (err) => {
        fs.unlink(dest, () => {});
        reject(err);
      });
  });
}

async function main() {
  const { archive, bin } = getPlatformArchive();
  const vendorDir = path.join(__dirname, "vendor");
  fs.mkdirSync(vendorDir, { recursive: true });
  const targetBin = path.join(vendorDir, bin);

  const existing = findLocalBinary(bin);
  if (existing) {
    fs.copyFileSync(existing, targetBin);
    if (os.platform() !== "win32") fs.chmodSync(targetBin, 0o755);
    console.log(`[ops-ecc] Linked existing native binary from ${existing}`);
    return;
  }

  const url = `https://github.com/${REPO}/releases/download/${VERSION}/${archive}`;
  const archivePath = path.join(vendorDir, archive);
  console.log(`[ops-ecc] Attempting download of prebuilt binary: ${url}`);

  try {
    await downloadFile(url, archivePath);
    if (archive.endsWith(".tar.gz")) {
      execSync(`tar -xzf "${archivePath}" -C "${vendorDir}"`, { stdio: "inherit" });
    } else if (archive.endsWith(".zip")) {
      execSync(
        `powershell -NoProfile -Command "Expand-Archive -Force '${archivePath}' '${vendorDir}'"`,
        { stdio: "inherit" }
      );
    }
    fs.unlinkSync(archivePath);
    if (os.platform() !== "win32") fs.chmodSync(targetBin, 0o755);
    console.log(`[ops-ecc] Installed native binary to ${targetBin}`);
  } catch (err) {
    console.warn(
      `[ops-ecc] Prebuilt release binary not downloaded (${err.message}). Falling back to system PATH / cargo install.`
    );
  }
}

main();
