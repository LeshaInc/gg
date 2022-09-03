#![allow(dead_code)]

use std::collections::HashMap;
use std::iter::Peekable;
use std::sync::Arc;

use rowan::{Checkpoint, GreenNodeBuilder, TextRange, TextSize};

use super::SyntaxKind::{self, *};
use super::{Expr, Lexer, SyntaxNode};
use crate::diagnostic::{Diagnostic, Severity, SourceComponent};
use crate::Source;

pub struct Parser<'s> {
    lexer: Peekable<Lexer<'s>>,
    builder: GreenNodeBuilder<'static>,
    recovery_set: HashMap<SyntaxKind, u32>,
    source: Arc<Source>,
    errors: Vec<String>,
}

impl Parser<'_> {
    pub fn new(source: &str) -> Parser<'_> {
        Parser {
            lexer: Lexer::new(source).peekable(),
            builder: GreenNodeBuilder::new(),
            recovery_set: HashMap::default(),
            source: Arc::new(Source::new("unknown.expr".into(), source.into())),
            errors: Vec::new(),
        }
    }

    pub fn finish(self) -> ParseResult {
        let node = SyntaxNode::new_root(self.builder.finish());

        let error_ranges = node.descendants().flat_map(|node| {
            if node.kind() == SyntaxKind::Error {
                Some(node.text_range())
            } else {
                None
            }
        });

        let errors = self
            .errors
            .into_iter()
            .zip(error_ranges)
            .map(|(error, mut range)| {
                let one = TextSize::from(1);

                if range.is_empty() {
                    range = TextRange::at(range.start(), one);
                }
                if range.start() == node.text_range().end() && u32::from(range.start()) > 0 {
                    range -= one;
                }

                Diagnostic::new(Severity::Error, "syntax error").with_source(
                    SourceComponent::new(self.source.clone()).with_label(
                        Severity::Error,
                        range,
                        error,
                    ),
                )
            })
            .collect();

        ParseResult {
            expr: node.first_child().and_then(Expr::cast),
            node,
            errors,
        }
    }

    fn skip_trivia(&mut self) {
        while let Some(&(text, token)) = self.lexer.peek() {
            if token.is_trivia() {
                self.builder.token(token.into(), text);
                self.lexer.next();
            } else {
                break;
            }
        }
    }

    fn peek(&mut self) -> Option<SyntaxKind> {
        self.skip_trivia();
        self.lexer.peek().map(|&(_, token)| token)
    }

    fn bump(&mut self) {
        if let Some((text, token)) = self.lexer.next() {
            self.builder.token(token.into(), text);
        }
    }

    fn checkpoint(&self) -> Checkpoint {
        self.builder.checkpoint()
    }

    fn start_node(&mut self, kind: SyntaxKind) {
        self.builder.start_node(kind.into());
    }

    fn start_node_at(&mut self, checkpoint: Checkpoint, kind: SyntaxKind) {
        self.builder.start_node_at(checkpoint, kind.into());
    }

    fn start_error(&mut self, message: impl Into<String>) {
        self.errors.push(message.into());
        self.start_node(Error);
    }

    fn finish_node(&mut self) {
        self.builder.finish_node();
    }

    fn push_recovery(&mut self, tokens: &[SyntaxKind]) {
        for &token in tokens {
            *self.recovery_set.entry(token).or_default() += 1;
        }
    }

    fn pop_recovery(&mut self) {
        self.recovery_set.retain(|_, v| {
            *v -= 1;
            *v > 0
        });
    }

    fn error_recover(&mut self) {
        while let Some(token) = self.peek() {
            if self.recovery_set.contains_key(&token) {
                break;
            } else {
                self.bump();
            }
        }
    }

    fn error_unexpected_token(&mut self, expected: &str) {
        let found = self
            .peek()
            .map(SyntaxKind::explain)
            .unwrap_or("end of file");

        let mut message = format!("expected {}", expected);

        message.push_str(", found ");
        message.push_str(found);

        self.start_error(message);
        self.error_recover();
        self.finish_node();
    }

    fn expect_one_of(&mut self, expected: &[SyntaxKind]) {
        if let Some(token) = self.peek() {
            if expected.contains(&token) {
                return self.bump();
            }
        }

        let mut message = String::new();

        if expected.len() > 1 {
            message.push_str("one of ");
        }

        for (i, token) in expected.iter().enumerate() {
            if i > 0 {
                message.push_str(", ");
            }

            message.push_str(token.explain())
        }

        self.error_unexpected_token(&message);
    }

    fn expect(&mut self, expected: SyntaxKind) {
        self.expect_one_of(&[expected]);
    }

    fn comma_separated(&mut self, end: SyntaxKind, mut func: impl FnMut(&mut Self)) {
        self.push_recovery(&[TokComma, end]);

        while self.peek() != Some(end) {
            func(self);

            if self.peek() == Some(TokComma) {
                self.bump();
            } else {
                break;
            }
        }

        self.pop_recovery();
    }

    pub fn root(&mut self) {
        self.start_node(Root);

        self.expr();

        if self.peek().is_some() {
            self.start_error("trailing characters");
            while self.peek().is_some() {
                self.bump();
            }
            self.finish_node();
        }

        self.finish_node();
    }

    pub fn expr(&mut self) {
        self.expr_bp(0)
    }

    fn expr_bp(&mut self, min_bp: u8) {
        let root = self.checkpoint();

        self.expr_lhs(root);

        while let Some(token) = self.peek() {
            if let Some(l_bp) = postfix_bp(token) {
                if l_bp < min_bp {
                    break;
                }

                match token {
                    TokLParen => self.expr_call(root),
                    TokLBracket | TokQuestionLBracket | TokDot | TokQuestionDot => {
                        self.expr_index(root)
                    }
                    _ => unreachable!(),
                }

                continue;
            }

            if let Some((l_bp, r_bp)) = infix_bp(token) {
                if l_bp < min_bp {
                    break;
                }

                self.start_node_at(root, ExprBinary);
                self.bump();
                self.expr_bp(r_bp);
                self.finish_node();
                continue;
            }

            break;
        }
    }

    fn expr_lhs(&mut self, root: Checkpoint) {
        if let Some(r_bp) = self.peek().and_then(prefix_bp) {
            self.start_node(ExprUnary);
            self.bump();
            self.expr_bp(r_bp);
            self.finish_node();
            return;
        }

        match self.peek() {
            Some(TokLParen) => self.expr_grouped(root),
            Some(TokLBracket) => self.expr_list(root),
            Some(TokLBrace) => self.expr_map(root),
            Some(TokFn) => self.expr_fn(root),
            Some(TokLet) => self.expr_let_in(root),
            Some(TokIf) => self.expr_if_else(root),
            Some(TokWhen) => self.expr_when(root),
            Some(TokNull) => self.expr_null(root),
            Some(TokTrue | TokFalse) => self.expr_bool(root),
            Some(TokInt) => self.expr_int(root),
            Some(TokFloat) => self.expr_float(root),
            Some(TokString) => self.expr_string(root),
            Some(TokIdent) => self.expr_binding(root),
            _ => self.error_unexpected_token("expression"),
        }
    }

    fn expr_grouped(&mut self, root: Checkpoint) {
        self.start_node_at(root, ExprGrouped);
        self.expect(TokLParen);
        self.push_recovery(&[TokRParen]);
        self.expr();
        self.pop_recovery();
        self.expect(TokRParen);
        self.finish_node();
    }

    fn expr_list(&mut self, root: Checkpoint) {
        self.start_node_at(root, ExprList);
        self.expect(TokLBracket);
        self.comma_separated(TokRBracket, |s| s.expr());
        self.expect(TokRBracket);
        self.finish_node();
    }

    fn expr_map(&mut self, root: Checkpoint) {
        self.start_node_at(root, ExprMap);
        self.expect(TokLBrace);

        self.comma_separated(TokRBrace, |s| {
            s.start_node(MapPair);
            s.push_recovery(&[TokAssign]);

            match s.peek() {
                Some(TokIdent) => s.bump(),
                Some(TokString) => s.expr_string(s.checkpoint()),
                Some(TokLBracket) => {
                    s.bump();
                    s.push_recovery(&[TokRBracket]);
                    s.expr();
                    s.pop_recovery();
                    s.expect(TokRBracket);
                }
                _ => s.error_unexpected_token("map key"),
            }

            s.pop_recovery();
            s.expect(TokAssign);
            s.expr();

            s.finish_node();
        });

        self.expect(TokRBrace);
        self.finish_node();
    }

    fn expr_fn(&mut self, root: Checkpoint) {
        self.start_node_at(root, ExprFn);
        self.expect(TokFn);
        self.push_recovery(&[TokColon]);

        self.expect(TokLParen);
        self.comma_separated(TokRParen, |s| s.expect(TokIdent));
        self.expect(TokRParen);

        self.pop_recovery();
        self.expect(TokColon);
        self.expr();
        self.finish_node();
    }

    fn expr_let_in(&mut self, root: Checkpoint) {
        self.start_node_at(root, ExprLetIn);
        self.expect(TokLet);

        self.comma_separated(TokIn, |s| {
            s.start_node(LetBinding);
            s.expect(TokIdent);
            s.expect(TokAssign);
            s.expr();
            s.finish_node();
        });

        self.expect(TokIn);
        self.expr();
        self.finish_node();
    }

    fn expr_if_else(&mut self, root: Checkpoint) {
        self.start_node_at(root, ExprIfElse);
        self.expect(TokIf);
        self.push_recovery(&[TokThen]);
        self.expr();
        self.pop_recovery();
        self.expect(TokThen);
        self.push_recovery(&[TokElse]);
        self.expr();
        self.pop_recovery();
        self.expect(TokElse);
        self.expr();
        self.finish_node();
    }

    fn expr_when(&mut self, root: Checkpoint) {
        self.start_node_at(root, ExprWhen);
        self.expect(TokWhen);
        self.push_recovery(&[TokIs]);
        self.expr();
        self.pop_recovery();
        self.expect(TokIs);

        loop {
            self.start_node(WhenCase);
            self.push_recovery(&[TokArrow]);
            self.pat();
            self.pop_recovery();
            self.expect(TokArrow);
            self.expr();
            self.finish_node();

            match self.peek() {
                Some(TokComma) => {
                    self.bump();
                    continue;
                }
                _ => break,
            }
        }

        self.finish_node();
    }

    fn expr_call(&mut self, root: Checkpoint) {
        self.start_node_at(root, ExprCall);
        self.expect(TokLParen);
        self.comma_separated(TokRParen, |s| s.expr());
        self.expect(TokRParen);
        self.finish_node();
    }

    fn expr_index(&mut self, root: Checkpoint) {
        self.start_node_at(root, ExprIndex);

        let is_shorthand = match self.peek() {
            Some(TokLBracket | TokQuestionLBracket) => false,
            Some(TokDot | TokQuestionDot) => true,
            _ => {
                self.error_unexpected_token("indexing operator");
                false
            }
        };

        self.bump();

        if is_shorthand {
            self.expect(TokIdent);
        } else {
            self.push_recovery(&[TokRBracket]);
            self.expr();
            self.pop_recovery();
            self.expect(TokRBracket);
        }

        self.finish_node();
    }

    fn expr_null(&mut self, root: Checkpoint) {
        self.start_node_at(root, ExprNull);
        self.expect(TokNull);
        self.finish_node();
    }

    fn expr_bool(&mut self, root: Checkpoint) {
        self.start_node_at(root, ExprBool);
        self.expect_one_of(&[TokTrue, TokFalse]);
        self.finish_node();
    }

    fn expr_int(&mut self, root: Checkpoint) {
        self.start_node_at(root, ExprInt);
        self.expect(TokInt);
        self.finish_node();
    }

    fn expr_float(&mut self, root: Checkpoint) {
        self.start_node_at(root, ExprFloat);
        self.expect(TokFloat);
        self.finish_node();
    }

    fn expr_string(&mut self, root: Checkpoint) {
        self.start_node_at(root, ExprString);
        self.expect(TokString);
        self.finish_node();
    }

    fn expr_binding(&mut self, root: Checkpoint) {
        self.start_node_at(root, ExprBinding);
        self.expect(TokIdent);
        self.finish_node();
    }

    fn pat(&mut self) {
        let root = self.checkpoint();
        self.pat_atom();

        while self.peek() == Some(TokPipe) {
            self.start_node_at(root, PatOr);
            self.bump();
            self.pat_atom();
            self.finish_node();
        }
    }

    fn pat_atom(&mut self) {
        let root = self.checkpoint();

        match self.peek() {
            Some(TokLParen) => self.pat_grouped(),
            Some(TokLBracket) => self.pat_list(),
            Some(TokRest) => self.pat_rest(),
            Some(TokInt) => self.pat_int(),
            Some(TokString) => self.pat_string(),
            Some(TokIdent) => self.pat_binding(),
            Some(TokHole) => self.pat_hole(),
            _ => self.error_unexpected_token("pattern"),
        }

        if self.peek() == Some(TokAs) {
            self.start_node_at(root, PatBinding);
            self.bump();
            self.expect(TokIdent);
            self.finish_node();
        }
    }

    fn pat_grouped(&mut self) {
        self.start_node(PatGrouped);
        self.expect(TokLParen);
        self.push_recovery(&[TokRParen]);
        self.pat();
        self.pop_recovery();
        self.expect(TokRParen);
        self.finish_node();
    }

    fn pat_list(&mut self) {
        self.start_node(PatList);
        self.expect(TokLBracket);
        self.push_recovery(&[TokRBracket]);
        self.comma_separated(TokRBracket, |s| s.pat());
        self.pop_recovery();
        self.expect(TokRBracket);
        self.finish_node();
    }

    fn pat_rest(&mut self) {
        self.start_node(PatRest);
        self.expect(TokRest);
        self.finish_node();
    }

    fn pat_int(&mut self) {
        self.start_node(PatInt);
        self.expect(TokInt);
        self.finish_node();
    }

    fn pat_string(&mut self) {
        self.start_node(PatString);
        self.expect(TokString);
        self.finish_node();
    }

    fn pat_binding(&mut self) {
        self.start_node(PatBinding);
        self.expect(TokIdent);
        self.finish_node();
    }

    fn pat_hole(&mut self) {
        self.start_node(PatHole);
        self.expect(TokHole);
        self.finish_node();
    }
}

pub struct ParseResult {
    pub node: SyntaxNode,
    pub expr: Option<Expr>,
    pub errors: Vec<Diagnostic>,
}

fn prefix_bp(token: SyntaxKind) -> Option<u8> {
    Some(match token {
        TokSub | TokNot => 14,
        _ => return None,
    })
}

fn infix_bp(token: SyntaxKind) -> Option<(u8, u8)> {
    Some(match token {
        TokOr | TokCoalesce => (1, 2),
        TokAnd => (3, 4),
        TokEq | TokNeq => (5, 6),
        TokLt | TokLe | TokGe | TokGt => (7, 8),
        TokAdd | TokSub => (9, 10),
        TokMul | TokDiv | TokRem => (11, 12),
        TokPow => (15, 16),
        _ => return None,
    })
}

fn postfix_bp(token: SyntaxKind) -> Option<u8> {
    Some(match token {
        TokLParen | TokLBracket | TokQuestionLBracket | TokDot | TokQuestionDot => 17,
        _ => return None,
    })
}

pub fn int_value(text: &str) -> Option<i64> {
    text.parse().ok()
}

pub fn float_value(text: &str) -> Option<f64> {
    text.parse().ok()
}

pub fn string_value(text: &str) -> String {
    text[1..text.len() - 1]
        .replace("\\\\", "\\")
        .replace("\\r", "\r")
        .replace("\\n", "\n")
        .replace("\\t", "\t")
}
