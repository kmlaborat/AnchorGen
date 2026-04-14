use std::collections::BTreeMap;

pub type BoundInputs = BTreeMap<String, String>;

pub fn bind_inputs(
    generator: &crate::config::GeneratorSpec,
    read_content: &str,
    cli_inputs: &BTreeMap<String, String>,
) -> Result<BoundInputs, String> {
    let mut bound = BTreeMap::new();

    // Validate stdin inputs per SPEC 4.4: exactly one variable MAY have source: stdin
    let stdin_count = generator
        .inputs
        .values()
        .filter(|spec| spec.source == "stdin")
        .count();

    if stdin_count > 1 {
        return Err("Exactly one input can have source: stdin".to_string());
    }

    for (var, spec) in &generator.inputs {
        let value = match spec.source.as_str() {
            "stdin" => read_content.to_string(),
            "cli" => {
                if let Some(value) = cli_inputs.get(var) {
                    value.clone()
                } else if spec.required {
                    return Err(format!("Missing required input: {}", var));
                } else {
                    // Optional input not provided - use empty string
                    // Per SPEC, all template variables must be bound
                    String::new()
                }
            }
            _ => return Err(format!("Invalid input source: {}", spec.source)),
        };
        bound.insert(var.clone(), value);
    }

    Ok(bound)
}
