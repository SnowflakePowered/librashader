use std::error::Error;

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

// todo: make this return error
macro_rules! assume_d3d12_init {
    ($value:ident, $call:literal) => {
        let $value = $value.expect($call);
    };
    (mut $value:ident, $call:literal) => {
        let mut $value = $value.expect($call);
    };
}

/// Macro for unwrapping result of a D3D function.
pub(crate) use assume_d3d12_init;
