use crate::{PreprocessError, SourceOutput};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::str::Lines;

#[cfg(feature = "line_directives")]
const GL_GOOGLE_CPP_STYLE_LINE_DIRECTIVE: &str =
    "#extension GL_GOOGLE_cpp_style_line_directive : require";

fn read_file(path: impl AsRef<Path>) -> Result<String, PreprocessError> {
    let path = path.as_ref();
    let mut source = String::new();
    File::open(path)
        .and_then(|mut f| f.read_to_string(&mut source))
        .map_err(|e| PreprocessError::IOError(path.to_path_buf(), e))?;
    Ok(source)
}

pub fn read_source(path: impl AsRef<Path>) -> Result<String, PreprocessError> {
    let path = path.as_ref();
    let source = read_file(path)?;
    let mut output = String::new();

    let source = source.trim();
    let mut lines = source.lines();

    if let Some(header) = lines.next() {
        if !header.starts_with("#version ") {
            return Err(PreprocessError::MissingVersionHeader);
        }
        output.push_line(header);
    } else {
        return Err(PreprocessError::UnexpectedEof);
    }

    #[cfg(feature = "line_directives")]
    output.push_line(GL_GOOGLE_CPP_STYLE_LINE_DIRECTIVE);

    output.mark_line(2, path.file_name().and_then(|f| f.to_str()).unwrap_or(""));
    preprocess(lines, path, &mut output)?;

    Ok(output)
}

fn preprocess(
    lines: Lines,
    file_name: impl AsRef<Path>,
    output: &mut String,
) -> Result<(), PreprocessError> {
    let file_name = file_name.as_ref();
    let include_path = file_name.parent().unwrap();
    let file_name = file_name.file_name().and_then(|f| f.to_str()).unwrap_or("");

    for (line_no, line) in lines.enumerate() {
        if let Some(include_file) = line.strip_prefix("#include ") {
            let include_file = include_file.trim().trim_matches('"');
            if include_file.is_empty() {
                return Err(PreprocessError::UnexpectedEol(line_no));
            }

            let mut include_path = include_path.to_path_buf();
            include_path.push(include_file);

            let source = read_file(&include_path)?;
            let source = source.trim();
            let lines = source.lines();

            let include_file = include_path
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("");
            output.mark_line(1, include_file);
            preprocess(lines, include_path, output)?;
            output.mark_line(line_no + 1, file_name);
            continue;
        }
        if line.starts_with("#endif") || line.starts_with("#pragma") {
            output.push_line(line);
            output.mark_line(line_no + 2, file_name);
            continue;
        }

        output.push_line(line)
    }
    Ok(())
}
