use std::collections::{BTreeMap, BTreeSet};

pub fn render_template(
    template: &str,
    inputs: &BTreeMap<String, String>,
) -> Result<String, String> {
    let mut result = template.to_string();

    // Track which inputs are actually used in the template
    let mut used_inputs = BTreeSet::new();

    for (var, value) in inputs {
        let placeholder = format!("{{{}}}", var);
        if result.contains(&placeholder) {
            used_inputs.insert(var.clone());
        }
        result = result.replace(&placeholder, value);
    }

    // Check for unbound variables (variables in template that have no input)
    let unbound = find_unbound_variables(template, inputs);
    if !unbound.is_empty() {
        return Err(format!("Template variable not bound: {}", unbound.join(", ")));
    }

    // Check for unused inputs (declared inputs not referenced in template)
    let unused: Vec<&String> = inputs
        .keys()
        .filter(|k| !used_inputs.contains(*k))
        .collect();
    if !unused.is_empty() {
        return Err(format!(
            "declared input '{}' is not referenced in template",
            unused.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
        ));
    }

    Ok(result)
}

fn find_unbound_variables(template: &str, inputs: &BTreeMap<String, String>) -> Vec<String> {
    let mut vars = Vec::new();
    let chars: Vec<char> = template.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '{' && i + 1 < chars.len() && chars[i + 1] != '{' {
            let mut var = String::new();
            let mut j = i + 1;
            while j < chars.len() && chars[j] != '}' {
                var.push(chars[j]);
                j += 1;
            }
            if j < chars.len() && chars[j] == '}' {
                if !var.is_empty() && !inputs.contains_key(&var) {
                    vars.push(var.clone());
                }
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }

    vars
}
