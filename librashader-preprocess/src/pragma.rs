use std::str::FromStr;
use nom::bytes::complete::{is_not, tag, take_until, take_while};
use nom::combinator::map_res;
use nom::IResult;
use nom::number::complete::float;
use nom::sequence::delimited;
use librashader::{ShaderFormat, ShaderParameter};
use crate::PreprocessError;

#[derive(Debug)]
pub(crate) struct ShaderMeta {
    pub(crate) format: ShaderFormat,
    pub(crate) parameters: Vec<ShaderParameter>,
    pub(crate) name: Option<String>
}

fn parse_parameter_string(input: &str) -> Result<ShaderParameter, PreprocessError>{
    fn parse_parameter_string_inner(input: &str) -> IResult<&str, ShaderParameter> {
        let (input, _) = tag("#pragma parameter ")(input)?;
        let (input, name) = take_while(|c| c != ' ')(input)?;
        let (input, _) = tag(" ")(input)?;
        let (input, description) = delimited(tag("\""), is_not("\""), tag("\""))(input)?;
        let (input, _) = tag(" ")(input)?;
        let (input, initial) = float(input)?;
        let (input, _) = tag(" ")(input)?;
        let (input, minimum) = float(input)?;
        let (input, _) = tag(" ")(input)?;
        let (input, maximum) = float(input)?;
        let (input, _) = tag(" ")(input)?;
        let (input, step) = float(input)?;
        Ok((input, ShaderParameter {
            id: name.to_string(),
            description: description.to_string(),
            initial,
            minimum,
            maximum,
            step
        }))
    }

    if let Ok((_, parameter)) = parse_parameter_string_inner(input) {
        Ok(parameter)
    } else {
        Err(PreprocessError::PragmaParseError(input.to_string()))
    }
}

pub(crate) fn parse_pragma_meta(source: impl AsRef<str>) -> Result<ShaderMeta, PreprocessError> {
    let source = source.as_ref();
    let mut parameters: Vec<ShaderParameter> = Vec::new();
    let mut format = ShaderFormat::default();
    let mut name = None;
    for line in source.lines() {
        if line.starts_with("#pragma parameter ") {
            let parameter = parse_parameter_string(line)?;
            if let Some(existing) = parameters.iter().find(|&p| p.id == parameter.id) {
                if existing != &parameter {
                    return Err(PreprocessError::DuplicatePragmaError(parameter.id))
                }
            } else {
                parameters.push(parameter);
            }
        }

        if line.starts_with("#pragma format ") {
            if format != ShaderFormat::Unknown {
                return Err(PreprocessError::DuplicatePragmaError(line.to_string()))
            }

            let format_string = line["#pragma format ".len()..].trim();
            format = ShaderFormat::from_str(&format_string)?;

            if format == ShaderFormat::Unknown {
                return Err(PreprocessError::UnknownShaderFormat)
            }
        }

        if line.starts_with("#pragma name ") {
            if name.is_some() {
                return Err(PreprocessError::DuplicatePragmaError(line.to_string()));
            }

            name = Some(line.trim().to_string())
        }
    }

    Ok(ShaderMeta { name, format, parameters })
}

#[cfg(test)]
mod test {
    use librashader::ShaderParameter;
    use crate::pragma::parse_parameter_string;

    #[test]
    fn parses_parameter_pragma() {
        assert_eq!(ShaderParameter {
            id: "exc".to_string(),
            description: "orizontal correction hack (games where players stay at center)".to_string(),
            initial: 0.0,
            minimum: -10.0,
            maximum: 10.0,
            step: 0.25
        }, parse_parameter_string(r#"#pragma parameter exc "orizontal correction hack (games where players stay at center)" 0.0 -10.0 10.0 0.25"#).unwrap())
    }
}