import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { test } from "node:test";

import {
  archiveName,
  bundledPackageNames,
  expectedDigest,
  isMusl,
  packageVersion,
  releaseUrls,
  resolveBundledBinary,
  verifyChecksum,
} from "./install.js";

const manifest = JSON.parse(fs.readFileSync(new URL("./package.json", import.meta.url), "utf8"));

test("packageVersion is the version in package.json", () => {
  assert.equal(packageVersion(), manifest.version);
});

test("archiveName follows the release asset naming for every shipped triple", () => {
  assert.equal(archiveName("2.7.3", "aarch64-apple-darwin"), "liter-llm-2.7.3-aarch64-apple-darwin.tar.gz");
  assert.equal(archiveName("2.7.3", "x86_64-unknown-linux-gnu"), "liter-llm-2.7.3-x86_64-unknown-linux-gnu.tar.gz");
  assert.equal(archiveName("2.7.3", "x86_64-pc-windows-msvc"), "liter-llm-2.7.3-x86_64-pc-windows-msvc.zip");
});

test("releaseUrls pin the exact tag, never latest", () => {
  const { archive, archiveUrl, checksumsUrl } = releaseUrls("2.7.3", "aarch64-apple-darwin");
  assert.equal(archive, "liter-llm-2.7.3-aarch64-apple-darwin.tar.gz");
  assert.equal(
    archiveUrl,
    "https://github.com/xberg-io/liter-llm/releases/download/v2.7.3/liter-llm-2.7.3-aarch64-apple-darwin.tar.gz",
  );
  assert.equal(checksumsUrl, "https://github.com/xberg-io/liter-llm/releases/download/v2.7.3/SHA256SUMS-2.7.3.txt");
  assert.ok(!archiveUrl.includes("latest") && !checksumsUrl.includes("latest"));
});

test("releaseUrls keep a prerelease version in tag, archive and checksum names", () => {
  const { archiveUrl, checksumsUrl } = releaseUrls("2.8.0-rc.1", "x86_64-unknown-linux-gnu");
  assert.ok(archiveUrl.includes("/download/v2.8.0-rc.1/liter-llm-2.8.0-rc.1-x86_64-unknown-linux-gnu.tar.gz"));
  assert.ok(checksumsUrl.endsWith("/v2.8.0-rc.1/SHA256SUMS-2.8.0-rc.1.txt"));
});

const payload = Buffer.from("liter-llm archive bytes");
const digest = crypto.createHash("sha256").update(payload).digest("hex");
const NAME = "liter-llm-2.7.3-x86_64-unknown-linux-gnu.tar.gz";

test("expectedDigest matches the asset name exactly and tolerates binary-mode markers", () => {
  const sums = `${"a".repeat(64)}  other-${NAME}\n${digest} *${NAME}\n`;
  assert.equal(expectedDigest(sums, NAME), digest);
  assert.equal(expectedDigest(sums, "missing.tar.gz"), null);
});

test("verifyChecksum accepts a matching digest", () => {
  assert.doesNotThrow(() => verifyChecksum(payload, NAME, `${digest}  ${NAME}\n`));
});

test("verifyChecksum rejects a tampered archive", () => {
  assert.throws(() => verifyChecksum(Buffer.from("tampered"), NAME, `${digest}  ${NAME}\n`), /checksum mismatch/u);
});

test("verifyChecksum refuses an archive with no checksum entry", () => {
  assert.throws(() => verifyChecksum(payload, NAME, `${digest}  another.tar.gz\n`), /no checksum entry/u);
});

const HERE = path.dirname(new URL(import.meta.url).pathname);

function fakePlatformPackage(root, name, binaryName) {
  const dir = path.join(root, "node_modules", ...name.split("/"));
  fs.mkdirSync(path.join(dir, "bin"), { recursive: true });
  fs.writeFileSync(path.join(dir, "package.json"), JSON.stringify({ name, version: "0.0.0" }));
  const binary = path.join(dir, "bin", binaryName);
  fs.writeFileSync(binary, '#!/bin/sh\necho bundled-binary "$@"\n');
  fs.chmodSync(binary, 0o755);
  return binary;
}

function launcherSandbox(root) {
  fs.mkdirSync(path.join(root, "bin"));
  fs.copyFileSync(path.join(HERE, "package.json"), path.join(root, "package.json"));
  fs.copyFileSync(path.join(HERE, "bin", "liter-llm.js"), path.join(root, "bin", "liter-llm.js"));
  const installSource = fs.readFileSync(path.join(HERE, "install.js"), "utf8");
  const marker = "export async function main() {";
  assert.ok(installSource.includes(marker), "install.js main() signature changed; update this test");
  fs.writeFileSync(
    path.join(root, "install.js"),
    installSource.replace(marker, `${marker}\n  throw new Error("download attempted");`),
  );
}

test("every optionalDependency is a bundled package name and pinned to the package version", () => {
  const bundled = new Set(
    [
      ["linux", "x64"],
      ["linux", "arm64"],
      ["darwin", "x64"],
      ["darwin", "arm64"],
      ["win32", "x64"],
      ["win32", "arm64"],
    ].flatMap(([platform, arch]) => bundledPackageNames(platform, arch, false)),
  );
  assert.deepEqual(new Set(Object.keys(manifest.optionalDependencies)), bundled);
  for (const version of Object.values(manifest.optionalDependencies)) assert.equal(version, manifest.version);
});

test("bundledPackageNames maps glibc linux, mac and windows and is empty for musl and unknown platforms", () => {
  const scope = "@xberg-io/liter-llm-cli";
  assert.deepEqual(bundledPackageNames("linux", "x64", false), [`${scope}-linux-x64-gnu`]);
  assert.deepEqual(bundledPackageNames("linux", "x64", true), []);
  assert.deepEqual(bundledPackageNames("darwin", "arm64", false), [`${scope}-darwin-arm64`]);
  assert.deepEqual(bundledPackageNames("win32", "arm64", false), [`${scope}-win32-arm64`]);
  assert.deepEqual(bundledPackageNames("freebsd", "x64", false), []);
});

test("isMusl reads the glibc runtime from the process report", () => {
  assert.equal(isMusl({ platform: "linux", report: () => ({ header: { glibcVersionRuntime: "2.39" } }) }), false);
  assert.equal(isMusl({ platform: "linux", report: () => ({ header: {} }) }), true);
  assert.equal(isMusl({ platform: "darwin", report: () => ({ header: {} }) }), false);
});

test("resolveBundledBinary returns the platform package binary and null when it is not installed", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "ll-bundled-"));
  try {
    const gnu = fakePlatformPackage(root, "@xberg-io/liter-llm-cli-linux-x64-gnu", "liter-llm");
    const resolve = (id) => {
      const file = path.join(root, "node_modules", ...id.split("/"));
      if (!fs.existsSync(file)) throw new Error(`Cannot find module ${id}`);
      return file;
    };
    assert.equal(resolveBundledBinary({ platform: "linux", arch: "x64", musl: false, resolve }), gnu);
    assert.equal(resolveBundledBinary({ platform: "linux", arch: "x64", musl: true, resolve }), null);
    assert.equal(resolveBundledBinary({ platform: "linux", arch: "arm64", musl: false, resolve }), null);
    assert.equal(resolveBundledBinary({ platform: "darwin", arch: "arm64", musl: false, resolve }), null);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test(
  "the launcher runs the bundled platform package and never reaches the download path",
  { skip: process.platform === "win32" || isMusl() },
  () => {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), "ll-launcher-"));
    try {
      launcherSandbox(root);
      const [pkg] = bundledPackageNames();
      assert.ok(pkg, `no bundled package is defined for ${process.platform}-${process.arch}`);
      fakePlatformPackage(root, pkg, "liter-llm");
      const result = spawnSync(process.execPath, [path.join(root, "bin", "liter-llm.js"), "--flag", "x"], {
        encoding: "utf8",
      });
      assert.equal(result.stdout.trim(), "bundled-binary --flag x");
      assert.equal(result.status, 0);
      assert.doesNotMatch(result.stderr, /download/);
    } finally {
      fs.rmSync(root, { recursive: true, force: true });
    }
  },
);

test(
  "the launcher falls through to the download path when no platform package is installed",
  { skip: process.platform === "win32" },
  () => {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), "ll-launcher-"));
    try {
      launcherSandbox(root);
      const result = spawnSync(process.execPath, [path.join(root, "bin", "liter-llm.js")], { encoding: "utf8" });
      assert.equal(result.status, 1);
      assert.match(result.stderr, /attempting download/);
      assert.match(result.stderr, /download attempted/);
    } finally {
      fs.rmSync(root, { recursive: true, force: true });
    }
  },
);
