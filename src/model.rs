use chrono::{NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};

/// Per-day summary totals printed by PiyoLog.
///
/// Fields that are absent from an export remain `0`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DaySummary {
    /// Total left-side breastfeeding minutes.
    pub breast_milk_left_minutes: u32,
    /// Total right-side breastfeeding minutes.
    pub breast_milk_right_minutes: u32,
    /// Number of formula records.
    pub formula_count: u32,
    /// Total formula amount in milliliters.
    pub formula_total_ml: u32,
    /// Number of expressed breast milk records.
    pub expressed_milk_count: u32,
    /// Total expressed breast milk amount in milliliters.
    pub expressed_milk_total_ml: u32,
    /// Total sleep duration in minutes.
    pub sleep_minutes: u32,
    /// Number of pee records.
    pub pee_count: u32,
    /// Number of poop records.
    pub poop_count: u32,
}

/// One timestamped record in a PiyoLog day.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Record {
    /// Calendar date that contains this record.
    pub date: NaiveDate,
    /// Time of day for this record.
    pub time: NaiveTime,
    /// Parsed record type and type-specific data.
    pub data: RecordData,
    /// Free-text memo attached to this record, if present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
}

/// Parsed data for a PiyoLog record.
///
/// Known record types use dedicated variants. Unknown future types and renamed
/// custom records use [`RecordData::Other`] with the original `type_name`, so
/// callers can still read exports from newer PiyoLog versions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RecordData {
    /// Breastfeeding record.
    Breastfeeding {
        /// Raw detail text from the export.
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
        /// Parsed left-side minutes.
        #[serde(skip_serializing_if = "Option::is_none")]
        left_minutes: Option<u32>,
        /// Parsed right-side minutes.
        #[serde(skip_serializing_if = "Option::is_none")]
        right_minutes: Option<u32>,
        /// Feeding order when the export explicitly encodes it.
        order: BreastMilkOrder,
        /// Estimated amount in milliliters when the detail contains an `ml` value.
        #[serde(skip_serializing_if = "Option::is_none")]
        amount_ml: Option<u32>,
    },
    /// Formula record.
    Formula {
        /// Raw detail text from the export.
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
        /// Amount in milliliters when the detail contains an `ml` value.
        #[serde(skip_serializing_if = "Option::is_none")]
        amount_ml: Option<u32>,
    },
    /// Expressed breast milk record.
    ExpressedBreastMilk {
        /// Raw detail text from the export.
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
        /// Amount in milliliters when the detail contains an `ml` value.
        #[serde(skip_serializing_if = "Option::is_none")]
        amount_ml: Option<u32>,
    },
    /// Bath record.
    Baths,
    /// Sleep start record.
    Sleep,
    /// Drink record.
    Drink {
        /// Raw detail text from the export.
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
        /// Amount in milliliters when the detail contains an `ml` value.
        #[serde(skip_serializing_if = "Option::is_none")]
        amount_ml: Option<u32>,
    },
    /// Wake-up record.
    WakeUp {
        /// Raw detail text from the export.
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
        /// Parsed duration in minutes.
        #[serde(skip_serializing_if = "Option::is_none")]
        duration_minutes: Option<u32>,
    },
    /// Pee record.
    Pee,
    /// Poop record.
    Poop {
        /// Raw detail text from the export.
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
    /// Pumping record.
    Pumping {
        /// Raw detail text from the export.
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
        /// Amount in milliliters when the detail contains an `ml` value.
        #[serde(skip_serializing_if = "Option::is_none")]
        amount_ml: Option<u32>,
    },
    /// Body temperature record.
    BodyTemp {
        /// Raw detail text from the export.
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
    /// Height record.
    Height {
        /// Raw detail text from the export.
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
    /// Weight record.
    Weight {
        /// Raw detail text from the export.
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
    /// Head size record.
    HeadSize {
        /// Raw detail text from the export.
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
    /// Chest size record.
    ChestSize {
        /// Raw detail text from the export.
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
    /// Solid food record.
    SolidFood,
    /// Snack record.
    Snack,
    /// Meal record.
    Meal,
    /// Cough record.
    Cough,
    /// Vomit record.
    Vomit,
    /// Rash record.
    Rash,
    /// Injury record.
    Injury,
    /// Medicine record.
    Medicine,
    /// Hospital record.
    Hospital,
    /// Vaccine record.
    Vaccine,
    /// Walk record.
    Walks {
        /// Raw detail text from the export.
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
        /// Parsed duration in minutes.
        #[serde(skip_serializing_if = "Option::is_none")]
        duration_minutes: Option<u32>,
    },
    /// Milestone record.
    Milestone,
    /// Other built-in record.
    Others,
    /// Notes record.
    Notes,
    /// Unknown or custom record type.
    Other {
        /// Original record type name from the export.
        type_name: String,
        /// Raw detail text from the export.
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
}

/// Explicit breastfeeding order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BreastMilkOrder {
    /// The export did not encode an order.
    Unspecified,
    /// Left side before right side.
    LeftThenRight,
    /// Right side before left side.
    RightThenLeft,
}

impl RecordData {
    /// Returns the parsed milliliter amount for amount-bearing records.
    ///
    /// Returns `None` for records that do not carry an amount or where the
    /// detail could not be interpreted as milliliters.
    pub fn amount_ml(&self) -> Option<u32> {
        match self {
            Self::Breastfeeding { amount_ml, .. }
            | Self::Formula { amount_ml, .. }
            | Self::ExpressedBreastMilk { amount_ml, .. }
            | Self::Drink { amount_ml, .. }
            | Self::Pumping { amount_ml, .. } => *amount_ml,
            _ => None,
        }
    }

    /// Returns the parsed duration in minutes for duration-bearing records.
    ///
    /// Breastfeeding duration is the sum of left and right minutes when both
    /// sides were parsed successfully.
    pub fn duration_minutes(&self) -> Option<u32> {
        match self {
            Self::Breastfeeding {
                left_minutes,
                right_minutes,
                ..
            } => Some((*left_minutes)? + (*right_minutes)?),
            Self::WakeUp {
                duration_minutes, ..
            }
            | Self::Walks {
                duration_minutes, ..
            } => *duration_minutes,
            _ => None,
        }
    }
}

/// Parsed data for one exported day.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Day {
    /// Calendar date for this day.
    pub date: NaiveDate,
    /// Child line from the export, if present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub child_info: Option<String>,
    /// Timestamped records for this day.
    pub records: Vec<Record>,
    /// Summary totals for this day.
    pub summary: DaySummary,
    /// Day-level memo text, if present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
}

/// Parsed PiyoLog export.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedExport {
    /// Days contained in the export.
    pub days: Vec<Day>,
}
