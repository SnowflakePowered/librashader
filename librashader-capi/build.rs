use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};

pub fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut buf = BufWriter::new(Vec::new());
    cbindgen::generate(crate_dir)
        .expect("Unable to generate bindings")
        .write(&mut buf);

    let bytes = buf.into_inner().expect("Unable to extract bytes");
    let string = String::from_utf8(bytes).expect("Unable to create string");
    // let string = string.replace("CHD_ERROR_", "CHDERR_");
    File::create("librashader.h")
        .expect("Unable to open file")
        .write_all(string.as_bytes())
        .expect("Unable to write bindings.")
}
