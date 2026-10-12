//! Keyboard drills: the outcome is checked on the sheet and the keystroke is required.

use super::{Gen, PARTIES};
use crate::check::Kind;
use crate::sheet::{Cell, a1};
use crate::{Result, Task};
use gridcraft_tr::{t as tr, ts};

fn key(g: &mut Gen, combo: &str) {
    g.check(combo, Kind::Keys { combo: combo.into() });
}

pub fn autosum(g: &mut Gen) -> Result<Task> {
    let n = g.rows();
    let t = g.table(ts("Платежи"), &[ts("Контрагент"), ts("Сумма")], n);
    for r in t.rows() {
        let p = g.d.pick(PARTIES);
        g.txt(&t, 0, r, p);
        let v = g.d.amount(1_000, 90_000, 100);
        g.num(&t, 1, r, v);
    }
    let cell = t.cell(1, t.last + 1);
    g.setup.text(t.cell(0, t.last + 1), ts("Итого"));
    g.setup.bold.push(t.cell(0, t.last + 1));
    g.setup.answer(cell.clone());
    g.formula_check(&cell, &format!("=SUM({})", t.range(1)));
    key(g, "Alt+=");
    let prompt = tr(&format!("Встань в {cell} и нажми Alt+= , затем Enter. Excel сам напишет СУММ."));
    let mut task = g.task(ts("Автосумма"), prompt, None, cell);
    task.goal_seconds = 10.0;
    Ok(task)
}

pub fn f4(g: &mut Gen) -> Result<Task> {
    let n = g.rows();
    let t = g.table(ts("Курсовые разницы"), &[ts("Платёж"), ts("Сумма, $")], n);
    for r in t.rows() {
        g.txt(&t, 0, r, &tr(&format!("Платёж {}", r - t.first + 1)));
        let v = g.d.amount(100, 9_000, 10);
        g.num(&t, 1, r, v);
    }
    let lc = t.right + 2;
    g.setup.text(a1(t.header, lc), ts("Курс"));
    g.setup.bold.push(a1(t.header, lc));
    let rate = a1(t.header + 1, lc);
    g.setup.input(rate.clone(), Cell::Num(g.d.int(80, 99) as f64));
    let fixed = format!("${}${}", super::letter(lc), t.header + 1);
    let c1 = t.col(1);
    key(g, "F4");
    let first = t.cell(2, t.first);
    let mut task = g.column(ts("F4 ставит $"), &t, 2, ts("Сумма, ₽"), |r| format!("={}*{fixed}", a1(r, c1)), "", "");
    task.prompt = tr(&format!("В {first} набери ={}*{rate}, сразу нажми F4 (появятся $), Enter — и протяни вниз.", a1(t.first, c1)));
    task.example = None;
    Ok(task)
}

pub fn filldown(g: &mut Gen) -> Result<Task> {
    let n = g.rows().max(6);
    let t = g.table(ts("Накладная"), &[ts("Цена"), ts("Кол-во"), ts("Сумма")], n);
    for r in t.rows() {
        let p = g.d.amount(10, 900, 5);
        let q = g.d.int(1, 40) as f64;
        g.num(&t, 0, r, p);
        g.num(&t, 1, r, q);
    }
    let (a, b) = (t.col(0), t.col(1));
    g.setup.put(t.cell(2, t.first), Cell::Formula(format!("={}*{}", a1(t.first, a), a1(t.first, b))), false);
    for r in t.first + 1..=t.last {
        let cell = t.cell(2, r);
        g.setup.answer(cell.clone());
        g.formula_check(&cell, &format!("={}*{}", a1(r, a), a1(r, b)));
    }
    key(g, "Ctrl+D");
    let range = format!("{}:{}", t.cell(2, t.first), t.cell(2, t.last));
    let prompt = tr(&format!("Формула уже есть в {}. Выдели {range} (Shift+стрелки) и нажми Ctrl+D.", t.cell(2, t.first)));
    let mut task = g.task("Ctrl+D", prompt, None, t.cell(2, t.first));
    task.goal_seconds = 12.0;
    Ok(task)
}

pub fn jump(g: &mut Gen) -> Result<Task> {
    let n = 40 + g.l.int(0, 60) as u32;
    let t = g.table(ts("Длинный реестр"), &[ts("Контрагент"), ts("Сумма")], n);
    for r in t.rows() {
        let p = g.d.pick(PARTIES);
        g.txt(&t, 0, r, p);
        let v = g.d.amount(1_000, 90_000, 100);
        g.num(&t, 1, r, v);
    }
    let cell = t.cell(0, t.last + 1);
    g.check(&cell, Kind::Value { cell: cell.clone(), expected: Cell::Text(ts("Итого").into()) });
    key(g, "Ctrl+Down");
    let prompt = ts("Не прокручивая мышью: Ctrl+↓ — в конец столбца, ещё ↓ — и напиши «Итого» под последней строкой.").to_string();
    let mut task = g.task(ts("Ctrl+стрелки"), prompt, None, t.cell(0, t.header));
    task.goal_seconds = 10.0;
    Ok(task)
}

pub fn filter(g: &mut Gen) -> Result<Task> {
    let n = g.rows();
    let t = g.table(ts("Реестр"), &[ts("Контрагент"), ts("Сумма")], n);
    for r in t.rows() {
        let p = g.d.pick(PARTIES);
        g.txt(&t, 0, r, p);
        let v = g.d.amount(1_000, 90_000, 100);
        g.num(&t, 1, r, v);
    }
    g.check(ts("Фильтр"), Kind::Filter);
    key(g, "Ctrl+Shift+L");
    let mut task = g.task("Ctrl+Shift+L", ts("Включи фильтр одним нажатием: Ctrl+Shift+L.").into(), None, t.cell(0, t.first));
    task.reorders = true;
    task.goal_seconds = 6.0;
    Ok(task)
}

pub fn bold(g: &mut Gen) -> Result<Task> {
    let n = g.rows();
    let t = g.table(ts("Отчёт"), &[ts("Контрагент"), ts("Сумма"), ts("Статус")], n);
    g.setup.bold.clear();
    for r in t.rows() {
        let p = g.d.pick(PARTIES);
        g.txt(&t, 0, r, p);
        let v = g.d.amount(1_000, 90_000, 100);
        g.num(&t, 1, r, v);
        let st = if g.d.chance(50) { ts("Оплачен") } else { ts("Ждёт") };
        g.txt(&t, 2, r, st);
    }
    let head = format!("{}:{}", t.cell(0, t.header), t.cell(2, t.header));
    g.check(ts("Жирный"), Kind::Bold { range: head.clone() });
    key(g, "Ctrl+B");
    let mut task = g.task("Ctrl+B", tr(&format!("Выдели шапку {head} (Shift+→) и нажми Ctrl+B.")), None, t.cell(0, t.header));
    task.goal_seconds = 6.0;
    Ok(task)
}

pub fn today(g: &mut Gen) -> Result<Task> {
    let n = g.rows().min(6);
    let t = g.table(ts("Журнал документов"), &[ts("Документ"), ts("Дата получения")], n);
    for r in t.rows() {
        let no = 10 + g.l.int(0, 89);
        g.txt(&t, 0, r, &tr(&format!("Акт № {no}")));
        if r < t.last {
            let d = g.some_date();
            g.day(&t, 1, r, d);
        }
    }
    let cell = t.cell(1, t.last);
    g.setup.answer(cell.clone());
    key(g, "Ctrl+;");
    g.check(&cell, Kind::Today { cell: cell.clone() });
    let mut task = g.task(ts("Ctrl+; — сегодня"), tr(&format!("Последний акт пришёл сегодня. Встань в {cell}, нажми Ctrl+; и Enter.")), None, cell);
    task.goal_seconds = 6.0;
    Ok(task)
}
