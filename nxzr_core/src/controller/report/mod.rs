use thiserror::Error;

pub mod input;
pub mod output;
pub mod subcommand;

#[derive(Clone, Error, Debug)]
pub enum ReportError {
    // Invalid value range has been entered.
    #[error("invalid value range supplied")]
    InvalidRange,
    // Indicates that given data has not enough length. Usually used in constructors.
    #[error("a length of the data is too short")]
    TooShort,
    // Indicates that given data is malformed thus cannot be processed. Usually used in constructors.
    #[error("the data is malformed thus cannot be processed")]
    Malformed,
    // Returned when accessing/processing data that do not support given bounds.
    #[error("out of bounds; invalid index")]
    OutOfBounds,
    // There's no data for a value within a range.
    //
    // Usually used instead of `OutOfBounds` for a return value of getter
    // methods where `OutOfBounds` is not appropriate.
    //
    // Since it's more descriptive to indicate that you are accessing
    // non-existent data than just saying data out-of-bounds.
    #[error("no data available")]
    NoDataAvailable,
    // Returned if invariant violation happens.
    #[error("invariant error occurred")]
    Invariant,
}
