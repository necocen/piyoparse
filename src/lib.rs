//! Parser for PiyoLog export text.
//!
//! `piyoparse` reads PiyoLog day or month exports into Rust structs. The
//! parser intentionally uses one tolerant path for both iOS and Android export
//! layouts instead of asking callers to classify the platform first.
//!
//! # Example
//!
//! ```
//! use piyoparse::{parse, RecordData};
//!
//! let export = "\
//! 【ぴよログ】2026/5/10(日)
//! 赤ちゃん (6か月21日)
//!
//! 01:40   ミルク 180ml
//! 08:05   母乳 左7分 / 右7分 (30ml)
//!
//! 母乳合計 左 7分 / 右 7分
//! ミルク合計 1回 180ml
//! ";
//!
//! let parsed = parse(export)?;
//! let day = &parsed.days[0];
//!
//! assert_eq!(day.records.len(), 2);
//! assert_eq!(day.summary.formula_total_ml, 180);
//!
//! match &day.records[0].data {
//!     RecordData::Formula { amount_ml, .. } => assert_eq!(*amount_ml, Some(180)),
//!     other => panic!("unexpected record data: {other:?}"),
//! }
//!
//! # Ok::<(), piyoparse::ParseError>(())
//! ```
//!
//! This crate is an unofficial tool and is not affiliated with PiyoLog or its
//! operating company.

mod error;
mod model;
mod parser;

#[cfg(feature = "wasm")]
mod wasm;

pub use error::{ParseError, Result};
pub use model::{BreastMilkOrder, Day, DaySummary, ParsedExport, Record, RecordData};
pub use parser::parse;
