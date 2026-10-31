//! Choosing the function: the question is said in work words, and the learner decides which
//! tool answers it. Every other drill names its function; here naming it is the exercise.
//!
//! Level 1 offers three candidates (the right one and the two most easily confused with it),
//! level 2 asks with plain cue words, level 3 in a manager's words, level 4 asks three
//! different questions about one table. Any formula that gives the right number passes.

use super::{Gen, PARTIES, Table};
use crate::sheet::{Cell, ru};
use crate::{Result, Task};
use gridcraft_tr::{t as tr, ts};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ask {
    Total,
    Count,
    Max,
    Min,
    Average,
    SumBy,
    CountBy,
    AverageBy,
    Lookup,
}

/// What the learner needs on the sheet: the asked supplier and the asked invoice number.
struct Params {
    party: String,
    party_name: &'static str,
    number: String,
    number_value: f64,
}

impl Ask {
    fn function(self) -> &'static str {
        match self {
            Ask::Total => ts("СУММ"),
            Ask::Count => ts("СЧЁТ"),
            Ask::Max => ts("МАКС"),
            Ask::Min => ts("МИН"),
            Ask::Average => ts("СРЗНАЧ"),
            Ask::SumBy => ts("СУММЕСЛИ"),
            Ask::CountBy => ts("СЧЁТЕСЛИ"),
            Ask::AverageBy => ts("СРЗНАЧЕСЛИ"),
            Ask::Lookup => ts("ВПР"),
        }
    }

    /// What the function does, in one plain phrase.
    fn does(self) -> &'static str {
        match self {
            Ask::Total => ts("складывает все числа"),
            Ask::Count => ts("считает, сколько ячеек с числами"),
            Ask::Max => ts("находит самое большое число"),
            Ask::Min => ts("находит самое маленькое число"),
            Ask::Average => ts("считает среднее"),
            Ask::SumBy => ts("складывает только строки, где выполнено условие"),
            Ask::CountBy => ts("считает, сколько строк подходит под условие"),
            Ask::AverageBy => ts("считает среднее только по подходящим строкам"),
            Ask::Lookup => ts("находит строку по коду и берёт из неё значение"),
        }
    }

    /// The two answers a learner most often confuses with this one.
    fn confusable(self) -> [Ask; 2] {
        match self {
            Ask::Total => [Ask::SumBy, Ask::Count],
            Ask::Count => [Ask::Total, Ask::CountBy],
            Ask::Max => [Ask::Min, Ask::Total],
            Ask::Min => [Ask::Max, Ask::Average],
            Ask::Average => [Ask::AverageBy, Ask::Total],
            Ask::SumBy => [Ask::CountBy, Ask::Total],
            Ask::CountBy => [Ask::SumBy, Ask::Count],
            Ask::AverageBy => [Ask::Average, Ask::SumBy],
            Ask::Lookup => [Ask::SumBy, Ask::Max],
        }
    }

    fn reference(self, t: &Table, p: &Params) -> String {
        let (who, amount) = (t.range(0), t.range(2));
        match self {
            Ask::Total => format!("=SUM({amount})"),
            Ask::Count => format!("=COUNT({amount})"),
            Ask::Max => format!("=MAX({amount})"),
            Ask::Min => format!("=MIN({amount})"),
            Ask::Average => format!("=AVERAGE({amount})"),
            Ask::SumBy => format!("=SUMIF({who},{},{amount})", p.party),
            Ask::CountBy => format!("=COUNTIF({who},{})", p.party),
            Ask::AverageBy => format!("=AVERAGEIF({who},{},{amount})", p.party),
            Ask::Lookup => format!("=VLOOKUP({},{}:{},2,FALSE)", p.number, t.cell(1, t.first), t.cell(2, t.last)),
        }
    }

    /// The question with the cue words left in plain sight (level 2).
    fn plain(self, p: &Params) -> String {
        match self {
            Ask::Total => ts("Сколько всего денег по всем счетам?").to_string(),
            Ask::Count => ts("Сколько всего счетов в таблице?").to_string(),
            Ask::Max => ts("Какой самый крупный счёт?").to_string(),
            Ask::Min => ts("Какой самый маленький счёт?").to_string(),
            Ask::Average => ts("Какая средняя сумма счёта?").to_string(),
            Ask::SumBy => tr(&format!("Сколько денег по счетам контрагента из {}?", p.party)),
            Ask::CountBy => tr(&format!("Сколько счетов у контрагента из {}?", p.party)),
            Ask::AverageBy => tr(&format!("Какая средняя сумма счёта у контрагента из {}?", p.party)),
            Ask::Lookup => tr(&format!("Какая сумма у счёта с номером из {}?", p.number)),
        }
    }

    /// The same question as a manager would put it (levels 3–4).
    fn work(self, p: &Params) -> String {
        match self {
            Ask::Total => ts("Финдиректор спрашивает, во что нам обошлись все эти счета вместе.").to_string(),
            Ask::Count => ts("Для отчёта нужно знать, сколько раз нам выставляли счёт.").to_string(),
            Ask::Max => ts("Директор хочет знать наш рекордный счёт по деньгам.").to_string(),
            Ask::Min => ts("На какую сумму был самый скромный счёт?").to_string(),
            Ask::Average => ts("Сколько обычно стоит один счёт, если взять всё вместе?").to_string(),
            Ask::SumBy => tr(&format!("Бухгалтерия сверяет расчёты с {}: на какую сумму он нам выставил?", p.party_name)),
            Ask::CountBy => tr(&format!("Сколько раз {} выставлял нам счёт?", p.party_name)),
            Ask::AverageBy => tr(&format!("Какой обычно счёт у {}?", p.party_name)),
            Ask::Lookup => tr(&format!("Звонят по счёту № {}: на какую сумму он был?", p.number_value)),
        }
    }

    /// How to hear the function in the question (hint 1), quoting the words actually asked.
    fn cue(self, work: bool) -> &'static str {
        if work {
            return match self {
                Ask::Total => ts("«во что обошлись все эти счета вместе» значит сложить все суммы"),
                Ask::Count => ts("«сколько раз» значит посчитать строки, а не сложить деньги"),
                Ask::Max => ts("«рекордный» значит наибольшее число"),
                Ask::Min => ts("«самый скромный» значит наименьшее число"),
                Ask::Average => ts("«обычно стоит один» значит среднее"),
                Ask::SumBy => ts("«на какую сумму он нам выставил» значит сложить, но только строки этого контрагента: нужна функция с условием"),
                Ask::CountBy => ts("«сколько раз … выставлял» значит посчитать строки, но только этого контрагента: нужна функция с условием"),
                Ask::AverageBy => ts("«обычно счёт у» значит среднее, но только по строкам этого контрагента: нужна функция с условием"),
                Ask::Lookup => ts("«по счёту №» значит найти одну строку по номеру и взять из неё сумму"),
            };
        }
        match self {
            Ask::Total => ts("«всего денег» значит сложить все суммы"),
            Ask::Count => ts("«сколько счетов» значит посчитать строки, а не сложить деньги"),
            Ask::Max => ts("«самый крупный» значит наибольшее число"),
            Ask::Min => ts("«самый маленький» значит наименьшее число"),
            Ask::Average => ts("«средняя» или «обычно» значит среднее"),
            Ask::SumBy => ts("«денег» значит сложить, а «у контрагента» значит не все строки, а только его: нужна функция с условием"),
            Ask::CountBy => ts("«сколько счетов» значит посчитать строки, а «у контрагента» значит только его: нужна функция с условием"),
            Ask::AverageBy => ts("«средняя» значит среднее, а «у контрагента» значит только по его строкам: нужна функция с условием"),
            Ask::Lookup => ts("«счёт с номером» значит найти одну строку по коду и взять из неё сумму"),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Ask::Total => ts("Всего:"),
            Ask::Count => ts("Счетов:"),
            Ask::Max => ts("Крупнейший:"),
            Ask::Min => ts("Самый мелкий:"),
            Ask::Average => ts("Средний:"),
            Ask::SumBy => ts("Сумма по нему:"),
            Ask::CountBy => ts("Его счетов:"),
            Ask::AverageBy => ts("Его средний:"),
            Ask::Lookup => ts("Сумма счёта:"),
        }
    }
}

const TOTALS: &[Ask] = &[Ask::Total, Ask::Count, Ask::Max, Ask::Min, Ask::Average, Ask::SumBy, Ask::CountBy, Ask::AverageBy];
const WITH_LOOKUP: &[Ask] =
    &[Ask::Total, Ask::Count, Ask::Max, Ask::Average, Ask::SumBy, Ask::CountBy, Ask::AverageBy, Ask::Lookup, Ask::Lookup, Ask::Lookup];

/// After totals and conditions: СУММ, СЧЁТ, МАКС, МИН, СРЗНАЧ and their «…ЕСЛИ» forms.
pub fn totals(g: &mut Gen) -> Result<Task> {
    build(g, TOTALS)
}

/// After lookup: the same choice, now with ВПР among the candidates.
pub fn with_lookup(g: &mut Gen) -> Result<Task> {
    build(g, WITH_LOOKUP)
}

fn build(g: &mut Gen, pool: &[Ask]) -> Result<Task> {
    let (t, parties) = super::drills::invoices(g, ts("Счета от поставщиков"), 3);
    let party_name = parties.first().copied().unwrap_or(ts(PARTIES[0]));

    // Which questions: one, or three different ones at level 4. Layout seed only.
    let count = if g.level >= 4 { 3 } else { 1 };
    let mut asks: Vec<Ask> = Vec::new();
    while asks.len() < count {
        let a = pool.get(g.l.below(pool.len())).copied().unwrap_or(Ask::Total);
        if !asks.contains(&a) {
            asks.push(a);
        }
    }

    let mut offset = 1;
    let mut party = String::new();
    if asks.iter().any(|a| matches!(a, Ask::SumBy | Ask::CountBy | Ask::AverageBy)) {
        party = g.param(&t, offset, ts("Контрагент:"), Cell::Text(party_name.into()));
        offset += 1;
    }
    let row = t.first + g.l.below(t.n() as usize) as u32;
    let number_value = match g.setup.value_of(&t.cell(1, row)) {
        Some(Cell::Num(v)) => *v,
        _ => 0.0,
    };
    let mut number = String::new();
    if asks.contains(&Ask::Lookup) {
        number = g.param(&t, offset, ts("Счёт №:"), Cell::Num(number_value));
        offset += 1;
    }
    let p = Params { party, party_name, number, number_value };

    let mut cells = Vec::new();
    let mut answers = Vec::new();
    for (i, a) in asks.iter().enumerate() {
        let cell = g.answer(&t, offset + i as u32, a.label());
        let reference = a.reference(&t, &p);
        g.formula_check(&cell, &reference);
        answers.push(format!("{cell}: {}", ru(&reference)));
        cells.push(cell);
    }

    let first = cells.first().cloned().unwrap_or_default();
    let prompt = match g.level {
        1 => {
            let a = asks.first().copied().unwrap_or(Ask::Total);
            let mut options = vec![a];
            options.extend(a.confusable());
            g.l.shuffle(&mut options);
            let list: Vec<String> = options.iter().map(|o| format!("• {} - {}", o.function(), o.does())).collect();
            tr(&format!("{} Выбери подходящую функцию и напиши формулу в {first}:\n{}", a.plain(&p), list.join("\n")))
        }
        2 => tr(&format!("{} Ответ - формулой в {first}.", asks.first().map(|a| a.plain(&p)).unwrap_or_default())),
        3 => tr(&format!("{} Ответ - формулой в {first}.", asks.first().map(|a| a.work(&p)).unwrap_or_default())),
        _ => {
            let list: Vec<String> = asks.iter().zip(&cells).map(|(a, c)| format!("• {c}: {}", a.work(&p))).collect();
            tr(&format!("Три вопроса по одной таблице, на каждый своя функция:\n{}", list.join("\n")))
        }
    };

    let work = g.level >= 3;
    let mut task = g.task(ts("Какую функцию выбрать"), prompt, None, first);
    // The hints teach reading the question, not the syntax of one function.
    task.hints = vec![
        tr(&format!("Ищи в вопросе слова-подсказки: {}.", asks.iter().map(|a| a.cue(work)).collect::<Vec<_>>().join("; "))),
        tr(&format!("Подходит: {}.", asks.iter().map(|a| format!("{} ({})", a.function(), a.does())).collect::<Vec<_>>().join("; "))),
        tr(&format!("Ответ: {}", answers.join("\n"))),
    ];
    task.goal_seconds += 10.0 * count as f32;
    Ok(task)
}
