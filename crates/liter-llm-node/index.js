"use strict";

const { platform, arch } = process;
const isMusl = () => {
  // Prefer the report-header `glibcVersion` string when present — fastest and
  // environments leave `glibcVersion` undefined even on glibc systems, so the
  // `=== undefined` branch from older napi-rs templates produces a false
  if (typeof process.report === "object" && typeof process.report.getReport === "function") {
    const report = process.report.getReport();
    if (report && report.header && typeof report.header.glibcVersion === "string") {
      return false;
    }
  }
  try {
    require("fs").statSync("/lib64/ld-musl-x86_64.so.1");
    return true;
  } catch {
    return false;
  }
};

let nativeBinding = null;
const loadErrors = [];

function requireOptionalDependency(name) {
  try {
    return require(name);
  } catch (e) {
    loadErrors.push(`Optional dependency ${name}: ${e.message}`);
    return null;
  }
}

const tryLoadBinding = () => {
  const targets = [
    ["linux", "x64", "gnu", "./liter-llm-node.linux-x64-gnu.node", "@xberg-io/liter-llm-linux-x64-gnu"],
    ["linux", "arm64", "gnu", "./liter-llm-node.linux-arm64-gnu.node", "@xberg-io/liter-llm-linux-arm64-gnu"],
    ["darwin", "x64", null, "./liter-llm-node.darwin-x64.node", "@xberg-io/liter-llm-darwin-x64"],
    ["darwin", "arm64", null, "./liter-llm-node.darwin-arm64.node", "@xberg-io/liter-llm-darwin-arm64"],
    ["win32", "x64", null, "./liter-llm-node.win32-x64-msvc.node", "@xberg-io/liter-llm-win32-x64-msvc"],
    ["win32", "arm64", null, "./liter-llm-node.win32-arm64-msvc.node", "@xberg-io/liter-llm-win32-arm64-msvc"],
  ];

  for (const [plat, a, abi, localPath, optionalDep] of targets) {
    if (platform !== plat || arch !== a) {
      continue;
    }

    if (plat === "linux" && abi) {
      const isCurMusl = isMusl();
      if ((abi === "musl") !== isCurMusl) {
        continue;
      }
    }

    try {
      nativeBinding = require(localPath);
      if (nativeBinding) {
        return;
      }
    } catch (e) {
      loadErrors.push(e.message);
    }

    try {
      const optBinding = requireOptionalDependency(optionalDep);
      if (optBinding) {
        nativeBinding = optBinding;
        return;
      }
    } catch (e) {
      loadErrors.push(e.message);
    }
  }
};

tryLoadBinding();

if (!nativeBinding) {
  throw new Error(`Failed to load native binding for ${platform}-${arch}. Errors: ${loadErrors.join(", ")}`);
}

module.exports = nativeBinding;
