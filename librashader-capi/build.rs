use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

pub fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut buf = BufWriter::new(Vec::new());
    let output_file = target_dir()
        .join("librashader.h")
        .display()
        .to_string();
    cbindgen::generate(crate_dir)
        .expect("Unable to generate bindings")
        .write(&mut buf);

    let bytes = buf.into_inner().expect("Unable to extract bytes");
    let string = String::from_utf8(bytes).expect("Unable to create string");
    // let string = string.replace("CHD_ERROR_", "CHDERR_");
    File::create(output_file)
        .expect("Unable to open file")
        .write_all(string.as_bytes())
        .expect("Unable to write bindings.")
}

/// Find the location of the `target/` directory. Note that this may be
/// overridden by `cmake`, so we also need to check the `CARGO_TARGET_DIR`
/// variable.
fn target_dir() -> PathBuf {
    if let Ok(target) = env::var("CARGO_TARGET_DIR") {
        PathBuf::from(target)
    } else {
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("target")
    }
}