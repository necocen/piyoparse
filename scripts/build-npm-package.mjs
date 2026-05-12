#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import {
  cpSync,
  mkdirSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const pkgDir = join(repoRoot, "pkg");
const nodeBuildDir = join(repoRoot, "_build", "wasm-pack-node");
const nodePkgDir = join(pkgDir, "node");
const linkDir = parseLinkDir();

rmSync(pkgDir, { recursive: true, force: true });
run("wasm-pack", [
  "build",
  "--release",
  "--target",
  "bundler",
  "--scope",
  "necocen",
  "--out-dir",
  "pkg",
  "--features",
  "wasm",
]);

rmSync(nodeBuildDir, { recursive: true, force: true });
run("wasm-pack", [
  "build",
  "--release",
  "--target",
  "nodejs",
  "--scope",
  "necocen",
  "--out-dir",
  nodeBuildDir,
  "--features",
  "wasm",
]);

rmSync(nodePkgDir, { recursive: true, force: true });
mkdirSync(nodePkgDir, { recursive: true });

const nodeEntrypoint = readFileSync(join(nodeBuildDir, "piyoparse.js"), "utf8").replace(
  'const wasmPath = `${__dirname}/piyoparse_bg.wasm`;',
  "const wasmPath = require('path').join(__dirname, '..', 'piyoparse_bg.wasm');",
);
writeFileSync(join(nodePkgDir, "piyoparse.cjs"), nodeEntrypoint);
cpSync(join(nodeBuildDir, "piyoparse.d.ts"), join(nodePkgDir, "piyoparse.d.ts"));
cpSync(join(nodeBuildDir, "piyoparse.d.ts"), join(nodePkgDir, "piyoparse.d.cts"));

const packageJsonPath = join(pkgDir, "package.json");
const packageJson = JSON.parse(readFileSync(packageJsonPath, "utf8"));
packageJson.main = "node/piyoparse.cjs";
packageJson.module = "piyoparse.js";
packageJson.types = "piyoparse.d.ts";
packageJson.exports = {
  ".": {
    browser: {
      types: "./piyoparse.d.ts",
      default: "./piyoparse.js",
    },
    import: {
      types: "./piyoparse.d.ts",
      node: "./node/piyoparse.cjs",
      default: "./piyoparse.js",
    },
    require: {
      types: "./node/piyoparse.d.cts",
      default: "./node/piyoparse.cjs",
    },
    default: "./piyoparse.js",
  },
  "./bundler": {
    types: "./piyoparse.d.ts",
    default: "./piyoparse.js",
  },
  "./node": {
    types: "./node/piyoparse.d.cts",
    default: "./node/piyoparse.cjs",
  },
  "./package.json": "./package.json",
};
packageJson.files = [
  "piyoparse_bg.wasm",
  "piyoparse.js",
  "piyoparse_bg.js",
  "piyoparse.d.ts",
  "piyoparse_bg.wasm.d.ts",
  "node/",
];
writeFileSync(packageJsonPath, `${JSON.stringify(packageJson, null, 2)}\n`);

if (linkDir !== null) {
  const scopedPackageDir = join(linkDir, "@necocen");
  const packageLink = join(scopedPackageDir, "piyoparse");
  mkdirSync(scopedPackageDir, { recursive: true });
  rmSync(packageLink, { recursive: true, force: true });
  symlinkSync(pkgDir, packageLink, "dir");
}

function parseLinkDir() {
  if (process.argv.includes("--link")) {
    return join(repoRoot, "node_modules");
  }

  const linkDirIndex = process.argv.indexOf("--link-dir");
  if (linkDirIndex === -1) {
    return null;
  }

  const value = process.argv[linkDirIndex + 1];
  if (!value) {
    throw new Error("--link-dir requires a directory path");
  }

  return resolve(repoRoot, value);
}

function run(command, args) {
  const result = spawnSync(command, args, {
    cwd: repoRoot,
    stdio: "inherit",
  });

  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}
