// SPDX-FileCopyrightText: © TPT Solutions
// SPDX-License-Identifier: MIT OR Apache-2.0

//! tpt-mathr benchmark harness (development tool, `publish = false`).
//!
//! Contract (docs/benchmarks.md, todo Phase 0): every recorded benchmark
//! carries **hardware, compiler, flags, operand size, operation, threads,
//! backend, and engine version** — no number is published without that
//! metadata, and no performance claim exists without a recorded run.
//!
//! The harness is dependency-free: it times operations with
//! `std::time::Instant` over enough repetitions to be stable, reports
//! ns/op with an explicit repetition count, and emits one JSON line per
//! measurement suitable for later aggregation. Criterion may replace the
//! timing loop after its dependency audit clears
//! (docs/dependency-audit.md); the metadata contract will not change.
//!
//! Current status: the harness skeleton runs and self-checks its timing
//! loop. Real operand-size sweeps (32 B – 100 MiB) and parallel-scaling
//! runs (1/2/4/8+ threads) land with Phases 2–3.

use std::time::Instant;

/// The metadata contract: every measurement is published with this context.
#[derive(Debug, Clone)]
struct BenchMeta {
    engine_version: String,
    bench_version: String,
    compiler: String,
    target: String,
    opt_level: String,
    /// Hardware identity; `unknown` is honest and allowed — it just means
    /// the run is not comparable across machines.
    cpu_model: String,
    logical_cores: usize,
}

/// One recorded measurement.
#[derive(Debug, Clone)]
struct Measurement {
    suite: &'static str,
    operation: &'static str,
    operand_bits: usize,
    threads: usize,
    backend: &'static str,
    reps: usize,
    total_ns: u128,
}

impl Measurement {
    fn ns_per_op(&self) -> f64 {
        self.total_ns as f64 / self.reps as f64
    }

    fn to_json(&self, meta: &BenchMeta) -> String {
        // Hand-rolled JSON: the dependency budget for the harness is zero.
        let esc = |s: &str| s.replace('\\', "\\\\").replace('"', "'");
        format!(
            concat!(
                "{{\"meta\":{{\"engine_version\":\"{}\",\"bench_version\":\"{}\",",
                "\"compiler\":\"{}\",\"target\":\"{}\",\"opt_level\":\"{}\",",
                "\"cpu_model\":\"{}\",\"logical_cores\":{}}},",
                "\"bench\":{{\"suite\":\"{}\",\"operation\":\"{}\",\"operand_bits\":{},",
                "\"threads\":{},\"backend\":\"{}\",\"reps\":{},\"total_ns\":{},",
                "\"ns_per_op\":{:.2}}}}}"
            ),
            esc(&meta.engine_version),
            esc(&meta.bench_version),
            esc(&meta.compiler),
            esc(&meta.target),
            esc(&meta.opt_level),
            esc(&meta.cpu_model),
            meta.logical_cores,
            self.suite,
            self.operation,
            self.operand_bits,
            self.threads,
            self.backend,
            self.reps,
            self.total_ns,
            self.ns_per_op(),
        )
    }
}

fn cpu_model() -> String {
    // Best-effort, honest "unknown" otherwise. Phase 3 may extend per-OS.
    #[cfg(target_os = "linux")]
    {
        if let Ok(info) = std::fs::read_to_string("/proc/cpuinfo") {
            for line in info.lines() {
                if let Some(rest) = line.strip_prefix("model name") {
                    if let Some(model) = rest.split(':').nth(1) {
                        return model.trim().to_string();
                    }
                }
            }
        }
    }
    std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_else(|_| "unknown".into())
}

fn compiler_version() -> String {
    std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".into())
}

fn logical_cores() -> usize {
    std::thread::available_parallelism().map_or(0, std::num::NonZeroUsize::get)
}

/// Run `op` for `iters` iterations after a warm-up, returning the
/// measurement. Timing granularity is coarse (whole-loop), which is
/// sufficient for arithmetic operations measured in the microseconds and
/// up; micro-benchmarks below ~100 ns/op need the Phase 3 refinement.
fn measure(
    suite: &'static str,
    operation: &'static str,
    operand_bits: usize,
    threads: usize,
    backend: &'static str,
    iters: usize,
    mut op: impl FnMut(),
) -> Measurement {
    // Warm-up (allocator, caches, branch predictors).
    for _ in 0..(iters / 10).max(1) {
        op();
    }
    let start = Instant::now();
    for _ in 0..iters {
        op();
    }
    let total_ns = start.elapsed().as_nanos();
    Measurement {
        suite,
        operation,
        operand_bits,
        threads,
        backend,
        reps: iters,
        total_ns,
    }
}

fn main() {
    let meta = BenchMeta {
        engine_version: tpt_mathr_arith::VERSION.to_string(),
        bench_version: env!("CARGO_PKG_VERSION").to_string(),
        compiler: compiler_version(),
        target: format!("{}/{}", std::env::consts::OS, std::env::consts::ARCH),
        opt_level: if cfg!(debug_assertions) {
            "dev"
        } else {
            "release"
        }
        .to_string(),
        cpu_model: cpu_model(),
        logical_cores: logical_cores(),
    };

    println!("tpt-mathr benchmark harness — Phase 0 skeleton");
    println!("metadata: {meta:?}");

    let mut results = Vec::new();

    // Harness self-check: measure something with known cost-shape so a
    // broken timing loop is visible immediately. This is NOT an engine
    // benchmark and must never be quoted as one.
    results.push(measure(
        "harness",
        "selfcheck_noop",
        0,
        1,
        "scalar",
        1_000,
        || {
            std::hint::black_box(());
        },
    ));

    // Rational suite (Phase 4): reduce / add / mul / div / compare on
    // synthetic operands. Values are built from fixed limb patterns, so
    // runs are reproducible; no relative-performance claims are attached.
    {
        use tpt_mathr_arith::{BigInt, BigRat, Progress};
        let p = Progress::UNLIMITED;
        let mk = |seed: u64, limbs: usize| {
            let mut v = Vec::with_capacity(limbs);
            let mut state = seed | 1;
            for _ in 0..limbs {
                state = state.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(1);
                v.push(state);
            }
            let num = BigInt::from(tpt_mathr_arith::test_support::from_u64_limbs(&v));
            let den = tpt_mathr_arith::test_support::from_u64_limbs(&[seed.rotate_left(17) | 1]);
            BigRat::new(num, den, &p).expect("reduction")
        };

        let small_a = mk(0xA, 1);
        let small_b = mk(0xB, 1);
        let big_a = mk(0xC, 64); // 4096-bit class
        let big_b = mk(0xD, 64);

        results.push(measure(
            "rational",
            "reduce_64bit",
            64,
            1,
            "scalar",
            20_000,
            || {
                std::hint::black_box(
                    BigRat::new(
                        small_a.numerator().clone(),
                        small_a.denominator().clone(),
                        &p,
                    )
                    .expect("reduce"),
                );
            },
        ));
        results.push(measure(
            "rational",
            "add_64bit",
            64,
            1,
            "scalar",
            20_000,
            || {
                std::hint::black_box(small_a.add(&small_b, &p).expect("add"));
            },
        ));
        results.push(measure(
            "rational",
            "mul_64bit",
            64,
            1,
            "scalar",
            20_000,
            || {
                std::hint::black_box(small_a.mul(&small_b, &p).expect("mul"));
            },
        ));
        results.push(measure(
            "rational",
            "div_64bit",
            64,
            1,
            "scalar",
            20_000,
            || {
                std::hint::black_box(small_a.div(&small_b, &p).expect("div"));
            },
        ));
        results.push(measure(
            "rational",
            "cmp_64bit",
            64,
            1,
            "scalar",
            20_000,
            || {
                std::hint::black_box(small_a.cmp_rat(&small_b, &p).expect("cmp"));
            },
        ));
        results.push(measure(
            "rational",
            "add_4096bit",
            4096,
            1,
            "scalar",
            500,
            || {
                std::hint::black_box(big_a.add(&big_b, &p).expect("add"));
            },
        ));
        results.push(measure(
            "rational",
            "mul_4096bit",
            4096,
            1,
            "scalar",
            200,
            || {
                std::hint::black_box(big_a.mul(&big_b, &p).expect("mul"));
            },
        ));
        results.push(measure(
            "rational",
            "div_4096bit",
            4096,
            1,
            "scalar",
            100,
            || {
                std::hint::black_box(big_a.div(&big_b, &p).expect("div"));
            },
        ));
    }

    for m in &results {
        println!("{}", m.to_json(&meta));
    }

    println!(
        "\nNo engine benchmarks yet: operand sweeps (32 B - 100 MiB) and\n\
         parallel-scaling runs land with Phases 2-3 (see docs/benchmarks.md).\n\
         No performance claims are made (Phase 0 milestone policy)."
    );
}
