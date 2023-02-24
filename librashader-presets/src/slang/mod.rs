mod parse;
mod token;

pub(crate) type Span<'a> = LocatedSpan<&'a str>;

use nom_locate::LocatedSpan;
pub use parse::parse_preset;
pub use parse::parse_values;

