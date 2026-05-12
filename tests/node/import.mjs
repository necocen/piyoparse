import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { parsePiyolog } from "@necocen/piyoparse";

const __dirname = dirname(fileURLToPath(import.meta.url));
const fixture = readFileSync(
  join(__dirname, "..", "fixtures", "android_export_day_with_header.txt"),
  "utf8",
);

const parsed = parsePiyolog(fixture);
assert.equal(parsed.days.length, 1);
assert.equal(parsed.days[0].records.length, 15);
assert.equal(parsed.days[0].records[2].data.kind, "formula");
assert.equal(parsed.days[0].records[2].data.amount_ml, 180);
