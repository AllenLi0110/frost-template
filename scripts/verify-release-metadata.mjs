import { existsSync, readFileSync } from "node:fs";

function fail(message) {
  console.error(message);
  process.exit(1);
}

function readText(path) {
  if (!existsSync(path)) {
    fail(`Missing required release metadata file: ${path}`);
  }

  return readFileSync(path, "utf8");
}

function readJson(path) {
  try {
    return JSON.parse(readText(path));
  } catch (error) {
    fail(`Could not parse ${path}: ${error.message}`);
  }
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function parseArgs(argv) {
  const result = { tag: null };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];

    if (arg === "--tag") {
      result.tag = argv[index + 1] ?? null;
      index += 1;
      continue;
    }

    fail(`Unknown argument: ${arg}`);
  }

  return result;
}

const { tag } = parseArgs(process.argv.slice(2));
const version = readText("VERSION").trim();
const semverPattern = /^\d+\.\d+\.\d+(?:-[0-9A-Za-z](?:[0-9A-Za-z.-]*[0-9A-Za-z])?)?(?:\+[0-9A-Za-z](?:[0-9A-Za-z.-]*[0-9A-Za-z])?)?$/;

if (!semverPattern.test(version)) {
  fail(`VERSION must be a SemVer value, got: ${version}`);
}

if (tag !== null) {
  if (!tag.startsWith("v")) {
    fail(`Release tag must start with v, got: ${tag}`);
  }

  const tagVersion = tag.slice(1);

  if (tagVersion !== version) {
    fail(`Release tag ${tag} does not match VERSION ${version}`);
  }
}

const frontendPackage = readJson("frontend/package.json");
const frontendLock = readJson("frontend/package-lock.json");

if (frontendPackage.version !== version) {
  fail(`frontend/package.json version ${frontendPackage.version} does not match VERSION ${version}`);
}

if (frontendLock.version !== version) {
  fail(`frontend/package-lock.json version ${frontendLock.version} does not match VERSION ${version}`);
}

if (frontendLock.packages?.[""]?.version !== version) {
  fail(`frontend/package-lock.json root package version does not match VERSION ${version}`);
}

const cargoToml = readText("backend/Cargo.toml");
const workspacePackageMatch = cargoToml.match(/\[workspace\.package\][\s\S]*?version\s*=\s*"([^"]+)"/);

if (!workspacePackageMatch) {
  fail("backend/Cargo.toml is missing [workspace.package] version metadata");
}

if (workspacePackageMatch[1] !== version) {
  fail(`backend/Cargo.toml workspace version ${workspacePackageMatch[1]} does not match VERSION ${version}`);
}

const changelog = readText("CHANGELOG.md");
const changelogHeadingPattern = new RegExp(`^## \\[${escapeRegExp(version)}\\] - \\d{4}-\\d{2}-\\d{2}$`, "m");

if (!changelogHeadingPattern.test(changelog)) {
  fail(`CHANGELOG.md is missing a dated section for ${version}`);
}

readText("docs/release-process.md");

console.log(`Release metadata verified for ${version}.`);
