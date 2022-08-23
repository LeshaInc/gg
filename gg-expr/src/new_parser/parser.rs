use std::iter::Peekable;

use rowan::{Checkpoint, GreenNode, GreenNodeBuilder};

use super::Lexer;
use super::SyntaxKind::{self, *};

pub struct Parser<'s> {
    lexer: Peekable<Lexer<'s>>,
    builder: GreenNodeBuilder<'static>,
    errors: Vec<String>,
}

impl Parser<'_> {
    pub fn new(source: &str) -> Parser<'_> {
        Parser {
            lexer: Lexer::new(source).peekable(),
            builder: GreenNodeBuilder::new(),
            errors: Vec::new(),
        }
    }

    pub fn finish(self) -> GreenNode {
        self.builder.finish()
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

    fn wrap(&mut self, token: SyntaxKind) {
        self.start_node(token);
        let tok = self.bump();
        self.finish_node();
        tok
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

    fn expect(&mut self, expected: SyntaxKind) {
        if self.peek() == Some(expected) {
            return self.bump();
        }

        self.start_error("unexpected token");
        self.bump();
        self.finish_node();
    }

    fn comma_separated(&mut self, end: SyntaxKind, mut func: impl FnMut(&mut Self)) {
        while self.peek() != Some(end) {
            func(self);

            if self.peek() == Some(TokComma) {
                self.bump();
            } else {
                break;
            }
        }
    }

    pub fn root(&mut self) {
        self.start_node(Root);
        self.expr();
        self.skip_trivia();
        self.finish_node();
    }

    pub fn expr(&mut self) {
        self.expr_bp(0)
    }

    fn expr_bp(&mut self, min_bp: u8) {
        let root = self.checkpoint();

        self.expr_lhs();

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
                self.wrap(BinaryOp);
                self.expr_bp(r_bp);
                self.finish_node();
                continue;
            }

            break;
        }
    }

    fn expr_lhs(&mut self) {
        if let Some(r_bp) = self.peek().and_then(prefix_bp) {
            self.start_node(ExprUnary);
            self.wrap(UnaryOp);
            self.expr_bp(r_bp);
            self.finish_node();
            return;
        }

        match self.peek() {
            Some(TokLParen) => self.expr_grouped(),
            Some(TokLBracket) => self.expr_list(),
            Some(TokLBrace) => self.expr_map(),
            Some(TokFn) => self.expr_fn(),
            Some(TokLet) => self.expr_let_in(),
            Some(TokInt) => self.expr_int(),
            Some(TokFloat) => self.expr_float(),
            Some(TokString) => self.expr_string(),
            Some(TokIdent) => self.expr_binding(),
            // v => todo!("{:?}", v),
            _ => {}
        }
    }

    fn expr_grouped(&mut self) {
        self.start_node(ExprGrouped);
        self.expect(TokLParen);
        self.expr();
        self.expect(TokRParen);
        self.finish_node();
    }

    fn expr_list(&mut self) {
        self.start_node(ExprList);
        self.expect(TokLBracket);
        self.comma_separated(TokRBracket, |s| s.expr());
        self.expect(TokRBracket);
        self.finish_node();
    }

    fn expr_map(&mut self) {
        self.start_node(ExprMap);
        self.expect(TokLBrace);

        self.comma_separated(TokRBrace, |s| {
            s.start_node(MapPair);

            match s.peek() {
                Some(TokIdent) => s.bump(),
                Some(TokString) => s.expr_string(),
                Some(TokLBracket) => {
                    s.bump();
                    s.expr();
                    s.expect(TokRBracket);
                }
                _ => todo!(),
            }

            s.expect(TokAssign);
            s.expr();

            s.finish_node();
        });

        self.expect(TokRBrace);
        self.finish_node();
    }

    fn expr_fn(&mut self) {
        self.start_node(ExprFn);
        self.expect(TokFn);

        self.start_node(FnArgs);
        self.expect(TokLParen);
        self.comma_separated(TokRParen, |s| s.pat());
        self.expect(TokRParen);
        self.finish_node();

        self.expect(TokColon);
        self.expr();
        self.finish_node();
    }

    fn expr_let_in(&mut self) {
        self.start_node(ExprLetIn);
        self.expect(TokLet);

        self.comma_separated(TokIn, |s| {
            s.start_node(LetBinding);
            s.pat();
            s.expect(TokAssign);
            s.expr();
            s.finish_node();
        });

        self.expect(TokIn);
        self.expr();
        self.finish_node();
    }

    fn expr_call(&mut self, root: Checkpoint) {
        self.start_node_at(root, ExprCall);
        self.start_node(CallArgs);
        self.expect(TokLParen);
        self.comma_separated(TokRParen, |s| s.expr());
        self.expect(TokRParen);
        self.finish_node();
        self.finish_node();
    }

    fn expr_index(&mut self, root: Checkpoint) {
        self.start_node_at(root, ExprIndex);

        let is_shorthand = match self.peek() {
            Some(TokLBracket | TokQuestionLBracket) => false,
            Some(TokDot | TokQuestionDot) => true,
            _ => todo!(),
        };

        self.bump();

        if is_shorthand {
            self.expect(TokIdent);
        } else {
            self.expr();
            self.expect(TokRBracket);
        }

        self.finish_node();
    }

    fn expr_int(&mut self) {
        self.start_node(ExprInt);
        self.expect(TokInt);
        self.finish_node();
    }

    fn expr_float(&mut self) {
        self.start_node(ExprFloat);
        self.expect(TokFloat);
        self.finish_node();
    }

    fn expr_string(&mut self) {
        self.start_node(ExprString);
        self.expect(TokString);
        self.finish_node();
    }

    fn expr_binding(&mut self) {
        self.start_node(ExprBinding);
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
            _ => todo!(),
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
        self.pat();
        self.expect(TokRParen);
        self.finish_node();
    }

    fn pat_list(&mut self) {
        self.start_node(PatList);
        self.expect(TokLBracket);
        self.comma_separated(TokRBracket, |s| s.pat());
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
        self.start_node(PatBinding);
        self.expect(TokHole);
        self.finish_node();
    }
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
