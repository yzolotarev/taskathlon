//! SQL drills on small generated databases (a shop, a staff list, flights).
//! The layout seed fixes the question (columns, thresholds); the data seed fixes the rows.

use super::{CITIES, Gen};
use crate::sheet::{Cell, a1, date, date_text};
use crate::sql::{SqlTable, SqlTask, SqlVal};
use crate::{Result, Task};
use gridcraft_tr::{t as tr, ts};

const NAMES: &[&str] = &[
    "Иванов",
    "Петрова",
    "Сидоров",
    "Кузнецова",
    "Смирнов",
    "Попова",
    "Волков",
    "Морозова",
    "Лебедев",
    "Козлова",
    "Новиков",
    "Соколова",
    "Павлов",
    "Семёнова",
    "Голубев",
    "Виноградова",
    "Богданов",
    "Орлова",
    "Фёдоров",
    "Зайцева",
];
const CATEGORIES: &[&str] = &["Канцтовары", "Техника", "Мебель", "Бумага", "Хозтовары"];
const STATUSES: &[&str] = &["paid", "paid", "paid", "new", "cancelled"];
const DEPTS: &[&str] = &["Бухгалтерия", "Продажи", "Склад", "ИТ"];
const DOMAINS: &[&str] = &["mail.ru", "yandex.ru", "gmail.com"];
const AIRPORTS: &[&str] = &["Шереметьево", "Пулково", "Внуково", "Кольцово", "Домодедово"];

fn iso(serial: f64) -> String {
    let t = date_text(serial);
    let p: Vec<&str> = t.split('.').collect();
    match (p.first(), p.get(1), p.get(2)) {
        (Some(d), Some(m), Some(y)) => format!("{y}-{m}-{d}"),
        _ => t,
    }
}

fn int(v: i64) -> SqlVal {
    SqlVal::Int(v)
}
fn text(v: &str) -> SqlVal {
    SqlVal::Text(v.into())
}
fn cols(spec: &[(&str, &str)]) -> Vec<(String, String)> {
    spec.iter().map(|(c, t)| (c.to_string(), t.to_string())).collect()
}

struct Shop {
    clients: SqlTable,
    orders: SqlTable,
}

/// Clients and orders. A couple of clients never order (for LEFT JOIN); amounts are distinct.
fn shop(g: &mut Gen, n_clients: usize, n_orders: usize) -> Shop {
    let mut names: Vec<&'static str> = NAMES.iter().copied().map(ts).collect();
    g.d.shuffle(&mut names);
    let mut clients = Vec::new();
    for i in 0..n_clients {
        let id = i as i64 + 1;
        let name = names.get(i % names.len()).copied().unwrap_or(ts("Иванов"));
        let city = g.d.pick(CITIES);
        let reg = iso(date(2025, 1, 15) + g.d.int(0, 400) as f64);
        clients.push(vec![int(id), text(name), text(city), text(&reg)]);
    }
    let silent: Vec<i64> = (0..2).map(|_| g.d.int(1, n_clients as i64)).collect();
    let buyers: Vec<i64> = (1..=n_clients as i64).filter(|c| !silent.contains(c)).collect();
    let mut orders = Vec::new();
    for i in 0..n_orders {
        let id = i as i64 + 1;
        let client = buyers.get(g.d.below(buyers.len().max(1))).copied().unwrap_or(1);
        let day = iso(date(2026, 1, 5) + g.d.int(0, 170) as f64);
        let cat = g.d.pick(CATEGORIES);
        let amount = g.d.int(5, 600) * 100 + id;
        let status = g.d.pick(STATUSES);
        orders.push(vec![int(id), int(client), text(&day), text(cat), int(amount), text(status)]);
    }
    Shop {
        clients: SqlTable {
            name: "clients".into(),
            columns: cols(&[("id", "INTEGER"), ("name", "TEXT"), ("city", "TEXT"), ("reg_date", "TEXT")]),
            rows: clients,
        },
        orders: SqlTable {
            name: "orders".into(),
            columns: cols(&[
                ("id", "INTEGER"),
                ("client_id", "INTEGER"),
                ("order_date", "TEXT"),
                ("category", "TEXT"),
                ("amount", "INTEGER"),
                ("status", "TEXT"),
            ]),
            rows: orders,
        },
    }
}

/// Staff with distinct salaries; some e-mails and the heads' manager_id are NULL.
fn staff(g: &mut Gen, n: usize) -> SqlTable {
    let mut names: Vec<&'static str> = NAMES.iter().copied().map(ts).collect();
    g.d.shuffle(&mut names);
    let mut rows = Vec::new();
    for i in 0..n {
        let id = i as i64 + 1;
        let name = names.get(i % names.len()).copied().unwrap_or(ts("Иванов"));
        let dept = DEPTS.get(i % DEPTS.len()).copied().map(ts).unwrap_or(ts("ИТ"));
        let salary = g.d.int(45, 190) * 1000 + id * 10;
        let hired = iso(date(2019, 1, 10) + g.d.int(0, 2500) as f64);
        let manager = if i < DEPTS.len() { SqlVal::Null } else { int((i % DEPTS.len()) as i64 + 1) };
        let email = if g.d.chance(25) { SqlVal::Null } else { SqlVal::Text(format!("user{id}@{}", g.d.pick(DOMAINS))) };
        rows.push(vec![int(id), text(name), text(dept), int(salary), text(&hired), manager, email]);
    }
    SqlTable {
        name: "employees".into(),
        columns: cols(&[
            ("id", "INTEGER"),
            ("name", "TEXT"),
            ("dept", "TEXT"),
            ("salary", "INTEGER"),
            ("hire_date", "TEXT"),
            ("manager_id", "INTEGER"),
            ("email", "TEXT"),
        ]),
        rows,
    }
}

/// Pilots, planes and flights, in the style of bank analyst interview tasks.
fn flights(g: &mut Gen, n: usize) -> Vec<SqlTable> {
    let mut names: Vec<&'static str> = NAMES.iter().copied().map(ts).collect();
    g.d.shuffle(&mut names);
    let pilots: Vec<Vec<SqlVal>> =
        (0..10).map(|i| vec![int(i + 1), text(names.get(i as usize).copied().unwrap_or(ts("Иванов"))), int(g.d.int(26, 60))]).collect();
    let planes: Vec<Vec<SqlVal>> = (0..6)
        .map(|i| {
            let cargo = i64::from(i >= 4);
            vec![
                int(i + 1),
                text(if cargo == 1 { ts("Ан-124") } else { ["SSJ-100", "A320", "B737", "MC-21"][i as usize % 4] }),
                int(if cargo == 1 { 0 } else { g.d.int(9, 22) * 10 }),
                int(cargo),
            ]
        })
        .collect();
    let mut fl = Vec::new();
    for i in 0..n {
        let first = g.d.int(1, 10);
        let second = (first + g.d.int(0, 8)) % 10 + 1;
        let day = iso(date(2026, 6, 1) + g.d.int(0, 120) as f64);
        fl.push(vec![int(i as i64 + 1), text(&day), int(g.d.int(1, 6)), int(first), int(second), text(g.d.pick(AIRPORTS))]);
    }
    vec![
        SqlTable { name: "pilots".into(), columns: cols(&[("pilot_id", "INTEGER"), ("name", "TEXT"), ("age", "INTEGER")]), rows: pilots },
        SqlTable {
            name: "planes".into(),
            columns: cols(&[("plane_id", "INTEGER"), ("model", "TEXT"), ("capacity", "INTEGER"), ("cargo_flg", "INTEGER")]),
            rows: planes,
        },
        SqlTable {
            name: "flights".into(),
            columns: cols(&[
                ("flight_id", "INTEGER"),
                ("flight_dt", "TEXT"),
                ("plane_id", "INTEGER"),
                ("first_pilot_id", "INTEGER"),
                ("second_pilot_id", "INTEGER"),
                ("destination", "TEXT"),
            ]),
            rows: fl,
        },
    ]
}

fn sizes(g: &Gen) -> (usize, usize) {
    match g.level {
        1 => (6, 10),
        2 => (8, 14),
        3 => (10, 20),
        _ => (12, 30),
    }
}

/// Shows the tables on the sheet (name, header, rows) side by side, read-only reference.
fn show_tables(g: &mut Gen, tables: &[SqlTable]) {
    let mut col = 0u32;
    for t in tables {
        g.setup.text(a1(1, col), tr(&format!("Таблица {}", t.name)));
        g.setup.bold.push(a1(1, col));
        let headers: Vec<&str> = t.columns.iter().map(|(c, _)| c.as_str()).collect();
        g.setup.header(2, col, &headers);
        for (i, (c, _)) in t.columns.iter().enumerate() {
            let w = if c.contains("date") || c.contains("dt") || c == "email" || c == "name" || c == "destination" { 120.0 } else { 80.0 };
            g.setup.width(col + i as u32, w);
        }
        for (r, row) in t.rows.iter().enumerate() {
            for (i, v) in row.iter().enumerate() {
                let cell = a1(3 + r as u32, col + i as u32);
                let value = match v {
                    SqlVal::Null => Cell::Text("NULL".into()),
                    SqlVal::Int(x) => Cell::Num(*x as f64),
                    SqlVal::Real(x) => Cell::Num(*x),
                    SqlVal::Text(s) => Cell::Text(s.clone()),
                };
                g.setup.put(cell, value, false);
            }
        }
        col += t.columns.len() as u32 + 1;
    }
}

/// Builds the drill. `ask2` names the construct; `ask3` is the business question.
fn finish(g: &mut Gen, title: &str, tables: Vec<SqlTable>, reference: &str, ordered: bool, ask2: &str, ask3: &str) -> Result<Task> {
    show_tables(g, &tables);
    let prompt = match g.level {
        1 => ts("Набери запрос и нажми Ctrl+Enter (или «Выполнить»):").to_string(),
        2 => ask2.to_string(),
        _ => ask3.to_string(),
    };
    let mut t = g.task(title, prompt, Some(reference.to_string()), "A1".into());
    t.sql = Some(SqlTask { tables, reference: reference.into(), ordered });
    t.goal_seconds = 25.0 + 20.0 * f32::from(g.level);
    Ok(t)
}

pub fn select(g: &mut Gen) -> Result<Task> {
    let (nc, no) = sizes(g);
    let s = shop(g, nc, no);
    let (q, a2, a3) = match g.l.below(3) {
        0 => ("SELECT name, city FROM clients", ts("Выбери из clients столбцы name и city."), ts("Нужен список клиентов: фамилия и город.")),
        1 => ("SELECT id, amount FROM orders", ts("Выбери из orders столбцы id и amount."), ts("Покажи номера всех заказов и их суммы.")),
        _ => (
            "SELECT name, reg_date FROM clients",
            ts("Выбери из clients столбцы name и reg_date."),
            ts("Когда регистрировался каждый клиент? Фамилия и дата."),
        ),
    };
    finish(g, "SELECT", vec![s.clients, s.orders], q, false, a2, a3)
}

pub fn where_(g: &mut Gen) -> Result<Task> {
    let (nc, no) = sizes(g);
    let s = shop(g, nc, no);
    let x = g.l.int(1, 3) * 10_000;
    let city = g.l.pick(CITIES);
    let (q, a2, a3) = match (g.level >= 4, g.l.below(2)) {
        (true, _) => (
            format!("SELECT id, amount FROM orders WHERE amount > {x} AND status = 'paid'"),
            tr(&format!("Заказы с amount > {x} и status = 'paid' (WHERE … AND …): id и amount.")),
            tr(&format!("Какие оплаченные заказы дороже {x}? Номер и сумма.")),
        ),
        (false, 0) => (
            format!("SELECT id, amount FROM orders WHERE amount > {x}"),
            tr(&format!("Отбери заказы с amount больше {x} (WHERE): id и amount.")),
            tr(&format!("Какие заказы дороже {x} ₽? Номер и сумма.")),
        ),
        _ => (
            format!("SELECT name FROM clients WHERE city = '{city}'"),
            tr(&format!("Отбери клиентов, у которых city = '{city}' (WHERE, текст в одинарных кавычках): name.")),
            tr(&format!("Кто из клиентов из города {city}? Только фамилии.")),
        ),
    };
    finish(g, "WHERE", vec![s.clients, s.orders], &q, false, &a2, &a3)
}

pub fn order(g: &mut Gen) -> Result<Task> {
    let (nc, no) = sizes(g);
    let s = shop(g, nc, no);
    let k = g.l.int(3, 5);
    let (q, a2, a3) = if g.level >= 4 {
        (
            format!("SELECT id, amount FROM orders WHERE status = 'paid' ORDER BY amount DESC LIMIT {k}"),
            tr(&format!("Топ-{k} оплаченных заказов по сумме: WHERE, ORDER BY amount DESC, LIMIT {k}. Столбцы id, amount.")),
            tr(&format!("Назови {k} самых крупных оплаченных заказа (от большего к меньшему): номер и сумма.")),
        )
    } else if g.l.chance(50) {
        (
            format!("SELECT id, amount FROM orders ORDER BY amount DESC LIMIT {k}"),
            tr(&format!("Отсортируй заказы по amount по убыванию и оставь {k} (ORDER BY … DESC LIMIT {k}). Столбцы id, amount.")),
            tr(&format!("Какие {k} заказа самые крупные? От большего к меньшему: номер и сумма.")),
        )
    } else {
        (
            format!("SELECT id, amount FROM orders ORDER BY amount LIMIT {k}"),
            tr(&format!("Отсортируй заказы по amount по возрастанию и оставь {k} (ORDER BY … LIMIT {k}). Столбцы id, amount.")),
            tr(&format!("Какие {k} заказа самые мелкие? От меньшего к большему: номер и сумма.")),
        )
    };
    finish(g, ts("ORDER BY и LIMIT"), vec![s.clients, s.orders], &q, true, &a2, &a3)
}

pub fn like(g: &mut Gen) -> Result<Task> {
    let n = sizes(g).1.min(16);
    let e = staff(g, n);
    let domain = g.l.pick(DOMAINS);
    let lo = g.l.int(6, 9) * 10_000;
    let hi = lo + 50_000;
    let (q, a2, a3) = match g.l.below(3) {
        0 => (
            format!("SELECT name, email FROM employees WHERE email LIKE '%@{domain}'"),
            tr(&format!("Найди сотрудников с почтой на {domain}: WHERE email LIKE '%@{domain}'. Столбцы name, email.")),
            tr(&format!("У кого почта на {domain}? Фамилия и почта.")),
        ),
        1 => (
            format!("SELECT name, salary FROM employees WHERE salary BETWEEN {lo} AND {hi}"),
            tr(&format!("Оклад от {lo} до {hi} включительно: WHERE salary BETWEEN {lo} AND {hi}. Столбцы name, salary.")),
            tr(&format!("У кого оклад в вилке {lo}–{hi} ₽ (включая границы)? Фамилия и оклад.")),
        ),
        _ => (
            ts("SELECT name, dept FROM employees WHERE dept IN ('Бухгалтерия', 'Склад')").to_string(),
            ts("Сотрудники отделов Бухгалтерия и Склад: WHERE dept IN ('Бухгалтерия', 'Склад'). Столбцы name, dept.").to_string(),
            ts("Кто работает в бухгалтерии или на складе? Фамилия и отдел.").to_string(),
        ),
    };
    finish(g, "LIKE, BETWEEN, IN", vec![e], &q, false, &a2, &a3)
}

pub fn null(g: &mut Gen) -> Result<Task> {
    let n = sizes(g).1.min(16);
    let e = staff(g, n);
    let (q, a2, a3) = match (g.level >= 3, g.l.below(2)) {
        (true, 0) => (
            ts("SELECT name, COALESCE(email, 'нет почты') FROM employees").to_string(),
            ts("Выведи name и почту, а вместо NULL — текст 'нет почты' (COALESCE).").to_string(),
            ts("Список для рассылки: фамилия и почта; где почты нет, напиши 'нет почты'.").to_string(),
        ),
        (_, 0) => (
            "SELECT name FROM employees WHERE email IS NULL".to_string(),
            ts("У кого не заполнена почта? WHERE email IS NULL (не = NULL!). Столбец name.").to_string(),
            ts("Кадровику нужны сотрудники без e-mail. Только фамилии.").to_string(),
        ),
        _ => (
            "SELECT name, dept FROM employees WHERE manager_id IS NULL".to_string(),
            ts("Кто без руководителя? WHERE manager_id IS NULL. Столбцы name, dept.").to_string(),
            ts("Найди руководителей отделов — у них нет своего руководителя. Фамилия и отдел.").to_string(),
        ),
    };
    finish(g, "NULL", vec![e], &q, false, &a2, &a3)
}

pub fn agg(g: &mut Gen) -> Result<Task> {
    let (nc, no) = sizes(g);
    let s = shop(g, nc, no);
    let (q, a2, a3) = match (g.level >= 4, g.l.below(3)) {
        (true, _) => (
            "SELECT COUNT(*), SUM(amount), ROUND(AVG(amount), 2) FROM orders WHERE status = 'paid'".to_string(),
            ts("По оплаченным заказам: количество, сумма и средний чек (COUNT, SUM, ROUND(AVG(…), 2)).").to_string(),
            ts("Сводка по оплаченным заказам в одну строку: сколько их, на какую сумму и средний чек (до копеек).").to_string(),
        ),
        (_, 0) => (
            "SELECT SUM(amount) FROM orders".to_string(),
            ts("Сумма всех заказов: SELECT SUM(amount) FROM orders.").to_string(),
            ts("На какую сумму всего заказов?").to_string(),
        ),
        (_, 1) => (
            "SELECT COUNT(*) FROM orders WHERE status = 'cancelled'".to_string(),
            ts("Сколько заказов со status = 'cancelled'? COUNT(*) и WHERE.").to_string(),
            ts("Сколько заказов отменили?").to_string(),
        ),
        _ => (
            "SELECT MAX(amount) FROM orders WHERE status = 'paid'".to_string(),
            ts("Самый крупный оплаченный заказ: MAX(amount) с WHERE status = 'paid'.").to_string(),
            ts("Какой самый крупный оплаченный заказ (сумма)?").to_string(),
        ),
    };
    finish(g, "COUNT, SUM, AVG", vec![s.clients, s.orders], &q, false, &a2, &a3)
}

pub fn distinct(g: &mut Gen) -> Result<Task> {
    let (nc, no) = sizes(g);
    let s = shop(g, nc, no);
    let (q, a2, a3) = if g.l.chance(50) {
        (
            "SELECT COUNT(DISTINCT client_id) FROM orders",
            ts("Сколько разных клиентов делали заказы? COUNT(DISTINCT client_id)."),
            ts("Сколько уникальных покупателей в таблице заказов?"),
        )
    } else {
        (
            "SELECT DISTINCT city FROM clients",
            ts("Список городов без повторов: SELECT DISTINCT city."),
            ts("Из каких городов наши клиенты? Каждый город один раз."),
        )
    };
    finish(g, "DISTINCT", vec![s.clients, s.orders], q, false, a2, a3)
}

pub fn group(g: &mut Gen) -> Result<Task> {
    let (nc, no) = sizes(g);
    let s = shop(g, nc, no);
    let (q, a2, a3) = match (g.level >= 4, g.l.below(3)) {
        (true, _) => (
            "SELECT category, COUNT(*), SUM(amount) FROM orders WHERE status = 'paid' GROUP BY category".to_string(),
            ts("По каждой категории оплаченных заказов: количество и сумма (WHERE, затем GROUP BY category).").to_string(),
            ts("Отчёт по категориям, только оплаченные: категория, число заказов, выручка.").to_string(),
        ),
        (_, 0) => (
            "SELECT category, SUM(amount) FROM orders GROUP BY category".to_string(),
            ts("Сумма заказов по каждой категории: GROUP BY category. Столбцы category, SUM(amount).").to_string(),
            ts("Выручка по категориям: категория и сумма.").to_string(),
        ),
        (_, 1) => (
            "SELECT city, COUNT(*) FROM clients GROUP BY city".to_string(),
            ts("Сколько клиентов в каждом городе: GROUP BY city, COUNT(*).").to_string(),
            ts("Сколько у нас клиентов в каждом городе?").to_string(),
        ),
        _ => (
            "SELECT status, COUNT(*) FROM orders GROUP BY status".to_string(),
            ts("Сколько заказов в каждом статусе: GROUP BY status, COUNT(*).").to_string(),
            ts("Разбей заказы по статусам: статус и количество.").to_string(),
        ),
    };
    finish(g, "GROUP BY", vec![s.clients, s.orders], &q, false, &a2, &a3)
}

pub fn having(g: &mut Gen) -> Result<Task> {
    let (nc, no) = sizes(g);
    let s = shop(g, nc, no);
    let k = if g.level >= 3 { 3 } else { 2 };
    let x = g.l.int(4, 8) * 10_000;
    let (q, a2, a3) = if g.l.chance(50) {
        (
            format!("SELECT client_id, COUNT(*) FROM orders GROUP BY client_id HAVING COUNT(*) >= {k}"),
            tr(&format!("Клиенты с {k} и более заказами: GROUP BY client_id, затем HAVING COUNT(*) >= {k}.")),
            tr(&format!("Кто из клиентов заказывал хотя бы {k} раза? client_id и число заказов.")),
        )
    } else {
        (
            format!("SELECT client_id, SUM(amount) FROM orders GROUP BY client_id HAVING SUM(amount) > {x}"),
            tr(&format!("Клиенты с суммой заказов больше {x}: GROUP BY client_id, HAVING SUM(amount) > {x}.")),
            tr(&format!("Кто купил у нас больше чем на {x} ₽ в сумме? client_id и сумма.")),
        )
    };
    finish(g, "HAVING", vec![s.clients, s.orders], &q, false, &a2, &a3)
}

pub fn dates(g: &mut Gen) -> Result<Task> {
    let (nc, no) = sizes(g);
    let s = shop(g, nc, no);
    let m = g.l.int(2, 5);
    let last = [31, 28, 31, 30, 31, 30].get(m as usize - 1).copied().unwrap_or(30);
    let month = ["", ts("январь"), ts("февраль"), ts("март"), ts("апрель"), ts("май"), ts("июнь")].get(m as usize).copied().unwrap_or("");
    let (q, a2, a3) = if g.level >= 3 && g.l.chance(50) {
        (
            "SELECT strftime('%m', order_date), SUM(amount) FROM orders GROUP BY strftime('%m', order_date)".to_string(),
            ts("Выручка по месяцам: GROUP BY strftime('%m', order_date) (в SQLite месяц достают так).").to_string(),
            ts("Помесячная выручка: номер месяца и сумма. (SQLite: strftime('%m', дата).)").to_string(),
        )
    } else {
        (
            format!("SELECT COUNT(*), SUM(amount) FROM orders WHERE order_date BETWEEN '2026-{m:02}-01' AND '2026-{m:02}-{last}'"),
            tr(&format!("Заказы за {month} 2026: WHERE order_date BETWEEN '2026-{m:02}-01' AND '2026-{m:02}-{last}'. COUNT(*) и SUM(amount).")),
            tr(&format!("Сколько заказов было за {month} 2026 и на какую сумму? Даты хранятся как 'ГГГГ-ММ-ДД'.")),
        )
    };
    finish(g, ts("Даты"), vec![s.clients, s.orders], &q, false, &a2, &a3)
}

pub fn join(g: &mut Gen) -> Result<Task> {
    let (nc, no) = sizes(g);
    let s = shop(g, nc, no);
    let x = g.l.int(1, 3) * 10_000;
    let (q, a2, a3) = if g.level >= 3 && g.l.chance(50) {
        (
            "SELECT c.name, SUM(o.amount) FROM orders o JOIN clients c ON c.id = o.client_id GROUP BY c.name".to_string(),
            ts("Сумма заказов по фамилии клиента: JOIN clients ON c.id = o.client_id, затем GROUP BY c.name.").to_string(),
            ts("Сколько купил каждый клиент? Фамилия и сумма (фамилии — в clients, суммы — в orders).").to_string(),
        )
    } else {
        (
            format!("SELECT c.name, o.amount FROM orders o JOIN clients c ON c.id = o.client_id WHERE o.amount > {x}"),
            tr(&format!("Соедини orders и clients (JOIN … ON c.id = o.client_id) и покажи name и amount для заказов > {x}.")),
            tr(&format!("Чьи заказы больше {x} ₽? Фамилия клиента и сумма заказа.")),
        )
    };
    finish(g, "JOIN", vec![s.clients, s.orders], &q, false, &a2, &a3)
}

pub fn left(g: &mut Gen) -> Result<Task> {
    let (nc, no) = sizes(g);
    let s = shop(g, nc, no);
    let (q, a2, a3) = if g.level >= 3 && g.l.chance(50) {
        (
            "SELECT c.name, COUNT(o.id) FROM clients c LEFT JOIN orders o ON o.client_id = c.id GROUP BY c.id, c.name",
            ts("Число заказов у каждого клиента, включая тех, у кого 0: LEFT JOIN и COUNT(o.id)."),
            ts("Для каждого клиента — сколько у него заказов (у кого нет — 0)."),
        )
    } else {
        (
            "SELECT c.name FROM clients c LEFT JOIN orders o ON o.client_id = c.id WHERE o.id IS NULL",
            ts("Клиенты без заказов: clients LEFT JOIN orders, затем WHERE o.id IS NULL."),
            ts("Кто из клиентов ещё ни разу не заказывал? Фамилии."),
        )
    };
    finish(g, "LEFT JOIN", vec![s.clients, s.orders], q, false, a2, a3)
}

pub fn case(g: &mut Gen) -> Result<Task> {
    let (nc, no) = sizes(g);
    let s = shop(g, nc, no);
    let x = g.l.int(2, 4) * 10_000;
    let q = tr(&format!("SELECT id, amount, CASE WHEN amount >= {x} THEN 'крупный' ELSE 'обычный' END FROM orders"));
    let a2 = tr(&format!("Добавь третий столбец: 'крупный', если amount >= {x}, иначе 'обычный' (CASE WHEN … THEN … ELSE … END)."));
    let a3 = tr(&format!("Размети заказы: id, сумма и метка 'крупный' (от {x} ₽) или 'обычный'."));
    finish(g, "CASE", vec![s.clients, s.orders], &q, false, &a2, &a3)
}

pub fn sub(g: &mut Gen) -> Result<Task> {
    let (nc, no) = sizes(g);
    let (q, a2, a3, tables) = match g.l.below(3) {
        0 => {
            let s = shop(g, nc, no);
            (
                "SELECT id, amount FROM orders WHERE amount > (SELECT AVG(amount) FROM orders)",
                ts("Заказы дороже среднего: WHERE amount > (SELECT AVG(amount) FROM orders). Столбцы id, amount."),
                ts("Какие заказы дороже среднего чека? Номер и сумма."),
                vec![s.clients, s.orders],
            )
        }
        1 => {
            let s = shop(g, nc, no);
            (
                "SELECT name FROM clients WHERE id IN (SELECT client_id FROM orders WHERE status = 'cancelled')",
                ts("Клиенты с отменёнными заказами: WHERE id IN (SELECT client_id FROM orders WHERE status = 'cancelled')."),
                ts("У кого из клиентов были отмены? Фамилии."),
                vec![s.clients, s.orders],
            )
        }
        _ => {
            let e = staff(g, no.min(16));
            (
                "SELECT name, salary FROM employees WHERE salary > (SELECT AVG(salary) FROM employees)",
                ts("Оклад выше среднего по компании: подзапрос (SELECT AVG(salary) FROM employees)."),
                ts("У кого оклад выше среднего по компании? Фамилия и оклад."),
                vec![e],
            )
        }
    };
    finish(g, ts("Подзапрос"), tables, q, false, a2, a3)
}

pub fn window(g: &mut Gen) -> Result<Task> {
    let n = sizes(g).1.min(16);
    let (q, a2, a3, tables, ordered) = match (g.level >= 3, g.l.below(2)) {
        (true, 0) => (
            "SELECT dept, name, salary FROM (SELECT dept, name, salary, ROW_NUMBER() OVER (PARTITION BY dept ORDER BY salary DESC) AS rn FROM employees) WHERE rn = 1",
            ts("Самый высокий оклад в каждом отделе: ROW_NUMBER() OVER (PARTITION BY dept ORDER BY salary DESC) в подзапросе, потом rn = 1."),
            ts("Кто получает больше всех в своём отделе? Отдел, фамилия, оклад."),
            vec![staff(g, n)],
            false,
        ),
        (_, 0) => (
            "SELECT name, dept, salary, RANK() OVER (PARTITION BY dept ORDER BY salary DESC) FROM employees",
            ts("Место по окладу внутри отдела: RANK() OVER (PARTITION BY dept ORDER BY salary DESC)."),
            ts("Для каждого сотрудника — его место по окладу в своём отделе (1 — самый высокий)."),
            vec![staff(g, n)],
            false,
        ),
        _ => {
            let s = shop(g, 6, n);
            (
                "SELECT id, amount, SUM(amount) OVER (ORDER BY id) FROM orders ORDER BY id",
                ts("Нарастающий итог по номеру заказа: SUM(amount) OVER (ORDER BY id), отсортируй по id."),
                ts("Покажи нарастающую выручку: номер заказа, сумма и накопленный итог (по порядку номеров)."),
                vec![s.orders],
                true,
            )
        }
    };
    finish(g, ts("Оконные функции"), tables, q, ordered, a2, a3)
}

pub fn interview(g: &mut Gen) -> Result<Task> {
    let n = 20 + 10 * usize::from(g.level);
    let tables = flights(g, n);
    let airport = g.l.pick(AIRPORTS);
    let age = g.l.int(4, 5) * 10 - 5;
    let k = 2;
    let (q, a2, a3): (String, String, String) = match g.l.below(3) {
        0 => (
            format!(
                "SELECT p.name FROM pilots p WHERE p.pilot_id IN (SELECT second_pilot_id FROM flights WHERE destination = '{airport}' AND flight_dt BETWEEN '2026-08-01' AND '2026-08-31' GROUP BY second_pilot_id HAVING COUNT(*) >= {k})"
            ),
            tr(&format!("Пилоты, которые вторым пилотом летали в {airport} в августе 2026 не меньше {k} раз: подзапрос с GROUP BY second_pilot_id и HAVING.")),
            tr(&format!("Выведи имена пилотов, которые в августе 2026 летали в {airport} вторым пилотом {k} раза и больше.")),
        ),
        1 => (
            format!(
                "SELECT DISTINCT p.name FROM flights f JOIN planes pl ON pl.plane_id = f.plane_id JOIN pilots p ON p.pilot_id = f.first_pilot_id WHERE pl.cargo_flg = 1 AND p.age > {age}"
            ),
            tr(&format!("Капитаны (first_pilot_id) старше {age} лет, летавшие на грузовых самолётах (cargo_flg = 1): два JOIN и DISTINCT.")),
            tr(&format!("Какие командиры воздушного судна старше {age} лет хоть раз летали на грузовом самолёте? Имена без повторов.")),
        ),
        _ => (
            "SELECT p.name, SUM(pl.capacity) FROM flights f JOIN planes pl ON pl.plane_id = f.plane_id JOIN pilots p ON p.pilot_id = f.first_pilot_id WHERE pl.cargo_flg = 0 GROUP BY p.pilot_id, p.name".into(),
            ts("Сколько мест перевёз каждый капитан на пассажирских рейсах (cargo_flg = 0): два JOIN, GROUP BY, SUM(capacity).").into(),
            ts("Для каждого командира — сколько пассажирских мест он перевёз за всё время (сумма capacity пассажирских рейсов).").into(),
        ),
    };
    let ordered = q.contains("LIMIT");
    finish(g, ts("Задача с собеседования"), tables, &q, ordered, &a2, &a3)
}
