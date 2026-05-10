use crate::parse;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const TYPESCRIPT_TYPES: &str = r#"
export type PiyologDate = string;
export type PiyologTime = string;

export type BreastMilkOrder =
  | "unspecified"
  | "left_then_right"
  | "right_then_left";

export interface ParsedExport {
  days: Day[];
}

export interface Day {
  date: PiyologDate;
  child_info?: string;
  records: PiyologRecord[];
  summary: DaySummary;
  memo?: string;
}

export interface DaySummary {
  breast_milk_left_minutes: number;
  breast_milk_right_minutes: number;
  formula_count: number;
  formula_total_ml: number;
  expressed_milk_count: number;
  expressed_milk_total_ml: number;
  sleep_minutes: number;
  pee_count: number;
  poop_count: number;
}

export interface PiyologRecord {
  date: PiyologDate;
  time: PiyologTime;
  data: RecordData;
  memo?: string;
}

export type RecordData =
  | BreastfeedingRecord
  | FormulaRecord
  | ExpressedBreastMilkRecord
  | DrinkRecord
  | WakeUpRecord
  | WalksRecord
  | PumpingRecord
  | DetailRecord
  | EmptyRecord
  | OtherRecord;

export interface BreastfeedingRecord {
  kind: "breastfeeding";
  detail?: string;
  left_minutes?: number;
  right_minutes?: number;
  order: BreastMilkOrder;
  amount_ml?: number;
}

export interface FormulaRecord {
  kind: "formula";
  detail?: string;
  amount_ml?: number;
}

export interface ExpressedBreastMilkRecord {
  kind: "expressed_breast_milk";
  detail?: string;
  amount_ml?: number;
}

export interface DrinkRecord {
  kind: "drink";
  detail?: string;
  amount_ml?: number;
}

export interface WakeUpRecord {
  kind: "wake_up";
  detail?: string;
  duration_minutes?: number;
}

export interface WalksRecord {
  kind: "walks";
  detail?: string;
  duration_minutes?: number;
}

export interface PumpingRecord {
  kind: "pumping";
  detail?: string;
  amount_ml?: number;
}

export interface DetailRecord {
  kind:
    | "poop"
    | "body_temp"
    | "height"
    | "weight"
    | "head_size"
    | "chest_size";
  detail?: string;
}

export interface EmptyRecord {
  kind:
    | "baths"
    | "sleep"
    | "pee"
    | "solid_food"
    | "snack"
    | "meal"
    | "cough"
    | "vomit"
    | "rash"
    | "injury"
    | "medicine"
    | "hospital"
    | "vaccine"
    | "milestone"
    | "others"
    | "notes";
}

export interface OtherRecord {
  kind: "other";
  type_name: string;
  detail?: string;
}

export function parsePiyolog(input: string): ParsedExport;
export function parsePiyologJson(input: string): string;
"#;

#[wasm_bindgen(js_name = parsePiyolog, skip_typescript)]
pub fn parse_piyolog_wasm(input: &str) -> std::result::Result<JsValue, JsValue> {
    let parsed = parse(input).map_err(|error| JsValue::from_str(&error.to_string()))?;
    serde_wasm_bindgen::to_value(&parsed).map_err(|error| JsValue::from_str(&error.to_string()))
}

#[wasm_bindgen(js_name = parsePiyologJson, skip_typescript)]
pub fn parse_piyolog_json_wasm(input: &str) -> std::result::Result<String, JsValue> {
    let parsed = parse(input).map_err(|error| JsValue::from_str(&error.to_string()))?;
    serde_json::to_string(&parsed).map_err(|error| JsValue::from_str(&error.to_string()))
}
