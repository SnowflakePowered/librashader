use std::borrow::Borrow;
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::ops::Deref;
use strumbra::SharedString;

#[repr(transparent)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, PartialOrd, Ord, Hash)]
pub struct ParamString(SharedString);

impl Deref for ParamString {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl From<&str> for ParamString {
    #[inline]
    fn from(s: &str) -> Self {
        ParamString(SharedString::try_from(s)
            .expect("ParamString with more than 4294967295 characters. Parameter too large."))
    }
}

impl From<&String> for ParamString {
    #[inline]
    fn from(s: &String) -> Self {
        ParamString(SharedString::try_from(s)
            .expect("ParamString with more than 4294967295 characters. Parameter too large."))
    }
}


impl From<String> for ParamString {
    #[inline]
    fn from(s: String) -> Self {
        ParamString(SharedString::try_from(s)
            .expect("ParamString with more than 4294967295 characters. Parameter too large."))
    }
}

impl Display for ParamString {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for ParamString {
    #[inline]
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl Borrow<str> for ParamString {
    fn borrow(&self) -> &str {
        self.0.as_ref()
    }
}

impl PartialEq<&str> for ParamString {
    fn eq(&self, other: &&str) -> bool {
        self.0.as_str() == *other
    }
}

impl PartialEq<str> for ParamString {
    fn eq(&self, other: &str) -> bool {
        self.0.as_str() == other
    }
}

impl PartialEq<String> for ParamString {
    fn eq(&self, other: &String) -> bool {
        self.0.as_str() == other
    }
}

impl PartialEq<ParamString> for &str {
    fn eq(&self, other: &ParamString) -> bool {
        *self == other.as_ref()
    }
}

impl PartialEq<ParamString> for String {
    fn eq(&self, other: &ParamString) -> bool {
        *self == other.as_ref()
    }
}

impl PartialEq<ParamString> for &String {
    fn eq(&self, other: &ParamString) -> bool {
        *self == other.as_ref()
    }
}


impl Eq for ParamString {}

impl PartialOrd<String> for ParamString {
    fn partial_cmp(&self, other: &String) -> Option<Ordering> {
        self.0.partial_cmp(other)
    }
}

impl PartialOrd<str> for ParamString {
    fn partial_cmp(&self, other: &str) -> Option<Ordering> {
        self.0.partial_cmp(other)
    }
}

impl PartialOrd<&str> for ParamString {
    fn partial_cmp(&self, other: &&str) -> Option<Ordering> {
        self.0.partial_cmp(*other)
    }
}


impl ParamString {
    /// Pushes a new string onto the `ParamString`.
    ///
    /// This could incur up to two allocations.
    pub fn push_str(&mut self, s: &str) {
        let new= format!("{self}{s}");
        self.0 = SharedString::try_from(new)
            .expect("ParamString with more than 4294967295 characters.");
    }

    // Extracts a string slice containing the entire `ParamString`.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    pub fn test_push_str() {
        let mut string = ParamString::from("Hello");
        string.push_str(" World");
        assert_eq!("Hello World", string);
    }
}