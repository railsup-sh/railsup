//! AI analysis using Claude Code CLI
//!
//! Auto-detects Claude and streams analysis if available.

use super::report::DiagnosticReport;
use anyhow::Result;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

/// Check if Claude Code CLI is available
pub fn is_claude_available() -> bool {
    which::which("claude").is_ok()
}

/// Stream AI analysis of the diagnostic report
pub fn stream_analysis(report: &DiagnosticReport) -> Result<()> {
    if !is_claude_available() {
        return Ok(()); // Silently skip if not available
    }

    println!();
    println!("{}", "─".repeat(50));
    println!();
    println!("Analyzing with Claude...");
    println!();

    let prompt = build_prompt(report)?;

    let mut child = Command::new("claude")
        .args([
            "--print",
            "--model",
            "haiku",
            "--output-format",
            "stream-json",
            "--verbose",
            &prompt,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    let stdout = child.stdout.take().expect("stdout");
    let reader = BufReader::new(stdout);

    let mut wrapper = WordWrapper::new(76);

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        if line.is_empty() {
            continue;
        }

        // Parse Claude CLI JSON event
        if let Ok(event) = serde_json::from_str::<ClaudeEvent>(&line) {
            if event.event_type == "assistant" {
                // Extract text from message content
                if let Some(ref message) = event.message {
                    for block in &message.content {
                        if block.content_type == "text" {
                            wrapper.write(&block.text);
                        }
                    }
                }
            }
        }
    }

    wrapper.flush();
    println!();

    child.wait()?;
    Ok(())
}

/// Event from Claude CLI stream-json output
#[derive(serde::Deserialize)]
struct ClaudeEvent {
    #[serde(rename = "type")]
    event_type: String,
    message: Option<Message>,
}

#[derive(serde::Deserialize)]
struct Message {
    content: Vec<ContentBlock>,
}

#[derive(serde::Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    #[serde(default)]
    text: String,
}

/// Build the prompt for AI analysis
fn build_prompt(report: &DiagnosticReport) -> Result<String> {
    let json = serde_json::to_string_pretty(report)?;

    Ok(format!(
        r#"You are analyzing a railsup doctor diagnostic report.

Railsup is THE way to install and run Ruby on Rails. It manages Ruby versions
in ~/.railsup/ruby/ and gems in ~/.railsup/gems/. Users may have other version
managers (rbenv, asdf, rvm) installed that can conflict.

Key things to check:
- ruby_status.any_installed: false means no Ruby installed
- ruby_status.default_set: false means no default configured
- shell_integration.placement: "NotFound" or "BeforeVersionManagers" are problems
- conflicts: look for tools with impact "Blocking"
- path_analysis.ruby_correct: false means wrong ruby is being used

Diagnostic Report:
```json
{json}
```

Provide a brief, conversational analysis (2-4 sentences):
1. Is the setup healthy or are there issues?
2. If issues exist, what's the most important one to fix?
3. One specific actionable recommendation

Be direct and friendly. No markdown formatting. Plain text only.
Example tone: "Your setup looks good. rbenv is installed but railsup takes
precedence thanks to correct shell-init placement. No action needed."
"#
    ))
}

/// Word wrapper for streaming output
struct WordWrapper {
    max_width: usize,
    col: usize,
    word_buf: String,
}

impl WordWrapper {
    fn new(max_width: usize) -> Self {
        Self {
            max_width,
            col: 0,
            word_buf: String::new(),
        }
    }

    fn write(&mut self, text: &str) {
        for ch in text.chars() {
            match ch {
                '\n' => {
                    self.flush_word();
                    print!("\n");
                    self.col = 0;
                }
                ' ' => {
                    self.flush_word();
                    if self.col > 0 {
                        print!(" ");
                        self.col += 1;
                    }
                }
                _ => {
                    self.word_buf.push(ch);
                }
            }
        }
    }

    fn flush_word(&mut self) {
        if self.word_buf.is_empty() {
            return;
        }

        let word_len = self.word_buf.len();

        // Wrap if needed
        if self.col > 0 && self.col + word_len > self.max_width {
            print!("\n");
            self.col = 0;
        }

        print!("{}", self.word_buf);
        std::io::stdout().flush().ok();
        self.col += word_len;
        self.word_buf.clear();
    }

    fn flush(&mut self) {
        self.flush_word();
    }
}
