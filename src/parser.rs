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
        // Reject keywords
        if matches!(
            name.as_str(),
            "let" | "in" | "if" | "then" | "else" | "fun" | "true" | "false"
        ) {
            // Return a parser that fails for keywords 
            attempt(string("NEVER_MATCHES_ANYTHING_XYZ")).map(move |_| name.clone()).right()
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
    fn primary[Input]()(Input) -> Expr
    where [Input: Stream<Token = char>]
    {
        choice((
            attempt(let_expr()),
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
        spaces().with(expr()).skip(spaces())
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
}
