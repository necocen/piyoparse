import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import {
  parsePiyolog,
  parsePiyologJson,
  type BreastfeedingRecord,
  type FormulaRecord,
  type OtherRecord,
  type ParsedExport,
  type PiyologRecord,
  type RecordData,
} from "piyoparse";

const fixture = readFileSync(
  join(__dirname, "..", "..", "fixtures", "android_export_day_with_header.txt"),
  "utf8",
);

const parsed: ParsedExport = parsePiyolog(fixture);
assert.equal(parsed.days.length, 1);

const day = parsed.days[0]!;
assert.equal(day.date, "2026-05-10");
assert.equal(day.child_info, "赤ちゃん (6か月21日)");
assert.equal(day.records.length, 15);
assert.equal(day.summary.sleep_minutes, 105);
assert.equal(day.memo, "父：\nAndroid日次メモ1\nAndroid日次メモ2");

const breastfeedingRecord: PiyologRecord = day.records[3]!;
assertBreastfeedingRecord(breastfeedingRecord.data);
assert.equal(breastfeedingRecord.time, "08:05:00");
assert.equal(breastfeedingRecord.data.detail, "左7分 / 右7分 (30ml)");
assert.equal(breastfeedingRecord.data.left_minutes, 7);
assert.equal(breastfeedingRecord.data.right_minutes, 7);
assert.equal(breastfeedingRecord.data.order, "unspecified");
assert.equal(breastfeedingRecord.data.amount_ml, 30);

const formulaData: RecordData = day.records[2]!.data;
assertFormulaRecord(formulaData);
assert.equal(formulaData.detail, "180ml");
assert.equal(formulaData.amount_ml, 180);

const customRecord = day.records[8]!;
assertOtherRecord(customRecord.data);
assert.equal(customRecord.data.type_name, "カスタム1");
assert.equal(customRecord.memo, "カスタムメモ");

const parsedFromJson: ParsedExport = JSON.parse(parsePiyologJson(fixture));
assert.deepEqual(parsedFromJson, parsed);

function assertBreastfeedingRecord(data: RecordData): asserts data is BreastfeedingRecord {
  assert.equal(data.kind, "breastfeeding");
}

function assertFormulaRecord(data: RecordData): asserts data is FormulaRecord {
  assert.equal(data.kind, "formula");
}

function assertOtherRecord(data: RecordData): asserts data is OtherRecord {
  assert.equal(data.kind, "other");
}
