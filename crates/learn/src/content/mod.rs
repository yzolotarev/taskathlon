//! Seeded exercise generators. `layout` fixes where things are and what is asked; `data`
//! fixes the numbers, so grading can re-run the learner's formulas on other numbers.

mod acc;
pub mod cases;
mod drills;
mod keys;
mod sql;

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
        "lookup.vlookup",
        5,
        "ВПР",
        "ВПР ищет значение в первом столбце таблицы и берёт ответ из столбца с номером N. ЛОЖЬ в конце — точное совпадение.",
        "=ВПР(что_ищем; таблица; номер_столбца; ЛОЖЬ)",
        drills::vlookup
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
    s!("text.trim", 6, "СЖПРОБЕЛЫ", "Выгрузки приходят с лишними пробелами. СЖПРОБЕЛЫ убирает их.", "=СЖПРОБЕЛЫ(A2)", drills::trim),
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
    s!("acc.vat", 10, "Выделить НДС", "НДС 22% внутри суммы = сумма × 22/122 (ставка с 2026 года). Округляй до копеек.", "=ОКРУГЛ(B2*22/122;2)", acc::vat_extract),
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
];

pub fn skill(id: &str) -> Option<&'static Skill> {
    SKILLS.iter().find(|s| s.id == id)
}

pub fn build(spec: &Spec) -> Result<Task> {
    if let Some(case) = CASES.iter().find(|c| c.id == spec.skill) {
        let mut g = Gen::new(spec);
        return (case.build)(&mut g);
    }
    let sk = skill(&spec.skill).ok_or_else(|| crate::LearnError::Invalid(format!("Нет навыка {}", spec.skill)))?;
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
    pub fn categories(&mut self, t: &Table, col: u32, pool: &[&str], must: &str) {
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
        let sk = skill(&self.spec.skill);
        let mut hints = vec![sk.map(|s| s.idea.to_string()).unwrap_or_default(), sk.map(|s| format!("Шаблон: {}", s.syntax)).unwrap_or_default()];
        if let Some(e) = &example {
            hints.push(format!("Ответ: {e}"));
        }
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

    /// One formula in one cell. `ask2` names the tool; `ask3` is a business question.
    pub fn single(&mut self, title: &str, cell: &str, reference: &str, ask2: &str, ask3: &str) -> Task {
        self.formula_check(cell, reference);
        let shown = ru(reference);
        let prompt = match self.level {
            1 => format!("Введи в {cell} формулу:"),
            2 => format!("{ask2} Ответ — в {cell}."),
            _ => format!("{ask3} Ответ — в {cell}."),
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
            1 => format!("Введи в {first} формулу и протяни её вниз до {last}:"),
            2 => format!("{ask2} Заполни столбец «{header}» ({first}:{last})."),
            _ => format!("{ask3} Заполни {first}:{last}."),
        };
        let mut task = self.task(title, prompt, Some(shown), first);
        task.goal_seconds += 10.0;
        task
    }
}

pub fn letter(col: u32) -> String {
    col_name(col)
}
