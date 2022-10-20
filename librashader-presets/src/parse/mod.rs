use nom::{ExtendInto, Offset};
use nom_locate::LocatedSpan;
use std::str;

mod token;
mod value;

pub type Span<'a> = LocatedSpan<&'a str>;
use crate::error::ParsePresetError;
pub use token::do_lex;
pub use token::Token;
pub use value::parse_values;
