use crate::ast::*;
use crate::{tokenize, Span, Spanned, Token};

pub struct Parser<'a> {
    input: &'a str,
    tokens: Vec<Spanned<Token>>,
    pos: usize,
}

impl Parser<'_> {
    pub fn new(input: &str) -> Parser<'_> {
        let tokens = tokenize(input);
        Parser {
            input,
            tokens,
            pos: 0,
        }
    }

    fn peek(&self) -> Spanned<Token> {
        if let Some(token) = self.tokens.get(self.pos) {
            *token
        } else {
            let len = self.input.len() as u32;
            let span = Span::new(len, len + 1);
            Spanned::new(span, Token::Eof)
        }
    }

    fn next(&mut self) -> Spanned<Token> {
        let token = self.peek();
        self.pos += 1;
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
            Token::LParen => {
                self.next();
                let expr = self.expr();
                self.next();

                expr
            }
            Token::Int => self.expr_int(),
            Token::Float => self.expr_float(),
            _ => {
                self.next();
                Spanned::new(token.span, Expr::Error)
            }
        }
    }

    fn expr_int(&mut self) -> Spanned<Expr> {
        let span = self.next().span;
        let slice = span.slice(self.input);

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
        let slice = span.slice(self.input);
        let expr = slice.parse().map(Expr::Float).unwrap_or(Expr::Error);
        Spanned::new(span, expr)
    }
}
