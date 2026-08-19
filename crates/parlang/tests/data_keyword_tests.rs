use parlang::parse_expr as parse;

#[test]
fn test_data_keyword_parses_adt() {
    assert!(parse("data Color = Red | Green in Red").is_ok());
}

#[test]
fn test_type_keyword_rejected_for_adt() {
    // Note: this synonym acceptance is intentionally removed in Slice 2b Plan B.
    assert!(parse("type Color = Red | Green in Red").is_err());
}

#[test]
fn test_data_adt_display_uses_data_word() {
    let expr = parse("data Color = Red | Green in Red").expect("parse should succeed");
    let formatted = format!("{expr}");
    assert!(formatted.contains("(data Color"));
    assert!(!formatted.contains("(type Color"));
}
