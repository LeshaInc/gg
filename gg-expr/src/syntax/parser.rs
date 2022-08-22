use std::sync::Arc;

use super::*;
use crate::diagnostic::{Component, Diagnostic, Label, Severity, SourceComponent};
use crate::Source;

pub struct Parser {
    source: Arc<Source>,
    tokens: Vec<Spanned<Token>>,
    pos: usize,
    diagnostics: Vec<Diagnostic>,
}

impl Parser {
    pub fn new(source: Arc<Source>) -> Parser {
        let tokens = tokenize(&source.text);
        Parser {
            source,
            tokens,
            pos: 0,
            diagnostics: Vec::new(),
        }
    }

    pub fn diagnostics(&mut self) -> impl Iterator<Item = Diagnostic> + '_ {
        self.diagnostics.drain(..)
    }

    fn peek(&self) -> Spanned<Token> {
        if let Some(token) = self.tokens.get(self.pos) {
            *token
        } else {
            let len = self.source.text.len() as u32;
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
                source: self.source.clone(),
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

        loop {
            let token = self.peek().item;

            if let Some(l_bp) = postfix_bp(token) {
                if l_bp < min_bp {
                    break;
                }

                lhs = match token {
                    Token::LParen => self.expr_call(lhs),
                    Token::LBracket | Token::QuestionLBracket => self.expr_index(lhs),
                    _ => continue,
                };
                continue;
            }

            if let Some((l_bp, r_bp)) = infix_bp(token) {
                if l_bp < min_bp {
                    break;
                }

                let op = BinOp::from_token(token).unwrap();

                self.next();

                let rhs = self.expr_bp(r_bp);
                let span = Span::new(lhs.span.start, rhs.span.end);
                let expr = Expr::BinOp(BinOpExpr {
                    lhs: Box::new(lhs),
                    op,
                    rhs: Box::new(rhs),
                });

                lhs = Spanned::new(span, expr);
                continue;
            }

            break;
        }

        lhs
    }

    fn expr_lhs(&mut self) -> Spanned<Expr> {
        let token = self.peek();

        if let Some(r_bp) = prefix_bp(token.item) {
            let op = UnOp::from_token(token.item).unwrap();
            self.next();

            let rhs = self.expr_bp(r_bp);
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
            Token::String => self.expr_string(),
            Token::Ident => self.expr_var(),
            Token::LBracket => self.expr_list(),
            Token::Fn => self.expr_fn(),
            Token::If => self.expr_if_else(),
            Token::Let => self.expr_let_in(),
            _ => {
                let span = self.unexpected_token("expression");
                Spanned::new(span, Expr::Error)
            }
        }
    }

    fn expr_paren(&mut self) -> Spanned<Expr> {
        let start = self.next().span.start;
        let expr = self.expr();
        let end = self.expect_token(&[Token::RParen]).span.end;

        let span = Span::new(start, end);
        let expr = Expr::Paren(Box::new(expr));
        Spanned::new(span, expr)
    }

    fn expr_int(&mut self) -> Spanned<Expr> {
        let span = self.next().span;
        let slice = span.slice(&self.source.text);

        let expr = if let Some(stripped) = slice.strip_prefix("0x") {
            i64::from_str_radix(stripped, 16)
                .map(Expr::Int)
                .unwrap_or(Expr::Error)
        } else {
            slice.parse().map(Expr::Int).unwrap_or(Expr::Error)
        };

        Spanned::new(span, expr)
    }

    fn expr_float(&mut self) -> Spanned<Expr> {
        let span = self.next().span;
        let slice = span.slice(&self.source.text);
        let expr = slice.parse().map(Expr::Float).unwrap_or(Expr::Error);
        Spanned::new(span, expr)
    }

    fn expr_string(&mut self) -> Spanned<Expr> {
        let span = self.next().span;
        let slice = span.slice(&self.source.text);
        let str = slice[1..slice.len() - 1]
            .replace("\\\\", "\\")
            .replace("\\r", "\r")
            .replace("\\n", "\n")
            .replace("\\t", "\t");
        let expr = Expr::String(Arc::new(str));
        Spanned::new(span, expr)
    }

    fn expr_var(&mut self) -> Spanned<Expr> {
        let span = self.next().span;
        let expr = Expr::Var(span.slice(&self.source.text).into());
        Spanned::new(span, expr)
    }

    fn expr_list(&mut self) -> Spanned<Expr> {
        let start = self.next().span.start;

        let mut exprs = Vec::new();

        while self.peek().item != Token::RBracket {
            exprs.push(self.expr());

            if self.peek().item == Token::Comma {
                self.next();
            } else {
                break;
            }
        }

        let end = self.expect_token(&[Token::RBracket]).span.end;
        let span = Span::new(start, end);
        let expr = Expr::List(ListExpr { exprs });

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
                    args.push(token.span.slice(&self.source.text).into());
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

    fn expr_call(&mut self, func: Spanned<Expr>) -> Spanned<Expr> {
        self.next();

        let mut args = Vec::new();

        while self.peek().item != Token::RParen {
            args.push(self.expr());

            if self.peek().item == Token::Comma {
                self.next();
            } else {
                break;
            }
        }

        let end = self.expect_token(&[Token::RParen]).span.end;

        let span = Span::new(func.span.start, end);
        let expr = Expr::Call(CallExpr {
            func: Box::new(func),
            args,
        });

        Spanned::new(span, expr)
    }

    fn expr_index(&mut self, lhs: Spanned<Expr>) -> Spanned<Expr> {
        let op = match self.next().item {
            Token::LBracket => BinOp::Index,
            Token::QuestionLBracket => BinOp::IndexNullable,
            _ => unreachable!(),
        };

        let rhs = self.expr();
        let end = self.expect_token(&[Token::RBracket]).span.end;

        let span = Span::new(lhs.span.start, end);
        let expr = Expr::BinOp(BinOpExpr {
            lhs: Box::new(lhs),
            op,
            rhs: Box::new(rhs),
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

    fn expr_let_in(&mut self) -> Spanned<Expr> {
        let start = self.next().span.start;

        let mut vars = Vec::new();

        loop {
            let span = self.expect_token(&[Token::Ident]).span;
            let var = span.slice(&self.source.text).to_owned();
            self.expect_token(&[Token::Assign]);
            let expr = self.expr();
            vars.push((var, Box::new(expr)));

            if self.peek().item == Token::Comma {
                self.next();
            } else {
                break;
            }
        }

        self.expect_token(&[Token::In]);
        let inner = self.expr();

        let span = Span::new(start, inner.span.end);
        let expr = Expr::LetIn(LetInExpr {
            vars,
            expr: Box::new(inner),
        });

        Spanned::new(span, expr)
    }
}

pub fn prefix_bp(token: Token) -> Option<u8> {
    Some(match token {
        Token::Sub | Token::Not => 14,
        _ => return None,
    })
}

pub fn infix_bp(token: Token) -> Option<(u8, u8)> {
    use Token::*;

    Some(match token {
        Or => (1, 2),
        And => (3, 4),
        Eq | Neq => (5, 6),
        Lt | Le | Ge | Gt => (7, 8),
        Add | Sub => (9, 10),
        Mul | Div | Rem => (11, 12),
        Pow => (15, 16),
        _ => return None,
    })
}

pub fn postfix_bp(token: Token) -> Option<u8> {
    Some(match token {
        Token::LParen | Token::LBracket | Token::QuestionLBracket => 17,
        _ => return None,
    })
}
