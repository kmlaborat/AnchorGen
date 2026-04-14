# AnchorGen Specification Compliance Fixes Implementation Plan

**Goal:** Fix critical specification discrepancies in the AnchorGen codebase to achieve full compliance with docs/SPEC.md

**Architecture:** The implementation fixes two critical bugs (tag extraction and determinism) and adds missing validation through targeted changes to the Rust source files.

**Tech Stack:** Rust, Cargo, serde_yaml, serde

---

## Task 1: Fix Tag Extraction - Non-Greedy Matching

**Files:**
- Modify: `src/extract.rs:27-49`

**Step 1: Implement the fix**

The current implementation has a bug where it searches for the closing tag from the beginning of the output instead of after the opening tag.

**Current code (lines 34-36):**
```rust
let content_end = output
    .find(end)
    .ok_or("No closing tag found".to_string())?;
```

**Fix (lines 34-37):**
```rust
let content_end = output[content_start..]
    .find(end)
    .ok_or("No closing tag found".to_string())? + content_start;
```

**Step 2: Run tests**

```bash
cargo test
```

**Expected:** All existing tests pass.

---

## Task 2: Fix Determinism - Replace HashMap with BTreeMap

**Files:**
- Modify: `src/binding.rs:1`
- Modify: `src/template.rs:1`
- Modify: `src/template.rs:7`
- Modify: `src/template.rs:10`

**Step 1: Update binding.rs**

**Current (lines 1, 5):**
```rust
use std::collections::HashMap;

pub type BoundInputs = HashMap<String, String>;

pub fn bind_inputs(
    generator: &crate::config::GeneratorSpec,
    read_content: &str,
    cli_inputs: &HashMap<String, String>,
) -> Result<BoundInputs, String> {
```

**Fix:**
```rust
use std::collections::BTreeMap;

pub type BoundInputs = BTreeMap<String, String>;

pub fn bind_inputs(
    generator: &crate::config::GeneratorSpec,
    read_content: &str,
    cli_inputs: &BTreeMap<String, String>,
) -> Result<BoundInputs, String> {
```

**Step 2: Update template.rs**

**Current (lines 1, 7, 10):**
```rust
use std::collections::{HashMap, HashSet};

pub fn render_template(
    template: &str,
    inputs: &HashMap<String, String>,
) -> Result<String, String> {
    let mut result = template.to_string();
    let mut used_inputs = HashSet::new();
```

**Fix:**
```rust
use std::collections::{BTreeMap, BTreeSet};

pub fn render_template(
    template: &str,
    inputs: &BTreeMap<String, String>,
) -> Result<String, String> {
    let mut result = template.to_string();
    let mut used_inputs = BTreeSet::new();
```

**Step 3: Run tests**

```bash
cargo test
```

**Expected:** All tests pass.

---

## Task 3: Add Validation for Stdin Inputs

**Files:**
- Modify: `src/binding.rs:1-35`

**Step 1: Add validation in bind_inputs**

**Current code:**
```rust
pub fn bind_inputs(
    generator: &crate::config::GeneratorSpec,
    read_content: &str,
    cli_inputs: &BTreeMap<String, String>,
) -> Result<BoundInputs, String> {
    let mut bound = BTreeMap::new();

    for (var, spec) in &generator.inputs {
```

**Fix:**
```rust
pub fn bind_inputs(
    generator: &crate::config::GeneratorSpec,
    read_content: &str,
    cli_inputs: &BTreeMap<String, String>,
) -> Result<BoundInputs, String> {
    let mut bound = BTreeMap::new();
    let mut stdin_count = 0;

    // Validate stdin inputs per SPEC 4.4
    for (_var, spec) in &generator.inputs {
        if spec.source == "stdin" {
            stdin_count += 1;
        }
    }

    if stdin_count > 1 {
        return Err("Exactly one input can have source: stdin".to_string());
    }

    for (var, spec) in &generator.inputs {
```

**Step 2: Run tests**

```bash
cargo test
```

**Expected:** All tests pass.

---

## Task 4: Add Comprehensive Tests for Tag Extraction

**Files:**
- Create: `tests/tag_extraction_tests.rs`

**Step 1: Create test file with all test cases**

**Step 2: Run tests**

```bash
cargo test
```

**Expected:** All tests pass.

---

## Task 5: Update Documentation

**Files:**
- Modify: `docs/discrepancy-analysis.md`
- Modify: `docs/detailed-analysis.md`
- Modify: `docs/implementation-status.md`

**Step 1: Update all documentation to reflect fixes**

**Step 2: Verify all tests pass**

```bash
cargo test
```

---

## Task 6: Final Verification

**Files:**
- N/A

**Step 1: Run all tests**

```bash
cargo test --all
```

**Step 2: Check code compiles without warnings**

```bash
cargo clippy --all-features --all-targets -- -D warnings
```

**Step 3: Verify specification compliance**

Check against the specification to ensure all requirements are met.

---

## Testing Strategy

1. **Unit Tests**: Test each function in isolation
2. **Integration Tests**: Test the full pipeline
3. **Edge Cases**: Test empty strings, special characters, etc.
4. **Error Handling**: Test all error paths

## Commit Messages

```
fix: add non-greedy tag extraction
test: add comprehensive tag extraction tests
feat: add stdin input validation
docs: update specification compliance documentation
```
