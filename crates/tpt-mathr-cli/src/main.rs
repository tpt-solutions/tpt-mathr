// SPDX-FileCopyrightText: © TPT Solutions
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `tpt-mathr-cli` — command-line access to the tpt-mathr engine.
//!
//! Planned subcommands (Phase 11): `calc`, `bigint`, `rational`, `float`,
//! `poly`, `prime`, `search`, `verify`, `benchmark`. The binary always runs
//! hosted (`std`); it is a thin, dependency-free wrapper over the engine
//! crates.

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.len() {
        1 => {
            eprintln!("{} {} — no subcommand given", name(), version());
            usage();
            std::process::exit(2);
        }
        _ => {
            eprintln!("{} {}: subcommands arrive with Phase 11", name(), version());
            eprintln!("requested: {:?}", &args[1..]);
            usage();
            std::process::exit(2);
        }
    }
}

fn name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

fn usage() {
    eprintln!(
        "\nUSAGE:\n    {name} <SUBCOMMAND>\n\nSUBCOMMANDS (planned, Phase 11):\n    calc, bigint, rational, float, poly, prime, search, verify, benchmark\n",
        name = name()
    );
}
