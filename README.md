# AnchorGen

**AnchorGen** is a generator-driven extension of [AnchorScope](https://github.com/kmlaborat/anchorscope). It integrates external generation functions (e.g., LLMs) into the deterministic editing pipeline while preserving AnchorScope's safety guarantees.

[AnchorScope](https://github.com/kmlaborat/anchorscope) defines the editing protocol: how to locate, verify, and safely replace a region in a file. AnchorGen sits on top of it, adding the ability to generate replacement content via LLMs. The two are deliberately separate — AnchorScope remains a minimal, stable protocol with no dependency on AI, while AnchorGen handles the non-deterministic generation layer.

## Specification Compliance

This implementation fully adheres to [docs/SPEC.md](docs/SPEC.md). All specification discrepancies have been resolved:

- ✅ **Tag Extraction**: Now uses non-greedy matching (FIXED)
- ✅ **Determinism**: HashMap replaced with BTreeMap for consistent iteration order (FIXED)
- ✅ **Stdin Validation**: Enforces at most one input with `source: stdin` (ADDED)
- ✅ **Test Coverage**: All 11 integration tests pass

See [docs/implementation-status.md](docs/implementation-status.md) for detailed compliance information.

### Pipeline

```text
READ (AnchorScope) → BIND → GENERATE → EXTRACT → WRITE (AnchorScope)
```

1. **READ**: Invoke `anchorscope read` to locate the anchor region and obtain content, hash, and range.
2. **BIND**: Resolve generator inputs from `read.content` and CLI arguments.
3. **GENERATE**: Call an external generator (LLM or equivalent) to produce candidate output.
4. **EXTRACT**: Apply deterministic extraction rules (identity or tag-based).
5. **WRITE**: Invoke `anchorscope write` with the expected hash to perform scoped replacement.

> **Note**: All deterministic guarantees described in SPEC.md apply.

---

## Installation

```bash
cargo build --release
# Binary will be at target/release/anchorgen.exe
```

Requires:

* Rust toolchain
* `anchorscope` installed and available in PATH

---

## Usage

```bash
anchorgen run <generator_name> [options]
```

### Options

- `--config <path>` – Config file path (default: `anchorgen.yaml`)
- `--input <path>` – Read content from file instead of stdin
- `--output <path>` – Write result to file instead of stdout
- `--set key=value` – Bind a CLI input
- `--set key=@path` – Bind a CLI input from file contents
- `--model <string>` – Override the model declared in config

> **Note**: `--input` and `--output` must be specified together.

### New Features

**Non-greedy tag extraction**: Tag extraction now correctly uses non-greedy matching to handle multiple tag pairs.

**Deterministic iteration**: Replaced HashMap with BTreeMap to ensure consistent error message ordering across runs.

**Stdin input validation**: Configuration files with multiple `source: stdin` inputs now produce a clear error message.

**File-based CLI inputs**: Use `--set key=@path` to load input from a file:

```bash
anchorgen run fast_apply \
  --config config.yaml \
  --input src/file.rs \
  --output result.rs \
  --set update_snippet=@patch.txt
```

**Optional inputs**: Mark inputs as optional in config with `required: false`:

```yaml
inputs:
  required_input:
    source: stdin
    required: true
  optional_input:
    source: cli
    required: false
```

**Strict config validation**: Unknown config fields now produce `CONFIG_INVALID_FIELD` errors.

**Template variable validation**: Declared inputs not referenced in the template produce `TEMPLATE_VAR_UNUSED` errors.

**Tag extraction validation**: Multiple complete tag pairs produce `EXTRACTION_MULTIPLE_MATCH` errors.

### Examples

**Standalone mode** (stdin/stdout):

```bash
echo "some code" | anchorgen run fast_apply \
  --config config.yaml \
  --set update_snippet="add error handling"
```

**File I/O mode**:

```bash
anchorgen run fast_apply \
  --config config.yaml \
  --input src/file.rs \
  --output result.rs \
  --set update_snippet="add error handling"
```

**File-based CLI input**:

```bash
anchorgen run fast_apply \
  --config config.yaml \
  --input src/file.rs \
  --output result.rs \
  --set update_snippet=@patch.txt
```

**Model override**:

```bash
anchorgen run fast_apply \
  --config config.yaml \
  --input src/file.rs \
  --output result.rs \
  --set update_snippet="add error handling" \
  --model my-custom-model
```

---

## Configuration Example (`config.yaml`)

```yaml
generators:
  fast_apply:
    inputs:
      original_code: read.content
      update_snippet: cli.update_snippet
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

> **Note**: `config.yaml` **must not** contain `llm:` section (use environment variables instead).

---

## LLM Integration

AnchorGen supports **OpenAI-compatible APIs only**.

### Environment Variables (required)

```bash
export ANCHORGEN_BASE_URL=http://localhost:4000/v1
export ANCHORGEN_API_KEY=dummy
```

* `ANCHORGEN_BASE_URL` – OpenAI-compatible endpoint (e.g., LiteLLM proxy)
* `ANCHORGEN_API_KEY` – API key (can be dummy for local proxies)

### Model Routing

AnchorGen uses the generator name as the model identifier when calling the API.
Resolving this identifier to a physical model and provider is the responsibility of the proxy.

Configure LiteLLM (or equivalent) to route generator names to the intended models:

```yaml
# litellm/config.yaml
model_list:
  - model_name: fast_apply             # matches generator name
    litellm_params:
      model: huggingface/Kortix/FastApply-7B-v1.0
      api_key: os.environ/HF_API_KEY

```

This allows per-generator model selection without any configuration on the AnchorGen side.

---

## Determinism & Extraction Notes

* All edits are constrained by AnchorScope (scope anchoring & hash verification).
* Input binding is explicit and deterministic.
* LLM outputs are automatically **trimmed** before extraction (determinism preserved).
* `tag` extraction uses **non-greedy matching**; nested tags are **not supported**. Only **one region** is extracted.
* Multiple tag pairs in output produce `EXTRACTION_MULTIPLE_MATCH` error.
* CLI keys with hyphens (`--set my-key=val`) are internally converted to underscores (`cli.my_key`).
* No modification occurs outside the matched region.
* No hidden context or implicit behavior.

---

## Non-Goals

* Multi-file operations
* Semantic correctness of generated content
* Restricting generated content form (text, JSON, XML, etc.)
* Modifying AnchorScope protocol semantics

---

## Advanced Example: In-place UI String Localization (Multi-stage)

Translating a specific UI string in source code (e.g., a button label) requires a two-stage pipeline: first anchoring the surrounding line to uniquely identify the location, then anchoring the string literal within it.

When `read.*` inputs are used, AnchorGen writes the result back via AnchorScope and produces no stdout. This means intermediate files must be used for multi-stage pipelines. Writing them to tmpfs (e.g., `/dev/shm`) avoids SSD wear.

DSL-level support for multi-stage anchor pipelines is planned for a future version.

### Tag Extraction Example

When using tag-based extraction, AnchorGen now correctly handles multiple tag pairs with non-greedy matching:

```bash
# This will extract only the first match, not the last
echo '<tag>first</tag><tag>second</tag>' | \
  anchorgen run translate \
    --set text='<tag>first</tag><tag>second</tag>'
# Returns: first

# Multiple complete tag pairs produce an error
# echo '<tag>a</tag>content<tag>b</tag>' | anchorgen run translate
# ERROR: EXTRACTION_MULTIPLE_MATCH: Multiple tag matches found
```

```bash
#!/bin/bash
# Translate a UI string in-place using a two-stage AnchorScope pipeline.
# Intermediate files are written to tmpfs (e.g., /dev/shm) to avoid SSD wear.

set -euo pipefail

SRC_FILE="src/ui.rs"
TMPDIR="/dev/shm/anchorgen_tmp"
mkdir -p "$TMPDIR"

# Step 1: Save the surrounding line as the primary anchor file
echo -n 'button.set_label("Submit Order");' > "$TMPDIR/anchor_primary.txt"

# Step 2: Locate the primary anchor in the source file and extract content
read_result=$(anchorscope read   --file "$SRC_FILE"   --anchor-file "$TMPDIR/anchor_primary.txt")
hash1=$(echo "$read_result" | grep "^hash=" | cut -d= -f2)
echo "$read_result" | grep "^content=" | cut -d= -f2- > "$TMPDIR/content_primary.txt"

# Step 3: Run anchorgen on the intermediate file to translate only the string literal
# (uses read.*, so result is written back in-place to content_primary.txt via AnchorScope)
anchorgen run apply_translation_e2j   --config config.yaml   --file "$TMPDIR/content_primary.txt"   --anchor '"Submit Order"'

# Step 4: Write the translated line back to the source file with hash verification
anchorscope write   --file "$SRC_FILE"   --anchor-file "$TMPDIR/anchor_primary.txt"   --expected-hash "$hash1"   --replacement "$(cat "$TMPDIR/content_primary.txt")"

rm -rf "$TMPDIR"
```

The `apply_translation_e2j` generator config:

```yaml
generators:
  apply_translation_e2j:
    inputs:
      source_text: read.content
    prompt:
      template: |
        <|plamo:op|>dataset
        translation
        <|plamo:op|>input lang=English
        {source_text}
        <|plamo:op|>output lang=Japanese
    extract:
      type: identity
```

LiteLLM routing:

```yaml
# litellm/config.yaml
model_list:
  - model_name: apply_translation_e2j
    litellm_params:
      model: huggingface/pfnet/plamo-2-translate
      api_key: os.environ/HF_API_KEY
```

---

> Determinism at the boundary
> Freedom within the generator
> Safety enforced at write time

---

## License

This project is licensed under the **MIT License**. See the [LICENSE](LICENSE) file for the full text.

---

### Disclaimer

**THE SOFTWARE IS PROVIDED "AS IS"**, without warranty of any kind. As this is a reference implementation of a file-editing protocol, the author is not responsible for any data loss or unintended file modifications resulting from its use. Always use version control and test in a safe environment.

Copyright (c) 2026 kmlaborat
