pub fn extract_output(
    output: &str,
    extract_spec: Option<&crate::config::ExtractSpec>,
) -> Result<String, String> {
    let default_spec = crate::config::ExtractSpec {
        extract_type: "identity".to_string(),
        start: None,
        end: None,
    };
    let spec = extract_spec.unwrap_or(&default_spec);

    match spec.extract_type.as_str() {
        "identity" => Ok(output.to_string()),
        "tag" => {
            let start = spec
                .start
                .as_ref()
                .ok_or("Tag extraction requires 'start'")?;
            let end = spec
                .end
                .as_ref()
                .ok_or("Tag extraction requires 'end'")?;

            extract_tag(output, start, end)
        }
        _ => Err(format!("Unknown extract type: {}", spec.extract_type)),
    }
}

pub fn extract_tag(output: &str, start: &str, end: &str) -> Result<String, String> {
    let start_idx = output
        .find(start)
        .ok_or("No tag match found".to_string())?;

    let content_start = start_idx + start.len();
    let content_end = output[content_start..]
        .find(end)
        .ok_or("No closing tag found".to_string())? + content_start;

    let content = &output[content_start..content_end];

    // Check for multiple matches
    let remaining = &output[content_end..];
    if remaining.find(start).is_some() {
        return Err("Multiple tag matches found".to_string());
    }

    Ok(content.trim().to_string())
}
