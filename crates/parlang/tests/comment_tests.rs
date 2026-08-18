use parlang::{parse, Expr};

#[test]
fn leading_line_comment() {
    assert_eq!(parse("-- comment\n42"), Ok(Expr::Int(42)));
}

#[test]
fn trailing_line_comment() {
    assert_eq!(parse("42 -- comment"), Ok(Expr::Int(42)));
}

#[test]
fn greedy_double_dash_is_line_comment() {
    // "1--2" should parse as 1 followed by a line comment "-2", not "1 - -2"
    assert_eq!(parse("1--2"), Ok(Expr::Int(1)));
}

#[test]
fn line_comment_running_to_eof_no_trailing_newline() {
    assert_eq!(
        parse("7 -- comment with no trailing newline"),
        Ok(Expr::Int(7))
    );
}

#[test]
fn leading_block_comment() {
    assert_eq!(parse("{- comment -} 42"), Ok(Expr::Int(42)));
}

#[test]
fn trailing_block_comment() {
    assert_eq!(parse("42 {- comment -}"), Ok(Expr::Int(42)));
}

#[test]
fn multi_line_block_comment() {
    let src = "{- this is\n   a multi-line\n   comment -}\n99";
    assert_eq!(parse(src), Ok(Expr::Int(99)));
}

#[test]
fn nested_block_comment() {
    assert_eq!(
        parse("{- outer {- inner -} still outer -} 9"),
        Ok(Expr::Int(9))
    );
}

#[test]
fn inline_comments_around_arithmetic_operands() {
    let expr = parse("1 {- one -} + {- plus -} 2 {- two -}").unwrap();
    let expected = parse("1 + 2").unwrap();
    assert_eq!(expr, expected);
}

#[test]
fn line_comment_between_operands() {
    let src = "1 +\n-- add two\n2";
    let expr = parse(src).unwrap();
    let expected = parse("1 + 2").unwrap();
    assert_eq!(expr, expected);
}

#[test]
fn comments_within_let_expression() {
    let src = "let x = 10 -- bind x\nin x + 1 -- use x";
    let expr = parse(src).unwrap();
    let expected = parse("let x = 10 in x + 1").unwrap();
    assert_eq!(expr, expected);
}

#[test]
fn unterminated_block_comment_is_error() {
    assert!(parse("{- unterminated").is_err());
}

#[test]
fn comment_between_if_branches() {
    let src = "if true -- cond\nthen 1 -- then branch\nelse 2 -- else branch";
    let expr = parse(src).unwrap();
    let expected = parse("if true then 1 else 2").unwrap();
    assert_eq!(expr, expected);
}

#[test]
fn comment_only_between_function_application() {
    let src = "(fun x -> x + 1) {- apply -} 41";
    let expr = parse(src).unwrap();
    let expected = parse("(fun x -> x + 1) 41").unwrap();
    assert_eq!(expr, expected);
}
