import { execFileSync, spawnSync } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import https from "node:https";
import os from "node:os";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const REPO = "xberg-io/liter-llm";
const BIN_NAME = "liter-llm";
const PKG_NAME = "liter-llm-cli";
const USER_AGENT = "liter-llm-cli-npm-proxy";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const BIN_DIR = path.join(__dirname, "bin");

// ~keep The package is published in lockstep with the product, so its own version names the one
// release asset it may install. Never resolve "latest": a newer binary than the wrapper was tested with.
export function packageVersion() {
  const manifest = JSON.parse(fs.readFileSync(path.join(__dirname, "package.json"), "utf8"));
  if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/u.test(manifest.version)) {
    throw new Error(`package.json version '${manifest.version}' is not a release version`);
  }
  return manifest.version;
}

export function targetTriple() {
  const type = os.type();
  const arch = os.arch();

  if (type === "Windows_NT") {
    if (arch === "x64") return "x86_64-pc-windows-msvc";
    throw new Error(`unsupported Windows arch: ${arch}`);
  }
  if (type === "Linux") {
    if (arch === "x64") return "x86_64-unknown-linux-gnu";
    if (arch === "arm64") return "aarch64-unknown-linux-gnu";
    throw new Error(`unsupported Linux arch: ${arch}`);
  }
  if (type === "Darwin") {
    if (arch === "arm64") return "aarch64-apple-darwin";
    if (arch === "x64") return "x86_64-apple-darwin";
    throw new Error(`unsupported macOS arch: ${arch}`);
  }
  throw new Error(`unsupported platform: ${type} ${arch}`);
}

function binaryName() {
  return os.type() === "Windows_NT" ? `${BIN_NAME}.exe` : BIN_NAME;
}

function httpGetBuffer(url, { headers = {} } = {}, maxRedirects = 5) {
  return new Promise((resolve, reject) => {
    if (maxRedirects < 0) {
      reject(new Error("too many redirects"));
      return;
    }
    if (!/^https:\/\//iu.test(url)) {
      reject(new Error(`refusing non-https URL: ${url}`));
      return;
    }
    const req = https.get(url, { headers: { "User-Agent": USER_AGENT, ...headers } }, (res) => {
      if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
        res.resume();
        const next = res.headers.location;
        if (!/^https:\/\//iu.test(next)) {
          return reject(new Error(`refusing non-https redirect to: ${next}`));
        }
        return httpGetBuffer(next, { headers }, maxRedirects - 1).then(resolve, reject);
      }
      if (res.statusCode !== 200) {
        res.resume();
        return reject(new Error(`HTTP ${res.statusCode} for ${url}`));
      }
      const chunks = [];
      res.on("data", (c) => chunks.push(c));
      res.on("end", () => resolve(Buffer.concat(chunks)));
      res.on("error", reject);
    });
    req.on("error", reject);
    req.setTimeout(60000, () => {
      req.destroy();
      reject(new Error(`timeout for ${url}`));
    });
  });
}

export class CliUnavailableError extends Error {
  constructor(message) {
    super(message);
    this.name = "CliUnavailableError";
  }
}

export function archiveName(version, triple) {
  const extension = triple.endsWith("-windows-msvc") ? "zip" : "tar.gz";
  return `${BIN_NAME}-${version}-${triple}.${extension}`;
}

export function releaseUrls(version, triple) {
  const base = `https://github.com/${REPO}/releases/download/v${encodeURIComponent(version)}`;
  const archive = archiveName(version, triple);
  return { archive, archiveUrl: `${base}/${archive}`, checksumsUrl: `${base}/SHA256SUMS-${version}.txt` };
}

export function expectedDigest(text, assetName) {
  for (const raw of text.split(/\r?\n/u)) {
    const line = raw.trim();
    if (!line) continue;
    const parts = line.split(/\s+/u);
    if (parts.length < 2) continue;
    const name = parts.at(-1).replace(/^\*/u, "");
    if (name === assetName) return parts[0].toLowerCase();
  }
  return null;
}

export function verifyChecksum(archiveBuf, name, sumsText) {
  const expected = expectedDigest(sumsText, name);
  if (!expected) {
    throw new Error(`no checksum entry for ${name} in SHA256SUMS — refusing to install unverified binary`);
  }
  const actual = crypto.createHash("sha256").update(archiveBuf).digest("hex").toLowerCase();
  if (actual !== expected) {
    throw new Error(`checksum mismatch for ${name} (expected ${expected}, got ${actual})`);
  }
}

async function download(url, what) {
  try {
    return await httpGetBuffer(url);
  } catch (err) {
    if (/HTTP 404/u.test(err.message)) {
      throw new CliUnavailableError(`${what} not found at ${url}`);
    }
    throw err;
  }
}

function isUnsafeEntry(name) {
  const entry = String(name).replaceAll("\\", "/").trim();
  if (!entry) return false;
  if (entry.startsWith("/")) return true;
  if (/^[a-zA-Z]:/u.test(entry)) return true;
  if (entry.startsWith("//")) return true;
  return entry.split("/").some((part) => part === "..");
}

function listTarEntries(archivePath) {
  const result = spawnSync("tar", ["-tzf", archivePath]);
  if (result.status !== 0) {
    const stderr = result.stderr ? result.stderr.toString() : "";
    throw new Error(`tar listing failed: ${stderr || result.error}`);
  }
  return result.stdout
    .toString()
    .split(/\r?\n/u)
    .map((s) => s.trim())
    .filter(Boolean);
}

function extractTarGz(archivePath, destDir) {
  const result = spawnSync("tar", ["-xzf", archivePath, "-C", destDir]);
  if (result.status !== 0) {
    const stderr = result.stderr ? result.stderr.toString() : "";
    throw new Error(`tar extraction failed: ${stderr || result.error}`);
  }
}

function listZipEntries(archivePath) {
  if (os.type() === "Windows_NT") {
    const script =
      "$ErrorActionPreference='Stop';" +
      "Add-Type -AssemblyName System.IO.Compression.FileSystem;" +
      "[System.IO.Compression.ZipFile]::OpenRead($args[0]).Entries |" +
      " ForEach-Object { $_.FullName }";
    const out = execFileSync("powershell", ["-NoProfile", "-NonInteractive", "-Command", script, archivePath], {
      encoding: "utf8",
    });
    return out
      .split(/\r?\n/u)
      .map((s) => s.trim())
      .filter(Boolean);
  }
  const result = spawnSync("unzip", ["-Z1", archivePath]);
  if (result.status !== 0) {
    const stderr = result.stderr ? result.stderr.toString() : "";
    throw new Error(`zip listing failed: ${stderr || result.error}`);
  }
  return result.stdout
    .toString()
    .split(/\r?\n/u)
    .map((s) => s.trim())
    .filter(Boolean);
}

function extractZip(archivePath, destDir) {
  if (os.type() === "Windows_NT") {
    const result = spawnSync("powershell", [
      "-NoProfile",
      "-NonInteractive",
      "-Command",
      "Expand-Archive",
      "-LiteralPath",
      archivePath,
      "-DestinationPath",
      destDir,
      "-Force",
    ]);
    if (result.status !== 0) {
      const stderr = result.stderr ? result.stderr.toString() : "";
      throw new Error(`zip extraction failed: ${stderr || result.error}`);
    }
    return;
  }
  const result = spawnSync("unzip", ["-o", archivePath, "-d", destDir]);
  if (result.status !== 0) {
    const stderr = result.stderr ? result.stderr.toString() : "";
    throw new Error(`zip extraction failed: ${stderr || result.error}`);
  }
}

function findBinary(dir, name) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      const found = findBinary(full, name);
      if (found) return found;
    } else if (entry.name === name) {
      return full;
    }
  }
  return null;
}

function findDir(dir, name) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (!entry.isDirectory()) continue;
    if (entry.name === name) return path.join(dir, entry.name);
    const found = findDir(path.join(dir, entry.name), name);
    if (found) return found;
  }
  return null;
}

function safeExtract(archivePath, assetName, dest) {
  const isZip = assetName.toLowerCase().endsWith(".zip");
  const entries = isZip ? listZipEntries(archivePath) : listTarEntries(archivePath);
  for (const entry of entries) {
    if (isUnsafeEntry(entry)) {
      throw new Error(`refusing unsafe archive entry: ${entry}`);
    }
  }

  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), `${PKG_NAME}-x-`));
  try {
    if (isZip) {
      extractZip(archivePath, tmpDir);
    } else {
      extractTarGz(archivePath, tmpDir);
    }

    const binName = binaryName();
    const extractedBin = findBinary(tmpDir, binName);
    if (!extractedBin) {
      throw new CliUnavailableError(`archive ${assetName} did not contain expected CLI binary ${binName}`);
    }
    const finalBin = path.join(dest, binName);
    fs.copyFileSync(extractedBin, finalBin);

    const libDir = findDir(tmpDir, "lib");
    if (libDir) {
      fs.cpSync(libDir, path.join(dest, "lib"), { recursive: true });
    }
    return finalBin;
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
}

export async function main() {
  const binName = binaryName();
  const finalPath = path.join(BIN_DIR, binName);
  if (fs.existsSync(finalPath)) {
    try {
      const stat = fs.statSync(finalPath);
      const sizeOk = stat.size > 0;
      const execOk = os.type() === "Windows_NT" || (stat.mode & 0o111) !== 0;
      if (sizeOk && execOk) return;
    } catch {}
  }

  fs.mkdirSync(BIN_DIR, { recursive: true });

  const version = packageVersion();
  const triple = targetTriple();
  const { archive, archiveUrl, checksumsUrl } = releaseUrls(version, triple);
  process.stderr.write(`Downloading ${BIN_NAME} ${version} asset ${archive}...\n`);

  const archiveBuf = await download(archiveUrl, `${BIN_NAME} ${version} archive for ${triple}`);
  const sumsText = (await download(checksumsUrl, `checksums for ${BIN_NAME} ${version}`)).toString("utf8");
  verifyChecksum(archiveBuf, archive, sumsText);
  process.stderr.write(`Checksum verified for ${archive}.\n`);

  const stageDir = fs.mkdtempSync(path.join(os.tmpdir(), `${PKG_NAME}-dl-`));
  try {
    const archivePath = path.join(stageDir, path.basename(archive));
    fs.writeFileSync(archivePath, archiveBuf);
    safeExtract(archivePath, archive, BIN_DIR);
  } finally {
    fs.rmSync(stageDir, { recursive: true, force: true });
  }

  if (os.type() !== "Windows_NT") {
    fs.chmodSync(finalPath, 0o755);
  }
  process.stderr.write(`${BIN_NAME} installed.\n`);
}

if (import.meta.url === pathToFileURL(process.argv[1] || "").href) {
  main().catch((err) => {
    process.stderr.write(`Error installing ${BIN_NAME}: ${err.message}\n`);
    process.exit(1);
  });
}
