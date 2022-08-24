use std::sync::Arc;

use super::*;
use crate::diagnostic::{Component, Diagnostic, Label, Severity, SourceComponent};
use crate::new_parser::{TextRange, TextSize};
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
            let range = if len > 0 {
                TextRange::new(TextSize::from(len - 1), TextSize::from(len))
            } else {
                TextRange::default()
            };

            Spanned::new(range, Token::Eof)
        }
    }

    fn next(&mut self) -> Spanned<Token> {
        let token = self.peek();
        self.pos += 1;
        token
    }

    fn error(&mut self, range: TextRange, message: String) {
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
                    range,
                    message,
                }],
            })],
        });
    }

    fn unexpected_token(&mut self, expected: &str) -> TextRange {
        let token = self.next();
        let range = token.range;
        let message = format!("expected {} near {}", expected, token.item.explain());

        self.error(range, message);

        range
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
                    Token::LBracket | Token::QuestionLBracket | Token::Dot | Token::QuestionDot => {
                        self.expr_index(lhs)
                    }
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
                let range = TextRange::new(lhs.range.start(), rhs.range.end());
                let expr = Expr::BinOp(BinOpExpr {
                    lhs: Box::new(lhs),
                    op,
                    rhs: Box::new(rhs),
                });

                lhs = Spanned::new(range, expr);
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
            let range = TextRange::new(token.range.start(), rhs.range.end());
            let expr = Expr::UnOp(UnOpExpr {
                op,
                expr: Box::new(rhs),
            });

            return Spanned::new(range, expr);
        }

        match token.item {
            Token::LParen => self.expr_paren(),
            Token::Int => self.expr_int(),
            Token::Float => self.expr_float(),
            Token::String => self.expr_string(),
            Token::Ident => self.expr_var(),
            Token::LBracket => self.expr_list(),
            Token::LBrace => self.expr_map(),
            Token::Fn => self.expr_fn(),
            Token::If => self.expr_if_else(),
            Token::Let => self.expr_let_in(),
            _ => {
                let range = self.unexpected_token("expression");
                Spanned::new(range, Expr::Error)
            }
        }
    }

    fn expr_paren(&mut self) -> Spanned<Expr> {
        let start = self.next().range.start();
        let expr = self.expr();
        let end = self.expect_token(&[Token::RParen]).range.end();

        let range = TextRange::new(start, end);
        let expr = Expr::Paren(Box::new(expr));
        Spanned::new(range, expr)
    }

    fn expr_int(&mut self) -> Spanned<Expr> {
        let range = self.next().range;
        let slice = &self.source.text[range];

        let expr = if let Some(stripped) = slice.strip_prefix("0x") {
            i64::from_str_radix(stripped, 16)
                .map(Expr::Int)
                .unwrap_or(Expr::Error)
        } else {
            slice.parse().map(Expr::Int).unwrap_or(Expr::Error)
        };

        Spanned::new(range, expr)
    }

    fn expr_float(&mut self) -> Spanned<Expr> {
        let range = self.next().range;
        let slice = &self.source.text[range];
        let expr = slice.parse().map(Expr::Float).unwrap_or(Expr::Error);
        Spanned::new(range, expr)
    }

    fn expr_string(&mut self) -> Spanned<Expr> {
        let range = self.next().range;
        let slice = &self.source.text[range];
        let str = slice[1..slice.len() - 1]
            .replace("\\\\", "\\")
            .replace("\\r", "\r")
            .replace("\\n", "\n")
            .replace("\\t", "\t");
        let expr = Expr::String(Arc::new(str));
        Spanned::new(range, expr)
    }

    fn expr_var(&mut self) -> Spanned<Expr> {
        let range = self.next().range;
        let slice = &self.source.text[range];
        let expr = Expr::Var(slice.into());
        Spanned::new(range, expr)
    }

    fn expr_list(&mut self) -> Spanned<Expr> {
        let start = self.next().range.start();

        let mut exprs = Vec::new();

        while self.peek().item != Token::RBracket {
            exprs.push(self.expr());

            if self.peek().item == Token::Comma {
                self.next();
            } else {
                break;
            }
        }

        let end = self.expect_token(&[Token::RBracket]).range.end();
        let range = TextRange::new(start, end);
        let expr = Expr::List(ListExpr { exprs });

        Spanned::new(range, expr)
    }

    fn expr_map(&mut self) -> Spanned<Expr> {
        let start = self.next().range.start();

        let mut pairs = Vec::new();

        while self.peek().item != Token::RBrace {
            let token = self.peek();
            let key = match token.item {
                Token::LBracket => {
                    self.next();
                    let key = self.expr();
                    self.expect_token(&[Token::RBracket]);
                    MapKey::Expr(key)
                }
                Token::String => MapKey::Expr(self.expr_string()),
                Token::Ident => {
                    self.next();
                    let text = self.source.text[token.range].to_string();
                    MapKey::Ident(Spanned::new(token.range, text))
                }
                _ => {
                    self.expect_token(&[Token::LBracket, Token::Ident, Token::String]);
                    MapKey::Expr(Spanned::new(token.range, Expr::Error))
                }
            };

            self.expect_token(&[Token::Assign]);
            let value = self.expr();

            pairs.push((key, value));

            if self.peek().item == Token::Comma {
                self.next();
            } else {
                break;
            }
        }

        let end = self.expect_token(&[Token::RBrace]).range.end();
        let range = TextRange::new(start, end);
        let expr = Expr::Map(MapExpr { pairs });

        Spanned::new(range, expr)
    }

    fn expr_fn(&mut self) -> Spanned<Expr> {
        let mut args = Vec::new();
        let start = self.next().range.start();

        self.expect_token(&[Token::LParen]);

        loop {
            let token = self.peek();

            match token.item {
                Token::RParen => break,
                Token::Ident => {
                    self.next();
                    args.push(self.source.text[token.range].into());
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
        let range = TextRange::new(start, inner.range.end());
        let expr = Expr::Func(FuncExpr {
            args,
            expr: Box::new(inner),
        });

        Spanned::new(range, expr)
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

        let end = self.expect_token(&[Token::RParen]).range.end();

        let range = TextRange::new(func.range.start(), end);
        let expr = Expr::Call(CallExpr {
            func: Box::new(func),
            args,
        });

        Spanned::new(range, expr)
    }

    fn expr_index(&mut self, lhs: Spanned<Expr>) -> Spanned<Expr> {
        let (op, short) = match self.next().item {
            Token::LBracket => (BinOp::Index, false),
            Token::QuestionLBracket => (BinOp::IndexNullable, false),
            Token::Dot => (BinOp::Index, true),
            Token::QuestionDot => (BinOp::IndexNullable, true),
            _ => unreachable!(),
        };

        let rhs = if short {
            let token = self.expect_token(&[Token::Ident]);
            let str = self.source.text[token.range].to_string();
            Spanned::new(token.range, Expr::String(Arc::new(str)))
        } else {
            self.expr()
        };

        let end = if short {
            rhs.range.end()
        } else {
            self.expect_token(&[Token::RBracket]).range.end()
        };

        let range = TextRange::new(lhs.range.start(), end);
        let expr = Expr::BinOp(BinOpExpr {
            lhs: Box::new(lhs),
            op,
            rhs: Box::new(rhs),
        });

        Spanned::new(range, expr)
    }

    fn expr_if_else(&mut self) -> Spanned<Expr> {
        let start = self.next().range.start();

        let cond = self.expr();
        self.expect_token(&[Token::Then]);
        let if_true = self.expr();
        self.expect_token(&[Token::Else]);
        let if_false = self.expr();

        let range = TextRange::new(start, if_false.range.end());
        let expr = Expr::IfElse(IfElseExpr {
            cond: Box::new(cond),
            if_true: Box::new(if_true),
            if_false: Box::new(if_false),
        });

        Spanned::new(range, expr)
    }

    fn expr_let_in(&mut self) -> Spanned<Expr> {
        let start = self.next().range.start();

        let mut vars = Vec::new();

        loop {
            let range = self.expect_token(&[Token::Ident]).range;
            let var = self.source.text[range].to_string();
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

        let range = TextRange::new(start, inner.range.end());
        let expr = Expr::LetIn(LetInExpr {
            vars,
            expr: Box::new(inner),
        });

        Spanned::new(range, expr)
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
        Or | Coalesce => (1, 2),
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
    use Token::*;

    Some(match token {
        LParen | LBracket | QuestionLBracket | Dot | QuestionDot => 17,
        _ => return None,
    })
}
