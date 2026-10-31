//! Declarative supply/procurement track. Stages organise existing generators;
//! they do not award proficiency for reading instructions or choosing an answer.
//! Workbook tasks are procedural: enter, fill, inspect and independently verify.

/// Help fades within each stage, not just as the track advances.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Support {
    WorkedExample,
    ToolHint,
    Independent,
    Transfer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Practice {
    pub level: u8,
    pub support: Support,
    pub instruction: &'static str,
}

/// These map directly to the four existing drill levels. UI should never show
/// the worked formula in independent/transfer practice before submission.
pub const PRACTICE: &[Practice] = &[
    Practice {
        level: 1, support: Support::WorkedExample, instruction: "Повтори действие в книге, затем объясни назначение каждой ссылки."
    },
    Practice {
        level: 2, support: Support::ToolHint, instruction: "Повтори на новых данных: инструмент назван, готовой формулы нет."
    },
    Practice {
        level: 3, support: Support::Independent, instruction: "Реши рабочий вопрос без шаблона и проверь результат вручную на одной строке."
    },
    Practice {
        level: 4, support: Support::Transfer, instruction: "Перенеси способ на другую раскладку и новые данные. Проверь границы диапазонов."
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Transfer {
    pub task: &'static str,
    /// Independent control, not a restatement of the submitted formula.
    pub verification: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stage {
    pub id: &'static str,
    pub title: &'static str,
    /// Existing skill ids, in teaching order. Final stage uses a case id instead.
    pub skills: &'static [&'static str],
    /// Stage ids; all prerequisites precede this stage in SUPPLY.
    pub prerequisites: &'static [&'static str],
    pub practice: &'static [Practice],
    pub transfer: Transfer,
    pub independent_expectation: &'static str,
    pub case: Option<&'static str>,
}

pub const SUPPLY: &[Stage] = &[
    Stage {
        id: "supply.01",
        title: "Ввести и найти заказ",
        skills: &["start.enter", "keys.jump"],
        prerequisites: &[],
        practice: PRACTICE,
        transfer: Transfer {
            task: "Введи три заказа: артикул, количество, статус. Перейди к последней строке и обратно без мыши.",
            verification: "Сверь три артикула и количества с исходным заданием, посчитай строки.",
        },
        independent_expectation: "Самостоятельно вводит данные, выбирает ячейку и находит границы реестра.",
        case: None,
    },
    Stage {
        id: "supply.02",
        title: "Рассчитать одну позицию",
        skills: &["start.mul", "start.add", "start.sub", "start.div"],
        prerequisites: &["supply.01"],
        practice: PRACTICE,
        transfer: Transfer {
            task: "Посчитай стоимость позиции по цене и количеству, затем измени количество и проверь пересчёт.",
            verification: "Сравни произведение вручную; формула должна ссылаться на данные, а не содержать прежние числа.",
        },
        independent_expectation: "Сам выбирает исходные ячейки и понимает автоматический пересчёт.",
        case: None,
    },
    Stage {
        id: "supply.03",
        title: "Обработать весь заказ",
        skills: &["refs.fill", "keys.filldown"],
        prerequisites: &["supply.02"],
        practice: PRACTICE,
        transfer: Transfer {
            task: "Напиши одну формулу и заполни остальные строки выделением диапазона и Ctrl+D.",
            verification: "Проверь формулы первой, средней и последней строк; каждая должна брать свою цену и количество.",
        },
        independent_expectation: "Выделяет нужный диапазон и переносит относительные ссылки без ручного расчёта каждой строки.",
        case: None,
    },
    Stage {
        id: "supply.04",
        title: "Проверить итог заказа",
        skills: &["fn.sum", "keys.autosum"],
        prerequisites: &["supply.03"],
        practice: PRACTICE,
        transfer: Transfer {
            task: "Посчитай итог, найди пропущенную строку и проверь итог после изменения количества.",
            verification: "Сверь границы диапазона и ручную сумму небольшой выборки; не включай итоговую ячейку в себя.",
        },
        independent_expectation: "Самостоятельно получает итог и диагностирует ошибочные границы диапазона.",
        case: None,
    },
    Stage {
        id: "supply.05",
        title: "Применить общие условия",
        skills: &["refs.abs", "keys.f4", "fn.round", "refs.mixed", "supply.need"],
        prerequisites: &["supply.04"],
        practice: PRACTICE,
        transfer: Transfer {
            task: "Примени общую ставку или курс ко всем строкам; в отдельной вариации заполни расчёт в двух направлениях.",
            verification: "Проверь закрепление общей ячейки, изменение курса и разницу между округлением и форматом.",
        },
        independent_expectation: "Отличает округление значения от отображения двух знаков и контролирует диапазон итога.",
        case: None,
    },
    Stage {
        id: "supply.06",
        title: "Разобрать реестр",
        skills: &["data.sort", "data.filter", "data.freeze", "data.format", "keys.filter"],
        prerequisites: &["supply.05"],
        practice: PRACTICE,
        transfer: Transfer {
            task: "Отсортируй весь реестр по сумме и оставь нужные заказы фильтром.",
            verification: "Проверь связность артикула, количества и цены; после снятия фильтра все строки должны сохраниться.",
        },
        independent_expectation: "Сортирует строки целиком и отличает скрытые строки от удалённых.",
        case: None,
    },
    Stage {
        id: "supply.07",
        title: "Проверить сроки поставок",
        skills: &["date.due", "date.diff", "logic.if", "logic.andor", "supply.overdue"],
        prerequisites: &["supply.06"],
        practice: PRACTICE,
        transfer: Transfer {
            task: "Выяви просроченные незавершённые заказы, не отмечая доставленные как задержанные.",
            verification: "Проверь случаи до срока, в день срока и после срока; отдельно проверь уже доставленный заказ.",
        },
        independent_expectation: "Сопоставляет дату и состояние заказа, объясняет границу просрочки.",
        case: None,
    },
    Stage {
        id: "supply.08",
        title: "Суммы по поставщикам",
        skills: &["cond.sumif", "cond.countif", "fn.minmax", "fn.average", "fn.count", "check.counttext", "check.reconcile", "choose.totals"],
        prerequisites: &["supply.05", "supply.07"],
        practice: PRACTICE,
        transfer: Transfer {
            task: "Собери сумму закупок и число заказов по каждому поставщику; критерий возьми из отдельной ячейки.",
            verification: "Отбери строки одного поставщика вручную. Сумма всех поставщиков должна равняться общему итогу, число заказов — числу строк.",
        },
        independent_expectation: "Согласует диапазоны критерия и суммы; различает сумму денег и число заказов.",
        case: None,
    },
    Stage {
        id: "supply.09",
        title: "Поставщик и статус вместе",
        skills: &["cond.sumifs", "cond.countifs"],
        prerequisites: &["supply.08"],
        practice: PRACTICE,
        transfer: Transfer {
            task: "Построй срез суммы и количества заказов для пары поставщик/статус, включая комбинацию без заказов.",
            verification: "Вручную выбери строки, удовлетворяющие обоим условиям. Проверь ноль для пустой группы и сумму всех групп.",
        },
        independent_expectation: "Комбинирует условия через И, проверяет пустые группы и не удваивает строки в итогах.",
        case: None,
    },
    Stage {
        id: "supply.10",
        title: "Связать каталог и заказы",
        skills: &[
            "lookup.vlookup",
            "lookup.error_observe",
            "lookup.iferror.wrap",
            "lookup.iferror.combine",
            "lookup.xlookup",
            "lookup.index",
            "lookup.missing",
            "check.catalog",
            "logic.iferror",
            "text.trim",
            "check.spaces",
            "text.number",
            "choose.lookup",
        ],
        prerequisites: &["supply.09"],
        practice: PRACTICE,
        transfer: Transfer {
            task: "Подтяни цену и поставщика по артикулу, обработай отсутствующий код и текстовую цену.",
            verification: "Сравни три артикула с каталогом; проверь точное совпадение, закрепление диапазона и явное сообщение об отсутствующем коде.",
        },
        independent_expectation: "Сам выбирает точный поиск, не скрывает неизвестный артикул нулевой ценой.",
        case: None,
    },
    Stage {
        id: "supply.11",
        title: "Сводная по поставщику и статусу",
        skills: &["data.pivot"],
        prerequisites: &["supply.09", "supply.10"],
        practice: PRACTICE,
        transfer: Transfer {
            task: "Создай настоящую сводную: поставщик в строках, статус в столбцах, сумма заказа в значениях; затем обнови после изменения источника в копии.",
            verification: "Сравни одну группу с ручной выборкой и общий итог с СУММ. Проверь СУММА вместо СЧЁТ и включение последней строки.",
        },
        independent_expectation: "Сам задаёт источник, поля и агрегат. В итоговом кейсе проверяются объект сводной и контрольные суммы; её обновление требует отдельного наблюдаемого испытания.",
        case: None,
    },
    Stage {
        id: "supply.12",
        title: "Самостоятельный кейс снабженца",
        skills: &[],
        prerequisites: &["supply.10", "supply.11"],
        practice: &[],
        transfer: Transfer {
            task: "Реши case.procurement: каталог → заказы → живые суммы → СУММЕСЛИ → сводная поставщик/статус, без готовых формул.",
            verification: "Сверь три строки с каталогом вручную, сравни общий итог с суммой поставщиков и групп статусов. В копии измени цену и количество; проверь пересчёт и обновление сводной.",
        },
        independent_expectation: "Сдаёт живые формулы, настоящий объект сводной и числовой контроль, объясняет проверку и самостоятельно исправляет расхождения.",
        case: Some("case.procurement"),
    },
];

pub fn stage(id: &str) -> Option<&'static Stage> {
    SUPPLY.iter().find(|stage| stage.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn supply_has_twelve_ordered_stages_with_registered_content() {
        assert_eq!(SUPPLY.len(), 12);
        let mut seen = BTreeSet::new();
        for stage in SUPPLY {
            assert!(!stage.title.is_empty());
            assert!(!stage.independent_expectation.is_empty());
            assert!(!stage.transfer.task.is_empty());
            assert!(!stage.transfer.verification.is_empty());
            assert!(stage.prerequisites.iter().all(|id| seen.contains(id)), "{} prerequisites", stage.id);
            assert!(seen.insert(stage.id), "duplicate stage id");
            for id in stage.skills {
                assert!(crate::content::skill(id).is_some(), "unregistered skill {id}");
            }
            if let Some(id) = stage.case {
                assert!(crate::content::cases::case(id).is_some());
                assert!(stage.skills.is_empty());
                assert!(stage.practice.is_empty());
            } else {
                assert!(!stage.skills.is_empty());
                assert_eq!(stage.practice, PRACTICE);
            }
        }
        assert_eq!(SUPPLY.last().unwrap().case, Some("case.procurement"));
        assert!(stage("missing").is_none());
    }

    #[test]
    fn scaffolding_fades_to_independent_transfer() {
        assert_eq!(PRACTICE.iter().map(|p| p.level).collect::<Vec<_>>(), vec![1, 2, 3, 4]);
        assert_eq!(
            PRACTICE.iter().map(|p| p.support).collect::<Vec<_>>(),
            vec![Support::WorkedExample, Support::ToolHint, Support::Independent, Support::Transfer]
        );
    }
}
