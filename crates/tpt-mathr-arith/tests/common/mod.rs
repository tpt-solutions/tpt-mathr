// SPDX-FileCopyrightText: © TPT Solutions
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Differential-test harness helpers shared by the integration test files
//! in this directory. Included via `mod common;` (or `#[path]` from
//! sibling files). Dev-only: never part of any published crate.

use std::io::Write;
use std::process::{Command, Stdio};

/// A Python 3 interpreter that can evaluate big-integer expressions.
pub struct Python {
    bin: &'static str,
}

impl Python {
    /// Find a usable interpreter, or `None` (tests then skip with a note).
    ///
    /// Dev-only dependency: Python is a *reference implementation* for
    /// differential testing (spec §45: reference implementation ≠ runtime
    /// dependency).
    pub fn discover() -> Option<Self> {
        for bin in ["python3", "python", "py"] {
            let ok = Command::new(bin)
                .args(["-c", "import sys; assert sys.version_info >= (3, 8)"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok_and(|s| s.success());
            if ok {
                return Some(Self {
                    // SAFETY-free leak: small fixed set of names, process lifetime.
                    bin: match bin {
                        "python3" => "python3",
                        "python" => "python",
                        _ => "py",
                    },
                });
            }
        }
        None
    }

    /// Evaluate an integer expression and return its decimal rendering.
    ///
    /// The expression is evaluated with only the stdlib available; huge
    /// integer printing is enabled explicitly (CPython 3.11+ limits
    /// int→str conversion by default).
    pub fn eval_bigint(&self, expr: &str) -> Result<String, String> {
        let program = format!(
            "import sys\nif hasattr(sys, 'set_int_max_str_digits'):\n    sys.set_int_max_str_digits(0)\nprint({expr})"
        );
        let child = Command::new(self.bin)
            .arg("-c")
            .arg(&program)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("spawn {0}: {e}", self.bin))?;
        let out = child.wait_with_output().map_err(|e| format!("wait: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "python failed ({}): {}",
                out.status,
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        let stdout = String::from_utf8_lossy(&out.stdout);
        Ok(stdout.trim().to_string())
    }

    /// Batch-evaluate with a custom preamble (extra definitions prepended
    /// to the interpreter script, e.g. pure-Python reference helpers).
    pub fn eval_with_preamble(
        &self,
        preamble: &str,
        exprs: &[String],
    ) -> Result<Vec<String>, String> {
        let script = format!(
            "import sys, math
from fractions import Fraction
{preamble}
if hasattr(sys, 'set_int_max_str_digits'):
    sys.set_int_max_str_digits(0)
for line in sys.stdin:
    line = line.rstrip('\\n')
    if line:
        print(eval(line))
"
        );
        let mut child = Command::new(self.bin)
            .arg("-c")
            .arg(&script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("spawn {0}: {e}", self.bin))?;
        let stdin = child.stdin.take().expect("stdin piped");
        let corpus = exprs.to_vec();
        let writer = std::thread::spawn(move || {
            let mut stdin = stdin;
            for e in &corpus {
                if writeln!(stdin, "{e}").is_err() {
                    break;
                }
            }
        });
        let out = child.wait_with_output().map_err(|e| format!("wait: {e}"))?;
        let _ = writer.join();
        if !out.status.success() {
            return Err(format!(
                "python failed ({}): {}",
                out.status,
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        let stdout = String::from_utf8_lossy(&out.stdout);
        Ok(stdout
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(str::to_string)
            .collect())
    }

    /// Feed a batch of expressions through one interpreter process and
    /// collect the printed lines, in order. Far cheaper than one process
    /// per case for large corpora. Expressions must contain no newlines.
    ///
    /// The stdin feed runs on a separate thread: writing the whole corpus
    /// before reading stdout deadlocks once the child's output pipe buffer
    /// fills (the child blocks writing while we block writing).
    pub fn eval_bigint_batch(&self, exprs: &[String]) -> Result<Vec<String>, String> {
        let script = String::from(
            "import sys, math\nfrom fractions import Fraction\nif hasattr(sys, 'set_int_max_str_digits'):\n    sys.set_int_max_str_digits(0)\nfor line in sys.stdin:\n    line = line.rstrip('\\n')\n    if line:\n        print(eval(line))\n",
        );

        let mut child = Command::new(self.bin)
            .arg("-c")
            .arg(&script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("spawn {0}: {e}", self.bin))?;
        let stdin = child.stdin.take().expect("stdin piped");
        let corpus = exprs.to_vec();
        let writer = std::thread::spawn(move || {
            let mut stdin = stdin;
            for e in &corpus {
                // A write error means the child died; the read side reports it.
                if writeln!(stdin, "{e}").is_err() {
                    break;
                }
            }
        });
        let out = child.wait_with_output().map_err(|e| format!("wait: {e}"))?;
        let _ = writer.join();
        if !out.status.success() {
            return Err(format!(
                "python failed ({}): {}",
                out.status,
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        let stdout = String::from_utf8_lossy(&out.stdout);
        Ok(stdout
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(str::to_string)
            .collect())
    }
}

/// Skip note emitted when the reference interpreter is unavailable —
/// visible in CI logs, silent for developers without Python.
pub fn skip_note(test: &str) {
    println!("cargo:warning=skipping {test}: no Python 3 interpreter found");
}
