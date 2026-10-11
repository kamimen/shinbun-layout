//! Solves a problem file exactly: `exact problem.json table.json`
use shinbun_layout::{MeasurementTable, Problem, SolveOptions};
use shinbun_layout_exact::{solve, total};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [problem, table] = args.as_slice() else {
        eprintln!("usage: exact <problem.json> <table.json>");
        std::process::exit(2);
    };
    let read = |p: &str| std::fs::read_to_string(p).unwrap_or_else(|e| panic!("cannot read {p}: {e}"));
    let problem: Problem = serde_json::from_str(&read(problem)).expect("problem");
    let table: MeasurementTable = serde_json::from_str(&read(table)).expect("table");
    let opts = SolveOptions::default();
    let e = solve(&problem, &table, &opts).expect("solve");
    let placed: Vec<&str> = e.layout.placements.iter().map(|p| p.id.as_str()).collect();
    let unplaced: Vec<&str> = problem
        .items
        .iter()
        .map(|i| i.id.as_str())
        .filter(|i| !placed.contains(i))
        .collect();
    println!(
        "objective {:.2} total {:.2} unplaced {:?}",
        e.objective,
        total(&problem, &table, &opts, &e.layout),
        unplaced
    );
}
