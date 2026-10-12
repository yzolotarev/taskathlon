//! Open work cases: a message from a manager, raw data, and results graded by value.
//! The method is free; formulas are rewarded where the brief asks for a live table.

use super::{CITIES, DEPTS, Gen, PARTIES, PEOPLE, PRODUCTS};
use crate::check::{Check, Kind};
use crate::sheet::{Cell, a1, date, date_text, num_text};
use crate::{Brief, Mode, Result, Task};
use std::collections::BTreeMap;

pub struct CaseDef {
    pub id: &'static str,
    pub title: &'static str,
    pub role: &'static str,
    pub minutes: u32,
    /// Skills that make the case fair; it unlocks when they reach level 2.
    pub requires: &'static [&'static str],
    pub build: fn(&mut Gen) -> Result<Task>,
}

pub const CASES: &[CaseDef] = &[
    CaseDef {
        id: "case.sales",
        title: "Отчёт по продажам",
        role: "Офис-аналитик",
        minutes: 20,
        requires: &["fn.sum", "cond.sumif", "fn.minmax"],
        build: sales,
    },
    CaseDef {
        id: "case.payroll",
        title: "Зарплатная ведомость",
        role: "Помощник бухгалтера",
        minutes: 20,
        requires: &["refs.abs", "fn.round", "acc.payroll"],
        build: payroll,
    },
    CaseDef {
        id: "case.bank",
        title: "Сверка с банковской выпиской",
        role: "Помощник бухгалтера",
        minutes: 20,
        requires: &["cond.countif", "lookup.xlookup", "logic.if"],
        build: bank,
    },
    CaseDef {
        id: "case.act", title: "Акт сверки с контрагентом", role: "Бухгалтер", minutes: 20, requires: &["fn.sum", "acc.entries"], build: act
    },
    CaseDef {
        id: "case.ar",
        title: "Просроченная дебиторка",
        role: "Бухгалтер / аналитик",
        minutes: 25,
        requires: &["date.diff", "logic.if", "cond.sumifs"],
        build: receivables,
    },
    CaseDef {
        id: "case.cleanup",
        title: "Выгрузка из 1С",
        role: "Помощник бухгалтера",
        minutes: 15,
        requires: &["text.trim", "text.number", "cond.sumif"],
        build: cleanup,
    },
    CaseDef {
        id: "case.purchase",
        title: "Сравнение поставщиков",
        role: "Стажёр в отдел закупок",
        minutes: 20,
        requires: &["fn.minmax", "lookup.index", "refs.fill"],
        build: purchase,
    },
    CaseDef {
        id: "case.stock",
        title: "Остатки на складе",
        role: "Офис-аналитик / склад",
        minutes: 15,
        requires: &["start.sub", "logic.if", "start.mul"],
        build: stock,
    },
    CaseDef {
        id: "case.budget",
        title: "План-факт по бюджету",
        role: "Помощник экономиста",
        minutes: 15,
        requires: &["start.sub", "logic.if", "fn.sum"],
        build: budget,
    },
    CaseDef {
        id: "case.vacation",
        title: "График отпусков",
        role: "Стажёр HR / документооборот",
        minutes: 15,
        requires: &["date.diff", "cond.sumif", "fn.minmax"],
        build: vacation,
    },
    CaseDef {
        id: "case.retail",
        title: "Продажи магазинов",
        role: "Аналитик в ритейле",
        minutes: 20,
        requires: &["fn.round", "cond.sumif", "fn.minmax"],
        build: retail,
    },
];

pub fn case(id: &str) -> Option<&'static CaseDef> {
    CASES.iter().find(|c| c.id == id)
}

fn push(g: &mut Gen, skill: &str, label: &str, kind: Kind) {
    g.checks.push(Check { label: label.into(), skill: skill.into(), kind });
}

fn constant(v: f64) -> String {
    format!("={v}")
}

fn finish(g: &mut Gen, id: &str, brief: Brief, deliverables: &[String], focus: String) -> Task {
    let def = case(id);
    let mut t = g.task(def.map(|d| d.title).unwrap_or("Кейс"), String::new(), None, focus);
    t.spec.skill = id.into();
    t.mode = Mode::Case;
    t.prompt = deliverables.iter().enumerate().map(|(i, d)| format!("{}. {d}", i + 1)).collect::<Vec<_>>().join("\n");
    t.brief = Some(brief);
    t.goal_seconds = def.map(|d| d.minutes as f32 * 60.0).unwrap_or(900.0);
    t.hints = vec![
        "Разбей задачу: что именно нужно сдать? Ответы — в жёлтые ячейки.".into(),
        "Сначала посчитай одну строку/одного контрагента, потом протяни.".into(),
        "Проверь себя: сходится ли итог с суммой исходной таблицы?".into(),
    ];
    t
}

fn size(g: &Gen, base: u32) -> u32 {
    base + 4 * u32::from(g.level.saturating_sub(1))
}

pub fn sales(g: &mut Gen) -> Result<Task> {
    let n = size(g, 14);
    let managers = g.l.sample(PEOPLE, 4);
    g.setup.header(1, 0, &["Дата", "Менеджер", "Город", "Сумма"]);
    for (c, w) in [(0, 90.0), (1, 120.0), (2, 90.0), (3, 100.0)] {
        g.setup.width(c, w);
    }
    let mut totals: BTreeMap<&str, f64> = BTreeMap::new();
    for r in 2..2 + n {
        let d = date(2026, 3, 1) + g.d.int(0, 30) as f64;
        let m = managers.get(g.d.below(managers.len())).copied().unwrap_or("Петров С.");
        let city = g.d.pick(CITIES);
        let v = g.d.amount(5_000, 250_000, 500);
        g.setup.input(a1(r, 0), Cell::Date(d));
        g.setup.input(a1(r, 1), Cell::Text(m.into()));
        g.setup.input(a1(r, 2), Cell::Text(city.into()));
        g.setup.input(a1(r, 3), Cell::Num(v));
        *totals.entry(m).or_default() += v;
    }
    let last = 1 + n;
    g.setup.header(1, 5, &["Менеджер", "План"]);
    g.setup.width(5, 120.0);
    let mut over = 0.0;
    for (i, m) in managers.iter().enumerate() {
        let r = 2 + i as u32;
        let got = totals.get(m).copied().unwrap_or(0.0);
        let plan = ((got * g.d.int(80, 120) as f64 / 100.0) / 10_000.0).round() * 10_000.0 + 10_000.0 * f64::from(u8::from(i == 0));
        g.setup.input(a1(r, 5), Cell::Text((*m).into()));
        g.setup.input(a1(r, 6), Cell::Num(plan));
        if got >= plan {
            over += 1.0;
        }
    }
    let best =
        totals.iter().max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal)).map(|(m, _)| (*m).to_string()).unwrap_or_default();
    for (r, label) in [(1, "Выручка за март:"), (2, "Лучший менеджер:"), (3, "Выполнили план, чел.:")] {
        g.setup.text(a1(r, 8), label);
        g.setup.bold.push(a1(r, 8));
        g.setup.answer(a1(r, 9));
    }
    g.setup.width(8, 170.0);
    g.setup.width(9, 120.0);
    push(g, "fn.sum", "Выручка за март (J1)", Kind::Formula { cell: "J1".into(), reference: format!("=SUM(D2:D{last})"), live: true });
    push(g, "fn.minmax", "Лучший менеджер (J2)", Kind::Value { cell: "J2".into(), expected: Cell::Text(best) });
    push(g, "logic.if", "Выполнили план (J3)", Kind::Formula { cell: "J3".into(), reference: constant(over), live: false });
    let pairs = managers.iter().map(|m| ((*m).to_string(), format!("=SUMIF(B2:B{last},\"{m}\",D2:D{last})"))).collect();
    push(g, "cond.sumif", "Продажи по менеджерам", Kind::Pairs { pairs });
    let brief = Brief {
        from: "Ольга Сергеевна, коммерческий директор".into(),
        subject: "Итоги марта".into(),
        body: "Привет! Подготовь, пожалуйста, к планёрке итоги марта: сколько всего продали, кто лучший, и кто из менеджеров выполнил план. И табличку продаж по каждому менеджеру — хочу видеть цифры.".into(),
    };
    let deliverables = vec![
        "J1 — выручка за март (формулой).".into(),
        "J2 — фамилия лучшего менеджера по продажам.".into(),
        "J3 — сколько менеджеров выполнили план (план в F:G).".into(),
        "Сводка «менеджер → сумма продаж» где-нибудь на листе (сводная или СУММЕСЛИ).".into(),
    ];
    Ok(finish(g, "case.sales", brief, &deliverables, "J1".into()))
}

pub fn payroll(g: &mut Gen) -> Result<Task> {
    let n = size(g, 6);
    g.setup.header(1, 0, &["Сотрудник", "Оклад", "Отработано, дн.", "Начислено", "НДФЛ 13%", "К выплате"]);
    for (c, w) in [(0, 130.0), (1, 90.0), (2, 120.0), (3, 110.0), (4, 100.0), (5, 110.0)] {
        g.setup.width(c, w);
    }
    g.setup.text("H1", "Рабочих дней в месяце:");
    g.setup.width(7, 190.0);
    let days = g.d.int(19, 23) as f64;
    g.setup.input("I1", Cell::Num(days));
    let people = g.l.sample(PEOPLE, n as usize);
    let (mut sum_acc, mut sum_pay) = (0.0, 0.0);
    for i in 0..n {
        let r = 2 + i;
        let who = people.get(i as usize).map(|s| (*s).to_string()).unwrap_or_else(|| format!("Сотрудник {}", i + 1));
        g.setup.text(a1(r, 0), who);
        let salary = g.d.amount(35_000, 150_000, 1_000);
        g.setup.input(a1(r, 1), Cell::Num(salary));
        let worked = if g.d.chance(60) { days } else { (days - g.d.int(1, 8) as f64).max(1.0) };
        g.setup.input(a1(r, 2), Cell::Num(worked));
        let accrued = (salary / days * worked * 100.0).round() / 100.0;
        sum_acc += accrued;
        sum_pay += accrued - (accrued * 0.13).round();
        for c in 3..=5 {
            g.setup.answer(a1(r, c));
        }
        push(
            g,
            "refs.abs",
            &format!("Начислено, строка {r}"),
            Kind::Formula { cell: a1(r, 3), reference: format!("=ROUND(B{r}/$I$1*C{r},2)"), live: true },
        );
        push(
            g,
            "acc.payroll",
            &format!("НДФЛ, строка {r}"),
            Kind::Formula { cell: a1(r, 4), reference: format!("=ROUND(ROUND(B{r}/$I$1*C{r},2)*0.13,0)"), live: true },
        );
        push(
            g,
            "acc.payroll",
            &format!("К выплате, строка {r}"),
            Kind::Formula { cell: a1(r, 5), reference: format!("=ROUND(B{r}/$I$1*C{r},2)-ROUND(ROUND(B{r}/$I$1*C{r},2)*0.13,0)"), live: true },
        );
    }
    let total = 2 + n;
    g.setup.text(a1(total, 0), "Итого");
    g.setup.bold.push(a1(total, 0));
    for c in 3..=5 {
        g.setup.answer(a1(total, c));
    }
    push(g, "fn.sum", "Итого начислено", Kind::Formula { cell: a1(total, 3), reference: constant(sum_acc), live: false });
    push(g, "fn.sum", "Итого к выплате", Kind::Formula { cell: a1(total, 5), reference: constant(sum_pay), live: false });
    let brief = Brief {
        from: "Марина Викторовна, главный бухгалтер".into(),
        subject: "Ведомость за месяц".into(),
        body: "Добрый день! Посчитай, пожалуйста, ведомость: начисление по отработанным дням (оклад / рабочие дни × отработано, до копеек), НДФЛ 13% до рубля и к выплате. Внизу итоги. Ведомость должна пересчитываться, если поменяются дни.".into(),
    };
    let deliverables = vec![
        "D — начислено: оклад / рабочие дни (I1) × отработано, ОКРУГЛ до копеек.".into(),
        "E — НДФЛ 13% от начисленного, ОКРУГЛ до рубля.".into(),
        "F — к выплате = начислено − НДФЛ.".into(),
        format!("Строка {total} — итоги по столбцам D и F."),
    ];
    Ok(finish(g, "case.payroll", brief, &deliverables, "D2".into()))
}

pub fn bank(g: &mut Gen) -> Result<Task> {
    let n = size(g, 10);
    g.setup.text("A1", "Журнал оплат (наш учёт)");
    g.setup.bold.push("A1".into());
    g.setup.header(2, 0, &["Дата", "№ п/п", "Контрагент", "Сумма", "В выписке?"]);
    g.setup.text("G1", "Выписка банка");
    g.setup.bold.push("G1".into());
    g.setup.header(2, 6, &["Дата", "№ п/п", "Сумма"]);
    for (c, w) in [(0, 90.0), (1, 70.0), (2, 130.0), (3, 100.0), (4, 100.0), (6, 90.0), (7, 70.0), (8, 100.0)] {
        g.setup.width(c, w);
    }
    let k = 2 + usize::from(g.level >= 3);
    let mut missing: Vec<usize> = (0..n as usize).collect();
    g.l.shuffle(&mut missing);
    missing.truncate(k);
    let mut bank = Vec::new();
    let (mut cnt, mut sum) = (0.0, 0.0);
    for i in 0..n as usize {
        let r = 3 + i as u32;
        let d = date(2026, 4, 1) + g.d.int(0, 25) as f64;
        let no = 100.0 + i as f64 * 3.0 + g.l.int(0, 2) as f64;
        let v = g.d.amount(2_000, 300_000, 50);
        g.setup.input(a1(r, 0), Cell::Date(d));
        g.setup.input(a1(r, 1), Cell::Num(no));
        g.setup.input(a1(r, 2), Cell::Text(g.d.pick(PARTIES).into()));
        g.setup.input(a1(r, 3), Cell::Num(v));
        let present = !missing.contains(&i);
        let cell = a1(r, 4);
        g.setup.answer(cell.clone());
        push(g, "cond.countif", &format!("E{r}"), Kind::Value { cell, expected: Cell::Text(if present { "Да" } else { "Нет" }.into()) });
        if present {
            bank.push((d + g.d.int(0, 1) as f64, no, v));
        } else {
            cnt += 1.0;
            sum += v;
        }
    }
    g.l.shuffle(&mut bank);
    for (j, (d, no, v)) in bank.iter().enumerate() {
        let r = 3 + j as u32;
        g.setup.input(a1(r, 6), Cell::Date(*d));
        g.setup.input(a1(r, 7), Cell::Num(*no));
        g.setup.input(a1(r, 8), Cell::Num(*v));
    }
    for (r, label) in [(2, "Не прошло платежей:"), (3, "На сумму:")] {
        g.setup.text(a1(r, 10), label);
        g.setup.bold.push(a1(r, 10));
        g.setup.answer(a1(r, 11));
    }
    g.setup.width(10, 170.0);
    push(g, "cond.countif", "Не прошло платежей (L2)", Kind::Formula { cell: "L2".into(), reference: constant(cnt), live: false });
    push(g, "cond.sumif", "На сумму (L3)", Kind::Formula { cell: "L3".into(), reference: constant(sum), live: false });
    let brief = Brief {
        from: "Марина Викторовна, главный бухгалтер".into(),
        subject: "Сверь оплаты с выпиской".into(),
        body: "Привет! Поставщики жалуются, что не получили деньги. Сверь, пожалуйста, наш журнал оплат с выпиской банка по номеру платёжки: какие платежи не прошли и на какую сумму.".into(),
    };
    let deliverables = vec![
        "E — для каждой строки журнала «Да», если платёжка есть в выписке, иначе «Нет» (сверяй по № п/п).".into(),
        "L2 — сколько платежей не прошло.".into(),
        "L3 — на какую сумму.".into(),
    ];
    Ok(finish(g, "case.bank", brief, &deliverables, "E3".into()))
}

pub fn act(g: &mut Gen) -> Result<Task> {
    let n = size(g, 6);
    let party = g.l.pick(PARTIES);
    g.setup.text("A1", "Наш учёт: расчёты с покупателем (сч. 62)");
    g.setup.bold.push("A1".into());
    g.setup.header(2, 0, &["Дата", "Документ", "Дебет", "Кредит"]);
    g.setup.text("F1", format!("Данные {party}"));
    g.setup.bold.push("F1".into());
    g.setup.header(2, 5, &["Дата", "Документ", "Дебет", "Кредит"]);
    for (c, w) in [(0, 90.0), (1, 150.0), (2, 100.0), (3, 100.0), (5, 90.0), (6, 150.0), (7, 100.0), (8, 100.0)] {
        g.setup.width(c, w);
    }
    let lost = g.l.below(n as usize);
    let (mut our_dt, mut our_kt, mut their_dt, mut their_kt) = (0.0, 0.0, 0.0, 0.0);
    let mut lost_doc = String::new();
    let mut tr = 3;
    for i in 0..n as usize {
        let r = 3 + i as u32;
        let d = date(2026, 1, 15) + (i as f64) * 9.0 + g.d.int(0, 5) as f64;
        let ship = i % 2 == 0;
        let doc = if ship { format!("Реализация № {}", 10 + i) } else { format!("Оплата п/п № {}", 200 + i) };
        let v = g.d.amount(10_000, 400_000, 100);
        g.setup.input(a1(r, 0), Cell::Date(d));
        g.setup.input(a1(r, 1), Cell::Text(doc.clone()));
        if ship {
            g.setup.input(a1(r, 2), Cell::Num(v));
            our_dt += v;
        } else {
            g.setup.input(a1(r, 3), Cell::Num(v));
            our_kt += v;
        }
        if ship && i == lost - lost % 2 {
            lost_doc = doc;
            continue;
        }
        g.setup.input(a1(tr, 5), Cell::Date(d + g.d.int(0, 2) as f64));
        g.setup.input(a1(tr, 6), Cell::Text(doc));
        if ship {
            g.setup.input(a1(tr, 8), Cell::Num(v));
            their_kt += v;
        } else {
            g.setup.input(a1(tr, 7), Cell::Num(v));
            their_dt += v;
        }
        tr += 1;
    }
    for (r, label) in [(2, "Сальдо по нашим данным:"), (3, "Сальдо по их данным:"), (4, "Расхождение:"), (5, "Нет у контрагента:")]
    {
        g.setup.text(a1(r, 10), label);
        g.setup.bold.push(a1(r, 10));
        g.setup.answer(a1(r, 11));
    }
    g.setup.width(10, 200.0);
    g.setup.width(11, 160.0);
    let ours = our_dt - our_kt;
    let theirs = their_kt - their_dt;
    push(g, "fn.sum", "Наше сальдо (L2)", Kind::Formula { cell: "L2".into(), reference: constant(ours), live: false });
    push(g, "fn.sum", "Их сальдо (L3)", Kind::Formula { cell: "L3".into(), reference: constant(theirs), live: false });
    push(g, "start.sub", "Расхождение (L4)", Kind::Formula { cell: "L4".into(), reference: constant((ours - theirs).abs()), live: false });
    push(g, "acc.entries", "Пропавший документ (L5)", Kind::Value { cell: "L5".into(), expected: Cell::Text(lost_doc) });
    let brief = Brief {
        from: "Марина Викторовна, главный бухгалтер".into(),
        subject: format!("Акт сверки с {party}"),
        body: format!(
            "{party} прислали свой акт сверки, сальдо не сходится. Посчитай сальдо по нашим данным (Дебет − Кредит) и по их (у них зеркально: Кредит − Дебет), разницу и найди документ, которого у них нет."
        ),
    };
    let deliverables = vec![
        "L2 — сальдо по нашим данным (дебет − кредит).".into(),
        "L3 — сальдо по данным контрагента (кредит − дебет).".into(),
        "L4 — расхождение (положительным числом).".into(),
        "L5 — название документа, которого нет у контрагента (как в нашей таблице).".into(),
    ];
    Ok(finish(g, "case.act", brief, &deliverables, "L2".into()))
}

pub fn receivables(g: &mut Gen) -> Result<Task> {
    let n = size(g, 10);
    let report = date(2026, 5, 31);
    g.setup.header(1, 0, &["Контрагент", "Счёт", "Дата", "Отсрочка", "Сумма", "Оплачено", "Долг", "Просрочка, дн."]);
    for (c, w) in [(0, 130.0), (1, 70.0), (2, 90.0), (3, 80.0), (4, 100.0), (5, 100.0), (6, 100.0), (7, 120.0)] {
        g.setup.width(c, w);
    }
    g.setup.text("J1", "Отчётная дата:");
    g.setup.input("K1", Cell::Date(report));
    g.setup.width(9, 170.0);
    g.setup.width(10, 110.0);
    let parties = g.l.sample(PARTIES, 4);
    let mut overdue: BTreeMap<&str, f64> = BTreeMap::new();
    let mut total = 0.0;
    for i in 0..n {
        let r = 2 + i;
        let p = parties.get(g.d.below(parties.len())).copied().unwrap_or("ООО Альфа");
        let d = date(2026, 3, 1) + g.d.int(0, 80) as f64;
        let delay = [10.0, 14.0, 30.0];
        let delay = delay.get(g.d.below(3)).copied().unwrap_or(14.0);
        let v = g.d.amount(5_000, 200_000, 100);
        let paid = match g.d.below(3) {
            0 => v,
            1 => 0.0,
            _ => (v * 0.5 / 100.0).round() * 100.0,
        };
        g.setup.input(a1(r, 0), Cell::Text(p.into()));
        g.setup.input(a1(r, 1), Cell::Num((500 + i) as f64));
        g.setup.input(a1(r, 2), Cell::Date(d));
        g.setup.input(a1(r, 3), Cell::Num(delay));
        g.setup.input(a1(r, 4), Cell::Num(v));
        g.setup.input(a1(r, 5), Cell::Num(paid));
        g.setup.answer(a1(r, 6));
        g.setup.answer(a1(r, 7));
        push(g, "start.sub", &format!("Долг, строка {r}"), Kind::Formula { cell: a1(r, 6), reference: format!("=E{r}-F{r}"), live: true });
        push(
            g,
            "date.diff",
            &format!("Просрочка, строка {r}"),
            Kind::Formula { cell: a1(r, 7), reference: format!("=IF(E{r}-F{r}>0,MAX(0,$K$1-(C{r}+D{r})),0)"), live: true },
        );
        let debt = v - paid;
        if debt > 0.0 && report - (d + delay) > 0.0 {
            *overdue.entry(p).or_default() += debt;
            total += debt;
        }
    }
    g.setup.text("J3", "Просрочено всего:");
    g.setup.bold.push("J3".into());
    g.setup.answer("K3");
    push(g, "cond.sumifs", "Просрочено всего (K3)", Kind::Formula { cell: "K3".into(), reference: constant(total), live: false });
    let pairs = parties.iter().map(|p| ((*p).to_string(), constant(overdue.get(p).copied().unwrap_or(0.0)))).collect();
    push(g, "cond.sumifs", "Просрочка по контрагентам", Kind::Pairs { pairs });
    let brief = Brief {
        from: "Андрей Петрович, финансовый директор".into(),
        subject: "Кто нам должен".into(),
        body: format!(
            "Добрый день! Нужна картина по дебиторке на {}: долг по каждому счёту, на сколько дней просрочен, и сколько всего просрочено по каждому клиенту. Завтра звоню должникам.",
            date_text(report)
        ),
    };
    let deliverables = vec![
        "G — долг по счёту (сумма − оплачено).".into(),
        "H — дни просрочки на отчётную дату K1: если долг есть и срок (дата + отсрочка) прошёл, иначе 0.".into(),
        "K3 — просроченный долг всего.".into(),
        "Сводка «контрагент → просроченный долг» где-нибудь на листе.".into(),
    ];
    Ok(finish(g, "case.ar", brief, &deliverables, "G2".into()))
}

pub fn cleanup(g: &mut Gen) -> Result<Task> {
    let n = size(g, 10);
    g.setup.header(1, 0, &["Контрагент (1С)", "Сумма (1С)", "", "Контрагент", "Сумма"]);
    for (c, w) in [(0, 190.0), (1, 110.0), (2, 30.0), (3, 150.0), (4, 110.0)] {
        g.setup.width(c, w);
    }
    let parties = g.l.sample(PARTIES, 3);
    let mut totals: BTreeMap<&str, f64> = BTreeMap::new();
    let mut all = 0.0;
    for i in 0..n {
        let r = 2 + i;
        let p = parties.get(g.d.below(parties.len())).copied().unwrap_or("ООО Альфа");
        let messy = match g.d.below(3) {
            0 => format!("  {p}"),
            1 => format!("{p}  "),
            _ => p.replace(' ', "   "),
        };
        let v = g.d.int(1_000, 600_000) as f64;
        g.setup.input(a1(r, 0), Cell::Text(messy));
        g.setup.input(a1(r, 1), Cell::Text(num_text(v)));
        g.setup.answer(a1(r, 3));
        g.setup.answer(a1(r, 4));
        push(g, "text.trim", &format!("Контрагент, строка {r}"), Kind::Formula { cell: a1(r, 3), reference: format!("=TRIM(A{r})"), live: true });
        push(
            g,
            "text.number",
            &format!("Сумма, строка {r}"),
            Kind::Formula { cell: a1(r, 4), reference: format!("=VALUE(SUBSTITUTE(B{r},\" \",\"\"))"), live: true },
        );
        *totals.entry(p).or_default() += v;
        all += v;
    }
    g.setup.text("G1", "Итого:");
    g.setup.bold.push("G1".into());
    g.setup.answer("H1");
    g.setup.width(6, 90.0);
    g.setup.width(7, 120.0);
    push(g, "fn.sum", "Итого (H1)", Kind::Formula { cell: "H1".into(), reference: constant(all), live: false });
    let pairs = parties.iter().map(|p| ((*p).to_string(), constant(totals.get(p).copied().unwrap_or(0.0)))).collect();
    push(g, "cond.sumif", "Итоги по контрагентам", Kind::Pairs { pairs });
    let brief = Brief {
        from: "Марина Викторовна, главный бухгалтер".into(),
        subject: "Выгрузка не считается".into(),
        body: "Выгрузила из 1С реестр, а СУММ выдаёт ноль и контрагенты дублируются в фильтре. Приведи, пожалуйста, в порядок и дай итоги по каждому контрагенту.".into(),
    };
    let deliverables = vec![
        "D — чистое название контрагента (без лишних пробелов).".into(),
        "E — сумма числом.".into(),
        "H1 — общий итог.".into(),
        "Сводка «контрагент → сумма» где-нибудь на листе.".into(),
    ];
    Ok(finish(g, "case.cleanup", brief, &deliverables, "D2".into()))
}

fn label_answers(g: &mut Gen, col: u32, labels: &[&str]) {
    for (i, label) in labels.iter().enumerate() {
        let r = 1 + i as u32;
        g.setup.text(a1(r, col), *label);
        g.setup.bold.push(a1(r, col));
        g.setup.answer(a1(r, col + 1));
    }
    g.setup.width(col, 200.0);
    g.setup.width(col + 1, 120.0);
}

pub fn purchase(g: &mut Gen) -> Result<Task> {
    let n = size(g, 6);
    let suppliers = g.l.sample(PARTIES, 3);
    let mut head = vec!["Товар", "Кол-во"];
    head.extend(suppliers.iter().copied());
    head.extend(["Мин. цена", "Лучший поставщик", "Сумма закупки"]);
    g.setup.header(1, 0, &head);
    for (c, w) in [(0, 120.0), (1, 70.0), (2, 110.0), (3, 110.0), (4, 110.0), (5, 100.0), (6, 150.0), (7, 120.0)] {
        g.setup.width(c, w);
    }
    let items = g.l.sample(PRODUCTS, n as usize);
    let (mut best_total, mut current_total) = (String::new(), String::new());
    for i in 0..n {
        let r = 2 + i;
        g.setup.text(a1(r, 0), items.get(i as usize).copied().unwrap_or("Товар"));
        g.setup.input(a1(r, 1), Cell::Num(g.d.int(5, 120) as f64));
        // Three distinct prices so the cheapest supplier is unique.
        let base = g.d.int(40, 900);
        let mut offs = [0i64, 1, 2];
        g.d.shuffle(&mut offs);
        for (k, o) in offs.iter().enumerate() {
            let price = (base + o * g.d.int(3, 40)) as f64 + 0.5 * k as f64 * f64::from(u8::from(*o == 0));
            g.setup.input(a1(r, 2 + k as u32), Cell::Num(price));
        }
        for c in 5..=7 {
            g.setup.answer(a1(r, c));
        }
        push(g, "fn.minmax", &format!("Мин. цена, строка {r}"), Kind::Formula { cell: a1(r, 5), reference: format!("=MIN(C{r}:E{r})"), live: true });
        push(
            g,
            "lookup.index",
            &format!("Поставщик, строка {r}"),
            Kind::Formula { cell: a1(r, 6), reference: format!("=INDEX($C$1:$E$1,MATCH(MIN(C{r}:E{r}),C{r}:E{r},0))"), live: true },
        );
        push(g, "refs.fill", &format!("Сумма, строка {r}"), Kind::Formula { cell: a1(r, 7), reference: format!("=B{r}*MIN(C{r}:E{r})"), live: true });
        best_total.push_str(&format!("+B{r}*MIN(C{r}:E{r})"));
        current_total.push_str(&format!("+B{r}*C{r}"));
    }
    label_answers(g, 9, &["Итого по лучшим ценам:", "Экономия против первого поставщика:"]);
    push(g, "fn.sum", "Итого (K1)", Kind::Formula { cell: "K1".into(), reference: format!("={}", best_total.trim_start_matches('+')), live: true });
    push(
        g,
        "fn.sum",
        "Экономия (K2)",
        Kind::Formula {
            cell: "K2".into(),
            reference: format!("=({})-({})", current_total.trim_start_matches('+'), best_total.trim_start_matches('+')),
            live: true,
        },
    );
    let first = suppliers.first().copied().unwrap_or("первый поставщик");
    let brief = Brief {
        from: "Айдар Маратович, руководитель закупок".into(),
        subject: "Коммерческие предложения".into(),
        body: format!(
            "Пришли КП от трёх поставщиков. Для каждой позиции найди минимальную цену и кто её дал, посчитай сумму закупки. И сколько сэкономим, если уйдём от «{first}», у которого сейчас всё берём. Цены будут обновляться — нужны формулы."
        ),
    };
    let deliverables = vec![
        "F — минимальная цена по позиции.".into(),
        "G — название поставщика с этой ценой (ИНДЕКС+ПОИСКПОЗ).".into(),
        "H — сумма закупки = кол-во × мин. цена.".into(),
        "K1 — итого по лучшим ценам; K2 — экономия против первого поставщика (столбец C).".into(),
    ];
    Ok(finish(g, "case.purchase", brief, &deliverables, "F2".into()))
}

pub fn stock(g: &mut Gen) -> Result<Task> {
    let n = size(g, 7);
    g.setup.header(1, 0, &["Товар", "На начало", "Приход", "Расход", "Цена", "Мин. запас", "На конец", "Заказать?", "Стоимость"]);
    for (c, w) in [(0, 120.0), (1, 85.0), (2, 75.0), (3, 75.0), (4, 70.0), (5, 90.0), (6, 85.0), (7, 90.0), (8, 100.0)] {
        g.setup.width(c, w);
    }
    let items = g.l.sample(PRODUCTS, n as usize);
    let (mut value, mut to_order) = (String::new(), 0.0);
    for i in 0..n {
        let r = 2 + i;
        g.setup.text(a1(r, 0), items.get(i as usize).copied().unwrap_or("Товар"));
        let start = g.d.int(10, 200);
        let inc = g.d.int(0, 150);
        let out = g.d.int(0, start + inc);
        let min = g.d.int(10, 60);
        g.setup.input(a1(r, 1), Cell::Num(start as f64));
        g.setup.input(a1(r, 2), Cell::Num(inc as f64));
        g.setup.input(a1(r, 3), Cell::Num(out as f64));
        g.setup.input(a1(r, 4), Cell::Num(g.d.int(30, 2500) as f64));
        g.setup.input(a1(r, 5), Cell::Num(min as f64));
        if start + inc - out < min {
            to_order += 1.0;
        }
        for c in 6..=8 {
            g.setup.answer(a1(r, c));
        }
        push(g, "start.sub", &format!("На конец, строка {r}"), Kind::Formula { cell: a1(r, 6), reference: format!("=B{r}+C{r}-D{r}"), live: true });
        push(
            g,
            "logic.if",
            &format!("Заказать, строка {r}"),
            Kind::Formula { cell: a1(r, 7), reference: format!("=IF(B{r}+C{r}-D{r}<F{r},\"Да\",\"Нет\")"), live: true },
        );
        push(
            g,
            "start.mul",
            &format!("Стоимость, строка {r}"),
            Kind::Formula { cell: a1(r, 8), reference: format!("=(B{r}+C{r}-D{r})*E{r}"), live: true },
        );
        value.push_str(&format!("+(B{r}+C{r}-D{r})*E{r}"));
    }
    label_answers(g, 10, &["Стоимость остатков:", "Позиций к заказу:"]);
    push(
        g,
        "fn.sum",
        "Стоимость остатков (L1)",
        Kind::Formula { cell: "L1".into(), reference: format!("={}", value.trim_start_matches('+')), live: true },
    );
    push(g, "cond.countif", "Позиций к заказу (L2)", Kind::Formula { cell: "L2".into(), reference: constant(to_order), live: false });
    let brief = Brief {
        from: "Сергей, начальник склада".into(),
        subject: "Остатки на конец месяца".into(),
        body: "Посчитай остатки на конец, отметь, что пора дозаказать (если остаток ниже минимального запаса — «Да», иначе «Нет»), и сколько денег лежит на складе.".into(),
    };
    let deliverables = vec![
        "G — остаток на конец = начало + приход − расход.".into(),
        "H — «Да», если остаток ниже мин. запаса, иначе «Нет».".into(),
        "I — стоимость остатка = остаток × цена.".into(),
        "L1 — общая стоимость остатков; L2 — сколько позиций заказать.".into(),
    ];
    Ok(finish(g, "case.stock", brief, &deliverables, "G2".into()))
}

pub fn budget(g: &mut Gen) -> Result<Task> {
    let lines = ["Аренда", "Зарплата", "Реклама", "Связь", "Канцтовары", "Транспорт", "Обучение", "Командировки", "IT-сервисы", "Представительские"];
    let n = size(g, 6).min(lines.len() as u32);
    g.setup.header(1, 0, &["Статья", "План", "Факт", "Отклонение", "Исполнение, %", "Статус"]);
    for (c, w) in [(0, 150.0), (1, 100.0), (2, 100.0), (3, 110.0), (4, 120.0), (5, 120.0)] {
        g.setup.width(c, w);
    }
    for i in 0..n {
        let r = 2 + i;
        g.setup.text(a1(r, 0), lines.get(i as usize).copied().unwrap_or("Статья"));
        let plan = g.d.amount(20_000, 900_000, 1_000);
        let fact = (plan * g.d.int(70, 130) as f64 / 100.0).round();
        g.setup.input(a1(r, 1), Cell::Num(plan));
        g.setup.input(a1(r, 2), Cell::Num(fact));
        for c in 3..=5 {
            g.setup.answer(a1(r, c));
        }
        push(g, "start.sub", &format!("Отклонение, строка {r}"), Kind::Formula { cell: a1(r, 3), reference: format!("=C{r}-B{r}"), live: true });
        push(g, "start.div", &format!("Исполнение, строка {r}"), Kind::Formula { cell: a1(r, 4), reference: format!("=C{r}/B{r}"), live: true });
        push(
            g,
            "logic.if",
            &format!("Статус, строка {r}"),
            Kind::Formula { cell: a1(r, 5), reference: format!("=IF(C{r}>B{r}*1.1,\"Перерасход\",\"В норме\")"), live: true },
        );
    }
    let last = 1 + n;
    let total = last + 1;
    g.setup.text(a1(total, 0), "Итого");
    g.setup.bold.push(a1(total, 0));
    for c in 1..=3 {
        g.setup.answer(a1(total, c));
    }
    push(g, "fn.sum", "Итого план", Kind::Formula { cell: a1(total, 1), reference: format!("=SUM(B2:B{last})"), live: true });
    push(g, "fn.sum", "Итого факт", Kind::Formula { cell: a1(total, 2), reference: format!("=SUM(C2:C{last})"), live: true });
    push(g, "fn.sum", "Итого отклонение", Kind::Formula { cell: a1(total, 3), reference: format!("=SUM(C2:C{last})-SUM(B2:B{last})"), live: true });
    let brief = Brief {
        from: "Елена Петровна, финансовый директор".into(),
        subject: "План-факт за квартал".into(),
        body: "Нужен план-факт по статьям: отклонение в рублях, исполнение в процентах и статус — «Перерасход», если факт больше плана больше чем на 10%, иначе «В норме». Внизу — итоги.".into(),
    };
    let deliverables = vec![
        "D — отклонение = факт − план.".into(),
        "E — исполнение = факт / план (можно формат %).".into(),
        "F — «Перерасход», если факт > план × 1,1, иначе «В норме».".into(),
        format!("Строка {total} — итоги по B, C, D."),
    ];
    Ok(finish(g, "case.budget", brief, &deliverables, "D2".into()))
}

pub fn vacation(g: &mut Gen) -> Result<Task> {
    let n = size(g, 8);
    g.setup.header(1, 0, &["Сотрудник", "Отдел", "Начало", "Окончание", "Дней"]);
    for (c, w) in [(0, 130.0), (1, 110.0), (2, 100.0), (3, 100.0), (4, 70.0)] {
        g.setup.width(c, w);
    }
    let people = g.l.sample(PEOPLE, PEOPLE.len());
    let depts = g.l.sample(DEPTS, 3);
    let mut longest = (0i64, String::new());
    let mut total = 0.0;
    for i in 0..n {
        let r = 2 + i;
        let who = people.get(i as usize).map(|s| (*s).to_string()).unwrap_or_else(|| format!("Сотрудник {}", i + 1));
        g.setup.text(a1(r, 0), who.clone());
        let dept = depts.get(g.d.below(depts.len())).copied().unwrap_or("Склад");
        g.setup.input(a1(r, 1), Cell::Text(dept.into()));
        let start = date(2026, 6, 1) + g.d.int(0, 80) as f64;
        // Distinct lengths so the longest vacation is unique.
        let days = 5 + i as i64 * 2 + g.d.int(0, 1);
        g.setup.input(a1(r, 2), Cell::Date(start));
        g.setup.input(a1(r, 3), Cell::Date(start + days as f64 - 1.0));
        g.setup.answer(a1(r, 4));
        push(g, "date.diff", &format!("Дней, строка {r}"), Kind::Formula { cell: a1(r, 4), reference: format!("=D{r}-C{r}+1"), live: true });
        total += days as f64;
        if days > longest.0 {
            longest = (days, who);
        }
    }
    let last = 1 + n;
    label_answers(g, 6, &["Всего дней отпуска:", "Самый длинный отпуск у:"]);
    push(g, "fn.sum", "Всего дней (H1)", Kind::Formula { cell: "H1".into(), reference: format!("=SUM(D2:D{last})-SUM(C2:C{last})+{n}"), live: true });
    push(g, "fn.minmax", "Самый длинный (H2)", Kind::Value { cell: "H2".into(), expected: Cell::Text(longest.1) });
    let pairs = depts
        .iter()
        .map(|d| {
            ((*d).to_string(), format!("=SUMIF(B2:B{last},\"{d}\",D2:D{last})-SUMIF(B2:B{last},\"{d}\",C2:C{last})+COUNTIF(B2:B{last},\"{d}\")"))
        })
        .collect();
    push(g, "cond.sumif", "Дни по отделам", Kind::Pairs { pairs });
    let _ = total;
    let brief = Brief {
        from: "Анна, HR-менеджер".into(),
        subject: "График отпусков на лето".into(),
        body: "Посчитай, пожалуйста, сколько календарных дней отпуска у каждого (оба дня включительно), сколько всего, у кого самый длинный, и сколько дней отпуска приходится на каждый отдел.".into(),
    };
    let deliverables = vec![
        "E — дней отпуска (окончание − начало + 1).".into(),
        "H1 — всего дней; H2 — фамилия с самым длинным отпуском.".into(),
        "Сводка «отдел → дней» где-нибудь на листе.".into(),
    ];
    Ok(finish(g, "case.vacation", brief, &deliverables, "E2".into()))
}

pub fn retail(g: &mut Gen) -> Result<Task> {
    let n = size(g, 12);
    let stores = ["Пятёрочка на Ленина", "Магазин у дома №3", "Супермаркет Центр", "Дискаунтер Северный"];
    let cats = ["Молочка", "Овощи", "Бакалея", "Напитки"];
    g.setup.header(1, 0, &["Магазин", "Категория", "Выручка", "Чеков", "Средний чек"]);
    for (c, w) in [(0, 180.0), (1, 100.0), (2, 100.0), (3, 70.0), (4, 110.0)] {
        g.setup.width(c, w);
    }
    let mut by_store: BTreeMap<&str, f64> = BTreeMap::new();
    let mut all = 0.0;
    for i in 0..n {
        let r = 2 + i;
        // Every store appears; the order is shuffled by the data seed.
        let store = if (i as usize) < stores.len() { stores.get(i as usize).copied().unwrap_or("Магазин") } else { g.d.pick(&stores) };
        let cat = g.d.pick(&cats);
        let rev = g.d.amount(40_000, 900_000, 10) + i as f64;
        let checks = g.d.int(80, 1500) as f64;
        g.setup.input(a1(r, 0), Cell::Text(store.into()));
        g.setup.input(a1(r, 1), Cell::Text(cat.into()));
        g.setup.input(a1(r, 2), Cell::Num(rev));
        g.setup.input(a1(r, 3), Cell::Num(checks));
        g.setup.answer(a1(r, 4));
        push(
            g,
            "fn.round",
            &format!("Средний чек, строка {r}"),
            Kind::Formula { cell: a1(r, 4), reference: format!("=ROUND(C{r}/D{r},2)"), live: true },
        );
        *by_store.entry(store).or_default() += rev;
        all += rev;
    }
    let last = 1 + n;
    let (top, top_rev) = by_store
        .iter()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(s, v)| ((*s).to_string(), *v))
        .unwrap_or_default();
    label_answers(g, 6, &["Лучший магазин:", "Его доля выручки:"]);
    push(g, "fn.minmax", "Лучший магазин (H1)", Kind::Value { cell: "H1".into(), expected: Cell::Text(top) });
    push(g, "start.div", "Доля (H2)", Kind::Formula { cell: "H2".into(), reference: constant(top_rev / all), live: false });
    let pairs = stores.iter().map(|s| ((*s).to_string(), format!("=SUMIF(A2:A{last},\"{s}\",C2:C{last})"))).collect();
    push(g, "cond.sumif", "Выручка по магазинам", Kind::Pairs { pairs });
    let brief = Brief {
        from: "Дмитрий, руководитель отдела аналитики".into(),
        subject: "Срез по магазинам за неделю".into(),
        body: "Это тестовое задание. Посчитай средний чек по каждой строке (до копеек), выручку по каждому магазину, какой магазин лучший и какую долю общей выручки он даёт.".into(),
    };
    let deliverables = vec![
        "E — средний чек = выручка / чеки, ОКРУГЛ до копеек.".into(),
        "Сводка «магазин → выручка» где-нибудь на листе (сводная или СУММЕСЛИ).".into(),
        "H1 — лучший магазин; H2 — его доля в общей выручке (число или %).".into(),
    ];
    Ok(finish(g, "case.retail", brief, &deliverables, "E2".into()))
}
