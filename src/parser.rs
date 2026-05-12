use crate::error::{ParseError, Result};
use crate::model::{BreastMilkOrder, Day, DaySummary, ParsedExport, Record, RecordData};
use chrono::{NaiveDate, NaiveTime};

#[derive(Debug, Clone, Copy)]
struct Scanner<'a> {
    rest: &'a str,
}

impl<'a> Scanner<'a> {
    fn new(rest: &'a str) -> Self {
        Self { rest }
    }

    fn rest(&self) -> &'a str {
        self.rest
    }

    fn strip_prefix(&mut self, prefix: &str) -> bool {
        if let Some(rest) = self.rest.strip_prefix(prefix) {
            self.rest = rest;
            true
        } else {
            false
        }
    }

    fn skip_spaces(&mut self) {
        self.rest = self.rest.trim_start_matches(is_piyolog_space);
    }

    fn take_space_run(&mut self) -> usize {
        let mut count = 0;
        let mut end = 0;

        for (index, character) in self.rest.char_indices() {
            if !is_piyolog_space(character) {
                break;
            }
            count += 1;
            end = index + character.len_utf8();
        }

        self.rest = &self.rest[end..];
        count
    }

    fn take_char_if(&mut self, predicate: impl FnOnce(char) -> bool) -> Option<char> {
        let mut characters = self.rest.char_indices();
        let (_, character) = characters.next()?;
        if !predicate(character) {
            return None;
        }

        let next = characters
            .next()
            .map(|(index, _)| index)
            .unwrap_or(self.rest.len());
        self.rest = &self.rest[next..];
        Some(character)
    }

    fn take_ascii_digits(&mut self) -> Option<&'a str> {
        let end = self
            .rest
            .as_bytes()
            .iter()
            .take_while(|byte| byte.is_ascii_digit())
            .count();
        if end == 0 {
            return None;
        }

        let (digits, rest) = self.rest.split_at(end);
        self.rest = rest;
        Some(digits)
    }

    fn take_ascii_digits_exact(&mut self, length: usize) -> Option<&'a str> {
        if self.rest.len() < length
            || !self.rest.as_bytes()[..length]
                .iter()
                .all(|byte| byte.is_ascii_digit())
        {
            return None;
        }

        let (digits, rest) = self.rest.split_at(length);
        self.rest = rest;
        Some(digits)
    }

    fn take_i32(&mut self) -> Option<i32> {
        self.take_ascii_digits()?.parse().ok()
    }

    fn take_u32(&mut self) -> Option<u32> {
        self.take_ascii_digits()?.parse().ok()
    }
}

fn is_piyolog_space(character: char) -> bool {
    matches!(character, ' ' | '\t' | '\u{3000}')
}

fn trim_end_piyolog_spaces(value: &str) -> &str {
    value.trim_end_matches(is_piyolog_space)
}

/// Parses a PiyoLog export.
///
/// The input may be a single-day export or a month export containing multiple
/// day blocks. Both iOS-style and Android-style spacing are handled by the same
/// parser. Missing or unknown record details are preserved as raw strings where
/// possible rather than causing the whole export to fail.
///
/// # Errors
///
/// Returns [`ParseError::MissingDate`](crate::ParseError::MissingDate) when no
/// day header can be found. Malformed date, time, or record lines return the
/// corresponding [`ParseError`](crate::ParseError).
///
/// # Example
///
/// ```
/// let export = "\
/// 【ぴよログ】2026/5/10(日)
/// 赤ちゃん (6か月21日)
///
/// 01:40   ミルク 180ml
///
/// ミルク合計 1回 180ml
/// ";
///
/// let parsed = piyoparse::parse(export)?;
/// assert_eq!(parsed.days[0].records.len(), 1);
/// assert_eq!(parsed.days[0].summary.formula_total_ml, 180);
///
/// # Ok::<(), piyoparse::ParseError>(())
/// ```
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
    let record_line =
        parse_record_line(first_line).ok_or_else(|| ParseError::InvalidRecordLine {
            line: raw.to_string(),
        })?;

    let time_text = format!("{}:{}", record_line.hour, record_line.minute);
    let hour = record_line
        .hour
        .parse()
        .map_err(|_| ParseError::InvalidTime {
            time: time_text.clone(),
        })?;
    let minute = record_line
        .minute
        .parse()
        .map_err(|_| ParseError::InvalidTime {
            time: time_text.clone(),
        })?;
    let time = NaiveTime::from_hms_opt(hour, minute, 0)
        .ok_or(ParseError::InvalidTime { time: time_text })?;

    let first_payload = record_line.payload;
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

#[derive(Debug, Clone, Copy)]
struct ParsedRecordLine<'a> {
    hour: &'a str,
    minute: &'a str,
    payload: &'a str,
}

fn parse_record_line(line: &str) -> Option<ParsedRecordLine<'_>> {
    let mut scanner = Scanner::new(trim_end_piyolog_spaces(line));
    let hour = scanner.take_ascii_digits_exact(2)?;
    scanner.strip_prefix(":").then_some(())?;
    let minute = scanner.take_ascii_digits_exact(2)?;
    (scanner.take_space_run() >= 2).then_some(())?;

    let payload = trim_end_piyolog_spaces(scanner.rest());
    (!payload.is_empty()).then_some(ParsedRecordLine {
        hour,
        minute,
        payload,
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
    let payload = trim_end_piyolog_spaces(payload);
    let Some((space_start, space_end, space_count)) = find_space_run(payload, 1) else {
        return (payload.to_string(), None, None);
    };

    let record_type = payload[..space_start].to_string();
    let remainder = &payload[space_end..];
    if remainder.trim().is_empty() {
        return (record_type, None, None);
    }

    if space_count >= 2 {
        return (record_type, None, clean_text(remainder));
    }

    if let Some((start, end, _)) = find_space_run(remainder, 2) {
        let detail = clean_text(&remainder[..start]);
        let memo = clean_text(&remainder[end..]);
        return (record_type, detail, memo);
    }

    (record_type, clean_text(remainder), None)
}

fn parse_summary(lines: &[&str]) -> DaySummary {
    let mut summary = DaySummary::default();

    for line in lines {
        if let Some((left_minutes, right_minutes)) = parse_summary_breast(line) {
            summary.breast_milk_left_minutes = left_minutes;
            summary.breast_milk_right_minutes = right_minutes;
        } else if let Some((count, total_ml)) = parse_summary_count_total(line, "ミルク合計") {
            summary.formula_count = count;
            summary.formula_total_ml = total_ml;
        } else if let Some((count, total_ml)) = parse_summary_count_total(line, "搾母乳合計") {
            summary.expressed_milk_count = count;
            summary.expressed_milk_total_ml = total_ml;
        } else if let Some((hours, minutes)) = parse_summary_sleep(line) {
            summary.sleep_minutes = hours * 60 + minutes;
        } else if let Some(count) = parse_summary_count(line, "おしっこ合計") {
            summary.pee_count = count;
        } else if let Some(count) = parse_summary_count(line, "うんち合計") {
            summary.poop_count = count;
        }
    }

    summary
}

fn parse_summary_breast(line: &str) -> Option<(u32, u32)> {
    let mut scanner = Scanner::new(line);
    scanner.strip_prefix("母乳合計").then_some(())?;
    scanner.skip_spaces();
    scanner.strip_prefix("左").then_some(())?;
    scanner.skip_spaces();
    let left_minutes = scanner.take_u32()?;
    scanner.strip_prefix("分").then_some(())?;
    scanner.skip_spaces();
    scanner.strip_prefix("/").then_some(())?;
    scanner.skip_spaces();
    scanner.strip_prefix("右").then_some(())?;
    scanner.skip_spaces();
    let right_minutes = scanner.take_u32()?;
    scanner.strip_prefix("分").then_some(())?;
    Some((left_minutes, right_minutes))
}

fn parse_summary_count_total(line: &str, prefix: &str) -> Option<(u32, u32)> {
    let mut scanner = Scanner::new(line);
    scanner.strip_prefix(prefix).then_some(())?;
    scanner.skip_spaces();
    let count = scanner.take_u32()?;
    scanner.strip_prefix("回").then_some(())?;
    scanner.skip_spaces();
    let total_ml = scanner.take_u32()?;
    scanner.strip_prefix("ml").then_some(())?;
    Some((count, total_ml))
}

fn parse_summary_sleep(line: &str) -> Option<(u32, u32)> {
    let mut scanner = Scanner::new(line);
    scanner.strip_prefix("睡眠合計").then_some(())?;
    scanner.skip_spaces();
    let hours = scanner.take_u32()?;
    scanner.strip_prefix("時間").then_some(())?;
    let minutes = scanner.take_u32()?;
    scanner.strip_prefix("分").then_some(())?;
    Some((hours, minutes))
}

fn parse_summary_count(line: &str, prefix: &str) -> Option<u32> {
    let mut scanner = Scanner::new(line);
    scanner.strip_prefix(prefix).then_some(())?;
    scanner.skip_spaces();
    let count = scanner.take_u32()?;
    scanner.strip_prefix("回").then_some(())?;
    Some(count)
}

fn parse_date_line(line: &str) -> Result<Option<NaiveDate>> {
    let mut scanner = Scanner::new(line.trim_end());
    scanner.strip_prefix("【ぴよログ】");
    scanner.skip_spaces();

    let Some(year) = scanner.take_i32() else {
        return Ok(None);
    };
    if !scanner.strip_prefix("/") {
        return Ok(None);
    }
    let Some(month) = scanner.take_u32() else {
        return Ok(None);
    };
    if !scanner.strip_prefix("/") {
        return Ok(None);
    }
    let Some(day) = scanner.take_u32() else {
        return Ok(None);
    };
    if !scanner.strip_prefix("(") || !scanner.rest().contains(')') {
        return Ok(None);
    }

    Ok(NaiveDate::from_ymd_opt(year, month, day))
}

fn is_record_start(line: &str) -> bool {
    parse_record_line(line.trim_end()).is_some()
}

fn is_summary_start(line: &str) -> bool {
    let line = line.trim_end();
    line.starts_with("母乳合計")
        || line.starts_with("ミルク合計")
        || line.starts_with("搾母乳合計")
        || line.starts_with("睡眠合計")
        || line.starts_with("おしっこ合計")
        || line.starts_with("うんち合計")
}

fn is_separator(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.len() >= 5 && trimmed.bytes().all(|byte| byte == b'-')
}

fn find_space_run(value: &str, min_len: usize) -> Option<(usize, usize, usize)> {
    let mut run_start = None;
    let mut run_count = 0;
    let mut run_end = 0;

    for (index, character) in value.char_indices() {
        if is_piyolog_space(character) {
            run_start.get_or_insert(index);
            run_count += 1;
            run_end = index + character.len_utf8();
            continue;
        }

        if run_count >= min_len {
            return Some((run_start?, run_end, run_count));
        }
        run_start = None;
        run_count = 0;
    }

    (run_count >= min_len).then_some((run_start?, run_end, run_count))
}

fn extract_amount_ml(value: &str) -> Option<u32> {
    for (ml_index, _) in value.match_indices("ml") {
        if let Some(amount) = trailing_ascii_number(&value[..ml_index]) {
            return Some(amount);
        }
    }

    None
}

fn extract_duration_minutes(value: &str) -> Option<u32> {
    let mut in_digits = false;
    for (index, character) in value.char_indices() {
        if character.is_ascii_digit() {
            if !in_digits {
                let mut scanner = Scanner::new(&value[index..]);
                if let Some(hours) = scanner.take_u32()
                    && scanner.strip_prefix("時間")
                    && let Some(minutes) = scanner.take_u32()
                    && scanner.strip_prefix("分")
                {
                    return Some(hours * 60 + minutes);
                }
            }
            in_digits = true;
        } else {
            in_digits = false;
        }
    }

    let mut in_digits = false;
    for (index, character) in value.char_indices() {
        if character.is_ascii_digit() {
            if !in_digits {
                let mut scanner = Scanner::new(&value[index..]);
                if let Some(minutes) = scanner.take_u32()
                    && scanner.strip_prefix("分")
                {
                    return Some(minutes);
                }
            }
            in_digits = true;
        } else {
            in_digits = false;
        }
    }

    None
}

fn trailing_ascii_number(value: &str) -> Option<u32> {
    let end = value.len();
    let start = value
        .char_indices()
        .rev()
        .find_map(|(index, character)| (!character.is_ascii_digit()).then_some(index + 1))
        .unwrap_or(0);

    (start < end).then(|| value[start..end].parse().ok())?
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
    for (index, character) in value.char_indices() {
        if matches!(character, '左' | '右')
            && let Some(parts) = extract_breast_milk_parts_at(&value[index..])
        {
            return Some(parts);
        }
    }

    None
}

fn extract_breast_milk_parts_at(value: &str) -> Option<BreastMilkParts> {
    let mut scanner = Scanner::new(value);
    let first_side = scanner
        .take_char_if(|character| matches!(character, '左' | '右'))
        .and_then(parse_breast_milk_side)?;
    scanner.skip_spaces();
    let first_minutes = scanner.take_u32()?;
    scanner.strip_prefix("分").then_some(())?;
    scanner.skip_spaces();
    let separator = scanner.take_char_if(|character| matches!(character, '/' | '→' | '←'))?;
    scanner.skip_spaces();
    let second_side = scanner
        .take_char_if(|character| matches!(character, '左' | '右'))
        .and_then(parse_breast_milk_side)?;
    scanner.skip_spaces();
    let second_minutes = scanner.take_u32()?;
    scanner.strip_prefix("分").then_some(())?;

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

fn parse_breast_milk_side(value: char) -> Option<BreastMilkSide> {
    match value {
        '左' => Some(BreastMilkSide::Left),
        '右' => Some(BreastMilkSide::Right),
        _ => None,
    }
}

fn extract_breast_milk_order(
    first_side: BreastMilkSide,
    separator: char,
    second_side: BreastMilkSide,
) -> BreastMilkOrder {
    match separator {
        '/' => BreastMilkOrder::Unspecified,
        '→' => order_from_sides(first_side, second_side),
        '←' => order_from_sides(second_side, first_side),
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
    const FULL_WIDTH_SPACING_DAY: &str =
        include_str!("../tests/fixtures/full_width_spacing_day.txt");

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
    fn parses_full_width_spacing() {
        let parsed = parse(FULL_WIDTH_SPACING_DAY).unwrap();
        let day = &parsed.days[0];

        assert_eq!(day.date, NaiveDate::from_ymd_opt(2026, 5, 10).unwrap());
        assert_eq!(day.summary.breast_milk_left_minutes, 5);
        assert_eq!(day.summary.breast_milk_right_minutes, 6);
        assert_eq!(day.summary.formula_total_ml, 180);
        assert_eq!(
            day.records[0].data,
            RecordData::Formula {
                amount_ml: Some(180),
                detail: Some("180ml".to_string()),
            }
        );
        assert_eq!(
            day.records[1].data,
            RecordData::Breastfeeding {
                left_minutes: Some(5),
                right_minutes: Some(6),
                order: BreastMilkOrder::Unspecified,
                amount_ml: None,
                detail: Some("左　5分　/　右　6分".to_string()),
            }
        );
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
