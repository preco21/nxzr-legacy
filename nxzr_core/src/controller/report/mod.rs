use strum::{Display, IntoStaticStr};

pub mod input;
pub mod output;
pub mod subcommand;

#[derive(Clone, Copy, Debug, Display, Eq, PartialEq, Ord, PartialOrd, Hash, IntoStaticStr)]
pub enum ReportError {
    // Invalid value range has been entered.
    InvalidRange,
    // Indicates that given data has not enough length. Usually used in constructors.
    TooShort,
    // Indicates that given data is malformed thus cannot be processed. Usually used in constructors.
    Malformed,
    // Returned when accessing/processing data that do not support given bounds.
    OutOfBounds,
    // There's no data for a value within a range. Usually used instead of
    // `OutOfBounds` for a return value of getter methods where `OutOfBounds` is
    // not appropriate. Since it's more descriptive to indicate that you are
    // accessing no-existent data than just saying data out-of-bounds.
    NoDataAvailable,
    // Returned if invariant violation happens.
    Invariant,
}

impl std::error::Error for ReportError {}
