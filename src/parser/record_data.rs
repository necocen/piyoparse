use crate::model::{BreastMilkOrder, RecordData};

use super::scanner::Scanner;

pub(super) fn parse_record_data(record_type: &str, detail: Option<String>) -> RecordData {
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
