# AnchorGen Specification Compliance Implementation Plan

> **REQUIRED SUB-SKILL:** Use the executing-plans skill to implement this plan task-by-task.

**Goal:** Implement a stateless CLI tool that renders prompts from templates, calls OpenAI-compatible LLMs, and extracts deterministic results - fully compliant with docs/SPEC.md

**Architecture:** Rust CLI using clap for argument parsing, with modules for config parsing, input binding, template rendering, LLM calls, and extraction. All error handling follows a closed set of error codes.

**Tech Stack:** Rust 2021 edition, clap (CLI), serde/serde_yaml (config), reqwest (HTTP), serde_json (error output)

---

## Current State

**Issues Found:**
- All modules are in `src/main.rs` - needs refactoring into separate files
- Compilation fails with 14 errors (missing module definitions, type annotations)
- CLI uses `--file` and `--anchor` instead of `--input` and `--output`
- Config format differs from SPEC (uses `read.*` and `cli.*` sources instead of `stdin` and `cli`)
- Error format uses JSON instead of `ERROR <CODE>: <message>` to stderr

---

## Task 0: Create Module File Structure

**Files:**
- Create: `src/config.rs`
- Create: `src/binding.rs`
- Create: `src/template.rs`
- Create: `src/extract.rs`
- Create: `src/llm.rs`
- Create: `src/generator.rs`
- Create: `src/anchorscope.rs`
- Create: `src/mock_llm.rs`
- Modify: `src/main.rs` (will become CLI entry point only)

**Step 1: Extract config.rs**

Create `src/config.rs` with:
- `Config` struct
- `GeneratorSpec` struct
- `InputSpec` struct
- `ExtractSpec` struct
- `ConfigError` enum

**Step 2: Extract binding.rs**

Create `src/binding.rs` with:
- `bind_inputs()` function
- Input source validation

**Step 3: Extract template.rs**

Create `src/template.rs` with:
- `render_template()` function
- Variable substitution logic

**Step 4: Extract extract.rs**

Create `src/extract.rs` with:
- `extract_output()` function
- Identity and tag extraction

**Step 5: Extract llm.rs**

Create `src/llm.rs` with:
- `LlmConfig` struct
- `resolve_llm_config()` function
- `generate()` function for OpenAI API call

**Step 6: Extract generator.rs**

Create `src/generator.rs` with:
- `get_generator()` function

**Step 7: Extract anchorscope.rs**

Create `src/anchorscope.rs` with:
- `ReadOutput` struct
- `run_read()` function
- `run_write()` function

**Step 8: Extract mock_llm.rs**

Create `src/mock_llm.rs` with:
- Mock LLM logic for testing

**Step 9: Update main.rs**

Refactor main.rs to:
- Import all modules
- Keep only `main()`, `parse_args()`, `run()`, `call_llm()` functions

---

## Task 1: Implement config.rs with Full Validation

**Files:**
- Create: `src/config.rs`

**Step 1: Define structs**

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub generators: std::collections::HashMap<String, GeneratorSpec>,
}

#[derive(Debug, Deserialize)]
pub struct GeneratorSpec {
    pub model: String,
    pub inputs: std::collections::HashMap<String, InputSpec>,
    pub prompt: PromptSpec,
    #[serde(default)]
    pub extract: Option<ExtractSpec>,
}

#[derive(Debug, Deserialize)]
pub struct InputSpec {
    pub source: String,
    #[serde(default = "default_true")]
    pub required: bool,
}

#[derive(Debug, Deserialize)]
pub struct PromptSpec {
    pub template: String,
}

#[derive(Debug, Deserialize)]
pub struct ExtractSpec {
    pub r#type: String,
    #[serde(default)]
    pub start: Option<String>,
    #[serde(default)]
    pub end: Option<String>,
}

fn default_true() -> bool {
    true
}
```

**Step 2: Define error types**

```rust
use serde_yaml;
use std::io;

#[derive(Debug)]
pub enum ConfigError {
    Io(io::Error),
    Parse(String),
    InvalidField { field: String },
}

impl From<io::Error> for ConfigError {
    fn from(err: io::Error) -> Self {
        ConfigError::Io(err)
    }
}

impl From<serde_yaml::Error> for ConfigError {
    fn from(err: serde_yaml::Error) -> Self {
        ConfigError::Parse(err.to_string())
    }
}
```

**Step 3: Implement Config methods**

```rust
impl Config {
    pub fn from_file(path: &str) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&content)?;
        
        // Validate: check for unknown top-level fields
        Self::validate(&config)?;
        
        Ok(config)
    }
    
    fn validate(&self) -> Result<(), ConfigError> {
        // Only allow "generators" at top level
        // This requires custom deserialization or post-validation
        Ok(())
    }
}
```

**Step 4: Run to verify**

Run: `cargo check`
Expected: Compiles with no errors

---

## Task 2: Implement binding.rs

**Files:**
- Create: `src/binding.rs`

**Step 1: Define bindings**

```rust
use std::collections::HashMap;

pub type BoundInputs = HashMap<String, String>;

pub fn bind_inputs(
    generator: &crate::config::GeneratorSpec,
    read_output: &crate::anchorscope::ReadOutput,
    cli_inputs: &HashMap<String, String>,
) -> Result<BoundInputs, String> {
    let mut bound = HashMap::new();
    
    for (var, spec) in &generator.inputs {
        let value = match spec.source.as_str() {
            "stdin" => read_output.content.clone(),
            "cli" => cli_inputs
                .get(var)
                .map(|s| s.clone())
                .ok_or_else(|| format!("Missing required input: {}", var))?,
            _ => return Err(format!("Invalid input source: {}", spec.source)),
        };
        bound.insert(var.clone(), value);
    }
    
    // Check for unused declared inputs (optional validation)
    // Check for undeclared inputs in template (optional validation)
    
    Ok(bound)
}
```

**Step 2: Run to verify**

Run: `cargo check`
Expected: Compiles with no errors

---

## Task 3: Implement template.rs

**Files:**
- Create: `src/template.rs`

**Step 1: Implement rendering**

```rust
use std::collections::HashMap;

pub fn render_template(
    template: &str,
    inputs: &HashMap<String, String>,
) -> Result<String, String> {
    let mut result = template.to_string();
    
    for (var, value) in inputs {
        let placeholder = format!("{{{}}}", var);
        result = result.replace(&placeholder, value);
    }
    
    // Check for unbound variables
    for var in find_variables(template) {
        if !inputs.contains_key(&var) {
            return Err(format!("Template variable not bound: {}", var));
        }
    }
    
    Ok(result)
}

fn find_variables(template: &str) -> Vec<String> {
    let mut vars = Vec::new();
    let mut chars = template.chars().peekable();
    
    while let Some(c) = chars.next() {
        if c == '{' {
            let mut var = String::new();
            while let Some(&next) = chars.peek() {
                if next == '}' {
                    chars.next();
                    if !var.is_empty() {
                        vars.push(var.clone());
                    }
                    break;
                }
                var.push(next);
                chars.next();
            }
        }
    }
    
    vars
}
```

**Step 2: Run to verify**

Run: `cargo check`
Expected: Compiles with no errors

---

## Task 4: Implement extract.rs

**Files:**
- Create: `src/extract.rs`

**Step 1: Implement extraction**

```rust
pub fn extract_output(
    output: &str,
    extract_spec: Option<&crate::config::ExtractSpec>,
) -> Result<String, String> {
    let spec = extract_spec.unwrap_or(&crate::config::ExtractSpec {
        r#type: "identity".to_string(),
        start: None,
        end: None,
    });
    
    match spec.r#type.as_str() {
        "identity" => Ok(output.to_string()),
        "tag" => {
            let start = spec.start.as_ref().ok_or("Tag extraction requires 'start'")?;
            let end = spec.end.as_ref().ok_or("Tag extraction requires 'end'")?;
            
            extract_tag(output, start, end)
        }
        _ => Err(format!("Unknown extract type: {}", spec.r#type)),
    }
}

fn extract_tag(output: &str, start: &str, end: &str) -> Result<String, String> {
    let start_idx = output
        .find(start)
        .ok_or("No tag match found")?;
    
    let content_start = start_idx + start.len();
    let content_end = output
        .find(end)
        .ok_or("No closing tag found")?;
    
    let content = &output[content_start..content_end];
    
    // Check for multiple matches
    let remaining = &output[content_end..];
    if remaining.find(start).is_some() {
        return Err("Multiple tag matches found");
    }
    
    Ok(content.trim().to_string())
}
```

**Step 2: Run to verify**

Run: `cargo check`
Expected: Compiles with no errors

---

## Task 5: Implement llm.rs

**Files:**
- Create: `src/llm.rs`

**Step 1: Define LLM config**

```rust
use reqwest;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct LlmConfig {
    pub base_url: String,
    pub api_key: String,
}

pub fn resolve_llm_config() -> Result<LlmConfig, String> {
    let base_url = std::env::var("ANCHORGEN_BASE_URL")
        .map_err(|_| "Missing ANCHORGEN_BASE_URL")?;
    let api_key = std::env::var("ANCHORGEN_API_KEY")
        .map_err(|_| "Missing ANCHORGEN_API_KEY")?;
    
    Ok(LlmConfig { base_url, api_key })
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
}

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Deserialize)]
struct Message {
    content: String,
}

pub fn generate(prompt: &str, config: &LlmConfig, model: &str) -> Result<String, String> {
    let client = reqwest::blocking::Client::new();
    
    let url = format!("{}/v1/chat/completions", config.base_url);
    
    let request = ChatRequest {
        model: model.to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        }],
    };
    
    let response = client
        .post(&url)
        .header("Authorization", &format!("Bearer {}", config.api_key))
        .json(&request)
        .send()
        .map_err(|e| format!("LLM request failed: {}", e))?;
    
    if !response.status().is_success() {
        return Err(format!("LLM API returned status: {}", response.status()));
    }
    
    let chat_response: ChatResponse = response
        .json()
        .map_err(|e| format!("Failed to parse LLM response: {}", e))?;
    
    chat_response
        .choices
        .first()
        .map(|c| c.message.content.trim().to_string())
        .ok_or("LLM response contains no choices".to_string())
}
```

**Step 2: Run to verify**

Run: `cargo check`
Expected: Compiles with no errors

---

## Task 6: Implement generator.rs

**Files:**
- Create: `src/generator.rs`

**Step 1: Implement generator lookup**

```rust
pub fn get_generator<'a>(
    config: &'a crate::config::Config,
    name: &str,
) -> Result<&'a crate::config::GeneratorSpec, String> {
    config
        .generators
        .get(name)
        .ok_or_else(|| format!("Generator '{}' not found", name))
}
```

**Step 2: Run to verify**

Run: `cargo check`
Expected: Compiles with no errors

---

## Task 7: Implement anchorscope.rs

**Files:**
- Create: `src/anchorscope.rs`

**Step 1: Define read output and functions**

```rust
#[derive(Debug)]
pub struct ReadOutput {
    pub content: String,
    pub hash: String,
    pub start_line: usize,
    pub end_line: usize,
}

pub fn run_read(file: &str, anchor: &str) -> Result<ReadOutput, String> {
    // Use anchorscope CLI to read
    let output = std::process::Command::new("anchorscope")
        .arg("read")
        .arg("--file")
        .arg(file)
        .arg("--anchor")
        .arg(anchor)
        .output()
        .map_err(|e| format!("Failed to run anchorscope read: {}", e))?;
    
    if !output.status.success() {
        return Err(format!("anchorscope read failed: {}", 
            String::from_utf8_lossy(&output.stderr)));
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_read_output(&stdout)
}

fn parse_read_output(output: &str) -> Result<ReadOutput, String> {
    let mut content = String::new();
    let mut hash = String::new();
    let mut start_line = 0;
    let mut end_line = 0;
    
    for line in output.lines() {
        if line.starts_with("content=") {
            content = line.trim_start_matches("content=").to_string();
        } else if line.starts_with("hash=") {
            hash = line.trim_start_matches("hash=").to_string();
        } else if line.starts_with("start_line=") {
            start_line = line.trim_start_matches("start_line=").parse().map_err(|_| {
                "Invalid start_line value"
            })?;
        } else if line.starts_with("end_line=") {
            end_line = line.trim_start_matches("end_line=").parse().map_err(|_| {
                "Invalid end_line value"
            })?;
        }
    }
    
    Ok(ReadOutput {
        content,
        hash,
        start_line,
        end_line,
    })
}

pub fn run_write(
    file: &str,
    anchor: &str,
    expected_hash: &str,
    replacement: &str,
) -> Result<(), String> {
    // Write replacement to a temp file first
    // Then call anchorscope write
    
    // For now, use stdin
    let mut child = std::process::Command::new("anchorscope")
        .arg("write")
        .arg("--file")
        .arg(file)
        .arg("--anchor")
        .arg(anchor)
        .arg("--expected-hash")
        .arg(expected_hash)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to run anchorscope write: {}", e))?;
    
    {
        let stdin = child.stdin.as_mut().ok_or("Failed to open stdin")?;
        stdin
            .write_all(replacement.as_bytes())
            .map_err(|e| format!("Failed to write replacement: {}", e))?;
    }
    
    let output = child
        .wait_with_output()
        .map_err(|e| format!("Failed to wait for anchorscope: {}", e))?;
    
    if !output.status.success() {
        return Err(format!("anchorscope write failed: {}", 
            String::from_utf8_lossy(&output.stderr)));
    }
    
    Ok(())
}
```

**Step 2: Run to verify**

Run: `cargo check`
Expected: Compiles with no errors

---

## Task 8: Implement mock_llm.rs

**Files:**
- Create: `src/mock_llm.rs`

**Step 1: Mock LLM for testing**

```rust
pub fn mock_generate(prompt: &str) -> Result<String, String> {
    let expected = std::env::var("ANCHORGEN_EXPECTED_PROMPT")
        .map_err(|_| "MOCK_NOT_FOUND: ANCHORGEN_EXPECTED_PROMPT not set")?;
    
    if prompt != expected {
        return Err(format!(
            "MOCK_NOT_FOUND: prompt mismatch.\nExpected:\n{}\n\nGot:\n{}",
            expected, prompt
        ));
    }
    
    std::env::var("ANCHORGEN_MOCK_RESPONSE")
        .map_err(|_| "MOCK_NOT_FOUND: ANCHORGEN_MOCK_RESPONSE not set")
}
```

**Step 2: Run to verify**

Run: `cargo check`
Expected: Compiles with no errors

---

## Task 9: Update main.rs to Match SPEC CLI

**Files:**
- Modify: `src/main.rs`

**Step 1: Rewrite main.rs**

```rust
use std::collections::HashMap;
use std::process;

mod config;
mod binding;
mod template;
mod extract;
mod anchorscope;
mod llm;
mod generator;
mod mock_llm;

use crate::anchorscope::ReadOutput;
use crate::binding::bind_inputs;
use crate::config::Config;
use crate::extract::extract_output;
use crate::generator::get_generator;
use crate::llm::{resolve_llm_config, generate};
use crate::template::render_template;

fn main() {
    let args = parse_args();
    
    if let Err(e) = run(args) {
        eprintln!("ERROR {}: {}", e.code(), e.message());
        process::exit(1);
    }
}

fn parse_args() -> Result<Args, AppError> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        return Err(AppError::new("CLI_USAGE_ERROR", "Usage: anchorgen run <generator_name> [options]"));
    }
    
    let mut iter = args.iter().map(|s| s.as_str()).skip(1);
    
    // First arg must be "run"
    let subcmd = iter.next().ok_or_else(|| AppError::new("CLI_USAGE_ERROR", "missing subcommand"))?;
    if subcmd != "run" {
        return Err(AppError::new("CLI_USAGE_ERROR", "first argument must be 'run'"));
    }
    
    // Generator name
    let generator = iter.next().ok_or_else(|| AppError::new("CLI_USAGE_ERROR", "missing generator name"))?.to_string();
    
    let mut config_path = String::new();
    let mut input_path = None;
    let mut output_path = None;
    let mut model_override = None;
    let mut cli_inputs = HashMap::new();
    
    while let Some(arg) = iter.next() {
        match arg {
            "--config" => {
                if config_path.is_empty() {
                    config_path = iter.next().ok_or_else(|| AppError::new("CLI_USAGE_ERROR", "missing value for --config"))?.to_string();
                } else {
                    return Err(AppError::new("CLI_USAGE_ERROR", "--config specified multiple times"));
                }
            }
            "--input" => {
                if input_path.is_none() {
                    input_path = Some(iter.next().ok_or_else(|| AppError::new("CLI_USAGE_ERROR", "missing value for --input"))?.to_string());
                } else {
                    return Err(AppError::new("CLI_USAGE_ERROR", "--input specified multiple times"));
                }
            }
            "--output" => {
                if output_path.is_none() {
                    output_path = Some(iter.next().ok_or_else(|| AppError::new("CLI_USAGE_ERROR", "missing value for --output"))?.to_string());
                } else {
                    return Err(AppError::new("CLI_USAGE_ERROR", "--output specified multiple times"));
                }
            }
            "--set" => {
                let kv = iter.next().ok_or_else(|| AppError::new("CLI_USAGE_ERROR", "missing value for --set"))?;
                if !kv.contains('=') {
                    return Err(AppError::new("CLI_USAGE_ERROR", format!("invalid --set format: '{}', expected key=value", kv)));
                }
                let mut parts = kv.splitn(2, '=');
                let key = parts.next().unwrap().to_string();
                let value = parts.next().unwrap().to_string();
                cli_inputs.insert(key, value);
            }
            "--model" => {
                if model_override.is_none() {
                    model_override = Some(iter.next().ok_or_else(|| AppError::new("CLI_USAGE_ERROR", "missing value for --model"))?.to_string());
                } else {
                    return Err(AppError::new("CLI_USAGE_ERROR", "--model specified multiple times"));
                }
            }
            _ => {
                return Err(AppError::new("CLI_USAGE_ERROR", format!("unexpected argument: '{}'", arg)));
            }
        }
    }
    
    if config_path.is_empty() {
        config_path = "anchorgen.yaml".to_string();
    }
    
    // --input and --output must be used together
    match (input_path.is_some(), output_path.is_some()) {
        (true, false) => return Err(AppError::new("CLI_USAGE_ERROR", "--input and --output must be specified together")),
        (false, true) => return Err(AppError::new("CLI_USAGE_ERROR", "--input and --output must be specified together")),
        _ => {}
    }
    
    Ok(Args {
        generator,
        config_path,
        input_path,
        output_path,
        model_override,
        cli_inputs,
    })
}

fn run(args: Args) -> Result<(), AppError> {
    // 1. Load config
    let config = Config::from_file(&args.config_path)
        .map_err(|e| AppError::new("CONFIG_INVALID", e.to_string()))?;
    
    // 2. Get generator
    let gen_cfg = get_generator(&config, &args.generator)
        .map_err(|_| AppError::new("GENERATOR_NOT_FOUND", format!("Generator '{}' not found", args.generator)))?;
    
    // 3. Read input
    let input_content = if let Some(path) = &args.input_path {
        std::fs::read_to_string(path)
            .map_err(|e| AppError::new("IO_ERROR", format!("file not found: {}", path)))?
    } else {
        let mut content = String::new();
        std::io::stdin()
            .read_to_string(&mut content)
            .map_err(|e| AppError::new("IO_ERROR", format!("read failure: {}", e)))?;
        
        // Validate UTF-8
        if !content.is_utf8() {
            return Err(AppError::new("IO_ERROR", "invalid UTF-8"));
        }
        content
    };
    
    // 4. Bind inputs
    let bound_inputs = bind_inputs(gen_cfg, &input_content, &args.cli_inputs)
        .map_err(|e| {
            if e.contains("Unknown") {
                AppError::new("INPUT_UNKNOWN", e)
            } else if e.contains("Missing") {
                AppError::new("INPUT_MISSING", e)
            } else if e.contains("Invalid") {
                AppError::new("INPUT_SOURCE_INVALID", e)
            } else {
                AppError::new("INPUT_MISSING", e)
            }
        })?;
    
    // 5. Render template
    let prompt = render_template(&gen_cfg.prompt.template, &bound_inputs)
        .map_err(|e| {
            if e.contains("not found") || e.contains("not bound") {
                AppError::new("TEMPLATE_VAR_MISSING", e)
            } else if e.contains("Unused") {
                AppError::new("TEMPLATE_VAR_UNUSED", e)
            } else {
                AppError::new("TEMPLATE_VAR_MISSING", e)
            }
        })?;
    
    // 6. Call LLM
    let model = args.model_override.as_deref().unwrap_or(&gen_cfg.model);
    let llm_output = if std::env::var("ANCHORGEN_USE_MOCK").is_ok() {
        mock_llm::mock_generate(&prompt)
            .map_err(|e| {
                if e.contains("MOCK_NOT_FOUND") {
                    AppError::new("MOCK_NOT_FOUND", e)
                } else {
                    AppError::new("LLM_REQUEST_FAILED", e)
                }
            })
    } else {
        let resolved = resolve_llm_config()
            .map_err(|e| AppError::new("LLM_CONFIG_MISSING", e))?;
        generate(&prompt, &resolved, model)
            .map_err(|e| {
                if e.contains("Missing ANCHORGEN") {
                    AppError::new("LLM_CONFIG_MISSING", e)
                } else {
                    AppError::new("LLM_REQUEST_FAILED", e)
                }
            })
    }?;
    
    // 7. Extract output
    let output = extract_output(&llm_output, gen_cfg.extract.as_ref())
        .map_err(|e| {
            if e.contains("No match") || e.contains("No tag") {
                AppError::new("EXTRACTION_NO_MATCH", e)
            } else if e.contains("Multiple") {
                AppError::new("EXTRACTION_MULTIPLE_MATCH", e)
            } else {
                AppError::new("EXTRACTION_NO_MATCH", e)
            }
        })?;
    
    // 8. Write output
    if let Some(path) = &args.output_path {
        std::fs::write(path, &output)
            .map_err(|e| AppError::new("IO_ERROR", format!("write failure: {}", e)))?;
    } else {
        println!("{}", output);
    }
    
    Ok(())
}

#[derive(Debug)]
struct Args {
    generator: String,
    config_path: String,
    input_path: Option<String>,
    output_path: Option<String>,
    model_override: Option<String>,
    cli_inputs: HashMap<String, String>,
}

#[derive(Debug)]
struct AppError {
    code: &'static str,
    message: String,
}

impl AppError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        AppError {
            code,
            message: message.into(),
        }
    }
    
    fn code(&self) -> &'static str {
        self.code
    }
    
    fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
```

**Step 2: Run to verify**

Run: `cargo check`
Expected: Compiles with no errors

---

## Task 10: Add Integration Tests

**Files:**
- Create: `tests/integration_tests.rs`

**Step 1: Create test file**

```rust
use std::process::Command;

#[test]
fn test_command_line_help() {
    let output = Command::new("cargo")
        .args(["run", "--", "--help"])
        .output()
        .expect("Failed to execute command");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("run"));
    assert!(stdout.contains("--config"));
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
    // This test would require a config with required inputs
    // For now, just verify the error format
    let output = Command::new("cargo")
        .args(["run", "--", "run", "fast_apply", "--config", "config.example.yaml"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to execute command");
    
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR"));
}
```

**Step 2: Run tests**

Run: `cargo test`
Expected: Tests run with some expected failures

---

## Task 11: Verify Build and Fix Remaining Issues

**Step 1: Full build**

Run: `cargo build --release`
Expected: Successful build with no errors

**Step 2: Run clippy**

Run: `cargo clippy`
Expected: No warnings or fix warnings

**Step 3: Test with config.example.yaml**

Run: `echo "test" | cargo run -- run fast_apply --config config.example.yaml --set update_snippet="test"`
Expected: Output matches SPEC format

---

## Summary

**Files to create:**
1. `src/config.rs` - Config parsing with validation
2. `src/binding.rs` - Input binding
3. `src/template.rs` - Template rendering
4. `src/extract.rs` - Output extraction
5. `src/llm.rs` - LLM API calls
6. `src/generator.rs` - Generator lookup
7. `src/anchorscope.rs` - AnchorScope integration
8. `src/mock_llm.rs` - Mock for testing
9. `tests/integration_tests.rs` - Integration tests

**Files to modify:**
1. `src/main.rs` - Rewrite to match SPEC CLI

**Error codes to use:**
- `INPUT_MISSING`
- `INPUT_UNKNOWN`
- `INPUT_SOURCE_INVALID`
- `CONFIG_INVALID`
- `CONFIG_INVALID_FIELD`
- `TEMPLATE_VAR_MISSING`
- `TEMPLATE_VAR_UNUSED`
- `EXTRACTION_NO_MATCH`
- `EXTRACTION_MULTIPLE_MATCH`
- `GENERATOR_NOT_FOUND`
- `LLM_CONFIG_MISSING`
- `LLM_REQUEST_FAILED`
- `CLI_USAGE_ERROR`
- `IO_ERROR: <details>`

**Output format:**
- Errors to stderr: `ERROR <CODE>: <message>`
- Success: stdout for content
