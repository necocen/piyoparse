use crate::error::{ParseError, Result};
use crate::model::{BreastMilkOrder, Day, DaySummary, ParsedExport, Record, RecordData};
use chrono::{NaiveDate, NaiveTime};
use regex::Regex;
use std::sync::LazyLock;

static RE_SEPARATOR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^-{5,}$").unwrap());
static RE_DATE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:【ぴよログ】)?\s*(\d{4})/(\d{1,2})/(\d{1,2})\([^)]*\)").unwrap()
});
static RE_RECORD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\d{2}):(\d{2})\s{2,}(.+?)\s*$").unwrap());
static RE_SUMMARY_START: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(母乳|ミルク|搾母乳|睡眠|おしっこ|うんち)合計").unwrap());
static RE_AMOUNT_ML: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\d+)ml").unwrap());
static RE_DURATION_HM: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\d+)時間(\d+)分").unwrap());
static RE_DURATION_M: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\d+)分").unwrap());
static RE_BREAST_MILK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"([左右])\s*(\d+)分\s*([/→←])\s*([左右])\s*(\d+)分").unwrap());
static RE_SUMMARY_BREAST: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^母乳合計\s*左\s*(\d+)分\s*/\s*右\s*(\d+)分").unwrap());
static RE_SUMMARY_FORMULA: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^ミルク合計\s*(\d+)回\s*(\d+)ml").unwrap());
static RE_SUMMARY_EXPRESSED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^搾母乳合計\s*(\d+)回\s*(\d+)ml").unwrap());
static RE_SUMMARY_SLEEP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^睡眠合計\s*(\d+)時間(\d+)分").unwrap());
static RE_SUMMARY_PEE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^おしっこ合計\s*(\d+)回").unwrap());
static RE_SUMMARY_POOP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^うんち合計\s*(\d+)回").unwrap());

pub fn parse(input: &str) -> Result<ParsedExport> {
    let normalized = input.replace("\r\n", "\n").replace('\r', "\n");
    let lines: Vec<&str> = normalized.lines().collect();
    let date_indices: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| parse_date_line(line).ok().flatten().map(|_| index))
        .collect();

    if date_indices.is_empty() {
        return Err(ParseError::MissingDate);
    }

    let mut days = Vec::with_capacity(date_indices.len());
    for (position, start) in date_indices.iter().copied().enumerate() {
        let end = date_indices
            .get(position + 1)
            .copied()
            .unwrap_or(lines.len());
        days.push(parse_day_block(&lines[start..end])?);
    }

    Ok(ParsedExport { days })
}

fn parse_day_block(lines: &[&str]) -> Result<Day> {
    let date_line = lines.first().ok_or(ParseError::MissingDate)?;
    let date = parse_date_line(date_line)?.ok_or_else(|| ParseError::InvalidDate {
        line: (*date_line).to_string(),
    })?;

    let mut index = 1;
    let child_info = if let Some(line) = lines.get(index).map(|line| line.trim_end()) {
        if !line.trim().is_empty()
            && !is_record_start(line)
            && !is_summary_start(line)
            && !is_separator(line)
        {
            index += 1;
            Some(line.to_string())
        } else {
            None
        }
    } else {
        None
    };

    while lines
        .get(index)
        .is_some_and(|line| line.trim().is_empty() || is_separator(line.trim_end()))
    {
        index += 1;
    }

    let mut record_lines = Vec::new();
    while let Some(line) = lines.get(index).copied() {
        let trimmed = line.trim_end();
        if is_summary_start(trimmed) || is_separator(trimmed) {
            break;
        }
        record_lines.push(line);
        index += 1;
    }

    let records = parse_records(date, &record_lines)?;

    let mut summary_lines = Vec::new();
    while let Some(line) = lines.get(index).copied() {
        let trimmed = line.trim_end();
        if is_separator(trimmed) {
            break;
        }
        if trimmed.trim().is_empty() {
            index += 1;
            break;
        }
        summary_lines.push(trimmed);
        index += 1;
    }
    let summary = parse_summary(&summary_lines);

    let memo_lines: Vec<&str> = lines[index..]
        .iter()
        .copied()
        .take_while(|line| !is_separator(line.trim_end()))
        .collect();

    Ok(Day {
        date,
        child_info,
        records,
        summary,
        memo: clean_join(&memo_lines),
    })
}

fn parse_records(date: NaiveDate, lines: &[&str]) -> Result<Vec<Record>> {
    let mut raw_records = Vec::new();
    let mut current: Option<String> = None;

    for line in lines {
        let trimmed = line.trim_end();
        if is_record_start(trimmed) {
            if let Some(raw) = current.take() {
                raw_records.push(raw);
            }
            current = Some(trimmed.to_string());
        } else if let Some(raw) = current.as_mut() {
            raw.push('\n');
            raw.push_str(trimmed);
        }
    }

    if let Some(raw) = current {
        raw_records.push(raw);
    }

    raw_records
        .iter()
        .map(|raw| parse_record(date, raw))
        .collect()
}

fn parse_record(date: NaiveDate, raw: &str) -> Result<Record> {
    let (first_line, continuation) = raw.split_once('\n').unwrap_or((raw, ""));
    let captures = RE_RECORD
        .captures(first_line)
        .ok_or_else(|| ParseError::InvalidRecordLine {
            line: raw.to_string(),
        })?;

    let time_text = format!("{}:{}", &captures[1], &captures[2]);
    let time = NaiveTime::parse_from_str(&time_text, "%H:%M")
        .map_err(|_| ParseError::InvalidTime { time: time_text })?;

    let first_payload = captures[3].trim_end();
    let (record_type, detail, mut memo) = split_record_payload(first_payload);
    let continuation = clean_continuation(continuation);
    if let Some(continuation) = continuation {
        memo = Some(match memo {
            Some(existing) => format!("{existing}\n{continuation}"),
            None => continuation.trim_start_matches('\n').to_string(),
        });
    }

    let data = parse_record_data(&record_type, detail);

    Ok(Record {
        date,
        time,
        data,
        memo,
    })
}

fn parse_record_data(record_type: &str, detail: Option<String>) -> RecordData {
    let detail_text = detail.as_deref();

    match record_type {
        "母乳" => {
            let breast_milk = detail_text.and_then(extract_breast_milk_parts);
            let amount_ml = detail_text.and_then(extract_amount_ml);
            RecordData::Breastfeeding {
                left_minutes: breast_milk.map(|parts| parts.left_minutes),
                right_minutes: breast_milk.map(|parts| parts.right_minutes),
                order: breast_milk
                    .map(|parts| parts.order)
                    .unwrap_or(BreastMilkOrder::Unspecified),
                amount_ml,
                detail,
            }
        }
        "ミルク" => RecordData::Formula {
            amount_ml: detail_text.and_then(extract_amount_ml),
            detail,
        },
        "搾母乳" => RecordData::ExpressedBreastMilk {
            amount_ml: detail_text.and_then(extract_amount_ml),
            detail,
        },
        "お風呂" => no_detail_record(record_type, detail, RecordData::Baths),
        "寝る" => no_detail_record(record_type, detail, RecordData::Sleep),
        "おしっこ" => no_detail_record(record_type, detail, RecordData::Pee),
        "うんち" => RecordData::Poop { detail },
        "搾乳" => RecordData::Pumping {
            amount_ml: detail_text.and_then(extract_amount_ml),
            detail,
        },
        "体温" => RecordData::BodyTemp { detail },
        "身長" => RecordData::Height { detail },
        "体重" => RecordData::Weight { detail },
        "頭囲" => RecordData::HeadSize { detail },
        "胸囲" => RecordData::ChestSize { detail },
        "離乳食" => no_detail_record(record_type, detail, RecordData::SolidFood),
        "おやつ" => no_detail_record(record_type, detail, RecordData::Snack),
        "ごはん" => no_detail_record(record_type, detail, RecordData::Meal),
        "せき" => no_detail_record(record_type, detail, RecordData::Cough),
        "吐く" => no_detail_record(record_type, detail, RecordData::Vomit),
        "発疹" => no_detail_record(record_type, detail, RecordData::Rash),
        "けが" => no_detail_record(record_type, detail, RecordData::Injury),
        "くすり" => no_detail_record(record_type, detail, RecordData::Medicine),
        "病院" => no_detail_record(record_type, detail, RecordData::Hospital),
        "予防接種" => no_detail_record(record_type, detail, RecordData::Vaccine),
        "できた" => no_detail_record(record_type, detail, RecordData::Milestone),
        "その他" => no_detail_record(record_type, detail, RecordData::Others),
        "メモ" => no_detail_record(record_type, detail, RecordData::Notes),
        custom_type if custom_type.starts_with("カスタム") => RecordData::Other {
            type_name: custom_type.to_string(),
            detail,
        },
        "のみもの" => RecordData::Drink {
            amount_ml: detail_text.and_then(extract_amount_ml),
            detail,
        },
        "起きる" => RecordData::WakeUp {
            duration_minutes: detail_text.and_then(extract_duration_minutes),
            detail,
        },
        "さんぽ" => RecordData::Walks {
            duration_minutes: detail_text.and_then(extract_duration_minutes),
            detail,
        },
        _ => RecordData::Other {
            type_name: record_type.to_string(),
            detail,
        },
    }
}

fn no_detail_record(record_type: &str, detail: Option<String>, data: RecordData) -> RecordData {
    if detail.is_some() {
        RecordData::Other {
            type_name: record_type.to_string(),
            detail,
        }
    } else {
        data
    }
}

fn split_record_payload(payload: &str) -> (String, Option<String>, Option<String>) {
    let payload = payload.trim_end();
    let Some(first_space) = payload.find(' ') else {
        return (payload.to_string(), None, None);
    };

    let record_type = payload[..first_space].to_string();
    let remainder = &payload[first_space + 1..];
    if remainder.trim().is_empty() {
        return (record_type, None, None);
    }

    if remainder.starts_with(' ') {
        return (record_type, None, clean_text(remainder));
    }

    if let Some((start, end)) = find_ascii_space_run(remainder, 2) {
        let detail = clean_text(&remainder[..start]);
        let memo = clean_text(&remainder[end..]);
        return (record_type, detail, memo);
    }

    (record_type, clean_text(remainder), None)
}

fn parse_summary(lines: &[&str]) -> DaySummary {
    let mut summary = DaySummary::default();

    for line in lines {
        if let Some(captures) = RE_SUMMARY_BREAST.captures(line) {
            summary.breast_milk_left_minutes = captures[1].parse().unwrap_or(0);
            summary.breast_milk_right_minutes = captures[2].parse().unwrap_or(0);
        } else if let Some(captures) = RE_SUMMARY_FORMULA.captures(line) {
            summary.formula_count = captures[1].parse().unwrap_or(0);
            summary.formula_total_ml = captures[2].parse().unwrap_or(0);
        } else if let Some(captures) = RE_SUMMARY_EXPRESSED.captures(line) {
            summary.expressed_milk_count = captures[1].parse().unwrap_or(0);
            summary.expressed_milk_total_ml = captures[2].parse().unwrap_or(0);
        } else if let Some(captures) = RE_SUMMARY_SLEEP.captures(line) {
            let hours: u32 = captures[1].parse().unwrap_or(0);
            let minutes: u32 = captures[2].parse().unwrap_or(0);
            summary.sleep_minutes = hours * 60 + minutes;
        } else if let Some(captures) = RE_SUMMARY_PEE.captures(line) {
            summary.pee_count = captures[1].parse().unwrap_or(0);
        } else if let Some(captures) = RE_SUMMARY_POOP.captures(line) {
            summary.poop_count = captures[1].parse().unwrap_or(0);
        }
    }

    summary
}

fn parse_date_line(line: &str) -> Result<Option<NaiveDate>> {
    let Some(captures) = RE_DATE.captures(line.trim_end()) else {
        return Ok(None);
    };

    let year: i32 = captures[1].parse().map_err(|_| ParseError::InvalidDate {
        line: line.to_string(),
    })?;
    let month: u32 = captures[2].parse().map_err(|_| ParseError::InvalidDate {
        line: line.to_string(),
    })?;
    let day: u32 = captures[3].parse().map_err(|_| ParseError::InvalidDate {
        line: line.to_string(),
    })?;

    Ok(NaiveDate::from_ymd_opt(year, month, day))
}

fn is_record_start(line: &str) -> bool {
    RE_RECORD.is_match(line.trim_end())
}

fn is_summary_start(line: &str) -> bool {
    RE_SUMMARY_START.is_match(line.trim_end())
}

fn is_separator(line: &str) -> bool {
    RE_SEPARATOR.is_match(line.trim())
}

fn find_ascii_space_run(value: &str, min_len: usize) -> Option<(usize, usize)> {
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b' ' {
            index += 1;
            continue;
        }

        let start = index;
        while index < bytes.len() && bytes[index] == b' ' {
            index += 1;
        }
        if index - start >= min_len {
            return Some((start, index));
        }
    }

    None
}

fn extract_amount_ml(value: &str) -> Option<u32> {
    RE_AMOUNT_ML
        .captures(value)
        .and_then(|captures| captures[1].parse().ok())
}

fn extract_duration_minutes(value: &str) -> Option<u32> {
    if let Some(captures) = RE_DURATION_HM.captures(value) {
        let hours: u32 = captures[1].parse().ok()?;
        let minutes: u32 = captures[2].parse().ok()?;
        return Some(hours * 60 + minutes);
    }

    RE_DURATION_M
        .captures(value)
        .and_then(|captures| captures[1].parse().ok())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BreastMilkSide {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BreastMilkParts {
    left_minutes: u32,
    right_minutes: u32,
    order: BreastMilkOrder,
}

fn extract_breast_milk_parts(value: &str) -> Option<BreastMilkParts> {
    let captures = RE_BREAST_MILK.captures(value)?;
    let first_side = parse_breast_milk_side(&captures[1])?;
    let first_minutes: u32 = captures[2].parse().ok()?;
    let separator = &captures[3];
    let second_side = parse_breast_milk_side(&captures[4])?;
    let second_minutes: u32 = captures[5].parse().ok()?;

    let mut left_minutes = None;
    let mut right_minutes = None;
    for (side, minutes) in [(first_side, first_minutes), (second_side, second_minutes)] {
        match side {
            BreastMilkSide::Left => left_minutes = Some(minutes),
            BreastMilkSide::Right => right_minutes = Some(minutes),
        }
    }

    Some(BreastMilkParts {
        left_minutes: left_minutes?,
        right_minutes: right_minutes?,
        order: extract_breast_milk_order(first_side, separator, second_side),
    })
}

fn parse_breast_milk_side(value: &str) -> Option<BreastMilkSide> {
    match value {
        "左" => Some(BreastMilkSide::Left),
        "右" => Some(BreastMilkSide::Right),
        _ => None,
    }
}

fn extract_breast_milk_order(
    first_side: BreastMilkSide,
    separator: &str,
    second_side: BreastMilkSide,
) -> BreastMilkOrder {
    match separator {
        "/" => BreastMilkOrder::Unspecified,
        "→" => order_from_sides(first_side, second_side),
        "←" => order_from_sides(second_side, first_side),
        _ => BreastMilkOrder::Unspecified,
    }
}

fn order_from_sides(first: BreastMilkSide, second: BreastMilkSide) -> BreastMilkOrder {
    match (first, second) {
        (BreastMilkSide::Left, BreastMilkSide::Right) => BreastMilkOrder::LeftThenRight,
        (BreastMilkSide::Right, BreastMilkSide::Left) => BreastMilkOrder::RightThenLeft,
        _ => BreastMilkOrder::Unspecified,
    }
}

fn clean_text(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn clean_continuation(value: &str) -> Option<String> {
    let trimmed = value.trim_end();
    if trimmed.trim().is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn clean_join(lines: &[&str]) -> Option<String> {
    let first = lines.iter().position(|line| !line.trim().is_empty())?;
    let last = lines.iter().rposition(|line| !line.trim().is_empty())?;
    Some(lines[first..=last].join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const IOS_DAY: &str = include_str!("../tests/fixtures/ios_day.txt");
    const ANDROID_DAY: &str = include_str!("../tests/fixtures/android_day.txt");
    const ANDROID_DAY_WITH_HEADER: &str =
        include_str!("../tests/fixtures/android_export_day_with_header.txt");
    const IOS_MONTH: &str = include_str!("../tests/fixtures/export_2026_05_ios.txt");
    const ANDROID_MONTH: &str = include_str!("../tests/fixtures/export_2026_05_android.txt");
    const ORDERED_BREAST_MILK_DAY: &str =
        include_str!("../tests/fixtures/ordered_breast_milk_day.txt");
    const TYPED_RECORDS_DAY: &str = include_str!("../tests/fixtures/typed_records_day.txt");
    const KNOWN_RECORD_TYPES_DAY: &str =
        include_str!("../tests/fixtures/known_record_types_day.txt");
    const MEMO_ONLY_RECORDS_DAY: &str = include_str!("../tests/fixtures/memo_only_records_day.txt");

    #[test]
    fn parses_ios_day() {
        let parsed = parse(IOS_DAY).unwrap();
        assert_eq!(parsed.days.len(), 1);

        let day = &parsed.days[0];
        assert_eq!(day.date, NaiveDate::from_ymd_opt(2026, 5, 1).unwrap());
        assert_eq!(day.child_info.as_deref(), Some("赤ちゃん (6か月12日)"));
        assert_eq!(day.records.len(), 4);
        assert_eq!(day.summary.breast_milk_left_minutes, 5);
        assert_eq!(day.summary.breast_milk_right_minutes, 5);
        assert_eq!(day.summary.formula_count, 1);
        assert_eq!(day.summary.formula_total_ml, 170);
        assert_eq!(day.summary.expressed_milk_count, 0);
        assert_eq!(day.summary.expressed_milk_total_ml, 0);
        assert_eq!(day.summary.sleep_minutes, 150);
        assert_eq!(day.summary.pee_count, 0);
        assert_eq!(day.summary.poop_count, 1);
        assert_eq!(day.memo.as_deref(), Some("日次メモ1\n\n日次メモ2"));

        let wake = &day.records[0];
        assert_eq!(
            wake.data,
            RecordData::WakeUp {
                detail: Some("(2時間30分)".to_string()),
                duration_minutes: Some(150),
            }
        );

        let breast_milk = &day.records[2];
        assert_eq!(
            breast_milk.data,
            RecordData::Breastfeeding {
                detail: Some("左 5分 / 右 5分 (25ml)".to_string()),
                left_minutes: Some(5),
                right_minutes: Some(5),
                order: BreastMilkOrder::Unspecified,
                amount_ml: Some(25),
            }
        );

        let poop = &day.records[3];
        assert_eq!(poop.data, RecordData::Poop { detail: None });
        assert_eq!(poop.memo.as_deref(), Some("記録メモ1"));
    }

    #[test]
    fn parses_android_day() {
        let parsed = parse(ANDROID_DAY).unwrap();
        let day = &parsed.days[0];
        assert_eq!(day.date, NaiveDate::from_ymd_opt(2026, 5, 1).unwrap());
        assert_eq!(day.child_info.as_deref(), Some("赤ちゃん (6か月12日)"));
        assert_eq!(day.records.len(), 4);
        assert_eq!(day.summary.formula_total_ml, 170);
        assert_eq!(day.records[3].memo.as_deref(), Some("記録メモ1"));
    }

    #[test]
    fn parses_android_day_export_with_header() {
        let parsed = parse(ANDROID_DAY_WITH_HEADER).unwrap();
        assert_eq!(parsed.days.len(), 1);

        let day = &parsed.days[0];
        assert_eq!(day.date, NaiveDate::from_ymd_opt(2026, 5, 10).unwrap());
        assert_eq!(day.child_info.as_deref(), Some("赤ちゃん (6か月21日)"));
        assert_eq!(day.records.len(), 15);
        assert_eq!(day.summary.breast_milk_left_minutes, 7);
        assert_eq!(day.summary.breast_milk_right_minutes, 7);
        assert_eq!(day.summary.formula_count, 1);
        assert_eq!(day.summary.formula_total_ml, 180);
        assert_eq!(day.summary.expressed_milk_count, 1);
        assert_eq!(day.summary.expressed_milk_total_ml, 110);
        assert_eq!(day.summary.sleep_minutes, 105);
        assert_eq!(day.summary.pee_count, 1);
        assert_eq!(day.summary.poop_count, 0);
        assert_eq!(
            day.memo.as_deref(),
            Some("父：\nAndroid日次メモ1\nAndroid日次メモ2")
        );

        assert_eq!(
            day.records[3].data,
            RecordData::Breastfeeding {
                detail: Some("左7分 / 右7分 (30ml)".to_string()),
                left_minutes: Some(7),
                right_minutes: Some(7),
                order: BreastMilkOrder::Unspecified,
                amount_ml: Some(30),
            }
        );
        assert_eq!(day.records[5].data, RecordData::Others);
        assert_eq!(day.records[5].memo.as_deref(), Some("その他テスト"));
        assert_eq!(day.records[6].data, RecordData::Notes);
        assert_eq!(day.records[6].memo.as_deref(), Some("メモテスト"));
        assert_eq!(day.records[7].data, RecordData::Milestone);
        assert_eq!(day.records[7].memo.as_deref(), Some("できたテスト"));
        assert_eq!(
            day.records[8].data,
            RecordData::Other {
                type_name: "カスタム1".to_string(),
                detail: None,
            }
        );
        assert_eq!(day.records[8].memo.as_deref(), Some("カスタムメモ"));
        assert_eq!(day.records[9].data, RecordData::SolidFood);
        assert_eq!(day.records[10].data, RecordData::Snack);
        assert_eq!(day.records[11].data, RecordData::Meal);
        assert_eq!(day.records[12].data, RecordData::Vaccine);
        assert_eq!(day.records[13].data, RecordData::Hospital);
        assert_eq!(day.records[14].data, RecordData::Vomit);
    }

    #[test]
    fn parses_ios_single_day_header_and_ordered_breast_milk() {
        let parsed = parse(ORDERED_BREAST_MILK_DAY).unwrap();
        let day = &parsed.days[0];

        assert_eq!(day.date, NaiveDate::from_ymd_opt(2026, 5, 9).unwrap());
        assert_eq!(day.records.len(), 3);
        assert_eq!(
            day.records[0].data,
            RecordData::Breastfeeding {
                detail: Some("左 5分 ← 右 10分 (30ml)".to_string()),
                left_minutes: Some(5),
                right_minutes: Some(10),
                order: BreastMilkOrder::RightThenLeft,
                amount_ml: Some(30),
            }
        );
        assert_eq!(
            day.records[1].data,
            RecordData::Breastfeeding {
                detail: Some("右 5分 → 左 10分".to_string()),
                left_minutes: Some(10),
                right_minutes: Some(5),
                order: BreastMilkOrder::RightThenLeft,
                amount_ml: None,
            }
        );
        assert_eq!(
            day.records[2].data,
            RecordData::Breastfeeding {
                detail: Some("左 10分 → 右 5分 (35ml)".to_string()),
                left_minutes: Some(10),
                right_minutes: Some(5),
                order: BreastMilkOrder::LeftThenRight,
                amount_ml: Some(35),
            }
        );
    }

    #[test]
    fn parses_ios_month_export() {
        let parsed = parse(IOS_MONTH).unwrap();

        assert_eq!(parsed.days.len(), 2);
        assert_eq!(
            parsed.days[0].date,
            NaiveDate::from_ymd_opt(2026, 5, 1).unwrap()
        );
        assert_eq!(
            parsed.days[1].date,
            NaiveDate::from_ymd_opt(2026, 5, 2).unwrap()
        );
        assert_eq!(parsed.days[0].records.len(), 2);
        assert_eq!(parsed.days[1].records.len(), 2);
        assert_eq!(
            parsed
                .days
                .iter()
                .map(|day| day.records.len())
                .sum::<usize>(),
            4
        );
    }

    #[test]
    fn parses_android_month_export() {
        let parsed = parse(ANDROID_MONTH).unwrap();

        assert_eq!(parsed.days.len(), 2);
        assert_eq!(
            parsed.days[0].date,
            NaiveDate::from_ymd_opt(2026, 5, 1).unwrap()
        );
        assert_eq!(
            parsed.days[1].date,
            NaiveDate::from_ymd_opt(2026, 5, 2).unwrap()
        );
        assert_eq!(parsed.days[0].records.len(), 2);
        assert_eq!(parsed.days[1].records.len(), 2);
        assert_eq!(
            parsed
                .days
                .iter()
                .map(|day| day.records.len())
                .sum::<usize>(),
            4
        );
    }

    #[test]
    fn extracts_feeding_amounts() {
        let parsed = parse(IOS_DAY).unwrap();
        let formula = &parsed.days[0].records[1];
        assert_eq!(
            formula.data,
            RecordData::Formula {
                detail: Some("170ml".to_string()),
                amount_ml: Some(170),
            }
        );
    }

    #[test]
    fn parses_typed_amount_duration_and_fallback_records() {
        let parsed = parse(TYPED_RECORDS_DAY).unwrap();
        let records = &parsed.days[0].records;

        assert_eq!(
            records[0].data,
            RecordData::ExpressedBreastMilk {
                detail: Some("90ml".to_string()),
                amount_ml: Some(90),
            }
        );
        assert_eq!(
            records[1].data,
            RecordData::Drink {
                detail: Some("50ml".to_string()),
                amount_ml: Some(50),
            }
        );
        assert_eq!(
            records[2].data,
            RecordData::Walks {
                detail: Some("(0時間35分)".to_string()),
                duration_minutes: Some(35),
            }
        );
        assert_eq!(
            records[3].data,
            RecordData::Other {
                type_name: "カスタム1".to_string(),
                detail: None,
            }
        );
        assert_eq!(records[3].memo.as_deref(), Some("任意の値"));
    }

    #[test]
    fn parses_known_record_types_as_dedicated_variants() {
        let parsed = parse(KNOWN_RECORD_TYPES_DAY).unwrap();
        let records = &parsed.days[0].records;

        assert_eq!(records[0].data, RecordData::Baths);
        assert_eq!(records[1].data, RecordData::Sleep);
        assert_eq!(records[2].data, RecordData::Pee);
        assert_eq!(
            records[3].data,
            RecordData::Poop {
                detail: Some("(ちょこっと/ふつう)".to_string()),
            }
        );
        assert_eq!(
            records[4].data,
            RecordData::Pumping {
                detail: Some("100ml".to_string()),
                amount_ml: Some(100),
            }
        );
        assert_eq!(
            records[5].data,
            RecordData::BodyTemp {
                detail: Some("36.7℃".to_string()),
            }
        );
        assert_eq!(
            records[6].data,
            RecordData::Height {
                detail: Some("68.5cm".to_string()),
            }
        );
        assert_eq!(
            records[7].data,
            RecordData::Weight {
                detail: Some("7.75kg".to_string()),
            }
        );
        assert_eq!(
            records[8].data,
            RecordData::HeadSize {
                detail: Some("42.1cm".to_string()),
            }
        );
        assert_eq!(
            records[9].data,
            RecordData::ChestSize {
                detail: Some("41.9cm".to_string()),
            }
        );
        assert_eq!(records[10].data, RecordData::SolidFood);
        assert_eq!(records[11].data, RecordData::Snack);
        assert_eq!(records[12].data, RecordData::Meal);
        assert_eq!(records[13].data, RecordData::Cough);
        assert_eq!(records[14].data, RecordData::Vomit);
        assert_eq!(records[15].data, RecordData::Rash);
        assert_eq!(records[16].data, RecordData::Injury);
        assert_eq!(records[17].data, RecordData::Medicine);
        assert_eq!(records[18].data, RecordData::Hospital);
        assert_eq!(records[19].data, RecordData::Vaccine);
        assert_eq!(records[20].data, RecordData::Milestone);
        assert_eq!(records[20].memo.as_deref(), Some("できたテスト"));
        assert_eq!(records[21].data, RecordData::Others);
        assert_eq!(records[21].memo.as_deref(), Some("その他テスト"));
        assert_eq!(records[22].data, RecordData::Notes);
        assert_eq!(records[22].memo.as_deref(), Some("メモテスト"));
        assert_eq!(
            records[23].data,
            RecordData::Other {
                type_name: "カスタム1".to_string(),
                detail: None,
            }
        );
        assert_eq!(records[23].memo.as_deref(), Some("ああ"));
        assert_eq!(
            records[24].data,
            RecordData::Other {
                type_name: "新タイプ".to_string(),
                detail: Some("未知の値".to_string()),
            }
        );
        assert_eq!(
            records[25].data,
            RecordData::Other {
                type_name: "おしっこ".to_string(),
                detail: Some("少量".to_string()),
            }
        );
    }

    #[test]
    fn parses_memo_only_known_records() {
        let parsed = parse(MEMO_ONLY_RECORDS_DAY).unwrap();
        let records = &parsed.days[0].records;

        assert_eq!(records[0].data, RecordData::Others);
        assert_eq!(records[0].memo.as_deref(), Some("その他テスト"));
        assert_eq!(records[1].data, RecordData::Notes);
        assert_eq!(records[1].memo.as_deref(), Some("メモテスト"));
        assert_eq!(records[2].data, RecordData::Milestone);
        assert_eq!(records[2].memo.as_deref(), Some("できたテスト"));
        assert_eq!(
            records[3].data,
            RecordData::Pumping {
                detail: Some("30ml".to_string()),
                amount_ml: Some(30),
            }
        );
    }

    #[test]
    fn serializes_record_data_as_tagged_object() {
        let data = RecordData::Breastfeeding {
            detail: Some("左 5分 → 右 5分 (25ml)".to_string()),
            left_minutes: Some(5),
            right_minutes: Some(5),
            order: BreastMilkOrder::LeftThenRight,
            amount_ml: Some(25),
        };

        assert_eq!(
            serde_json::to_value(data).unwrap(),
            serde_json::json!({
                "kind": "breastfeeding",
                "detail": "左 5分 → 右 5分 (25ml)",
                "left_minutes": 5,
                "right_minutes": 5,
                "order": "left_then_right",
                "amount_ml": 25
            })
        );

        assert_eq!(
            serde_json::to_value(RecordData::Breastfeeding {
                detail: None,
                left_minutes: Some(5),
                right_minutes: Some(0),
                order: BreastMilkOrder::Unspecified,
                amount_ml: None,
            })
            .unwrap(),
            serde_json::json!({
                "kind": "breastfeeding",
                "left_minutes": 5,
                "right_minutes": 0,
                "order": "unspecified"
            })
        );

        assert_eq!(
            serde_json::to_value(RecordData::Other {
                type_name: "寝る".to_string(),
                detail: None,
            })
            .unwrap(),
            serde_json::json!({ "kind": "other", "type_name": "寝る" })
        );
    }

    #[test]
    fn omits_absent_optional_fields_when_serializing() {
        let record = Record {
            date: NaiveDate::from_ymd_opt(2026, 5, 10).unwrap(),
            time: NaiveTime::from_hms_opt(1, 35, 0).unwrap(),
            data: RecordData::Pee,
            memo: None,
        };

        assert_eq!(
            serde_json::to_value(record).unwrap(),
            serde_json::json!({
                "date": "2026-05-10",
                "time": "01:35:00",
                "data": { "kind": "pee" }
            })
        );

        let day = Day {
            date: NaiveDate::from_ymd_opt(2026, 5, 10).unwrap(),
            child_info: None,
            records: Vec::new(),
            summary: DaySummary::default(),
            memo: None,
        };

        assert_eq!(
            serde_json::to_value(day).unwrap(),
            serde_json::json!({
                "date": "2026-05-10",
                "records": [],
                "summary": {
                    "breast_milk_left_minutes": 0,
                    "breast_milk_right_minutes": 0,
                    "formula_count": 0,
                    "formula_total_ml": 0,
                    "expressed_milk_count": 0,
                    "expressed_milk_total_ml": 0,
                    "sleep_minutes": 0,
                    "pee_count": 0,
                    "poop_count": 0
                }
            })
        );
    }
}
