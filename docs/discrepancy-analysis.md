# Specification vs Implementation Discrepancy Analysis

This document compares the implementation against the specification in `docs/SPEC.md`.

## Overview

After analyzing the codebase, here are the key findings:

## 1. **Error Handling Format** ❌

**SPEC Section**: 7.1 Error Model
> All errors MUST be written to stderr as:
> ```
> ERROR <CODE>: <message>
> ```

**Current Implementation**:
```rust
eprintln!("ERROR {}: {}", e.code(), e.message());
```

**Status**: ✅ IMPLEMENTED - The main function correctly writes to stderr in the required format.

---

## 2. **Input Binding - Optional Inputs** ❌

**SPEC Section 4.4**:
> *Exactly one variable MAY be declared with `source: stdin`*

**Current Implementation** (`binding.rs`):
```rust
"cli" => {
    if let Some(value) = cli_inputs.get(var) {
        value.clone()
    } else if spec.required {
        return Err(format!("Missing required input: {}", var));
    } else {
        // Optional input not provided - use empty string
        String::new()
    }
}
```

**Issue**: The spec says stdin can be optional (exactly one MAY be stdin), but the current implementation doesn't enforce that exactly one input has `source: stdin`. It also doesn't handle optional stdin inputs correctly.

**Status**: ⚠️ PARTIALLY IMPLEMENTED - Optional CLI inputs work, but stdin handling may not follow spec strictly.

---

## 3. **Template Variable Detection** ❌

**SPEC Section 4.5**:
> Variables are referenced as `{variable_name}`.

**Current Implementation** (`template.rs`):
The `find_unbound_variables` function looks for `{variable_name}` pattern but doesn't handle escaped braces `{{}}`.

**SPEC Section 4.5**:
> No escape mechanism in v2.

**Status**: ✅ CONSISTENT - No escape mechanism is implemented, which matches the spec.

---

## 4. **Tag Extraction** ❌

**SPEC Section 4.6**:
> * MUST use non-greedy matching
> * Nesting is NOT supported
> * Zero match → `EXTRACTION_NO_MATCH`
> * Multiple matches → `EXTRACTION_MULTIPLE_MATCH`

**Current Implementation** (`extract.rs`):
```rust
fn extract_tag(output: &str, start: &str, end: &str) -> Result<String, String> {
    let start_idx = output
        .find(start)
        .ok_or("No tag match found".to_string())?;

    let content_start = start_idx + start.len();
    let content_end = output
        .find(end)
        .ok_or("No closing tag found".to_string())?;

    let content = &output[content_start..content_end];

    // Check for multiple matches
    let remaining = &output[content_end..];
    if remaining.find(start).is_some() {
        return Err("Multiple tag matches found".to_string());
    }

    Ok(content.trim().to_string())
}
```

**Issues**:
1. ❌ **Does not use non-greedy matching** - It finds the FIRST occurrence of `start` tag, then the FIRST occurrence of `end` tag after it. This could incorrectly match nested structures.
2. ❌ **Multiple match detection is wrong** - The current check `if remaining.find(start).is_some()` only checks if there's another opening tag, but doesn't properly detect multiple complete tag pairs.
3. ❌ **Doesn't handle case where `end` tag appears before `start` tag** - The `content_end` could be before `content_start` if tags are in wrong order.

**Status**: ❌ NOT CONFORMANT - Tag extraction doesn't follow SPEC requirements for non-greedy matching and multiple match detection.

---

## 5. **Model Override** ❌

**SPEC Section 5.1**:
> The `model` field MAY be overridden at runtime via `--model`:
> ```bash
> anchorgen run fast_apply --model fast-apply-1b --set ...
> ```
> Resolution order: `--model` flag > config `model` field.

**Current Implementation** (`main.rs`):
```rust
let model = args.model_override.as_deref().unwrap_or(&gen_cfg.model);
```

**Status**: ✅ IMPLEMENTED - Model override via `--model` flag works as specified.

---

## 6. **File I/O Mode** ❌

**SPEC Section 3.2**:
> `--input` and `--output` MUST NOT be used partially.
> If one is specified, the other MUST also be specified.
> Violation → `CLI_USAGE_ERROR`

**Current Implementation**:
The validation exists in `main.rs`:
```rust
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
```

**Status**: ✅ IMPLEMENTED - Correctly enforces that `--input` and `--output` must be used together.

---

## 7. **Unknown --set Key Handling** ❌

**SPEC Section 4.4**:
> Unknown `--set` key → `INPUT_UNKNOWN`

**Current Implementation**:
The binding code only checks if a key exists in `cli_inputs`, but doesn't validate against declared inputs at the CLI level. The validation happens in `bind_inputs` which generates an error like "Unknown key: xyz" but the error code mapping might not be correct.

**Status**: ⚠️ PARTIALLY IMPLEMENTED - Unknown keys are detected but the error message format may not match spec exactly.

---

## 8. **LLM API** ❌

**SPEC Section 5.2**:
> AnchorGen supports OpenAI-compatible APIs only.
> ```
> POST /v1/chat/completions
> ```

**Current Implementation** (`llm.rs`):
The code sends a POST request to `{base_url}/v1/chat/completions` with the correct request body structure.

**Status**: ✅ IMPLEMENTED - Uses OpenAI-compatible API as required.

---

## 9. **Environment Variables** ❌

**SPEC Section 5.3**:
> ```
> ANCHORGEN_BASE_URL   # required
> ANCHORGEN_API_KEY    # required
> ```
> Missing environment variable → `LLM_CONFIG_MISSING`

**Current Implementation** (`llm.rs`):
```rust
pub fn resolve_llm_config() -> Result<LlmConfig, String> {
    let base_url = std::env::var("ANCHORGEN_BASE_URL")
        .map_err(|_| "Missing ANCHORGEN_BASE_URL")?;
    let api_key = std::env::var("ANCHORGEN_API_KEY")
        .map_err(|_| "Missing ANCHORGEN_API_KEY")?;
    // ...
}
```

**Status**: ✅ IMPLEMENTED - Correctly requires both environment variables.

---

## 10. **Config File Schema** ❌

**SPEC Section 4.3**:
```yaml
generators:
  <name>:
    model: <string>               # required
    inputs:
      <variable>:
        source: stdin | cli       # required
        required: true | false    # default: true
    prompt:
      template: |
        ...{variable}...
    extract:
      type: identity | tag
      start: <string>             # required if type is tag
      end: <string>               # required if type is tag
```

**Current Implementation** (`config.rs`):
- ✅ Correctly deserializes the config with serde
- ✅ Validates unknown top-level fields
- ✅ Supports `model`, `inputs`, `prompt.template`, and optional `extract`

**Status**: ✅ IMPLEMENTED - Config schema is correctly implemented.

---

## 11. **Error Codes** ❌

**SPEC Section 7.2**:
> All failures MUST use this set [of error codes].

**Current Implementation**: The code uses the following error codes:
- ✅ `CLI_USAGE_ERROR`
- ✅ `CONFIG_INVALID_FIELD`
- ✅ `CONFIG_INVALID`
- ✅ `GENERATOR_NOT_FOUND`
- ✅ `INPUT_MISSING`
- ✅ `INPUT_UNKNOWN`
- ✅ `INPUT_SOURCE_INVALID`
- ✅ `TEMPLATE_VAR_MISSING`
- ✅ `TEMPLATE_VAR_UNUSED`
- ✅ `EXTRACTION_NO_MATCH`
- ✅ `EXTRACTION_MULTIPLE_MATCH`
- ✅ `LLM_CONFIG_MISSING`
- ✅ `LLM_REQUEST_FAILED`
- ✅ `IO_ERROR`

**Status**: ✅ IMPLEMENTED - All error codes match the spec.

---

## 12. **AnchorScope Integration** ❌

**SPEC Section 3.3**:
> When used with AnchorScope's `pipe --file-io` mode, AnchorScope passes `--input` and `--output` paths to AnchorGen.

**Current Implementation**: The implementation supports `--input` and `--output` flags for file I/O mode.

**Status**: ✅ IMPLEMENTED - File I/O mode is supported.

---

## 13. **Determinism Guarantees** ❌

**SPEC Section 8**:
> AnchorGen guarantees that the following are deterministic given identical inputs, config, and environment variables:
> 1. Input binding
> 2. Prompt rendering
> 3. Extraction
> 4. Error output

**Current Implementation**: The implementation is mostly deterministic, but:
- ❌ HashMap iteration order is used in some places which is non-deterministic in Rust

**Status**: ⚠️ PARTIALLY IMPLEMENTED - HashMap ordering could cause non-deterministic error messages.

---

## 14. **Missing Features** ❌

**SPEC Section 4.3**:
> Unknown top-level fields → `CONFIG_INVALID_FIELD`

**Current Implementation**: The custom deserializer in `config.rs` checks for unknown fields.

**Status**: ✅ IMPLEMENTED

---

## Summary Table

| # | Feature | Spec | Implementation | Status |
|---|---------|------|----------------|--------|
| 1 | Error Format | stderr: `ERROR <CODE>: <msg>` | ✅ | ✅ |
| 2 | Optional Inputs | `required: false` | ⚠️ Partial | ⚠️ |
| 3 | Template Vars | `{var}` syntax | ✅ | ✅ |
| 4 | Tag Extraction | Non-greedy, no nesting | ❌ | ❌ |
| 5 | Model Override | `--model` flag | ✅ | ✅ |
| 6 | File I/O | `--input` + `--output` | ✅ | ✅ |
| 7 | Unknown --set | `INPUT_UNKNOWN` | ⚠️ Partial | ⚠️ |
| 8 | LLM API | OpenAI-compatible | ✅ | ✅ |
| 9 | Env Vars | `ANCHORGEN_*` | ✅ | ✅ |
| 10 | Config Schema | YAML with validation | ✅ | ✅ |
| 11 | Error Codes | Closed set | ✅ | ✅ |
| 12 | AnchorScope | File I/O mode | ✅ | ✅ |
| 13 | Determinism | Stable iteration | ⚠️ HashMap | ⚠️ |
| 14 | Config Validation | Unknown field errors | ✅ | ✅ |

---

## Critical Issues to Fix

1. **Tag Extraction** - Does not implement non-greedy matching
2. **Determinism** - HashMap iteration order may cause non-deterministic error messages
3. **Optional Inputs** - May not handle all edge cases correctly

## Recommendations

1. Fix the tag extraction to properly implement non-greedy matching
2. Replace HashMap with BTreeMap for deterministic iteration order
3. Add comprehensive tests for optional input handling
4. Document the escape mechanism (or lack thereof) more clearly
