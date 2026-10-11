use std::{env, fs, process};

fn read(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("cannot read {path}: {e}");
        process::exit(2);
    })
}

fn validate(problem: &str, table: &str, layout: &str) -> Result<String, String> {
    let problem: shinbun_layout::Problem =
        serde_json::from_str(problem).map_err(|e| format!("invalid problem: {e}"))?;
    let table: shinbun_layout::MeasurementTable =
        serde_json::from_str(table).map_err(|e| format!("invalid measurements: {e}"))?;
    let layout: shinbun_layout::Layout =
        serde_json::from_str(layout).map_err(|e| format!("invalid layout: {e}"))?;
    let found = shinbun_layout::validate(&problem, &table, &layout, &shinbun_layout::Rules::default());
    serde_json::to_string(&found).map_err(|e| e.to_string())
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let result = match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["measurements", problem] => {
            shinbun_layout::json::required_measurements(&read(problem)).map_err(|e| e.to_string())
        }
        ["solve", problem, table] => {
            shinbun_layout::json::solve(&read(problem), &read(table), "").map_err(|e| e.to_string())
        }
        ["solve", problem, table, options] => {
            shinbun_layout::json::solve(&read(problem), &read(table), &read(options))
                .map_err(|e| e.to_string())
        }
        ["validate", problem, table, layout] => validate(&read(problem), &read(table), &read(layout)),
        _ => {
            eprintln!(
                "usage: shinbun-layout measurements <problem.json>\n       shinbun-layout solve <problem.json> <table.json> [options.json]\n       shinbun-layout validate <problem.json> <table.json> <layout.json>"
            );
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
