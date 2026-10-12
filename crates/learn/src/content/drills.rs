//! Formula and table drills.

use super::{CITIES, DEPTS, Gen, MONTHS, PARTIES, PEOPLE, PRODUCTS, Table};
use crate::Result;
use crate::Task;
use crate::check::Kind;
use crate::sheet::{Cell, a1, num_text};

fn row_of(g: &mut Gen, t: &Table) -> u32 {
    if g.level == 1 { t.first } else { t.first + g.l.below(t.n() as usize) as u32 }
}

/// A computed column where only one row is asked (before filling down is taught).
fn one_row(g: &mut Gen, title: &str, t: &Table, header: &str, row: u32, reference: &str, ask2: &str, ask3: &str) -> Task {
    let col = t.right - t.left + 1;
    g.setup.text(t.cell(col, t.header), header);
    g.setup.bold.push(t.cell(col, t.header));
    g.setup.width(t.col(col), 120.0);
    let cell = t.cell(col, row);
    g.setup.answer(cell.clone());
    g.single(title, &cell, reference, ask2, ask3)
}

pub fn enter(g: &mut Gen) -> Result<Task> {
    let n = g.rows().min(8);
    let t = g.table("Прайс-лист", &["Товар", "Цена, ₽"], n);
    let items = g.l.sample(PRODUCTS, n as usize);
    let blanks = match g.level {
        1 => 1,
        2 => 2,
        _ => 3,
    }
    .min(n as usize);
    let mut asks = Vec::new();
    let mut first_blank = None;
    for (i, r) in t.rows().enumerate() {
        let name = items.get(i).copied().unwrap_or("Товар");
        g.txt(&t, 0, r, name);
        let price = g.d.amount(50, 4000, 10);
        if i >= (n as usize - blanks) {
            let cell = t.cell(1, r);
            g.setup.answer(cell.clone());
            g.check(&cell, Kind::Value { cell: cell.clone(), expected: Cell::Num(price) });
            asks.push(format!("{name} — {}", num_text(price)));
            first_blank.get_or_insert(cell);
        } else {
            g.num(&t, 1, r, price);
        }
    }
    let focus = first_blank.unwrap_or_else(|| t.cell(1, t.first));
    let prompt = if g.level == 1 {
        format!("Поставщик прислал новую цену. Выдели {focus}, набери число и нажми Enter: {}.", asks.join(", "))
    } else {
        format!("Впиши недостающие цены: {}.", asks.join(", "))
    };
    let mut task = g.task("Ввод данных", prompt, None, focus);
    task.hints = vec![
        "Кликни ячейку или дойди до неё стрелками, набери число без пробелов и нажми Enter.".into(),
        "Пробелы между тысячами набирать не нужно: 1250, а не 1 250.".into(),
    ];
    Ok(task)
}

pub fn add(g: &mut Gen) -> Result<Task> {
    let n = g.rows();
    let t = g.table("Выручка по точкам", &["Точка", "Наличные", "Безнал"], n);
    let cities = g.l.sample(CITIES, n as usize);
    for (i, r) in t.rows().enumerate() {
        let c = cities.get(i % cities.len().max(1)).copied().unwrap_or("Точка");
        g.txt(&t, 0, r, &format!("{c} {}", i + 1));
        let cash = g.d.amount(5_000, 90_000, 100);
        let card = g.d.amount(10_000, 150_000, 100);
        g.num(&t, 1, r, cash);
        g.num(&t, 2, r, card);
    }
    let row = row_of(g, &t);
    let reference = format!("={}+{}", t.cell(1, row), t.cell(2, row));
    let name = format!("строки {row}");
    Ok(one_row(
        g,
        "Сложение",
        &t,
        "Итого",
        row,
        &reference,
        &format!("Сложи наличные и безнал для {name} знаком +."),
        &format!("Сколько всего выручки у точки в {name}?"),
    ))
}

pub fn sub(g: &mut Gen) -> Result<Task> {
    let n = g.rows();
    let t = g.table("Склад", &["Товар", "Приход", "Расход"], n);
    let items = g.l.sample(PRODUCTS, n as usize);
    for (i, r) in t.rows().enumerate() {
        g.txt(&t, 0, r, items.get(i).copied().unwrap_or("Товар"));
        let inn = g.d.int(50, 400) as f64;
        let out = g.d.int(10, inn as i64) as f64;
        g.num(&t, 1, r, inn);
        g.num(&t, 2, r, out);
    }
    let row = row_of(g, &t);
    let reference = format!("={}-{}", t.cell(1, row), t.cell(2, row));
    let item = g.setup.value_of(&t.cell(0, row)).map(|c| if let Cell::Text(s) = c { s.clone() } else { String::new() }).unwrap_or_default();
    Ok(one_row(
        g,
        "Вычитание",
        &t,
        "Остаток",
        row,
        &reference,
        &format!("Посчитай остаток «{item}»: приход минус расход."),
        &format!("Сколько «{item}» осталось на складе?"),
    ))
}

pub fn mul(g: &mut Gen) -> Result<Task> {
    let n = g.rows();
    let t = g.table("Счёт поставщика", &["Товар", "Цена", "Кол-во"], n);
    let items = g.l.sample(PRODUCTS, n as usize);
    for (i, r) in t.rows().enumerate() {
        g.txt(&t, 0, r, items.get(i).copied().unwrap_or("Товар"));
        let p = g.d.amount(20, 3000, 5);
        let q = g.d.int(1, 60) as f64;
        g.num(&t, 1, r, p);
        g.num(&t, 2, r, q);
    }
    let row = row_of(g, &t);
    let reference = format!("={}*{}", t.cell(1, row), t.cell(2, row));
    let item = text_at(g, &t.cell(0, row));
    Ok(one_row(
        g,
        "Умножение",
        &t,
        "Сумма",
        row,
        &reference,
        &format!("Умножь цену на количество для «{item}» (знак *)."),
        &format!("На какую сумму купили «{item}»?"),
    ))
}

fn text_at(g: &Gen, cell: &str) -> String {
    match g.setup.value_of(cell) {
        Some(Cell::Text(s)) => s.clone(),
        _ => String::new(),
    }
}

pub fn div(g: &mut Gen) -> Result<Task> {
    let n = g.rows();
    let t = g.table("Закупка", &["Товар", "Сумма", "Кол-во"], n);
    let items = g.l.sample(PRODUCTS, n as usize);
    for (i, r) in t.rows().enumerate() {
        g.txt(&t, 0, r, items.get(i).copied().unwrap_or("Товар"));
        let p = g.d.amount(20, 2000, 5);
        let q = g.d.int(2, 40) as f64;
        g.num(&t, 1, r, p * q);
        g.num(&t, 2, r, q);
    }
    let row = row_of(g, &t);
    let reference = format!("={}/{}", t.cell(1, row), t.cell(2, row));
    let item = text_at(g, &t.cell(0, row));
    Ok(one_row(
        g,
        "Деление",
        &t,
        "Цена за шт.",
        row,
        &reference,
        &format!("Раздели сумму на количество для «{item}» (знак /)."),
        &format!("Почём обошлась одна штука «{item}»?"),
    ))
}

pub fn vat(g: &mut Gen) -> Result<Task> {
    let n = g.rows();
    let t = g.table("Услуги", &["Услуга", "Сумма без НДС"], n);
    let names = [
        "Доставка",
        "Монтаж",
        "Консультация",
        "Аренда",
        "Ремонт",
        "Обучение",
        "Уборка",
        "Хранение",
        "Печать",
        "Проверка",
        "Настройка",
        "Перевозка",
        "Сборка",
        "Охрана",
        "Реклама",
        "Связь",
    ];
    for (i, r) in t.rows().enumerate() {
        g.txt(&t, 0, r, names.get(i).copied().unwrap_or("Услуга"));
        let v = g.d.amount(1_000, 90_000, 100);
        g.num(&t, 1, r, v);
    }
    let row = row_of(g, &t);
    let reference = format!("={}*1.22", t.cell(1, row));
    let item = text_at(g, &t.cell(0, row));
    Ok(one_row(
        g,
        "Сумма с НДС",
        &t,
        "С НДС 22%",
        row,
        &reference,
        &format!("Посчитай сумму «{item}» с НДС 22%: умножь на 1,22."),
        &format!("Сколько клиент заплатит за «{item}» вместе с НДС 22%?"),
    ))
}

pub fn fill(g: &mut Gen) -> Result<Task> {
    let n = g.rows();
    let t = g.table("Накладная", &["Товар", "Цена", "Кол-во"], n);
    let items = g.l.sample(PRODUCTS, n as usize);
    for (i, r) in t.rows().enumerate() {
        g.txt(&t, 0, r, items.get(i % items.len().max(1)).copied().unwrap_or("Товар"));
        let p = g.d.amount(20, 3000, 5);
        let q = g.d.int(1, 50) as f64;
        g.num(&t, 1, r, p);
        g.num(&t, 2, r, q);
    }
    let (c1, c2) = (t.col(1), t.col(2));
    Ok(g.column(
        "Протянуть вниз",
        &t.clone(),
        3,
        "Сумма",
        |r| format!("={}*{}", a1(r, c1), a1(r, c2)),
        "Посчитай сумму каждой строки (цена × кол-во) одной формулой, протянутой вниз.",
        "Посчитай суммы по всем строкам накладной.",
    ))
}

pub fn absolute(g: &mut Gen) -> Result<Task> {
    let n = g.rows();
    let variant = g.l.below(3);
    let (title, head, param, rate, colname, ask3) = match variant {
        0 => ("НДС к начислению", "Сумма без НДС", "Ставка НДС", 0.22, "НДС", "Начисли НДС по всем строкам по ставке из параметров."),
        1 => ("Пересчёт из долларов", "Сумма, $", "Курс $", g.d.int(80, 98) as f64, "Сумма, ₽", "Переведи все суммы в рубли по курсу из параметров."),
        _ => {
            ("Скидка постоянному клиенту", "Сумма", "Скидка", 0.05 * g.d.int(1, 4) as f64, "Скидка, ₽", "Посчитай скидку в рублях для каждой строки.")
        }
    };
    let t = g.table(title, &["Позиция", head], n);
    for r in t.rows() {
        let label = format!("Позиция {}", r - t.first + 1);
        g.txt(&t, 0, r, &label);
        let v = if variant == 1 { g.d.amount(100, 9_000, 10) } else { g.d.amount(1_000, 90_000, 100) };
        g.num(&t, 1, r, v);
    }
    let lc = t.right + 3;
    g.setup.text(a1(t.header, lc), param);
    g.setup.bold.push(a1(t.header, lc));
    g.setup.width(lc, 110.0);
    let rate_cell = a1(t.header + 1, lc);
    g.setup.input(rate_cell.clone(), Cell::Num(rate));
    let fixed = format!("${}${}", super::letter(lc), t.header + 1);
    let c1 = t.col(1);
    let ask2 = format!("Умножь каждую сумму на {param} из {rate_cell}. Адрес {rate_cell} закрепи знаками $ (клавиша F4), чтобы он не сдвигался.");
    let mut task = g.column("Закрепить $", &t, 2, colname, |r| format!("={}*{fixed}", a1(r, c1)), &ask2, ask3);
    task.hints.insert(1, format!("Без $ при протягивании {rate_cell} превратится в {} — там пусто.", a1(t.header + 2, lc)));
    Ok(task)
}

pub fn mixed(g: &mut Gen) -> Result<Task> {
    let n = g.rows().min(8);
    let k = if g.level <= 2 { 2 } else { 3 };
    let (header, left) = g.origin("Цены с наценкой");
    let t = Table { header, first: header + 1, last: header + n, left, right: left + 1 + k };
    g.setup.header(header, left, &["Товар", "Закупка"]);
    g.setup.width(left, 120.0);
    let coefs = [1.1, 1.25, 1.4, 1.5, 1.8];
    for j in 0..k {
        let c = coefs.get(g.l.below(coefs.len())).copied().unwrap_or(1.2) + 0.05 * j as f64;
        g.setup.input(t.cell(2 + j, header), Cell::Num((c * 100.0).round() / 100.0));
        g.setup.bold.push(t.cell(2 + j, header));
    }
    let items = g.l.sample(PRODUCTS, n as usize);
    for (i, r) in t.rows().enumerate() {
        g.txt(&t, 0, r, items.get(i).copied().unwrap_or("Товар"));
        let v = g.d.amount(50, 5000, 10);
        g.num(&t, 1, r, v);
    }
    let bc = super::letter(t.col(1));
    for r in t.rows() {
        for j in 0..k {
            let cc = super::letter(t.col(2 + j));
            let cell = t.cell(2 + j, r);
            g.setup.answer(cell.clone());
            g.formula_check(&cell, &format!("=${bc}{r}*{cc}${header}"));
        }
    }
    let first = t.cell(2, t.first);
    let last = t.cell(1 + k, t.last);
    let example = crate::sheet::ru(&format!("=${bc}{}*{}${header}", t.first, super::letter(t.col(2))));
    let prompt = match g.level {
        1 => format!("Введи в {first} формулу и протяни её вправо и вниз до {last}:"),
        2 => format!("Заполни {first}:{last}: закупка × коэффициент из шапки. Одна формула: столбец закупки закрепи $B, строку шапки — C$1."),
        _ => format!("Заполни таблицу цен {first}:{last} одной формулой, протянутой вправо и вниз."),
    };
    let mut task = g.task("Смешанные ссылки", prompt, Some(example), first);
    task.goal_seconds += 20.0;
    Ok(task)
}

/// A ledger of invoices: party | number | amount (+ distractor VAT column at level 4).
fn invoices(g: &mut Gen, title: &str, party_pool: usize) -> (Table, Vec<&'static str>) {
    let n = g.rows();
    let headers: &[&str] = if g.level >= 4 {
        &["Контрагент", "Счёт №", "Сумма", "в т.ч. НДС"]
    } else {
        &["Контрагент", "Счёт №", "Сумма"]
    };
    let t = g.table(title, headers, n);
    let parties = g.l.sample(PARTIES, party_pool);
    let must = parties.first().copied().unwrap_or("ООО Альфа");
    g.categories(&t, 0, &parties, must);
    for r in t.rows() {
        let no = 100 + g.l.int(0, 899);
        g.num(&t, 1, r, no as f64);
        let v = g.d.amount(3_000, 250_000, 50);
        g.num(&t, 2, r, v);
        if g.level >= 4 {
            g.num(&t, 3, r, (v * 22.0 / 122.0 * 100.0).round() / 100.0);
        }
    }
    (t, parties)
}

pub fn sum(g: &mut Gen) -> Result<Task> {
    let (t, _) = invoices(g, "Реестр счетов", 4);
    let cell = g.answer(&t, 1, "Итого:");
    let reference = format!("=SUM({})", t.range(2));
    Ok(g.single("СУММ", &cell, &reference, "Посчитай итог столбца «Сумма» функцией СУММ.", "Какова общая сумма всех счетов?"))
}

fn salaries(g: &mut Gen, title: &str) -> (Table, Vec<&'static str>) {
    let n = g.rows();
    let t = g.table(title, &["Сотрудник", "Отдел", "Оклад"], n);
    let depts = g.l.sample(DEPTS, 3);
    let must = depts.first().copied().unwrap_or("Склад");
    g.categories(&t, 1, &depts, must);
    for (i, r) in t.rows().enumerate() {
        let who = PEOPLE.get(i % PEOPLE.len()).copied().unwrap_or("Сотрудник");
        let who = if i >= PEOPLE.len() { format!("{who} ({})", i / PEOPLE.len() + 1) } else { who.to_string() };
        g.txt(&t, 0, r, &who);
        let v = g.d.amount(35_000, 160_000, 500);
        g.num(&t, 2, r, v);
    }
    (t, depts)
}

pub fn average(g: &mut Gen) -> Result<Task> {
    let (t, _) = salaries(g, "Штатное расписание");
    let cell = g.answer(&t, 1, "Средний оклад:");
    let reference = format!("=AVERAGE({})", t.range(2));
    Ok(g.single("СРЗНАЧ", &cell, &reference, "Найди средний оклад функцией СРЗНАЧ.", "Какой в компании средний оклад?"))
}

pub fn minmax(g: &mut Gen) -> Result<Task> {
    let max = g.l.chance(50);
    let (t, _) = invoices(g, "Платежи за месяц", 4);
    let (label, f, ask2, ask3) = if max {
        ("Максимум:", "MAX", "Найди самый крупный платёж функцией МАКС.", "Какой был самый крупный платёж?")
    } else {
        ("Минимум:", "MIN", "Найди самый маленький платёж функцией МИН.", "Какой был самый маленький платёж?")
    };
    let cell = g.answer(&t, 1, label);
    let reference = format!("={f}({})", t.range(2));
    Ok(g.single("МИН и МАКС", &cell, &reference, ask2, ask3))
}

pub fn count(g: &mut Gen) -> Result<Task> {
    let n = g.rows();
    let t = g.table("Счета к оплате", &["Контрагент", "Сумма", "Оплачено"], n);
    let parties = g.l.sample(PARTIES, 5);
    g.categories(&t, 0, &parties, parties.first().copied().unwrap_or("ООО Альфа"));
    let forced = t.first + g.d.below(t.n() as usize) as u32;
    for r in t.rows() {
        let v = g.d.amount(5_000, 120_000, 100);
        g.num(&t, 1, r, v);
        if r == forced || g.d.chance(55) {
            g.num(&t, 2, r, v);
        }
    }
    let paid = g.l.chance(50);
    let cell = g.answer(&t, 1, if paid { "Оплачено счетов:" } else { "Всего счетов:" });
    let (reference, ask2, ask3) = if paid {
        (format!("=COUNT({})", t.range(2)), "Посчитай, сколько ячеек в «Оплачено» заполнено числами (СЧЁТ).", "Сколько счетов уже оплачено?")
    } else {
        (format!("=COUNTA({})", t.range(0)), "Посчитай количество строк по столбцу «Контрагент» функцией СЧЁТЗ.", "Сколько всего счетов в реестре?")
    };
    Ok(g.single("СЧЁТ и СЧЁТЗ", &cell, &reference, ask2, ask3))
}

pub fn round(g: &mut Gen) -> Result<Task> {
    let n = g.rows();
    let t = g.table("Счета", &["Счёт", "Сумма без НДС"], n);
    for r in t.rows() {
        let no = 200 + g.l.int(0, 799);
        g.txt(&t, 0, r, &format!("№ {no}"));
        let v = g.d.int(100_000, 9_000_000) as f64 / 100.0 + 0.07;
        g.num(&t, 1, r, (v * 100.0).round() / 100.0);
    }
    let c1 = t.col(1);
    Ok(g.column(
        "ОКРУГЛ",
        &t,
        2,
        "НДС 22%",
        |r| format!("=ROUND({}*0.22,2)", a1(r, c1)),
        "Начисли НДС 22% и округли до копеек: ОКРУГЛ(сумма*0,22; 2).",
        "Начисли НДС 22% по каждому счёту, до копеек.",
    ))
}

pub fn if_(g: &mut Gen) -> Result<Task> {
    let n = g.rows();
    let t = g.table("Контроль оплат", &["Контрагент", "Сумма счёта", "Оплачено"], n);
    let parties = g.l.sample(PARTIES, 6);
    g.categories(&t, 0, &parties, parties.first().copied().unwrap_or("ООО Альфа"));
    for (i, r) in t.rows().enumerate() {
        let v = g.d.amount(5_000, 150_000, 100);
        g.num(&t, 1, r, v);
        let paid = if i == 0 || g.d.chance(50) { v } else { (v * g.d.int(0, 9) as f64 / 10.0).round() };
        g.num(&t, 2, r, paid);
    }
    let (b, c) = (t.col(1), t.col(2));
    Ok(g.column(
        "ЕСЛИ",
        &t,
        3,
        "Статус",
        |r| format!("=IF({}>={},\"Оплачено\",\"Долг\")", a1(r, c), a1(r, b)),
        "Через ЕСЛИ: если оплачено не меньше суммы — «Оплачено», иначе «Долг».",
        "Проставь статус каждого счёта: «Оплачено» или «Долг».",
    ))
}

pub fn and_or(g: &mut Gen) -> Result<Task> {
    let n = g.rows();
    let t = g.table("Премии менеджерам", &["Менеджер", "План", "Факт", "Опоздания"], n);
    for (i, r) in t.rows().enumerate() {
        g.txt(&t, 0, r, PEOPLE.get(i % PEOPLE.len()).copied().unwrap_or("Менеджер"));
        let plan = g.d.amount(300_000, 900_000, 10_000);
        let fact = (plan * g.d.int(70, 130) as f64 / 100.0).round();
        g.num(&t, 1, r, plan);
        g.num(&t, 2, r, fact);
        let late = if g.d.chance(60) { 0.0 } else { g.d.int(1, 4) as f64 };
        g.num(&t, 3, r, late);
    }
    let (b, c, d) = (t.col(1), t.col(2), t.col(3));
    if g.l.chance(50) {
        Ok(g.column(
            "И / ИЛИ",
            &t,
            4,
            "Премия",
            |r| format!("=IF(AND({}>={},{}=0),\"Да\",\"Нет\")", a1(r, c), a1(r, b), a1(r, d)),
            "Премия «Да», если факт ≥ плана И опозданий 0. Используй ЕСЛИ(И(…; …); \"Да\"; \"Нет\").",
            "Премию получают те, кто выполнил план и ни разу не опоздал. Проставь «Да»/«Нет».",
        ))
    } else {
        Ok(g.column(
            "И / ИЛИ",
            &t,
            4,
            "Беседа",
            |r| format!("=IF(OR({}<{},{}>2),\"Да\",\"Нет\")", a1(r, c), a1(r, b), a1(r, d)),
            "Беседа «Да», если факт < плана ИЛИ опозданий больше 2. Используй ЕСЛИ(ИЛИ(…; …); \"Да\"; \"Нет\").",
            "Руководитель хочет поговорить с теми, кто не выполнил план или опоздал больше двух раз. Отметь «Да»/«Нет».",
        ))
    }
}

pub fn iferror(g: &mut Gen) -> Result<Task> {
    let n = g.rows();
    let t = g.table("Себестоимость", &["Товар", "Затраты", "Выпущено, шт"], n);
    let items = g.l.sample(PRODUCTS, n as usize);
    let zero = t.first + g.d.below(t.n() as usize) as u32;
    for (i, r) in t.rows().enumerate() {
        g.txt(&t, 0, r, items.get(i % items.len().max(1)).copied().unwrap_or("Товар"));
        let cost = g_amount(g);
        g.num(&t, 1, r, cost);
        let q = if r == zero || g.d.chance(15) { 0.0 } else { g.d.int(5, 300) as f64 };
        g.num(&t, 2, r, q);
    }
    let (b, c) = (t.col(1), t.col(2));
    Ok(g.column(
        "ЕСЛИОШИБКА",
        &t,
        3,
        "На 1 шт",
        |r| format!("=IFERROR({}/{},0)", a1(r, b), a1(r, c)),
        "Раздели затраты на выпуск, а ошибку #ДЕЛ/0! замени нулём через ЕСЛИОШИБКА.",
        "Посчитай затраты на штуку. Где выпуска не было, должен стоять 0, а не ошибка.",
    ))
}

fn g_amount(g: &mut Gen) -> f64 {
    g.d.amount(10_000, 500_000, 100)
}

pub fn sumif(g: &mut Gen) -> Result<Task> {
    let (t, parties) = invoices(g, "Счета от поставщиков", 4);
    let who = parties.first().copied().unwrap_or("ООО Альфа");
    let crit = g.param(&t, 1, "Контрагент:", Cell::Text(who.into()));
    let cell = g.answer(&t, 2, "Сумма по нему:");
    let reference = format!("=SUMIF({},{crit},{})", t.range(0), t.range(2));
    Ok(g.single(
        "СУММЕСЛИ",
        &cell,
        &reference,
        &format!("Сложи суммы только по контрагенту из {crit} функцией СУММЕСЛИ."),
        &format!("Сколько всего мы должны заплатить контрагенту, указанному в {crit}?"),
    ))
}

pub fn countif(g: &mut Gen) -> Result<Task> {
    let (t, parties) = invoices(g, "Счета от поставщиков", 4);
    let who = parties.first().copied().unwrap_or("ООО Альфа");
    let crit = g.param(&t, 1, "Контрагент:", Cell::Text(who.into()));
    let cell = g.answer(&t, 2, "Счетов:");
    let reference = format!("=COUNTIF({},{crit})", t.range(0));
    Ok(g.single(
        "СЧЁТЕСЛИ",
        &cell,
        &reference,
        &format!("Посчитай число счетов контрагента из {crit} функцией СЧЁТЕСЛИ."),
        &format!("Сколько счетов выставил контрагент из {crit}?"),
    ))
}

pub fn compare(g: &mut Gen) -> Result<Task> {
    let (t, _) = invoices(g, "Крупные платежи", 5);
    let limit = g.l.amount(40_000, 120_000, 10_000);
    let crit = g.param(&t, 1, "Порог, ₽:", Cell::Num(limit));
    let sum = g.l.chance(50);
    let cell = g.answer(&t, 2, if sum { "Сумма крупных:" } else { "Число крупных:" });
    let reference = if sum { format!("=SUMIF({},\">\"&{crit})", t.range(2)) } else { format!("=COUNTIF({},\">\"&{crit})", t.range(2)) };
    let what = if sum { "Сложи" } else { "Посчитай" };
    Ok(g.single(
        "Условие «больше»",
        &cell,
        &reference,
        &format!("{what} платежи больше порога из {crit}. Условие: \">\"&{crit}."),
        &format!("{} платежей больше порога из {crit}?", if sum { "На какую сумму" } else { "Сколько" }),
    ))
}

pub fn sumifs(g: &mut Gen) -> Result<Task> {
    let n = g.rows();
    let t = g.table("Счета", &["Контрагент", "Статус", "Сумма"], n);
    let parties = g.l.sample(PARTIES, 3);
    let who = parties.first().copied().unwrap_or("ООО Альфа");
    let statuses = ["Оплачен", "Не оплачен"];
    let status = if g.l.chance(50) { "Оплачен" } else { "Не оплачен" };
    let forced = t.first + g.d.below(t.n() as usize) as u32;
    for r in t.rows() {
        let (p, s) = if r == forced { (who, status) } else { (g.d.pick(&parties), g.d.pick(&statuses)) };
        g.txt(&t, 0, r, p);
        g.txt(&t, 1, r, s);
        let v = g.d.amount(3_000, 200_000, 50);
        g.num(&t, 2, r, v);
    }
    let c1 = g.param(&t, 1, "Контрагент:", Cell::Text(who.into()));
    let c2 = g.param(&t, 2, "Статус:", Cell::Text(status.into()));
    let cell = g.answer(&t, 3, "Сумма:");
    let reference = format!("=SUMIFS({},{},{c1},{},{c2})", t.range(2), t.range(0), t.range(1));
    Ok(g.single(
        "СУММЕСЛИМН",
        &cell,
        &reference,
        &format!("СУММЕСЛИМН по двум условиям: контрагент из {c1} и статус из {c2}."),
        &format!("Какая сумма у счетов контрагента из {c1} со статусом из {c2}?"),
    ))
}

pub fn countifs(g: &mut Gen) -> Result<Task> {
    let n = g.rows();
    let t = g.table("Сделки", &["Менеджер", "Город", "Сумма"], n);
    let people = g.l.sample(PEOPLE, 3);
    let cities = g.l.sample(CITIES, 3);
    let who = people.first().copied().unwrap_or("Петров С.");
    let city = cities.first().copied().unwrap_or("Москва");
    let forced = t.first + g.d.below(t.n() as usize) as u32;
    for r in t.rows() {
        let (p, c) = if r == forced { (who, city) } else { (g.d.pick(&people), g.d.pick(&cities)) };
        g.txt(&t, 0, r, p);
        g.txt(&t, 1, r, c);
        let v = g.d.amount(10_000, 300_000, 1_000);
        g.num(&t, 2, r, v);
    }
    let c1 = g.param(&t, 1, "Менеджер:", Cell::Text(who.into()));
    let c2 = g.param(&t, 2, "Город:", Cell::Text(city.into()));
    let cell = g.answer(&t, 3, "Сделок:");
    let reference = format!("=COUNTIFS({},{c1},{},{c2})", t.range(0), t.range(1));
    Ok(g.single(
        "СЧЁТЕСЛИМН",
        &cell,
        &reference,
        &format!("СЧЁТЕСЛИМН: сделки менеджера из {c1} в городе из {c2}."),
        &format!("Сколько сделок закрыл менеджер из {c1} в городе из {c2}?"),
    ))
}

pub fn averageif(g: &mut Gen) -> Result<Task> {
    let (t, depts) = salaries(g, "Оклады по отделам");
    let dept = depts.first().copied().unwrap_or("Склад");
    let crit = g.param(&t, 1, "Отдел:", Cell::Text(dept.into()));
    let cell = g.answer(&t, 2, "Средний оклад:");
    let reference = format!("=AVERAGEIF({},{crit},{})", t.range(1), t.range(2));
    Ok(g.single(
        "СРЗНАЧЕСЛИ",
        &cell,
        &reference,
        &format!("Средний оклад отдела из {crit} — функция СРЗНАЧЕСЛИ."),
        &format!("Какой средний оклад в отделе из {crit}?"),
    ))
}

/// Price list: code | product | price. Codes and names follow the layout seed.
fn price_list(g: &mut Gen, code_first: bool) -> (Table, Vec<String>) {
    let n = g.rows();
    let headers: &[&str] = if code_first { &["Код", "Товар", "Цена"] } else { &["Товар", "Код", "Цена"] };
    let t = g.table("Справочник товаров", headers, n);
    let mut codes: Vec<String> = Vec::new();
    let items = g.l.sample(PRODUCTS, n as usize);
    for (i, r) in t.rows().enumerate() {
        let code = format!("А-{}", 1000 + g.l.int(1, 8999));
        let item = items.get(i % items.len().max(1)).copied().unwrap_or("Товар");
        let item = if i >= items.len() { format!("{item} {}", i / items.len().max(1) + 1) } else { item.to_string() };
        let (ci, ni) = if code_first { (0, 1) } else { (1, 0) };
        g.txt(&t, ci, r, &code);
        g.txt(&t, ni, r, &item);
        let p = g.d.amount(30, 5000, 5);
        g.num(&t, 2, r, p);
        codes.push(code);
    }
    (t, codes)
}

pub fn vlookup(g: &mut Gen) -> Result<Task> {
    let (t, codes) = price_list(g, true);
    let key = codes.get(g.l.below(codes.len())).cloned().unwrap_or_default();
    let k = g.param(&t, 1, "Код:", Cell::Text(key));
    let cell = g.answer(&t, 2, "Цена:");
    let table = format!("{}:{}", t.cell(0, t.first), t.cell(2, t.last));
    let reference = format!("=VLOOKUP({k},{table},3,FALSE)");
    Ok(g.single(
        "ВПР",
        &cell,
        &reference,
        &format!("Найди цену товара с кодом из {k} через ВПР: таблица {table}, столбец 3, ЛОЖЬ."),
        &format!("Сколько стоит товар с кодом из {k}?"),
    ))
}

pub fn xlookup(g: &mut Gen) -> Result<Task> {
    let (t, codes) = price_list(g, true);
    let key = codes.get(g.l.below(codes.len())).cloned().unwrap_or_default();
    let k = g.param(&t, 1, "Код:", Cell::Text(key));
    let want_name = g.l.chance(40);
    let cell = g.answer(&t, 2, if want_name { "Товар:" } else { "Цена:" });
    let reference = format!("=XLOOKUP({k},{},{})", t.range(0), t.range(if want_name { 1 } else { 2 }));
    let what = if want_name { "название" } else { "цену" };
    Ok(g.single(
        "ПРОСМОТРX",
        &cell,
        &reference,
        &format!("Найди {what} по коду из {k} через ПРОСМОТРX."),
        &format!("Какой {} у товара с кодом из {k}?", if want_name { "название" } else { "цена" }),
    ))
}

pub fn index_match(g: &mut Gen) -> Result<Task> {
    let (t, codes) = price_list(g, false);
    let key = codes.get(g.l.below(codes.len())).cloned().unwrap_or_default();
    let k = g.param(&t, 1, "Код:", Cell::Text(key));
    let cell = g.answer(&t, 2, "Цена:");
    let reference = format!("=INDEX({},MATCH({k},{},0))", t.range(2), t.range(1));
    Ok(g.single(
        "ИНДЕКС + ПОИСКПОЗ",
        &cell,
        &reference,
        &format!("Код стоит не в первом столбце, поэтому ВПР не подойдёт. ИНДЕКС(цены; ПОИСКПОЗ({k}; коды; 0))."),
        &format!("Найди цену по коду из {k}. Код — во втором столбце."),
    ))
}

pub fn lookup_missing(g: &mut Gen) -> Result<Task> {
    let (t, codes) = price_list(g, true);
    let m = if g.level <= 2 { 3 } else { 5 };
    let oc = t.right + 2;
    g.setup.header(t.header, oc, &["Заказ: код", "Цена"]);
    g.setup.width(oc, 120.0);
    g.setup.width(oc + 1, 110.0);
    let table = format!("${}${}:${}${}", super::letter(t.col(0)), t.first, super::letter(t.col(2)), t.last);
    let missing_at = g.l.below(m);
    for i in 0..m {
        let r = t.first + i as u32;
        let code =
            if i == missing_at { format!("Б-{}", 100 + g.l.int(0, 899)) } else { codes.get(g.l.below(codes.len())).cloned().unwrap_or_default() };
        g.setup.text(a1(r, oc), code);
        let cell = a1(r, oc + 1);
        g.setup.answer(cell.clone());
        g.formula_check(&cell, &format!("=IFERROR(VLOOKUP({},{table},3,FALSE),\"нет\")", a1(r, oc)));
    }
    let first = a1(t.first, oc + 1);
    let last = a1(t.first + m as u32 - 1, oc + 1);
    let example = crate::sheet::ru(&format!("=IFERROR(VLOOKUP({},{table},3,FALSE),\"нет\")", a1(t.first, oc)));
    let prompt = match g.level {
        1 => format!("Введи в {first} и протяни до {last}:"),
        2 => format!("Подтяни цены заказа из справочника (ВПР). Где кода нет — выведи «нет» через ЕСЛИОШИБКА. {first}:{last}."),
        _ => format!("Подтяни цены к заказу ({first}:{last}). Если товара нет в справочнике, напиши «нет»."),
    };
    let mut task = g.task("Если не найдено", prompt, Some(example), first);
    task.goal_seconds += 20.0;
    Ok(task)
}

pub fn trim(g: &mut Gen) -> Result<Task> {
    let n = g.rows();
    let t = g.table("Выгрузка из 1С", &["Контрагент (сырой)"], n);
    g.setup.width(t.col(0), 200.0);
    for r in t.rows() {
        let p = g.d.pick(PARTIES);
        let messy = match g.d.below(3) {
            0 => format!("  {p}"),
            1 => format!("{p}   "),
            _ => format!(" {}", p.replace(' ', "   ")),
        };
        g.txt(&t, 0, r, &messy);
    }
    let c0 = t.col(0);
    Ok(g.column(
        "СЖПРОБЕЛЫ",
        &t,
        1,
        "Контрагент",
        |r| format!("=TRIM({})", a1(r, c0)),
        "Очисти названия от лишних пробелов функцией СЖПРОБЕЛЫ.",
        "Названия из выгрузки с лишними пробелами — приведи их в порядок.",
    ))
}

pub fn join(g: &mut Gen) -> Result<Task> {
    let n = g.rows();
    let t = g.table("Сотрудники", &["Фамилия", "Имя"], n);
    let last = ["Иванова", "Петров", "Сидоров", "Кузнецова", "Смирнова", "Попов", "Волкова", "Морозов", "Лебедев", "Козлова"];
    let first = ["Анна", "Сергей", "Кирилл", "Мария", "Ольга", "Дмитрий", "Елена", "Илья", "Павел", "Ирина"];
    for r in t.rows() {
        let a = g.d.pick(&last);
        let b = g.d.pick(&first);
        g.txt(&t, 0, r, a);
        g.txt(&t, 1, r, b);
    }
    let (a, b) = (t.col(0), t.col(1));
    Ok(g.column(
        "Склеить текст",
        &t,
        2,
        "ФИО",
        |r| format!("={}&\" \"&{}", a1(r, a), a1(r, b)),
        "Склей фамилию, пробел и имя знаком &: A2&\" \"&B2.",
        "Собери «Фамилия Имя» в одну ячейку для каждого сотрудника.",
    ))
}

pub fn left_right(g: &mut Gen) -> Result<Task> {
    let n = g.rows();
    let t = g.table("Номера документов", &["Документ"], n);
    let pref = ["СФ", "ТН", "АК", "ПП"];
    for r in t.rows() {
        let p = g.d.pick(&pref);
        let no = g.d.int(1, 9999);
        g.txt(&t, 0, r, &format!("{p}-2026-{no:04}"));
    }
    let c0 = t.col(0);
    if g.l.chance(50) {
        Ok(g.column(
            "ЛЕВСИМВ / ПРАВСИМВ",
            &t,
            1,
            "Номер",
            |r| format!("=RIGHT({},4)", a1(r, c0)),
            "Номер — последние 4 символа: ПРАВСИМВ(документ; 4).",
            "Вытащи из кода документа его номер (последние 4 цифры).",
        ))
    } else {
        Ok(g.column(
            "ЛЕВСИМВ / ПРАВСИМВ",
            &t,
            1,
            "Тип",
            |r| format!("=LEFT({},2)", a1(r, c0)),
            "Тип — первые 2 символа: ЛЕВСИМВ(документ; 2).",
            "Вытащи из кода документа его тип (первые 2 буквы).",
        ))
    }
}

pub fn proper(g: &mut Gen) -> Result<Task> {
    let n = g.rows();
    let t = g.table("Клиенты", &["Имя из анкеты"], n);
    g.setup.width(t.col(0), 180.0);
    let names =
        ["анна иванова", "СЕРГЕЙ ПЕТРОВ", "мария кузнецова", "оЛЬГА смирнова", "дмитрий попов", "ЕЛЕНА ВОЛКОВА", "илья морозов", "павел лебедев"];
    for r in t.rows() {
        let v = g.d.pick(&names);
        g.txt(&t, 0, r, v);
    }
    let c0 = t.col(0);
    Ok(g.column(
        "Регистр",
        &t,
        1,
        "Имя",
        |r| format!("=PROPER({})", a1(r, c0)),
        "ПРОПНАЧ делает первую букву каждого слова заглавной.",
        "Приведи имена к виду «Анна Иванова».",
    ))
}

pub fn text_number(g: &mut Gen) -> Result<Task> {
    let n = g.rows();
    let t = g.table("Выгрузка из 1С", &["Контрагент", "Сумма (текст)"], n);
    let parties = g.l.sample(PARTIES, 4);
    g.categories(&t, 0, &parties, parties.first().copied().unwrap_or("ООО Альфа"));
    for r in t.rows() {
        let v = g.d.int(1_000, 990_000);
        g.txt(&t, 1, r, &num_text(v as f64));
    }
    let c1 = t.col(1);
    let mut task = g.column(
        "Текст → число",
        &t,
        2,
        "Сумма",
        |r| format!("=VALUE(SUBSTITUTE({}, \" \", \"\"))", a1(r, c1)),
        "Сумма записана текстом с пробелами. ЗНАЧЕН(ПОДСТАВИТЬ(ячейка; \" \"; \"\")) превратит её в число.",
        "СУММ не видит суммы из выгрузки — они текстом. Сделай из них настоящие числа.",
    );
    task.hints.push("Пробел внутри кавычек: \" \" — пробел, \"\" — пусто.".into());
    Ok(task)
}

fn dated(g: &mut Gen, title: &str, headers: &[&str]) -> Table {
    let n = g.rows();
    let t = g.table(title, headers, n);
    for r in t.rows() {
        let no = 300 + g.l.int(0, 699);
        g.txt(&t, 0, r, &format!("Счёт {no}"));
    }
    t
}

fn date_answers(g: &mut Gen, t: &Table, col: u32) {
    for r in t.rows() {
        g.setup.formats.push((t.cell(col, r), "dd.mm.yyyy".into()));
    }
}

pub fn due(g: &mut Gen) -> Result<Task> {
    let t = dated(g, "Сроки оплаты", &["Счёт", "Дата счёта", "Отсрочка, дн."]);
    for r in t.rows() {
        let d = g.some_date();
        g.day(&t, 1, r, d);
        let days = [5.0, 10.0, 14.0, 30.0, 45.0];
        let k = days.get(g.d.below(days.len())).copied().unwrap_or(30.0);
        g.num(&t, 2, r, k);
    }
    date_answers(g, &t, 3);
    let (b, c) = (t.col(1), t.col(2));
    Ok(g.column(
        "Срок оплаты",
        &t,
        3,
        "Срок оплаты",
        |r| format!("={}+{}", a1(r, b), a1(r, c)),
        "Срок = дата счёта + дни отсрочки.",
        "До какого числа нужно оплатить каждый счёт?",
    ))
}

pub fn date_diff(g: &mut Gen) -> Result<Task> {
    let t = dated(g, "Просрочка", &["Счёт", "Срок оплаты", "Оплачен"]);
    for r in t.rows() {
        let d = g.some_date();
        g.day(&t, 1, r, d);
        let late = g.d.int(1, 40) as f64;
        g.day(&t, 2, r, d + late);
    }
    let (b, c) = (t.col(1), t.col(2));
    let mut task = g.column(
        "Дни просрочки",
        &t,
        3,
        "Просрочка, дн.",
        |r| format!("={}-{}", a1(r, c), a1(r, b)),
        "Дни просрочки = дата оплаты − срок.",
        "На сколько дней опоздала каждая оплата?",
    );
    for r in t.rows() {
        task.setup.formats.push((t.cell(3, r), "0".into()));
    }
    Ok(task)
}

pub fn month(g: &mut Gen) -> Result<Task> {
    let t = dated(g, "Реестр", &["Счёт", "Дата"]);
    for r in t.rows() {
        let d = g.some_date();
        g.day(&t, 1, r, d);
    }
    let b = t.col(1);
    if g.l.chance(70) {
        Ok(g.column(
            "МЕСЯЦ",
            &t,
            2,
            "Месяц",
            |r| format!("=MONTH({})", a1(r, b)),
            "МЕСЯЦ(дата) — номер месяца от 1 до 12.",
            "Проставь номер месяца для каждого счёта (для сводки по месяцам).",
        ))
    } else {
        Ok(g.column("МЕСЯЦ и ГОД", &t, 2, "Год", |r| format!("=YEAR({})", a1(r, b)), "ГОД(дата) — год.", "Проставь год для каждого счёта."))
    }
}

pub fn eomonth(g: &mut Gen) -> Result<Task> {
    let t = dated(g, "Закрытие периода", &["Счёт", "Дата"]);
    for r in t.rows() {
        let d = g.some_date();
        g.day(&t, 1, r, d);
    }
    date_answers(g, &t, 2);
    let b = t.col(1);
    Ok(g.column(
        "КОНМЕСЯЦА",
        &t,
        2,
        "Конец месяца",
        |r| format!("=EOMONTH({},0)", a1(r, b)),
        "КОНМЕСЯЦА(дата; 0) — последний день месяца этой даты.",
        "Для каждого счёта укажи последний день его месяца (дата закрытия периода).",
    ))
}

pub fn sort(g: &mut Gen) -> Result<Task> {
    let (t, _) = invoices(g, "Реестр счетов", 5);
    let by_amount = g.l.chance(60);
    let (key, desc, text) = if by_amount {
        (t.col(2), true, "по сумме от большей к меньшей")
    } else {
        (t.col(0), false, "по контрагенту от А до Я")
    };
    g.check("Сортировка", Kind::Sorted { first: t.first, last: t.last, cols: (t.left, t.right), key, descending: desc });
    let prompt = match g.level {
        1 => format!("Встань в любую ячейку таблицы → вкладка «Данные» → «Сортировка». Отсортируй {text}."),
        _ => format!("Отсортируй реестр {text}."),
    };
    let mut task = g.task("Сортировка", prompt, None, t.cell(0, t.first));
    task.reorders = true;
    Ok(task)
}

pub fn filter(g: &mut Gen) -> Result<Task> {
    let (t, parties) = invoices(g, "Реестр счетов", 4);
    let who = parties.first().copied().unwrap_or("ООО Альфа").to_string();
    g.check("Фильтр", Kind::FilterShows { col: t.col(0), first: t.first, last: t.last, value: who.clone() });
    let prompt = match g.level {
        1 => format!("Нажми Ctrl+Shift+L, затем стрелку в заголовке «Контрагент» и оставь только «{who}»."),
        _ => format!("Оставь на экране только счета «{who}»."),
    };
    let mut task = g.task("Фильтр", prompt, None, t.cell(0, t.header));
    task.reorders = true;
    Ok(task)
}

pub fn freeze(g: &mut Gen) -> Result<Task> {
    let level = g.level;
    g.level = 1;
    let (t, _) = invoices(g, "Реестр", 6);
    g.level = level;
    for r in t.last + 1..=t.last + 40 {
        g.setup.input(a1(r, t.col(0)), Cell::Text(g.d.pick(PARTIES).into()));
        g.setup.input(a1(r, t.col(1)), Cell::Num((500 + r) as f64));
        g.setup.input(a1(r, t.col(2)), Cell::Num(g.d.amount(1000, 90_000, 100)));
    }
    g.check("Закрепление", Kind::Frozen { rows: t.header });
    let prompt =
        "Таблица длинная: закрепи шапку, чтобы она не уезжала при прокрутке (Вид → Закрепить области → Закрепить верхнюю строку).".to_string();
    Ok(g.task("Закрепить шапку", prompt, None, "A2".into()))
}

pub fn numfmt(g: &mut Gen) -> Result<Task> {
    let (t, _) = invoices(g, "Реестр счетов", 4);
    let range = t.range(2);
    g.check("Формат", Kind::NumFmt { range: range.clone(), contains: "#,##0".into() });
    let prompt = match g.level {
        1 => format!("Выдели {range}, нажми Ctrl+1 → «Числовой», поставь «Разделитель групп разрядов»."),
        _ => format!("Покажи суммы {range} с разделителем тысяч, как принято в отчётах."),
    };
    Ok(g.task("Формат чисел", prompt, None, t.cell(2, t.first)))
}

pub fn dedupe(g: &mut Gen) -> Result<Task> {
    let n = g.rows();
    let t = g.table("Справочник контрагентов", &["Контрагент", "ИНН"], n);
    let uniq = (n as usize * 2 / 3).max(3);
    let parties = g.l.sample(PARTIES, uniq.min(PARTIES.len()));
    let mut rows: Vec<(String, f64)> = parties.iter().map(|p| (p.to_string(), 7_700_000_000.0 + g.l.int(0, 99_999_999) as f64)).collect();
    let distinct = rows.len();
    while rows.len() < n as usize {
        let i = g.l.below(distinct);
        if let Some(r) = rows.get(i).cloned() {
            rows.push(r);
        }
    }
    g.l.shuffle(&mut rows);
    for (i, (p, inn)) in rows.iter().enumerate() {
        let r = t.first + i as u32;
        g.txt(&t, 0, r, p);
        g.num(&t, 1, r, *inn);
    }
    g.setup.width(t.col(1), 120.0);
    g.check("Дубликаты", Kind::Unique { col: t.col(0), first: t.first, last: t.last, expected: distinct });
    let prompt = match g.level {
        1 => "Встань в таблицу → «Данные» → «Удалить дубликаты» → ОК.".to_string(),
        _ => "В справочнике повторы. Оставь каждого контрагента по одному разу.".to_string(),
    };
    let mut task = g.task("Удалить дубликаты", prompt, None, t.cell(0, t.first));
    task.reorders = true;
    Ok(task)
}

pub fn pivot(g: &mut Gen) -> Result<Task> {
    let (t, parties) = invoices(g, "Счета за квартал", 3);
    let pairs: Vec<(String, String)> = parties.iter().map(|p| ((*p).to_string(), format!("=SUMIF({},\"{p}\",{})", t.range(0), t.range(2)))).collect();
    g.check("Сводка по контрагентам", Kind::Pairs { pairs });
    let prompt = match g.level {
        1 => "Встань в таблицу → «Вставка» → «Сводная таблица» → ОК. Перетащи «Контрагент» в строки, «Сумма» — в значения.".to_string(),
        2 => "Сделай сводку: сумма счетов по каждому контрагенту (сводной таблицей или СУММЕСЛИ рядом с таблицей).".to_string(),
        _ => "Руководитель просит итоги по каждому контрагенту за квартал. Подготовь сводку любым способом.".to_string(),
    };
    let mut task = g.task("Сводная таблица", prompt, None, t.cell(0, t.first));
    task.goal_seconds = 120.0;
    task.hints.push(format!("Без сводной: впиши имена контрагентов в столбец справа, а рядом =СУММЕСЛИ({};имя;{}).", t.abs(0), t.abs(2)));
    let _ = MONTHS;
    Ok(task)
}
