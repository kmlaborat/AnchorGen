# AnchorGen SPEC Compliance Remediation Plan

> **REQUIRED SUB-SKILL:** Use the executing-plans skill to implement this plan task-by-task.

**Goal:** Implement the 5 missing SPEC features identified in the code review to achieve 100% compliance with docs/SPEC.md

**Architecture:** Add file-based CLI input support, strict config validation, optional input handling, and template variable usage validation. Maintain backward compatibility with existing functionality.

**Tech Stack:** Rust 2021 edition, existing module structure (config.rs, binding.rs, main.rs, template.rs)

---

## Current State Summary

**Implemented (85%):**
- Core pipeline: INPUT → BIND → RENDER → GENERATE → EXTRACT → OUTPUT
- stdin/stdout and --input/--output file I/O
- Template variable substitution
- Identity and tag extraction
- Error codes and formatting
- Mock LLM for testing
- 6 integration tests

**Missing (15%):**
1. `--set key=@path` file-based CLI inputs
2. Config field validation (unknown fields → CONFIG_INVALID_FIELD)
3. Optional inputs (`required: false`)
4. TEMPLATE_VAR_UNUSED validation
5. Proper use of CONFIG_INVALID_FIELD error

---

## Task 0: Add --set key=@path Support

**Files:**
- Modify: `src/main.rs` (parse_args function)
- Test: `tests/integration_tests.rs`

**Step 1: Write test for file-based --set**

```rust
#[test]
fn test_set_from_file() {
    // Create a test input file
    std::fs::write("tests/test_input.txt", "test content from file").unwrap();
    
    let output = Command::new("cargo")
        .args([
            "run", "--", "run", "fast_apply",
            "--config", "config.example.yaml",
            "--input", "tests/invalid_utf8.bin",
            "--output", "tests/output.txt",
            "--set", "update_snippet=@tests/test_input.txt"
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should fail at LLM_CONFIG_MISSING, not at input parsing
    assert!(stderr.contains("LLM_CONFIG_MISSING"));
    
    // Clean up
    std::fs::remove_file("tests/test_input.txt").ok();
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test integration_tests test_set_from_file`
Expected: FAIL (compilation error: argument format not handled)

**Step 3: Implement @path parsing in parse_args**

Modify `src/main.rs` in the `--set` handler:

```rust
"--set" => {
    let kv = iter
        .next()
        .ok_or_else(|| AppError::new("CLI_USAGE_ERROR", "missing value for --set"))?;
    if !kv.contains('=') {
        return Err(AppError::new(
            "CLI_USAGE_ERROR",
            format!("invalid --set format: '{}', expected key=value", kv),
        ));
    }
    let mut parts = kv.splitn(2, '=');
    let key = parts.next().unwrap().to_string();
    let value_str = parts.next().unwrap();
    
    // Check if value starts with @ (file reference)
    let value = if value_str.starts_with('@') {
        let file_path = &value_str[1..]; // Remove @ prefix
        std::fs::read_to_string(file_path)
            .map_err(|_| AppError::new("IO_ERROR", format!("file not found: {}", file_path)))?
    } else {
        value_str.to_string()
    };
    
    cli_inputs.insert(key, value);
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test --test integration_tests test_set_from_file`
Expected: PASS (fails at LLM_CONFIG_MISSING as expected)

**Step 5: Commit**

```bash
git add src/main.rs tests/integration_tests.rs
git commit -m "feat: add --set key=@path support for file-based CLI inputs"
```

---

## Task 1: Add Config Field Validation

**Files:**
- Modify: `src/config.rs`
- Test: `tests/integration_tests.rs`

**Step 1: Write test for unknown config field**

```rust
#[test]
fn test_unknown_config_field() {
    // Create a config with an unknown top-level field
    let invalid_config = r#"
unknown_field: some_value
generators:
  test_gen:
    model: test-model
    inputs:
      input1:
        source: stdin
    prompt:
      template: "test {input1}"
"#;
    
    std::fs::write("tests/invalid_config.yaml", invalid_config).unwrap();
    
    let output = Command::new("cargo")
        .args([
            "run", "--", "run", "test_gen",
            "--config", "tests/invalid_config.yaml"
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR"));
    assert!(stderr.contains("CONFIG_INVALID_FIELD"));
    assert!(stderr.contains("unknown_field"));
    
    // Clean up
    std::fs::remove_file("tests/invalid_config.yaml").ok();
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test integration_tests test_unknown_config_field`
Expected: FAIL (assertion fails - CONFIG_INVALID_FIELD not found)

**Step 3: Implement custom config deserialization**

Modify `src/config.rs` to use a wrapper struct for validation:

```rust
use serde::de::Deserializer;
use serde::Deserialize;
use std::collections::HashSet;

// Custom deserializer that rejects unknown top-level fields
impl<'de> Deserialize<'de> for Config {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{MapAccess, Visitor};
        use std::fmt;

        struct ConfigVisitor;

        impl<'de> Visitor<'de> for ConfigVisitor {
            type Value = Config;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct Config with only 'generators' field")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Config, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut generators = None;
                let allowed_fields: HashSet<&str> = ["generators"].iter().cloned().collect();

                while let Some(key) = map.next_key::<String>()? {
                    if !allowed_fields.contains(key.as_str()) {
                        return Err(serde::de::Error::custom(format!(
                            "unknown field '{}', only 'generators' is allowed",
                            key
                        )));
                    }
                    if key == "generators" {
                        if generators.is_some() {
                            return Err(serde::de::Error::duplicate_field("generators"));
                        }
                        generators = Some(map.next_value()?);
                    }
                }

                Ok(Config {
                    generators: generators.unwrap_or_default(),
                })
            }
        }

        deserializer.deserialize_map(ConfigVisitor)
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test --test integration_tests test_unknown_config_field`
Expected: PASS

**Step 5: Commit**

```bash
git add src/config.rs tests/integration_tests.rs
git commit -m "feat: add strict config field validation with CONFIG_INVALID_FIELD"
```

---

## Task 2: Add Optional Input Support

**Files:**
- Modify: `src/config.rs`
- Modify: `src/binding.rs`
- Test: `tests/integration_tests.rs`

**Step 1: Restore required field in InputSpec**

Modify `src/config.rs`:

```rust
#[derive(Debug, Deserialize)]
pub struct InputSpec {
    pub source: String,
    #[serde(default = "default_true")]
    pub required: bool,
}

fn default_true() -> bool {
    true
}
```

**Step 2: Write test for optional input**

```rust
#[test]
fn test_optional_input() {
    // Create a config with an optional input
    let config_with_optional = r#"
generators:
  test_optional:
    model: test-model
    inputs:
      required_input:
        source: stdin
        required: true
      optional_input:
        source: cli
        required: false
    prompt:
      template: "test {required_input}{optional_input}"
"#;
    
    std::fs::write("tests/optional_config.yaml", config_with_optional).unwrap();
    
    // Test with optional input provided
    let output = Command::new("cargo")
        .args([
            "run", "--", "run", "test_optional",
            "--config", "tests/optional_config.yaml",
            "--input", "tests/invalid_utf8.bin",
            "--output", "tests/output.txt",
            "--set", "optional_input=provided"
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to execute command");

    // Should fail at LLM_CONFIG_MISSING, not at binding
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("LLM_CONFIG_MISSING"));
    
    // Test without optional input (should also work)
    let output2 = Command::new("cargo")
        .args([
            "run", "--", "run", "test_optional",
            "--config", "tests/optional_config.yaml",
            "--input", "tests/invalid_utf8.bin",
            "--output", "tests/output.txt"
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to execute command");

    assert!(!output2.status.success());
    let stderr2 = String::from_utf8_lossy(&output2.stderr);
    // Should also fail at LLM_CONFIG_MISSING, not INPUT_MISSING
    assert!(stderr2.contains("LLM_CONFIG_MISSING"));
    
    // Clean up
    std::fs::remove_file("tests/optional_config.yaml").ok();
}
```

**Step 3: Update bind_inputs to handle optional inputs**

Modify `src/binding.rs`:

```rust
pub fn bind_inputs(
    generator: &crate::config::GeneratorSpec,
    read_content: &str,
    cli_inputs: &HashMap<String, String>,
) -> Result<BoundInputs, String> {
    let mut bound = HashMap::new();

    for (var, spec) in &generator.inputs {
        let value = match spec.source.as_str() {
            "stdin" => read_content.to_string(),
            "cli" => {
                if let Some(value) = cli_inputs.get(var) {
                    value.clone()
                } else if spec.required {
                    return Err(format!("Missing required input: {}", var));
                } else {
                    // Optional input not provided - use empty string or skip
                    // Per SPEC, all template variables must be bound, so use empty string
                    String::new()
                }
            }
            _ => return Err(format!("Invalid input source: {}", spec.source)),
        };
        bound.insert(var.clone(), value);
    }

    Ok(bound)
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test --test integration_tests test_optional_input`
Expected: PASS

**Step 5: Commit**

```bash
git add src/config.rs src/binding.rs tests/integration_tests.rs
git commit -m "feat: add optional input support with required field"
```

---

## Task 3: Add TEMPLATE_VAR_UNUSED Validation

**Files:**
- Modify: `src/template.rs`
- Test: `tests/integration_tests.rs`

**Step 1: Write test for unused variable**

```rust
#[test]
fn test_unused_template_variable() {
    // Create a config with an unused input
    let config_with_unused = r#"
generators:
  test_unused:
    model: test-model
    inputs:
      used_input:
        source: stdin
      unused_input:
        source: cli
    prompt:
      template: "test {used_input}"
"#;
    
    std::fs::write("tests/unused_config.yaml", config_with_unused).unwrap();
    
    let output = Command::new("cargo")
        .args([
            "run", "--", "run", "test_unused",
            "--config", "tests/unused_config.yaml",
            "--input", "tests/invalid_utf8.bin"
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR"));
    assert!(stderr.contains("TEMPLATE_VAR_UNUSED"));
    assert!(stderr.contains("unused_input"));
    
    // Clean up
    std::fs::remove_file("tests/unused_config.yaml").ok();
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test integration_tests test_unused_template_variable`
Expected: FAIL (assertion fails - TEMPLATE_VAR_UNUSED not found)

**Step 3: Add unused variable detection in render_template**

Modify `src/template.rs`:

```rust
pub fn render_template(
    template: &str,
    inputs: &HashMap<String, String>,
) -> Result<String, String> {
    let mut result = template.to_string();

    // Track which inputs are used
    let mut used_inputs = HashSet::new();

    for (var, value) in inputs {
        let placeholder = format!("{{{}}}", var);
        if result.contains(&placeholder) {
            used_inputs.insert(var.clone());
        }
        result = result.replace(&placeholder, value);
    }

    // Check for unbound variables
    let unbound = find_unbound_variables(template, inputs);
    if !unbound.is_empty() {
        return Err(format!("Template variable not bound: {}", unbound.join(", ")));
    }

    // Check for unused inputs
    let unused: Vec<&String> = inputs
        .keys()
        .filter(|k| !used_inputs.contains(*k))
        .collect();
    if !unused.is_empty() {
        return Err(format!(
            "Template variable unused: {}",
            unused.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
        ));
    }

    Ok(result)
}

use std::collections::HashSet;
```

**Step 4: Update main.rs to map the new error**

Modify `src/main.rs` in the render_template error handling:

```rust
let prompt = render_template(&gen_cfg.prompt.template, &bound_inputs)
    .map_err(|e| {
        if e.contains("not found") || e.contains("not bound") {
            AppError::new("TEMPLATE_VAR_MISSING", e)
        } else if e.contains("unused") || e.contains("Unused") {
            AppError::new("TEMPLATE_VAR_UNUSED", e)
        } else {
            AppError::new("TEMPLATE_VAR_MISSING", e)
        }
    })?;
```

**Step 5: Run test to verify it passes**

Run: `cargo test --test integration_tests test_unused_template_variable`
Expected: PASS

**Step 6: Commit**

```bash
git add src/template.rs src/main.rs tests/integration_tests.rs
git commit -m "feat: add TEMPLATE_VAR_UNUSED validation for unused inputs"
```

---

## Task 4: Update Error Messages for Clarity

**Files:**
- Modify: `src/config.rs`
- Modify: `src/binding.rs`
- Modify: `src/template.rs`

**Step 1: Improve error message for CONFIG_INVALID_FIELD**

Modify `src/config.rs` in the custom deserializer:

```rust
return Err(serde::de::Error::custom(format!(
    "unknown field '{}' at top level, only 'generators' is allowed",
    key
)));
```

**Step 2: Improve error message for TEMPLATE_VAR_UNUSED**

Modify `src/template.rs`:

```rust
return Err(format!(
    "declared input '{}' is not referenced in template",
    unused.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
));
```

**Step 3: Run all tests**

Run: `cargo test`
Expected: All tests pass

**Step 4: Commit**

```bash
git add src/config.rs src/template.rs
git commit -m "refactor: improve error messages for better diagnostics"
```

---

## Task 5: Final Verification and Documentation

**Files:**
- Modify: `README.md`
- Test: Run full test suite

**Step 1: Update README with new features**

Add to README.md under "Usage" section:

```markdown
### File-based CLI Inputs

Use `--set key=@path` to load input from a file:

```bash
anchorgen run fast_apply \
  --config config.yaml \
  --input src/file.rs \
  --set update_snippet=@patch.txt
```

### Optional Inputs

Mark inputs as optional in config:

```yaml
inputs:
  required_input:
    source: stdin
    required: true
  optional_input:
    source: cli
    required: false
```

### Strict Config Validation

Unknown config fields now produce `CONFIG_INVALID_FIELD` errors.
```

**Step 2: Run full test suite**

Run: `cargo test`
Expected: All tests pass

**Step 3: Build release**

Run: `cargo build --release`
Expected: Successful build with no errors

**Step 4: Commit**

```bash
git add README.md
git commit -m "docs: update README with new features"
```

---

## Summary

**Files to modify:**
1. `src/main.rs` - Add @path parsing
2. `src/config.rs` - Add field validation and required field
3. `src/binding.rs` - Handle optional inputs
4. `src/template.rs` - Add unused variable detection
5. `tests/integration_tests.rs` - Add 4 new tests
6. `README.md` - Document new features

**New tests to add:**
1. `test_set_from_file` - File-based CLI inputs
2. `test_unknown_config_field` - Config validation
3. `test_optional_input` - Optional input handling
4. `test_unused_template_variable` - Unused variable detection

**Expected outcome:**
- 100% SPEC compliance
- 10/10 integration tests passing
- Clean release build
- Updated documentation

---

## Execution Notes

- Each task builds on the previous one
- Run tests after each task to catch regressions
- The order matters: add @path support first, then config validation, then optional inputs, then unused variable detection
- All changes are backward compatible with existing configs
