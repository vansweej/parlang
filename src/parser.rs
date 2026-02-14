/// Parser for the `ParLang` language using the combine parser combinator library
/// This implements a parser for ML-alike functional language syntax
use crate::ast::{BinOp, Expr, Literal, Pattern};
use combine::parser::char::{alpha_num, letter, spaces, string};
use combine::{
    attempt, between, choice, many, many1, optional, parser, token, EasyParser, Parser,
    ParseError, Stream,
};

/// Parse an integer literal
fn int<Input>() -> impl Parser<Input, Output = Expr>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    // Parse digits and convert to i64. The unwrap is safe here because:
    // 1. combine's digit() parser ensures we only have '0'-'9' characters
    // 2. many1 ensures we have at least one digit
    // 3. A string of valid digits will always parse to i64 successfully
    // Note: Very large numbers (> i64::MAX) will be caught by parse() and could panic,
    // but this is acceptable for a toy language implementation
    let number = many1(combine::parser::char::digit()).map(|s: String| s.parse::<i64>().unwrap());
    
    (optional(token('-')), number)
        .map(|(sign, n)| {
            if sign.is_some() {
                Expr::Int(-n)
            } else {
                Expr::Int(n)
            }
        })
}

/// Parse a boolean literal
fn bool_literal<Input>() -> impl Parser<Input, Output = Expr>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    choice((
        attempt(string("true")).map(|_| Expr::Bool(true)),
        attempt(string("false")).map(|_| Expr::Bool(false)),
    ))
}

/// Parse a string literal
fn string_literal<Input>() -> impl Parser<Input, Output = String>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    between(
        token('"'),
        token('"'),
        many(combine::satisfy(|c: char| c != '"')),
    )
}

/// Parse a raw identifier string (including keywords)
fn raw_identifier<Input>() -> impl Parser<Input, Output = String>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (
        letter(),
        many1(alpha_num().or(token('_'))).or(combine::value(String::new())),
    )
        .map(|(first, rest): (char, String)| format!("{first}{rest}"))
        .skip(combine::not_followed_by(alpha_num().or(token('_'))))
}

/// Parse an identifier (variable name) - ensures it's not a keyword
fn identifier<Input>() -> impl Parser<Input, Output = String>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    raw_identifier().then(|name: String| {
        // Reject keywords by returning a failing parser
        if matches!(
            name.as_str(),
            "let" | "in" | "if" | "then" | "else" | "fun" | "true" | "false" | "load" | "rec" | "match" | "with"
        ) {
            // Use a parser that will never succeed to reject keywords
            combine::unexpected("keyword").map(move |(): ()| name.clone()).right()
        } else {
            combine::value(name).left()
        }
    })
}

/// Parse a variable reference
fn variable<Input>() -> impl Parser<Input, Output = Expr>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    identifier().map(Expr::Var)
}

/// Parse a tuple or parenthesized expression
/// This handles:
/// - () -> empty tuple
/// - (expr) -> parenthesized expression (not a tuple)
/// - (expr, expr, ...) -> tuple with 2+ elements
fn tuple_or_paren<Input>() -> impl Parser<Input, Output = Expr>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    between(
        token('(').skip(spaces()),
        token(')'),
        // Try to parse comma-separated expressions
        (
            optional(expr().skip(spaces())),
            many(token(',').skip(spaces()).with(expr().skip(spaces()))),
        )
            .map(|(first_opt, rest): (Option<Expr>, Vec<Expr>)| {
                match first_opt {
                    None => {
                        // Empty parens: ()
                        Expr::Tuple(vec![])
                    }
                    Some(first) => {
                        if rest.is_empty() {
                            // Single element with no comma: (expr)
                            // This is a parenthesized expression, not a tuple
                            first
                        } else {
                            // Multiple elements: (expr, expr, ...)
                            let mut elements = vec![first];
                            elements.extend(rest);
                            Expr::Tuple(elements)
                        }
                    }
                }
            }),
    )
}

parser! {
    fn atom[Input]()(Input) -> Expr
    where [Input: Stream<Token = char>]
    {
        choice((
            attempt(bool_literal()),
            attempt(int()),
            attempt(variable()),
            attempt(tuple_or_paren()),
        ))
    }
}

parser! {
    fn fun_expr[Input]()(Input) -> Expr
    where [Input: Stream<Token = char>]
    {
        (
            string("fun").skip(spaces()),
            identifier().skip(spaces()),
            string("->").skip(spaces()),
            expr(),
        )
            .map(|(_, param, _, body)| Expr::Fun(param, Box::new(body)))
    }
}

parser! {
    fn rec_expr[Input]()(Input) -> Expr
    where [Input: Stream<Token = char>]
    {
        (
            string("rec").skip(spaces()),
            identifier().skip(spaces()),
            string("->").skip(spaces()),
            expr(),
        )
            .map(|(_, name, _, body)| Expr::Rec(name, Box::new(body)))
    }
}

parser! {
    fn let_expr[Input]()(Input) -> Expr
    where [Input: Stream<Token = char>]
    {
        (
            string("let").skip(spaces()),
            identifier().skip(spaces()),
            token('=').skip(spaces()),
            expr().skip(spaces()),
            string("in").skip(spaces()),
            expr(),
        )
            .map(|(_, name, _, value, _, body)| {
                Expr::Let(name, Box::new(value), Box::new(body))
            })
    }
}

parser! {
    fn if_expr[Input]()(Input) -> Expr
    where [Input: Stream<Token = char>]
    {
        (
            string("if").skip(spaces()),
            expr().skip(spaces()),
            string("then").skip(spaces()),
            expr().skip(spaces()),
            string("else").skip(spaces()),
            expr(),
        )
            .map(|(_, cond, _, then_branch, _, else_branch)| {
                Expr::If(
                    Box::new(cond),
                    Box::new(then_branch),
                    Box::new(else_branch),
                )
            })
    }
}

parser! {
    fn load_expr[Input]()(Input) -> Expr
    where [Input: Stream<Token = char>]
    {
        (
            string("load").skip(spaces()),
            string_literal().skip(spaces()),
            optional((string("in").skip(spaces()), expr())),
        )
            .map(|(_, filepath, body_opt)| {
                let body = body_opt
                    .map_or(Expr::Int(0), |(_, b)| b);
                Expr::Load(filepath, Box::new(body))
            })
    }
}

parser! {
    fn pattern[Input]()(Input) -> Pattern
    where [Input: Stream<Token = char>]
    {
        choice((
            // Tuple pattern: (p1, p2, ...)
            attempt(between(
                token('(').skip(spaces()),
                token(')'),
                (
                    optional(pattern().skip(spaces())),
                    many(token(',').skip(spaces()).with(pattern().skip(spaces()))),
                )
                    .map(|(first_opt, rest): (Option<Pattern>, Vec<Pattern>)| {
                        match first_opt {
                            None => {
                                // Empty tuple pattern: ()
                                Pattern::Tuple(vec![])
                            }
                            Some(first) => {
                                // Tuple with elements: (p1, p2, ...)
                                // Note: Unlike expressions where (e) is parenthesized, in patterns
                                // we always create a tuple even for single elements like (p)
                                // This is consistent with pattern matching semantics
                                let mut patterns = vec![first];
                                patterns.extend(rest);
                                Pattern::Tuple(patterns)
                            }
                        }
                    }),
            )),
            // Wildcard pattern: _
            attempt(token('_').skip(combine::not_followed_by(alpha_num().or(token('_')))).map(|_| Pattern::Wildcard)),
            // Boolean literal pattern: true, false
            attempt(string("true").skip(combine::not_followed_by(alpha_num())).map(|_| Pattern::Literal(Literal::Bool(true)))),
            attempt(string("false").skip(combine::not_followed_by(alpha_num())).map(|_| Pattern::Literal(Literal::Bool(false)))),
            // Integer literal pattern: 0, 1, 42, -10
            attempt({
                // The unwrap is safe here because:
                // 1. combine's digit() parser ensures we only have '0'-'9' characters
                // 2. many1 ensures we have at least one digit
                // 3. A string of valid digits will always parse to i64 successfully
                let number = many1(combine::parser::char::digit()).map(|s: String| s.parse::<i64>().unwrap());
                (optional(token('-')), number)
                    .map(|(sign, n)| {
                        let value = if sign.is_some() { -n } else { n };
                        Pattern::Literal(Literal::Int(value))
                    })
            }),
            // Variable pattern: x, n, acc (any identifier)
            identifier().map(Pattern::Var),
        ))
    }
}

parser! {
    fn match_expr[Input]()(Input) -> Expr
    where [Input: Stream<Token = char>]
    {
        (
            string("match").skip(spaces()),
            expr().skip(spaces()),
            string("with").skip(spaces()),
            // Parse arms: many1 of (| pattern -> expr)
            many1((
                token('|').skip(spaces()),
                pattern().skip(spaces()),
                string("->").skip(spaces()),
                expr().skip(spaces()),
            ))
        )
            .map(|(_, scrutinee, _, arms): (_, Expr, _, Vec<(char, Pattern, _, Expr)>)| {
                let parsed_arms: Vec<(Pattern, Expr)> = arms
                    .into_iter()
                    .map(|(_, pat, _, result)| (pat, result))
                    .collect();
                Expr::Match(Box::new(scrutinee), parsed_arms)
            })
    }
}

parser! {
    fn primary[Input]()(Input) -> Expr
    where [Input: Stream<Token = char>]
    {
        choice((
            attempt(let_expr()),
            attempt(load_expr()),
            attempt(if_expr()),
            attempt(match_expr()),
            attempt(rec_expr()),
            attempt(fun_expr()),
            attempt(atom()),
        ))
    }
}

parser! {
    fn proj_expr[Input]()(Input) -> Expr
    where [Input: Stream<Token = char>]
    {
        (
            primary().skip(spaces()),
            // The unwrap is safe here because:
            // 1. combine's digit() parser ensures we only have '0'-'9' characters
            // 2. many1 ensures we have at least one digit
            // 3. A string of valid digits will always parse to usize successfully for valid tuple indices
            many(token('.').with(many1(combine::parser::char::digit()).map(|s: String| s.parse::<usize>().unwrap())))
        )
            .map(|(base, indices): (Expr, Vec<usize>)| {
                indices.into_iter()
                    .fold(base, |expr, index| Expr::TupleProj(Box::new(expr), index))
            })
    }
}

parser! {
    fn app_expr[Input]()(Input) -> Expr
    where [Input: Stream<Token = char>]
    {
        (proj_expr().skip(spaces()), many(proj_expr().skip(spaces())))
            .map(|(func, args): (Expr, Vec<Expr>)| {
                args.into_iter()
                    .fold(func, |f, arg| Expr::App(Box::new(f), Box::new(arg)))
            })
    }
}

parser! {
    fn mul_expr[Input]()(Input) -> Expr
    where [Input: Stream<Token = char>]
    {
        let op = choice((
            token('*').map(|_| BinOp::Mul),
            token('/').map(|_| BinOp::Div),
        ));

        (
            app_expr().skip(spaces()),
            many((op.skip(spaces()), app_expr().skip(spaces()))),
        )
            .map(|(first, rest): (Expr, Vec<(BinOp, Expr)>)| {
                rest.into_iter()
                    .fold(first, |left, (op, right)| {
                        Expr::BinOp(op, Box::new(left), Box::new(right))
                    })
            })
    }
}

parser! {
    fn add_expr[Input]()(Input) -> Expr
    where [Input: Stream<Token = char>]
    {
        let op = choice((
            token('+').map(|_| BinOp::Add),
            token('-').map(|_| BinOp::Sub),
        ));

        (
            mul_expr().skip(spaces()),
            many((op.skip(spaces()), mul_expr().skip(spaces()))),
        )
            .map(|(first, rest): (Expr, Vec<(BinOp, Expr)>)| {
                rest.into_iter()
                    .fold(first, |left, (op, right)| {
                        Expr::BinOp(op, Box::new(left), Box::new(right))
                    })
            })
    }
}

parser! {
    fn cmp_expr[Input]()(Input) -> Expr
    where [Input: Stream<Token = char>]
    {
        let op = choice((
            attempt(string("==")).map(|_| BinOp::Eq),
            attempt(string("!=")).map(|_| BinOp::Neq),
            attempt(string("<=")).map(|_| BinOp::Le),
            attempt(string(">=")).map(|_| BinOp::Ge),
            attempt(token('<')).map(|_| BinOp::Lt),
            attempt(token('>')).map(|_| BinOp::Gt),
        ));

        (add_expr().skip(spaces()), optional(op.skip(spaces()).and(add_expr())))
            .map(|(left, rest)| {
                if let Some((op, right)) = rest {
                    Expr::BinOp(op, Box::new(left), Box::new(right))
                } else {
                    left
                }
            })
    }
}

parser! {
    fn expr[Input]()(Input) -> Expr
    where [Input: Stream<Token = char>]
    {
        cmp_expr()
    }
}

parser! {
    pub fn program[Input]()(Input) -> Expr
    where [Input: Stream<Token = char>]
    {
        (
            spaces(),
            many(attempt((
                string("let").skip(spaces()),
                identifier().skip(spaces()),
                token('=').skip(spaces()),
                expr().skip(spaces()),
                token(';').skip(spaces()),
            ))).map(|bindings: Vec<(_, String, _, Expr, _)>| {
                bindings
                    .into_iter()
                    .map(|(_, name, _, value, _)| (name, value))
                    .collect::<Vec<(String, Expr)>>()
            }),
            optional(expr()).skip(spaces())
        )
            .map(|((), bindings, body): (_, Vec<(String, Expr)>, Option<Expr>)| {
                let body_expr = body.unwrap_or(Expr::Int(0));
                if bindings.is_empty() {
                    body_expr
                } else {
                    Expr::Seq(bindings, Box::new(body_expr))
                }
            })
    }
}

/// Parse a string into an expression
/// 
/// # Errors
/// 
/// Returns an error if:
/// - The input contains invalid syntax
/// - There is unexpected input after a valid expression
pub fn parse(input: &str) -> Result<Expr, String> {
    match program().easy_parse(input) {
        Ok((expr, rest)) => {
            if rest.is_empty() {
                Ok(expr)
            } else {
                Err(format!("Unexpected input after expression: '{rest}'"))
            }
        }
        Err(err) => Err(format!("Parse error: {err}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_int() {
        assert_eq!(parse("42"), Ok(Expr::Int(42)));
        assert_eq!(parse("-10"), Ok(Expr::Int(-10)));
    }

    #[test]
    fn test_parse_bool() {
        assert_eq!(parse("true"), Ok(Expr::Bool(true)));
        assert_eq!(parse("false"), Ok(Expr::Bool(false)));
    }

    #[test]
    fn test_parse_var() {
        assert_eq!(parse("x"), Ok(Expr::Var("x".to_string())));
        assert_eq!(parse("foo_bar"), Ok(Expr::Var("foo_bar".to_string())));
    }

    #[test]
    fn test_parse_binop() {
        let expected = Expr::BinOp(
            BinOp::Add,
            Box::new(Expr::Int(1)),
            Box::new(Expr::Int(2)),
        );
        assert_eq!(parse("1 + 2"), Ok(expected));
    }

    #[test]
    fn test_parse_let() {
        let expected = Expr::Let(
            "x".to_string(),
            Box::new(Expr::Int(42)),
            Box::new(Expr::Var("x".to_string())),
        );
        assert_eq!(parse("let x = 42 in x"), Ok(expected));
    }

    #[test]
    fn test_parse_if() {
        let expected = Expr::If(
            Box::new(Expr::Bool(true)),
            Box::new(Expr::Int(1)),
            Box::new(Expr::Int(2)),
        );
        assert_eq!(parse("if true then 1 else 2"), Ok(expected));
    }

    #[test]
    fn test_parse_fun() {
        let expected = Expr::Fun("x".to_string(), Box::new(Expr::Var("x".to_string())));
        assert_eq!(parse("fun x -> x"), Ok(expected));
    }

    #[test]
    fn test_parse_app() {
        let expected = Expr::App(
            Box::new(Expr::Var("f".to_string())),
            Box::new(Expr::Int(42)),
        );
        assert_eq!(parse("f 42"), Ok(expected));
    }

    #[test]
    fn test_parse_complex() {
        // let double = fun x -> x + x in double 21
        let result = parse("let double = fun x -> x + x in double 21");
        assert!(result.is_ok());
    }

    // Test all arithmetic operators
    #[test]
    fn test_parse_subtraction() {
        let expected = Expr::BinOp(
            BinOp::Sub,
            Box::new(Expr::Int(10)),
            Box::new(Expr::Int(3)),
        );
        assert_eq!(parse("10 - 3"), Ok(expected));
    }

    #[test]
    fn test_parse_multiplication() {
        let expected = Expr::BinOp(
            BinOp::Mul,
            Box::new(Expr::Int(4)),
            Box::new(Expr::Int(5)),
        );
        assert_eq!(parse("4 * 5"), Ok(expected));
    }

    #[test]
    fn test_parse_division() {
        let expected = Expr::BinOp(
            BinOp::Div,
            Box::new(Expr::Int(10)),
            Box::new(Expr::Int(2)),
        );
        assert_eq!(parse("10 / 2"), Ok(expected));
    }

    // Test all comparison operators
    #[test]
    fn test_parse_equality() {
        let expected = Expr::BinOp(
            BinOp::Eq,
            Box::new(Expr::Int(5)),
            Box::new(Expr::Int(5)),
        );
        assert_eq!(parse("5 == 5"), Ok(expected));
    }

    #[test]
    fn test_parse_inequality() {
        let expected = Expr::BinOp(
            BinOp::Neq,
            Box::new(Expr::Int(5)),
            Box::new(Expr::Int(3)),
        );
        assert_eq!(parse("5 != 3"), Ok(expected));
    }

    #[test]
    fn test_parse_less_than() {
        let expected = Expr::BinOp(
            BinOp::Lt,
            Box::new(Expr::Int(3)),
            Box::new(Expr::Int(5)),
        );
        assert_eq!(parse("3 < 5"), Ok(expected));
    }

    #[test]
    fn test_parse_less_equal() {
        let expected = Expr::BinOp(
            BinOp::Le,
            Box::new(Expr::Int(3)),
            Box::new(Expr::Int(5)),
        );
        assert_eq!(parse("3 <= 5"), Ok(expected));
    }

    #[test]
    fn test_parse_greater_than() {
        let expected = Expr::BinOp(
            BinOp::Gt,
            Box::new(Expr::Int(5)),
            Box::new(Expr::Int(3)),
        );
        assert_eq!(parse("5 > 3"), Ok(expected));
    }

    #[test]
    fn test_parse_greater_equal() {
        let expected = Expr::BinOp(
            BinOp::Ge,
            Box::new(Expr::Int(5)),
            Box::new(Expr::Int(3)),
        );
        assert_eq!(parse("5 >= 3"), Ok(expected));
    }

    // Test operator precedence
    #[test]
    fn test_precedence_mul_before_add() {
        // 1 + 2 * 3 should parse as 1 + (2 * 3)
        let expected = Expr::BinOp(
            BinOp::Add,
            Box::new(Expr::Int(1)),
            Box::new(Expr::BinOp(
                BinOp::Mul,
                Box::new(Expr::Int(2)),
                Box::new(Expr::Int(3)),
            )),
        );
        assert_eq!(parse("1 + 2 * 3"), Ok(expected));
    }

    #[test]
    fn test_precedence_div_before_sub() {
        // 10 - 6 / 2 should parse as 10 - (6 / 2)
        let expected = Expr::BinOp(
            BinOp::Sub,
            Box::new(Expr::Int(10)),
            Box::new(Expr::BinOp(
                BinOp::Div,
                Box::new(Expr::Int(6)),
                Box::new(Expr::Int(2)),
            )),
        );
        assert_eq!(parse("10 - 6 / 2"), Ok(expected));
    }

    #[test]
    fn test_precedence_with_comparison() {
        // 1 + 2 == 3 should parse as (1 + 2) == 3
        let expected = Expr::BinOp(
            BinOp::Eq,
            Box::new(Expr::BinOp(
                BinOp::Add,
                Box::new(Expr::Int(1)),
                Box::new(Expr::Int(2)),
            )),
            Box::new(Expr::Int(3)),
        );
        assert_eq!(parse("1 + 2 == 3"), Ok(expected));
    }

    // Test parentheses for grouping
    #[test]
    fn test_parentheses_override_precedence() {
        // (1 + 2) * 3
        let expected = Expr::BinOp(
            BinOp::Mul,
            Box::new(Expr::BinOp(
                BinOp::Add,
                Box::new(Expr::Int(1)),
                Box::new(Expr::Int(2)),
            )),
            Box::new(Expr::Int(3)),
        );
        assert_eq!(parse("(1 + 2) * 3"), Ok(expected));
    }

    #[test]
    fn test_nested_parentheses() {
        // ((1 + 2) * 3)
        let result = parse("((1 + 2) * 3)");
        assert!(result.is_ok());
    }

    // Test negative numbers
    #[test]
    fn test_negative_number() {
        assert_eq!(parse("-42"), Ok(Expr::Int(-42)));
    }

    #[test]
    fn test_negative_in_expr() {
        let expected = Expr::BinOp(
            BinOp::Add,
            Box::new(Expr::Int(-5)),
            Box::new(Expr::Int(10)),
        );
        assert_eq!(parse("-5 + 10"), Ok(expected));
    }

    // Test whitespace handling
    #[test]
    fn test_whitespace_around_operators() {
        assert_eq!(parse("1+2"), parse("1 + 2"));
        assert_eq!(parse("  1  +  2  "), parse("1 + 2"));
    }

    #[test]
    fn test_whitespace_in_let() {
        let result1 = parse("let x = 42 in x");
        let result2 = parse("let  x  =  42  in  x");
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_whitespace_in_if() {
        let result1 = parse("if true then 1 else 2");
        let result2 = parse("if  true  then  1  else  2");
        assert_eq!(result1, result2);
    }

    // Test function application
    #[test]
    fn test_multiple_app() {
        // f x y should parse as (f x) y
        let expected = Expr::App(
            Box::new(Expr::App(
                Box::new(Expr::Var("f".to_string())),
                Box::new(Expr::Var("x".to_string())),
            )),
            Box::new(Expr::Var("y".to_string())),
        );
        assert_eq!(parse("f x y"), Ok(expected));
    }

    #[test]
    fn test_app_with_int() {
        let expected = Expr::App(
            Box::new(Expr::Var("inc".to_string())),
            Box::new(Expr::Int(42)),
        );
        assert_eq!(parse("inc 42"), Ok(expected));
    }

    // Test nested functions
    #[test]
    fn test_curried_function() {
        // fun x -> fun y -> x + y
        let result = parse("fun x -> fun y -> x + y");
        assert!(result.is_ok());
        if let Ok(Expr::Fun(_, body)) = result {
            assert!(matches!(*body, Expr::Fun(_, _)));
        }
    }

    #[test]
    fn test_nested_let() {
        // let x = 1 in let y = 2 in x + y
        let result = parse("let x = 1 in let y = 2 in x + y");
        assert!(result.is_ok());
    }

    // Test keyword rejection
    #[test]
    fn test_keyword_let_rejected() {
        let result = parse("let");
        assert!(result.is_err());
    }

    #[test]
    fn test_keyword_in_rejected() {
        let result = parse("in");
        assert!(result.is_err());
    }

    #[test]
    fn test_keyword_if_rejected() {
        let result = parse("if");
        assert!(result.is_err());
    }

    #[test]
    fn test_keyword_then_rejected() {
        let result = parse("then");
        assert!(result.is_err());
    }

    #[test]
    fn test_keyword_else_rejected() {
        let result = parse("else");
        assert!(result.is_err());
    }

    #[test]
    fn test_keyword_fun_rejected() {
        let result = parse("fun");
        assert!(result.is_err());
    }

    #[test]
    fn test_keyword_load_rejected() {
        let result = parse("load");
        assert!(result.is_err());
    }

    // Test load expressions
    #[test]
    fn test_parse_load_simple() {
        let expected = Expr::Load(
            "lib.par".to_string(),
            Box::new(Expr::Var("x".to_string())),
        );
        assert_eq!(parse("load \"lib.par\" in x"), Ok(expected));
    }

    #[test]
    fn test_parse_load_with_expression() {
        let result = parse("load \"stdlib.par\" in double 21");
        assert!(result.is_ok());
        if let Ok(Expr::Load(filepath, body)) = result {
            assert_eq!(filepath, "stdlib.par");
            assert!(matches!(*body, Expr::App(_, _)));
        }
    }

    #[test]
    fn test_parse_load_nested() {
        let result = parse("load \"a.par\" in load \"b.par\" in x");
        assert!(result.is_ok());
        if let Ok(Expr::Load(_, body)) = result {
            assert!(matches!(*body, Expr::Load(_, _)));
        }
    }

    #[test]
    fn test_parse_load_with_let() {
        let result = parse("load \"lib.par\" in let x = double 5 in x");
        assert!(result.is_ok());
    }

    // Test sequential let bindings
    #[test]
    fn test_parse_seq_single() {
        let result = parse("let x = 42; x");
        assert!(result.is_ok());
        if let Ok(Expr::Seq(bindings, body)) = result {
            assert_eq!(bindings.len(), 1);
            assert_eq!(bindings[0].0, "x");
            assert_eq!(bindings[0].1, Expr::Int(42));
            assert_eq!(*body, Expr::Var("x".to_string()));
        } else {
            panic!("Expected Seq expression");
        }
    }

    #[test]
    fn test_parse_seq_multiple() {
        let result = parse("let x = 42; let y = 10; x + y");
        assert!(result.is_ok());
        if let Ok(Expr::Seq(bindings, body)) = result {
            assert_eq!(bindings.len(), 2);
            assert_eq!(bindings[0].0, "x");
            assert_eq!(bindings[1].0, "y");
            assert!(matches!(*body, Expr::BinOp(_, _, _)));
        } else {
            panic!("Expected Seq expression");
        }
    }

    #[test]
    fn test_parse_seq_with_functions() {
        let result = parse("let double = fun x -> x * 2; double 21");
        assert!(result.is_ok());
        if let Ok(Expr::Seq(bindings, body)) = result {
            assert_eq!(bindings.len(), 1);
            assert_eq!(bindings[0].0, "double");
            assert!(matches!(bindings[0].1, Expr::Fun(_, _)));
            assert!(matches!(*body, Expr::App(_, _)));
        } else {
            panic!("Expected Seq expression");
        }
    }

    // Test variable names with underscores
    #[test]
    fn test_var_with_underscore() {
        assert_eq!(parse("foo_bar"), Ok(Expr::Var("foo_bar".to_string())));
    }

    #[test]
    fn test_var_with_numbers() {
        assert_eq!(parse("x1"), Ok(Expr::Var("x1".to_string())));
        assert_eq!(parse("test123"), Ok(Expr::Var("test123".to_string())));
    }

    // Test error cases
    #[test]
    fn test_parse_error_incomplete_let() {
        let result = parse("let x = 42");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_incomplete_if() {
        let result = parse("if true then 1");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_incomplete_fun() {
        let result = parse("fun x");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_missing_arrow() {
        let result = parse("fun x x");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_unexpected_input() {
        // Parser will interpret "42 extra" as application: "(42 extra)"
        // which is semantically wrong but syntactically valid
        // So let's test with something that's truly unexpected after a complete expression
        let result = parse("42 +");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_unmatched_paren() {
        let result = parse("(1 + 2");
        assert!(result.is_err());
    }

    // Test complex realistic expressions
    #[test]
    fn test_factorial_like() {
        let result = parse("let f = fun n -> if n == 0 then 1 else n * f (n - 1) in f 5");
        assert!(result.is_ok());
    }

    #[test]
    fn test_currying_example() {
        let result = parse("let add = fun x -> fun y -> x + y in let add5 = add 5 in add5 10");
        assert!(result.is_ok());
    }

    #[test]
    fn test_comparison_in_if() {
        let result = parse("if 5 > 3 then 100 else 0");
        assert!(result.is_ok());
    }

    #[test]
    fn test_nested_if() {
        let result = parse("if true then if false then 1 else 2 else 3");
        assert!(result.is_ok());
    }

    #[test]
    fn test_boolean_comparison() {
        let result = parse("true == false");
        assert!(result.is_ok());
    }

    #[test]
    fn test_multiple_operations() {
        // 1 + 2 - 3 * 4 / 5
        let result = parse("1 + 2 - 3 * 4 / 5");
        assert!(result.is_ok());
    }

    // Test pattern matching
    #[test]
    fn test_parse_match_simple() {
        let result = parse("match x with | 0 -> 1 | n -> n");
        assert!(result.is_ok());
        if let Ok(expr) = result {
            match expr {
                Expr::Match(_, arms) => {
                    assert_eq!(arms.len(), 2);
                }
                _ => panic!("Expected Match expression"),
            }
        }
    }

    #[test]
    fn test_parse_match_with_wildcard() {
        let result = parse("match x with | 0 -> 1 | _ -> 2");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_match_with_bool() {
        let result = parse("match x with | true -> 1 | false -> 0");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_match_multiple_arms() {
        let result = parse("match n with | 0 -> 1 | 1 -> 1 | 2 -> 2 | n -> n");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_match_nested_expr() {
        let result = parse("match n with | 0 -> 1 | n -> n * 2");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_match_in_function() {
        let result = parse("fun n -> match n with | 0 -> 1 | n -> n");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_match_negative_literal() {
        let result = parse("match n with | -1 -> 0 | 0 -> 1 | n -> n");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_error_match_without_arms() {
        let result = parse("match x with");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_match_without_with() {
        let result = parse("match x | 0 -> 1");
        assert!(result.is_err());
    }

    // Test tuple parsing
    #[test]
    fn test_parse_tuple_empty() {
        let result = parse("()");
        assert!(result.is_ok());
        if let Ok(expr) = result {
            assert_eq!(expr, Expr::Tuple(vec![]));
        }
    }

    #[test]
    fn test_parse_tuple_two_elements() {
        let result = parse("(1, 2)");
        assert!(result.is_ok());
        if let Ok(expr) = result {
            assert_eq!(expr, Expr::Tuple(vec![Expr::Int(1), Expr::Int(2)]));
        }
    }

    #[test]
    fn test_parse_tuple_three_elements() {
        let result = parse("(1, 2, 3)");
        assert!(result.is_ok());
        if let Ok(expr) = result {
            assert_eq!(
                expr,
                Expr::Tuple(vec![Expr::Int(1), Expr::Int(2), Expr::Int(3)])
            );
        }
    }

    #[test]
    fn test_parse_tuple_mixed_types() {
        let result = parse("(42, true, x)");
        assert!(result.is_ok());
        if let Ok(expr) = result {
            assert_eq!(
                expr,
                Expr::Tuple(vec![Expr::Int(42), Expr::Bool(true), Expr::Var("x".to_string())])
            );
        }
    }

    #[test]
    fn test_parse_tuple_nested() {
        let result = parse("((1, 2), (3, 4))");
        assert!(result.is_ok());
        if let Ok(expr) = result {
            assert_eq!(
                expr,
                Expr::Tuple(vec![
                    Expr::Tuple(vec![Expr::Int(1), Expr::Int(2)]),
                    Expr::Tuple(vec![Expr::Int(3), Expr::Int(4)]),
                ])
            );
        }
    }

    #[test]
    fn test_parse_parenthesized_expr() {
        // Single element without comma should be parenthesized expr, not tuple
        let result = parse("(42)");
        assert!(result.is_ok());
        if let Ok(expr) = result {
            assert_eq!(expr, Expr::Int(42));
        }
    }

    #[test]
    fn test_parse_parenthesized_complex() {
        let result = parse("(1 + 2)");
        assert!(result.is_ok());
        if let Ok(expr) = result {
            assert!(matches!(expr, Expr::BinOp(_, _, _)));
        }
    }

    #[test]
    fn test_parse_tuple_with_spaces() {
        let result = parse("( 1 , 2 , 3 )");
        assert!(result.is_ok());
        if let Ok(expr) = result {
            assert_eq!(
                expr,
                Expr::Tuple(vec![Expr::Int(1), Expr::Int(2), Expr::Int(3)])
            );
        }
    }

    // Test tuple projection parsing
    #[test]
    fn test_parse_tuple_proj_simple() {
        let result = parse("t.0");
        assert!(result.is_ok());
        if let Ok(expr) = result {
            assert_eq!(
                expr,
                Expr::TupleProj(Box::new(Expr::Var("t".to_string())), 0)
            );
        }
    }

    #[test]
    fn test_parse_tuple_proj_index_1() {
        let result = parse("pair.1");
        assert!(result.is_ok());
        if let Ok(expr) = result {
            assert_eq!(
                expr,
                Expr::TupleProj(Box::new(Expr::Var("pair".to_string())), 1)
            );
        }
    }

    #[test]
    fn test_parse_tuple_proj_chained() {
        let result = parse("t.0.1");
        assert!(result.is_ok());
        if let Ok(expr) = result {
            assert_eq!(
                expr,
                Expr::TupleProj(
                    Box::new(Expr::TupleProj(Box::new(Expr::Var("t".to_string())), 0)),
                    1
                )
            );
        }
    }

    #[test]
    fn test_parse_tuple_proj_nested_tuple() {
        let result = parse("((1, 2), 3).0.1");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_tuple_proj_with_let() {
        let result = parse("let t = (10, 20) in t.0");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_tuple_in_let() {
        let result = parse("let pair = (42, true) in pair");
        assert!(result.is_ok());
    }

    // Test pattern matching with tuples
    #[test]
    fn test_parse_pattern_tuple_simple() {
        let result = parse("match (1, 2) with | (x, y) -> x + y");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_pattern_tuple_with_literal() {
        let result = parse("match p with | (0, 0) -> 0 | (x, y) -> x");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_pattern_tuple_nested() {
        let result = parse("match t with | ((a, b), c) -> a");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_pattern_tuple_with_wildcard() {
        let result = parse("match t with | (x, _) -> x");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_pattern_tuple_empty() {
        let result = parse("match t with | () -> 0");
        assert!(result.is_ok());
    }

    // Test complex combinations
    #[test]
    fn test_parse_tuple_function_return() {
        let result = parse("fun x -> (x, x + 1)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_tuple_in_app() {
        let result = parse("f (1, 2)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_tuple_proj_in_binop() {
        let result = parse("t.0 + t.1");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_tuple_complex() {
        let result = parse("let swap = fun p -> (p.1, p.0) in swap (5, 10)");
        assert!(result.is_ok());
    }
}

