# AnchorGen Specification v0.1.0

## Prompt-Template-Based LLM Generation Tool

**AnchorGen is a stateless CLI tool that renders a prompt from a declared template,
calls an OpenAI-compatible LLM, and returns the result.**

It is designed to integrate with AnchorScope's external tool pipeline,
and can also be used standalone.

The key words "MUST", "MUST NOT", "SHOULD", and "MAY" in this document are to
be interpreted as described in [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119).

---

## 1. Concept (Informative)

### 1.1 Role in the AnchorScope Ecosystem

AnchorScope manages the boundary: reading a scoped region, verifying its hash,
and writing back a verified replacement. It does not prescribe what happens
between read and write.

AnchorGen fills that gap. It is the external tool that:

1. Receives content from AnchorScope (via pipe or file)
2. Renders a prompt from a declared template
3. Calls an LLM
4. Returns the result to AnchorScope (via pipe or file)

AnchorGen has no knowledge of anchors, hashes, or file state.
All write-safety guarantees are enforced by AnchorScope.

### 1.2 Standalone Use

AnchorGen can also be used independently of AnchorScope.
Any task expressible as a prompt template with explicit inputs is supported:
code transformation, translation, summarization, structured extraction, and so on.

### 1.3 Design Principles

* **Stateless**: no buffer, no session, no implicit context
* **Declarative**: behavior is fully determined by config and explicit inputs
* **Deterministic at the boundary**: prompt rendering and extraction are deterministic;
  only LLM output is non-deterministic
* **Transparent**: no provider-specific logic; all routing is delegated to a proxy

---

## 2. Execution Model (Normative)

### 2.1 Pipeline

```
INPUT → BIND → RENDER → GENERATE → EXTRACT → OUTPUT
```

### 2.2 Stage Definitions

#### INPUT

Read raw content from stdin (default) or from the file specified by `--input`.

Content MUST be valid UTF-8.
Invalid UTF-8 → `IO_ERROR: invalid UTF-8`

#### BIND

Resolve all declared inputs.

* `stdin` or `--input` content is bound to the variable declared as `source: stdin`
* `--set key=value` arguments are bound to variables declared as `source: cli`
* `--set key=@path` reads the value from the file at `path`

Failures:
* Required input missing → `INPUT_MISSING`
* Undeclared `--set` key provided → `INPUT_UNKNOWN`

#### RENDER

Render the prompt template with all bound inputs.

Rendering MUST be deterministic.
All template variables MUST be bound.

Failures:
* Template variable not bound → `TEMPLATE_VAR_MISSING`
* Declared input not used in template → `TEMPLATE_VAR_UNUSED`

#### GENERATE

Call the LLM via the configured endpoint.

No retry. No fallback. No implicit context.

Failures:
* LLM call fails → `LLM_REQUEST_FAILED`

#### EXTRACT

Produce the final output string from the LLM response.

Extraction MUST be deterministic.

Supported types: `identity`, `tag` (see Section 4.3).

Failures:
* No match → `EXTRACTION_NO_MATCH`
* Multiple matches → `EXTRACTION_MULTIPLE_MATCH`

#### OUTPUT

Write the extracted string to stdout (default) or to the file specified by `--output`.

---

## 3. I/O Model (Normative)

### 3.1 Default: stdin / stdout

```bash
echo "..." | anchorgen run <generator> --set key=value
```

Input is read from stdin.
Output is written to stdout.

### 3.2 File I/O Mode

```bash
anchorgen run <generator> --input /path/to/content --output /path/to/result --set key=value
```

`--input` and `--output` MUST NOT be used partially.
If one is specified, the other MUST also be specified.

Violation → `CLI_USAGE_ERROR`

### 3.3 AnchorScope Integration

When used with AnchorScope's `pipe --file-io` mode, AnchorScope passes
`--input` and `--output` paths to AnchorGen.
AnchorGen reads from `--input` and writes to `--output`.
AnchorScope then validates and normalizes the output upon re-entry.

```bash
# stdout mode
as.pipe --true-id {id} --out \
  | anchorgen run fast_apply --set update_snippet="..." \
  | as.pipe --true-id {id} --in

# file-io mode
as.pipe --true-id {id} \
  --tool "anchorgen run fast_apply --set update_snippet='...'" \
  --input /path/to/input \
  --output /path/to/output
```

> AnchorGen does not perform UTF-8 validation or CRLF normalization on output.
> These are AnchorScope's responsibility upon re-entry.

---

## 4. Configuration Model (Normative)

### 4.1 Config File

Default path: `anchorgen.yaml` in the current directory.
Override with `--config <path>`.

### 4.2 Top-Level Schema

```yaml
generators:
  <name>: GeneratorSpec
```

Unknown top-level fields → `CONFIG_INVALID_FIELD`
Missing required fields → `CONFIG_INVALID`

### 4.3 Generator Specification

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

### 4.4 Inputs

#### Allowed Sources

| Source  | Meaning                                      |
| ------- | -------------------------------------------- |
| `stdin` | Content from stdin or `--input` file         |
| `cli`   | Value provided via `--set key=value` or `--set key=@path` |

**Constraints:**

* Exactly one variable MAY be declared with `source: stdin`
* All inputs MUST be resolvable at BIND time
* Missing required input → `INPUT_MISSING`
* Unknown `--set` key → `INPUT_UNKNOWN`

### 4.5 Prompt Template

Variables are referenced as `{variable_name}`.

**Constraints:**

* All template variables MUST be declared in `inputs`
* All declared inputs MUST appear in the template
* Unknown variable in template → `TEMPLATE_VAR_MISSING`
* Unused declared input → `TEMPLATE_VAR_UNUSED`
* No escape mechanism in v2

### 4.6 Extraction

#### identity

```yaml
extract:
  type: identity
```

Returns full LLM output as-is.

#### tag

```yaml
extract:
  type: tag
  start: "<updated-code>"
  end: "</updated-code>"
```

**Rules:**

* MUST match exactly one region
* MUST use non-greedy matching
* Nesting is NOT supported
* Zero match → `EXTRACTION_NO_MATCH`
* Multiple matches → `EXTRACTION_MULTIPLE_MATCH`

---

## 5. Model and LLM Integration (Normative)

### 5.1 Model Identifier

Each generator declares a `model` field.
This value is passed as-is to the `model` parameter of the LLM API request.

```yaml
generators:
  fast_apply:
    model: fast-apply-7b
  translate:
    model: gpt-4o
```

The `model` field MAY be overridden at runtime via `--model`:

```bash
anchorgen run fast_apply --model fast-apply-1b --set ...
```

Resolution order: `--model` flag > config `model` field.

### 5.2 API

AnchorGen supports OpenAI-compatible APIs only.

```
POST /v1/chat/completions
```

Provider-specific logic MUST NOT be implemented.
All provider abstraction and routing MUST be handled externally (e.g., LiteLLM).

### 5.3 Environment Variables

```
ANCHORGEN_BASE_URL   # required
ANCHORGEN_API_KEY    # required
```

Missing environment variable → `LLM_CONFIG_MISSING`

No LLM configuration in `anchorgen.yaml` is allowed.

---

## 6. CLI Interface (Normative)

### 6.1 Command

```bash
anchorgen run <generator_name> [options]
```

### 6.2 Options

```
--config <path>       Config file path (default: anchorgen.yaml)
--input <path>        Read content from file instead of stdin
--output <path>       Write result to file instead of stdout
--set key=value       Bind a CLI input
--set key=@path       Bind a CLI input from file contents
--model <string>      Override the model declared in config
```

**Constraints:**

* `--input` and `--output` MUST be specified together or not at all
* Unknown `--set` key → `INPUT_UNKNOWN`
* Missing required `--set` key → `INPUT_MISSING`
* Invalid combination → `CLI_USAGE_ERROR`

---

## 7. Error Model (Normative)

### 7.1 Format

All errors MUST be written to stderr as:

```
ERROR <CODE>: <message>
```

Exit code MUST be non-zero on any error.

### 7.2 Error Codes (Closed Set)

#### Input

* `INPUT_MISSING` — required input not provided
* `INPUT_UNKNOWN` — undeclared `--set` key provided
* `INPUT_SOURCE_INVALID` — declared source is not valid

#### Config

* `CONFIG_INVALID` — missing required field
* `CONFIG_INVALID_FIELD` — unknown field in config

#### Template

* `TEMPLATE_VAR_MISSING` — template references undeclared variable
* `TEMPLATE_VAR_UNUSED` — declared input not referenced in template

#### Extraction

* `EXTRACTION_NO_MATCH` — tag extraction found no match
* `EXTRACTION_MULTIPLE_MATCH` — tag extraction found more than one match

#### Generator

* `GENERATOR_NOT_FOUND` — specified generator name not in config

#### LLM

* `LLM_CONFIG_MISSING` — required environment variable not set
* `LLM_REQUEST_FAILED` — LLM API call failed

#### CLI

* `CLI_USAGE_ERROR` — invalid CLI argument combination

#### I/O

* `IO_ERROR: file not found`
* `IO_ERROR: permission denied`
* `IO_ERROR: invalid UTF-8`
* `IO_ERROR: read failure`
* `IO_ERROR: write failure`

New error codes MUST NOT be added. All failures MUST use this set.

---

## 8. Determinism Guarantees

AnchorGen guarantees that the following are deterministic given identical inputs,
config, and environment variables:

1. Input binding
2. Prompt rendering
3. Extraction
4. Error output

LLM output is treated as **external input** and is explicitly non-deterministic.

---

## 9. Non-Goals

AnchorGen does NOT:

* Manage file state or anchors (AnchorScope's responsibility)
* Validate or normalize UTF-8 / CRLF on output (AnchorScope's responsibility upon re-entry)
* Implement provider-specific LLM logic
* Support multiple API formats beyond OpenAI-compatible
* Support regex extraction (v2)
* Support multi-step or chained generation
* Enforce semantic correctness of LLM output
* Infer implicit context

---

## 10. Example

### fast_apply

Apply a code change to a scoped region via a fast-apply specialized model.

```yaml
generators:
  fast_apply:
    model: fast-apply-7b

    inputs:
      original_code:
        source: stdin
      update_snippet:
        source: cli

    prompt:
      template: |
        <|im_start|>system
        You are a coding assistant that helps merge code updates, ensuring every modification is fully integrated.<|im_end|>

        <|im_start|>user
        Merge all changes from the <update> snippet into the <code> below.
        - Preserve the code's structure, order, comments, and indentation exactly.
        - Output only the updated code, enclosed within <updated-code> and </updated-code> tags.
        - Do not include any additional text, explanations, placeholders, ellipses, or code fences.

        <code>{original_code}</code>

        <update>{update_snippet}</update>

        Provide the complete updated code.<|im_end|>

        <|im_start|>assistant

    extract:
      type: tag
      start: "<updated-code>"
      end: "</updated-code>"
```

Usage with AnchorScope:

```bash
# stdout mode
as.pipe --true-id {id} --out \
  | anchorgen run fast_apply --set update_snippet="add error handling to the loop" \
  | as.pipe --true-id {id} --in

# file-io mode
as.pipe --true-id {id} \
  --tool "anchorgen run fast_apply --set update_snippet='add error handling to the loop'" \
  --file-io
```

Standalone usage:

```bash
cat my_function.py \
  | anchorgen run fast_apply --set update_snippet="add error handling to the loop" \
  > result.py
```

---

## 11. Extensibility (Informative)

AnchorGen's OpenAI-compatible API boundary means any model accessible
via a compatible endpoint can be integrated without modifying this spec.
This includes locally-hosted diffusion language models (e.g., LLaDA, DMax)
served through a compatibility layer such as LiteLLM or a custom FastAPI wrapper.

---

## Summary

AnchorGen is:

> A stateless, declarative prompt execution engine designed to operate
> at the boundary of deterministic editing and non-deterministic generation.

* Deterministic at the boundary: prompt, extraction, and errors
* Non-deterministic within: LLM output
* Stateless: no buffer, no implicit context, no side effects
* Composable: works standalone or inside AnchorScope's pipeline
