use crate::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

impl Keyword {
    pub fn lexeme(self) -> &'static str {
        match self {
            Keyword::Import => "import",
            Keyword::As => "as",
            Keyword::Extern => "extern",
            Keyword::Fun => "fun",
            Keyword::Async => "async",
            Keyword::Let => "let",
            Keyword::Var => "var",
            Keyword::Struct => "struct",
            Keyword::Enum => "enum",
            Keyword::Trait => "trait",
            Keyword::Impl => "impl",
            Keyword::Return => "return",
            Keyword::In => "in",
            Keyword::If => "if",
            Keyword::Else => "else",
            Keyword::While => "while",
            Keyword::For => "for",
            Keyword::Switch => "switch",
            Keyword::Try => "try",
            Keyword::Catch => "catch",
            Keyword::Unsafe => "unsafe",
            Keyword::Assembly => "assembly",
            Keyword::Slice => "slice",
            Keyword::Tuple => "tuple",
            Keyword::Mut => "mut",
            Keyword::Await => "await",
            Keyword::True => "true",
            Keyword::False => "false",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keyword {
    Import,
    As,
    Extern,
    Fun,
    Async,
    Let,
    Var,
    Struct,
    Enum,
    Trait,
    Impl,
    Return,
    In,
    If,
    Else,
    While,
    For,
    Switch,
    Try,
    Catch,
    Unsafe,
    Assembly,
    Slice,
    Tuple,
    Mut,
    Await,
    True,
    False,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Identifier(String),
    IntegerLiteral(String),
    FloatLiteral(String),
    StringLiteral(String),
    CharLiteral(char),
    Keyword(Keyword),
    LeftBrace,
    RightBrace,
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    Comma,
    Dot,
    DotDot,
    Semicolon,
    Colon,
    ColonColon,
    ThinArrow,
    Equals,
    EqualEqual,
    BangEqual,
    Ampersand,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Question,
    Bang,
    Pipe,
    AmpersandAmpersand,
    PipePipe,
    At,
    Eof,
}
