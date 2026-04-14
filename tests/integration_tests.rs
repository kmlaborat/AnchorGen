use std::process::Command;

#[test]
fn test_usage_message() {
    let output = Command::new("cargo")
        .args(["run", "--"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("CLI_USAGE_ERROR"));
    assert!(stderr.contains("run"));
}

#[test]
fn test_missing_config() {
    let output = Command::new("cargo")
        .args(["run", "--", "run", "test_gen"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR"));
    assert!(stderr.contains("CONFIG_INVALID"));
}

#[test]
fn test_generator_not_found() {
    let output = Command::new("cargo")
        .args(["run", "--", "run", "nonexistent_gen", "--config", "config.example.yaml"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR"));
    assert!(stderr.contains("GENERATOR_NOT_FOUND"));
}

#[test]
fn test_missing_required_input() {
    let output = Command::new("cargo")
        .args(["run", "--", "run", "fast_apply", "--config", "config.example.yaml"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR"));
}

#[test]
fn test_input_missing_when_required() {
    let output = Command::new("cargo")
        .args(["run", "--", "run", "fast_apply", "--config", "config.example.yaml", "--input", "test.txt"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR"));
    assert!(stderr.contains("CLI_USAGE_ERROR"));
}

#[test]
fn test_invalid_utf8_input() {
    let output = Command::new("cargo")
        .args(["run", "--", "run", "fast_apply", "--config", "config.example.yaml", "--input", "tests/invalid_utf8.bin", "--output", "tests/output.txt", "--set", "update_snippet=test"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR"));
    assert!(stderr.contains("LLM_CONFIG_MISSING"));
}

#[test]
fn test_set_from_file() {
    let output = Command::new("cargo")
        .args([
            "run", "--", "run", "fast_apply",
            "--config", "config.example.yaml",
            "--input", "tests/invalid_utf8.bin",
            "--output", "tests/output.txt",
            "--set", "update_snippet=@tests/nonexistent_file.txt"
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("IO_ERROR"));
    assert!(stderr.contains("file not found"));
}

#[test]
fn test_unknown_config_field() {
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
    
    std::fs::remove_file("tests/invalid_config.yaml").ok();
}

#[test]
fn test_optional_input() {
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
    
    let output = Command::new("cargo")
        .args([
            "run", "--", "run", "test_optional",
            "--config", "tests/optional_config.yaml",
            "--input", "tests/invalid_utf8.bin",
            "--output", "tests/output.txt"
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("LLM_CONFIG_MISSING"));
    
    std::fs::remove_file("tests/optional_config.yaml").ok();
}

#[test]
fn test_unused_template_variable() {
    let config_with_unused = r#"
generators:
  test_unused:
    model: test-model
    inputs:
      used_input:
        source: stdin
      unused_input:
        source: cli
        required: false
    prompt:
      template: "test {used_input}"
"#;
    
    std::fs::write("tests/unused_config.yaml", config_with_unused).unwrap();
    
    let output = Command::new("cargo")
        .args([
            "run", "--", "run", "test_unused",
            "--config", "tests/unused_config.yaml",
            "--input", "tests/invalid_utf8.bin",
            "--output", "tests/output.txt"
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR"));
    assert!(stderr.contains("TEMPLATE_VAR_UNUSED"));
    assert!(stderr.contains("unused_input"));
    
    std::fs::remove_file("tests/unused_config.yaml").ok();
}

#[test]
fn test_tag_extraction_basic() {
    let config = r#"
generators:
  test_tag:
    model: test-model
    inputs:
      input1:
        source: stdin
    prompt:
      template: "test {input1}"
    extract:
      type: tag
      start: "<tag>"
      end: "</tag>"
"#;
    std::fs::write("tests/tag_config.yaml", config).unwrap();
    
    let output = Command::new("cargo")
        .args([
            "run", "--", "run", "test_tag",
            "--config", "tests/tag_config.yaml",
            "--input", "tests/invalid_utf8.bin",
            "--output", "tests/output.txt",
            "--set", "input1=<tag>extract this</tag>"
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("LLM_CONFIG_MISSING"));
    
    std::fs::remove_file("tests/tag_config.yaml").ok();
}
