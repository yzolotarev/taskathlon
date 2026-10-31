//! Continuous procurement workbook. Stage changes never replace the learner's session.
//!
//! The route is a single transferable ladder: the layout (table start row/column, and the
//! order of the price/quantity columns) is derived from the route seed, so an answer cannot
//! be memorised as "B2*C2". A wrong step never advances: the app names the failing cell and
//! drops into a short corrective variation of the same atomic action in the same workbook,
//! then returns to the main sequence. The final stages are a manager's procurement brief.
use super::*;
use gridcraft_learn_core::{
    Check, Kind,
    rng::Rng,
    sheet::{self, Cell, Setup},
};

pub const PREFIX: &str = "route.supply.";
/// Number of ladder stages (0..STAGES-1). The manager's brief is stages 9..=11.
pub const STAGES: usize = 12;

/// High bit marking a corrective variation task; the low bits carry its index.
const FIX_BIT: u64 = 1 << 63;
/// Clear area for corrective variations, well below every main table layout.
const FIX_WINDOW: &str = "A24:H41";

/// Item pool for the first order; drawn directly (never localised) so tests stay stable.
const ITEMS: [&str; 10] = ["Бумага", "Ручки", "Папки", "Маркеры", "Степлеры", "Клей-карандаш", "Ножницы", "Скотч", "Блокноты", "Конверты"];

/// The manager's fixed procurement example. Prices and quantities are exact by requirement.
const COMPOSITE: [(&str, f64, f64); 5] =
    [("Датчик", 8500., 6.), ("Кабель", 1200., 30.), ("Контроллер", 42000., 2.), ("Крепёж", 150., 100.), ("Блок питания", 12500., 4.)];

pub struct Route {
    pub quiet: bool,
    pub paused: bool,
    pub elapsed: f32,
    pub started: Instant,
    pub feedback: Option<String>,
    pub complete: bool,
}
impl Route {
    pub fn seconds(&self) -> f32 {
        self.elapsed + if self.paused || self.complete { 0.0 } else { self.started.elapsed().as_secs_f32() }
    }
}

/// Column indices for one order table: name, price, quantity, cost.
fn cols(base: u32, swap: bool) -> (u32, u32, u32, u32) {
    if swap { (base, base + 2, base + 1, base + 3) } else { (base, base + 1, base + 2, base + 3) }
}

fn cost_ref(row: u32, price_col: u32, qty_col: u32) -> String {
    format!("={}*{}", sheet::a1(row, price_col), sheet::a1(row, qty_col))
}

fn sum_ref(first: &str, last: &str) -> String {
    format!("=SUM({first}:{last})")
}

fn value(cell: &str, expected: Cell) -> Check {
    Check { label: cell.into(), skill: "supply".into(), kind: Kind::Value { cell: cell.into(), expected } }
}

fn formula(cell: &str, reference: &str) -> Check {
    Check { label: cell.into(), skill: "supply".into(), kind: Kind::Formula { cell: cell.into(), reference: reference.into(), live: false } }
}

/// A correction task carries `FIX_BIT` in its spec data; a main task never does.
pub fn is_fix(task: &Task) -> bool {
    task.spec.data & FIX_BIT != 0
}

fn fix_index(spec: &Spec) -> u64 {
    (spec.data & !FIX_BIT) >> 8
}

fn main_spec(stage: usize, seed: u64) -> Spec {
    let mut spec = Spec::new(&format!("{PREFIX}{stage}"), 1, seed);
    // Keep main tasks clear of the corrective marker regardless of the derived data word.
    spec.data &= !FIX_BIT;
    spec
}

/// The main ladder task for `stage`. Deterministic: (stage, seed) always yields the same route.
pub fn task(stage: usize, seed: u64) -> Task {
    let stage = stage.min(STAGES - 1);
    let mut r = Rng::new(seed ^ 0x51A7_1234_ABCD_0001);
    let row_off = r.below(3) as u32; // 0..=2 shifts both tables down
    let col_off = r.below(2) as u32; // 0..=1 shifts both tables right
    let swap = r.chance(50);
    let (name_col, price_col, qty_col, cost_col) = cols(col_off, swap);

    let h1 = 1 + row_off;
    let data1 = h1 + 1;
    let given1 = data1 + 1;
    let data2 = h1 + 9;
    let n2 = COMPOSITE.len() as u32;
    let total2 = data2 + n2;

    let mut pool: Vec<&'static str> = ITEMS.to_vec();
    r.shuffle(&mut pool);
    let item = pool[0];
    let given_names = [pool[1], pool[2], pool[3]];
    let price1 = r.amount(50, 900, 50);
    let qty1 = r.int(2, 9) as f64;
    let qty2 = qty1 + r.int(1, 5) as f64;
    let mut given: Vec<(&'static str, f64, f64)> = Vec::new();
    for name in given_names {
        let price = r.amount(20, 400, 5);
        let qty = r.int(2, 12) as f64;
        given.push((name, price, qty));
    }
    let qty_change = given[1].2 + r.int(1, 5) as f64;
    let n_given = given.len() as u32;
    let total1 = given1 + n_given;

    let c1 = if swap { "Количество" } else { "Цена" };
    let c2 = if swap { "Цена" } else { "Количество" };
    let mut setup = Setup::default();
    setup.header(h1, col_off, &["Товар", c1, c2, "Стоимость"]);
    setup.header(data2 - 1, col_off, &["Позиция", c1, c2, "Стоимость"]);
    for (i, (name, price, qty)) in given.iter().enumerate() {
        let row = given1 + i as u32;
        setup.text(sheet::a1(row, name_col), *name);
        setup.put(sheet::a1(row, price_col), Cell::Num(*price), false);
        setup.put(sheet::a1(row, qty_col), Cell::Num(*qty), false);
    }
    for (i, (name, price, qty)) in COMPOSITE.iter().enumerate() {
        let row = data2 + i as u32;
        setup.text(sheet::a1(row, name_col), *name);
        setup.put(sheet::a1(row, price_col), Cell::Num(*price), false);
        setup.put(sheet::a1(row, qty_col), Cell::Num(*qty), false);
    }
    setup.text(sheet::a1(total1, name_col), "Итого");
    setup.text(sheet::a1(total2, name_col), "Итого");
    setup.width(name_col, 180.);
    for col in [price_col, qty_col, cost_col] {
        setup.width(col, 125.);
    }

    let mut checks = vec![value(&sheet::a1(data1, name_col), Cell::Text(item.into()))];
    if stage >= 1 {
        checks.push(value(&sheet::a1(data1, price_col), Cell::Num(price1)));
    }
    if stage >= 2 {
        checks.push(value(&sheet::a1(data1, qty_col), Cell::Num(if stage >= 3 { qty2 } else { qty1 })));
    }
    if stage >= 4 {
        checks.push(formula(&sheet::a1(data1, cost_col), &cost_ref(data1, price_col, qty_col)));
    }
    if stage >= 5 {
        checks.push(formula(&sheet::a1(given1, cost_col), &cost_ref(given1, price_col, qty_col)));
    }
    if stage >= 6 {
        for i in 1..n_given {
            let row = given1 + i;
            checks.push(formula(&sheet::a1(row, cost_col), &cost_ref(row, price_col, qty_col)));
        }
    }
    if stage >= 7 {
        checks.push(formula(&sheet::a1(total1, cost_col), &sum_ref(&sheet::a1(data1, cost_col), &sheet::a1(given1 + n_given - 1, cost_col))));
    }
    if stage >= 8 {
        checks.push(value(&sheet::a1(given1 + 1, qty_col), Cell::Num(qty_change)));
    }
    if stage >= 9 {
        for i in 0..n2 {
            let row = data2 + i;
            checks.push(formula(&sheet::a1(row, cost_col), &cost_ref(row, price_col, qty_col)));
        }
        checks.push(formula(&sheet::a1(total2, cost_col), &sum_ref(&sheet::a1(data2, cost_col), &sheet::a1(data2 + n2 - 1, cost_col))));
    }
    if stage >= 10 {
        checks.push(value(&sheet::a1(data2 + 1, qty_col), Cell::Num(45.)));
    }

    let focus = match stage {
        0 => sheet::a1(data1, name_col),
        1 => sheet::a1(data1, price_col),
        2 | 3 => sheet::a1(data1, qty_col),
        4 => sheet::a1(data1, cost_col),
        5 => sheet::a1(given1, cost_col),
        6 => format!("{}:{}", sheet::a1(given1, cost_col), sheet::a1(given1 + n_given - 1, cost_col)),
        7 => sheet::a1(total1, cost_col),
        8 => sheet::a1(given1 + 1, qty_col),
        9 => sheet::a1(data2, cost_col),
        10 => sheet::a1(data2 + 1, qty_col),
        _ => sheet::a1(total2, cost_col),
    };

    let prompt = match stage {
        0 => format!("В {} напечатайте «{}» — это новая позиция заказа. Нажмите Enter.", sheet::a1(data1, name_col), item),
        1 => format!("В {} введите цену {} для позиции «{}».", sheet::a1(data1, price_col), sheet::num_text(price1), item),
        2 => format!("В {} введите количество {}.", sheet::a1(data1, qty_col), sheet::num_text(qty1)),
        3 => format!("Заказ уточнили: замените {} на {} (двойной щелчок или F2).", sheet::a1(data1, qty_col), sheet::num_text(qty2)),
        4 => format!(
            "Стоимость = цена × количество. В {} введите ={}*{}.",
            sheet::a1(data1, cost_col),
            sheet::a1(data1, price_col),
            sheet::a1(data1, qty_col)
        ),
        5 => format!(
            "В {} введите ={}*{} для строки «{}». Ссылки сдвинулись на строку.",
            sheet::a1(given1, cost_col),
            sheet::a1(given1, price_col),
            sheet::a1(given1, qty_col),
            given[0].0
        ),
        6 => format!(
            "Выделите {}:{} и нажмите Ctrl+D — формула заполнит строки с относительными ссылками.",
            sheet::a1(given1, cost_col),
            sheet::a1(given1 + n_given - 1, cost_col)
        ),
        7 => format!(
            "В {} посчитайте весь заказ: =СУММ({}:{}).",
            sheet::a1(total1, cost_col),
            sheet::a1(data1, cost_col),
            sheet::a1(given1 + n_given - 1, cost_col)
        ),
        8 => format!(
            "Поставщик уточнил количество: замените {} на {}. Стоимости и итог должны пересчитаться.",
            sheet::a1(given1 + 1, qty_col),
            sheet::num_text(qty_change)
        ),
        9 => format!(
            "Андрей · Руководитель снабжения: «Соберите расчёт по закупке в таблице ниже. В столбце {} посчитайте стоимость каждой позиции формулой цена × количество, а в {} — общий итог. Способ любой — главное, чтобы это были живые формулы.»",
            sheet::col_name(cost_col),
            sheet::a1(total2, cost_col)
        ),
        10 => format!(
            "Андрей · Руководитель снабжения: «Кабеля нужно больше: в {} поставьте 45 вместо 30. Стоимость строки и общий итог должны пересчитаться сами — формулы ссылаются на данные.»",
            sheet::a1(data2 + 1, qty_col)
        ),
        _ => "Андрей · Руководитель снабжения: «Проверьте расчёт и отправьте результат кнопкой „Отправить заказы“.»".into(),
    };

    Task {
        spec: main_spec(stage, seed),
        mode: Mode::Drill,
        title: "Заказ поставщику".into(),
        prompt,
        example: None,
        hints: vec![],
        focus,
        setup,
        checks,
        goal_seconds: 0.,
        reorders: true,
        brief: None,
        sql: None,
    }
}

/// Which atomic action the stage introduces; the corrective variation repeats exactly this.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Act {
    Name,
    Price,
    Qty,
    QtyChange,
    Cost,
    Fill,
    Total,
    Composite,
}

fn act(stage: usize) -> Act {
    match stage {
        0 => Act::Name,
        1 => Act::Price,
        2 => Act::Qty,
        3 => Act::QtyChange,
        4 | 5 => Act::Cost,
        6 => Act::Fill,
        7 => Act::Total,
        9 | 11 => Act::Composite,
        _ => Act::QtyChange,
    }
}

/// A short (20–40 s) variation of the current atomic action, in a fresh layout inside the
/// same workbook. Its spec carries `FIX_BIT` so the app knows to return to the main step.
pub fn fix_task(stage: usize, seed: u64, index: u64) -> Task {
    let stage = stage.min(STAGES - 1);
    let mut r = Rng::new(seed ^ index.wrapping_mul(0x2545_f491_4f6c_dd1d) ^ 0x71A2_9E00_1234);
    let cr = 26 + r.below(8) as u32; // 26..=33
    let cc = r.below(4) as u32; // 0..=3
    let swap = r.chance(50);
    let (name_col, price_col, qty_col, cost_col) = cols(cc, swap);
    let c1 = if swap { "Количество" } else { "Цена" };
    let c2 = if swap { "Цена" } else { "Количество" };
    let mut setup = Setup::default();
    setup.header(cr, cc, &["Товар", c1, c2, "Стоимость"]);
    setup.width(name_col, 180.);
    for col in [price_col, qty_col, cost_col] {
        setup.width(col, 125.);
    }
    let mut pool: Vec<&'static str> = ITEMS.to_vec();
    r.shuffle(&mut pool);
    let (n1, n2, n3) = (pool[0], pool[1], pool[2]);
    let p1 = r.amount(20, 900, 5);
    let q1 = r.int(2, 9) as f64;
    let p2 = r.amount(20, 900, 5);
    let q2 = r.int(2, 9) as f64;
    let p3 = r.amount(20, 900, 5);
    let q3 = r.int(2, 9) as f64;
    let q1b = q1 + r.int(1, 4) as f64;

    let r1 = cr + 1;
    let r2 = cr + 2;
    let r3 = cr + 3;
    let mut checks: Vec<Check> = Vec::new();
    let focus = match act(stage) {
        Act::Name => {
            setup.put(sheet::a1(r1, price_col), Cell::Num(p1), false);
            setup.put(sheet::a1(r1, qty_col), Cell::Num(q1), false);
            setup.text(sheet::a1(r2, name_col), n2);
            setup.put(sheet::a1(r2, price_col), Cell::Num(p2), false);
            setup.put(sheet::a1(r2, qty_col), Cell::Num(q2), false);
            checks.push(value(&sheet::a1(r1, name_col), Cell::Text(n1.into())));
            sheet::a1(r1, name_col)
        }
        Act::Price => {
            setup.text(sheet::a1(r1, name_col), n1);
            setup.put(sheet::a1(r1, qty_col), Cell::Num(q1), false);
            setup.text(sheet::a1(r2, name_col), n2);
            setup.put(sheet::a1(r2, price_col), Cell::Num(p2), false);
            setup.put(sheet::a1(r2, qty_col), Cell::Num(q2), false);
            checks.push(value(&sheet::a1(r1, price_col), Cell::Num(p1)));
            sheet::a1(r1, price_col)
        }
        Act::Qty => {
            setup.text(sheet::a1(r1, name_col), n1);
            setup.put(sheet::a1(r1, price_col), Cell::Num(p1), false);
            setup.text(sheet::a1(r2, name_col), n2);
            setup.put(sheet::a1(r2, price_col), Cell::Num(p2), false);
            setup.put(sheet::a1(r2, qty_col), Cell::Num(q2), false);
            checks.push(value(&sheet::a1(r1, qty_col), Cell::Num(q1)));
            sheet::a1(r1, qty_col)
        }
        Act::QtyChange => {
            setup.text(sheet::a1(r1, name_col), n1);
            setup.put(sheet::a1(r1, price_col), Cell::Num(p1), false);
            setup.put(sheet::a1(r1, qty_col), Cell::Num(q1), false);
            setup.put(sheet::a1(r1, cost_col), Cell::Formula(cost_ref(r1, price_col, qty_col)), false);
            checks.push(value(&sheet::a1(r1, qty_col), Cell::Num(q1b)));
            checks.push(formula(&sheet::a1(r1, cost_col), &cost_ref(r1, price_col, qty_col)));
            sheet::a1(r1, qty_col)
        }
        Act::Cost => {
            setup.text(sheet::a1(r1, name_col), n1);
            setup.put(sheet::a1(r1, price_col), Cell::Num(p1), false);
            setup.put(sheet::a1(r1, qty_col), Cell::Num(q1), false);
            setup.text(sheet::a1(r2, name_col), n2);
            setup.put(sheet::a1(r2, price_col), Cell::Num(p2), false);
            setup.put(sheet::a1(r2, qty_col), Cell::Num(q2), false);
            checks.push(formula(&sheet::a1(r1, cost_col), &cost_ref(r1, price_col, qty_col)));
            sheet::a1(r1, cost_col)
        }
        Act::Fill => {
            setup.text(sheet::a1(r1, name_col), n1);
            setup.put(sheet::a1(r1, price_col), Cell::Num(p1), false);
            setup.put(sheet::a1(r1, qty_col), Cell::Num(q1), false);
            setup.put(sheet::a1(r1, cost_col), Cell::Formula(cost_ref(r1, price_col, qty_col)), false);
            setup.text(sheet::a1(r2, name_col), n2);
            setup.put(sheet::a1(r2, price_col), Cell::Num(p2), false);
            setup.put(sheet::a1(r2, qty_col), Cell::Num(q2), false);
            setup.text(sheet::a1(r3, name_col), n3);
            setup.put(sheet::a1(r3, price_col), Cell::Num(p3), false);
            setup.put(sheet::a1(r3, qty_col), Cell::Num(q3), false);
            checks.push(formula(&sheet::a1(r2, cost_col), &cost_ref(r2, price_col, qty_col)));
            checks.push(formula(&sheet::a1(r3, cost_col), &cost_ref(r3, price_col, qty_col)));
            format!("{}:{}", sheet::a1(r2, cost_col), sheet::a1(r3, cost_col))
        }
        Act::Total => {
            setup.text(sheet::a1(r1, name_col), n1);
            setup.put(sheet::a1(r1, price_col), Cell::Num(p1), false);
            setup.put(sheet::a1(r1, qty_col), Cell::Num(q1), false);
            setup.put(sheet::a1(r1, cost_col), Cell::Formula(cost_ref(r1, price_col, qty_col)), false);
            setup.text(sheet::a1(r2, name_col), n2);
            setup.put(sheet::a1(r2, price_col), Cell::Num(p2), false);
            setup.put(sheet::a1(r2, qty_col), Cell::Num(q2), false);
            setup.put(sheet::a1(r2, cost_col), Cell::Formula(cost_ref(r2, price_col, qty_col)), false);
            setup.text(sheet::a1(r3, name_col), "Итого");
            checks.push(formula(&sheet::a1(r3, cost_col), &sum_ref(&sheet::a1(r1, cost_col), &sheet::a1(r2, cost_col))));
            sheet::a1(r3, cost_col)
        }
        Act::Composite => {
            for (row, nm, p, q) in [(r1, n1, p1, q1), (r2, n2, p2, q2)] {
                setup.text(sheet::a1(row, name_col), nm);
                setup.put(sheet::a1(row, price_col), Cell::Num(p), false);
                setup.put(sheet::a1(row, qty_col), Cell::Num(q), false);
            }
            setup.text(sheet::a1(r3, name_col), "Итого");
            checks.push(formula(&sheet::a1(r1, cost_col), &cost_ref(r1, price_col, qty_col)));
            checks.push(formula(&sheet::a1(r2, cost_col), &cost_ref(r2, price_col, qty_col)));
            checks.push(formula(&sheet::a1(r3, cost_col), &sum_ref(&sheet::a1(r1, cost_col), &sheet::a1(r2, cost_col))));
            sheet::a1(r1, cost_col)
        }
    };

    let prompt = format!(
        "Тренировка в {} — то же действие на новом примере: {}",
        sheet::a1(cr, name_col),
        match act(stage) {
            Act::Name => format!("напечатайте название позиции в {}.", sheet::a1(r1, name_col)),
            Act::Price => format!("введите цену в {}.", sheet::a1(r1, price_col)),
            Act::Qty => format!("введите количество в {}.", sheet::a1(r1, qty_col)),
            Act::QtyChange => format!("поправьте количество в {} — стоимость пересчитается.", sheet::a1(r1, qty_col)),
            Act::Cost => format!("посчитайте стоимость позиции в {} формулой.", sheet::a1(r1, cost_col)),
            Act::Fill => format!("заполните формулами {}:{}.", sheet::a1(r2, cost_col), sheet::a1(r3, cost_col)),
            Act::Total => format!("посчитайте итог в {} формулой.", sheet::a1(r3, cost_col)),
            Act::Composite => format!(
                "посчитайте стоимость позиций в {}:{} и итог в {}.",
                sheet::a1(r1, cost_col),
                sheet::a1(r2, cost_col),
                sheet::a1(r3, cost_col)
            ),
        }
    );

    Task {
        spec: Spec { skill: format!("{PREFIX}{stage}"), level: 1, layout: seed, data: FIX_BIT | (index << 8) | stage as u64 },
        mode: Mode::Drill,
        title: "Заказ поставщику".into(),
        prompt,
        example: None,
        hints: vec![],
        focus,
        setup,
        checks,
        goal_seconds: 0.,
        reorders: true,
        brief: None,
        sql: None,
    }
}

/// Names of the requirements that are not met (empty = the step is done). Names the cells.
fn failures(task: &Task, session: &gridcraft_engine::Session) -> Result<Vec<String>, LearnError> {
    let wb = &session.doc()?.wb;
    let mut out: Vec<String> = Vec::new();
    let targets: Vec<String> = task
        .checks
        .iter()
        .filter_map(|c| match &c.kind {
            Kind::Value { cell, .. } | Kind::Formula { cell, .. } => Some(cell.clone()),
            _ => None,
        })
        .collect();

    for check in &task.checks {
        match &check.kind {
            Kind::Value { cell, expected } => {
                let want = match expected {
                    Cell::Num(n) | Cell::Date(n) => gridcraft_engine::core::Value::number(*n),
                    Cell::Text(t) => gridcraft_engine::core::Value::text(t.as_str()),
                    Cell::Formula(_) => {
                        out.push(format!("{cell}: нужно посчитать значение"));
                        continue;
                    }
                };
                if !gridcraft_learn_core::check::equivalent(&sheet::value(session, cell)?, &want) {
                    out.push(format!("{cell}: нужно {}", sheet::show(&want)));
                }
            }
            Kind::Formula { cell, .. } if !sheet::has_formula(wb, cell) => {
                out.push(format!("{cell}: нужна формула, а не число"));
            }
            _ => {}
        }
    }

    // Given data stays intact, apart from cells the learner is explicitly asked to change.
    for (cell, content, _) in &task.setup.cells {
        if targets.iter().any(|t| t == cell) {
            continue;
        }
        let want = match content {
            Cell::Num(n) | Cell::Date(n) => gridcraft_engine::core::Value::number(*n),
            Cell::Text(t) => gridcraft_engine::core::Value::text(t.as_str()),
            _ => continue,
        };
        if !gridcraft_learn_core::check::equivalent(&sheet::value(session, cell)?, &want) {
            out.push(format!("{cell}: исходные данные изменены"));
        }
    }

    // Formulas are retested on altered quantities/prices, so a hardcoded result cannot pass.
    let scratch = sheet::a1(1, 30);
    for offset in [0., 3., 17.] {
        let mut probe = gridcraft_engine::Session::new();
        probe.add_document(gridcraft_engine::DocState::new((*session.doc()?.wb).clone(), None, "Проверка заказа".into()));
        if offset != 0. {
            let mut i = 0.;
            for (cell, content, _) in &task.setup.cells {
                if let Cell::Num(n) = content {
                    i += 1.;
                    sheet::set(&mut probe, cell, &Cell::Num(*n + offset + i * 3.))?;
                }
            }
        }
        for check in &task.checks {
            if let Kind::Formula { cell, reference, .. } = &check.kind {
                if !sheet::has_formula(wb, cell) {
                    continue; // already reported above
                }
                sheet::set(&mut probe, &scratch, &Cell::Formula(reference.clone()))?;
                if !gridcraft_learn_core::check::equivalent(&sheet::value(&probe, cell)?, &sheet::value(&probe, &scratch)?) {
                    out.push(format!("{cell}: формула ошибается на изменённых данных"));
                }
            }
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}

/// Atomic grading for the supply route: value cells, live formulas retested on altered data.
pub fn valid(task: &Task, session: &gridcraft_engine::Session) -> Result<bool, LearnError> {
    Ok(failures(task, session)?.is_empty())
}

impl LearningApp {
    pub fn supply_stage(&self) -> Option<usize> {
        self.active.as_ref()?.task.spec.skill.strip_prefix(PREFIX)?.parse::<usize>().ok().filter(|s| *s < STAGES)
    }
    pub fn start_supply(&mut self) -> Result<(), LearnError> {
        let seed = self.profile.seed();
        self.load(task(0, seed), None, None, None)?;
        self.supply = Some(Route { quiet: true, paused: false, elapsed: 0., started: instant(), feedback: None, complete: false });
        self.screen = Screen::Supply;
        self.save()
    }
    pub fn pause_supply(&mut self) {
        if let Some(route) = &mut self.supply {
            route.elapsed = route.seconds();
            route.started = instant();
            route.paused = !route.paused;
            if let Some(a) = &mut self.active {
                a.query = if route.paused { "paused" } else { "" }.into();
            }
        }
        let r = self.save();
        self.handle(r);
    }

    /// Presents a corrective variation of the current atomic action in the same workbook.
    fn start_fix(&mut self, stage: usize, seed: u64, index: u64) -> Result<(), LearnError> {
        let fix = fix_task(stage, seed, index);
        self.sheet.session.execute("edit.clearContents", serde_json::json!({ "range": FIX_WINDOW }))?;
        fix.setup.apply(&mut self.sheet.session)?;
        if let Some(a) = &mut self.active {
            a.task = fix.clone();
        }
        self.sheet.session.execute("selection.set", serde_json::json!({ "range": fix.focus }))?;
        self.graded_revision = self.revision();
        self.save()
    }

    /// Brings the learner back to the main step after a successful correction.
    fn return_from_fix(&mut self, stage: usize, seed: u64) -> Result<(), LearnError> {
        let main = task(stage, seed);
        if let Some(a) = &mut self.active {
            a.task = main.clone();
        }
        self.sheet.session.execute("selection.set", serde_json::json!({ "range": main.focus }))?;
        if let Some(route) = &mut self.supply {
            route.feedback = Some("Верно! Возвращаемся к основному заказу — исправьте шаг.".into());
        }
        self.graded_revision = self.revision();
        self.save()
    }

    pub fn check_supply(&mut self, submit: bool) -> Result<(), LearnError> {
        let Some(stage) = self.supply_stage() else { return Ok(()) };
        if self.sheet.editor.is_some() || self.supply.as_ref().is_none_or(|r| r.paused || r.complete) {
            return Ok(());
        }
        let in_fix = self.active.as_ref().is_some_and(|a| is_fix(&a.task));
        if stage == STAGES - 1 && !submit && !in_fix {
            return Ok(());
        }
        let Some(current) = self.active.as_ref().map(|a| a.task.clone()) else { return Ok(()) };
        let revision = self.revision();
        if !submit && revision == self.graded_revision {
            return Ok(());
        }
        self.graded_revision = revision;
        let seed = current.spec.layout;
        let passed = valid(&current, &self.sheet.session)?;

        if in_fix {
            if passed {
                self.return_from_fix(stage, seed)?;
            } else if submit {
                let index = fix_index(&current.spec);
                let msgs = failures(&current, &self.sheet.session)?;
                self.start_fix(stage, seed, index + 1)?;
                if let Some(route) = &mut self.supply {
                    route.feedback = Some(format!("Ещё раз, аккуратнее — {}.", msgs.join("; ")));
                }
            }
            return Ok(());
        }

        if !passed {
            if submit {
                let msgs = failures(&current, &self.sheet.session)?;
                if let Some(route) = &mut self.supply {
                    route.feedback = Some(format!("Пока не готово — {}. Потренируемся на коротком примере.", msgs.join("; ")));
                }
                self.start_fix(stage, seed, 0)?;
            }
            return Ok(());
        }

        if let Some(route) = &mut self.supply {
            route.feedback = Some(
                if stage == STAGES - 1 {
                    "Заказы отправлены. Формулы работают и на изменённых данных."
                } else {
                    "✓ Готово — продолжайте в этой же книге."
                }
                .into(),
            );
            if stage == STAGES - 1 {
                route.elapsed = route.seconds();
                route.complete = true;
            }
        }
        if stage < STAGES - 1
            && let Some(a) = &mut self.active
        {
            a.task = task(stage + 1, a.task.spec.layout);
        }
        // Do not move selection: Enter belongs solely to the spreadsheet editor.
        self.save()
    }
}

pub fn show(app: &mut LearningApp, ui: &mut egui::Ui) {
    let p = crate::view::palette(app.sheet.ui.dark);
    egui::CentralPanel::default().frame(egui::Frame::NONE.fill(p.bg).inner_margin(egui::Margin::symmetric(16, 10))).show(ui, |ui| route(app, ui, &p));
}

fn route(app: &mut LearningApp, ui: &mut egui::Ui, p: &crate::view::Palette) {
    let stage = app.supply_stage().unwrap_or(0);
    let in_fix = app.active.as_ref().is_some_and(|a| is_fix(&a.task));
    ui.horizontal(|ui| {
        if ui.button("← Сохранить и выйти").clicked() {
            if app.supply.as_ref().is_some_and(|r| !r.paused) {
                app.pause_supply();
            }
            let r = app.save();
            app.handle(r);
            app.screen = Screen::Home;
        }
        ui.heading("Заказ поставщику");
        ui.label(format!("Шаг {} / {STAGES} · первая книга", stage + 1));
        if ui.button(if app.supply.as_ref().is_some_and(|r| r.paused) { "Продолжить" } else { "Пауза" }).clicked() {
            app.pause_supply();
        }
    });
    let prompt = app.active.as_ref().map(|a| a.task.prompt.clone()).unwrap_or_default();
    egui::Frame::NONE.fill(p.card).stroke(egui::Stroke::new(2.0, p.border)).corner_radius(12.0).inner_margin(egui::Margin::symmetric(14, 10)).show(
        ui,
        |ui| {
            ui.set_width(ui.available_width());
            if in_fix {
                ui.label(egui::RichText::new("Тренировка: короткий пример того же действия.").color(p.orange));
            }
            ui.label(egui::RichText::new(prompt).size(18.0).color(p.text));
            if let Some(route) = &app.supply {
                if !route.quiet {
                    ui.label(egui::RichText::new(format!("Время работы: {:.0} с", route.seconds())).color(p.muted));
                }
                if let Some(text) = &route.feedback {
                    ui.label(egui::RichText::new(text).color(p.muted));
                }
            }
        },
    );
    ui.add_space(6.0);
    if in_fix {
        if ui.button("Проверить пример").clicked() {
            let r = app.check_supply(true);
            app.handle(r);
        }
    } else if stage == STAGES - 1 && !app.supply.as_ref().is_some_and(|r| r.complete) {
        if ui.button("Отправить заказы").clicked() {
            let r = app.check_supply(true);
            app.handle(r);
        }
    } else if ui.button("Проверить этот шаг").clicked() {
        let r = app.check_supply(true);
        app.handle(r);
    }
    if app.supply.as_ref().is_some_and(|r| r.paused) {
        ui.label("Пауза. Книга сохранена, время остановлено.");
        return;
    }
    ui.separator();
    let complete = app.supply.as_ref().is_some_and(|r| r.complete);
    ui.add_enabled_ui(!complete, |ui| app.sheet.ui(ui));
    if app.supply.as_ref().is_some_and(|r| r.complete) {
        return;
    }
    let r = app.check_supply(false);
    app.handle(r);
}
