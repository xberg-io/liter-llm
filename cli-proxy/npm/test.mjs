import assert from "node:assert/strict";
import crypto from "node:crypto";
import fs from "node:fs";
import { test } from "node:test";

import { archiveName, expectedDigest, packageVersion, releaseUrls, verifyChecksum } from "./install.js";

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
