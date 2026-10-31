//! Seeded exercise generators. `layout` fixes where things are and what is asked; `data`
//! fixes the numbers, so grading can re-run the learner's formulas on other numbers.

use gridcraft_tr::{t as tr, ts};
mod acc;
pub mod asm;
pub mod cases;
mod choose;
mod control;
mod drills;
mod keys;
pub mod live;
mod orion;
mod sql;
mod stock;

use crate::check::{Check, Kind};
use crate::rng::Rng;
use crate::sheet::{Cell, Setup, a1, col_name, date, ru};
use crate::{Mode, Result, Spec, Task};

pub use cases::CASES;

pub struct Unit {
    pub title: &'static str,
    pub subtitle: &'static str,
    /// RGB used for the unit band on the path.
    pub color: [u8; 3],
}

pub const UNITS: &[Unit] = &[
    Unit { title: "Старт", subtitle: "Ввод и простые формулы", color: [16, 185, 129] },
    Unit { title: "Протягивание и $", subtitle: "Одна формула на весь столбец", color: [59, 130, 246] },
    Unit { title: "Итоги", subtitle: "СУММ, СРЗНАЧ, МИН, МАКС, ОКРУГЛ", color: [168, 85, 247] },
    Unit { title: "Условия", subtitle: "ЕСЛИ, И, ИЛИ, ЕСЛИОШИБКА", color: [249, 115, 22] },
    Unit { title: "Итоги по условию", subtitle: "СУММЕСЛИ, СЧЁТЕСЛИ, …МН", color: [239, 68, 68] },
    Unit { title: "Поиск", subtitle: "ВПР, ПРОСМОТРX, ИНДЕКС+ПОИСКПОЗ", color: [43, 112, 201] },
    Unit { title: "Текст и выгрузки", subtitle: "Чистка данных из 1С", color: [0, 150, 136] },
    Unit { title: "Даты", subtitle: "Сроки, просрочка, месяцы", color: [234, 179, 8] },
    Unit { title: "Работа с таблицей", subtitle: "Сортировка, фильтр, сводная", color: [132, 204, 22] },
    Unit { title: "Горячие клавиши", subtitle: "Скорость «как робот»", color: [80, 80, 90] },
    Unit { title: "Бухучёт", subtitle: "Счета, проводки, НДС, зарплата", color: [220, 90, 160] },
    Unit { title: "SQL", subtitle: "Запросы как на собеседовании в банк и ритейл", color: [70, 90, 220] },
    Unit {
        title: "Сборка отчёта", subtitle: "Из грязных листов — в аккуратную таблицу", color: [0, 121, 107]
    },
    Unit { title: "Неделя в Орионе", subtitle: "Работа ассистента снабжения, день за днём", color: [14, 116, 144] },
    Unit { title: "Рабочие кейсы", subtitle: "Задачи без подсказок", color: [255, 120, 60] },
];

pub struct Skill {
    pub id: &'static str,
    pub unit: usize,
    pub name: &'static str,
    /// First hint: the idea, without syntax.
    pub idea: &'static str,
    /// Second hint: the Russian Excel syntax.
    pub syntax: &'static str,
    pub build: fn(&mut Gen) -> Result<Task>,
}

pub const LEVELS: [&str; 4] = ["Повтори за мной", "С подсказкой", "Сам", "Как на работе"];

macro_rules! s {
    ($id:literal, $unit:literal, $name:literal, $idea:literal, $syntax:literal, $f:path) => {
        Skill { id: $id, unit: $unit, name: $name, idea: $idea, syntax: $syntax, build: $f }
    };
}

/// The path, in teaching order.
pub const SKILLS: &[Skill] = &[
    s!("start.enter", 0, "Ввод данных", "Выдели ячейку, набери значение и нажми Enter.", "Просто набери число или текст", drills::enter),
    s!("start.add", 0, "Сложение", "Формула всегда начинается со знака =. Затем ссылки на ячейки и знак +.", "=B2+C2", drills::add),
    s!("start.sub", 0, "Вычитание", "Остаток = было − ушло. Ссылайся на ячейки, а не на числа.", "=B2-C2", drills::sub),
    s!("start.mul", 0, "Умножение", "Сумма строки = цена × количество. Умножение — звёздочка *.", "=B2*C2", drills::mul),
    s!("start.div", 0, "Деление", "Цена за штуку = сумма ÷ количество. Деление — косая черта /.", "=B2/C2", drills::div),
    s!("start.vat", 0, "Сумма с НДС", "С 2026 года НДС 22%: сумма с НДС в 1,22 раза больше. Число с запятой: 1,22.", "=B2*1,22", drills::vat),
    s!(
        "refs.fill",
        1,
        "Протянуть вниз",
        "Напиши формулу в первой строке и протяни её вниз за маленький квадрат в углу ячейки (или выдели столбец и нажми Ctrl+D).",
        "=B2*C2, затем протянуть",
        drills::fill
    ),
    s!(
        "refs.abs",
        1,
        "Закрепить $",
        "Если все строки берут одну и ту же ячейку (ставку), её адрес закрепляют знаком $. Клавиша F4 ставит $ сама.",
        "=B2*$G$1",
        drills::absolute
    ),
    s!(
        "refs.mixed",
        1,
        "Смешанные ссылки",
        "$B2 закрепляет столбец, B$1 — строку. Тогда одну формулу можно протянуть и вправо, и вниз.",
        "=$A2*B$1",
        drills::mixed
    ),
    s!("fn.sum", 2, "СУММ", "Итог столбца — функция СУММ от всего диапазона.", "=СУММ(D2:D9)", drills::sum),
    s!("fn.average", 2, "СРЗНАЧ", "Среднее — СРЗНАЧ от диапазона.", "=СРЗНАЧ(D2:D9)", drills::average),
    s!("fn.minmax", 2, "МИН и МАКС", "Самое маленькое — МИН, самое большое — МАКС.", "=МАКС(D2:D9)", drills::minmax),
    s!("fn.count", 2, "СЧЁТ и СЧЁТЗ", "СЧЁТ считает ячейки с числами, СЧЁТЗ — все непустые.", "=СЧЁТЗ(A2:A9)", drills::count),
    s!(
        "check.counttext",
        2,
        "Проверь себя: число или текст",
        "СЧЁТЗ считает все заполненные ячейки, СЧЁТ только числа. Разница - это числа, записанные текстом: СУММ их пропускает.",
        "=СЧЁТЗ(B2:B9)-СЧЁТ(B2:B9)",
        control::count_text
    ),
    s!("fn.round", 2, "ОКРУГЛ", "Деньги округляют до копеек: ОКРУГЛ(число; 2).", "=ОКРУГЛ(B2*0,22;2)", drills::round),
    s!("logic.if", 3, "ЕСЛИ", "ЕСЛИ(условие; что если да; что если нет). Текст — в кавычках.", "=ЕСЛИ(C2>=B2;\"Оплачено\";\"Долг\")", drills::if_),
    s!(
        "logic.andor",
        3,
        "И / ИЛИ",
        "И — все условия верны, ИЛИ — хотя бы одно. Ставятся внутрь ЕСЛИ.",
        "=ЕСЛИ(И(B2>=C2;D2=0);\"Да\";\"Нет\")",
        drills::and_or
    ),
    s!("logic.iferror", 3, "ЕСЛИОШИБКА", "ЕСЛИОШИБКА(формула; что показать вместо ошибки).", "=ЕСЛИОШИБКА(B2/C2;0)", drills::iferror),
    s!(
        "supply.need",
        3,
        "Сколько заказать под производство",
        "Потребность = норма на одно устройство × план выпуска. К заказу = потребность − склад − в пути, но не меньше нуля: МАКС(0; …).",
        "=B2*$I$2 и =МАКС(0;E2-C2-D2)",
        stock::need
    ),
    s!(
        "cond.sumif",
        4,
        "СУММЕСЛИ",
        "Сумма только тех строк, где выполнено условие.",
        "=СУММЕСЛИ(где_искать; что_искать; что_суммировать)",
        drills::sumif
    ),
    s!("cond.countif", 4, "СЧЁТЕСЛИ", "Сколько строк подходит под условие.", "=СЧЁТЕСЛИ(где_искать; что_искать)", drills::countif),
    s!(
        "cond.compare",
        4,
        "Условие «больше»",
        "Условие можно писать как текст: \">10000\". Или склеить знак со ссылкой: \">\"&H1.",
        "=СУММЕСЛИ(D2:D9;\">\"&H1)",
        drills::compare
    ),
    s!(
        "cond.sumifs",
        4,
        "СУММЕСЛИМН",
        "Несколько условий сразу. Сначала — что суммировать, потом пары «диапазон; условие».",
        "=СУММЕСЛИМН(что_суммировать; диапазон1; условие1; диапазон2; условие2)",
        drills::sumifs
    ),
    s!(
        "cond.countifs",
        4,
        "СЧЁТЕСЛИМН",
        "Счёт строк по нескольким условиям: пары «диапазон; условие».",
        "=СЧЁТЕСЛИМН(диапазон1; условие1; диапазон2; условие2)",
        drills::countifs
    ),
    s!(
        "cond.averageif",
        4,
        "СРЗНАЧЕСЛИ",
        "Среднее только по подходящим строкам.",
        "=СРЗНАЧЕСЛИ(где_искать; что_искать; что_усреднять)",
        drills::averageif
    ),
    s!(
        "choose.totals",
        4,
        "Какую функцию выбрать",
        "Сначала пойми, что спрашивают: сложить, посчитать строки, найти самое большое или среднее. Слова «у контрагента», «по поставщику» значат: не все строки, нужна функция с условием.",
        "сложить → СУММ, сколько штук → СЧЁТ, с условием → …ЕСЛИ",
        choose::totals
    ),
    s!(
        "check.reconcile",
        4,
        "Проверь себя: части = целое",
        "Сумма по всем статусам (или поставщикам) должна равняться общей сумме. Разница показывает деньги, которые потерялись из-за ошибки в написании.",
        "=СУММ(C2:C9)-СУММЕСЛИ(B2:B9;\"Получен\";C2:C9)-…",
        control::reconcile
    ),
    s!(
        "lookup.vlookup",
        5,
        "ВПР",
        "ВПР ищет значение в первом столбце таблицы и берёт ответ из столбца с номером N. ЛОЖЬ в конце — точное совпадение.",
        "=ВПР(что_ищем; таблица; номер_столбца; ЛОЖЬ)",
        drills::vlookup
    ),
    s!(
        "lookup.error_observe",
        5,
        "Когда код не найден",
        "Точное совпадение ВПР даёт #Н/Д, если такого кода нет. Сначала замечай ошибку, не маскируй её.",
        "ВПР(код; каталог; столбец; ЛОЖЬ) → #Н/Д",
        drills::lookup_error_observe
    ),
    s!(
        "lookup.iferror.wrap",
        5,
        "Обернуть результат в ЕСЛИОШИБКА",
        "ЕСЛИОШИБКА получает уже вычисленный результат: если это ошибка — подставляет понятную замену.",
        "=ЕСЛИОШИБКА(готовый_результат; \"нет\")",
        drills::lookup_iferror_wrap
    ),
    s!(
        "lookup.iferror.combine",
        5,
        "Соединить ВПР и ЕСЛИОШИБКА",
        "ВПР ищет цену, внешняя ЕСЛИОШИБКА обрабатывает только случай отсутствующего артикула.",
        "=ЕСЛИОШИБКА(ВПР(...); \"нет\")",
        drills::lookup_iferror_combine
    ),
    s!(
        "lookup.xlookup",
        5,
        "ПРОСМОТРX",
        "ПРОСМОТРX: что ищем, где ищем, откуда взять ответ. Проще и надёжнее ВПР.",
        "=ПРОСМОТРX(что_ищем; где_ищем; откуда_ответ)",
        drills::xlookup
    ),
    s!(
        "lookup.index",
        5,
        "ИНДЕКС + ПОИСКПОЗ",
        "ПОИСКПОЗ находит номер строки, ИНДЕКС берёт значение из этой строки.",
        "=ИНДЕКС(откуда_ответ; ПОИСКПОЗ(что_ищем; где_ищем; 0))",
        drills::index_match
    ),
    s!(
        "lookup.missing",
        5,
        "Если не найдено",
        "Если кода нет в справочнике, ВПР даёт #Н/Д. Оберни поиск в ЕСЛИОШИБКА.",
        "=ЕСЛИОШИБКА(ВПР(...);\"нет\")",
        drills::lookup_missing
    ),
    s!(
        "check.catalog",
        5,
        "Проверь себя: есть ли код в каталоге",
        "Прежде чем тянуть ВПР, проверь каждый код: СЧЁТЕСЛИ по каталогу даёт 0 там, где ВПР даст #Н/Д.",
        "=СЧЁТЕСЛИ($E$2:$E$8;A2)",
        control::catalog
    ),
    s!(
        "choose.lookup",
        5,
        "Какую функцию выбрать: с поиском",
        "Если спрашивают про одну строку по коду или номеру, это поиск (ВПР). Если про много строк сразу, это итог: СУММ, СЧЁТ или их форма с условием.",
        "одна строка по коду → ВПР, много строк → итог",
        choose::with_lookup
    ),
    s!("text.trim", 6, "СЖПРОБЕЛЫ", "Выгрузки приходят с лишними пробелами. СЖПРОБЕЛЫ убирает их.", "=СЖПРОБЕЛЫ(A2)", drills::trim),
    s!(
        "check.spaces",
        6,
        "Проверь себя: лишние пробелы",
        "Лишний пробел не видно, но из-за него название считается другим. Длина до и после СЖПРОБЕЛЫ покажет его.",
        "=ДЛСТР(A2)-ДЛСТР(СЖПРОБЕЛЫ(A2))",
        control::spaces
    ),
    s!("text.join", 6, "Склеить текст", "Знак & склеивает текст. Пробел между словами — \" \".", "=A2&\" \"&B2", drills::join),
    s!("text.leftright", 6, "ЛЕВСИМВ / ПРАВСИМВ", "ЛЕВСИМВ берёт N символов слева, ПРАВСИМВ — справа.", "=ПРАВСИМВ(A2;4)", drills::left_right),
    s!("text.proper", 6, "Регистр", "ПРОПНАЧ делает «Иванов Иван», ПРОПИСН — «ИВАНОВ».", "=ПРОПНАЧ(A2)", drills::proper),
    s!(
        "text.number",
        6,
        "Текст → число",
        "1С часто выгружает суммы как текст «12 500». Убери пробелы ПОДСТАВИТЬ и преврати в число ЗНАЧЕН.",
        "=ЗНАЧЕН(ПОДСТАВИТЬ(A2;\" \";\"\"))",
        drills::text_number
    ),
    s!("date.due", 7, "Срок оплаты", "Дата — это число дней. Дата + 30 = через 30 дней.", "=B2+C2", drills::due),
    s!("date.diff", 7, "Дни просрочки", "Разность двух дат — количество дней между ними.", "=C2-B2", drills::date_diff),
    s!(
        "supply.overdue",
        7,
        "Просроченные поставки",
        "Поставка просрочена, когда срок прошёл И товар не получен. Оба условия сразу - И внутри ЕСЛИ.",
        "=ЕСЛИ(И(D2<>\"Получен\";C2<$H$2);\"Просрочено\";\"\")",
        stock::overdue
    ),
    s!("date.month", 7, "МЕСЯЦ и ГОД", "МЕСЯЦ(дата) даёт номер месяца 1–12, ГОД — год.", "=МЕСЯЦ(B2)", drills::month),
    s!("date.eomonth", 7, "КОНМЕСЯЦА", "КОНМЕСЯЦА(дата; 0) — последний день того же месяца.", "=КОНМЕСЯЦА(B2;0)", drills::eomonth),
    s!(
        "data.sort",
        8,
        "Сортировка",
        "Встань в таблицу → Данные → Сортировка. Сортируй всю таблицу, иначе строки перемешаются.",
        "Данные → Сортировка",
        drills::sort
    ),
    s!(
        "data.filter",
        8,
        "Фильтр",
        "Включи фильтр (Ctrl+Shift+L), нажми стрелку в заголовке и оставь нужное значение.",
        "Данные → Фильтр",
        drills::filter
    ),
    s!(
        "data.freeze",
        8,
        "Закрепить шапку",
        "Чтобы шапка не уезжала при прокрутке: Вид → Закрепить области → Закрепить верхнюю строку.",
        "Вид → Закрепить области",
        drills::freeze
    ),
    s!(
        "data.format",
        8,
        "Формат чисел",
        "Деньги показывают с разделителем тысяч и двумя знаками: Ctrl+1 → Числовой.",
        "Ctrl+1 → Числовой",
        drills::numfmt
    ),
    s!(
        "data.dedupe",
        8,
        "Удалить дубликаты",
        "Данные → Удалить дубликаты. Проверь, по каким столбцам сравнивать.",
        "Данные → Удалить дубликаты",
        drills::dedupe
    ),
    s!(
        "data.pivot",
        8,
        "Сводная таблица",
        "Сводная собирает итоги по группам без формул: Вставка → Сводная таблица. Можно и через СУММЕСЛИ.",
        "Вставка → Сводная таблица",
        drills::pivot
    ),
    s!(
        "asm.pivot",
        8,
        "Сводная из нескольких выгрузок",
        "Сводная читает один список. Выгрузки складывают друг под другом под одну шапку, добавляют столбец «откуда строка» (месяц, склад) - и только потом строят сводную. На работе это же автоматически делает Power Query: Данные → Получить данные → Из папки.",
        "Копировать месяцы под одну шапку → столбец «Месяц» → Вставка → Сводная таблица",
        asm::pivot
    ),
    s!("keys.autosum", 9, "Автосумма Alt+=", "Встань под столбцом чисел и нажми Alt+= — Excel сам напишет СУММ.", "Alt+=", keys::autosum),
    s!("keys.f4", 9, "F4 ставит $", "Набирая формулу, встань курсором на адрес и нажми F4 — появятся $.", "F4", keys::f4),
    s!("keys.filldown", 9, "Ctrl+D", "Выдели ячейку с формулой и ячейки под ней, нажми Ctrl+D — формула скопируется вниз.", "Ctrl+D", keys::filldown),
    s!("keys.jump", 9, "Ctrl+стрелки", "Ctrl+↓ прыгает в конец столбца. Ctrl+Shift+↓ выделяет до конца.", "Ctrl+↓", keys::jump),
    s!("keys.filter", 9, "Ctrl+Shift+L", "Ctrl+Shift+L включает и выключает фильтр.", "Ctrl+Shift+L", keys::filter),
    s!("keys.bold", 9, "Ctrl+B", "Выдели и нажми Ctrl+B — жирный шрифт.", "Ctrl+B", keys::bold),
    s!("keys.date", 9, "Ctrl+; — сегодня", "Ctrl+; вставляет сегодняшнюю дату.", "Ctrl+;", keys::today),
    s!(
        "acc.accounts",
        10,
        "План счетов",
        "50 — касса, 51 — расчётный счёт, 60 — поставщики, 62 — покупатели, 70 — зарплата, 68 — налоги.",
        "Номер счёта числом",
        acc::accounts
    ),
    s!(
        "acc.entries",
        10,
        "Проводки Дт/Кт",
        "Дебет — куда пришло, кредит — откуда ушло. Оплата поставщику с расчётного счёта: Дт 60 Кт 51.",
        "Дт 60 — Кт 51",
        acc::entries
    ),
    s!(
        "acc.vat",
        10,
        "Выделить НДС",
        "НДС 22% внутри суммы = сумма × 22/122 (ставка с 2026 года). Округляй до копеек.",
        "=ОКРУГЛ(B2*22/122;2)",
        acc::vat_extract
    ),
    s!(
        "acc.payroll",
        10,
        "НДФЛ и к выплате",
        "НДФЛ 13% удерживают из начисленного. К выплате = начислено − НДФЛ.",
        "=ОКРУГЛ(B2*13%;0)",
        acc::payroll
    ),
    s!(
        "acc.insurance",
        10,
        "Страховые взносы",
        "Работодатель платит взносы сверху зарплаты (в тренажёре — 30%).",
        "=ОКРУГЛ(B2*30%;2)",
        acc::insurance
    ),
    s!("acc.osv", 10, "Сальдо в ОСВ", "Для активного счёта: сальдо на конец = начальное + оборот Дт − оборот Кт.", "=B2+C2-D2", acc::osv),
    s!(
        "sql.select",
        11,
        "SELECT",
        "SELECT перечисляет столбцы, FROM — таблицу. Порядок столбцов важен.",
        "SELECT name, city FROM clients",
        sql::select
    ),
    s!(
        "sql.where",
        11,
        "WHERE",
        "WHERE оставляет только подходящие строки. Текст — в одинарных кавычках.",
        "SELECT … FROM orders WHERE amount > 30000",
        sql::where_
    ),
    s!(
        "sql.order",
        11,
        "ORDER BY и LIMIT",
        "ORDER BY сортирует (DESC — по убыванию), LIMIT оставляет первые N строк.",
        "… ORDER BY amount DESC LIMIT 3",
        sql::order
    ),
    s!(
        "sql.like",
        11,
        "LIKE, BETWEEN, IN",
        "LIKE '%@mail.ru' — по шаблону, BETWEEN a AND b — диапазон с границами, IN (…) — из списка.",
        "WHERE email LIKE '%@mail.ru'",
        sql::like
    ),
    s!(
        "sql.null",
        11,
        "NULL",
        "Пустое значение NULL проверяют через IS NULL / IS NOT NULL. COALESCE подставляет замену.",
        "WHERE email IS NULL",
        sql::null
    ),
    s!(
        "sql.agg",
        11,
        "COUNT, SUM, AVG",
        "Агрегаты сворачивают строки в одно число: COUNT(*), SUM, AVG, MIN, MAX.",
        "SELECT SUM(amount) FROM orders",
        sql::agg
    ),
    s!(
        "sql.distinct",
        11,
        "DISTINCT",
        "DISTINCT убирает повторы. COUNT(DISTINCT x) считает разные значения.",
        "SELECT COUNT(DISTINCT client_id) FROM orders",
        sql::distinct
    ),
    s!(
        "sql.group",
        11,
        "GROUP BY",
        "GROUP BY делит строки на группы, агрегат считается по каждой группе.",
        "SELECT category, SUM(amount) FROM orders GROUP BY category",
        sql::group
    ),
    s!(
        "sql.having",
        11,
        "HAVING",
        "WHERE фильтрует строки до группировки, HAVING — группы после неё.",
        "… GROUP BY client_id HAVING COUNT(*) >= 3",
        sql::having
    ),
    s!(
        "sql.dates",
        11,
        "Даты в SQL",
        "Даты — текст 'ГГГГ-ММ-ДД', их можно сравнивать и BETWEEN. Месяц в SQLite: strftime('%m', дата).",
        "WHERE order_date BETWEEN '2026-03-01' AND '2026-03-31'",
        sql::dates
    ),
    s!(
        "sql.join",
        11,
        "JOIN",
        "JOIN склеивает строки двух таблиц по ключу: ON c.id = o.client_id. Псевдонимы o и c сокращают запись.",
        "FROM orders o JOIN clients c ON c.id = o.client_id",
        sql::join
    ),
    s!(
        "sql.left",
        11,
        "LEFT JOIN",
        "LEFT JOIN оставляет все строки левой таблицы; где пары нет — NULL. Так находят «тех, у кого нет».",
        "FROM clients c LEFT JOIN orders o ON o.client_id = c.id WHERE o.id IS NULL",
        sql::left
    ),
    s!(
        "sql.case",
        11,
        "CASE WHEN",
        "CASE WHEN условие THEN значение ELSE другое END — это ЕСЛИ из Excel внутри SQL.",
        "CASE WHEN amount >= 30000 THEN 'крупный' ELSE 'обычный' END",
        sql::case
    ),
    s!(
        "sql.sub",
        11,
        "Подзапросы",
        "Запрос внутри запроса: сравнить со средним или взять список id через IN (SELECT …).",
        "WHERE amount > (SELECT AVG(amount) FROM orders)",
        sql::sub
    ),
    s!(
        "sql.window",
        11,
        "Оконные функции",
        "Окно считает по группе, не сворачивая строки: ROW_NUMBER/RANK() OVER (PARTITION BY … ORDER BY …), SUM() OVER.",
        "ROW_NUMBER() OVER (PARTITION BY dept ORDER BY salary DESC)",
        sql::window
    ),
    s!(
        "sql.interview",
        11,
        "Задачи с собеседования",
        "Разбей вопрос: какие таблицы, как связаны, какие фильтры, что агрегировать.",
        "JOIN + WHERE + GROUP BY + HAVING",
        sql::interview
    ),
    s!(
        "asm.columns",
        12,
        "Перенос по столбцам",
        "Каждый столбец источника - под свой заголовок отчёта, даже если он называется иначе и стоит в другом месте.",
        "Ctrl+C столбца → Ctrl+V под нужным заголовком",
        asm::columns
    ),
    s!(
        "asm.numbers",
        12,
        "Числа из текста",
        "«5 шт», «1200 тг» и число-текст Excel не считает. В отчёте должны быть настоящие числа.",
        "=ЗНАЧЕН(ПОДСТАВИТЬ(C4;\" шт\";\"\"))",
        asm::numbers
    ),
    s!(
        "asm.dates",
        12,
        "Даты в один формат",
        "Дата текстом или словами - не дата: её нельзя сортировать и вычитать. В отчёте - настоящие даты.",
        "=ЗНАЧЕН(B4) или =ДАТА(2026;3;12)",
        asm::dates
    ),
    s!(
        "asm.names",
        12,
        "Названия по справочнику",
        "Один поставщик - одно написание, как в справочнике. Иначе СУММЕСЛИ и сводная посчитают его как разных.",
        "Исправь вручную; лишние пробелы - =СЖПРОБЕЛЫ(B4)",
        asm::names
    ),
    s!(
        "asm.lookup",
        12,
        "Цена с другого листа",
        "Цену и название берут из прайса по артикулу. Ссылка на другой лист: 'Прайс поставщика'!A2.",
        "=ВПР(B2;'Прайс поставщика'!$A$2:$C$11;3;ЛОЖЬ)",
        asm::lookup
    ),
    s!(
        "asm.merge",
        12,
        "Два источника в один список",
        "Списки двух людей складывают друг под другом, приводя столбцы к одной шапке. Повторы - один раз.",
        "Скопировать первый список, под ним - второй",
        asm::merge
    ),
    s!(
        "asm.format",
        12,
        "Оформление отчёта",
        "Отчёт для руководителя: жирная закреплённая шапка, суммы с разделителем, порядок по дате, строка «Итого».",
        "Ctrl+B; Вид → Закрепить; Ctrl+1; Данные → Сортировка",
        asm::format
    ),
    s!(
        "asm.report",
        12,
        "Отчёт целиком",
        "Всё вместе: перенести из нескольких листов, почистить даты, числа и названия, подтянуть цены, досчитать и оформить.",
        "Перенос → чистка → поиск → расчёт → оформление",
        asm::report
    ),
    s!(
        "orion.pivotread",
        13,
        "Сводная: найди ответ",
        "Руководитель задаёт вопрос словами - ты решаешь, что в строки, что в столбцы, сумма или количество, и находишь ответ на пересечении.",
        "Вставка → Сводная таблица → Строки / Столбцы / Значения",
        orion::pivotread
    ),
    s!(
        "orion.open",
        13,
        "Пн: открытые заказы",
        "Первое, что делают с реестром: сколько заказов в каждом статусе и на какую сумму. Сколько - СЧЁТЕСЛИ, на сколько - СУММЕСЛИ.",
        "=СЧЁТЕСЛИ(статусы;\"В пути\")",
        orion::open
    ),
    s!(
        "orion.overdue",
        13,
        "Вт: утренний контроль",
        "Каждое утро: что уже опоздало, что вот-вот придёт. Полученное не трогаем.",
        "=ЕСЛИ(статус=\"Получен\";\"\";ЕСЛИ(дата<сегодня;\"Просрочено\";ЕСЛИ(дата-сегодня<=3;\"Скоро\";\"\")))",
        orion::overdue
    ),
    s!(
        "orion.contact",
        13,
        "Вт: кому звонить",
        "Контакты поставщика берут из справочника по названию - ВПР с точным совпадением.",
        "=ВПР(что_ищем;таблица;номер_столбца;ЛОЖЬ)",
        orion::contact
    ),
    s!(
        "orion.fx",
        13,
        "Ср: счета в валютах",
        "Счёт в долларах или юанях переводят в тенге по курсу из таблицы курсов, а не числом в формуле.",
        "=сумма*ВПР(валюта;курсы;2;ЛОЖЬ)",
        orion::fx
    ),
    s!(
        "orion.payweek",
        13,
        "Ср: оплаты недели",
        "Сколько платить за период - СУММЕСЛИМН с двумя условиями на дату, границы включительно.",
        "=СУММЕСЛИМН(суммы;сроки;\">=\"&начало;сроки;\"<=\"&конец)",
        orion::payweek
    ),
    s!(
        "orion.prepay",
        13,
        "Ср: предоплата",
        "Китай часто работает по предоплате: часть при заказе, остальное перед отгрузкой.",
        "=ОКРУГЛ(сумма*процент;2)",
        orion::prepay
    ),
    s!(
        "orion.reconcile",
        13,
        "Чт: сверка документов",
        "Перед оплатой сверяют три бумаги: что заказали, что выставили, что пришло.",
        "=ЕСЛИОШИБКА(ВПР(артикул;'Заказ'!таблица;3;ЛОЖЬ);0)",
        orion::reconcile
    ),
    s!(
        "orion.defects",
        13,
        "Чт: входной контроль",
        "Доля брака - деление. Выше порога - претензия поставщику.",
        "=ЕСЛИ(брак/получено>порог;\"Претензия\";\"Принять\")",
        orion::defects
    ),
    s!(
        "orion.mrp",
        13,
        "Пт: потребность под план",
        "Потребность = норма на изделие × план, по всем моделям. Докупаем то, чего не хватает с учётом склада и пути.",
        "=МАКС(0;потребность-склад-в_пути)",
        orion::mrp
    ),
    s!(
        "orion.moq",
        13,
        "Пт: упаковка и мин. партия",
        "Заказывают целыми упаковками и не меньше минимальной партии поставщика.",
        "=МАКС(мин_партия;ОКРУГЛВВЕРХ(нужно/упаковка;0)*упаковка)",
        orion::moq
    ),
    s!(
        "orion.reorder",
        13,
        "Пт: когда заказывать",
        "Заказывать надо, пока запаса хватает на срок поставки плюс страховой запас.",
        "=сегодня+ЦЕЛОЕ(остаток/расход)-срок-страховой",
        orion::reorder
    ),
    s!(
        "orion.landed",
        13,
        "Сб: полная цена до склада",
        "Цена в инвойсе - не наша цена: плюс доставка, пошлина и НДС на импорт.",
        "=(стоимость+доставка)*(1+пошлина)*(1+НДС)",
        orion::landed
    ),
    s!(
        "orion.compare",
        13,
        "Сб: выбор предложения",
        "Сравнивают полную цену штуки на своём складе, а не цену в прайсе.",
        "=(цена*курс*кол_во+доставка)*(1+пошлина)/кол_во",
        orion::compare
    ),
    s!(
        "orion.join",
        13,
        "Вс: заказы + каталог",
        "Для сводной все поля должны быть в одном списке: недостающее приносят ВПР по артикулу.",
        "=ВПР(B2;'Каталог'!$A$2:$D$13;3;ЛОЖЬ)",
        orion::join
    ),
    s!(
        "orion.abc",
        13,
        "Вс: ABC-анализ",
        "Правило Парето: немногие позиции держат большую часть денег. A - до 80% накопленной доли.",
        "=СУММ($C$2:C2)/СУММ($C$2:$C$13)",
        orion::abc
    ),
    s!(
        "orion.exam",
        13,
        "Тестовое на время",
        "Незнакомая таблица и три вопроса за 15 минут, как на собеседовании.",
        "ВПР → СУММЕСЛИ → СЧЁТЕСЛИМН",
        orion::exam
    ),
    s!(
        "orion.quarter",
        13,
        "Большой кейс: квартал",
        "Реальная папка снабженца: план, спецификация, два склада, заказы в пути, справочник, пять прайсов в разных валютах, курсы и продажи по странам. Собрать заказ на месяц, бюджет и сводную продаж.",
        "ВПР / СУММЕСЛИ по артикулу с разных листов → расчёт → сводная",
        orion::quarter
    ),
];

pub fn skill(id: &str) -> Option<&'static Skill> {
    SKILLS.iter().find(|s| s.id == id)
}

pub fn build(spec: &Spec) -> Result<Task> {
    if spec.skill.starts_with(live::PREFIX) {
        return live::build(spec);
    }
    if let Some(case) = CASES.iter().find(|c| c.id == spec.skill) {
        let mut g = Gen::new(spec);
        return (case.build)(&mut g);
    }
    let sk = skill(&spec.skill).ok_or_else(|| crate::LearnError::Invalid(tr(&format!("Нет навыка {}", spec.skill))))?;
    let mut g = Gen::new(spec);
    (sk.build)(&mut g)
}

// ---------- shared data pools ----------

pub const PARTIES: &[&str] = &["ООО Альфа", "ООО Вектор", "АО Гранит", "ООО Ромашка", "ИП Смирнов", "ООО Меридиан", "ООО Орион", "АО Стройдом"];
pub const PRODUCTS: &[&str] = &["Бумага А4", "Картридж", "Папка", "Ручки", "Степлер", "Скрепки", "Маркер", "Блокнот", "Конверты", "Тонер"];
pub const PEOPLE: &[&str] = &["Иванова А.", "Петров С.", "Сидоров К.", "Кузнецова М.", "Смирнова О.", "Попов Д.", "Волкова Е.", "Морозов И."];
pub const DEPTS: &[&str] = &["Бухгалтерия", "Склад", "Продажи", "Логистика"];
pub const CITIES: &[&str] = &["Москва", "Казань", "Самара", "Пермь", "Тула"];
pub const MONTHS: &[&str] = &["Январь", "Февраль", "Март", "Апрель", "Май", "Июнь"];

/// Generation context for one exercise.
pub struct Gen {
    pub spec: Spec,
    pub l: Rng,
    pub d: Rng,
    pub level: u8,
    pub setup: Setup,
    pub checks: Vec<Check>,
}

/// Where a generated table sits. Rows are 1-based; columns 0-based.
#[derive(Clone, Debug)]
pub struct Table {
    pub header: u32,
    pub first: u32,
    pub last: u32,
    pub left: u32,
    pub right: u32,
}

impl Table {
    pub fn col(&self, i: u32) -> u32 {
        self.left + i
    }
    pub fn range(&self, i: u32) -> String {
        crate::sheet::range(self.col(i), self.first, self.last)
    }
    pub fn abs(&self, i: u32) -> String {
        crate::sheet::abs_range(self.col(i), self.first, self.last)
    }
    pub fn cell(&self, i: u32, row: u32) -> String {
        a1(row, self.col(i))
    }
    pub fn rows(&self) -> std::ops::RangeInclusive<u32> {
        self.first..=self.last
    }
    pub fn n(&self) -> u32 {
        self.last + 1 - self.first
    }
}

impl Gen {
    pub fn new(spec: &Spec) -> Self {
        Self {
            spec: spec.clone(),
            l: Rng::new(spec.layout),
            d: Rng::new(spec.data),
            level: spec.level.clamp(1, 4),
            setup: Setup::default(),
            checks: Vec::new(),
        }
    }

    pub fn id(&self) -> String {
        self.spec.skill.clone()
    }

    /// Row count grows with the level; layout seed only.
    pub fn rows(&mut self) -> u32 {
        match self.level {
            1 => 5,
            2 => 6 + self.l.below(2) as u32,
            3 => 8 + self.l.below(3) as u32,
            _ => 12 + self.l.below(5) as u32,
        }
    }

    /// Top-left of the table. Early levels always start at A1 so nothing distracts.
    pub fn origin(&mut self, title: &str) -> (u32, u32) {
        match self.level {
            1 | 2 => (1, 0),
            3 => {
                if self.l.chance(50) {
                    self.setup.text("A1", title);
                    self.setup.bold.push("A1".into());
                    (3, 0)
                } else {
                    (1, 0)
                }
            }
            _ => {
                let col = self.l.below(2) as u32;
                self.setup.text(a1(1, col), title);
                self.setup.bold.push(a1(1, col));
                (3, col)
            }
        }
    }

    /// Writes a header row; returns a table with `n` data rows below it.
    pub fn table(&mut self, title: &str, headers: &[&str], n: u32) -> Table {
        let (header, left) = self.origin(title);
        self.setup.header(header, left, headers);
        for (i, h) in headers.iter().enumerate() {
            let w = (h.chars().count() as f32 * 8.0 + 24.0).clamp(80.0, 170.0);
            self.setup.width(left + i as u32, w);
        }
        Table { header, first: header + 1, last: header + n, left, right: left + headers.len() as u32 - 1 }
    }

    pub fn num(&mut self, t: &Table, col: u32, row: u32, v: f64) {
        self.setup.input(t.cell(col, row), Cell::Num(v));
    }
    pub fn txt(&mut self, t: &Table, col: u32, row: u32, v: &str) {
        self.setup.input(t.cell(col, row), Cell::Text(v.into()));
    }
    pub fn day(&mut self, t: &Table, col: u32, row: u32, v: f64) {
        self.setup.input(t.cell(col, row), Cell::Date(v));
    }

    /// A random date in the first half of 2026.
    pub fn some_date(&mut self) -> f64 {
        date(2026, 1, 10) + self.d.int(0, 150) as f64
    }

    /// Fills a category column from `pool` (data seed) and guarantees `must` appears.
    pub fn categories(&mut self, t: &Table, col: u32, pool: &[&'static str], must: &str) {
        let n = t.n();
        let forced = t.first + self.d.below(n as usize) as u32;
        for r in t.rows() {
            let v = if r == forced { must } else { self.d.pick(pool) };
            self.txt(t, col, r, v);
        }
    }

    /// A labelled parameter cell to the right of the table (`label` | value). Returns its address.
    pub fn param(&mut self, t: &Table, row_offset: u32, label: &str, value: Cell) -> String {
        let lc = t.right + 2;
        self.setup.text(a1(t.header + row_offset, lc), label);
        self.setup.width(lc, 150.0);
        let cell = a1(t.header + row_offset, lc + 1);
        self.setup.width(lc + 1, 120.0);
        self.setup.put(cell.clone(), value, false);
        cell
    }

    /// The answer cell: labelled, highlighted, right of the table.
    pub fn answer(&mut self, t: &Table, row_offset: u32, label: &str) -> String {
        let lc = t.right + 2;
        self.setup.text(a1(t.header + row_offset, lc), label);
        self.setup.bold.push(a1(t.header + row_offset, lc));
        self.setup.width(lc, 150.0);
        self.setup.width(lc + 1, 120.0);
        let cell = a1(t.header + row_offset, lc + 1);
        self.setup.answer(cell.clone());
        cell
    }

    pub fn check(&mut self, label: &str, kind: Kind) {
        let skill = self.id();
        self.checks.push(Check { label: label.into(), skill, kind });
    }

    pub fn formula_check(&mut self, cell: &str, reference: &str) {
        self.check(cell, Kind::Formula { cell: cell.into(), reference: reference.into(), live: true });
    }

    pub fn task(&mut self, title: &str, prompt: String, example: Option<String>, focus: String) -> Task {
        let hints = self.hints(example.as_deref(), &focus);
        Task {
            spec: self.spec.clone(),
            mode: Mode::Drill,
            title: title.into(),
            prompt,
            example: if self.level == 1 { example } else { None },
            hints,
            focus,
            setup: std::mem::take(&mut self.setup),
            checks: std::mem::take(&mut self.checks),
            goal_seconds: 20.0 + 15.0 * f32::from(self.level),
            reorders: false,
            brief: None,
            sql: None,
        }
    }

    /// Each hint says more than the one before and never repeats what is already on screen.
    /// At level 1 the formula is shown, so the hints explain it instead of echoing it.
    fn hints(&self, example: Option<&str>, focus: &str) -> Vec<String> {
        let Some(sk) = skill(&self.spec.skill) else {
            return example.map(|e| vec![tr(&format!("Ответ: {e}"))]).unwrap_or_default();
        };
        let syntax = ts(sk.syntax);
        let mut hints = vec![ts(sk.idea).to_string()];
        if self.level == 1 {
            match example {
                Some(e) if e.starts_with('=') => {
                    hints.extend(breakdown(syntax, e));
                    hints.push(if e.contains('(') {
                        tr(&format!("Набери формулу из образца в {focus}: начни со знака =, части разделяй точкой с запятой, в конце нажми Enter."))
                    } else {
                        tr(&format!("Набери формулу из образца в {focus}: начни со знака =, в конце нажми Enter."))
                    });
                }
                Some(e) if e != syntax => hints.push(tr(&format!("Шаблон: {syntax}"))),
                _ => {}
            }
        } else {
            hints.push(tr(&format!("Шаблон: {syntax}")));
            if let Some(e) = example {
                hints.extend(breakdown(syntax, e));
                hints.push(tr(&format!("Ответ: {e}")));
            }
        }
        hints.retain(|h| !h.is_empty());
        hints
    }

    /// One formula in one cell. `ask2` names the tool; `ask3` is a business question.
    pub fn single(&mut self, title: &str, cell: &str, reference: &str, ask2: &str, ask3: &str) -> Task {
        self.formula_check(cell, reference);
        let shown = ru(reference);
        let prompt = match self.level {
            1 => tr(&format!("Введи в {cell} формулу:")),
            2 => tr(&format!("{ask2} Ответ — в {cell}.")),
            _ => tr(&format!("{ask3} Ответ — в {cell}.")),
        };
        self.task(title, prompt, Some(shown), cell.to_string())
    }

    /// A formula column: one answer per data row, written once and filled down.
    pub fn column(&mut self, title: &str, t: &Table, col: u32, header: &str, reference: impl Fn(u32) -> String, ask2: &str, ask3: &str) -> Task {
        self.setup.text(t.cell(col, t.header), header);
        self.setup.bold.push(t.cell(col, t.header));
        self.setup.width(t.col(col), 130.0);
        for r in t.rows() {
            let cell = t.cell(col, r);
            self.setup.answer(cell.clone());
            self.formula_check(&cell, &reference(r));
        }
        let first = t.cell(col, t.first);
        let last = t.cell(col, t.last);
        let shown = ru(&reference(t.first));
        let prompt = match self.level {
            1 => tr(&format!("Введи в {first} формулу и протяни её вниз до {last}:")),
            2 => tr(&format!("{ask2} Заполни столбец «{header}» ({first}:{last}).")),
            _ => tr(&format!("{ask3} Заполни {first}:{last}.")),
        };
        let mut task = self.task(title, prompt, Some(shown), first);
        task.goal_seconds += 10.0;
        task
    }
}

/// Pairs each placeholder of the skill template with the real argument of the example:
/// `=ВПР(что_ищем; таблица; …)` + `=ВПР(E3;A2:C6;…)` → «E3 - что ищем, A2:C6 - таблица».
fn breakdown(syntax: &str, example: &str) -> Option<String> {
    let (tf, targs) = call_args(syntax)?;
    let (ef, eargs) = call_args(example)?;
    if tf.to_lowercase() != ef.to_lowercase() || targs.len() != eargs.len() {
        return None;
    }
    let placeholder = |a: &str| a.chars().next().is_some_and(|c| c.is_lowercase()) && !a.contains(['"', '(']);
    let parts: Vec<String> =
        targs.iter().zip(&eargs).filter(|(t, e)| placeholder(t) && t != e).map(|(t, e)| format!("{e} - {}", t.replace('_', " "))).collect();
    (!parts.is_empty()).then(|| tr(&format!("Что значит каждая часть: {}", parts.join(", "))))
}

/// `=F(a; b(c; d); e)` → ("F", ["a", "b(c; d)", "e"]), split on top-level separators only.
fn call_args(formula: &str) -> Option<(&str, Vec<String>)> {
    let body = formula.trim().strip_prefix('=')?;
    let open = body.find('(')?;
    let inner = body.get(open + 1..)?.strip_suffix(')')?;
    let sep = if gridcraft_tr::english() { ',' } else { ';' };
    let (mut args, mut cur, mut depth, mut quoted) = (Vec::new(), String::new(), 0i32, false);
    for c in inner.chars() {
        match c {
            '"' => quoted = !quoted,
            '(' if !quoted => depth += 1,
            ')' if !quoted => depth -= 1,
            _ => {}
        }
        if c == sep && depth == 0 && !quoted {
            args.push(cur.trim().to_string());
            cur.clear();
        } else {
            cur.push(c);
        }
    }
    args.push(cur.trim().to_string());
    Some((body.get(..open)?.trim(), args))
}

pub fn letter(col: u32) -> String {
    col_name(col)
}
