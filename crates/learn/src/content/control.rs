//! «Проверь себя»: a control formula proves a table is clean, or points at the row that is not.
//!
//! Each kind of entry error has its own check, taught right after the function it uses:
//! СЧЁТЗ − СЧЁТ finds numbers stored as text, «whole minus parts» finds a status spelled
//! differently, СЧЁТЕСЛИ against the catalog finds an article that is not there, and
//! ДЛСТР − ДЛСТР(СЖПРОБЕЛЫ) finds stray spaces. The learner writes the check, then names the row.
//! Errors start visible and turn invisible at levels 3–4, where only the formula sees them.

use super::{Gen, PARTIES, PRODUCTS, Table};
use crate::check::{Expectation, Kind};
use crate::sheet::{Cell, a1, ru};
use crate::{Result, Task};
use gridcraft_tr::{t as tr, ts};

/// The row the error sits in; layout seed only, so re-checks on other data keep it.
fn broken_row(g: &mut Gen, t: &Table) -> u32 {
    t.first + g.l.below(t.n() as usize) as u32
}

/// «Строка с ошибкой:» to the right of everything; the learner types the row number.
fn row_answer(g: &mut Gen, col: u32, row: u32, broken: u32) -> String {
    g.setup.text(a1(row, col), ts("Строка с ошибкой:"));
    g.setup.bold.push(a1(row, col));
    g.setup.width(col, 150.0);
    g.setup.width(col + 1, 110.0);
    let cell = a1(row, col + 1);
    g.setup.answer(cell.clone());
    g.check(&cell, Kind::Expected { cell: cell.clone(), expected: Expectation::Number(f64::from(broken)), formula: false });
    cell
}

/// One control formula in one cell, then the broken row.
fn single(g: &mut Gen, t: &Table, label: &str, reference: &str, broken: u32) -> (String, String) {
    let ctl = g.answer(t, 1, label);
    g.formula_check(&ctl, reference);
    let row = row_answer(g, t.right + 2, t.header + 2, broken);
    (ctl, row)
}

/// `fill`: the control is a column, written once and filled down.
#[allow(clippy::too_many_arguments)]
fn finish(g: &mut Gen, title: &str, prompts: [String; 3], example: String, focus: String, hints: [String; 2], broken: u32, fill: bool) -> Task {
    let [p1, p2, p3] = prompts;
    let prompt = match g.level {
        1 => p1,
        2 => p2,
        _ => p3,
    };
    // The exercise is finding, not fixing: an edited source would hide the very row asked for.
    let prompt = format!("{prompt} {}", ts("Ничего в таблице не исправляй: задача только найти строку."));
    // At level 1 the formula is on screen; the last hint then gives only the row.
    let answer = match (g.level, fill) {
        (1, _) => tr(&format!("Ответ: строка {broken}")),
        (_, true) => tr(&format!("Ответ: {example}, протянуть вниз; строка {broken}")),
        (_, false) => tr(&format!("Ответ: {example}, строка {broken}")),
    };
    let mut task = g.task(title, prompt, Some(example), focus);
    let [h1, h2] = hints;
    task.hints = vec![h1, h2, answer];
    task.goal_seconds += 15.0;
    task
}

/// СЧЁТЗ − СЧЁТ: how many amounts are text. Level 3+ drops the «тг», so only the alignment
/// (and the formula) gives it away.
pub fn count_text(g: &mut Gen) -> Result<Task> {
    let n = g.rows();
    let t = g.table(ts("Реестр счетов"), &[ts("Товар"), ts("Сумма")], n);
    let items = g.l.sample(PRODUCTS, n as usize);
    let broken = broken_row(g, &t);
    let unit = g.level <= 2;
    for (i, r) in t.rows().enumerate() {
        let item = items.get(i % items.len().max(1)).copied().unwrap_or(ts("Товар"));
        g.txt(&t, 0, r, item);
        let v = g.d.amount(1_000, 90_000, 10);
        if r == broken {
            // A visible «тг» first; later a 1С-style «12 500»: looks like a number, is text.
            let v = v as i64;
            let text = if unit { format!("{v} {}", ts("тг")) } else { format!("{} {:03}", v / 1000, v % 1000) };
            g.setup.input(t.cell(1, r), Cell::Text(text));
        } else {
            g.num(&t, 1, r, v);
        }
    }
    let range = t.range(1);
    let reference = format!("=COUNTA({range})-COUNT({range})");
    let (ctl, row) = single(g, &t, ts("Текст вместо чисел:"), &reference, broken);
    let shown = ru(&reference);
    Ok(finish(
        g,
        ts("Число или текст"),
        [
            tr(&format!(
                "В {ctl} введи контрольную формулу: сколько в столбце «Сумма» текста вместо чисел. Затем найди эту строку и впиши её номер в {row}."
            )),
            tr(&format!(
                "Сравни СЧЁТЗ и СЧЁТ по столбцу «Сумма» в {ctl}: разница - это числа, записанные текстом. Номер такой строки впиши в {row}."
            )),
            tr(&format!(
                "Итог по суммам не сходится с бумагами. Докажи формулой в {ctl}, что в столбце «Сумма» есть число, записанное текстом, и впиши номер этой строки в {row}."
            )),
        ],
        shown,
        ctl,
        [
            ts("СЧЁТЗ считает все заполненные ячейки, СЧЁТ только настоящие числа. Если разница не 0, столько чисел записано текстом.").to_string(),
            ts("Число-текст прижато к левому краю ячейки, настоящее число к правому. Ищи в столбце «Сумма» значение, которое стоит слева.")
                .to_string(),
        ],
        broken,
        false,
    ))
}

/// Whole minus parts: total of all amounts minus the total of every proper status. Not zero
/// means some status is spelled differently, and the difference is exactly its amount.
pub fn reconcile(g: &mut Gen) -> Result<Task> {
    let n = g.rows();
    let t = g.table(ts("Счета по статусам"), &[ts("Поставщик"), ts("Статус"), ts("Сумма")], n);
    let parties = g.l.sample(PARTIES, 3);
    let statuses = [ts("Получен"), ts("В пути"), ts("Заказан")];
    let broken = broken_row(g, &t);
    let variants: [&str; 3] =
        if gridcraft_tr::english() { ["Recieved", "Received ", "received."] } else { ["Получено", "Получен ", "получен."] };
    // Visible spellings first; a trailing space (invisible) from level 3.
    let variant = if g.level >= 3 { variants[1] } else { [variants[0], variants[2]][g.l.below(2)] };
    for r in t.rows() {
        let p = g.d.pick(&parties);
        g.txt(&t, 0, r, p);
        let s = if r == broken { variant } else { g.d.pick(&statuses) };
        g.txt(&t, 1, r, s);
        let v = g.d.amount(3_000, 250_000, 50);
        g.num(&t, 2, r, v);
    }
    let (st, amt) = (t.range(1), t.range(2));
    let reference =
        format!("=SUM({amt})-SUMIF({st},\"{}\",{amt})-SUMIF({st},\"{}\",{amt})-SUMIF({st},\"{}\",{amt})", statuses[0], statuses[1], statuses[2]);
    let (ctl, row) = single(g, &t, ts("Не попало в статусы:"), &reference, broken);
    let shown = ru(&reference);
    Ok(finish(
        g,
        ts("Сверка: части = целое"),
        [
            tr(&format!(
                "Сверка: общая сумма минус суммы по каждому правильному статусу должна дать 0. Введи её в {ctl}. Если вышло не 0, найди строку с этой суммой и впиши её номер в {row}."
            )),
            tr(&format!(
                "В {ctl} вычти из общей суммы (СУММ) суммы по статусам «Получен», «В пути» и «Заказан» (СУММЕСЛИ). Не 0 - значит, какой-то статус записан иначе. Номер этой строки впиши в {row}."
            )),
            tr(&format!(
                "Отчёт по статусам не сходится с общим итогом. Докажи формулой в {ctl}, сколько денег не попало ни в один статус, и впиши в {row} номер строки, где статус записан с ошибкой."
            )),
        ],
        shown,
        ctl,
        [
            ts("Каждый счёт должен попасть ровно в один статус. Тогда сумма по статусам равна общей сумме, а разница равна 0.").to_string(),
            ts("Разница - это ровно сумма строки, которая не попала ни в один статус. Найди в столбце «Сумма» это число: в его строке и ошибка.")
                .to_string(),
        ],
        broken,
        false,
    ))
}

/// One check per order row: how many times its article is in the catalog. A 0 is the broken row.
pub fn catalog(g: &mut Gen) -> Result<Task> {
    let n = g.rows();
    let t = g.table(ts("Заказ"), &[ts("Код"), ts("Кол-во"), ts("В каталоге")], n);
    let cat_col = t.right + 2;
    let size = 6 + g.l.below(3) as u32;
    g.setup.header(t.header, cat_col, &[ts("Код"), ts("Цена")]);
    g.setup.width(cat_col, 100.0);
    let codes: Vec<String> = (0..size).map(|_| tr(&format!("А-{}", 1000 + g.l.int(1, 8999)))).collect();
    for (i, code) in codes.iter().enumerate() {
        let r = t.first + i as u32;
        g.setup.input(a1(r, cat_col), Cell::Text(code.clone()));
        let p = g.d.amount(100, 9_000, 10);
        g.setup.input(a1(r, cat_col + 1), Cell::Num(p));
    }
    let broken = broken_row(g, &t);
    for r in t.rows() {
        let code = codes.get(g.l.below(codes.len())).cloned().unwrap_or_default();
        // Without its dash first (you can see it); a trailing space from level 3 (you cannot).
        let code = if r != broken {
            code
        } else if g.level >= 3 {
            format!("{code} ")
        } else {
            code.replacen('-', "", 1)
        };
        g.txt(&t, 0, r, &code);
        let q = g.d.int(1, 40) as f64;
        g.num(&t, 1, r, q);
    }
    let catalog = format!("${0}${1}:${0}${2}", super::letter(cat_col), t.first, t.first + size - 1);
    for r in t.rows() {
        let cell = t.cell(2, r);
        g.setup.answer(cell.clone());
        g.formula_check(&cell, &format!("=COUNTIF({catalog},{})", t.cell(0, r)));
    }
    let row = row_answer(g, cat_col + 3, t.header + 1, broken);
    let (first, last) = (t.cell(2, t.first), t.cell(2, t.last));
    let shown = ru(&format!("=COUNTIF({catalog},{})", t.cell(0, t.first)));
    Ok(finish(
        g,
        ts("Есть ли артикул в каталоге"),
        [
            tr(&format!(
                "В {first} посчитай, сколько раз код этой строки есть в каталоге, и протяни до {last}. Где 0, кода в каталоге нет: номер этой строки впиши в {row}."
            )),
            tr(&format!("Через СЧЁТЕСЛИ по каталогу проверь каждый код заказа ({first}:{last}). Номер строки, где получился 0, впиши в {row}.")),
            tr(&format!(
                "ВПР по этому заказу где-то даст #Н/Д. Заранее проверь формулой в {first}:{last}, какой код не найдётся в каталоге, и впиши номер его строки в {row}."
            )),
        ],
        shown,
        first,
        [
            ts("СЧЁТЕСЛИ(каталог; код) считает, сколько раз код есть в каталоге. 1 - всё в порядке, 0 - такого кода нет, ВПР по нему даст #Н/Д.")
                .to_string(),
            ts("$ держит каталог на месте, когда протягиваешь формулу вниз. Если код на вид такой же, как в каталоге, а СЧЁТЕСЛИ даёт 0, в нём лишний пробел.")
                .to_string(),
        ],
        broken,
        true,
    ))
}

/// Length before and after СЖПРОБЕЛЫ: not 0 means the name carries stray spaces.
pub fn spaces(g: &mut Gen) -> Result<Task> {
    let n = g.rows();
    let t = g.table(ts("Счета"), &[ts("Поставщик"), ts("Сумма"), ts("Лишних пробелов")], n);
    g.setup.width(t.col(0), 190.0);
    let parties = g.l.sample(PARTIES, 3);
    let broken = broken_row(g, &t);
    let pick = g.l.below(2);
    for r in t.rows() {
        let p = g.d.pick(&parties);
        // A doubled space inside first (looks odd), a trailing one from level 3 (looks fine).
        let name = if r != broken {
            p.to_string()
        } else if g.level >= 3 || pick == 0 {
            format!("{p} ")
        } else {
            p.replacen(' ', "  ", 1)
        };
        g.txt(&t, 0, r, &name);
        let v = g.d.amount(3_000, 250_000, 50);
        g.num(&t, 1, r, v);
    }
    for r in t.rows() {
        let cell = t.cell(2, r);
        g.setup.answer(cell.clone());
        let name = t.cell(0, r);
        g.formula_check(&cell, &format!("=LEN({name})-LEN(TRIM({name}))"));
    }
    let row = row_answer(g, t.right + 2, t.header + 1, broken);
    let (first, last) = (t.cell(2, t.first), t.cell(2, t.last));
    let shown = ru(&format!("=LEN({0})-LEN(TRIM({0}))", t.cell(0, t.first)));
    Ok(finish(
        g,
        ts("Лишние пробелы"),
        [
            tr(&format!(
                "В {first} посчитай, сколько лишних пробелов в названии: длина как есть минус длина после СЖПРОБЕЛЫ. Протяни до {last}. Номер строки, где не 0, впиши в {row}."
            )),
            tr(&format!("Сравни ДЛСТР названия и ДЛСТР(СЖПРОБЕЛЫ(…)) в {first}:{last}. Номер строки, где разница не 0, впиши в {row}.")),
            tr(&format!(
                "СУММЕСЛИ по одному поставщику не сходится, хотя названия на вид одинаковые. Проверь формулой в {first}:{last}, где в названии лишние пробелы, и впиши номер строки в {row}."
            )),
        ],
        shown,
        first,
        [
            ts("ДЛСТР считает символы, пробелы тоже. СЖПРОБЕЛЫ убирает пробелы по краям и двойные внутри. Если длина до и после разная, пробелы лишние.")
                .to_string(),
            ts("Пробел в конце названия глазом не видно, но СУММЕСЛИ и ВПР из-за него считают название другим.").to_string(),
        ],
        broken,
        true,
    ))
}
