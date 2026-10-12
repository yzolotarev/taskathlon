//! Localized aliases accepted at user-entry boundaries; file syntax stays English.

use crate::{Expr, ParseError, parser};

/// Formula-entry syntax selected by the interface language. Other interface languages
/// currently use the canonical English function table until a translation table is supplied.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FormulaLocale {
    #[default]
    En,
    Es,
    Ru,
}

const SPANISH: &[(&str, &str)] = &[
    ("SUM", "SUMA"),
    ("AVERAGE", "PROMEDIO"),
    ("IF", "SI"),
    ("COUNT", "CONTAR"),
    ("SUMIF", "SUMAR.SI"),
    ("COUNTIF", "CONTAR.SI"),
    ("VLOOKUP", "BUSCARV"),
    ("TODAY", "HOY"),
    ("TRUE", "VERDADERO"),
    ("FALSE", "FALSO"),
];

const RUSSIAN: &[(&str, &str)] = &[
    ("SUM", "СУММ"),
    ("AVERAGE", "СРЗНАЧ"),
    ("MIN", "МИН"),
    ("MAX", "МАКС"),
    ("COUNT", "СЧЁТ"),
    ("COUNTA", "СЧЁТЗ"),
    ("COUNTBLANK", "СЧИТАТЬПУСТОТЫ"),
    ("IF", "ЕСЛИ"),
    ("IFS", "ЕСЛИМН"),
    ("AND", "И"),
    ("OR", "ИЛИ"),
    ("NOT", "НЕ"),
    ("XOR", "ИСКЛИЛИ"),
    ("IFERROR", "ЕСЛИОШИБКА"),
    ("IFNA", "ЕСНД"),
    ("SUMIF", "СУММЕСЛИ"),
    ("SUMIFS", "СУММЕСЛИМН"),
    ("COUNTIF", "СЧЁТЕСЛИ"),
    ("COUNTIFS", "СЧЁТЕСЛИМН"),
    ("AVERAGEIF", "СРЗНАЧЕСЛИ"),
    ("AVERAGEIFS", "СРЗНАЧЕСЛИМН"),
    ("MAXIFS", "МАКСЕСЛИ"),
    ("MINIFS", "МИНЕСЛИ"),
    ("SUMPRODUCT", "СУММПРОИЗВ"),
    ("PRODUCT", "ПРОИЗВЕД"),
    ("VLOOKUP", "ВПР"),
    ("HLOOKUP", "ГПР"),
    ("XLOOKUP", "ПРОСМОТРX"),
    ("XMATCH", "ПОИСКПОЗX"),
    ("LOOKUP", "ПРОСМОТР"),
    ("INDEX", "ИНДЕКС"),
    ("MATCH", "ПОИСКПОЗ"),
    ("CHOOSE", "ВЫБОР"),
    ("OFFSET", "СМЕЩ"),
    ("INDIRECT", "ДВССЫЛ"),
    ("ROW", "СТРОКА"),
    ("COLUMN", "СТОЛБЕЦ"),
    ("ROWS", "ЧСТРОК"),
    ("COLUMNS", "ЧИСЛСТОЛБ"),
    ("ROUND", "ОКРУГЛ"),
    ("ROUNDUP", "ОКРУГЛВВЕРХ"),
    ("ROUNDDOWN", "ОКРУГЛВНИЗ"),
    ("MROUND", "ОКРУГЛТ"),
    ("CEILING", "ОКРВВЕРХ"),
    ("FLOOR", "ОКРВНИЗ"),
    ("TRUNC", "ОТБР"),
    ("INT", "ЦЕЛОЕ"),
    ("MOD", "ОСТАТ"),
    ("POWER", "СТЕПЕНЬ"),
    ("SQRT", "КОРЕНЬ"),
    ("SIGN", "ЗНАК"),
    ("TRIM", "СЖПРОБЕЛЫ"),
    ("CLEAN", "ПЕЧСИМВ"),
    ("UPPER", "ПРОПИСН"),
    ("LOWER", "СТРОЧН"),
    ("PROPER", "ПРОПНАЧ"),
    ("LEN", "ДЛСТР"),
    ("LEFT", "ЛЕВСИМВ"),
    ("RIGHT", "ПРАВСИМВ"),
    ("MID", "ПСТР"),
    ("FIND", "НАЙТИ"),
    ("SEARCH", "ПОИСК"),
    ("SUBSTITUTE", "ПОДСТАВИТЬ"),
    ("REPLACE", "ЗАМЕНИТЬ"),
    ("REPT", "ПОВТОР"),
    ("EXACT", "СОВПАД"),
    ("CONCATENATE", "СЦЕПИТЬ"),
    ("CONCAT", "СЦЕП"),
    ("TEXTJOIN", "ОБЪЕДИНИТЬ"),
    ("TEXTBEFORE", "ТЕКСТДО"),
    ("TEXTAFTER", "ТЕКСТПОСЛЕ"),
    ("TEXTSPLIT", "ТЕКСТРАЗД"),
    ("TEXT", "ТЕКСТ"),
    ("VALUE", "ЗНАЧЕН"),
    ("FIXED", "ФИКСИРОВАННЫЙ"),
    ("CHAR", "СИМВОЛ"),
    ("CODE", "КОДСИМВ"),
    ("TODAY", "СЕГОДНЯ"),
    ("NOW", "ТДАТА"),
    ("DATE", "ДАТА"),
    ("DATEVALUE", "ДАТАЗНАЧ"),
    ("TIME", "ВРЕМЯ"),
    ("YEAR", "ГОД"),
    ("MONTH", "МЕСЯЦ"),
    ("DAY", "ДЕНЬ"),
    ("HOUR", "ЧАС"),
    ("MINUTE", "МИНУТЫ"),
    ("WEEKDAY", "ДЕНЬНЕД"),
    ("WEEKNUM", "НОМНЕДЕЛИ"),
    ("EOMONTH", "КОНМЕСЯЦА"),
    ("EDATE", "ДАТАМЕС"),
    ("DATEDIF", "РАЗНДАТ"),
    ("DAYS", "ДНИ"),
    ("NETWORKDAYS", "ЧИСТРАБДНИ"),
    ("WORKDAY", "РАБДЕНЬ"),
    ("ISBLANK", "ЕПУСТО"),
    ("ISNUMBER", "ЕЧИСЛО"),
    ("ISTEXT", "ЕТЕКСТ"),
    ("ISERROR", "ЕОШИБКА"),
    ("ISNA", "ЕНД"),
    ("ISEVEN", "ЕЧЁТН"),
    ("ISODD", "ЕНЕЧЁТ"),
    ("NA", "НД"),
    ("UNIQUE", "УНИК"),
    ("FILTER", "ФИЛЬТР"),
    ("SORT", "СОРТ"),
    ("SORTBY", "СОРТПО"),
    ("SEQUENCE", "ПОСЛЕД"),
    ("TRANSPOSE", "ТРАНСП"),
    ("SUBTOTAL", "ПРОМЕЖУТОЧНЫЕ.ИТОГИ"),
    ("AGGREGATE", "АГРЕГАТ"),
    ("RANK.EQ", "РАНГ.РВ"),
    ("RANK", "РАНГ"),
    ("LARGE", "НАИБОЛЬШИЙ"),
    ("SMALL", "НАИМЕНЬШИЙ"),
    ("MEDIAN", "МЕДИАНА"),
    ("STDEV", "СТАНДОТКЛОН"),
    ("VAR", "ДИСП"),
    ("PERCENTILE", "ПЕРСЕНТИЛЬ"),
    ("QUARTILE", "КВАРТИЛЬ"),
    ("PMT", "ПЛТ"),
    ("NPV", "ЧПС"),
    ("IRR", "ВСД"),
    ("FV", "БС"),
    ("PV", "ПС"),
    ("RATE", "СТАВКА"),
    ("HYPERLINK", "ГИПЕРССЫЛКА"),
    ("TRUE", "ИСТИНА"),
    ("FALSE", "ЛОЖЬ"),
];

/// Russian names compare without case and treat Ё as Е and Cyrillic Х as Latin X
/// (Excel spells ПРОСМОТРX with a Latin X; learners type either).
fn russian_key(name: &str) -> String {
    name.chars()
        .flat_map(char::to_uppercase)
        .map(|c| match c {
            'Ё' => 'Е',
            'Х' => 'X',
            c => c,
        })
        .collect()
}

impl FormulaLocale {
    pub fn list_separator(self) -> char {
        if self == Self::En { ',' } else { ';' }
    }
    pub fn decimal_separator(self) -> char {
        if self == Self::En { '.' } else { ',' }
    }
    pub fn array_column_separator(self) -> char {
        if self == Self::En { ',' } else { '\\' }
    }
    pub fn function_name(self, canonical: &str) -> &str {
        let table = match self {
            Self::En => return canonical,
            Self::Es => SPANISH,
            Self::Ru => RUSSIAN,
        };
        table.iter().find(|(en, _)| en.eq_ignore_ascii_case(canonical)).map(|(_, local)| *local).unwrap_or(canonical)
    }
    pub fn canonical_name(self, localized: &str) -> &str {
        match self {
            Self::En => localized,
            Self::Es => SPANISH.iter().find(|(_, es)| es.eq_ignore_ascii_case(localized)).map(|(en, _)| *en).unwrap_or(localized),
            Self::Ru => {
                let key = russian_key(localized);
                RUSSIAN.iter().find(|(_, ru)| russian_key(ru) == key).map(|(en, _)| *en).unwrap_or(localized)
            }
        }
    }
}

/// Parses UI input using the selected function table and separators. Existing names and
/// LET/LAMBDA bindings take precedence over aliases. File and API syntax uses [`crate::parse`].
pub fn parse_input(src: &str, locale: FormulaLocale, known_name: impl Fn(&str) -> bool) -> Result<Expr, ParseError> {
    let mut expr = parser::parse_syntax(src, locale)?;
    if locale != FormulaLocale::En {
        normalize(&mut expr, locale, &known_name, &mut Vec::new(), 0, false)?;
    }
    Ok(expr)
}

/// Prints canonical expressions for editing. Name collisions retain the canonical spelling
/// so displaying then committing a formula never changes a defined-name reference.
pub fn print_input(expr: &Expr, locale: FormulaLocale, known_name: impl Fn(&str) -> bool) -> Result<String, ParseError> {
    // Check before cloning/printing: a flat operator chain can produce a deep AST even
    // though the parser itself did not recurse deeply.
    let mut pending = vec![(expr, 0usize)];
    while let Some((node, depth)) = pending.pop() {
        if depth > parser::MAX_DEPTH {
            return Err(ParseError { msg: "formula is nested too deeply".into(), pos: 0 });
        }
        match node {
            Expr::Unary(_, child) | Expr::Paren(child) => pending.push((child, depth + 1)),
            Expr::Binary(_, left, right) => {
                pending.push((left, depth + 1));
                pending.push((right, depth + 1));
            }
            Expr::Call(_, args) => pending.extend(args.iter().map(|e| (e, depth + 1))),
            Expr::Invoke(callee, args) => {
                pending.push((callee, depth + 1));
                pending.extend(args.iter().map(|e| (e, depth + 1)));
            }
            Expr::Array(rows) => pending.extend(rows.iter().flatten().map(|e| (e, depth + 1))),
            _ => {}
        }
    }
    if locale == FormulaLocale::En {
        return Ok(crate::print(expr));
    }
    let mut expr = expr.clone();
    normalize(&mut expr, locale, &known_name, &mut Vec::new(), 0, true)?;
    let canonical = crate::print(&expr);
    let tokens = crate::lexer::tokenize(&canonical).map_err(|e| ParseError { msg: e.msg, pos: e.pos })?;
    let mut out = String::new();
    let mut array_depth = 0usize;
    for token in tokens {
        use crate::lexer::Tok;
        let text = canonical.get(token.start..token.end).unwrap_or("");
        match token.tok {
            Tok::LBrace => {
                array_depth += 1;
                out.push_str(text);
            }
            Tok::RBrace => {
                array_depth = array_depth.saturating_sub(1);
                out.push_str(text);
            }
            Tok::Number(_) => out.push_str(&text.replace('.', &locale.decimal_separator().to_string())),
            Tok::Comma => out.push(if array_depth > 0 { locale.array_column_separator() } else { locale.list_separator() }),
            _ => out.push_str(text),
        }
    }
    Ok(out)
}

pub(crate) fn boolean_alias(name: &str) -> Option<bool> {
    let key = russian_key(name);
    if name.eq_ignore_ascii_case("VERDADERO") || key == "ИСТИНА" {
        Some(true)
    } else if name.eq_ignore_ascii_case("FALSO") || key == "ЛОЖЬ" {
        Some(false)
    } else {
        None
    }
}

fn normalize(
    expr: &mut Expr,
    locale: FormulaLocale,
    known_name: &impl Fn(&str) -> bool,
    bindings: &mut Vec<String>,
    depth: usize,
    display: bool,
) -> Result<(), ParseError> {
    if depth > parser::MAX_DEPTH {
        return Err(ParseError { msg: "formula is nested too deeply".into(), pos: 0 });
    }
    let bound = |name: &str, bindings: &[String]| bindings.iter().rev().any(|n| n.eq_ignore_ascii_case(name)) || known_name(name);
    match expr {
        Expr::Bool(value) if display => {
            let alias = locale.function_name(if *value { "TRUE" } else { "FALSE" });
            if !bound(alias, bindings) {
                *expr = Expr::Name(alias.into());
            }
        }
        Expr::Name(name) if !display && !bound(name, bindings) => {
            if let Some(value) = boolean_alias(name) {
                *expr = Expr::Bool(value);
            }
        }
        Expr::Call(name, args) if name == "LET" && args.len() >= 3 && !args.len().is_multiple_of(2) => {
            let saved = bindings.len();
            if let Some((body, pairs)) = args.split_last_mut() {
                for pair in pairs.chunks_mut(2) {
                    if let [declaration, value] = pair {
                        // The new name is not in scope in its own value expression.
                        normalize(value, locale, known_name, bindings, depth + 1, display)?;
                        if let Expr::Name(name) = declaration {
                            bindings.push(name.clone());
                        }
                    }
                }
                normalize(body, locale, known_name, bindings, depth + 1, display)?;
            }
            bindings.truncate(saved);
        }
        Expr::Call(name, args) if name == "LAMBDA" => {
            let saved = bindings.len();
            if let Some((body, params)) = args.split_last_mut() {
                for param in params {
                    if let Expr::Name(name) = param {
                        bindings.push(name.clone());
                    }
                }
                normalize(body, locale, known_name, bindings, depth + 1, display)?;
            }
            bindings.truncate(saved);
        }
        Expr::Call(name, args) => {
            let alias = if display { locale.function_name(name) } else { locale.canonical_name(name) };
            if !bound(name, bindings) && (!display || !bound(alias, bindings)) {
                *name = alias.to_string();
            }
            for arg in args {
                normalize(arg, locale, known_name, bindings, depth + 1, display)?;
            }
        }
        Expr::Invoke(callee, args) => {
            normalize(callee, locale, known_name, bindings, depth + 1, display)?;
            for arg in args {
                normalize(arg, locale, known_name, bindings, depth + 1, display)?;
            }
        }
        Expr::Unary(_, child) | Expr::Paren(child) => normalize(child, locale, known_name, bindings, depth + 1, display)?,
        Expr::Binary(_, left, right) => {
            normalize(left, locale, known_name, bindings, depth + 1, display)?;
            normalize(right, locale, known_name, bindings, depth + 1, display)?;
        }
        Expr::Array(rows) if display => {
            for row in rows {
                for child in row {
                    normalize(child, locale, known_name, bindings, depth + 1, display)?;
                }
            }
        }
        // Array elements have already been parsed as literals. Sheet/structured references
        // contain user-defined labels and must not be translated.
        _ => {}
    }
    Ok(())
}
