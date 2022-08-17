use std::collections::HashMap;

use miette::Diagnostic;
use thiserror::Error;

use super::ast::*;
use super::{tokenize, Span, Spanned, Token};

pub struct Parser<'a> {
    source: &'a str,
    tokens: Vec<Spanned<Token>>,
    pos: usize,
    errors: Vec<SyntaxError>,
    recovery_set: HashMap<Token, u32>,
    recovery_level: u32,
}

impl Parser<'_> {
    pub fn new(source: &str) -> Parser<'_> {
        let tokens = tokenize(source);
        Parser {
            source,
            tokens,
            pos: 0,
            errors: Vec::new(),
            recovery_set: HashMap::new(),
            recovery_level: 0,
        }
    }

    pub fn source(&self) -> &str {
        self.source
    }

    pub fn errors(&mut self) -> impl Iterator<Item = SyntaxError> + '_ {
        self.errors.drain(..)
    }

    fn peek(&self) -> Spanned<Token> {
        if let Some(token) = self.tokens.get(self.pos) {
            *token
        } else {
            let len = self.source.len() as u32;
            let span = if len > 0 {
                Span::new(len - 1, len)
            } else {
                Span::new(0, 0)
            };

            Spanned::new(span, Token::Eof)
        }
    }

    fn next(&mut self) -> Spanned<Token> {
        let token = self.peek();
        self.pos += 1;
        token
    }

    fn error(&mut self, span: Span, message: String) {
        self.errors.push(SyntaxError { span, message });
    }

    fn push_recovery(&mut self, tokens: &[Token]) {
        for &token in tokens {
            self.recovery_set
                .entry(token)
                .or_insert(self.recovery_level);
        }
        self.recovery_level += 1;
    }

    fn pop_recovery(&mut self) {
        self.recovery_level -= 1;
        let level = self.recovery_level;
        self.recovery_set.retain(|_, v| *v < level);
    }

    fn unexpected_token(&mut self, expected: &str) -> Span {
        let token = self.next();
        let mut span = token.span;
        let message = format!("expected {} near {}", expected, token.item.explain());

        loop {
            let token = self.peek();
            if token.item == Token::Eof || self.recovery_set.contains_key(&token.item) {
                break;
            } else {
                span.end = token.span.end;
                self.next();
            }
        }

        self.error(span, message);
        span
    }

    fn expect_token(&mut self, tokens: &[Token]) -> Spanned<Token> {
        let token = self.peek();

        if tokens.contains(&token.item) {
            return self.next();
        }

        let init = String::from(if tokens.len() > 1 { "one of " } else { "" });
        let msg = tokens.iter().fold(init, |mut acc, v| {
            acc.push_str(v.explain());
            acc.push_str(", ");
            acc
        });
        let msg = msg.trim_end_matches(", ");
        self.unexpected_token(msg);

        token
    }

    pub fn expr(&mut self) -> Spanned<Expr> {
        self.expr_bp(0)
    }

    fn expr_bp(&mut self, min_bp: u8) -> Spanned<Expr> {
        let mut lhs = self.expr_lhs();

        loop {
            let op = match BinOp::from_token(self.peek().item) {
                Some(v) => v,
                None => break,
            };

            let (l_bp, r_bp) = op.binding_power();
            if l_bp < min_bp {
                break;
            }

            self.next();

            let rhs = self.expr_bp(r_bp);
            let span = Span::new(lhs.span.start, rhs.span.end);
            let expr = Expr::BinOp(BinOpExpr {
                lhs: Box::new(lhs),
                op,
                rhs: Box::new(rhs),
            });

            lhs = Spanned::new(span, expr);
        }

        lhs
    }

    fn expr_lhs(&mut self) -> Spanned<Expr> {
        let token = self.peek();

        if let Some(op) = UnOp::from_token(token.item) {
            self.next();

            let rhs = self.expr_bp(op.binding_power());
            let span = Span::new(token.span.start, rhs.span.end);
            let expr = Expr::UnOp(UnOpExpr {
                op,
                expr: Box::new(rhs),
            });

            return Spanned::new(span, expr);
        }

        match token.item {
            Token::LParen => self.expr_paren(),
            Token::Int => self.expr_int(),
            Token::Float => self.expr_float(),
            Token::Ident => self.expr_var(),
            Token::Fn => self.expr_fn(),
            _ => {
                let span = self.unexpected_token("expression");
                Spanned::new(span, Expr::Error)
            }
        }
    }

    fn expr_paren(&mut self) -> Spanned<Expr> {
        self.next();
        self.push_recovery(&[Token::LParen]);
        let expr = self.expr();
        self.expect_token(&[Token::RParen]);
        self.pop_recovery();
        expr
    }

    fn expr_int(&mut self) -> Spanned<Expr> {
        let span = self.next().span;
        let slice = span.slice(self.source);

        let expr = if slice.starts_with("0x") {
            i32::from_str_radix(&slice[2..], 16)
                .map(Expr::Int)
                .unwrap_or(Expr::Error)
        } else {
            slice.parse().map(Expr::Int).unwrap_or(Expr::Error)
        };

        Spanned::new(span, expr)
    }

    fn expr_float(&mut self) -> Spanned<Expr> {
        let span = self.next().span;
        let slice = span.slice(self.source);
        let expr = slice.parse().map(Expr::Float).unwrap_or(Expr::Error);
        Spanned::new(span, expr)
    }

    fn expr_var(&mut self) -> Spanned<Expr> {
        let span = self.next().span;
        let expr = Expr::Var(span.slice(self.source).into());
        Spanned::new(span, expr)
    }

    fn expr_fn(&mut self) -> Spanned<Expr> {
        let mut args = Vec::new();
        let start = self.next().span.start;

        self.expect_token(&[Token::LParen]);
        self.push_recovery(&[Token::RParen, Token::Comma]);

        loop {
            let token = self.peek();

            match token.item {
                Token::RParen => break,
                Token::Ident => {
                    self.next();
                    args.push(token.span.slice(self.source).into());
                }
                _ => {
                    self.unexpected_token("function argument");
                    args.push("error".into());
                }
            }

            if self.peek().item == Token::Comma {
                self.next();
            } else {
                break;
            }
        }

        self.expect_token(&[Token::RParen]);
        self.pop_recovery();
        self.expect_token(&[Token::Colon]);

        let inner = self.expr();
        let span = Span::new(start, inner.span.end);
        let expr = Expr::Fn(FnExpr {
            args,
            expr: Box::new(inner),
        });

        Spanned::new(span, expr)
    }
}

#[derive(Debug, Error, Diagnostic)]
#[error("syntax error")]
#[diagnostic()]
pub struct SyntaxError {
    message: String,
    #[label("{}", message)]
    span: Span,
}
