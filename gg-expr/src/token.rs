use logos::Logos;

use crate::{Span, Spanned};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Logos)]
#[logos(subpattern decimal = r"[0-9](?:_*[0-9])*")]
pub enum Token {
    #[token("+")]
    Add,
    #[token("-")]
    Sub,
    #[token("*")]
    Mul,
    #[token("/")]
    Div,
    #[token("**")]
    Pow,
    #[token("%")]
    Rem,
    #[token("&&")]
    And,
    #[token("||")]
    Or,
    #[token("!")]
    Not,

    #[token("=")]
    Assign,
    #[token("<")]
    Lt,
    #[token("<=")]
    Le,
    #[token("==")]
    Eq,
    #[token("!=")]
    Neq,
    #[token(">=")]
    Ge,
    #[token(">")]
    Gt,

    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,

    #[token(",")]
    Comma,
    #[token(":")]
    Colon,
    #[token(";")]
    Semicolon,

    #[token("true")]
    True,
    #[token("false")]
    False,
    #[token("let")]
    Let,
    #[token("in")]
    In,
    #[token("with")]
    With,
    #[token("if")]
    If,
    #[token("then")]
    Then,
    #[token("else")]
    Else,

    #[regex(r"(?&decimal)", priority = 2)]
    #[regex(r"0x[0-9a-fA-F](?:_*[0-9a-fA-F])*")]
    Int,

    #[regex(r"(?&decimal)(?:\.(?&decimal))?(?:_*[eE][+-]?(?&decimal))?")]
    Float,

    #[regex("#[0-9a-fA-F]+")]
    HexColor,

    #[regex(r#""(?:[^"]|\\")*""#)]
    String,

    #[regex(r"[_a-zA-Z][_0-9a-zA-Z]*")]
    Ident,

    #[error]
    #[regex(r"[ \t\n\f]+", logos::skip)]
    #[regex(r"//[^\n]*", logos::skip)]
    Error,

    Eof,
}

impl Token {
    pub fn explain(self) -> &'static str {
        use Token::*;

        match self {
            Add => "`+`",
            Sub => "`-`",
            Mul => "`*`",
            Div => "`/`",
            Pow => "`**`",
            Rem => "`%`",
            And => "`&&`",
            Or => "`||`",
            Not => "`!`",

            Assign => "`=`",
            Lt => "`<`",
            Le => "`<=`",
            Eq => "`==`",
            Neq => "`!=`",
            Ge => "`>=`",
            Gt => "`>`",

            LParen => "`(`",
            RParen => "`)`",
            LBrace => "`{`",
            RBrace => "`}`",
            LBracket => "`[`",
            RBracket => "`]`",

            Comma => "`,`",
            Colon => "`:`",
            Semicolon => "`;`",

            True => "`true`",
            False => "`false`",
            Let => "`let`",
            In => "`in`",
            With => "`with`",
            If => "`if`",
            Then => "`then`",
            Else => "`else`",
            Int => "integer",
            Float => "float",
            HexColor => "hex color",
            String => "string",
            Ident => "identifier",
            Error => "unexpected character",
            Eof => "EOF",
        }
    }
}

pub fn tokenize(input: &str) -> Vec<Spanned<Token>> {
    assert!(
        u32::try_from(input.len()).is_ok(),
        "input length must fit into u32"
    );

    let mut lexer = Token::lexer(input);
    let mut tokens = Vec::new();

    while let Some(token) = lexer.next() {
        let span = lexer.span();
        tokens.push(Spanned {
            span: Span::new(span.start as u32, span.end as u32),
            item: token,
        });
    }

    tokens
}
