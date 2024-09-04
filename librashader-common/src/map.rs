/// Fast optimized hash map type for small sets.
pub type FastHashMap<K, V> =
    halfbrown::SizedHashMap<K, V, core::hash::BuildHasherDefault<rustc_hash::FxHasher>, 32>;

/// A string with small string optimizations up to 23 bytes.
pub type ShortString = smartstring::SmartString<smartstring::LazyCompact>;

pub use halfbrown;
