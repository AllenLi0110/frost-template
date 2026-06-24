import { readFileSync } from "node:fs";

function fail(message) {
  console.error(message);
  process.exit(1);
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

const rawVersion = process.argv[2];

if (!rawVersion) {
  fail("Usage: node scripts/extract-release-notes.mjs <version-or-tag>");
}

const version = rawVersion.startsWith("v") ? rawVersion.slice(1) : rawVersion;
const changelog = readFileSync("CHANGELOG.md", "utf8");
const headingPattern = new RegExp(`^## \\[${escapeRegExp(version)}\\] - \\d{4}-\\d{2}-\\d{2}\\n`, "m");
const headingMatch = changelog.match(headingPattern);

if (!headingMatch || headingMatch.index === undefined) {
  fail(`CHANGELOG.md is missing release notes for ${version}`);
}

const notesStart = headingMatch.index + headingMatch[0].length;
const rest = changelog.slice(notesStart);
const nextHeadingIndex = rest.search(/^## /m);
const notes = (nextHeadingIndex === -1 ? rest : rest.slice(0, nextHeadingIndex)).trim();

if (!notes) {
  fail(`CHANGELOG.md release notes for ${version} are empty`);
}

console.log(notes);
