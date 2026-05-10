use chrono::{NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DaySummary {
    pub breast_milk_left_minutes: u32,
    pub breast_milk_right_minutes: u32,
    pub formula_count: u32,
    pub formula_total_ml: u32,
    pub expressed_milk_count: u32,
    pub expressed_milk_total_ml: u32,
    pub sleep_minutes: u32,
    pub pee_count: u32,
    pub poop_count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Record {
    pub date: NaiveDate,
    pub time: NaiveTime,
    pub data: RecordData,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RecordData {
    Breastfeeding {
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        left_minutes: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        right_minutes: Option<u32>,
        order: BreastMilkOrder,
        #[serde(skip_serializing_if = "Option::is_none")]
        amount_ml: Option<u32>,
    },
    Formula {
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        amount_ml: Option<u32>,
    },
    ExpressedBreastMilk {
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        amount_ml: Option<u32>,
    },
    Baths,
    Sleep,
    Drink {
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        amount_ml: Option<u32>,
    },
    WakeUp {
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        duration_minutes: Option<u32>,
    },
    Pee,
    Poop {
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
    Pumping {
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        amount_ml: Option<u32>,
    },
    BodyTemp {
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
    Height {
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
    Weight {
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
    HeadSize {
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
    ChestSize {
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
    SolidFood,
    Snack,
    Meal,
    Cough,
    Vomit,
    Rash,
    Injury,
    Medicine,
    Hospital,
    Vaccine,
    Walks {
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        duration_minutes: Option<u32>,
    },
    Milestone,
    Others,
    Notes,
    Other {
        type_name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BreastMilkOrder {
    Unspecified,
    LeftThenRight,
    RightThenLeft,
}

impl RecordData {
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Day {
    pub date: NaiveDate,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub child_info: Option<String>,
    pub records: Vec<Record>,
    pub summary: DaySummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedExport {
    pub days: Vec<Day>,
}
