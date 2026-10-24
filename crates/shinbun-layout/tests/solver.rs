use proptest::prelude::*;
use shinbun_layout::*;

fn needs(item: &Item, paper: &Paper, span: u8) -> Option<u8> {
    let per_line = span as u32 * paper.chars as u32;
    let headline_lines = match (item.role, item.headline) {
        (Role::Article, Some(Headline::Xl)) => 4,
        (Role::Article, Some(Headline::L)) => 3,
        (Role::Article, Some(Headline::M)) => 2,
        (Role::Article, Some(Headline::S)) => 1,
        _ => 0,
    };
    let lines = match item.role {
        Role::Article | Role::Box => item.chars.div_ceil(per_line) + headline_lines,
        _ => 1,
    };
    (lines <= paper.lines as u32).then_some(lines as u8)
}

fn table_for(problem: &Problem) -> MeasurementTable {
    let mut t = MeasurementTable::default();
    for q in required_measurements(problem) {
        let item = problem.item(&q.id).unwrap();
        t.insert(&q.id, q.span, needs(item, &problem.paper, q.span));
    }
    t
}

fn item(id: &str, role: Role, priority: u8, chars: u32, headline: Option<Headline>) -> Item {
    let mut it = Item::new(id, role).priority(priority).chars(chars);
    it.headline = headline;
    it
}

fn at(x: u8, w: u8, y: u8, span: u8) -> Placement {
    Placement { x, w, y, span }
}

fn paper() -> Paper {
    Paper::new(Writing::Vertical, 10, 50, 10)
}

fn front_page() -> Problem {
    let mut nameplate = item("nameplate", Role::Nameplate, 10, 0, None);
    nameplate.fixed = Some(at(1, 8, 1, 3));
    let mut ad = item("ad", Role::Ad, 1, 0, None);
    ad.fixed = Some(at(1, 30, 9, 2));
    let mut lead = item("lead", Role::Article, 10, 700, Some(Headline::L));
    lead.lead = true;
    Problem {
        paper: paper(),
        items: vec![
            nameplate,
            ad,
            lead,
            item("second", Role::Article, 7, 500, Some(Headline::M)),
            item("third", Role::Article, 5, 400, Some(Headline::M)),
            item("fourth", Role::Article, 3, 250, Some(Headline::S)),
            item("fifth", Role::Article, 2, 150, Some(Headline::S)),
            item("photo", Role::Photo, 6, 0, None),
            item("box", Role::Box, 3, 120, None),
        ],
    }
}

fn hard(v: &[Violation]) -> Vec<&Violation> {
    v.iter().filter(|x| x.is_hard()).collect()
}

fn area(at: &Placement) -> u32 {
    at.w as u32 * at.span as u32
}

#[test]
fn a_front_page_is_placed_completely_and_validly() {
    let p = front_page();
    let t = table_for(&p);
    let s = solve(&p, &t, &SolveOptions::default()).unwrap();

    assert!(s.unplaced.is_empty(), "{:?}", s.unplaced);
    assert_eq!(s.layout.placements.len(), p.items.len());
    assert!(hard(&s.warnings).is_empty(), "{:?}", s.warnings);
    assert!(hard(&validate(&p, &t, &s.layout, &Rules::default())).is_empty());
    assert_eq!(
        s.layout.get("nameplate"),
        Some(&at(1, 8, 1, 3)),
        "fixed items stay"
    );
    assert_eq!(s.layout.get("ad"), Some(&at(1, 30, 9, 2)));
}

#[test]
fn the_page_is_mostly_filled() {
    let p = front_page();
    let s = solve(&p, &table_for(&p), &SolveOptions::default()).unwrap();
    let cells: u32 = s.layout.placements.iter().map(|x| area(&x.at)).sum();
    assert!(cells * 100 >= 10 * 50 * 70, "only {cells} of 500 cells are used");
}

#[test]
fn the_lead_story_is_the_biggest_story() {
    let p = front_page();
    let s = solve(&p, &table_for(&p), &SolveOptions::default()).unwrap();
    let lead = area(s.layout.get("lead").unwrap());
    for other in ["second", "third", "fourth", "fifth"] {
        assert!(
            lead >= area(s.layout.get(other).unwrap()),
            "lead {lead} < {other}"
        );
    }
}

#[test]
fn the_lead_story_sits_closer_to_the_start_corner_than_minor_ones() {
    let p = front_page();
    let s = solve(&p, &table_for(&p), &SolveOptions::default()).unwrap();
    let distance = |id: &str| {
        let a = s.layout.get(id).unwrap();
        (a.x as f64 - 1.0) / 50.0 + (a.y as f64 - 1.0) / 10.0
    };
    assert!(
        distance("lead") <= distance("fifth"),
        "lead {} vs fifth {}",
        distance("lead"),
        distance("fifth")
    );
}

#[test]
fn solving_is_deterministic() {
    let p = front_page();
    let t = table_for(&p);
    let a = solve(&p, &t, &SolveOptions::default()).unwrap();
    let b = solve(&p, &t, &SolveOptions::default()).unwrap();
    assert_eq!(a, b);
}

#[test]
fn the_beam_is_at_least_as_good_as_the_greedy_method() {
    let p = front_page();
    let t = table_for(&p);
    let greedy = Constructive::default().solve(&p, &t).unwrap();
    let beam = Beam::default().solve(&p, &t).unwrap();
    assert!(
        beam.score.total >= greedy.score.total - 1e-9,
        "beam {} < greedy {}",
        beam.score.total,
        greedy.score.total
    );
}

#[test]
fn refining_is_never_worse_than_the_beam_and_is_deterministic() {
    let p = front_page();
    let t = table_for(&p);
    let beam = Beam::default().solve(&p, &t).unwrap();
    let opts = SolveOptions::default().refine_iterations(200);
    let a = Refine::new(opts.clone()).solve(&p, &t).unwrap();
    let b = Refine::new(opts).solve(&p, &t).unwrap();
    assert!(a.score.total >= beam.score.total - 1e-9);
    assert_eq!(a, b);
    assert!(validate(&p, &t, &a.layout, &Rules::default())
        .iter()
        .all(|v| !v.is_hard()));
}

#[test]
fn alternatives_are_distinct_layouts() {
    let p = front_page();
    let s = solve(&p, &table_for(&p), &SolveOptions::default()).unwrap();
    assert!(!s.alternatives.is_empty());
    let mut all = vec![s.layout.clone()];
    all.extend(s.alternatives.iter().map(|a| a.layout.clone()));
    for (i, a) in all.iter().enumerate() {
        for b in &all[i + 1..] {
            assert_ne!(a, b);
        }
    }
    assert!(s
        .alternatives
        .iter()
        .all(|a| a.score.total <= s.score.total + 1e-9));
}

#[test]
fn an_item_that_cannot_fit_anywhere_is_reported_not_forced() {
    let mut p = front_page();
    p.items
        .push(item("monster", Role::Article, 4, 100_000, Some(Headline::M)));
    let t = table_for(&p);
    let s = solve(&p, &t, &SolveOptions::default()).unwrap();
    assert_eq!(s.unplaced.len(), 1);
    assert_eq!(s.unplaced[0].id, "monster");
    assert_eq!(s.unplaced[0].reason, Reason::NoShape);
    assert!(hard(&s.warnings).is_empty());
}

#[test]
fn when_the_page_is_full_the_least_important_item_is_dropped() {
    let mut items = vec![
        item("a", Role::Article, 9, 1500, Some(Headline::L)),
        item("b", Role::Article, 8, 1500, Some(Headline::L)),
        item("c", Role::Article, 1, 1500, Some(Headline::L)),
    ];
    for it in &mut items {
        it.min_span = Some(5);
        it.max_span = Some(5);
    }
    let p = Problem {
        paper: Paper::new(Writing::Vertical, 10, 40, 10),
        items,
    };
    let s = solve(&p, &table_for(&p), &SolveOptions::default()).unwrap();
    assert_eq!(
        s.unplaced.iter().map(|u| u.id.as_str()).collect::<Vec<_>>(),
        vec!["c"]
    );
    assert_eq!(s.unplaced[0].reason, Reason::NoSpace);
}

#[test]
fn malformed_problems_are_errors() {
    let mut p = front_page();
    p.items.push(item("lead", Role::Article, 5, 100, None));
    assert_eq!(p.check(), Err(ProblemError::DuplicateId("lead".into())));
    assert!(solve(&p, &MeasurementTable::default(), &SolveOptions::default()).is_err());

    let mut p = front_page();
    p.items[2].priority = 11;
    assert!(matches!(p.check(), Err(ProblemError::PriorityOutOfRange(_))));

    let mut p = front_page();
    p.items[0].fixed = Some(at(1, 80, 1, 3));
    assert!(matches!(p.check(), Err(ProblemError::FixedOutOfBounds(_))));

    let mut p = front_page();
    p.items[1].fixed = Some(at(3, 10, 2, 3));
    assert!(matches!(p.check(), Err(ProblemError::FixedOverlap(..))));
}

#[test]
fn the_json_interface_matches_the_typed_one() {
    let p = front_page();
    let t = table_for(&p);
    let typed = solve(&p, &t, &SolveOptions::default()).unwrap();

    let problem_json = serde_json::to_string(&p).unwrap();
    let queries: Vec<MeasureQuery> =
        serde_json::from_str(&json::required_measurements(&problem_json).unwrap()).unwrap();
    assert_eq!(queries, required_measurements(&p));

    let table_json = serde_json::to_string(&t).unwrap();
    let out = json::solve(&problem_json, &table_json, "").unwrap();
    let from_json: Solution = serde_json::from_str(&out).unwrap();
    assert_eq!(from_json, typed);

    assert!(matches!(
        json::solve("{", &table_json, ""),
        Err(json::Error::Problem(_))
    ));
    assert!(matches!(
        json::solve(&problem_json, "nope", ""),
        Err(json::Error::Measurements(_))
    ));
    assert!(matches!(
        json::solve(&problem_json, &table_json, "{"),
        Err(json::Error::Options(_))
    ));
    let broken = json::solve(
        "{\"paper\":{\"dan\":0,\"lines\":10},\"items\":[]}",
        &table_json,
        "",
    );
    assert!(matches!(broken, Err(json::Error::Invalid(_))));
    assert!(json::solve("{", &table_json, "")
        .unwrap_err()
        .to_string()
        .starts_with("invalid problem"));
}

#[test]
fn placements_serialize_like_shinbun_maker() {
    let p = Placed {
        id: "a".into(),
        at: at(9, 17, 3, 5),
    };
    assert_eq!(
        serde_json::to_string(&p).unwrap(),
        r#"{"id":"a","x":9,"w":17,"y":3,"span":5}"#
    );
}

fn page_with_furniture(writing: Writing) -> Problem {
    let paper = match writing {
        Writing::Vertical => Paper::new(writing, 10, 50, 10),
        Writing::Horizontal => Paper::new(writing, 6, 40, 10),
    };
    let mut lead = item("lead", Role::Article, 10, 300, Some(Headline::L));
    lead.lead = true;
    Problem {
        paper,
        items: vec![
            item("nameplate", Role::Nameplate, 10, 0, None),
            item("yoko", Role::Yoko, 4, 0, None),
            lead,
            item("second", Role::Article, 6, 200, Some(Headline::M)),
            item("third", Role::Article, 3, 120, Some(Headline::S)),
            item("ad", Role::Ad, 3, 0, None),
        ],
    }
}

fn looks_wide(paper: &Paper, at: &Placement) -> bool {
    let along_lines = at.w as f64 * 1.55;
    let along_columns = paper.length_em(at.span);
    match paper.writing {
        Writing::Vertical => along_lines > along_columns,
        Writing::Horizontal => along_columns > along_lines,
    }
}

#[test]
fn the_ad_sits_at_the_bottom_and_headlines_and_ads_lie_flat() {
    for writing in [Writing::Vertical, Writing::Horizontal] {
        let p = page_with_furniture(writing);
        let s = solve(&p, &table_for(&p), &SolveOptions::default()).unwrap();
        assert!(s.unplaced.is_empty(), "{writing:?}: {:?}", s.unplaced);
        let ad = *s.layout.get("ad").unwrap();
        let bottom = match writing {
            Writing::Vertical => ad.y + ad.span - 1 == p.paper.dan,
            Writing::Horizontal => ad.x + ad.w - 1 == p.paper.lines,
        };
        assert!(bottom, "{writing:?}: the ad is not at the bottom: {ad:?}");
        assert!(
            looks_wide(&p.paper, &ad),
            "{writing:?}: the ad stands upright: {ad:?}"
        );
        let yoko = *s.layout.get("yoko").unwrap();
        assert!(
            looks_wide(&p.paper, &yoko),
            "{writing:?}: the headline stands upright: {yoko:?}"
        );
    }
}

#[test]
fn the_nameplate_is_upright_in_vertical_writing_and_a_band_in_horizontal_writing() {
    let p = page_with_furniture(Writing::Vertical);
    let n = *solve(&p, &table_for(&p), &SolveOptions::default())
        .unwrap()
        .layout
        .get("nameplate")
        .unwrap();
    assert!(!looks_wide(&p.paper, &n), "{n:?}");
    assert!(n.span < p.paper.dan, "{n:?}");
    let p = page_with_furniture(Writing::Horizontal);
    let n = *solve(&p, &table_for(&p), &SolveOptions::default())
        .unwrap()
        .layout
        .get("nameplate")
        .unwrap();
    assert_eq!(n.span, p.paper.dan, "{n:?}");
    assert_eq!(n.x, 1, "the nameplate is not at the top: {n:?}");
}

fn arb_item(index: usize) -> impl Strategy<Value = Item> {
    (
        prop_oneof![
            Just(Role::Article),
            Just(Role::Article),
            Just(Role::Photo),
            Just(Role::Box)
        ],
        1u8..=10,
        20u32..2500,
        0u8..4,
    )
        .prop_map(move |(role, priority, chars, h)| {
            let headline = [Headline::Xl, Headline::L, Headline::M, Headline::S][h as usize];
            item(
                &format!("i{index}"),
                role,
                priority,
                chars,
                (role == Role::Article).then_some(headline),
            )
        })
}

fn arb_problem() -> impl Strategy<Value = Problem> {
    ((4u8..=12), (20u8..=60), prop::collection::vec(0..1usize, 1..9)).prop_flat_map(|(dan, lines, slots)| {
        let items: Vec<_> = (0..slots.len()).map(arb_item).collect();
        items.prop_map(move |items| Problem {
            paper: Paper::new(Writing::Vertical, dan, lines, 10),
            items,
        })
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn a_solution_never_breaks_a_hard_constraint(p in arb_problem()) {
        let t = table_for(&p);
        let opts = SolveOptions::default().beam_width(4).branching(4).refine_iterations(15).separate_moves(300);
        let s = solve(&p, &t, &opts).unwrap();
        prop_assert!(hard(&s.warnings).is_empty(), "{:?}", s.warnings);
        prop_assert!(hard(&validate(&p, &t, &s.layout, &Rules::default())).is_empty());
        for alt in &s.alternatives {
            prop_assert!(hard(&validate(&p, &t, &alt.layout, &Rules::default())).is_empty());
        }
    }

    #[test]
    fn every_item_is_placed_or_reported_exactly_once(p in arb_problem()) {
        let t = table_for(&p);
        let opts = SolveOptions::default().beam_width(4).branching(4).refine_iterations(15).separate_moves(300);
        let s = solve(&p, &t, &opts).unwrap();
        let mut ids: Vec<&str> = s.layout.placements.iter().map(|x| x.id.as_str()).collect();
        ids.extend(s.unplaced.iter().map(|u| u.id.as_str()));
        ids.sort_unstable();
        let mut expected: Vec<&str> = p.items.iter().map(|i| i.id.as_str()).collect();
        expected.sort_unstable();
        prop_assert_eq!(ids, expected);
    }

    #[test]
    fn the_same_input_gives_the_same_output(p in arb_problem()) {
        let t = table_for(&p);
        let opts = SolveOptions::default().beam_width(4).branching(4).refine_iterations(15).separate_moves(300);
        prop_assert_eq!(solve(&p, &t, &opts).unwrap(), solve(&p, &t, &opts).unwrap());
    }

    #[test]
    fn an_item_is_dropped_only_when_nothing_fits(p in arb_problem()) {
        let t = table_for(&p);
        let opts = SolveOptions::default().beam_width(4).branching(4).refine_iterations(15).separate_moves(300);
        let s = solve(&p, &t, &opts).unwrap();
        for u in &s.unplaced {
            if u.reason == Reason::NoShape {
                let it = p.item(&u.id).unwrap();
                prop_assert!((1..=p.paper.dan).all(|span| needs(it, &p.paper, span).is_none()));
            }
        }
    }
}
