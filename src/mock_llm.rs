pub fn mock_generate(prompt: &str) -> Result<String, String> {
    let expected = std::env::var("ANCHORGEN_EXPECTED_PROMPT")
        .map_err(|_| "MOCK_NOT_FOUND: ANCHORGEN_EXPECTED_PROMPT not set".to_string())?;

    if prompt != expected {
        return Err(format!(
            "MOCK_NOT_FOUND: prompt mismatch.\nExpected:\n{}\n\nGot:\n{}",
            expected,
            prompt
        ));
    }

    std::env::var("ANCHORGEN_MOCK_RESPONSE")
        .map(|s| s)
        .map_err(|_| "MOCK_NOT_FOUND: ANCHORGEN_MOCK_RESPONSE not set".to_string())
}
