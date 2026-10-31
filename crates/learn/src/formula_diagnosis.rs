//! Deterministic, no-LLM diagnosis of a learner's formula mistakes.
//!
//! The app types a raw formula (or a cell error value) and gets back the *mechanical* class of
//! mistake so it can route to a targeted drill of that atomic operation and log a "gap". Nothing
//! here is heuristic about intent: every classification is a function of the real parser output
//! or a direct scan of the raw text.
//!
//! The canonical parser ([`gridcraft_engine::formula::parse`]) understands the English function
//! table and English separators. A learner may type either language, so [`diagnose`] also accepts
//! the localized parser ([`parse_input`] with [`FormulaLocale::Ru`]) for structurally valid
//! Russian input, and recognizes Russian function names and error values.

use gridcraft_engine::{
    calc::is_known_function,
    core::{CellError, CellRef, RangeRef},
    formula::{
        Expr, FormulaLocale,
        lexer::{Tok, Token, tokenize},
        parse, parse_input,
    },
};

/// The mechanically-practisable classes of formula mistakes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FormulaIssue {
    /// `(` and `)` do not pair up (`=SUM(A1:A3`).
    UnbalancedParens,
    /// Wrong argument separator — e.g. `;` in an English formula or mixed `,`/`;`.
    BadArgumentSeparator,
    /// A function call placed where a value/operand is required, e.g.
    /// `=ЕСЛИ(B2>C2(=ЕСЛИ(D2>2));"Нет")`.
    CallInsideOperand,
    /// A called name that is not a known built-in function (`=НОВАЯФУНКЦИЯ(A1)`).
    UnknownFunction,
    /// A reference that is not a valid cell/column/row range (`=SUM(A1:)`, `=A1::B1`).
    BadRangeReference,
    /// A text literal whose closing `"` is missing (`=IF(A1>0,"Да)`).
    MissingClosingQuote,
    /// Decimal/list separator confusion (`1,22` / `;` where the other is expected).
    WrongDecimalOrListSeparator,
    /// The argument count or order looks wrong (e.g. far too many arguments).
    ArityOrArgOrderSuspected,
    /// The text does not start with `=` and is not a cell error value.
    NotAFormula,
    /// A real Excel error value supplied by the engine, kept verbatim.
    EngineError(String),
    /// Parses cleanly, or the failure does not match any known mechanical class.
    Other,
}

/// Classifies a raw formula the learner typed, or a cell error value.
///
/// A leading/trailing whitespace is ignored. Text starting with `=` is parsed; a clean parse is
/// [`FormulaIssue::Other`] (unless it calls an unknown function). Otherwise the parser error and
/// the raw text are scanned, most specific signal first.
pub fn diagnose(input: &str) -> FormulaIssue {
    let text = input.trim();
    if text.is_empty() {
        return FormulaIssue::NotAFormula;
    }
    if let Some(issue) = engine_error(text) {
        return issue;
    }
    let Some(body) = text.strip_prefix('=') else {
        return FormulaIssue::NotAFormula;
    };
    if body.trim().is_empty() {
        return FormulaIssue::NotAFormula;
    }

    // Canonical (English) syntax first: this is what gets stored and recalculated.
    let en = parse(body);
    if let Ok(expr) = &en {
        return successful_parse(expr);
    }
    // A structurally valid Russian formula is not a mistake.
    let ru = parse_input(body, FormulaLocale::Ru, |_| false);
    if let Ok(expr) = &ru {
        return successful_parse(expr);
    }

    let en_msg = en.as_ref().err().map(|e| e.msg.as_str());
    let ru_msg = ru.as_ref().err().map(|e| e.msg.as_str());
    classify(body, en_msg, ru_msg)
}

/// A short Russian instruction naming the atomic action to practise.
pub fn practice_hint(issue: &FormulaIssue) -> &'static str {
    match issue {
        FormulaIssue::UnbalancedParens => "Скобки: следи за парностью и порядком аргументов ЕСЛИ",
        FormulaIssue::BadArgumentSeparator => "Разделители: аргументы функции разделяй точкой с запятой",
        FormulaIssue::CallInsideOperand => "Функция — это значение: убери вызов там, где уже стоит операнд",
        FormulaIssue::UnknownFunction => "Имя функции: проверь написание по списку функций Excel",
        FormulaIssue::BadRangeReference => "Диапазон: обе границы через двоеточие, например A1:A3",
        FormulaIssue::MissingClosingQuote => "Кавычки: текст закрывается парной кавычкой",
        FormulaIssue::WrongDecimalOrListSeparator => "Дробная часть — запятая, аргументы — точка с запятой",
        FormulaIssue::ArityOrArgOrderSuspected => "Аргументы функции: проверь их число и порядок",
        FormulaIssue::NotAFormula => "Формула начинается со знака =",
        FormulaIssue::EngineError(_) => "Ошибка вычисления: проверь типы данных и ссылки",
        FormulaIssue::Other => "Структурной ошибки нет — формула разобрана",
    }
}

/// The closest existing skill in [`crate::content::SKILLS`], if any.
///
/// Only ids that really exist are returned; the lookup goes through [`crate::content::skill`].
pub fn skill_for(issue: &FormulaIssue) -> Option<&'static str> {
    let id = match issue {
        FormulaIssue::CallInsideOperand | FormulaIssue::UnbalancedParens | FormulaIssue::ArityOrArgOrderSuspected => "logic.if",
        FormulaIssue::BadRangeReference => "refs.abs",
        FormulaIssue::EngineError(e) if is_engine_arith_error(e) => "logic.iferror",
        _ => return None,
    };
    crate::content::skill(id).map(|_| id)
}

/// Whether the text is a known cell error value (canonical English or Russian display).
///
/// `#NAME?`/`#ИМЯ?` is reported as [`FormulaIssue::UnknownFunction`] — the learner's mistake is
/// the name, not the arithmetic. Every other error value is kept verbatim.
fn engine_error(s: &str) -> Option<FormulaIssue> {
    for e in CellError::ALL.iter().chain(std::iter::once(&CellError::Circ)) {
        let tok = e.as_str();
        if s.get(..tok.len()).is_some_and(|p| p.eq_ignore_ascii_case(tok)) {
            return Some(if matches!(e, CellError::Name) { FormulaIssue::UnknownFunction } else { FormulaIssue::EngineError(s.to_string()) });
        }
    }
    // Russian Excel error values (as used by `crate::sheet::error_ru`), longest tokens first.
    const RU: &[(&str, bool)] = &[
        ("#ДЕЛ/0!", false),
        ("#ЗНАЧ!", false),
        ("#ССЫЛКА!", false),
        ("#ПЕРЕНОС!", false),
        ("#ЧИСЛО!", false),
        ("#ПУСТО!", false),
        ("#КАЛК!", false),
        ("#ЦИКЛ!", false),
        ("#ИМЯ?", true),
        ("#Н/Д", false),
    ];
    for (tok, is_name) in RU {
        if s.starts_with(tok) {
            return Some(if *is_name { FormulaIssue::UnknownFunction } else { FormulaIssue::EngineError(s.to_string()) });
        }
    }
    None
}

/// A clean parse is fine unless it calls a name the engine does not know.
fn successful_parse(expr: &Expr) -> FormulaIssue {
    let functions = expr.functions();
    // LET/LAMBDA bind parameters that are invoked as calls; without the workbook's defined names
    // those are indistinguishable from typos, so stay conservative and do not flag them.
    let dynamic = functions.iter().any(|n| n.eq_ignore_ascii_case("LET") || n.eq_ignore_ascii_case("LAMBDA"));
    if !dynamic && functions.iter().any(|n| !function_known(n)) {
        return FormulaIssue::UnknownFunction;
    }
    FormulaIssue::Other
}

/// Whether `name` is a built-in function in English, Russian or Spanish spelling.
fn function_known(name: &str) -> bool {
    if is_known_function(name) {
        return true;
    }
    [FormulaLocale::Ru, FormulaLocale::Es].iter().any(|locale| {
        let canonical = locale.canonical_name(name);
        !canonical.eq_ignore_ascii_case(name) && is_known_function(canonical)
    })
}

fn is_engine_arith_error(e: &str) -> bool {
    const MARKERS: [&str; 6] = ["#DIV/0", "#ДЕЛ/0", "#VALUE", "#ЗНАЧ", "#NUM", "#ЧИСЛО"];
    MARKERS.iter().any(|m| e.contains(m))
}

/// Scans the raw text and both parse errors when neither locale parsed.
fn classify(body: &str, en_msg: Option<&str>, ru_msg: Option<&str>) -> FormulaIssue {
    // Odd number of `"` cannot be a valid text literal (doubled `""` is even).
    if body.matches('"').count() % 2 == 1 {
        return FormulaIssue::MissingClosingQuote;
    }

    let outside = chars_outside_quotes(body);
    let mut depth = 0i32;
    let mut went_negative = false;
    for &(_, c) in &outside {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                went_negative |= depth < 0;
            }
            _ => {}
        }
    }
    if depth != 0 || went_negative {
        return FormulaIssue::UnbalancedParens;
    }

    let tokens = tokenize(body).ok();
    if let Some(tokens) = &tokens
        && call_inside_operand(tokens)
    {
        return FormulaIssue::CallInsideOperand;
    }

    if [en_msg, ru_msg].into_iter().flatten().any(|m| m.contains("too many arguments")) {
        return FormulaIssue::ArityOrArgOrderSuspected;
    }

    if let Some(tokens) = &tokens {
        if tokens.iter().any(|t| matches!(&t.tok, Tok::Func(n) if !function_known(n) && !looks_like_ref(n))) {
            return FormulaIssue::UnknownFunction;
        }
        if bad_range(tokens) {
            return FormulaIssue::BadRangeReference;
        }
    }

    let has_comma = outside.iter().any(|&(_, c)| c == ',');
    let has_semicolon = outside.iter().any(|&(_, c)| c == ';');
    let decimal_comma = outside.windows(3).any(|w| w[0].1.is_ascii_digit() && w[1].1 == ',' && w[2].1.is_ascii_digit());
    if has_comma && has_semicolon {
        return FormulaIssue::BadArgumentSeparator;
    }
    if decimal_comma {
        return FormulaIssue::WrongDecimalOrListSeparator;
    }
    if has_semicolon {
        return FormulaIssue::BadArgumentSeparator;
    }
    FormulaIssue::Other
}

/// Detects `operand(` — a call where a value is expected.
///
/// The lexer folds a word directly followed by `(` into [`Tok::Func`], so a cell reference such as
/// `C2(` arrives as `Func("C2")`; a plain value such as `2(` leaves a separate [`Tok::LParen`].
fn call_inside_operand(tokens: &[Token]) -> bool {
    let mut prev: Option<&Tok> = None;
    for token in tokens {
        match &token.tok {
            Tok::Space => {}
            Tok::LParen => {
                if matches!(prev, Some(Tok::Number(_) | Tok::Text(_) | Tok::Bool(_) | Tok::Error(_) | Tok::RBrace)) {
                    return true;
                }
            }
            Tok::Func(name) if !function_known(name) && looks_like_ref(name) => return true,
            _ => {}
        }
        if !matches!(token.tok, Tok::Space) {
            prev = Some(&token.tok);
        }
    }
    false
}

/// A colon must sit between two reference-like ends (`A1:A3`, `A:A`, `3:5`).
fn bad_range(tokens: &[Token]) -> bool {
    let sig: Vec<&Tok> = tokens.iter().map(|t| &t.tok).filter(|t| !matches!(t, Tok::Space)).collect();
    for (i, tok) in sig.iter().enumerate() {
        if matches!(tok, Tok::Colon) {
            let left_ok = i > 0 && matches!(sig[i - 1], Tok::Word(_) | Tok::Number(_) | Tok::RParen);
            let right_ok = sig.get(i + 1).is_some_and(|n| matches!(n, Tok::Word(_) | Tok::Number(_)));
            if !left_ok || !right_ok {
                return true;
            }
        }
    }
    false
}

fn looks_like_ref(name: &str) -> bool {
    CellRef::parse(name).is_some() || RangeRef::parse(name).is_some()
}

/// Characters outside double-quoted text, with their byte positions. `""` inside a literal is an
/// escaped quote and does not close it.
fn chars_outside_quotes(s: &str) -> Vec<(usize, char)> {
    let chars: Vec<(usize, char)> = s.char_indices().collect();
    let mut out = Vec::with_capacity(chars.len());
    let mut in_string = false;
    let mut i = 0;
    while i < chars.len() {
        let (pos, c) = chars[i];
        if c == '"' {
            if in_string && chars.get(i + 1).is_some_and(|&(_, n)| n == '"') {
                i += 2;
                continue;
            }
            in_string = !in_string;
            i += 1;
            continue;
        }
        if !in_string {
            out.push((pos, c));
        }
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn call_inside_operand_is_flagged() {
        let issue = diagnose("=ЕСЛИ(B2>C2(=ЕСЛИ(D2>2));\"Нет\")");
        assert_eq!(issue, FormulaIssue::CallInsideOperand);
    }

    #[test]
    fn balanced_valid_formulas_have_no_structural_error() {
        for formula in ["=SUM(A1:A3)", "=IF(B2>C2,\"Да\",\"Нет\")", "=СУММ(A1:A3)", "=СУММ(A1;A2)", "=ЕСЛИ(B2>C2;\"Да\";\"Нет\")"]
        {
            assert_eq!(diagnose(formula), FormulaIssue::Other, "{formula}");
        }
    }

    #[test]
    fn separator_mistakes_are_classified() {
        // Russian list separator mixed with a comma separator in the same call.
        assert_eq!(diagnose("=SUM(A1;A2,A3)"), FormulaIssue::BadArgumentSeparator);
        // A malformed decimal-comma number that no locale can parse.
        assert_eq!(diagnose("=A1*1,22,5"), FormulaIssue::WrongDecimalOrListSeparator);
    }

    #[test]
    fn unknown_function_is_flagged() {
        assert_eq!(diagnose("=НЕСУЩЕСТВУЮЩАЯФУНКЦИЯ(A1)"), FormulaIssue::UnknownFunction);
        assert_eq!(diagnose("=BOGUSFUNC(A1)"), FormulaIssue::UnknownFunction);
    }

    #[test]
    fn unbalanced_parens_are_flagged() {
        assert_eq!(diagnose("=SUM(A1:A3"), FormulaIssue::UnbalancedParens);
        assert_eq!(diagnose("=SUM(A1:A3))"), FormulaIssue::UnbalancedParens);
    }

    #[test]
    fn missing_closing_quote_is_flagged() {
        assert_eq!(diagnose("=IF(A1>0,\"Да,B1)"), FormulaIssue::MissingClosingQuote);
    }

    #[test]
    fn bad_range_is_flagged() {
        assert_eq!(diagnose("=SUM(A1:)"), FormulaIssue::BadRangeReference);
    }

    #[test]
    fn engine_error_values_are_carried_through() {
        assert_eq!(diagnose("#ЗНАЧ!"), FormulaIssue::EngineError("#ЗНАЧ!".into()));
        assert_eq!(diagnose("#ДЕЛ/0!"), FormulaIssue::EngineError("#ДЕЛ/0!".into()));
        assert_eq!(diagnose("#N/A"), FormulaIssue::EngineError("#N/A".into()));
        assert_eq!(diagnose("#NAME?"), FormulaIssue::UnknownFunction);
        assert_eq!(diagnose("#ИМЯ?"), FormulaIssue::UnknownFunction);
    }

    #[test]
    fn plain_text_is_not_a_formula() {
        assert_eq!(diagnose("привет"), FormulaIssue::NotAFormula);
        assert_eq!(diagnose("123"), FormulaIssue::NotAFormula);
        assert_eq!(diagnose("   "), FormulaIssue::NotAFormula);
        assert_eq!(diagnose("="), FormulaIssue::NotAFormula);
    }

    #[test]
    fn skill_for_only_returns_existing_skills() {
        for issue in [
            FormulaIssue::CallInsideOperand,
            FormulaIssue::UnbalancedParens,
            FormulaIssue::BadRangeReference,
            FormulaIssue::ArityOrArgOrderSuspected,
            FormulaIssue::EngineError("#ДЕЛ/0!".into()),
        ] {
            let skill = skill_for(&issue).expect("a mapped issue has a skill");
            assert!(crate::content::skill(skill).is_some(), "missing skill {skill}");
        }
        assert_eq!(skill_for(&FormulaIssue::Other), None);
        assert_eq!(skill_for(&FormulaIssue::UnknownFunction), None);
    }

    #[test]
    fn hints_are_not_empty() {
        for issue in [
            FormulaIssue::UnbalancedParens,
            FormulaIssue::BadArgumentSeparator,
            FormulaIssue::CallInsideOperand,
            FormulaIssue::UnknownFunction,
            FormulaIssue::BadRangeReference,
            FormulaIssue::MissingClosingQuote,
            FormulaIssue::WrongDecimalOrListSeparator,
            FormulaIssue::ArityOrArgOrderSuspected,
            FormulaIssue::NotAFormula,
            FormulaIssue::EngineError("#REF!".into()),
            FormulaIssue::Other,
        ] {
            assert!(!practice_hint(&issue).is_empty());
        }
    }
}
