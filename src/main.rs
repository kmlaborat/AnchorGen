use std::collections::BTreeMap;
use std::io::Read;
use std::process;

mod config;
mod binding;
mod template;
pub mod extract;
mod anchorscope;
mod llm;
mod generator;
mod mock_llm;

use crate::binding::bind_inputs;
use crate::config::Config;
use crate::extract::extract_output;
use crate::generator::get_generator;
use crate::llm::{resolve_llm_config, generate};
use crate::template::render_template;

fn main() {
    let args = parse_args().expect("Failed to parse arguments");

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
    let mut cli_inputs = BTreeMap::new();

    while let Some(arg) = iter.next() {
        match arg {
            "--config" => {
                if config_path.is_empty() {
                    config_path = iter
                        .next()
                        .ok_or_else(|| AppError::new("CLI_USAGE_ERROR", "missing value for --config"))?
                        .to_string();
                } else {
                    return Err(AppError::new("CLI_USAGE_ERROR", "--config specified multiple times"));
                }
            }
            "--input" => {
                if input_path.is_none() {
                    input_path = Some(
                        iter.next()
                            .ok_or_else(|| AppError::new("CLI_USAGE_ERROR", "missing value for --input"))?
                            .to_string(),
                    );
                } else {
                    return Err(AppError::new("CLI_USAGE_ERROR", "--input specified multiple times"));
                }
            }
            "--output" => {
                if output_path.is_none() {
                    output_path = Some(
                        iter.next()
                            .ok_or_else(|| AppError::new("CLI_USAGE_ERROR", "missing value for --output"))?
                            .to_string(),
                    );
                } else {
                    return Err(AppError::new("CLI_USAGE_ERROR", "--output specified multiple times"));
                }
            }
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
            "--model" => {
                if model_override.is_none() {
                    model_override = Some(
                        iter.next()
                            .ok_or_else(|| AppError::new("CLI_USAGE_ERROR", "missing value for --model"))?
                            .to_string(),
                    );
                } else {
                    return Err(AppError::new("CLI_USAGE_ERROR", "--model specified multiple times"));
                }
            }
            _ => {
                return Err(AppError::new(
                    "CLI_USAGE_ERROR",
                    format!("unexpected argument: '{}'", arg),
                ));
            }
        }
    }

    if config_path.is_empty() {
        config_path = "anchorgen.yaml".to_string();
    }

    // --input and --output must be used together
    match (input_path.is_some(), output_path.is_some()) {
        (true, false) => {
            return Err(AppError::new(
                "CLI_USAGE_ERROR",
                "--input and --output must be specified together",
            ))
        }
        (false, true) => {
            return Err(AppError::new(
                "CLI_USAGE_ERROR",
                "--input and --output must be specified together",
            ))
        }
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
        .map_err(|e| {
            let err_str = e.to_string();
            if err_str.contains("unknown field") {
                // Extract the field name from the error message
                AppError::new("CONFIG_INVALID_FIELD", err_str)
            } else {
                AppError::new("CONFIG_INVALID", err_str)
            }
        })?;

    // 2. Get generator
    let gen_cfg = get_generator(&config, &args.generator)
        .map_err(|_| AppError::new("GENERATOR_NOT_FOUND", format!("Generator '{}' not found", args.generator)))?;

    // 3. Read input
    let input_content = if let Some(path) = &args.input_path {
        std::fs::read_to_string(path)
            .map_err(|_| AppError::new("IO_ERROR", format!("file not found: {}", path)))?
    } else {
        let mut bytes = Vec::new();
        std::io::stdin().read_to_end(&mut bytes)
            .map_err(|_| AppError::new("IO_ERROR", "read failure".to_string()))?;

        // Validate UTF-8
        std::str::from_utf8(&bytes)
            .map_err(|_| AppError::new("IO_ERROR", "invalid UTF-8".to_string()))?;

        String::from_utf8(bytes)
            .map_err(|_| AppError::new("IO_ERROR", "UTF-8 conversion error".to_string()))?
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
            let e_lower = e.to_lowercase();
            if e_lower.contains("not found") || e_lower.contains("not bound") {
                AppError::new("TEMPLATE_VAR_MISSING", e)
            } else if e_lower.contains("unused") || e_lower.contains("is not referenced") {
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
    cli_inputs: BTreeMap<String, String>,
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
