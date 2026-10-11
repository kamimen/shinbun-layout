//! Solves a built-in front page and draws the result as text: `cargo run --example ascii`
use shinbun_layout::*;

fn item(id: &str, role: Role, priority: u8, chars: u32, headline: Option<Headline>) -> Item {
    let mut it = Item::new(id, role).priority(priority).chars(chars);
    it.headline = headline;
    it
}

/// A stand-in for the browser: lines needed from the text length.
fn needs(item: &Item, paper: &Paper, span: u8) -> Option<u8> {
    let per_line = span as u32 * paper.chars as u32;
    let head = match (item.role, item.headline) {
        (Role::Article, Some(Headline::Xl)) => 4,
        (Role::Article, Some(Headline::L)) => 3,
        (Role::Article, Some(Headline::M)) => 2,
        (Role::Article, Some(Headline::S)) => 1,
        _ => 0,
    };
    let lines = match item.role {
        Role::Article | Role::Box => item.chars.div_ceil(per_line) + head,
        _ => 1,
    };
    (lines <= paper.lines as u32).then_some(lines as u8)
}

fn main() {
    let mut nameplate = item("N", Role::Nameplate, 10, 0, None);
    nameplate.fixed = Some(Placement {
        x: 1,
        w: 8,
        y: 1,
        span: 3,
    });
    let mut ad = item("$", Role::Ad, 1, 0, None);
    ad.fixed = Some(Placement {
        x: 1,
        w: 30,
        y: 9,
        span: 2,
    });
    let mut lead = item("1", Role::Article, 10, 700, Some(Headline::L));
    lead.lead = true;
    let problem = Problem {
        paper: Paper::new(Writing::Vertical, 10, 50, 10),
        items: vec![
            nameplate,
            ad,
            lead,
            item("2", Role::Article, 7, 500, Some(Headline::M)),
            item("3", Role::Article, 5, 400, Some(Headline::M)),
            item("4", Role::Article, 3, 250, Some(Headline::S)),
            item("5", Role::Article, 2, 150, Some(Headline::S)),
            item("P", Role::Photo, 6, 0, None),
            item("B", Role::Box, 3, 120, None),
        ],
    };
    let mut table = MeasurementTable::default();
    for q in required_measurements(&problem) {
        table.insert(
            &q.id,
            q.span,
            needs(problem.item(&q.id).unwrap(), &problem.paper, q.span),
        );
    }
    let width = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(16);
    let options = SolveOptions::default()
        .beam_width(width)
        .refine_iterations(std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(0));
    let solution = Refine::new(options).solve(&problem, &table).unwrap();
    draw(&problem, &solution.layout);
    println!("unplaced: {:?}", solution.unplaced);
    println!("score: {:?}", solution.score);
    println!("warnings: {:?}", solution.warnings);
}

/// Columns run top to bottom; lines run right to left, so the right edge is line 1.
fn draw(problem: &Problem, layout: &Layout) {
    let (dan, lines) = (problem.paper.dan as usize, problem.paper.lines as usize);
    let mut grid = vec![vec!['.'; lines]; dan];
    for p in &layout.placements {
        let c = p.id.chars().next().unwrap();
        for y in p.at.y..p.at.y + p.at.span {
            for x in p.at.x..p.at.x + p.at.w {
                grid[y as usize - 1][x as usize - 1] = c;
            }
        }
    }
    for row in &grid {
        println!("{}", row.iter().rev().collect::<String>());
    }
    for p in &layout.placements {
        println!(
            "{:>2}: x={:<2} w={:<2} y={:<2} span={}",
            p.id, p.at.x, p.at.w, p.at.y, p.at.span
        );
    }
}
