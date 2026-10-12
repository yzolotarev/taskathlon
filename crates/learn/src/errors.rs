//! Error literacy: notice → diagnose → repair on fresh data. Seeded task errors are
//! never treated as learner mistakes; discovery reads committed answer cells only.
use crate::{
    Check, Choice, ChoiceOption, Kind, Mode, Profile, Result, Spec, Task,
    sheet::{self, Cell, Setup},
};
use gridcraft_engine::{
    Session,
    core::{CellError, Value},
};
use gridcraft_tr::{t as tr, ts};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Encounter {
    pub code: String,
    pub cell: String,
}

pub fn explanation(code: &str) -> &'static str {
    match code {
        "#NAME?" => ts(
            "Имя не распознано. Проверь название функции или диапазона; текст в формуле заключай в кавычки. Это также может быть функция, которую движок пока не поддерживает.",
        ),
        "#DIV/0!" => ts(
            "Делитель равен нулю или ссылается на пустую ячейку. Проверь данные и реши, что означает отсутствие делителя; не подменяй результат нулём автоматически.",
        ),
        "#VALUE!" => ts("Операция получила неподходящий тип данных, например текст вместо числа. Проверь значения и аргументы функции."),
        "#REF!" => ts("Ссылка недействительна, например после удаления ячеек. Найди повреждённую ссылку и восстанови её по смыслу расчёта."),
        "#NUM!" => ts("Числовой аргумент или результат недопустим: например корень из отрицательного числа. Проверь область допустимых значений."),
        "#N/A" => ts(
            "Искомое значение не найдено. Проверь ключ и диапазон поиска. Отсутствие записи может быть нормальным результатом; обработай его явно.",
        ),
        "#NULL!" => {
            ts("Указанные диапазоны не пересекаются. Пробел между ссылками означает пересечение, поэтому проверь разделители и нужный диапазон.")
        }
        "#SPILL!" => ts("Результату массива не хватает свободных ячеек. Проверь область вывода и препятствия, прежде чем менять формулу."),
        "#CALC!" => ts(
            "Движок не смог получить допустимый результат массива, например фильтр не нашёл строк. Проверь условия и обработку пустого результата.",
        ),
        "#CIRC!" => ts("Формула прямо или через другие ячейки ссылается на себя. Проследи цепочку ссылок и разорви цикл."),
        "#GETTING_DATA" => {
            ts("Данные ещё загружаются. Дождись завершения и проверь источник; временное ожидание не всегда означает неверную формулу.")
        }
        _ => ts("Проверь формулу и её исходные данные."),
    }
}

/// A first step to inspect; the full explanation remains available on request.
pub fn short_explanation(code: &str) -> &'static str {
    match code {
        "#NAME?" => ts("Проверь название функции или диапазона."),
        "#DIV/0!" => ts("Проверь делитель: он не должен быть нулём или пустым."),
        "#VALUE!" => ts("Проверь тип данных: для расчёта может требоваться число."),
        "#REF!" => ts("Проверь ссылки на удалённые или перемещённые ячейки."),
        "#NUM!" => ts("Проверь допустимые значения числовых аргументов."),
        "#N/A" => ts("Проверь ключ и диапазон поиска."),
        "#NULL!" => ts("Проверь разделители и пересечение диапазонов."),
        "#SPILL!" => ts("Проверь, свободны ли ячейки для результата массива."),
        "#CALC!" => ts("Проверь условия и возможность пустого результата."),
        "#CIRC!" => ts("Проверь, не ссылается ли формула на саму себя."),
        "#GETTING_DATA" => ts("Дождись загрузки и проверь источник данных."),
        _ => ts("Проверь формулу и её исходные данные."),
    }
}

pub fn discover(profile: &mut Profile, task: &Task, session: &Session) -> Vec<Encounter> {
    if task.spec.skill.starts_with("errors.") || task.sql.is_some() {
        return Vec::new();
    }
    let mut found = Vec::new();
    for cell in task.answer_cells() {
        let Ok(Value::Error(error)) = sheet::value(session, &cell) else { continue };
        // Some exercises deliberately start with a formula demonstrating an error.
        if let Some(Cell::Formula(original)) = task.setup.value_of(&cell) {
            if sheet::probe(&task.setup, &[original]).is_ok_and(|v| matches!(v.first(), Some(Value::Error(_)))) {
                continue;
            }
        }
        let code = error.as_str().to_owned();
        if !profile.seen_errors.contains(&code) {
            profile.seen_errors.push(code.clone());
            profile.pending_errors.push(code.clone());
            found.push(Encounter { code, cell });
        }
    }
    found
}

pub fn skill_id(code: &str) -> String {
    format!("errors.{}", CellError::parse(code).map(|e| e.code()).unwrap_or(0))
}

pub fn build(spec: &Spec) -> Result<Task> {
    let error = CellError::ALL
        .into_iter()
        .chain([CellError::Circ])
        .find(|e| skill_id(e.as_str()) == spec.skill)
        .ok_or_else(|| crate::LearnError::Invalid("Неизвестная ошибка для тренировки".into()))?;
    let n = 20.0 + (spec.data % 80) as f64;
    let (broken, reference, instruction) = match error {
        CellError::Name => ("=SUUM(B1:B2)", "=SUM(B1:B2)", "В примере опечатка в имени функции. В B4 посчитай сумму B1 и B2 правильной функцией."),
        CellError::Div0 => ("=B1/0", "=B1/B2", "В примере делят на ноль. По условию делитель — число в B2. В B4 запиши правильное деление."),
        CellError::Value => {
            ("=B1+\"текст\"", "=B1+B2", "В примере складывают число и текст. По условию нужны два числа B1 и B2. Исправь расчёт в B4.")
        }
        CellError::Ref => {
            ("=B1+#REF!", "=B1+B2", "После удаления данных потерялась ссылка на второе слагаемое. Оно теперь в B2. Восстанови сумму в B4.")
        }
        CellError::Num => ("=SQRT(-B1)", "=SQRT(B1)", "Нужен корень из положительного B1. В примере лишний минус. Исправь формулу в B4."),
        CellError::NA => (
            "=MATCH(-1,B1:B2,0)",
            "=MATCH(B2,B1:B2,0)",
            "Ключ -1 отсутствует. По условию ищем число из B2 в B1:B2. В B4 найди его позицию точным поиском.",
        ),
        CellError::Null => (
            "=SUM(B1 B2)",
            "=SUM(B1:B2)",
            "В примере пробел означает пересечение непересекающихся ячеек. Нужна сумма диапазона B1:B2. Запиши её в B4.",
        ),
        CellError::Circ => ("=B4+B1", "=B1+B2", "Для B4 ссылка на B4 создаёт цикл. По условию складываем исходные B1 и B2. Запиши расчёт без цикла."),
        CellError::Spill => (
            "=SEQUENCE(2)",
            "=SUM(B1:B2)",
            "Массив из двух строк не помещается: под ним занята ячейка. Здесь нужен один итог — сумма B1:B2. В B4 запиши формулу с одним результатом.",
        ),
        CellError::Calc => (
            "=FILTER(B1:B2,B1:B2<0)",
            "=IF(COUNTIF(B1:B2,\"<0\")=0,\"Нет записей\",\"Есть записи\")",
            "Фильтр отрицательных чисел может вернуть пустой массив. В B4 проверь число отрицательных значений: если их нет, верни текст «Нет записей», иначе «Есть записи».",
        ),
        CellError::GettingData => (
            "#GETTING_DATA",
            "=SUM(B1:B2)",
            "Источник завершил загрузку: числовые данные уже в B1 и B2. В B4 посчитай итог. При ожидании загрузки формулу переписывать не требуется.",
        ),
    };
    let mut setup = Setup::default();
    setup.text("A1", ts("Первое число"));
    setup.input("B1", Cell::Num(if error == CellError::Calc && spec.data % 2 == 1 { -n } else { n }));
    setup.text("A2", ts("Второе число"));
    setup.input("B2", Cell::Num(4.0));
    setup.text("A3", ts("Пример причины"));
    setup.text("B3", format!("'{}", sheet::ru(broken)));
    setup.text("A4", ts("Твой расчёт"));
    setup.answer("B4");
    setup.width(0, 170.0);
    setup.width(1, 380.0);
    let code = error.as_str();
    let mut task = Task {
        spec: spec.clone(),
        mode: Mode::Drill,
        title: tr(&format!("Разбираем {code}")),
        prompt: tr(instruction),
        example: None,
        hints: vec![explanation(code).into(), sheet::ru(reference)],
        focus: "B4".into(),
        setup,
        checks: vec![Check {
            label: ts("Исправленный расчёт").into(),
            skill: spec.skill.clone(),
            kind: Kind::Formula { cell: "B4".into(), reference: reference.into(), live: true },
        }],
        goal_seconds: 90.0,
        reorders: false,
        brief: None,
        sql: None,
        choices: vec![Choice {
            question: tr(&format!("Что сначала проверить при {code}?")),
            skill: spec.skill.clone(),
            options: vec![
                ChoiceOption {
                    text: explanation(code).into(), ok: true, why: ts("Сначала найди причину, затем исправь расчёт по условию.").into()
                },
                ChoiceOption {
                    text: ts("Скрыть любую ошибку и всегда вернуть ноль").into(),
                    ok: false,
                    why: ts("Ноль может исказить результат. Сначала установи причину и реши, как обрабатывать её по условию.").into(),
                },
            ],
        }],
    };
    if spec.layout % 2 == 1 {
        task.choices[0].options.reverse();
    }
    Ok(task)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn encounters_are_deduplicated_and_persisted() {
        let task = Task::generate(&Spec::new("fn.sum", 1, 3)).unwrap();
        let mut session = task.session().unwrap();
        let cell = task.answer_cells()[0].clone();
        sheet::set(&mut session, &cell, &Cell::Formula("=NO_SUCH_FUNCTION(1)".into())).unwrap();
        let mut p = Profile::default();
        assert_eq!(discover(&mut p, &task, &session)[0].code, "#NAME?");
        let saved = serde_json::to_vec(&p).unwrap();
        let mut restored: Profile = serde_json::from_slice(&saved).unwrap();
        assert!(discover(&mut restored, &task, &session).is_empty());
        assert_eq!(restored.pending_errors, vec!["#NAME?"]);
        sheet::set(&mut session, &cell, &Cell::Formula("=1/0".into())).unwrap();
        assert_eq!(discover(&mut restored, &task, &session)[0].code, "#DIV/0!");
    }
    #[test]
    fn every_supported_error_has_a_solvable_repair() {
        for error in CellError::ALL.into_iter().chain([CellError::Circ]) {
            let spec = Spec::new(&skill_id(error.as_str()), 1, 5);
            let task = Task::generate(&spec).unwrap();
            let mut session = task.session().unwrap();
            assert!(discover(&mut Profile::default(), &task, &session).is_empty());
            assert!(!crate::evaluate(&task, &session, &[]).unwrap().passed());
            let Kind::Formula { reference, .. } = &task.checks[0].kind else { panic!("formula expected") };
            sheet::set(&mut session, "B4", &Cell::Formula(reference.clone())).unwrap();
            assert!(crate::evaluate(&task, &session, &[]).unwrap().passed(), "{}", error.as_str());
            // A hardcoded number must not demonstrate the ability to repair formulas.
            sheet::set(&mut session, "B4", &Cell::Num(0.0)).unwrap();
            assert!(!crate::evaluate(&task, &session, &[]).unwrap().passed());
        }
    }
}
