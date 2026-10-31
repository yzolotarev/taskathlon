//! «Неделя в Орионе»: the first working week of a supply assistant at a LoRaWAN device maker,
//! one day after another, from reading the order register to the weekly report.
//!
//! The world is invented but built from public facts about such a company: radio modems
//! (LA-IP, a cap modem for water meters, a gas modem with NB-IoT), components from Chinese
//! suppliers paid in USD/CNY with prepayment and minimum batches, local packaging in tenge.
//! Nothing here claims to be the company's internal process. Russian-only, like `asm`.

use super::{Gen, Table};
use crate::check::{Col, Expectation, Kind};
use crate::sheet::{Cell, a1, date, ru};
use crate::{Result, Task};

/// (name, currency, manager, phone)
const SUPPLIERS: &[(&str, &str, &str, &str)] = &[
    ("Shenzhen Lianda Electronics", "USD", "Ли Вэй", "+86 755 2690 1180"),
    ("Wuhan PowerCell", "CNY", "Чжан Мин", "+86 27 8765 4321"),
    ("Dongguan Plastech", "CNY", "Анна Хэ", "+86 769 2233 4455"),
    ("ТОО КазРадиоКомплект", "KZT", "Ержан Сейткали", "+7 727 355 12 40"),
    ("ТОО АлматыПак", "KZT", "Динара Абенова", "+7 727 390 77 15"),
];

/// (article, name, category, supplier index, price in the supplier's currency)
const PARTS: &[(&str, &str, &str, usize, f64)] = &[
    ("PCB-LAIP", "Плата LA-IP в сборе", "Электроника", 0, 6.8),
    ("PCB-TWM", "Плата TWM в сборе", "Электроника", 0, 5.9),
    ("RF-868", "LoRa-модуль 868 МГц", "Электроника", 0, 4.2),
    ("NB-MOD", "NB-IoT модуль", "Электроника", 0, 7.5),
    ("ANT-868", "Антенна 868 МГц", "Электроника", 0, 0.9),
    ("BAT-14505", "Батарея ER14505 3,6 В", "Батареи", 1, 9.5),
    ("CASE-TWM", "Корпус-крышка TWM", "Корпуса", 2, 4.8),
    ("CASE-LA", "Корпус LA-IP", "Корпуса", 2, 5.6),
    ("SEAL-01", "Уплотнитель корпуса", "Корпуса", 2, 0.6),
    ("CBL-IMP", "Кабель импульсный, м", "Кабель и упаковка", 3, 180.0),
    ("LBL-QR", "Этикетка с QR-кодом", "Кабель и упаковка", 4, 12.0),
    ("BOX-1", "Коробка индивидуальная", "Кабель и упаковка", 4, 95.0),
];

/// Bill of materials: component, then norm per LA-IP / TWM / GMZ.
const BOM: &[(&str, [f64; 3])] = &[
    ("Плата в сборе", [1.0, 1.0, 1.0]),
    ("LoRa-модуль 868 МГц", [1.0, 1.0, 1.0]),
    ("NB-IoT модуль", [0.0, 0.0, 1.0]),
    ("Батарея ER14505", [1.0, 1.0, 2.0]),
    ("Антенна 868 МГц", [1.0, 0.0, 1.0]),
    ("Корпус", [1.0, 1.0, 1.0]),
    ("Уплотнитель", [1.0, 1.0, 1.0]),
    ("Кабель импульсный, м", [3.0, 0.0, 0.0]),
    ("Этикетка с QR-кодом", [1.0, 1.0, 1.0]),
    ("Коробка индивидуальная", [1.0, 1.0, 1.0]),
    ("Гофрокороб на 20 шт", [0.05, 0.05, 0.05]),
];

const RECEIVED: &str = "Получен";
const OPEN: [&str; 2] = ["Заказан", "В пути"];

/// `B7` → `$B$7`.
fn dollar(cell: &str) -> String {
    let letters: String = cell.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
    let digits: String = cell.chars().skip_while(|c| c.is_ascii_alphabetic()).collect();
    format!("${letters}${digits}")
}

fn supplier(i: usize) -> &'static str {
    SUPPLIERS.get(i).map_or("Wuhan PowerCell", |s| s.0)
}

/// A formula column: header, highlighted answer cells and one live formula check per row.
/// Returns the first reference in Russian syntax, for the example.
fn answer_col(g: &mut Gen, t: &Table, col: u32, header: &str, reference: &dyn Fn(u32) -> String) -> String {
    // The header is part of the table, so parameters to the right never cover answers.
    let _ = header;
    g.setup.width(t.col(col), 135.0);
    for r in t.rows() {
        let cell = t.cell(col, r);
        g.setup.answer(cell.clone());
        g.formula_check(&cell, &reference(r));
    }
    format!("{}: {}", t.cell(col, t.first), ru(&reference(t.first)))
}

/// Level 1 shows the formulas; level 2 names the tool; from level 3 only Андрей's words.
fn finish(g: &mut Gen, title: &str, l1: String, l2: String, work: String, example: String, focus: String, hints: Vec<String>, seconds: f32) -> Task {
    let prompt = match g.level {
        1 => format!("{l1} Образец:"),
        2 => l2,
        _ => work,
    };
    let mut task = g.task(title, prompt, Some(example.clone()), focus);
    let mut all = hints;
    if g.level >= 2 {
        all.push(format!("Ответ: {example}"));
    }
    task.hints = all;
    task.goal_seconds = seconds + 20.0 * f32::from(g.level);
    task
}

// ---------------------------------------------------------------- день 1

/// How many orders are open, for how much, and how many with one supplier.
pub fn open(g: &mut Gen) -> Result<Task> {
    let n = g.rows() + 3;
    let t = g.table("Реестр заказов поставщикам (выгрузка из ERP)", &["№ заказа", "Поставщик", "Статус", "Сумма, тг"], n);
    g.setup.width(t.col(1), 200.0);
    let start = 1000 + g.l.int(0, 800);
    let statuses = ["Заказан", "В пути", RECEIVED];
    let forced = t.first + g.l.below(n as usize) as u32;
    let focus_sup = supplier(g.l.below(3));
    let forced_sup = t.first + ((forced - t.first + 1) % n);
    for (i, r) in t.rows().enumerate() {
        g.txt(&t, 0, r, &format!("PO-{}", start + i as i64));
        let s = if r == forced_sup { focus_sup } else { supplier(g.d.below(SUPPLIERS.len())) };
        g.txt(&t, 1, r, s);
        let st = if r == forced { "В пути" } else { statuses[g.d.below(3)] };
        g.txt(&t, 2, r, st);
        let v = g.d.amount(80_000, 2_500_000, 1_000);
        g.num(&t, 3, r, v);
    }
    let (st, amt, sup) = (t.abs(2), t.abs(3), t.abs(1));
    let a1c = g.answer(&t, 1, "Заказов в пути:");
    let a2c = g.answer(&t, 2, "Сумма в пути, тг:");
    let r1 = format!("=COUNTIF({st},\"В пути\")");
    let r2 = format!("=SUMIF({st},\"В пути\",{amt})");
    g.formula_check(&a1c, &r1);
    g.formula_check(&a2c, &r2);
    let mut example = format!("{a1c}: {}\n{a2c}: {}", ru(&r1), ru(&r2));
    if g.level >= 3 {
        let a3c = g.answer(&t, 3, "Открытых у поставщика:");
        let r3 = format!("=COUNTIFS({sup},\"{focus_sup}\",{st},\"<>{RECEIVED}\")");
        g.formula_check(&a3c, &r3);
        example.push_str(&format!("\n{a3c}: {}", ru(&r3)));
    }
    let work = if g.level >= 3 {
        format!(
            "Андрей: «Сколько у нас заказов в пути и на какую сумму? И сколько ещё не получено от {focus_sup} - хочу им сегодня позвонить». Ответы - в выделенные ячейки, формулами."
        )
    } else {
        String::new()
    };
    Ok(finish(
        g,
        "Открытые заказы",
        format!("Посчитай, сколько заказов в статусе «В пути» ({a1c}) и на какую они сумму ({a2c})."),
        format!("СЧЁТЕСЛИ: сколько заказов «В пути» → {a1c}. СУММЕСЛИ: их сумма → {a2c}."),
        work,
        example,
        a1c,
        vec![
            "«Сколько заказов» - это СЧЁТЕСЛИ (считает строки), «на какую сумму» - СУММЕСЛИ (складывает деньги). Не путай число заказов с суммой.".into(),
            "«Не получено» - это статус не равен «Получен»: условие \"<>Получен\". Два условия сразу (поставщик и статус) - СЧЁТЕСЛИМН.".into(),
        ],
        60.0,
    ))
}

// ---------------------------------------------------------------- день 2

/// Morning control: late, due within 3 days, or fine.
pub fn overdue(g: &mut Gen) -> Result<Task> {
    let n = g.rows() + 2;
    let t = g.table("Поставки: утренний контроль", &["№ заказа", "Поставщик", "Ожидаем", "Статус", "Контроль"], n);
    g.setup.width(t.col(1), 200.0);
    let today = date(2026, 10, 12) + g.l.int(0, 8) as f64;
    let today_cell = g.param(&t, 1, "Сегодня:", Cell::Date(today));
    let td = dollar(&today_cell);
    let start = 2000 + g.l.int(0, 700);
    let mut cases: Vec<usize> = (0..n as usize).map(|i| i % 5).collect();
    g.l.shuffle(&mut cases);
    for (i, r) in t.rows().enumerate() {
        g.txt(&t, 0, r, &format!("PO-{}", start + i as i64));
        let s = supplier(g.d.below(SUPPLIERS.len()));
        g.txt(&t, 1, r, s);
        let open = OPEN[g.d.below(2)];
        let (due, status) = match cases.get(i).copied().unwrap_or(4) {
            0 => (today - g.d.int(1, 9) as f64, open),
            1 => (today - g.d.int(1, 9) as f64, RECEIVED),
            2 => (today + g.d.int(0, 3) as f64, open),
            3 => (today + g.d.int(4, 20) as f64, open),
            _ => (today + g.d.int(-5, 15) as f64, RECEIVED),
        };
        g.day(&t, 2, r, due);
        g.txt(&t, 3, r, status);
    }
    let reference = |r: u32| {
        format!(
            "=IF({s}=\"{RECEIVED}\",\"\",IF({d}<{td},\"Просрочено\",IF({d}-{td}<=3,\"Скоро\",\"\")))",
            s = t.cell(3, r),
            d = t.cell(2, r)
        )
    };
    let example = answer_col(g, &t, 4, "Контроль", &reference);
    let (c1, c2) = (t.cell(4, t.first), t.cell(4, t.last));
    Ok(finish(
        g,
        "Утренний контроль поставок",
        format!("В {c1}:{c2}: если заказ не получен и дата прошла - «Просрочено», если до даты 3 дня и меньше - «Скоро», иначе пусто. Введи в {c1} и протяни."),
        format!("Вложенные ЕСЛИ в {c1}:{c2}: получен → пусто; дата < сегодня ({today_cell}) → «Просрочено»; дата − сегодня ≤ 3 → «Скоро»; иначе пусто."),
        format!(
            "Андрей: «С утра первым делом - какие поставки уже опаздывают, а какие вот-вот должны прийти (3 дня и меньше). Отметь в {c1}:{c2} «Просрочено» и «Скоро», чтобы я видел, кого сегодня дёргать». Полученные заказы не трогай."
        ),
        example,
        c1,
        vec![
            "Порядок проверок важен: сначала «получен?» - полученный заказ не опаздывает, даже если дата прошла. Потом «просрочен?», потом «скоро?».".into(),
            format!("Сегодняшняя дата в {today_cell} - в формуле со знаками $, чтобы не съехала при протягивании. «Осталось дней» = дата − сегодня."),
        ],
        80.0,
    ))
}

/// Who to call: the supplier's manager and phone from the directory.
pub fn contact(g: &mut Gen) -> Result<Task> {
    let n = g.rows();
    let t = g.table("Просроченные поставки", &["№ заказа", "Поставщик", "Ожидаем", "Менеджер", "Телефон"], n);
    g.setup.width(t.col(1), 210.0);
    // The directory sits to the right of the orders.
    let dc = t.right + 2;
    for (c, h) in ["Поставщик", "Менеджер", "Телефон"].iter().enumerate() {
        g.setup.text(a1(t.header, dc + c as u32), *h);
        g.setup.bold.push(a1(t.header, dc + c as u32));
        g.setup.width(dc + c as u32, if c == 0 { 210.0 } else { 150.0 });
    }
    let mut order: Vec<usize> = (0..SUPPLIERS.len()).collect();
    g.l.shuffle(&mut order);
    for (i, s) in order.iter().enumerate() {
        let (name, _, manager, phone) = SUPPLIERS.get(*s).copied().unwrap_or(SUPPLIERS[0]);
        let r = t.header + 1 + i as u32;
        g.setup.input(a1(r, dc), Cell::Text(name.into()));
        g.setup.input(a1(r, dc + 1), Cell::Text(manager.into()));
        g.setup.input(a1(r, dc + 2), Cell::Text(phone.into()));
    }
    let dir = format!("{}:{}", dollar(&a1(t.header + 1, dc)), dollar(&a1(t.header + SUPPLIERS.len() as u32, dc + 2)));
    // From level 4 the ERP export has a trailing space after one name.
    let spaced = (g.level >= 4).then(|| t.first + g.l.below(n as usize) as u32);
    let today = date(2026, 10, 14);
    let start = 3000 + g.l.int(0, 500);
    for (i, r) in t.rows().enumerate() {
        g.txt(&t, 0, r, &format!("PO-{}", start + i as i64));
        let s = supplier(g.d.below(SUPPLIERS.len()));
        let s = if Some(r) == spaced { format!("{s} ") } else { s.to_string() };
        g.txt(&t, 1, r, &s);
        let due = today - g.d.int(1, 10) as f64;
        g.day(&t, 2, r, due);
    }
    let key = |r: u32| if spaced.is_some() { format!("TRIM({})", t.cell(1, r)) } else { t.cell(1, r) };
    let manager = |r: u32| format!("=VLOOKUP({},{dir},2,FALSE)", key(r));
    let phone = |r: u32| format!("=VLOOKUP({},{dir},3,FALSE)", key(r));
    let e1 = answer_col(g, &t, 3, "Менеджер", &manager);
    let e2 = answer_col(g, &t, 4, "Телефон", &phone);
    let (m1, p2) = (t.cell(3, t.first), t.cell(4, t.last));
    let trim = if spaced.is_some() { " В выгрузке из ERP бывают лишние пробелы в названиях." } else { "" };
    Ok(finish(
        g,
        "Кому звонить",
        format!("Подтяни из справочника справа менеджера и телефон поставщика ({m1}:{p2}). Формулы в первую строку, потом протяни."),
        format!("ВПР по поставщику из справочника {dir}: менеджер - 2-й столбец, телефон - 3-й, точное совпадение (ЛОЖЬ)."),
        format!("Андрей: «По просрочкам нужно обзвонить поставщиков. Поставь рядом с каждым заказом, кому звонить и по какому номеру».{trim}"),
        format!("{e1}\n{e2}"),
        m1,
        vec![
            "Искомое - название поставщика, таблица - справочник со знаками $ (иначе при протягивании съедет), номер столбца - какой по счёту столбец справочника нужен, ЛОЖЬ - точное совпадение.".into(),
            "#Н/Д значит «не нашёл». Посмотри на название: лишний пробел в конце делает его другим текстом. СЖПРОБЕЛЫ(B2) убирает пробелы прямо внутри ВПР.".into(),
        ],
        90.0,
    ))
}

// ---------------------------------------------------------------- день 3

/// Invoices in USD/CNY/KZT to tenge by the rate table.
pub fn fx(g: &mut Gen) -> Result<Task> {
    let n = g.rows() + 1;
    let t = g.table("Счета поставщиков к оплате", &["№ счёта", "Поставщик", "Валюта", "Сумма", "В тенге"], n);
    g.setup.width(t.col(1), 210.0);
    let rc = t.right + 3;
    g.setup.text(a1(t.header, rc), "Валюта");
    g.setup.text(a1(t.header, rc + 1), "Курс, тг");
    g.setup.bold.push(a1(t.header, rc));
    g.setup.bold.push(a1(t.header, rc + 1));
    let usd = 470.0 + g.d.int(0, 60) as f64 + g.d.int(0, 99) as f64 / 100.0;
    let cny = 64.0 + g.d.int(0, 9) as f64 + g.d.int(0, 99) as f64 / 100.0;
    let mut rates = vec![("USD", usd), ("CNY", cny), ("KZT", 1.0)];
    g.l.shuffle(&mut rates);
    for (i, (c, v)) in rates.iter().enumerate() {
        g.setup.input(a1(t.header + 1 + i as u32, rc), Cell::Text((*c).into()));
        g.setup.input(a1(t.header + 1 + i as u32, rc + 1), Cell::Num(*v));
    }
    let table = format!("{}:{}", dollar(&a1(t.header + 1, rc)), dollar(&a1(t.header + 3, rc + 1)));
    let start = 500 + g.l.int(0, 300);
    for (i, r) in t.rows().enumerate() {
        g.txt(&t, 0, r, &format!("INV-{}", start + i as i64));
        let si = if i < 3 { [0usize, 1, 3][i] } else { g.d.below(SUPPLIERS.len()) };
        let (name, cur, ..) = SUPPLIERS.get(si).copied().unwrap_or(SUPPLIERS[0]);
        g.txt(&t, 1, r, name);
        g.txt(&t, 2, r, cur);
        let v = match cur {
            "USD" => g.d.amount(800, 30_000, 10),
            "CNY" => g.d.amount(5_000, 200_000, 100),
            _ => g.d.amount(50_000, 3_000_000, 1_000),
        };
        g.num(&t, 3, r, v);
    }
    let reference = |r: u32| format!("={}*VLOOKUP({},{table},2,FALSE)", t.cell(3, r), t.cell(2, r));
    let example = answer_col(g, &t, 4, "В тенге", &reference);
    for r in t.rows() {
        g.setup.formats.push((t.cell(4, r), "#,##0.00".into()));
    }
    let (c1, c2) = (t.cell(4, t.first), t.cell(4, t.last));
    Ok(finish(
        g,
        "Счета в разных валютах",
        format!("Переведи каждый счёт в тенге ({c1}:{c2}): сумма × курс его валюты из таблицы курсов справа."),
        format!("В {c1}:{c2}: сумма × ВПР(валюта; таблица курсов {table}; 2; ЛОЖЬ). У тенге курс 1."),
        format!("Бухгалтерия просит заявку на оплату в тенге. В {c1}:{c2} переведи каждый счёт по курсу из таблицы справа - так, чтобы при новом курсе всё пересчиталось само."),
        example,
        c1,
        vec![
            "Курс не вписывают числом в формулу: завтра он другой. Его берут ВПР из таблицы курсов по коду валюты.".into(),
            "Таблицу курсов закрепи знаками $ (F4), иначе при протягивании она съедет вниз и последние строки дадут #Н/Д.".into(),
        ],
        70.0,
    ))
}

/// What falls due this week, boundaries included.
pub fn payweek(g: &mut Gen) -> Result<Task> {
    let n = g.rows() + 3;
    let t = g.table("Счета к оплате, тг", &["№ счёта", "Поставщик", "Срок оплаты", "Сумма, тг"], n);
    g.setup.width(t.col(1), 210.0);
    let monday = date(2026, 10, 19) + 7.0 * g.l.int(0, 3) as f64;
    let c_from = g.param(&t, 1, "Неделя с:", Cell::Date(monday));
    let c_to = g.param(&t, 2, "по:", Cell::Date(monday + 6.0));
    let (from, to) = (dollar(&c_from), dollar(&c_to));
    let focus_sup = supplier(g.l.below(SUPPLIERS.len()));
    let mut kinds: Vec<usize> = (0..n as usize).map(|i| i % 5).collect();
    g.l.shuffle(&mut kinds);
    let start = 700 + g.l.int(0, 200);
    for (i, r) in t.rows().enumerate() {
        g.txt(&t, 0, r, &format!("INV-{}", start + i as i64));
        let s = if i % 3 == 0 { focus_sup } else { supplier(g.d.below(SUPPLIERS.len())) };
        g.txt(&t, 1, r, s);
        // Both boundaries appear: a learner with > instead of >= loses them.
        let due = match kinds.get(i).copied().unwrap_or(0) {
            0 => monday,
            1 => monday + 6.0,
            2 => monday + g.d.int(1, 5) as f64,
            3 => monday - g.d.int(1, 10) as f64,
            _ => monday + g.d.int(7, 20) as f64,
        };
        g.day(&t, 2, r, due);
        let v = g.d.amount(40_000, 4_000_000, 1_000);
        g.num(&t, 3, r, v);
    }
    let (due, amt, sup) = (t.abs(2), t.abs(3), t.abs(1));
    let a1c = g.answer(&t, 4, "К оплате на неделе, тг:");
    let a2c = g.answer(&t, 5, "Счетов на неделе:");
    let r1 = format!("=SUMIFS({amt},{due},\">=\"&{from},{due},\"<=\"&{to})");
    let r2 = format!("=COUNTIFS({due},\">=\"&{from},{due},\"<=\"&{to})");
    g.formula_check(&a1c, &r1);
    g.formula_check(&a2c, &r2);
    let mut example = format!("{a1c}: {}\n{a2c}: {}", ru(&r1), ru(&r2));
    if g.level >= 3 {
        let a3c = g.answer(&t, 6, "Из них поставщику:");
        let r3 = format!("=SUMIFS({amt},{due},\">=\"&{from},{due},\"<=\"&{to},{sup},\"{focus_sup}\")");
        g.formula_check(&a3c, &r3);
        example.push_str(&format!("\n{a3c}: {}", ru(&r3)));
    }
    let extra = if g.level >= 3 { format!(" И отдельно - сколько из этого уходит {focus_sup}.") } else { String::new() };
    Ok(finish(
        g,
        "Что платить на этой неделе",
        format!("Сумма счетов со сроком оплаты с {c_from} по {c_to} включительно - в {a1c}, их количество - в {a2c}."),
        format!("СУММЕСЛИМН с двумя условиями на срок: \">=\"&{c_from} и \"<=\"&{c_to}. Количество - СЧЁТЕСЛИМН с теми же условиями."),
        format!("Бухгалтерия: «Сколько денег нужно на оплату поставщикам на этой неделе ({c_from}–{c_to}) и сколько это счетов?»{extra}"),
        example,
        a1c,
        vec![
            "Условие с датой из ячейки пишут так: \">=\"&H2 - знак в кавычках, амперсанд склеивает его с датой.".into(),
            "Границы включаются: счёт со сроком ровно в понедельник или ровно в воскресенье - тоже эта неделя. Поэтому >= и <=, а не > и <.".into(),
        ],
        80.0,
    ))
}

/// Prepayment 30/70: how much to pay now and later.
pub fn prepay(g: &mut Gen) -> Result<Task> {
    let n = g.rows();
    let t = g.table("Новые заказы в Китай", &["№ заказа", "Поставщик", "Сумма, USD", "Предоплата", "Перед отгрузкой"], n);
    g.setup.width(t.col(1), 210.0);
    let pct = [0.3, 0.3, 0.5, 0.4][g.l.below(4)];
    let pc = g.param(&t, 1, "Предоплата:", Cell::Num(pct));
    g.setup.formats.push((pc.clone(), "0%".into()));
    let p = dollar(&pc);
    let start = 4000 + g.l.int(0, 500);
    for (i, r) in t.rows().enumerate() {
        g.txt(&t, 0, r, &format!("PO-{}", start + i as i64));
        let s = supplier(g.d.below(3));
        g.txt(&t, 1, r, s);
        let v = g.d.amount(1_000, 60_000, 1) + g.d.int(0, 99) as f64 / 100.0;
        g.num(&t, 2, r, v);
    }
    let pre = |r: u32| format!("=ROUND({}*{p},2)", t.cell(2, r));
    let rest = |r: u32| format!("={c}-ROUND({c}*{p},2)", c = t.cell(2, r));
    let e1 = answer_col(g, &t, 3, "Предоплата", &pre);
    let e2 = answer_col(g, &t, 4, "Перед отгрузкой", &rest);
    let (c1, c2) = (t.cell(3, t.first), t.cell(4, t.last));
    Ok(finish(
        g,
        "Предоплата и остаток",
        format!("Предоплата = сумма × процент из {pc}, округлить до центов. Остаток перед отгрузкой = сумма − предоплата. Заполни {c1}:{c2}."),
        format!("Предоплата: ОКРУГЛ(сумма × {pc}; 2), процент со знаками $. Остаток: сумма − предоплата."),
        format!("Китайские поставщики работают по схеме: {}% при заказе, остальное перед отгрузкой. Посчитай по каждому заказу, сколько платим сейчас и сколько потом ({c1}:{c2}).", (pct * 100.0) as i64),
        format!("{e1}\n{e2}"),
        t.cell(3, t.first),
        vec![
            "Процент предоплаты один на все заказы, он в отдельной ячейке - в формуле со знаками $. Если поставщик поменяет условия, правишь одну ячейку.".into(),
            "Деньги округляют до центов: ОКРУГЛ(…; 2). Остаток считают от уже округлённой предоплаты, тогда части в сумме дают ровно весь заказ.".into(),
        ],
        70.0,
    ))
}

// ---------------------------------------------------------------- день 4

/// Order vs invoice vs delivery note, any method; graded row by row.
pub fn reconcile(g: &mut Gen) -> Result<Task> {
    let level = g.level;
    let count = match level {
        1 => 4,
        2 => 5,
        3 => 6,
        _ => 8,
    };
    let mut idx: Vec<usize> = (0..PARTS.len()).collect();
    g.l.shuffle(&mut idx);
    idx.truncate(count);
    // Which line carries which problem (layout seed): 0 ok, 1 short delivery, 2 price, 3 not ordered.
    let mut kinds: Vec<u8> = (0..count).map(|i| if i == 0 { 1 } else { 0 }).collect();
    if level >= 2
        && let Some(k) = kinds.get_mut(1)
    {
        *k = 2;
    }
    if level >= 3
        && let Some(k) = kinds.get_mut(2)
    {
        *k = 3;
    }
    if level >= 4
        && let Some(k) = kinds.get_mut(3)
    {
        *k = 1;
    }
    g.l.shuffle(&mut kinds);
    let sheet = "Сверка";
    g.setup.first_sheet = Some(sheet.into());
    let headers = ["Артикул", "Заказано", "В счёте", "Пришло", "Итог"];
    g.setup.header(1, 0, &headers);
    g.setup.width(0, 120.0);
    for c in 1..5 {
        g.setup.width(c, 110.0);
    }
    let q = |s: &str, r: u32, c: u32| format!("'{s}'!{}", a1(r, c));
    for (c, h) in ["Артикул", "Наименование", "Кол-во", "Цена"].iter().enumerate() {
        g.setup.text(q("Заказ", 1, c as u32), *h);
    }
    for (c, h) in ["Артикул", "Кол-во", "Цена"].iter().enumerate() {
        g.setup.text(q("Счёт", 1, c as u32), *h);
    }
    for (c, h) in ["Артикул", "Принято складом"].iter().enumerate() {
        g.setup.text(q("Накладная", 1, c as u32), *h);
    }
    for s in ["Заказ", "Счёт", "Накладная"] {
        g.setup.bold.push(format!("'{s}'!A1:D1"));
        g.setup.widths.push((format!("'{s}'!A:B"), 170.0));
    }
    let (mut ro, mut ri, mut rn) = (2u32, 2u32, 2u32);
    let mut rows = Vec::new();
    for (i, pi) in idx.iter().enumerate() {
        let (art, name, _, _, price) = PARTS.get(*pi).copied().unwrap_or(PARTS[0]);
        let kind = kinds.get(i).copied().unwrap_or(0);
        let qty = g.d.amount(100, 5_000, 50);
        let short = g.d.amount(50, 400, 10).min(qty - 50.0);
        let (ordered, invoiced, came, verdict) = match kind {
            1 => (qty, qty, qty - short, "недовоз"),
            2 => (qty, qty, qty, "цена"),
            3 => (0.0, qty, qty, "нет в заказе"),
            _ => (qty, qty, qty, "ок"),
        };
        let inv_price = if kind == 2 { price * 1.08 } else { price };
        if kind != 3 {
            g.setup.input(q("Заказ", ro, 0), Cell::Text(art.into()));
            g.setup.input(q("Заказ", ro, 1), Cell::Text(name.into()));
            g.setup.input(q("Заказ", ro, 2), Cell::Num(ordered));
            g.setup.input(q("Заказ", ro, 3), Cell::Num(price));
            ro += 1;
        }
        g.setup.input(q("Счёт", ri, 0), Cell::Text(art.into()));
        g.setup.input(q("Счёт", ri, 1), Cell::Num(invoiced));
        g.setup.input(q("Счёт", ri, 2), Cell::Num((inv_price * 100.0).round() / 100.0));
        ri += 1;
        g.setup.input(q("Накладная", rn, 0), Cell::Text(art.into()));
        g.setup.input(q("Накладная", rn, 1), Cell::Num(came));
        rn += 1;
        g.setup.input(a1(2 + i as u32, 0), Cell::Text(art.into()));
        rows.push(vec![
            Expectation::Text(art.into()),
            Expectation::Number(ordered),
            Expectation::Number(invoiced),
            Expectation::Number(came),
            Expectation::Text(verdict.into()),
        ]);
    }
    g.check(
        "Сверка",
        Kind::Table {
            first: 2,
            left: 0,
            headers: headers.iter().map(|h| h.to_string()).collect(),
            cols: vec![Col::Given, Col::Number, Col::Number, Col::Number, Col::Text],
            rows,
            key: 0,
            notes: Vec::new(),
        },
    );
    let verdicts = match level {
        1 => "«ок» или «недовоз»",
        2 => "«ок», «недовоз» или «цена» (цена в счёте не как в заказе)",
        _ => "«ок», «недовоз», «цена» или «нет в заказе» (поставщик выставил то, что мы не заказывали; «Заказано» тогда 0)",
    };
    let prompt = match level {
        1 | 2 => format!(
            "Пришла партия. Сверь три документа на листах «Заказ», «Счёт», «Накладная»: по каждому артикулу на листе «Сверка» проставь количества и итог: {verdicts}. Удобно через ВПР с другого листа."
        ),
        _ => format!(
            "Андрей: «Склад принял партию, а бухгалтерия хочет оплатить счёт. Сверь заказ, счёт и накладную и скажи, где расхождения, - до оплаты». В «Итог»: {verdicts}."
        ),
    };
    let mut task = g.task("Сверка заказа, счёта и накладной", prompt, None, "B2".into());
    task.hints = vec![
        "Количества с других листов: =ВПР(A2;'Заказ'!$A$2:$D$20;3;ЛОЖЬ). Если артикула в заказе нет, ВПР даст #Н/Д - оберни в ЕСЛИОШИБКА(…;0).".into(),
        "Цена: сравни цену в счёте с ценой в заказе тем же ВПР (4-й столбец заказа, 3-й столбец счёта).".into(),
        "Итог можно вписать руками или формулой: =ЕСЛИ(B2=0;\"нет в заказе\";ЕСЛИ(цена_счёта<>цена_заказа;\"цена\";ЕСЛИ(D2<B2;\"недовоз\";\"ок\"))).".into(),
    ];
    task.goal_seconds = 180.0 + 60.0 * f32::from(level);
    Ok(task)
}

/// Incoming inspection: defect rate against a threshold.
pub fn defects(g: &mut Gen) -> Result<Task> {
    let n = g.rows();
    let t = g.table("Входной контроль партий", &["Партия", "Компонент", "Получено, шт", "Брак, шт", "Цена, тг", "Доля брака", "Решение"], n);
    g.setup.width(t.col(1), 190.0);
    let thr = [0.01, 0.01, 0.02][g.l.below(3)];
    let tc = g.param(&t, 1, "Допустимый брак:", Cell::Num(thr));
    g.setup.formats.push((tc.clone(), "0%".into()));
    let th = dollar(&tc);
    let bad = t.first + g.l.below(n as usize) as u32;
    for (i, r) in t.rows().enumerate() {
        g.txt(&t, 0, r, &format!("B-{}", 310 + i));
        let (_, name, _, _, price) = PARTS.get(g.d.below(PARTS.len())).copied().unwrap_or(PARTS[0]);
        g.txt(&t, 1, r, name);
        let got = g.d.amount(500, 6_000, 100);
        let rate = if r == bad { thr * 3.0 } else { g.d.int(0, 30) as f64 / 1000.0 };
        g.num(&t, 2, r, got);
        g.num(&t, 3, r, (got * rate).round());
        g.num(&t, 4, r, (price * 500.0).round());
    }
    let share = |r: u32| format!("={}/{}", t.cell(3, r), t.cell(2, r));
    let verdict = |r: u32| format!("=IF({}/{}>{th},\"Претензия\",\"Принять\")", t.cell(3, r), t.cell(2, r));
    let e1 = answer_col(g, &t, 5, "Доля брака", &share);
    let e2 = answer_col(g, &t, 6, "Решение", &verdict);
    for r in t.rows() {
        g.setup.formats.push((t.cell(5, r), "0.0%".into()));
    }
    let mut example = format!("{e1}\n{e2}");
    if g.level >= 2 {
        let ac = g.answer(&t, 3, "Сумма претензий, тг:");
        let (d, p, gt) = (t.abs(3), t.abs(4), t.abs(2));
        let r3 = format!("=SUMPRODUCT(({d}/{gt}>{th})*{d}*{p})");
        g.formula_check(&ac, &r3);
        example.push_str(&format!("\n{ac}: {}", ru(&r3)));
    }
    let (c1, c2) = (t.cell(5, t.first), t.cell(6, t.last));
    Ok(finish(
        g,
        "Входной контроль",
        format!("Доля брака = брак / получено. Если она больше допустимой ({tc}) - «Претензия», иначе «Принять». Заполни {c1}:{c2}."),
        format!("Доля = брак / получено (формат %). Решение: ЕСЛИ(доля > {tc}; \"Претензия\"; \"Принять\"). Сумма претензий - бракованные штуки × цена по партиям с претензией."),
        format!("ОТК прислал результаты входного контроля. Отметь, по каким партиям выставляем поставщику претензию (брак выше {}%), и на какую сумму.", (thr * 100.0) as i64),
        example,
        t.cell(5, t.first),
        vec![
            "Доля брака - это деление, а не разница: 15 бракованных из 1500 - это 1%.".into(),
            "Сумма претензий по всем партиям одной формулой: СУММПРОИЗВ((доля>порог)*брак*цена). Можно и столбцом «брак × цена, если претензия» плюс СУММ.".into(),
        ],
        80.0,
    ))
}

// ---------------------------------------------------------------- день 5

/// Need for three models from the BOM, minus stock and confirmed transit.
pub fn mrp(g: &mut Gen) -> Result<Task> {
    let level = g.level;
    let n = match level {
        1 => 5,
        2 => 7,
        _ => BOM.len() as u32,
    };
    let unconfirmed = level >= 4;
    let mut headers = vec!["Компонент", "LA-IP", "TWM", "GMZ", "На складе", "В пути", "Потребность", "К заказу"];
    if unconfirmed {
        headers = vec!["Компонент", "LA-IP", "TWM", "GMZ", "На складе", "В пути (подтв.)", "Обещают (не подтв.)", "Потребность", "К заказу"];
    }
    let t = g.table("Спецификация на 1 изделие и запасы", &headers, n);
    g.setup.width(t.col(0), 200.0);
    // Parameters are not probe inputs: they come from the layout seed.
    let plans = [g.l.amount(2_000, 5_000, 100), g.l.amount(3_000, 6_000, 100), g.l.amount(500, 1_500, 100)];
    let pc: Vec<String> = ["План LA-IP, шт:", "План TWM, шт:", "План GMZ, шт:"]
        .iter()
        .zip(plans)
        .enumerate()
        .map(|(i, (l, v))| g.param(&t, 1 + i as u32, l, Cell::Num(v)))
        .collect();
    let p: Vec<String> = pc.iter().map(|c| dollar(c)).collect();
    let covered = t.first + g.l.below(n as usize) as u32;
    for (i, r) in t.rows().enumerate() {
        let (name, norms) = BOM.get(i).copied().unwrap_or(BOM[0]);
        g.txt(&t, 0, r, name);
        for (k, v) in norms.iter().enumerate() {
            g.num(&t, 1 + k as u32, r, *v);
        }
        let need: f64 = norms.iter().zip(plans).map(|(a, b)| a * b).sum();
        let stock = if r == covered { need + g.d.amount(100, 900, 50) } else { g.d.amount(0, (need * 0.4) as i64, 50) };
        g.num(&t, 4, r, stock);
        let transit = g.d.amount(0, (need * 0.3) as i64, 50);
        g.num(&t, 5, r, transit);
        if unconfirmed {
            let promised = g.d.amount(0, (need * 0.3) as i64, 50);
            g.num(&t, 6, r, promised);
        }
    }
    let base = if unconfirmed { 7 } else { 6 };
    let need = |r: u32| format!("{}*{}+{}*{}+{}*{}", t.cell(1, r), p[0], t.cell(2, r), p[1], t.cell(3, r), p[2]);
    let need_ref = |r: u32| format!("={}", need(r));
    let order_ref = |r: u32| format!("=MAX(0,{}-{}-{})", need(r), t.cell(4, r), t.cell(5, r));
    let e1 = answer_col(g, &t, base, "Потребность", &need_ref);
    let e2 = answer_col(g, &t, base + 1, "К заказу", &order_ref);
    let shown2 = format!("{}: =МАКС(0;{}-{}-{})", t.cell(base + 1, t.first), t.cell(base, t.first), t.cell(4, t.first), t.cell(5, t.first));
    let _ = e2;
    let (c1, c2) = (t.cell(base, t.first), t.cell(base + 1, t.last));
    let trap = if unconfirmed { " «Обещают (не подтв.)» не вычитай: на устное «отправим» производство не запустишь." } else { "" };
    Ok(finish(
        g,
        "Сколько заказать под план",
        format!("Потребность = норма LA-IP × план LA-IP + норма TWM × план TWM + норма GMZ × план GMZ. К заказу = МАКС(0; потребность − склад − в пути). Заполни {c1}:{c2}."),
        format!("Потребность: сумма произведений нормы на план трёх моделей (планы - {}, {}, {}, со знаками $). К заказу: МАКС(0; потребность − склад − в пути).", pc[0], pc[1], pc[2]),
        format!("Производство дало план выпуска на месяц (справа). Андрей: «Посчитай по спецификации, сколько каждого компонента нужно на весь план и сколько докупить с учётом склада и того, что уже едет».{trap}"),
        format!("{e1}\n{shown2}"),
        c1,
        vec![
            "Спецификация - сколько штук компонента идёт в одно изделие каждой модели. Батарей на газовый модем GMZ две, кабеля на LA-IP - 3 метра, гофрокороба - 0,05 (одна коробка на 20 изделий).".into(),
            "Если на складе и в пути больше, чем нужно, разность отрицательная, а заказать минус нельзя - поэтому МАКС(0; …).".into(),
        ],
        120.0,
    ))
}

/// Pack multiples and minimum order quantity.
pub fn moq(g: &mut Gen) -> Result<Task> {
    let level = g.level;
    let n = g.rows();
    let with_moq = level >= 2;
    let mut headers = vec!["Компонент", "Нужно, шт", "В упаковке, шт"];
    if with_moq {
        headers.push("Мин. партия, шт");
    }
    headers.extend(["Заказать, шт", "Излишек, шт"]);
    let t = g.table("Заказ с учётом упаковки", &headers, n);
    g.setup.width(t.col(0), 200.0);
    let exact = t.first + g.l.below(n as usize) as u32;
    let small = t.first + ((exact - t.first + 1) % n);
    for (i, r) in t.rows().enumerate() {
        let (_, name, ..) = PARTS.get((i * 5 + 3) % PARTS.len()).copied().unwrap_or(PARTS[0]);
        g.txt(&t, 0, r, name);
        let pack = [50.0, 100.0, 200.0, 500.0, 1000.0][g.d.below(5)];
        let need = if r == exact { pack * g.d.int(2, 9) as f64 } else { g.d.amount(120, 9_000, 10) };
        let need = if r == small && with_moq { (pack * 0.6).round() } else { need };
        g.num(&t, 1, r, need);
        g.num(&t, 2, r, pack);
        if with_moq {
            let m = if r == small { pack * 4.0 } else { pack * g.d.int(1, 3) as f64 };
            g.num(&t, 3, r, m);
        }
    }
    let base = if with_moq { 4 } else { 3 };
    let order = |r: u32| {
        let up = format!("ROUNDUP({}/{},0)*{}", t.cell(1, r), t.cell(2, r), t.cell(2, r));
        if with_moq { format!("MAX({},{up})", t.cell(3, r)) } else { up }
    };
    let order_ref = |r: u32| format!("={}", order(r));
    let extra_ref = |r: u32| format!("={}-{}", order(r), t.cell(1, r));
    let e1 = answer_col(g, &t, base, "Заказать, шт", &order_ref);
    let _ = answer_col(g, &t, base + 1, "Излишек, шт", &extra_ref);
    let e2 = format!("{}: ={}-{}", t.cell(base + 1, t.first), t.cell(base, t.first), t.cell(1, t.first));
    let (c1, c2) = (t.cell(base, t.first), t.cell(base + 1, t.last));
    let moq_text = if with_moq { " и не меньше минимальной партии" } else { "" };
    Ok(finish(
        g,
        "Упаковка и минимальная партия",
        format!("Заказывают целыми упаковками: ОКРУГЛВВЕРХ(нужно / в упаковке; 0) × в упаковке{moq_text}. Излишек = заказ − нужно. Заполни {c1}:{c2}."),
        format!("Заказ: целые упаковки - ОКРУГЛВВЕРХ(нужно/упаковка; 0)×упаковка{}. Излишек = заказ − нужно.", if with_moq { ", и МАКС с минимальной партией" } else { "" }),
        format!("Поставщики продают только упаковками{moq_text}. Посчитай, сколько реально заказываем ({c1}:{c2}) и сколько получится лишнего - Андрей спросит, откуда на складе излишки."),
        format!("{e1}\n{e2}"),
        c1,
        vec![
            "Нужно 340 штук, в упаковке 100 → 340/100 = 3,4 упаковки → округляем ВВЕРХ до 4 → заказываем 400. Округлять вниз нельзя: не хватит на производство.".into(),
            "Минимальная партия: если поставщик не отгружает меньше 2000, а нужно 400 - заказ 2000. Это МАКС(мин. партия; округлённое количество).".into(),
        ],
        80.0,
    ))
}

/// Reorder date: days of cover, lead time and safety stock.
pub fn reorder(g: &mut Gen) -> Result<Task> {
    let n = g.rows();
    let t = g.table("Запасы на складе", &["Компонент", "Остаток, шт", "Расход в день, шт", "Срок поставки, дн", "Страховой запас, дн", "Хватит на, дн", "Заказать до", "Статус"], n);
    g.setup.width(t.col(0), 200.0);
    let today = date(2026, 10, 20) + g.l.int(0, 10) as f64;
    let tc = g.param(&t, 1, "Сегодня:", Cell::Date(today));
    let td = dollar(&tc);
    let urgent = t.first + g.l.below(n as usize) as u32;
    for (i, r) in t.rows().enumerate() {
        let (_, name, _, si, _) = PARTS.get((i * 7 + 1) % PARTS.len()).copied().unwrap_or(PARTS[0]);
        g.txt(&t, 0, r, name);
        let use_ = g.d.amount(50, 600, 10);
        let lead = if si <= 2 { g.d.int(35, 50) } else { g.d.int(3, 10) } as f64;
        let safety = g.d.int(5, 15) as f64;
        let days = if r == urgent { lead + safety - g.d.int(1, 10) as f64 } else { lead + safety + g.d.int(-15, 40) as f64 };
        let stock = (days.max(1.0) * use_ + g.d.int(0, 9) as f64).round();
        g.num(&t, 1, r, stock);
        g.num(&t, 2, r, use_);
        g.num(&t, 3, r, lead);
        g.num(&t, 4, r, safety);
    }
    let cover = |r: u32| format!("INT({}/{})", t.cell(1, r), t.cell(2, r));
    let by = |r: u32| format!("{td}+{}-{}-{}", cover(r), t.cell(3, r), t.cell(4, r));
    let r1 = |r: u32| format!("={}", cover(r));
    let r2 = |r: u32| format!("={}", by(r));
    let r3 = |r: u32| format!("=IF({}<={td},\"Заказать сейчас\",\"ОК\")", by(r));
    let e1 = answer_col(g, &t, 5, "Хватит на, дн", &r1);
    let _ = answer_col(g, &t, 6, "Заказать до", &r2);
    let _ = answer_col(g, &t, 7, "Статус", &r3);
    for r in t.rows() {
        g.setup.formats.push((t.cell(6, r), "dd.mm.yyyy".into()));
    }
    let (f, l) = (t.first, t.last);
    let e2 = format!("{}: ={tc}+{}-{}-{} (с $ у {tc})", t.cell(6, f), t.cell(5, f), t.cell(3, f), t.cell(4, f));
    let e3 = format!("{}: =ЕСЛИ({}<={tc};\"Заказать сейчас\";\"ОК\")", t.cell(7, f), t.cell(6, f));
    let (c1, c2) = (t.cell(5, f), t.cell(7, l));
    Ok(finish(
        g,
        "Когда заказывать",
        format!("Хватит на = ЦЕЛОЕ(остаток / расход). Заказать до = сегодня + хватит − срок поставки − страховой запас. Статус: если эта дата уже наступила - «Заказать сейчас», иначе «ОК». Заполни {c1}:{c2}."),
        format!("Дней хватит: ЦЕЛОЕ(остаток/расход). Последний день заказа: сегодня ({tc}) + дней хватит − срок поставки − страховой запас. Статус через ЕСЛИ."),
        format!("Андрей: «Посмотри по складу, что пора заказывать уже сейчас, чтобы линия не встала. Китай везёт по 40+ дней». Посчитай, на сколько дней хватит, последний день заказа и статус ({c1}:{c2})."),
        format!("{e1}\n{e2}\n{e3}"),
        c1,
        vec![
            "Логика: запаса хватит на N дней. Поставка идёт L дней, и хотим ещё S дней запаса на случай задержки. Значит заказывать надо не позже, чем через N − L − S дней от сегодня.".into(),
            "Если последний день заказа сегодня или уже прошёл - «Заказать сейчас». Для китайских поставщиков учитывай ещё праздники: на китайский Новый год фабрики стоят 2–3 недели.".into(),
        ],
        110.0,
    ))
}

// ---------------------------------------------------------------- день 6

/// Landed cost: tenge value, freight shared by value, duty and import VAT, per unit.
pub fn landed(g: &mut Gen) -> Result<Task> {
    let n = (g.rows() - 1).clamp(4, 8);
    let t = g.table("Партия из Китая", &["Позиция", "Цена, USD", "Кол-во, шт", "Стоимость, тг", "Доставка, тг", "С пошлиной и НДС, тг", "За штуку, тг"], n);
    g.setup.width(t.col(0), 200.0);
    let rate = 480.0 + g.l.int(0, 50) as f64;
    let freight = g.l.amount(300_000, 1_500_000, 10_000);
    let duty = [0.0, 0.05, 0.1][g.l.below(3)];
    let vat = [0.12, 0.16][g.l.below(2)];
    let rc = g.param(&t, 1, "Курс USD, тг:", Cell::Num(rate));
    let fc = g.param(&t, 2, "Доставка за партию, тг:", Cell::Num(freight));
    let dc = g.param(&t, 3, "Пошлина:", Cell::Num(duty));
    let vc = g.param(&t, 4, "НДС на импорт:", Cell::Num(vat));
    g.setup.formats.push((dc.clone(), "0%".into()));
    g.setup.formats.push((vc.clone(), "0%".into()));
    let (rt, fr, du, va) = (dollar(&rc), dollar(&fc), dollar(&dc), dollar(&vc));
    for (i, r) in t.rows().enumerate() {
        let (_, name, _, si, price) = PARTS.get(i % 9).copied().unwrap_or(PARTS[0]);
        let _ = si;
        g.txt(&t, 0, r, name);
        let usd = if price < 50.0 { price } else { 1.0 };
        let price = (usd * (90.0 + g.d.int(0, 20) as f64)).round() / 100.0;
        let qty = g.d.amount(500, 10_000, 100);
        g.num(&t, 1, r, price);
        g.num(&t, 2, r, qty);
    }
    let (pr, qt) = (t.abs(1), t.abs(2));
    let tg = |r: u32| format!("{}*{}*{rt}", t.cell(1, r), t.cell(2, r));
    let share = |r: u32| format!("{}*{}/SUMPRODUCT({pr},{qt})*{fr}", t.cell(1, r), t.cell(2, r));
    let total = |r: u32| format!("({}+{})*(1+{du})*(1+{va})", tg(r), share(r));
    let r1 = |r: u32| format!("={}", tg(r));
    let r2 = |r: u32| format!("={}", share(r));
    let r3 = |r: u32| format!("={}", total(r));
    let r4 = |r: u32| format!("={}/{}", total(r), t.cell(2, r));
    let e1 = answer_col(g, &t, 3, "Стоимость, тг", &r1);
    let _ = answer_col(g, &t, 4, "Доставка, тг", &r2);
    let _ = answer_col(g, &t, 5, "С пошлиной и НДС, тг", &r3);
    let _ = answer_col(g, &t, 6, "За штуку, тг", &r4);
    for c in 3..=6 {
        for r in t.rows() {
            g.setup.formats.push((t.cell(c, r), "#,##0.00".into()));
        }
    }
    let f = t.first;
    let d_all = t.abs(3);
    let example = format!(
        "{e1}\n{}: ={}/СУММ({d_all})*{fc}\n{}: =({}+{})*(1+{du})*(1+{va})\n{}: ={}/{}",
        t.cell(4, f),
        t.cell(3, f),
        t.cell(5, f),
        t.cell(3, f),
        t.cell(4, f),
        t.cell(6, f),
        t.cell(5, f),
        t.cell(2, f)
    );
    let (c1, c2) = (t.cell(3, f), t.cell(6, t.last));
    Ok(finish(
        g,
        "Полная цена до склада",
        format!("Стоимость в тенге = цена × кол-во × курс. Доставку делим по доле стоимости. Потом + пошлина и + НДС на импорт, и делим на штуки. Заполни {c1}:{c2} по образцу."),
        format!("Стоимость тг = цена × кол-во × {rc}. Доставка = стоимость / СУММ(стоимостей) × {fc}. Итого = (стоимость + доставка) × (1 + {dc}) × (1 + {vc}). За штуку = итого / кол-во. Все параметры - со знаками $."),
        format!("Андрей: «Посчитай, во сколько нам реально обходится каждая позиция из этой партии, уже на нашем складе: с доставкой, пошлиной и НДС». Ставки и курс - в ячейках справа, они меняются, поэтому только ссылками ({c1}:{c2})."),
        example,
        c1,
        vec![
            "Цена в инвойсе - ещё не цена для нас. Сверху: доставка, пошлина (процент от стоимости с доставкой) и НДС на импорт (процент от всего, включая пошлину). Поэтому (1 + пошлина) × (1 + НДС).".into(),
            "Доставку за всю партию распределяют по позициям пропорционально стоимости: дорогие позиции несут большую долю.".into(),
            "Ставки и курс не вписывай числами в формулы: если поменяется курс, пересчитывается вся таблица одной правкой.".into(),
        ],
        180.0,
    ))
}

/// Three offers for the same battery: unit cost on our shelf.
pub fn compare(g: &mut Gen) -> Result<Task> {
    let t = g.table("Предложения по батарее ER14505", &["Предложение", "Цена за шт", "Курс к тг", "Доставка за партию, тг", "Пошлина", "На складе за шт, тг"], 3);
    g.setup.width(t.col(0), 230.0);
    let qty = g.l.amount(5_000, 20_000, 1_000);
    let qc = g.param(&t, 1, "Нужно, шт:", Cell::Num(qty));
    let q = dollar(&qc);
    let cny = 65.0 + g.d.int(0, 8) as f64;
    let offers = [
        ("Китай EXW", 9.0 + g.d.int(0, 10) as f64 / 10.0, cny, g.d.amount(900_000, 1_600_000, 10_000), 0.05),
        ("Китай DAP", 10.5 + g.d.int(0, 10) as f64 / 10.0, cny, 0.0, 0.05),
        ("Местный", 700.0 + g.d.int(0, 120) as f64, 1.0, g.d.amount(0, 30_000, 5_000), 0.0),
    ];
    let mut best = ("", f64::MAX);
    for (i, r) in t.rows().enumerate() {
        let (name, price, rate, del, duty) = offers.get(i).copied().unwrap_or(offers[0]);
        g.txt(&t, 0, r, name);
        g.num(&t, 1, r, price);
        g.num(&t, 2, r, rate);
        g.num(&t, 3, r, del);
        g.num(&t, 4, r, duty);
        g.setup.formats.push((t.cell(4, r), "0%".into()));
        let unit = (price * rate * qty + del) * (1.0 + duty) / qty;
        if unit < best.1 {
            best = (name, unit);
        }
    }
    let unit = |r: u32| format!("=({}*{}*{q}+{})*(1+{})/{q}", t.cell(1, r), t.cell(2, r), t.cell(3, r), t.cell(4, r));
    let e1 = answer_col(g, &t, 5, "На складе за шт, тг", &unit);
    for r in t.rows() {
        g.setup.formats.push((t.cell(5, r), "#,##0.00".into()));
    }
    let ac = g.answer(&t, 3, "Выгоднее всего:");
    g.setup.width(t.right + 3, 260.0);
    g.check("Выбор", Kind::Value { cell: ac.clone(), expected: Cell::Text(best.0.into()) });
    let (c1, c2) = (t.cell(5, t.first), t.cell(5, t.last));
    Ok(finish(
        g,
        "Какое предложение выгоднее",
        format!("Цена на нашем складе за штуку = (цена × курс × нужно + доставка) × (1 + пошлина) / нужно. Заполни {c1}:{c2}, в {ac} впиши название самого выгодного предложения (как в столбце «Предложение»)."),
        format!("Для каждого предложения в {c1}:{c2}: (цена×курс×{qc} + доставка) × (1+пошлина) / {qc}. Самое выгодное - где меньше; название в {ac}."),
        format!("Андрей: «Три варианта по батареям: Wuhan PowerCell на условиях EXW Ухань (забираем и везём сами), тот же Wuhan на DAP Алматы (довозят до нас), и ТОО КазРадиоКомплект со своего склада. Китай дешевле на бумаге. Сравни, во что обойдётся штука уже у нас на складе, и скажи, какой вариант выгоднее» - в {ac}. Сроки и предоплату оценим отдельно."),
        format!("{e1}\n{ac}: название с минимальной ценой"),
        c1,
        vec![
            "EXW - забираем со склада продавца сами, доставка и таможня на нас. DAP - поставщик довозит до Алматы, растаможка на нас. У местного поставщика всё уже включено.".into(),
            "Сравнивают не цену в прайсе, а полную цену штуки на своём складе. И даже тогда решение не только про деньги: срок, предоплата, надёжность - это скажешь Андрею словами.".into(),
        ],
        120.0,
    ))
}

// ---------------------------------------------------------------- день 7

/// ABC: shares, running share and the class, on a list already sorted by amount.
pub fn abc(g: &mut Gen) -> Result<Task> {
    let n = (g.rows() + 3).min(PARTS.len() as u32);
    let t = g.table("Закупки за год по позициям (по убыванию)", &["Позиция", "Поставщик", "Сумма за год, тг", "Доля", "Накопленная доля", "Класс"], n);
    g.setup.width(t.col(0), 200.0);
    g.setup.width(t.col(1), 200.0);
    // A Pareto-like list: a few positions hold most of the money.
    let mut amounts: Vec<f64> = (0..n).map(|i| (60_000_000.0 * 0.62f64.powi(i as i32) * (0.9 + g.d.int(0, 20) as f64 / 100.0) / 1000.0).round() * 1000.0).collect();
    amounts.sort_by(|a, b| b.total_cmp(a));
    let mut order: Vec<usize> = vec![0, 5, 1, 6, 2, 7, 3, 9, 4, 8, 10, 11];
    order.truncate(n as usize);
    for (i, r) in t.rows().enumerate() {
        let (_, name, _, si, _) = PARTS.get(order.get(i).copied().unwrap_or(0)).copied().unwrap_or(PARTS[0]);
        g.txt(&t, 0, r, name);
        g.txt(&t, 1, r, supplier(si));
        g.num(&t, 2, r, amounts.get(i).copied().unwrap_or(1000.0));
    }
    let all = t.abs(2);
    let top = dollar(&t.cell(2, t.first));
    let share = |r: u32| format!("={}/SUM({all})", t.cell(2, r));
    let run = |r: u32| format!("SUM({top}:{})/SUM({all})", t.cell(2, r));
    let run_ref = |r: u32| format!("={}", run(r));
    let class = |r: u32| format!("=IF({}<=0.8,\"A\",IF({}<=0.95,\"B\",\"C\"))", run(r), run(r));
    let e1 = answer_col(g, &t, 3, "Доля", &share);
    let e2 = answer_col(g, &t, 4, "Накопленная доля", &run_ref);
    let _ = answer_col(g, &t, 5, "Класс", &class);
    for r in t.rows() {
        g.setup.formats.push((t.cell(3, r), "0.0%".into()));
        g.setup.formats.push((t.cell(4, r), "0.0%".into()));
    }
    let total: f64 = amounts.iter().sum();
    let mut acc = 0.0;
    let a_count = amounts
        .iter()
        .filter(|a| {
            acc += *a;
            acc / total <= 0.8
        })
        .count();
    let ac = g.answer(&t, 1, "Позиций в группе A:");
    g.check("Сколько в A", Kind::Value { cell: ac.clone(), expected: Cell::Num(a_count as f64) });
    let f = t.first;
    let e3 = format!("{}: =ЕСЛИ({}<=0,8;\"A\";ЕСЛИ({}<=0,95;\"B\";\"C\"))", t.cell(5, f), t.cell(4, f), t.cell(4, f));
    let (c1, c2) = (t.cell(3, f), t.cell(5, t.last));
    Ok(finish(
        g,
        "ABC-анализ закупок",
        format!("Доля = сумма / сумма всех. Накопленная доля = СУММ от первой строки до текущей / сумма всех. Класс: до 80% - A, до 95% - B, остальное - C. Заполни {c1}:{c2} и посчитай, сколько позиций в A ({ac})."),
        format!("Доля: сумма/СУММ(всех со $). Накопленная: СУММ($C$2:C2)/СУММ(всех) - начало диапазона закреплено, конец растёт. Класс - вложенные ЕСЛИ по накопленной доле: ≤80% A, ≤95% B, иначе C."),
        format!("Андрей: «Хочу понимать, на чём держатся наши деньги в закупках. Сделай ABC: доли, накопленная доля и класс (A до 80%, B до 95%, C остальное)». Список уже отсортирован по убыванию. Сколько позиций в A - в {ac}."),
        format!("{e1}\n{e2}\n{e3}"),
        c1,
        vec![
            "ABC - правило Парето: обычно 20% позиций держат 80% денег. Группа A - то, за чем следят каждый день: цены, сроки, запасной поставщик.".into(),
            "Накопленная доля: =СУММ($C$2:C2)/СУММ($C$2:$C$13). Первая ссылка закреплена, вторая при протягивании растёт - так в каждой строке сумма «от верха до меня».".into(),
            "В жизни список сначала сортируют по убыванию суммы: Данные → Сортировка, строки целиком.".into(),
        ],
        150.0,
    ))
}

/// Orders + catalogue: bring the category and supplier over by article, then a pivot.
pub fn join(g: &mut Gen) -> Result<Task> {
    use gridcraft_engine::model::PivotFunc;
    let level = g.level;
    let n = match level {
        1 => 6,
        2 => 8,
        3 => 10,
        _ => 12,
    };
    let sheet = "Заказы";
    g.setup.first_sheet = Some(sheet.into());
    let headers = ["№ заказа", "Артикул", "Кол-во", "Сумма, тг", "Категория", "Поставщик"];
    g.setup.header(1, 0, &headers);
    for (c, w) in [110.0, 110.0, 80.0, 110.0, 160.0, 210.0].iter().enumerate() {
        g.setup.width(c as u32, *w);
    }
    let cat = "Каталог";
    for (c, h) in ["Артикул", "Наименование", "Категория", "Поставщик"].iter().enumerate() {
        g.setup.text(format!("'{cat}'!{}", a1(1, c as u32)), *h);
    }
    g.setup.bold.push(format!("'{cat}'!A1:D1"));
    g.setup.widths.push((format!("'{cat}'!A:D"), 200.0));
    let mut idx: Vec<usize> = (0..PARTS.len()).collect();
    g.l.shuffle(&mut idx);
    for (i, pi) in idx.iter().enumerate() {
        let (art, name, category, si, _) = PARTS.get(*pi).copied().unwrap_or(PARTS[0]);
        let r = 2 + i as u32;
        g.setup.input(format!("'{cat}'!{}", a1(r, 0)), Cell::Text(art.into()));
        g.setup.input(format!("'{cat}'!{}", a1(r, 1)), Cell::Text(name.into()));
        g.setup.input(format!("'{cat}'!{}", a1(r, 2)), Cell::Text(category.into()));
        g.setup.input(format!("'{cat}'!{}", a1(r, 3)), Cell::Text(supplier(si).into()));
    }
    let table = format!("'{cat}'!$A$2:$D${}", PARTS.len() + 1);
    let start = 5000 + g.l.int(0, 400);
    for i in 0..n {
        let r = 2 + i;
        let (art, ..) = PARTS.get(g.d.below(PARTS.len())).copied().unwrap_or(PARTS[0]);
        let qty = g.d.amount(100, 5_000, 50);
        let sum = g.d.amount(50_000, 3_000_000, 1_000);
        // Qualified: the pivot sheet lands in front, and source checks must still find these.
        let at = |c: u32| format!("'{sheet}'!{}", a1(r, c));
        g.setup.input(at(0), Cell::Text(format!("PO-{}", start + i64::from(i))));
        g.setup.input(at(1), Cell::Text(art.into()));
        g.setup.input(at(2), Cell::Num(qty));
        g.setup.input(at(3), Cell::Num(sum));
    }
    // Qualified addresses: once the pivot sheet is in front, bare ones would point at it.
    for i in 0..n {
        let r = 2 + i;
        for (c, k) in [(4u32, 3), (5, 4)] {
            let cell = format!("'{sheet}'!{}", a1(r, c));
            g.setup.answer(a1(r, c));
            let reference = format!("=VLOOKUP('{sheet}'!B{r},{table},{k},FALSE)");
            g.check(&a1(r, c), Kind::Formula { cell, reference, live: true });
        }
    }
    let (rows, columns, func) = match level {
        1 => (vec!["Категория".to_string()], vec![], PivotFunc::Sum),
        2 | 3 => (vec!["Категория".to_string()], vec!["Поставщик".to_string()], PivotFunc::Sum),
        _ => (vec!["Поставщик".to_string()], vec!["Категория".to_string()], PivotFunc::Count),
    };
    g.check(
        "Сводная",
        Kind::Pivot {
            sheet: String::new(),
            source: format!("'{sheet}'!A1:F{}", n + 1),
            rows,
            columns,
            values: vec![("Сумма, тг".into(), func)],
        },
    );
    let prompt = match level {
        1 => format!("В заказах нет категории и поставщика - они в «Каталоге». Подтяни их ВПР по артикулу (E2: =ВПР(B2;{table};3;ЛОЖЬ), F2 - то же с 4), протяни. Потом сводная: «Категория» в строки, «Сумма, тг» в значения."),
        2 => format!("ВПР по артикулу из «Каталога» ({table}): категория - 3-й столбец, поставщик - 4-й. Потом сводная: категория в строках, поставщик в столбцах, сумма в значениях."),
        3 => "Андрей: «Сколько мы потратили по категориям - электроника, батареи, корпуса - и у кого?» В выгрузке заказов только артикулы; категории и поставщики - в «Каталоге». Подтяни и сделай сводную: категории строками, поставщики столбцами.".to_string(),
        _ => "Андрей: «Сколько заказов мы сделали у каждого поставщика по каждой категории?» Категории и поставщики - в «Каталоге» по артикулу. Нужна сводная: поставщики строками, категории столбцами, количество заказов.".to_string(),
    };
    let mut task = g.task("Заказы + каталог → сводная", prompt, None, "E2".into());
    task.hints = vec![
        "Сводной нужны все поля в одном списке. Если категория живёт в другой таблице, её сначала приносят ВПР по общему ключу - артикулу. Это называется «соединить таблицы».".into(),
        format!("=ВПР(B2;{table};3;ЛОЖЬ) - категория, с 4 - поставщик. Таблица каталога закреплена $, иначе съедет при протягивании."),
        "Сводная: встань в список → Вставка → Сводная таблица. «Сколько денег» - Сумма, «сколько заказов» - Количество (меню поля в «Значениях» → Итоги по).".into(),
    ];
    task.goal_seconds = 150.0 + 40.0 * f32::from(level);
    Ok(task)
}

/// Build a pivot, then read the answer off it: what the manager actually asks.
pub fn pivotread(g: &mut Gen) -> Result<Task> {
    use gridcraft_engine::model::PivotFunc;
    let level = g.level;
    let sheet = "Данные";
    g.setup.first_sheet = Some(sheet.into());
    let at = |r: u32, c: u32| format!("'{sheet}'!{}", a1(r, c));
    let headers = ["Поставщик", "Месяц", "Сумма, тг"];
    for (c, h) in headers.iter().enumerate() {
        g.setup.text(at(1, c as u32), *h);
    }
    g.setup.bold.push(format!("'{sheet}'!A1:C1"));
    g.setup.widths.push((format!("'{sheet}'!A:A"), 210.0));
    g.setup.widths.push((format!("'{sheet}'!B:C"), 110.0));
    let n = match level {
        1 => 8,
        2 => 10,
        3 => 14,
        _ => 18,
    };
    let sups: Vec<&str> = (0..4).map(supplier).collect();
    let months = ["Январь", "Февраль", "Март"];
    let focus_sup = sups[g.l.below(sups.len())];
    let focus_month = months[g.l.below(3)];
    let mut by_sup = std::collections::BTreeMap::<&str, f64>::new();
    let mut by_month = std::collections::BTreeMap::<&str, f64>::new();
    let mut pair = 0.0;
    let mut count = 0.0;
    for i in 0..n {
        let r = 2 + i;
        // Every supplier and every month appear, and the asked pair at least once.
        let su = if i == 0 { focus_sup } else { sups[(i as usize + g.d.below(4)) % 4] };
        let mo = if i == 0 { focus_month } else { months[(i as usize + g.d.below(3)) % 3] };
        let v = g.d.amount(50_000, 2_000_000, 1_000);
        g.setup.input(at(r, 0), Cell::Text(su.into()));
        g.setup.input(at(r, 1), Cell::Text(mo.into()));
        g.setup.input(at(r, 2), Cell::Num(v));
        *by_sup.entry(su).or_default() += v;
        *by_month.entry(mo).or_default() += v;
        if su == focus_sup && mo == focus_month {
            pair += v;
        }
        if su == focus_sup {
            count += 1.0;
        }
    }
    let total_sup = by_sup.get(focus_sup).copied().unwrap_or(0.0);
    let top_month = by_month.iter().max_by(|a, b| a.1.total_cmp(b.1)).map_or("Январь", |(m, _)| m).to_string();
    let source = format!("'{sheet}'!A1:C{}", n + 1);
    let (rows, columns, func, q1, a1v) = match level {
        1 => (vec!["Поставщик"], vec![], PivotFunc::Sum, format!("Сколько всего потратили у {focus_sup}?"), total_sup),
        2 => (vec!["Поставщик"], vec![], PivotFunc::Count, format!("Сколько раз мы закупались у {focus_sup}?"), count),
        _ => (vec!["Поставщик"], vec!["Месяц"], PivotFunc::Sum, format!("Сколько потратили у {focus_sup} за {}?", focus_month.to_lowercase()), pair),
    };
    g.check(
        "Сводная",
        Kind::Pivot {
            sheet: String::new(),
            source,
            rows: rows.iter().map(|x| x.to_string()).collect(),
            columns: columns.iter().map(|x| x.to_string()).collect(),
            values: vec![("Сумма, тг".into(), func)],
        },
    );
    let (lc, vc) = (4u32, 5u32);
    g.setup.widths.push((format!("'{sheet}'!E:E"), 330.0));
    g.setup.widths.push((format!("'{sheet}'!F:F"), 120.0));
    g.setup.text(at(2, lc), &q1);
    g.setup.answer(at(2, vc));
    g.check("Ответ 1", Kind::Value { cell: at(2, vc), expected: Cell::Num(a1v) });
    let mut asks = format!("1) {q1} → {}", a1(2, vc));
    if level >= 4 {
        let q2 = "В каком месяце потратили больше всего (по всем поставщикам)?";
        g.setup.text(at(3, lc), q2);
        g.setup.answer(at(3, vc));
        g.check("Ответ 2", Kind::Value { cell: at(3, vc), expected: Cell::Text(top_month) });
        asks.push_str(&format!("\n2) {q2} → {} (впиши название месяца)", a1(3, vc)));
    }
    let how = match level {
        1 => "Встань в таблицу → Вставка → Сводная таблица. «Поставщик» отметь (уйдёт в строки), «Сумма, тг» - в значения. Найди в сводной строку поставщика и перепиши число в ответ на листе «Данные».",
        2 => "Сводная: «Поставщик» в строки, «Сумма, тг» в значения, а потом поменяй Сумму на Количество (меню поля в «Значениях» → Итоги по: Количество): вопрос «сколько раз», а не «сколько денег».",
        3 => "Сводная: поставщики строками, месяцы столбцами, сумма в значениях. Ответ - на пересечении строки поставщика и столбца месяца.",
        _ => "Построй сводную, которая отвечает на оба вопроса сразу, и перенеси ответы на лист «Данные».",
    };
    let prompt = format!("Андрей спрашивает:\n{asks}\nОтветь с помощью сводной таблицы. {how}");
    let mut task = g.task("Сводная: найди ответ", prompt, None, at(2, 0));
    task.hints = vec![
        "Сводная собирается из одного списка: встань в любую ячейку таблицы → Вставка → Сводная таблица. Она откроется на новом листе.".into(),
        "Что куда: по чему группируем (поставщик, месяц) - в Строки или Столбцы; что считаем (сумма) - в Значения. «Сколько денег» - Сумма, «сколько раз» - Количество.".into(),
        "Ответ ищи на пересечении: строка нужного поставщика × столбец нужного месяца. Строка «Общий итог» - это всё вместе, не путай.".into(),
        "Ответы вписывай на листе «Данные» (вкладка внизу), в жёлтые ячейки.".into(),
    ];
    task.goal_seconds = 90.0 + 30.0 * f32::from(level);
    Ok(task)
}

/// (article, name, supplier index, norms per LA-IP / TWM / GMZ, pack, minimum order, price in the supplier's currency)
const KIT: &[(&str, &str, usize, [f64; 3], f64, f64, f64)] = &[
    ("PCB-LAIP", "Плата LA-IP в сборе", 0, [1.0, 0.0, 0.0], 100.0, 500.0, 6.8),
    ("PCB-TWM", "Плата TWM в сборе", 0, [0.0, 1.0, 0.0], 100.0, 500.0, 5.9),
    ("RF-868", "LoRa-модуль 868 МГц", 0, [1.0, 1.0, 1.0], 250.0, 1000.0, 4.2),
    ("BAT-14505", "Батарея ER14505 3,6 В", 1, [1.0, 1.0, 2.0], 500.0, 5000.0, 66.0),
    ("CASE-TWM", "Корпус-крышка TWM", 2, [0.0, 1.0, 0.0], 200.0, 1000.0, 33.0),
    ("LBL-QR", "Этикетка с QR-кодом", 4, [1.0, 1.0, 1.0], 1000.0, 1000.0, 12.0),
    ("PCB-GMZ", "Плата GMZ в сборе", 0, [0.0, 0.0, 1.0], 100.0, 300.0, 8.4),
    ("CBL-IMP", "Кабель импульсный, м", 3, [3.0, 0.0, 0.0], 100.0, 100.0, 180.0),
    ("NB-MOD", "NB-IoT модуль", 0, [0.0, 0.0, 1.0], 100.0, 200.0, 7.5),
    ("ANT-868", "Антенна 868 МГц", 0, [1.0, 0.0, 1.0], 500.0, 1000.0, 0.9),
    ("CASE-LA", "Корпус LA-IP", 2, [1.0, 0.0, 0.0], 200.0, 1000.0, 39.0),
    ("CASE-GMZ", "Корпус GMZ", 2, [0.0, 0.0, 1.0], 100.0, 500.0, 43.0),
    ("BOX-1", "Коробка индивидуальная", 4, [1.0, 1.0, 1.0], 100.0, 500.0, 95.0),
];

/// The final case: a supply folder of a dozen sheets, as on a real Monday.
pub fn quarter(g: &mut Gen) -> Result<Task> {
    use gridcraft_engine::model::PivotFunc;
    let level = g.level;
    let n = match level {
        1 => 6,
        2 => 8,
        3 => 11,
        _ => KIT.len(),
    };
    let with_moq = level >= 2;
    let unconfirmed = level >= 3;
    let report = "Отчёт";
    g.setup.first_sheet = Some(report.into());
    let q = |sh: &str, r: u32, c: u32| format!("'{sh}'!{}", a1(r, c));
    let head = |g: &mut Gen, sh: &str, hs: &[&str]| {
        for (c, h) in hs.iter().enumerate() {
            g.setup.text(q(sh, 1, c as u32), *h);
        }
        g.setup.bold.push(format!("'{sh}'!A1:{}", a1(1, hs.len() as u32 - 1)));
        g.setup.widths.push((format!("'{sh}'!A:{}", crate::sheet::col_name(hs.len() as u32 - 1)), 150.0));
    };
    let models = ["LA-IP", "TWM", "GMZ"];
    let plan = [g.d.amount(2_000, 5_000, 100), g.d.amount(3_000, 6_000, 100), g.d.amount(500, 1_500, 100)];
    let usd = 480.0 + g.d.int(0, 50) as f64;
    let cny = 64.0 + g.d.int(0, 9) as f64;
    let rate = |cur: &str| match cur {
        "USD" => usd,
        "CNY" => cny,
        _ => 1.0,
    };
    // Report template: the article list is given, everything else is the job.
    let rh = ["Артикул", "Компонент", "Нужно по плану", "На складах", "В пути", "Не хватает", "Заказать", "Поставщик", "Цена, тг/шт", "Сумма, тг"];
    for (c, h) in rh.iter().enumerate() {
        g.setup.text(a1(1, c as u32), *h);
        g.setup.width(c as u32, if c == 1 || c == 7 { 200.0 } else { 115.0 });
    }
    g.setup.bold.push(format!("A1:{}", a1(1, rh.len() as u32 - 1)));
    // Production plan and BOM.
    head(g, "План производства", &["Модель", "План, шт"]);
    for (i, m) in models.iter().enumerate() {
        g.setup.input(q("План производства", 2 + i as u32, 0), Cell::Text((*m).into()));
        g.setup.input(q("План производства", 2 + i as u32, 1), Cell::Num(plan[i]));
    }
    head(g, "Спецификация", &["Артикул", "LA-IP", "TWM", "GMZ"]);
    head(g, "Номенклатура", &["Артикул", "Наименование", "Поставщик", "Валюта"]);
    head(g, "Склад Алматы", &["Артикул", "Остаток, шт"]);
    head(g, "Склад Москва", &["Остаток, шт", "Артикул"]);
    let mut trh = vec!["№ заказа", "Артикул", "Кол-во", "Статус"];
    if !unconfirmed {
        trh.truncate(3);
    }
    head(g, "Заказы в пути", &trh);
    head(g, "Курсы", &["Валюта", "Курс, тг"]);
    for (i, c) in ["USD", "CNY", "KZT"].iter().enumerate() {
        g.setup.input(q("Курсы", 2 + i as u32, 0), Cell::Text((*c).into()));
        g.setup.input(q("Курсы", 2 + i as u32, 1), Cell::Num(rate(c)));
    }
    let price_sheet = |si: usize| -> String {
        let (_, cur, ..) = SUPPLIERS.get(si).copied().unwrap_or(SUPPLIERS[0]);
        let short = ["Lianda", "PowerCell", "Plastech", "КазРадиоКомплект", "АлматыПак"].get(si).copied().unwrap_or("Lianda");
        format!("Прайс {short} {cur}")
    };
    let used: std::collections::BTreeSet<usize> = KIT.iter().take(n).map(|k| k.2).collect();
    let mut price_rows = std::collections::BTreeMap::<usize, u32>::new();
    for si in &used {
        let sh = price_sheet(*si);
        let mut hs = vec!["Артикул", "Цена", "В упаковке"];
        if with_moq {
            hs.push("Мин. партия");
        }
        head(g, &sh, &hs);
        price_rows.insert(*si, 2);
    }
    let mut order: Vec<usize> = (0..n).collect();
    let mut msk_order = order.clone();
    g.l.shuffle(&mut order);
    g.l.shuffle(&mut msk_order);
    let covered = g.l.below(n);
    let small = (covered + 1) % n;
    let mut rows = Vec::new();
    let mut budget = 0.0;
    let mut po = 7000 + g.l.int(0, 900);
    let mut tr = 2u32;
    for (i, k) in KIT.iter().take(n).enumerate() {
        let (art, name, si, norms, pack, moq, price) = *k;
        let (sup, cur, ..) = SUPPLIERS.get(si).copied().unwrap_or(SUPPLIERS[0]);
        let need: f64 = norms.iter().zip(plan).map(|(a, b)| a * b).sum();
        // Stock split between the two warehouses; transit in one or two orders.
        let (alm, msk, conf, unconf) = if i == covered {
            (need, g.d.amount(100, 800, 50), 0.0, 0.0)
        } else if i == small {
            let m = if with_moq { (moq * 0.4).round() } else { (pack * 0.5).round() };
            let rest = need - m;
            ((rest * 0.6).round(), rest - (rest * 0.6).round(), 0.0, g.d.amount(100, 400, 50))
        } else {
            let a = g.d.amount(0, (need * 0.3) as i64, 50);
            let b = g.d.amount(0, (need * 0.2) as i64, 50);
            (a, b, g.d.amount(0, (need * 0.2) as i64, 50), g.d.amount(0, (need * 0.2) as i64, 50))
        };
        let unconf = if unconfirmed { unconf } else { 0.0 };
        // Rows of the spec, directory and stock sheets go in shuffled orders.
        let pos = order.iter().position(|x| *x == i).unwrap_or(i) as u32 + 2;
        let mpos = msk_order.iter().position(|x| *x == i).unwrap_or(i) as u32 + 2;
        g.setup.input(q("Спецификация", pos, 0), Cell::Text(art.into()));
        for (m, v) in norms.iter().enumerate() {
            g.setup.input(q("Спецификация", pos, 1 + m as u32), Cell::Num(*v));
        }
        g.setup.input(q("Номенклатура", mpos, 0), Cell::Text(art.into()));
        g.setup.input(q("Номенклатура", mpos, 1), Cell::Text(name.into()));
        g.setup.input(q("Номенклатура", mpos, 2), Cell::Text(sup.into()));
        g.setup.input(q("Номенклатура", mpos, 3), Cell::Text(cur.into()));
        g.setup.input(q("Склад Алматы", pos, 0), Cell::Text(art.into()));
        g.setup.input(q("Склад Алматы", pos, 1), Cell::Num(alm));
        g.setup.input(q("Склад Москва", mpos, 0), Cell::Num(msk));
        g.setup.input(q("Склад Москва", mpos, 1), Cell::Text(art.into()));
        for (qty, status) in [(conf, "Подтверждён"), (unconf, "Не подтверждён")] {
            if qty <= 0.0 {
                continue;
            }
            g.setup.input(q("Заказы в пути", tr, 0), Cell::Text(format!("PO-{po}")));
            g.setup.input(q("Заказы в пути", tr, 1), Cell::Text(art.into()));
            g.setup.input(q("Заказы в пути", tr, 2), Cell::Num(qty));
            if unconfirmed {
                g.setup.input(q("Заказы в пути", tr, 3), Cell::Text(status.into()));
            }
            tr += 1;
            po += 1;
        }
        let psh = price_sheet(si);
        let pr = price_rows.get(&si).copied().unwrap_or(2);
        g.setup.input(q(&psh, pr, 0), Cell::Text(art.into()));
        g.setup.input(q(&psh, pr, 1), Cell::Num(price));
        g.setup.input(q(&psh, pr, 2), Cell::Num(pack));
        if with_moq {
            g.setup.input(q(&psh, pr, 3), Cell::Num(moq));
        }
        price_rows.insert(si, pr + 1);
        let stock = alm + msk;
        let short = (need - stock - conf).max(0.0);
        let up = (short / pack).ceil() * pack;
        let buy = if short <= 0.0 { 0.0 } else if with_moq { up.max(moq) } else { up };
        let tg = price * rate(cur);
        budget += buy * tg;
        // Qualified: the pivot sheet lands in front of the report.
        g.setup.input(q(report, 2 + i as u32, 0), Cell::Text(art.into()));
        g.setup.input(q(report, 2 + i as u32, 1), Cell::Text(name.into()));
        rows.push(vec![
            Expectation::Text(art.into()),
            Expectation::Text(name.into()),
            Expectation::Number(need),
            Expectation::Number(stock),
            Expectation::Number(conf),
            Expectation::Number(short),
            Expectation::Number(buy),
            Expectation::Text(sup.into()),
            Expectation::Number(tg),
            Expectation::Number(buy * tg),
        ]);
    }
    g.check(
        "Заказ на месяц",
        Kind::Table {
            first: 2,
            left: 0,
            headers: rh.iter().map(|h| h.to_string()).collect(),
            cols: vec![Col::Given, Col::Given, Col::Number, Col::Number, Col::Number, Col::Number, Col::Number, Col::Text, Col::Number, Col::Number],
            rows,
            key: 0,
            notes: Vec::new(),
        },
    );
    // Sales of finished modems by country.
    let countries = ["Казахстан", "Россия", "Кыргызстан", "Польша", "Эстония"];
    let partners = ["ТОО ИнтерСеть", "ООО СмартУчёт", "ОсОО БишкекТелеком", "Riza Global Sp. z o.o.", "Baltic Metering OÜ"];
    head(g, "Продажи", &["Месяц", "Страна", "Партнёр", "Модель", "Кол-во, шт"]);
    let sales_n = 10 + 2 * u32::from(level);
    let mut by_country = std::collections::BTreeMap::<&str, f64>::new();
    for i in 0..sales_n {
        let r = 2 + i;
        let c = (i as usize + g.d.below(5)) % 5;
        let m = (i as usize + g.d.below(3)) % 3;
        let qty = g.d.amount(100, 3_000, 50);
        g.setup.input(q("Продажи", r, 0), Cell::Text(["Июль", "Август", "Сентябрь"][(i % 3) as usize].into()));
        g.setup.input(q("Продажи", r, 1), Cell::Text(countries[c].into()));
        g.setup.input(q("Продажи", r, 2), Cell::Text(partners[c].into()));
        g.setup.input(q("Продажи", r, 3), Cell::Text(models[m].into()));
        g.setup.input(q("Продажи", r, 4), Cell::Num(qty));
        *by_country.entry(countries[c]).or_default() += qty;
    }
    let top = by_country.iter().max_by(|a, b| a.1.total_cmp(b.1)).map_or("Казахстан", |(c, _)| c).to_string();
    g.check(
        "Сводная продаж",
        Kind::Pivot {
            sheet: String::new(),
            source: format!("'Продажи'!A1:E{}", sales_n + 1),
            rows: vec!["Страна".into()],
            columns: vec!["Модель".into()],
            values: vec![("Кол-во, шт".into(), PivotFunc::Sum)],
        },
    );
    let ar = n as u32 + 3;
    g.setup.text(q(report, ar, 7), "Бюджет закупки, тг:");
    g.setup.bold.push(q(report, ar, 7));
    g.setup.answer(q(report, ar, 9));
    g.check("Бюджет", Kind::Value { cell: q(report, ar, 9), expected: Cell::Num(budget) });
    g.setup.text(q(report, ar + 1, 7), "Страна №1 по продажам:");
    g.setup.bold.push(q(report, ar + 1, 7));
    g.setup.answer(q(report, ar + 1, 9));
    g.check("Лидер продаж", Kind::Value { cell: q(report, ar + 1, 9), expected: Cell::Text(top) });
    let moq_line = if with_moq { " и не меньше минимальной партии из прайса (но если не хватает 0, заказ 0)" } else { " (если не хватает 0, заказ 0)" };
    let transit_line = if unconfirmed { " Неподтверждённые заказы в пути не учитывай: в «В пути» только «Подтверждён»." } else { "" };
    let prompt = format!(
        "Андрей: «Собери заказ комплектующих на следующий месяц и цифры к планёрке».\n\
1) Лист «Отчёт»: по каждому артикулу - сколько нужно по плану (спецификация × план производства), сколько на двух складах вместе, сколько в пути, сколько не хватает (не меньше 0), сколько заказать целыми упаковками{moq_line}, у кого (по «Номенклатуре»), цена за штуку в тенге (цена из прайса поставщика × курс, без округления) и сумма.{transit_line}\n\
2) Бюджет закупки - сумма всех заказов → J{ar}.\n\
3) Сводная продаж по листу «Продажи»: страны строками, модели столбцами, сумма штук. Страна-лидер → J{}.",
        ar + 1
    );
    let mut task = g.task("Большой кейс: квартал", prompt, None, "C2".into());
    task.hints = vec![
        "Не пытайся всё сразу. Порядок: сначала «Нужно» (спецификация × план по каждой модели), потом склады, потом в пути, потом «Не хватает», потом заказ, поставщик, цена, сумма. Каждый столбец - одна формула, протянутая вниз.".into(),
        "Склады лежат в разном порядке строк и столбцов: только ВПР/СУММЕСЛИ по артикулу, не копирование. Москва: =СУММЕСЛИ('Склад Москва'!B:B;A2;'Склад Москва'!A:A). В пути: СУММЕСЛИ (или СУММЕСЛИМН со статусом) по артикулу.".into(),
        "Нужно по плану: =ВПР(A2;'Спецификация'!$A$2:$D$14;2;ЛОЖЬ)*'План производства'!$B$2 + то же с 3 и $B$3 + с 4 и $B$4.".into(),
        "Цена: у каждого поставщика свой прайс на своём листе. Проще всего - сложить все прайсы на новый лист «Все прайсы» друг под другом (одна шапка) и искать там одним ВПР. Курс - ВПР по валюте из «Номенклатуры» в таблице «Курсы». Заказ: =ЕСЛИ(F2=0;0;МАКС(мин_партия;ОКРУГЛВВЕРХ(F2/упаковка;0)*упаковка)).".into(),
    ];
    task.goal_seconds = 1200.0 + 300.0 * f32::from(level);
    task.reorders = false;
    Ok(task)
}

/// The test task of an interview: three questions on an unfamiliar book, on the clock.
pub fn exam(g: &mut Gen) -> Result<Task> {
    let n = g.rows() + 4;
    let t = g.table("Реестр заказов", &["№ заказа", "Артикул", "Поставщик", "Кол-во", "Ожидаем", "Статус", "Сумма, тг"], n);
    g.setup.width(t.col(2), 210.0);
    let today = date(2026, 10, 20);
    let tc = g.param(&t, 1, "Сегодня:", Cell::Date(today));
    // Price list to the right of the parameter.
    let pc = t.right + 5;
    g.setup.text(a1(t.header, pc), "Артикул");
    g.setup.text(a1(t.header, pc + 1), "Цена, тг");
    g.setup.bold.push(a1(t.header, pc));
    g.setup.bold.push(a1(t.header, pc + 1));
    g.setup.width(pc, 110.0);
    let mut prices = Vec::new();
    for (i, (art, _, _, _, price)) in PARTS.iter().enumerate() {
        let tenge = if *price < 50.0 { (price * 500.0).round() } else { *price };
        g.setup.input(a1(t.header + 1 + i as u32, pc), Cell::Text((*art).into()));
        g.setup.input(a1(t.header + 1 + i as u32, pc + 1), Cell::Num(tenge));
        prices.push(tenge);
    }
    let focus_sup = supplier(g.l.below(3));
    let start = 6000 + g.l.int(0, 900);
    let (mut sup_total, mut late) = (0.0, 0);
    for (i, r) in t.rows().enumerate() {
        let pi = g.d.below(PARTS.len());
        let (art, _, _, si, _) = PARTS.get(pi).copied().unwrap_or(PARTS[0]);
        let si = if i % 4 == 0 { SUPPLIERS.iter().position(|s| s.0 == focus_sup).unwrap_or(si) } else { si };
        let qty = g.d.amount(100, 3_000, 50);
        let due = today + g.d.int(-12, 15) as f64;
        let status = if g.d.chance(35) { RECEIVED } else { OPEN[g.d.below(2)] };
        g.txt(&t, 0, r, &format!("PO-{}", start + i as i64));
        g.txt(&t, 1, r, art);
        g.txt(&t, 2, r, supplier(si));
        g.num(&t, 3, r, qty);
        g.day(&t, 4, r, due);
        g.txt(&t, 5, r, status);
        if supplier(si) == focus_sup {
            sup_total += qty * prices.get(pi).copied().unwrap_or(0.0);
        }
        if status != RECEIVED && due < today {
            late += 1;
        }
    }
    let plist = format!("{}:{}", dollar(&a1(t.header + 1, pc)), dollar(&a1(t.header + PARTS.len() as u32, pc + 1)));
    let amount = |r: u32| format!("={}*VLOOKUP({},{plist},2,FALSE)", t.cell(3, r), t.cell(1, r));
    let _ = answer_col(g, &t, 6, "Сумма, тг", &amount);
    let q2 = g.answer(&t, 3, "Сумма у поставщика:");
    let q3 = g.answer(&t, 4, "Просрочено поставок:");
    g.check("Сумма у поставщика", Kind::Value { cell: q2.clone(), expected: Cell::Num(sup_total) });
    g.check("Просрочено", Kind::Value { cell: q3.clone(), expected: Cell::Num(f64::from(late)) });
    let (c1, c2) = (t.cell(6, t.first), t.cell(6, t.last));
    let prompt = format!(
        "Тестовое, 15 минут. Андрей прислал реестр и три вопроса:\n1) В {c1}:{c2} - сумма каждого заказа: кол-во × цена из прайса справа.\n2) На какую сумму заказано у {focus_sup}? → {q2}\n3) Сколько поставок просрочено (срок раньше {tc}, а статус не «Получен»)? → {q3}"
    );
    let mut task = g.task("Тестовое на время", prompt, None, c1);
    task.hints = vec![
        "Порядок: сначала столбец сумм (ВПР цены × кол-во), он нужен для второго вопроса. Потом СУММЕСЛИ по поставщику. Потом СЧЁТЕСЛИМН с двумя условиями.".into(),
        format!("Просрочено: =СЧЁТЕСЛИМН(срок;\"<\"&{tc};статус;\"<>Получен\")."),
    ];
    task.goal_seconds = 900.0;
    Ok(task)
}

#[cfg(test)]
mod tests {
    use crate::check::evaluate;
    use crate::{Spec, Task};


    use crate::check::Kind;
    use crate::sheet::{self, Cell};
    use serde_json::json;

    fn first_formula_cells(task: &Task) -> Vec<String> {
        task.checks.iter().filter_map(|c| if let Kind::Formula { cell, .. } = &c.kind { Some(cell.clone()) } else { None }).collect()
    }

    /// Writes `f(row)` into every row of a column, the way a learner fills down.
    fn fill(s: &mut gridcraft_engine::Session, col: &str, rows: std::ops::RangeInclusive<u32>, f: &dyn Fn(u32) -> String) {
        for r in rows {
            sheet::set(s, &format!("{col}{r}"), &Cell::Formula(f(r))).unwrap_or_else(|e| panic!("{e}"));
        }
    }

    fn rows_of(task: &Task, col: &str) -> std::ops::RangeInclusive<u32> {
        let rows: Vec<u32> = first_formula_cells(task)
            .iter()
            .filter(|c| c.starts_with(col) && c[col.len()..].chars().all(|ch| ch.is_ascii_digit()))
            .filter_map(|c| c[col.len()..].parse().ok())
            .collect();
        *rows.iter().min().unwrap_or(&2)..=*rows.iter().max().unwrap_or(&2)
    }

    fn passed(task: &Task, s: &gridcraft_engine::Session) -> (bool, String) {
        let e = evaluate(task, s, &[]).unwrap_or_else(|e| panic!("{e}"));
        (e.passed(), e.outcomes.iter().filter(|o| !o.passed).map(|o| format!("{}: {}", o.label, o.feedback)).collect::<Vec<_>>().join(" | "))
    }

    /// The natural way: later columns point at the learner's earlier ones.
    #[test]
    fn learner_style_formulas_using_their_own_columns_pass() {
        for seed in [2u64, 17] {
            // Level 1-2 tables start at A1 with headers in row 1.
            let task = Task::generate(&Spec::new("orion.mrp", 2, seed)).unwrap_or_else(|e| panic!("{e}"));
            let mut s = task.session().unwrap_or_else(|e| panic!("{e}"));
            let rows = rows_of(&task, "G");
            fill(&mut s, "G", rows.clone(), &|r| format!("=B{r}*$K$2+C{r}*$K$3+D{r}*$K$4"));
            fill(&mut s, "H", rows, &|r| format!("=MAX(0,G{r}-E{r}-F{r})"));
            let (ok, fb) = passed(&task, &s);
            assert!(ok, "mrp s{seed}: {fb}");

            let task = Task::generate(&Spec::new("orion.moq", 2, seed)).unwrap_or_else(|e| panic!("{e}"));
            let mut s = task.session().unwrap_or_else(|e| panic!("{e}"));
            let rows = rows_of(&task, "E");
            fill(&mut s, "E", rows.clone(), &|r| format!("=MAX(D{r},CEILING(B{r},C{r}))"));
            fill(&mut s, "F", rows, &|r| format!("=E{r}-B{r}"));
            let (ok, fb) = passed(&task, &s);
            assert!(ok, "moq s{seed}: {fb}");

            let task = Task::generate(&Spec::new("orion.reorder", 2, seed)).unwrap_or_else(|e| panic!("{e}"));
            let mut s = task.session().unwrap_or_else(|e| panic!("{e}"));
            let rows = rows_of(&task, "F");
            fill(&mut s, "F", rows.clone(), &|r| format!("=INT(B{r}/C{r})"));
            fill(&mut s, "G", rows.clone(), &|r| format!("=$K$2+F{r}-D{r}-E{r}"));
            fill(&mut s, "H", rows, &|r| format!("=IF(G{r}<=$K$2,\"Заказать сейчас\",\"ОК\")"));
            let (ok, fb) = passed(&task, &s);
            assert!(ok, "reorder s{seed}: {fb}");

            let task = Task::generate(&Spec::new("orion.landed", 2, seed)).unwrap_or_else(|e| panic!("{e}"));
            let mut s = task.session().unwrap_or_else(|e| panic!("{e}"));
            let rows = rows_of(&task, "D");
            let last = *rows.end();
            fill(&mut s, "D", rows.clone(), &|r| format!("=B{r}*C{r}*$J$2"));
            fill(&mut s, "E", rows.clone(), &|r| format!("=D{r}/SUM($D$2:$D${last})*$J$3"));
            fill(&mut s, "F", rows.clone(), &|r| format!("=(D{r}+E{r})*(1+$J$4)*(1+$J$5)"));
            fill(&mut s, "G", rows, &|r| format!("=F{r}/C{r}"));
            let (ok, fb) = passed(&task, &s);
            assert!(ok, "landed s{seed}: {fb}");

            let task = Task::generate(&Spec::new("orion.abc", 2, seed)).unwrap_or_else(|e| panic!("{e}"));
            let mut s = task.session().unwrap_or_else(|e| panic!("{e}"));
            let rows = rows_of(&task, "D");
            let last = *rows.end();
            fill(&mut s, "D", rows.clone(), &|r| format!("=C{r}/SUM($C$2:$C${last})"));
            fill(&mut s, "E", rows.clone(), &|r| if r == 2 { "=D2".into() } else { format!("=E{}+D{r}", r - 1) });
            fill(&mut s, "F", rows, &|r| format!("=IF(E{r}<=0.8,\"A\",IF(E{r}<=0.95,\"B\",\"C\"))"));
            let a_count = (2..=last).filter(|r| matches!(sheet::value(&s, &format!("F{r}")), Ok(gridcraft_engine::core::Value::Text(t)) if &*t == "A")).count();
            sheet::set(&mut s, "I2", &Cell::Num(a_count as f64)).unwrap_or_else(|e| panic!("{e}"));
            let (ok, fb) = passed(&task, &s);
            assert!(ok, "abc s{seed}: {fb}");
        }
    }

    /// Lookups with bare addresses on the orders sheet, then the pivot from the ribbon.
    #[test]
    fn join_by_hand_then_pivot_from_the_ribbon() {
        for level in 1..=4u8 {
            let task = Task::generate(&Spec::new("orion.join", level, 8)).unwrap_or_else(|e| panic!("{e}"));
            let Some(Kind::Pivot { rows, columns, values, .. }) = task.checks.iter().map(|c| c.kind.clone()).find(|k| matches!(k, Kind::Pivot { .. }))
            else {
                panic!("pivot")
            };
            let mut s = task.session().unwrap_or_else(|e| panic!("{e}"));
            let r = rows_of(&task, "'Заказы'!E");
            let _ = r;
            let n = first_formula_cells(&task).len() as u32 / 2;
            fill(&mut s, "E", 2..=n + 1, &|r| format!("=VLOOKUP(B{r},'Каталог'!$A$2:$D$13,3,FALSE)"));
            fill(&mut s, "F", 2..=n + 1, &|r| format!("=VLOOKUP(B{r},'Каталог'!$A$2:$D$13,4,FALSE)"));
            s.execute("selection.set", json!({"range": "'Заказы'!A2"})).unwrap_or_else(|e| panic!("{e}"));
            let out = s.execute("insert.pivotTable", json!({})).unwrap_or_else(|e| panic!("{e}"));
            let name = out["name"].as_str().map(str::to_string).unwrap_or_else(|| "PivotTable1".into());
            for f in rows.iter().chain(&columns) {
                s.execute("pivot.addField", json!({"pivot": name, "field": f, "area": "rows"})).unwrap_or_else(|e| panic!("{e}"));
            }
            for f in &columns {
                s.execute("pivot.moveField", json!({"pivot": name, "field": f, "from": "rows", "to": "columns"})).unwrap_or_else(|e| panic!("{e}"));
            }
            let (field, func) = values[0].clone();
            s.execute("pivot.addField", json!({"pivot": name, "field": field, "area": "values"})).unwrap_or_else(|e| panic!("{e}"));
            if func != gridcraft_engine::model::PivotFunc::Sum {
                s.execute("pivot.valueSettings", json!({"pivot": name, "field": field, "func": "count"})).unwrap_or_else(|e| panic!("{e}"));
            }
            let (ok, fb) = passed(&task, &s);
            assert!(ok, "join L{level}: {fb}");
        }
    }


    /// Build the pivot from the ribbon, read the number off it, type it on the data sheet.
    #[test]
    fn pivot_read_from_the_ribbon_and_typed_answers_pass() {
        for level in 1..=4u8 {
            for seed in [4u64, 31] {
                let task = Task::generate(&Spec::new("orion.pivotread", level, seed)).unwrap_or_else(|e| panic!("{e}"));
                let Some(Kind::Pivot { rows, columns, values, .. }) = task.checks.iter().map(|c| c.kind.clone()).find(|k| matches!(k, Kind::Pivot { .. }))
                else {
                    panic!("pivot")
                };
                let mut s = task.session().unwrap_or_else(|e| panic!("{e}"));
                s.execute("selection.set", json!({"range": "'Данные'!A2"})).unwrap_or_else(|e| panic!("{e}"));
                let out = s.execute("insert.pivotTable", json!({})).unwrap_or_else(|e| panic!("{e}"));
                let name = out["name"].as_str().map(str::to_string).unwrap_or_else(|| "PivotTable1".into());
                for f in rows.iter().chain(&columns) {
                    s.execute("pivot.addField", json!({"pivot": name, "field": f, "area": "rows"})).unwrap_or_else(|e| panic!("{e}"));
                }
                for f in &columns {
                    s.execute("pivot.moveField", json!({"pivot": name, "field": f, "from": "rows", "to": "columns"})).unwrap_or_else(|e| panic!("{e}"));
                }
                let (field, func) = values[0].clone();
                s.execute("pivot.addField", json!({"pivot": name, "field": field, "area": "values"})).unwrap_or_else(|e| panic!("{e}"));
                if func != gridcraft_engine::model::PivotFunc::Sum {
                    s.execute("pivot.valueSettings", json!({"pivot": name, "field": field, "func": "count"})).unwrap_or_else(|e| panic!("{e}"));
                }
                let (ok, fb) = passed(&task, &s);
                assert!(!ok && fb.contains("пуст"), "L{level}: answers still empty: {fb}");
                for c in &task.checks {
                    if let Kind::Value { cell, expected } = &c.kind {
                        sheet::set(&mut s, cell, expected).unwrap_or_else(|e| panic!("{e}"));
                    }
                }
                let (ok, fb) = passed(&task, &s);
                assert!(ok, "pivotread L{level} s{seed}: {fb}");
            }
        }
    }


    /// The whole case by formulas across sheets, the pivot from the ribbon, answers typed.
    #[test]
    fn quarter_case_by_cross_sheet_formulas() {
        for level in 1..=4u8 {
            for seed in [6u64, 77] {
                let task = Task::generate(&Spec::new("orion.quarter", level, seed)).unwrap_or_else(|e| panic!("{e}"));
                let mut s = task.session().unwrap_or_else(|e| panic!("{e}"));
                let Some(Kind::Table { rows, .. }) = task.checks.first().map(|c| c.kind.clone()) else { panic!("table") };
                let last = rows.len() as u32 + 1;
                let wb = s.doc().unwrap_or_else(|e| panic!("{e}")).wb.clone();
                let prices: Vec<String> = wb.sheets.iter().map(|sh| sh.name.to_string()).filter(|n| n.starts_with("Прайс")).collect();
                let lookup = |col: u32| {
                    prices.iter().rev().fold("NA()".to_string(), |acc, sh| format!("IFERROR(VLOOKUP(A{{r}},'{sh}'!$A:$D,{col},FALSE),{acc})"))
                };
                let (price, pack) = (lookup(2), lookup(3));
                let moq = lookup(4);
                let confirmed = if level >= 3 { ",'Заказы в пути'!$D:$D,\"Подтверждён\"" } else { "" };
                let buy = if level >= 2 { format!("IF(F{{r}}=0,0,MAX({moq},CEILING(F{{r}},{pack})))") } else { format!("IF(F{{r}}=0,0,CEILING(F{{r}},{pack}))") };
                let cols: Vec<(&str, String)> = vec![
                    ("C", "=VLOOKUP(A{r},'Спецификация'!$A$2:$D$14,2,FALSE)*'План производства'!$B$2+VLOOKUP(A{r},'Спецификация'!$A$2:$D$14,3,FALSE)*'План производства'!$B$3+VLOOKUP(A{r},'Спецификация'!$A$2:$D$14,4,FALSE)*'План производства'!$B$4".into()),
                    ("D", "=SUMIF('Склад Алматы'!$A:$A,A{r},'Склад Алматы'!$B:$B)+SUMIF('Склад Москва'!$B:$B,A{r},'Склад Москва'!$A:$A)".into()),
                    ("E", format!("=SUMIFS('Заказы в пути'!$C:$C,'Заказы в пути'!$B:$B,A{{r}}{confirmed})")),
                    ("F", "=MAX(0,C{r}-D{r}-E{r})".into()),
                    ("G", format!("={buy}")),
                    ("H", "=VLOOKUP(A{r},'Номенклатура'!$A:$D,3,FALSE)".into()),
                    ("I", format!("={price}*VLOOKUP(VLOOKUP(A{{r}},'Номенклатура'!$A:$D,4,FALSE),'Курсы'!$A$2:$B$4,2,FALSE)")),
                    ("J", "=G{r}*I{r}".into()),
                ];
                for (col, f) in &cols {
                    for r in 2..=last {
                        sheet::set(&mut s, &format!("'Отчёт'!{col}{r}"), &Cell::Formula(f.replace("{r}", &r.to_string()))).unwrap_or_else(|e| panic!("{e}"));
                    }
                }
                let (ok, fb) = passed(&task, &s);
                assert!(!ok && !fb.contains("Заказ на месяц"), "L{level} s{seed} table: {fb}");
                // Budget by formula, the leader typed after reading the pivot.
                let ar = last + 2;
                sheet::set(&mut s, &format!("'Отчёт'!J{ar}"), &Cell::Formula(format!("=SUM(J2:J{last})"))).unwrap_or_else(|e| panic!("{e}"));
                for c in &task.checks {
                    if let Kind::Value { cell, expected: expected @ Cell::Text(_) } = &c.kind {
                        sheet::set(&mut s, cell, expected).unwrap_or_else(|e| panic!("{e}"));
                    }
                }
                s.execute("selection.set", json!({"range": "'Продажи'!A2"})).unwrap_or_else(|e| panic!("{e}"));
                let out = s.execute("insert.pivotTable", json!({})).unwrap_or_else(|e| panic!("{e}"));
                let name = out["name"].as_str().map(str::to_string).unwrap_or_else(|| "PivotTable1".into());
                for (f, area) in [("Страна", "rows"), ("Модель", "rows")] {
                    s.execute("pivot.addField", json!({"pivot": name, "field": f, "area": area})).unwrap_or_else(|e| panic!("{e}"));
                }
                s.execute("pivot.moveField", json!({"pivot": name, "field": "Модель", "from": "rows", "to": "columns"})).unwrap_or_else(|e| panic!("{e}"));
                s.execute("pivot.addField", json!({"pivot": name, "field": "Кол-во, шт", "area": "values"})).unwrap_or_else(|e| panic!("{e}"));
                let (ok, fb) = passed(&task, &s);
                assert!(ok, "quarter L{level} s{seed}: {fb}");
            }
        }
    }

    #[test]
    fn every_orion_task_builds_fails_untouched_and_keeps_formulas_on_their_sheet() {
        for id in ["orion.reconcile", "orion.join", "orion.exam"] {
            for level in 1..=4u8 {
                for seed in [1u64, 5, 99] {
                    let task = Task::generate(&Spec::new(id, level, seed)).unwrap_or_else(|e| panic!("{id} L{level}: {e}"));
                    let s = task.session().unwrap_or_else(|e| panic!("{id}: {e}"));
                    assert!(!evaluate(&task, &s, &[]).unwrap_or_else(|e| panic!("{e}")).passed(), "{id} L{level} passes untouched");
                }
            }
        }
    }
}
