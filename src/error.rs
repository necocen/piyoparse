use thiserror::Error;

/// Result type returned by parser APIs.
pub type Result<T> = std::result::Result<T, ParseError>;

/// Errors that can occur while reading a PiyoLog export.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ParseError {
    /// The input did not contain any recognizable day header.
    #[error("PiyoLog export does not contain a date line")]
    MissingDate,

    /// A line looked like a date header, but could not be converted to a date.
    #[error("invalid PiyoLog date line: {line}")]
    InvalidDate { line: String },

    /// A record time could not be converted to `chrono::NaiveTime`.
    #[error("invalid PiyoLog time: {time}")]
    InvalidTime { time: String },

    /// A record-like line did not match the supported export record format.
    #[error("invalid PiyoLog record line: {line}")]
    InvalidRecordLine { line: String },
}
