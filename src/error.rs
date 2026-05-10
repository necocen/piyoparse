use thiserror::Error;

pub type Result<T> = std::result::Result<T, ParseError>;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ParseError {
    #[error("PiyoLog export does not contain a date line")]
    MissingDate,

    #[error("invalid PiyoLog date line: {line}")]
    InvalidDate { line: String },

    #[error("invalid PiyoLog time: {time}")]
    InvalidTime { time: String },

    #[error("invalid PiyoLog record line: {line}")]
    InvalidRecordLine { line: String },
}
