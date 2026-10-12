//! Blitz: a timed multiple-choice test in the style of retail and bank screening bots
//! (formulas, shortcuts, pivots, reading numbers, business conclusions, SQL).
//! Every question is generated from a seed and its answer is computed, not stored.

use crate::rng::Rng;
use crate::sheet::{date, date_text, num_text};
use crate::sql::{self, SqlTable, SqlVal};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Question {
    pub topic: String,
    pub prompt: String,
    /// A small table shown with the question (first row is the header).
    pub table: Vec<Vec<String>>,
    pub options: Vec<String>,
    pub correct: usize,
    pub seconds: u32,
    /// Path skill to practise after a miss.
    pub skill: String,
    pub explain: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Track {
    Excel,
    Sql,
    Mixed,
}

impl Track {
    pub fn title(self) -> &'static str {
        match self {
            Self::Excel => "Excel",
            Self::Sql => "SQL",
            Self::Mixed => "Excel + SQL",
        }
    }
}

/// Shuffles `options` (first = correct) and returns the question.
fn q(
    r: &mut Rng,
    topic: &str,
    prompt: String,
    table: Vec<Vec<String>>,
    mut options: Vec<String>,
    seconds: u32,
    skill: &str,
    explain: String,
) -> Question {
    let mut seen: Vec<String> = Vec::new();
    options.retain(|o| {
        if seen.contains(o) {
            false
        } else {
            seen.push(o.clone());
            true
        }
    });
    let right = options.first().cloned().unwrap_or_default();
    r.shuffle(&mut options);
    let correct = options.iter().position(|o| *o == right).unwrap_or(0);
    Question { topic: topic.into(), prompt, table, options, correct, seconds, skill: skill.into(), explain }
}

fn n(v: f64) -> String {
    num_text(v)
}

const CITIES: &[&str] = &["Москва", "Казань", "Самара", "Тула"];

fn mini_sales(r: &mut Rng) -> (Vec<Vec<String>>, Vec<(&'static str, f64)>) {
    let mut rows = vec![vec!["".into(), "A".into(), "B".into()], vec!["1".into(), "Город".into(), "Сумма".into()]];
    let mut data = Vec::new();
    for i in 0..5 {
        let c = CITIES.get(r.below(3)).copied().unwrap_or("Москва");
        let v = (r.int(2, 40) * 500) as f64;
        rows.push(vec![(i + 2).to_string(), c.into(), n(v)]);
        data.push((c, v));
    }
    (rows, data)
}

fn sumif(r: &mut Rng) -> Question {
    let (table, data) = mini_sales(r);
    let city = data.first().map(|d| d.0).unwrap_or("Москва");
    let want: f64 = data.iter().filter(|d| d.0 == city).map(|d| d.1).sum();
    let all: f64 = data.iter().map(|d| d.1).sum();
    let cnt = data.iter().filter(|d| d.0 == city).count() as f64;
    let other: f64 = data.iter().filter(|d| d.0 != city).map(|d| d.1).sum();
    q(
        r,
        "Формулы",
        format!("Что вернёт =СУММЕСЛИ(A2:A6;\"{city}\";B2:B6)?"),
        table,
        vec![n(want), n(all), n(cnt), n(other)],
        35,
        "cond.sumif",
        format!("Складываются суммы только строк с «{city}»: {}.", n(want)),
    )
}

fn countif(r: &mut Rng) -> Question {
    let (table, data) = mini_sales(r);
    let x = (r.int(4, 14) * 1000) as f64;
    let want = data.iter().filter(|d| d.1 > x).count();
    let ge = data.iter().filter(|d| d.1 >= x).count();
    q(
        r,
        "Формулы",
        format!("Что вернёт =СЧЁТЕСЛИ(B2:B6;\">{}\")?", x as i64),
        table,
        vec![want.to_string(), (5 - want).to_string(), (want + 1).to_string(), ge.max(want + 2).to_string()],
        30,
        "cond.countif",
        format!("Считаются ячейки строго больше {}: их {want}.", x as i64),
    )
}

fn vlookup(r: &mut Rng) -> Question {
    let mut codes: Vec<i64> = (0..4).map(|i| 100 + i * 7 + r.int(0, 5)).collect();
    codes.sort();
    let prices: Vec<f64> = (0..4).map(|_| (r.int(10, 99) * 10) as f64).collect();
    let mut table = vec![vec!["".into(), "D".into(), "E".into()], vec!["1".into(), "Код".into(), "Цена".into()]];
    for (i, (c, p)) in codes.iter().zip(&prices).enumerate() {
        table.push(vec![(i + 2).to_string(), c.to_string(), n(*p)]);
    }
    if r.chance(30) {
        let missing = codes.last().copied().unwrap_or(120) + 3;
        return q(
            r,
            "Формулы",
            format!("Что вернёт =ВПР({missing};D2:E5;2;ЛОЖЬ)?"),
            table,
            vec!["#Н/Д".into(), "0".into(), n(prices.last().copied().unwrap_or(0.0)), "#ЗНАЧ!".into()],
            35,
            "lookup.missing",
            "Точный поиск (ЛОЖЬ) не нашёл код — ВПР возвращает #Н/Д. Лечится ЕСЛИОШИБКА.".into(),
        );
    }
    let i = r.below(4);
    let code = codes.get(i).copied().unwrap_or(100);
    let price = prices.get(i).copied().unwrap_or(0.0);
    let next = prices.get((i + 1) % 4).copied().unwrap_or(0.0);
    q(
        r,
        "Формулы",
        format!("Что вернёт =ВПР({code};D2:E5;2;ЛОЖЬ)?"),
        table,
        vec![n(price), n(next), code.to_string(), "#Н/Д".into()],
        30,
        "lookup.vlookup",
        format!("Код {code} найден в строке {}, второй столбец — цена {}.", i + 2, n(price)),
    )
}

const HOTKEYS: &[(&str, &str)] = &[
    ("Включить или выключить фильтр", "Ctrl+Shift+L"),
    ("Автосумма", "Alt+="),
    ("Переключить ссылку на абсолютную ($)", "F4"),
    ("Вставить сегодняшнюю дату", "Ctrl+;"),
    ("Открыть «Формат ячеек»", "Ctrl+1"),
    ("Скопировать формулу из верхней ячейки вниз", "Ctrl+D"),
    ("Выделить весь столбец", "Ctrl+Пробел"),
    ("Выделить всю строку", "Shift+Пробел"),
    ("Перейти к последней заполненной ячейке вниз", "Ctrl+↓"),
    ("Редактировать активную ячейку", "F2"),
    ("Найти", "Ctrl+F"),
    ("Заменить", "Ctrl+H"),
    ("Превратить диапазон в «умную» таблицу", "Ctrl+T"),
    ("Вставить новый лист", "Shift+F11"),
    ("Удалить выделенные строки", "Ctrl+-"),
    ("Скрыть строки", "Ctrl+9"),
    ("Жирный шрифт", "Ctrl+B"),
    ("Сохранить как", "F12"),
];

fn hotkey(r: &mut Rng) -> Question {
    let i = r.below(HOTKEYS.len());
    let (what, key) = HOTKEYS.get(i).copied().unwrap_or(("Жирный шрифт", "Ctrl+B"));
    let mut opts = vec![key.to_string()];
    while opts.len() < 4 {
        let (_, k) = HOTKEYS.get(r.below(HOTKEYS.len())).copied().unwrap_or(("", "Ctrl+B"));
        if !opts.iter().any(|o| o == k) {
            opts.push(k.into());
        }
    }
    let skill = match key {
        "Ctrl+Shift+L" => "keys.filter",
        "Alt+=" => "keys.autosum",
        "F4" => "keys.f4",
        "Ctrl+;" => "keys.date",
        "Ctrl+D" => "keys.filldown",
        "Ctrl+↓" => "keys.jump",
        _ => "keys.bold",
    };
    q(r, "Горячие клавиши", format!("{what} — какое сочетание?"), Vec::new(), opts, 20, skill, format!("{what}: {key}."))
}

fn references(r: &mut Rng) -> Question {
    let dr = r.int(1, 4);
    let dc = r.int(1, 2);
    let col = |c: i64| ((b'A' + c as u8) as char).to_string();
    let (src_a, src_b) = (0i64, 1i64);
    let row = 2i64;
    let target = format!("{}{}", col(2 + dc), row + dr);
    let right = format!("={}{}*$F$1", col(src_a + dc), row + dr);
    let opts = vec![
        right.clone(),
        format!("={}{}*$F${}", col(src_a + dc), row + dr, 1 + dr),
        format!("=A{row}*$F$1"),
        format!("={}{}*$F$1", col(src_b + dc), row),
    ];
    q(
        r,
        "Ссылки",
        format!("В C{row} формула =A{row}*$F$1. Её скопировали в {target}. Что там будет?"),
        Vec::new(),
        opts,
        35,
        "refs.abs",
        format!("Относительная часть сдвигается на {dr} строк и {dc} столбцов, $F$1 закреплена: {right}."),
    )
}

fn percent(r: &mut Rng) -> Question {
    let before = (r.int(40, 200) * 1000) as f64;
    let pct = r.int(-30, 45);
    let pct = if pct == 0 { 12 } else { pct };
    let after = before * (100 + pct) as f64 / 100.0;
    q(
        r,
        "Расчёты",
        format!("Выручка была {} ₽, стала {} ₽. На сколько процентов она изменилась?", n(before), n(after)),
        Vec::new(),
        vec![format!("{pct:+}%"), format!("{:+}%", -pct), format!("{:+.1}%", (after - before) / after * 100.0), format!("{:+}%", pct * 2)],
        40,
        "start.div",
        format!("(стало − было) / было = {} / {} = {pct:+}%.", n(after - before), n(before)),
    )
}

fn avg_check(r: &mut Rng) -> Question {
    let rev = r.int(5, 25);
    let ord = r.int(10, 40);
    let rev = if rev == ord { rev + 5 } else { rev };
    let change = ((100 + rev) as f64 / (100 + ord) as f64 - 1.0) * 100.0;
    let right = if change < 0.0 {
        format!("Снизился примерно на {:.0}%", -change)
    } else {
        format!("Вырос примерно на {change:.0}%")
    };
    let wrong1 = if change < 0.0 {
        format!("Вырос примерно на {:.0}%", -change)
    } else {
        format!("Снизился примерно на {change:.0}%")
    };
    q(
        r,
        "Бизнес-вывод",
        format!("Выручка выросла на {rev}%, число чеков — на {ord}%. Что со средним чеком?"),
        Vec::new(),
        vec![right, wrong1, "Не изменился".into(), format!("Вырос на {}%", rev + ord)],
        45,
        "fn.round",
        format!("Средний чек = выручка / чеки: {} / {} − 1 ≈ {change:+.0}%.", (100 + rev) as f64 / 100.0, (100 + ord) as f64 / 100.0),
    )
}

fn growth_month(r: &mut Rng) -> Question {
    let months = ["Янв", "Фев", "Мар", "Апр", "Май"];
    let mut v = vec![(r.int(80, 120) * 1000) as f64];
    for i in 1..5 {
        let prev = v.get(i - 1).copied().unwrap_or(100_000.0);
        v.push(prev * (100 + r.int(-12, 25) + i as i64) as f64 / 100.0);
    }
    let v: Vec<f64> = v.iter().map(|x| x.round()).collect();
    let mut best = 1;
    for i in 1..5 {
        let g = |k: usize| v.get(k).copied().unwrap_or(1.0) / v.get(k - 1).copied().unwrap_or(1.0);
        if g(i) > g(best) {
            best = i;
        }
    }
    let max_i = v.iter().enumerate().max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal)).map(|x| x.0).unwrap_or(0);
    let table = vec![months.iter().map(|m| m.to_string()).collect(), v.iter().map(|x| n(*x)).collect()];
    let name = |i: usize| months.get(i).copied().unwrap_or("").to_string();
    let mut opts = vec![name(best)];
    for i in [max_i, (best + 1) % 5, (best + 2) % 5, (best + 3) % 5] {
        if i != best {
            opts.push(name(i));
        }
    }
    opts.truncate(4);
    q(
        r,
        "Чтение данных",
        "В каком месяце выручка выросла сильнее всего относительно предыдущего (в %)?".into(),
        table,
        opts,
        50,
        "start.div",
        format!("Считаем рост к предыдущему месяцу: максимальный — {}. Самая большая выручка — не то же самое, что самый большой рост.", name(best)),
    )
}

fn round_q(r: &mut Rng) -> Question {
    let whole = r.int(1, 99);
    let cents = r.int(101, 999);
    let x = format!("{whole},{cents:03}");
    let k = r.int(0, 2);
    // Integer arithmetic: 3 decimals → k decimals, half up.
    let v = whole * 1000 + cents;
    let div = 10i64.pow((3 - k) as u32);
    let fmt = |units: i64| {
        let scale = 10i64.pow(k as u32);
        if k == 0 { units.to_string() } else { format!("{},{:0w$}", units / scale, units % scale, w = k as usize) }
    };
    let rounded = (v + div / 2) / div;
    let trunc = v / div;
    let other = if rounded == trunc { trunc + 1 } else { trunc };
    q(
        r,
        "Формулы",
        format!("Что вернёт =ОКРУГЛ({x};{k})?"),
        Vec::new(),
        vec![fmt(rounded), fmt(other), fmt(rounded + 2), x.clone()],
        25,
        "fn.round",
        format!("ОКРУГЛ до {k} знаков по правилам арифметики: {}.", fmt(rounded)),
    )
}

fn eomonth(r: &mut Rng) -> Question {
    let m = r.int(1, 10);
    let d = r.int(2, 27);
    let start = date(2026, m, d);
    let end = date(2026, m + 1, 1) - 1.0;
    let next_end = date(2026, m + 2, 1) - 1.0;
    q(
        r,
        "Даты",
        format!("В A1 дата {}. Что вернёт =КОНМЕСЯЦА(A1;0)?", date_text(start)),
        Vec::new(),
        vec![date_text(end), date_text(next_end), date_text(date(2026, m, 1)), date_text(start + 30.0)],
        30,
        "date.eomonth",
        "КОНМЕСЯЦА(дата; 0) — последний день того же месяца.".into(),
    )
}

fn text_q(r: &mut Rng) -> Question {
    let words = [("Москва-Сити", 6), ("Казань-Арена", 6), ("Счёт-фактура", 4), ("Самара-Центр", 6)];
    let (w, k) = words.get(r.below(words.len())).copied().unwrap_or(("Москва-Сити", 6));
    let left: String = w.chars().take(k).collect();
    let right: String = w.chars().rev().take(k).collect::<Vec<_>>().into_iter().rev().collect();
    let more: String = w.chars().take(k + 1).collect();
    q(
        r,
        "Текст",
        format!("Что вернёт =ЛЕВСИМВ(\"{w}\";{k})?"),
        Vec::new(),
        vec![left.clone(), right, more, w.to_string()],
        25,
        "text.leftright",
        format!("Первые {k} символов: «{left}»."),
    )
}

const CONCEPTS: &[(&str, &str, &[&str], &str, &str)] = &[
    (
        "Сводные",
        "В сводной нужна выручка по менеджерам. Куда поставить поле «Менеджер»?",
        &["В строки", "В значения", "В фильтры", "Никуда — сводная сама поймёт"],
        "data.pivot",
        "Категория, по которой группируем, — в строки; сумма — в значения.",
    ),
    (
        "Сводные",
        "В исходную таблицу добавили строки. Что сделать, чтобы сводная их учла?",
        &[
            "Обновить сводную (и проверить диапазон источника)",
            "Ничего — она пересчитается сама",
            "Удалить и построить заново — другого способа нет",
            "Нажать F9",
        ],
        "data.pivot",
        "Сводная хранит кэш: её обновляют (Данные → Обновить все), а «умная» таблица-источник сама расширяет диапазон.",
    ),
    (
        "Ошибки",
        "Что означает ошибка #ДЕЛ/0!?",
        &["Деление на ноль или на пустую ячейку", "Значение не найдено", "Неверное имя функции", "Ссылка на удалённую ячейку"],
        "logic.iferror",
        "#ДЕЛ/0! — делитель равен нулю или пуст. Обычно лечат ЕСЛИОШИБКА или ЕСЛИ(делитель=0;…).",
    ),
    (
        "Ошибки",
        "Формула выдаёт #ИМЯ?. Самая вероятная причина?",
        &["Опечатка в имени функции или текст без кавычек", "Деление на ноль", "Не найден искомый код", "Слишком большое число"],
        "logic.iferror",
        "#ИМЯ? — Excel не узнал имя: опечатка или текст без кавычек.",
    ),
    (
        "Формулы",
        "Чем СЧЁТ отличается от СЧЁТЗ?",
        &["СЧЁТ считает только числа, СЧЁТЗ — все непустые", "Ничем", "СЧЁТЗ считает только пустые", "СЧЁТ работает только с текстом"],
        "fn.count",
        "СЧЁТ — ячейки с числами, СЧЁТЗ — любые непустые.",
    ),
    (
        "Графики",
        "Нужно показать долю каждого магазина в выручке (4 магазина). Какой график уместнее?",
        &["Круговая или линейчатая с процентами", "Точечная", "График (линия) по времени", "Биржевая"],
        "data.pivot",
        "Доли нескольких частей одного целого — круговая (если частей мало) или линейчатая.",
    ),
    (
        "Графики",
        "Нужно показать динамику выручки по месяцам за год. Какой график?",
        &["График (линия) или гистограмма по месяцам", "Круговая", "Точечная без подписей", "Лепестковая"],
        "data.pivot",
        "Динамика во времени — линия или столбцы по оси времени.",
    ),
    (
        "Данные",
        "Как быстрее всего убрать повторы в списке контрагентов?",
        &["Данные → Удалить дубликаты", "Отсортировать и удалять вручную", "Ctrl+Z", "Скрыть строки"],
        "data.dedupe",
        "Данные → Удалить дубликаты, указав столбцы для сравнения.",
    ),
    (
        "Бизнес-вывод",
        "Конверсия сайта упала с 4% до 3%, трафик вырос на 50%. Что с числом заказов?",
        &["Выросло примерно на 12,5%", "Упало на 25%", "Выросло на 50%", "Не изменилось"],
        "start.mul",
        "Заказы = трафик × конверсия: 1,5 × 0,75 = 1,125 → +12,5%.",
    ),
    (
        "Бизнес-вывод",
        "Маржа = (цена − себестоимость) / цена. Цена 200, себестоимость 150. Маржа?",
        &["25%", "33%", "50 ₽ это и есть маржа в %", "75%"],
        "start.div",
        "(200 − 150) / 200 = 25%. Наценка была бы 50 / 150 ≈ 33%.",
    ),
];

fn concept(r: &mut Rng) -> Question {
    let (topic, prompt, opts, skill, explain) = CONCEPTS.get(r.below(CONCEPTS.len())).copied().unwrap_or(CONCEPTS[0]);
    q(r, topic, prompt.into(), Vec::new(), opts.iter().map(|s| s.to_string()).collect(), 30, skill, explain.into())
}

const SQL_CONCEPTS: &[(&str, &[&str], &str, &str)] = &[
    (
        "Чем WHERE отличается от HAVING?",
        &[
            "WHERE фильтрует строки до группировки, HAVING — группы после",
            "Ничем, это синонимы",
            "HAVING работает только с JOIN",
            "WHERE нельзя использовать с GROUP BY",
        ],
        "sql.having",
        "WHERE — до GROUP BY, по строкам; HAVING — после, по агрегатам.",
    ),
    (
        "Сколько строк вернёт SELECT * FROM t WHERE x = NULL (в t есть строки с NULL)?",
        &["0", "Все строки с NULL", "Все строки", "Ошибку"],
        "sql.null",
        "Сравнение с NULL через = даёт UNKNOWN, строки не проходят. Нужно IS NULL.",
    ),
    (
        "Чем COUNT(*) отличается от COUNT(email)?",
        &["COUNT(email) не считает строки, где email IS NULL", "Ничем", "COUNT(*) не считает NULL", "COUNT(email) считает уникальные"],
        "sql.agg",
        "COUNT(столбец) пропускает NULL, COUNT(*) считает все строки.",
    ),
    (
        "В каком порядке логически выполняется запрос?",
        &[
            "FROM → WHERE → GROUP BY → HAVING → SELECT → ORDER BY",
            "SELECT → FROM → WHERE → ORDER BY",
            "WHERE → FROM → SELECT → GROUP BY",
            "FROM → SELECT → WHERE → HAVING",
        ],
        "sql.group",
        "Поэтому псевдоним из SELECT нельзя использовать в WHERE, а в ORDER BY — можно.",
    ),
    (
        "Что вернёт шаблон LIKE 'А%' ?",
        &["Строки, начинающиеся на «А»", "Строки, заканчивающиеся на «А»", "Ровно «А%»", "Строки длиной 2 символа"],
        "sql.like",
        "% — любое число любых символов, _ — ровно один символ.",
    ),
    (
        "Чем UNION отличается от UNION ALL?",
        &["UNION убирает дубликаты, UNION ALL — нет", "Ничем", "UNION ALL убирает дубликаты", "UNION работает только с одной таблицей"],
        "sql.distinct",
        "UNION = UNION ALL + DISTINCT, поэтому медленнее.",
    ),
    (
        "Клиенты без заказов: какой запрос верный?",
        &[
            "clients LEFT JOIN orders … WHERE orders.id IS NULL",
            "clients JOIN orders … WHERE orders.id IS NULL",
            "clients LEFT JOIN orders … WHERE orders.id = NULL",
            "orders LEFT JOIN clients … WHERE clients.id IS NULL",
        ],
        "sql.left",
        "LEFT JOIN сохраняет всех клиентов, у тех, кому пары нет, поля orders — NULL.",
    ),
];

fn sql_concept(r: &mut Rng) -> Question {
    let (prompt, opts, skill, explain) = SQL_CONCEPTS.get(r.below(SQL_CONCEPTS.len())).copied().unwrap_or(SQL_CONCEPTS[0]);
    q(r, "SQL", prompt.into(), Vec::new(), opts.iter().map(|s| s.to_string()).collect(), 30, skill, explain.into())
}

fn mini_orders(r: &mut Rng, rows: usize) -> SqlTable {
    let mut out = Vec::new();
    for i in 0..rows {
        let client = r.int(1, 4);
        let amount = r.int(1, 30) * 100;
        let status = ["paid", "paid", "new", "cancelled"].get(r.below(4)).copied().unwrap_or("paid");
        out.push(vec![SqlVal::Int(i as i64 + 1), SqlVal::Int(client), SqlVal::Int(amount), SqlVal::Text(status.into())]);
    }
    SqlTable {
        name: "orders".into(),
        columns: vec![
            ("id".into(), "INTEGER".into()),
            ("client_id".into(), "INTEGER".into()),
            ("amount".into(), "INTEGER".into()),
            ("status".into(), "TEXT".into()),
        ],
        rows: out,
    }
}

fn table_view(t: &SqlTable) -> Vec<Vec<String>> {
    let mut v = vec![t.columns.iter().map(|(c, _)| c.clone()).collect::<Vec<_>>()];
    v.extend(t.rows.iter().map(|r| r.iter().map(SqlVal::show).collect()));
    v
}

fn scalar(t: &SqlTable, query: &str) -> Option<String> {
    sql::run(std::slice::from_ref(t), query).ok().and_then(|rs| rs.rows.first().and_then(|r| r.first()).map(SqlVal::show))
}

fn rows_of(t: &SqlTable, query: &str) -> Option<usize> {
    sql::run(std::slice::from_ref(t), query).ok().map(|rs| rs.rows.len())
}

fn sql_result(r: &mut Rng) -> Option<Question> {
    let t = mini_orders(r, 6);
    let x = r.int(5, 20) * 100;
    let (query, skill, value) = match r.below(4) {
        0 => {
            let qy = format!("SELECT COUNT(*) FROM orders WHERE amount > {x}");
            let v = scalar(&t, &qy)?;
            (qy, "sql.where", v)
        }
        1 => {
            let qy = "SELECT SUM(amount) FROM orders WHERE status = 'paid'".to_string();
            let v = scalar(&t, &qy)?;
            (qy, "sql.agg", if v == "NULL" { "NULL".into() } else { v })
        }
        2 => {
            let qy = "SELECT client_id, COUNT(*) FROM orders GROUP BY client_id".to_string();
            let v = rows_of(&t, &qy)?.to_string();
            (format!("{qy}\n— сколько строк вернёт запрос?"), "sql.group", v)
        }
        _ => {
            let qy = "SELECT COUNT(DISTINCT client_id) FROM orders".to_string();
            let v = scalar(&t, &qy)?;
            (qy, "sql.distinct", v)
        }
    };
    let base: i64 = value.parse().unwrap_or(0);
    let step = if base >= 100 { 100 } else { 1 };
    let opts = vec![value.clone(), (base + step).to_string(), (base - step).max(0).to_string(), (base + 2 * step).to_string()];
    Some(q(
        r,
        "SQL",
        format!("Что вернёт запрос?\n{query}"),
        table_view(&t),
        opts,
        50,
        skill,
        format!("Ответ: {value}. Проверь фильтр и что именно агрегируется."),
    ))
}

/// Builds a blitz of `count` questions. SQL questions need the native SQLite engine.
pub fn blitz(seed: u64, track: Track, count: usize) -> Vec<Question> {
    let mut r = Rng::new(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15).wrapping_add(1));
    let excel: [fn(&mut Rng) -> Question; 11] =
        [sumif, countif, vlookup, hotkey, references, percent, avg_check, growth_month, round_q, eomonth, text_q];
    let mut out = Vec::new();
    let mut guard = 0;
    while out.len() < count && guard < count * 10 {
        guard += 1;
        let want_sql = match track {
            Track::Excel => false,
            Track::Sql => true,
            Track::Mixed => out.len() % 3 == 2,
        };
        let question = if want_sql {
            if r.chance(55) { sql_result(&mut r) } else { Some(sql_concept(&mut r)) }
        } else if r.chance(20) {
            Some(concept(&mut r))
        } else {
            let i = r.below(excel.len());
            excel.get(i).map(|f| f(&mut r))
        };
        if let Some(qn) = question
            && qn.options.len() >= 2
            && !out.iter().any(|o: &Question| o.prompt == qn.prompt && o.table == qn.table)
        {
            out.push(qn);
        }
    }
    out
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BlitzResult {
    pub at: u64,
    pub track: Option<Track>,
    pub correct: u32,
    pub total: u32,
    pub seconds: f32,
    pub points: u32,
}

/// Points for one answer: 10 for a correct one, plus up to 10 for speed.
pub fn points(correct: bool, used: f32, limit: u32) -> u32 {
    if !correct {
        return 0;
    }
    let left = (1.0 - used / limit.max(1) as f32).clamp(0.0, 1.0);
    10 + (left * 10.0).round() as u32
}
