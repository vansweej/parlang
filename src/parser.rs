/// Parser for the ParLang language using the combine parser combinator library
/// This implements a parser for ML-alike functional language syntax

use crate::ast::{BinOp, Expr};
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
        .map(|(first, rest): (char, String)| format!("{}{}", first, rest))
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
            "let" | "in" | "if" | "then" | "else" | "fun" | "true" | "false" | "load"
        ) {
            // Use a parser that will never succeed to reject keywords
            combine::unexpected("keyword").map(move |_: ()| name.clone()).right()
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

parser! {
    fn atom[Input]()(Input) -> Expr
    where [Input: Stream<Token = char>]
    {
        choice((
            attempt(bool_literal()),
            attempt(int()),
            attempt(variable()),
            attempt(between(
                token('(').skip(spaces()),
                token(')'),
                expr().skip(spaces()),
            )),
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
            string("in").skip(spaces()),
            expr(),
        )
            .map(|(_, filepath, _, body)| {
                Expr::Load(filepath, Box::new(body))
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
            attempt(fun_expr()),
            attempt(atom()),
        ))
    }
}

parser! {
    fn app_expr[Input]()(Input) -> Expr
    where [Input: Stream<Token = char>]
    {
        (primary().skip(spaces()), many(primary().skip(spaces())))
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
            expr().skip(spaces())
        )
            .map(|(_, bindings, body): (_, Vec<(String, Expr)>, Expr)| {
                if bindings.is_empty() {
                    body
                } else {
                    Expr::Seq(bindings, Box::new(body))
                }
            })
    }
}

/// Parse a string into an expression
pub fn parse(input: &str) -> Result<Expr, String> {
    match program().easy_parse(input) {
        Ok((expr, rest)) => {
            if rest.is_empty() {
                Ok(expr)
            } else {
                Err(format!("Unexpected input after expression: '{}'", rest))
            }
        }
        Err(err) => Err(format!("Parse error: {}", err)),
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
}
