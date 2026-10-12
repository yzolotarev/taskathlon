use gridcraft_formula::{FormulaLocale, parse, parse_input, print, print_input};

fn input(src: &str) -> String {
    print(&parse_input(src, FormulaLocale::Ru, |_| false).unwrap())
}

fn shown(canonical: &str) -> String {
    print_input(&parse(canonical).unwrap(), FormulaLocale::Ru, |_| false).unwrap()
}

#[test]
fn russian_names_separators_and_decimals() {
    for (src, expected) in [
        ("=СУММ(A1:A10)", "SUM(A1:A10)"),
        ("=сумм(A1:A10)", "SUM(A1:A10)"),
        ("=СУММЕСЛИ(C2:C9;\"Север\";A2:A9)", "SUMIF(C2:C9,\"Север\",A2:A9)"),
        ("=СЧЕТЕСЛИ(C2:C9;\">0\")", "COUNTIF(C2:C9,\">0\")"),
        ("=ВПР(G5;A2:B9;2;ЛОЖЬ)", "VLOOKUP(G5,A2:B9,2,FALSE)"),
        ("=ПРОСМОТРX(G5;A2:A9;B2:B9)", "XLOOKUP(G5,A2:A9,B2:B9)"),
        ("=ПРОСМОТРХ(G5;A2:A9;B2:B9)", "XLOOKUP(G5,A2:A9,B2:B9)"),
        ("=ОКРУГЛ(A2*0,2;2)", "ROUND(A2*0.2,2)"),
        ("=ЕСЛИОШИБКА(A2/B2;0)", "IFERROR(A2/B2,0)"),
        ("=ЕСЛИ(И(A2>0;B2>0);ИСТИНА;ЛОЖЬ)", "IF(AND(A2>0,B2>0),TRUE,FALSE)"),
        ("=ПРОМЕЖУТОЧНЫЕ.ИТОГИ(9;A2:A9)", "SUBTOTAL(9,A2:A9)"),
        ("=\"СУММ;1,5\"&A1", "\"СУММ;1,5\"&A1"),
    ] {
        assert_eq!(input(src), expected, "{src}");
    }
}

#[test]
fn russian_display_round_trips() {
    for canonical in
        ["SUM(A1:A10)", "SUMIFS(D2:D9,A2:A9,\"ООО Ромашка\",B2:B9,\">=\"&F1)", "ROUND(A2*0.2,2)", "IF(A1,TRUE,FALSE)", "XLOOKUP(G5,A2:A9,B2:B9)"]
    {
        let ru = shown(canonical);
        assert_eq!(input(&ru), canonical, "{ru}");
    }
    assert_eq!(shown("ROUND(A2*0.2,2)"), "ОКРУГЛ(A2*0,2;2)");
    assert_eq!(shown("COUNTIF(A1:A3,1)"), "СЧЁТЕСЛИ(A1:A3;1)");
}
