//! Supply-assistant work at an IoT device maker: components for a production plan, and
//! deliveries that are late. Both come straight from the job: «контроль за уровнем запасов
//! с учётом потребностей производства и сроков поставки» and «контроль своевременной поставки».

use super::{Gen, PARTIES};
use crate::sheet::{Cell, date, ru};
use crate::{Result, Task};
use gridcraft_tr::{t as tr, ts};

/// Components of one LoRaWAN sensor and how many go into a single device.
const BOM: &[(&str, f64)] = &[
    ("Плата контроллера", 1.0),
    ("Корпус IP67", 1.0),
    ("Батарея ER14505", 2.0),
    ("Антенна 868 МГц", 1.0),
    ("Крепёж DIN-рейка", 2.0),
    ("Уплотнитель корпуса", 1.0),
    ("Винт M3", 4.0),
    ("Этикетка с QR-кодом", 1.0),
];

/// Need = norm × plan; to order = need − stock − in transit, never below zero.
pub fn need(g: &mut Gen) -> Result<Task> {
    let n = (g.rows() as usize).min(BOM.len()).max(5) as u32;
    let headers = [ts("Комплектующая"), ts("На 1 устройство"), ts("На складе"), ts("В пути"), ts("Потребность"), ts("К заказу")];
    let t = g.table(ts("Комплектующие под выпуск датчиков"), &headers, n);
    g.setup.width(t.col(0), 190.0);
    let plan = g.l.amount(200, 1_200, 50);
    let plan_cell = g.param(&t, 1, ts("План выпуска, шт:"), Cell::Num(plan));
    let abs_plan = {
        let at = crate::sheet::address(&plan_cell)?;
        format!("${}${}", crate::sheet::col_name(at.col), at.row + 1)
    };
    // One component is already covered, so «не меньше нуля» really matters.
    let covered = t.first + g.l.below(n as usize) as u32;
    for (i, r) in t.rows().enumerate() {
        let (name, norm) = BOM.get(i).copied().unwrap_or(("Винт M3", 4.0));
        g.txt(&t, 0, r, ts(name));
        g.num(&t, 1, r, norm);
        let need = norm * plan;
        let (stock, transit) = if r == covered {
            (need + g.d.amount(20, 400, 10), 0.0)
        } else {
            (g.d.amount(0, (need * 0.5) as i64, 10), g.d.amount(0, (need * 0.3) as i64, 10))
        };
        g.num(&t, 2, r, stock);
        g.num(&t, 3, r, transit);
    }
    let mut first_need = String::new();
    let mut first_order = String::new();
    for r in t.rows() {
        let need_cell = t.cell(4, r);
        let order_cell = t.cell(5, r);
        g.setup.answer(need_cell.clone());
        g.setup.answer(order_cell.clone());
        let need_ref = format!("={}*{abs_plan}", t.cell(1, r));
        // The grader probes the bare sheet, where the learner's need column is still empty, so
        // the reference computes the need itself; the shown formula uses the need column.
        let order_ref = format!("=MAX(0,{}*{abs_plan}-{}-{})", t.cell(1, r), t.cell(2, r), t.cell(3, r));
        let order_shown = format!("=MAX(0,{}-{}-{})", t.cell(4, r), t.cell(2, r), t.cell(3, r));
        g.formula_check(&need_cell, &need_ref);
        g.formula_check(&order_cell, &order_ref);
        if r == t.first {
            first_need = format!("{need_cell}: {}", ru(&need_ref));
            first_order = format!("{order_cell}: {}", ru(&order_shown));
        }
    }
    let (n1, n2) = (t.cell(4, t.first), t.cell(4, t.last));
    let (o1, o2) = (t.cell(5, t.first), t.cell(5, t.last));
    let prompt = match g.level {
        1 => tr(&format!(
            "Посчитай потребность (норма × план выпуска) в {n1}:{n2} и сколько дозаказать в {o1}:{o2}: потребность минус склад минус то, что уже едет, но не меньше нуля. Введи формулы в первую строку и протяни вниз:"
        )),
        2 => tr(&format!(
            "Потребность в {n1}:{n2}: норма × план из {plan_cell} (закрепи его $). К заказу в {o1}:{o2}: МАКС(0; потребность − склад − в пути)."
        )),
        _ => tr(&format!(
            "Производство выпускает {} датчиков. Посчитай, сколько каждой комплектующей нужно ({n1}:{n2}) и сколько дозаказать у поставщиков ({o1}:{o2}) с учётом склада и того, что уже в пути.",
            plan as i64
        )),
    };
    let example = format!("{first_need}\n{first_order}");
    let mut task = g.task(ts("Сколько заказать под производство"), prompt, Some(example.clone()), n1);
    task.hints = vec![
        tr(&format!(
            "Потребность = сколько штук на одно устройство × план выпуска. План один на всех, поэтому в формуле {abs_plan} со знаком $: при протягивании ссылка не съедет."
        )),
        ts("К заказу = потребность − на складе − в пути. Если на складе и так хватает, разность будет отрицательной, а заказать минус нельзя: МАКС(0; …) превращает минус в 0.")
            .to_string(),
        if g.level == 1 { ts("Проверь себя: в строке, где на складе больше потребности, к заказу должно быть 0.").to_string() } else { tr(&format!("Ответ: {}", example)) },
    ];
    task.goal_seconds += 40.0;
    Ok(task)
}

/// Late = the due date has passed and the goods are not received.
pub fn overdue(g: &mut Gen) -> Result<Task> {
    let n = g.rows() + 1;
    let headers = [ts("№ заказа"), ts("Поставщик"), ts("Срок поставки"), ts("Статус"), ts("Контроль")];
    let t = g.table(ts("Контроль поставок"), &headers, n);
    g.setup.width(t.col(1), 170.0);
    let today = date(2026, 3, 16) + g.l.int(0, 10) as f64;
    let today_cell = g.param(&t, 1, ts("Сегодня:"), Cell::Date(today));
    let abs_today = {
        let at = crate::sheet::address(&today_cell)?;
        format!("${}${}", crate::sheet::col_name(at.col), at.row + 1)
    };
    let parties = g.l.sample(PARTIES, 3);
    let received = ts("Получен");
    let others = [ts("В пути"), ts("Заказан")];
    let start = 100 + g.l.int(0, 800);
    // Every case appears: late and not received, late but received, not yet due.
    let forced: Vec<usize> = {
        let mut v: Vec<usize> = (0..n as usize).collect();
        g.l.shuffle(&mut v);
        v.truncate(3);
        v
    };
    for (i, r) in t.rows().enumerate() {
        g.txt(&t, 0, r, &format!("PO-{}", start + i as i64));
        let p = g.d.pick(&parties);
        g.txt(&t, 1, r, p);
        let case = forced.iter().position(|x| *x == i).unwrap_or_else(|| g.l.below(3));
        let (due, status) = match case {
            0 => (today - g.l.int(1, 12) as f64, others[g.l.below(2)]),
            1 => (today - g.l.int(1, 12) as f64, received),
            _ => (today + g.l.int(0, 14) as f64, if g.l.chance(30) { received } else { others[g.l.below(2)] }),
        };
        g.day(&t, 2, r, due);
        g.txt(&t, 3, r, status);
    }
    let mut first_formula = String::new();
    for r in t.rows() {
        let cell = t.cell(4, r);
        g.setup.answer(cell.clone());
        let reference = format!("=IF(AND({}<>\"{received}\",{}<{abs_today}),\"{}\",\"\")", t.cell(3, r), t.cell(2, r), ts("Просрочено"));
        g.formula_check(&cell, &reference);
        if r == t.first {
            first_formula = ru(&reference);
        }
    }
    let (c1, c2) = (t.cell(4, t.first), t.cell(4, t.last));
    let prompt = match g.level {
        1 => tr(&format!(
            "Отметь просроченные поставки: если товар ещё не получен, а срок уже прошёл, в {c1}:{c2} должно стоять «Просрочено», иначе пусто. Введи в {c1} и протяни вниз:"
        )),
        2 => tr(&format!("В {c1}:{c2} через ЕСЛИ и И: статус не «{received}» и срок раньше даты из {today_cell} → «Просрочено», иначе \"\".")),
        _ => tr(&format!(
            "Руководитель просит к планёрке отметить заказы, которые поставщики задерживают. В {c1}:{c2} напиши «Просрочено» там, где срок прошёл, а товар так и не получен."
        )),
    };
    let mut task = g.task(ts("Просроченные поставки"), prompt, Some(first_formula.clone()), c1.clone());
    task.hints = vec![
        ts("Просрочено, когда верны оба условия сразу: товар не получен И срок уже прошёл. Два условия сразу - это И(…; …) внутри ЕСЛИ.").to_string(),
        tr(&format!(
            "«Не равно» пишется <>. Срок прошёл: дата срока < сегодня ({abs_today}, со знаком $). Полученный заказ не просрочен, даже если срок давно прошёл."
        )),
        if g.level == 1 {
            ts("Проверь себя: у заказа со статусом «Получен» в «Контроле» должно быть пусто, какой бы ни была дата.").to_string()
        } else {
            tr(&format!("Ответ: {first_formula}"))
        },
    ];
    task.goal_seconds += 30.0;
    Ok(task)
}
