//! Compares the heuristics with the exact optimum on random pages: `cargo run --release -p shinbun-layout-exact --bin gap -- [seeds] [items]`
use shinbun_layout::*;
use shinbun_layout_exact::{separable, solve as exact, total};
use std::time::Instant;

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn range(&mut self, lo: u64, hi: u64) -> u64 {
        lo + self.next() % (hi - lo + 1)
    }
}

fn needs(item: &Item, paper: &Paper, span: u8) -> Option<u8> {
    let per_line = span as u32 * paper.chars as u32;
    let head = match item.headline {
        Some(Headline::Xl) => 4,
        Some(Headline::L) => 3,
        Some(Headline::M) => 2,
        _ => 1,
    };
    let lines = match item.role {
        Role::Article | Role::Box => item.chars.div_ceil(per_line) + head,
        _ => 1,
    };
    (lines <= paper.lines as u32).then_some(lines as u8)
}

fn instance(seed: u64, n: u64) -> Problem {
    let mut r = Lcg(seed);
    let paper = Paper {
        writing: Writing::Vertical,
        dan: 8,
        lines: 36,
        chars: 10,
    };
    let mk = |id: String, role, priority, chars, headline| Item {
        id,
        role,
        priority,
        chars,
        headline,
        lead: false,
        min_span: None,
        max_span: None,
        fixed: None,
    };
    let mut nameplate = mk("N".into(), Role::Nameplate, 10, 0, None);
    nameplate.fixed = Some(Placement {
        x: 1,
        w: 6,
        y: 1,
        span: 3,
    });
    let mut items = vec![nameplate];
    for k in 0..n {
        let role = match r.range(0, 9) {
            0 => Role::Photo,
            1 => Role::Box,
            _ => Role::Article,
        };
        let headline = [Headline::S, Headline::M, Headline::L][r.range(0, 2) as usize];
        let chars = if role == Role::Photo {
            0
        } else {
            r.range(80, 700) as u32
        };
        let mut it = mk(format!("a{k}"), role, r.range(1, 10) as u8, chars, Some(headline));
        it.lead = k == 0 && role == Role::Article;
        items.push(it);
    }
    Problem { paper, items }
}

fn main() {
    let seeds: u64 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(5);
    let n: u64 = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(7);
    println!("seed  separable: exact beam(gap%) refine(gap%) | real total: exact beam refine | t_exact t_beam t_refine");
    for seed in 1..=seeds {
        let p = instance(seed, n);
        let mut t = MeasurementTable::default();
        for q in required_measurements(&p) {
            t.insert(&q.id, q.span, needs(p.item(&q.id).unwrap(), &p.paper, q.span));
        }
        let opts = SolveOptions::default();
        let t0 = Instant::now();
        let e = match exact(&p, &t, &opts) {
            Ok(e) => e,
            Err(err) => {
                println!("{seed:>4}  exact failed: {err}");
                continue;
            }
        };
        let te = t0.elapsed();
        let t0 = Instant::now();
        let b = Beam::new(opts.clone()).solve(&p, &t).unwrap();
        let tb = t0.elapsed();
        let t0 = Instant::now();
        let rf = Refine::new(SolveOptions {
            refine_iterations: 1500,
            ..opts.clone()
        })
        .solve(&p, &t)
        .unwrap();
        let tr = t0.elapsed();
        let (bs, rs) = (
            separable(&p, &t, &opts, &b.layout),
            separable(&p, &t, &opts, &rf.layout),
        );
        let gap = |x: f64| 100.0 * (e.objective - x) / e.objective.abs().max(1e-9);
        println!(
            "{seed:>4}  {:>6.2} {:>6.2} ({:>5.2}) {:>6.2} ({:>5.2}) | {:>6.2} {:>6.2} {:>6.2} | {:?} {:?} {:?}",
            e.objective,
            bs,
            gap(bs),
            rs,
            gap(rs),
            total(&p, &t, &opts, &e.layout),
            total(&p, &t, &opts, &b.layout),
            total(&p, &t, &opts, &rf.layout),
            te,
            tb,
            tr
        );
    }
}
