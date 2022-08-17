use std::sync::Arc;

use super::*;
use crate::diagnostic::{Component, Diagnostic, Label, Severity, Source, SourceComponent};

pub struct Parser<'a> {
    source: &'a str,
    tokens: Vec<Spanned<Token>>,
    pos: usize,
    diagnostic_source: Arc<Source>,
    diagnostics: Vec<Diagnostic>,
}

impl Parser<'_> {
    pub fn new(source: &str) -> Parser<'_> {
        let tokens = tokenize(source);
        Parser {
            source,
            tokens,
            pos: 0,
            diagnostic_source: Arc::new(Source::new("unknown.expr", source)),
            diagnostics: Vec::new(),
        }
    }

    pub fn source(&self) -> &str {
        self.source
    }

    pub fn diagnostics(&mut self) -> impl Iterator<Item = Diagnostic> + '_ {
        self.diagnostics.drain(..)
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
        if !self.diagnostics.is_empty() {
            return;
        }

        self.diagnostics.push(Diagnostic {
            severity: Severity::Error,
            message: "syntax error".into(),
            components: vec![Component::Source(SourceComponent {
                source: self.diagnostic_source.clone(),
                labels: vec![Label {
                    severity: Severity::Error,
                    span,
                    message,
                }],
            })],
        });
    }

    fn unexpected_token(&mut self, expected: &str) -> Span {
        let token = self.next();
        let span = token.span;
        let message = format!("expected {} near {}", expected, token.item.explain());

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

        while let Some(op) = BinOp::from_token(self.peek().item) {
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
            Token::If => self.expr_if_else(),
            _ => {
                let span = self.unexpected_token("expression");
                Spanned::new(span, Expr::Error)
            }
        }
    }

    fn expr_paren(&mut self) -> Spanned<Expr> {
        self.next();
        let expr = self.expr();
        self.expect_token(&[Token::RParen]);
        expr
    }

    fn expr_int(&mut self) -> Spanned<Expr> {
        let span = self.next().span;
        let slice = span.slice(self.source);

        let expr = if let Some(stripped) = slice.strip_prefix("0x") {
            i32::from_str_radix(stripped, 16)
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
        self.expect_token(&[Token::Colon]);

        let inner = self.expr();
        let span = Span::new(start, inner.span.end);
        let expr = Expr::Func(FuncExpr {
            args,
            expr: Box::new(inner),
        });

        Spanned::new(span, expr)
    }

    fn expr_if_else(&mut self) -> Spanned<Expr> {
        let start = self.next().span.start;

        let cond = self.expr();
        self.expect_token(&[Token::Then]);
        let if_true = self.expr();
        self.expect_token(&[Token::Else]);
        let if_false = self.expr();

        let span = Span::new(start, if_false.span.end);
        let expr = Expr::IfElse(IfElseExpr {
            cond: Box::new(cond),
            if_true: Box::new(if_true),
            if_false: Box::new(if_false),
        });

        Spanned::new(span, expr)
    }
}
