//! Turn a Zig AIR dump into Parquet, streaming.
//!
//! ```sh
//! zig build-obj --verbose-air -fno-emit-bin --zig-lib-dir <fork>/lib root.zig 2>&1 \
//!     | entl-zig-air out/
//! ```
//!
//! Writes `out/functions.parquet` and `out/instructions.parquet`, then reports
//! what it did with every line it read. The reporting is the point: a reader
//! that quietly skips syntax it does not know produces a clean-looking table
//! over a corpus it half understood, and the tally is what makes that visible.

#![allow(clippy::print_stdout, clippy::print_stderr)]

mod air;
mod store;

use std::io::{BufRead, Write};
use std::path::PathBuf;

fn main() -> std::process::ExitCode {
    let Some(directory) = std::env::args().nth(1).map(PathBuf::from) else {
        eprintln!("usage: entl-zig-air <output-directory>   (AIR dump on stdin)");
        return std::process::ExitCode::FAILURE;
    };

    let mut store = match store::Store::create(&directory) {
        Ok(store) => store,
        Err(error) => {
            eprintln!("entl-zig-air: {error}");
            return std::process::ExitCode::FAILURE;
        }
    };

    let mut reader = air::Reader::new();
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    let mut line = String::new();
    loop {
        line.clear();
        match input.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(error) => {
                eprintln!("entl-zig-air: reading input: {error}");
                return std::process::ExitCode::FAILURE;
            }
        }
        if let Some(function) = reader.push(&line)
            && let Err(error) = store.push(&function)
        {
            eprintln!("entl-zig-air: {error}");
            return std::process::ExitCode::FAILURE;
        }
    }
    if let Some(function) = reader.finish()
        && let Err(error) = store.push(&function)
    {
        eprintln!("entl-zig-air: {error}");
        return std::process::ExitCode::FAILURE;
    }

    let tally = *reader.tally();
    let functions = store.written_functions;
    let instructions = store.written_instructions;
    if let Err(error) = store.close() {
        eprintln!("entl-zig-air: {error}");
        return std::process::ExitCode::FAILURE;
    }

    let report = format!(
        "functions written    : {functions}\n\
         instructions written : {instructions}\n\
         \n\
         -- every line accounted for --\n\
         lines read           : {lines}\n\
         \u{20}\u{20}headers            : {headers}\n\
         \u{20}\u{20}instructions       : {read}\n\
         \u{20}\u{20}block delimiters   : {delimiters}\n\
         \u{20}\u{20}blank              : {blank}\n\
         \u{20}\u{20}liveness deaths    : {deaths}\n\
         \u{20}\u{20}other output       : {other}\n\
         \u{20}\u{20}UNPARSED           : {unparsed}\n\
         balances             : {balances}\n\
         functions whose count disagreed with their header: {miscounted}\n",
        lines = tally.lines,
        headers = tally.headers,
        read = tally.instructions,
        delimiters = tally.delimiters,
        blank = tally.blank,
        deaths = tally.deaths,
        other = tally.other,
        unparsed = tally.unparsed,
        balances = if tally.balances() { "yes" } else { "NO" },
        miscounted = tally.miscounted,
    );
    if let Err(error) = std::io::stdout().write_all(report.as_bytes()) {
        eprintln!("entl-zig-air: writing the report: {error}");
        return std::process::ExitCode::FAILURE;
    }

    if tally.unparsed > 0 || !tally.balances() {
        // Not a crash, but not a clean run either, and a pipeline should be
        // able to tell.
        return std::process::ExitCode::from(2);
    }
    std::process::ExitCode::SUCCESS
}
