//! JSON in, JSON out, for hosts written in other languages.
//!
//!   cargo run --release --example solve -- measurements problem.json
//!   cargo run --release --example solve -- solve problem.json table.json [options.json]
use std::{env, fs, process};

fn read(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("cannot read {path}: {e}");
        process::exit(2);
    })
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let result = match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["measurements", problem] => shinbun_layout::required_measurements_json(&read(problem)),
        ["solve", problem, table] => shinbun_layout::solve_json(&read(problem), &read(table), ""),
        ["solve", problem, table, options] => {
            shinbun_layout::solve_json(&read(problem), &read(table), &read(options))
        }
        _ => {
            eprintln!("usage: solve measurements <problem.json>\n       solve solve <problem.json> <table.json> [options.json]");
            process::exit(2);
        }
    };
    match result {
        Ok(json) => println!("{json}"),
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    }
}
