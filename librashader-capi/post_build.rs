use std::{env, fs};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

pub fn main() {
    // Do not update files on docsrs
    if env::var("DOCS_RS").is_ok() {
        return;
    }

    // Create headers.
    let crate_dir = env::var("CRATE_MANIFEST_DIR").unwrap();
    let mut buf = BufWriter::new(Vec::new());
    cbindgen::generate(crate_dir)
        .expect("Unable to generate bindings")
        .write(&mut buf);

    let bytes = buf.into_inner().expect("Unable to extract bytes");
    let string = String::from_utf8(bytes).expect("Unable to create string");
    // let string = string.replace("CHD_ERROR_", "CHDERR_");
    File::create(PathBuf::from(env::var("CRATE_OUT_DIR").unwrap()).join("librashader.h"))
        .expect("Unable to open file")
        .write_all(string.as_bytes())
        .expect("Unable to write bindings.");


    if cfg!(target_os = "linux") {
        let artifacts = &["liblibrashader.so", "liblibrashader.a"];
        for artifact in artifacts {
            let ext = artifact.strip_prefix("lib").unwrap();
            fs::rename(PathBuf::from(env::var("CRATE_OUT_DIR").unwrap()).join(artifact), PathBuf::from(env::var("CRATE_OUT_DIR").unwrap()).join(ext)).unwrap();
        }
    }
}
