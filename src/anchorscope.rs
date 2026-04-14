use std::io::Write;

#[derive(Debug)]
pub struct ReadOutput {
    pub content: String,
    pub hash: String,
    pub start_line: usize,
    pub end_line: usize,
}

pub fn run_read(file: &str, anchor: &str) -> Result<ReadOutput, String> {
    let output = std::process::Command::new("anchorscope")
        .arg("read")
        .arg("--file")
        .arg(file)
        .arg("--anchor")
        .arg(anchor)
        .output()
        .map_err(|e| format!("Failed to run anchorscope read: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "anchorscope read failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
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
            start_line = line
                .trim_start_matches("start_line=")
                .parse()
                .map_err(|_| "Invalid start_line value")?;
        } else if line.starts_with("end_line=") {
            end_line = line
                .trim_start_matches("end_line=")
                .parse()
                .map_err(|_| "Invalid end_line value")?;
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
        return Err(format!(
            "anchorscope write failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}
