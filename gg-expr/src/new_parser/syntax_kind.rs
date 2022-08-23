use logos::Logos;
use rowan::Language;

#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Logos)]
#[logos(subpattern decimal = r"[0-9](?:_*[0-9])*")]
pub enum SyntaxKind {
    #[regex(r"[ \t\n\f]+")]
    TokWhitespace,
    #[regex(r"//[^\n]*", logos::skip)]
    TokComment,
    #[token("+")]
    TokAdd,
    #[token("-")]
    TokSub,
    #[token("*")]
    TokMul,
    #[token("/")]
    TokDiv,
    #[token("**")]
    TokPow,
    #[token("%")]
    TokRem,
    #[token("&&")]
    TokAnd,
    #[token("||")]
    TokOr,
    #[token("|")]
    TokPipe,
    #[token("??")]
    TokCoalesce,
    #[token("!")]
    TokNot,
    #[token("=")]
    TokAssign,
    #[token("<")]
    TokLt,
    #[token("<=")]
    TokLe,
    #[token("==")]
    TokEq,
    #[token("!=")]
    TokNeq,
    #[token(">=")]
    TokGe,
    #[token(">")]
    TokGt,
    #[token("(")]
    TokLParen,
    #[token(")")]
    TokRParen,
    #[token("{")]
    TokLBrace,
    #[token("}")]
    TokRBrace,
    #[token("[")]
    TokLBracket,
    #[token("]")]
    TokRBracket,
    #[token("?[")]
    TokQuestionLBracket,
    #[token(".")]
    TokDot,
    #[token("?.")]
    TokQuestionDot,
    #[token(",")]
    TokComma,
    #[token(":")]
    TokColon,
    #[token("...")]
    TokRest,
    #[token("_")]
    TokHole,
    #[token("true")]
    TokTrue,
    #[token("false")]
    TokFalse,
    #[token("let")]
    TokLet,
    #[token("in")]
    TokIn,
    #[token("as")]
    TokAs,
    #[token("if")]
    TokIf,
    #[token("then")]
    TokThen,
    #[token("else")]
    TokElse,
    #[token("fn")]
    TokFn,
    #[regex(r"(?&decimal)", priority = 2)]
    #[regex(r"0x[0-9a-fA-F](?:_*[0-9a-fA-F])*")]
    TokInt,
    #[regex(r"(?&decimal)(?:\.(?&decimal))?(?:_*[eE][+-]?(?&decimal))?")]
    TokFloat,
    #[regex(r#""(?:[^"]|\\")*""#)]
    TokString,
    #[regex(r"[_a-zA-Z][_0-9a-zA-Z]*")]
    TokIdent,

    Root,

    ExprInt,
    ExprFloat,
    ExprString,
    ExprBinding,
    ExprBinary,
    ExprUnary,
    ExprGrouped,
    ExprList,
    ExprMap,
    ExprFn,
    ExprCall,
    ExprIndex,
    ExprIfElse,
    ExprLetIn,

    PatGrouped,
    PatOr,
    PatList,
    PatInt,
    PatString,
    PatRest,
    PatHole,
    PatBinding,

    MapPair,
    LetBinding,
    FnArgs,
    CallArgs,
    BinaryOp,
    UnaryOp,

    #[error]
    Error,
    Eof,
}

impl SyntaxKind {
    pub fn is_trivia(&self) -> bool {
        matches!(self, SyntaxKind::TokWhitespace | SyntaxKind::TokComment)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ExprLang;

impl Language for ExprLang {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        assert!(raw.0 < SyntaxKind::Eof as u16);
        unsafe { std::mem::transmute(raw.0) }
    }

    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        rowan::SyntaxKind(kind as u16)
    }
}

impl From<SyntaxKind> for rowan::SyntaxKind {
    fn from(v: SyntaxKind) -> Self {
        Self(v as u16)
    }
}
