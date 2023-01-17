use strum::{Display, IntoStaticStr};

pub mod input;
pub mod output;
pub mod subcommand;

#[derive(Clone, Copy, Debug, Display, Eq, PartialEq, Ord, PartialOrd, Hash, IntoStaticStr)]
pub enum ReportError {
    TooShort,
    Malformed,
    UnsupportedReportId,
    UnsupportedSubcommand,
    OutOfRange,
    Invariant,
}

impl std::error::Error for ReportError {}

pub type ReportResult<T> = Result<T, ReportError>;
