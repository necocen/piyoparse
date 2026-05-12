const assert = require("node:assert/strict");
const { readFileSync } = require("node:fs");
const { join } = require("node:path");
const { parsePiyolog } = require("@necocen/piyoparse");

const fixture = readFileSync(
  join(__dirname, "..", "fixtures", "android_export_day_with_header.txt"),
  "utf8",
);

const parsed = parsePiyolog(fixture);
assert.equal(parsed.days.length, 1);
assert.equal(parsed.days[0].records.length, 15);
assert.equal(parsed.days[0].records[2].data.kind, "formula");
assert.equal(parsed.days[0].records[2].data.amount_ml, 180);
