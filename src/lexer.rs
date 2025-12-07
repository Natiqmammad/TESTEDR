use crate::diagnostics::LexError;
use crate::span::Span;
use crate::token::{Keyword, Token, TokenKind};

pub fn lex(source: &str) -> Result<Vec<Token>, LexError> {
    Lexer::new(source).lex()
}

struct Lexer<'a> {
    source: &'a str,
    index: usize,
    line: usize,
    column: usize,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            index: 0,
            line: 1,
            column: 1,
        }
    }

    fn lex(mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();
        self.skip_bom();
        loop {
            self.skip_trivia()?;
            let ch = match self.peek_char() {
                Some(ch) => ch,
                None => break,
            };
            let token = if is_ident_start(ch) {
                self.lex_identifier()?
            } else if ch.is_ascii_digit() {
                self.lex_number()?
            } else {
                match ch {
                    '"' => self.lex_string()?,
                    '\'' => self.lex_char_literal()?,
                    '{' => self.simple_token(TokenKind::LeftBrace),
                    '}' => self.simple_token(TokenKind::RightBrace),
                    '(' => self.simple_token(TokenKind::LeftParen),
                    ')' => self.simple_token(TokenKind::RightParen),
                    '[' => self.simple_token(TokenKind::LeftBracket),
                    ']' => self.simple_token(TokenKind::RightBracket),
                    ',' => self.simple_token(TokenKind::Comma),
                    '.' => {
                        if self.peek_second_char() == Some('.') {
                            self.multi_char_token(2, TokenKind::DotDot)
                        } else {
                            self.simple_token(TokenKind::Dot)
                        }
                    }
                    ';' => self.simple_token(TokenKind::Semicolon),
                    ':' => {
                        if self.peek_second_char() == Some(':') {
                            self.multi_char_token(2, TokenKind::ColonColon)
                        } else {
                            self.simple_token(TokenKind::Colon)
                        }
                    }
                    '-' => {
                        if self.peek_second_char() == Some('>') {
                            self.multi_char_token(2, TokenKind::ThinArrow)
                        } else {
                            self.simple_token(TokenKind::Minus)
                        }
                    }
                    '=' => {
                        if self.peek_second_char() == Some('=') {
                            self.multi_char_token(2, TokenKind::EqualEqual)
                        } else {
                            self.simple_token(TokenKind::Equals)
                        }
                    }
                    '&' => {
                        if self.peek_second_char() == Some('&') {
                            self.multi_char_token(2, TokenKind::AmpersandAmpersand)
                        } else {
                            self.simple_token(TokenKind::Ampersand)
                        }
                    }
                    '|' => {
                        if self.peek_second_char() == Some('|') {
                            self.multi_char_token(2, TokenKind::PipePipe)
                        } else {
                            self.simple_token(TokenKind::Pipe)
                        }
                    }
                    '+' => self.simple_token(TokenKind::Plus),
                    '*' => self.simple_token(TokenKind::Star),
                    '/' => self.simple_token(TokenKind::Slash),
                    '%' => self.simple_token(TokenKind::Percent),
                    '?' => self.simple_token(TokenKind::Question),
                    '!' => {
                        if self.peek_second_char() == Some('=') {
                            self.multi_char_token(2, TokenKind::BangEqual)
                        } else {
                            self.simple_token(TokenKind::Bang)
                        }
                    }
                    '<' => {
                        if self.peek_second_char() == Some('=') {
                            self.multi_char_token(2, TokenKind::LessEqual)
                        } else {
                            self.simple_token(TokenKind::Less)
                        }
                    }
                    '>' => {
                        if self.peek_second_char() == Some('=') {
                            self.multi_char_token(2, TokenKind::GreaterEqual)
                        } else {
                            self.simple_token(TokenKind::Greater)
                        }
                    }
                    '@' => self.simple_token(TokenKind::At),
                    other => {
                        let span = Span::new(
                            self.index,
                            self.index + other.len_utf8(),
                            self.line,
                            self.column,
                        );
                        return Err(LexError::UnexpectedChar { ch: other, span });
                    }
                }
            };
            tokens.push(token);
        }
        tokens.push(Token::new(
            TokenKind::Eof,
            Span::new(self.index, self.index, self.line, self.column),
        ));
        Ok(tokens)
    }

    fn lex_identifier(&mut self) -> Result<Token, LexError> {
        let start_index = self.index;
        let start_line = self.line;
        let start_col = self.column;
        let mut ident = String::new();
        while let Some(ch) = self.peek_char() {
            if is_ident_char(ch) {
                if !ch.is_ascii() {
                    let span = Span::new(
                        self.index,
                        self.index + ch.len_utf8(),
                        self.line,
                        self.column,
                    );
                    return Err(LexError::NonAsciiIdentifierChar { ch, span });
                }
                ident.push(ch);
                self.advance_char();
            } else {
                break;
            }
        }
        let span = Span::new(start_index, self.index, start_line, start_col);
        let kind = keyword(&ident)
            .map(TokenKind::Keyword)
            .unwrap_or(TokenKind::Identifier(ident));
        Ok(Token::new(kind, span))
    }

    fn lex_number(&mut self) -> Result<Token, LexError> {
        let start_index = self.index;
        let start_line = self.line;
        let start_col = self.column;
        let mut literal = String::new();
        while let Some(ch) = self.peek_char() {
            if ch.is_ascii_digit() || ch == '_' {
                literal.push(ch);
                self.advance_char();
            } else {
                break;
            }
        }
        let mut is_float = false;
        if self.peek_char() == Some('.') {
            if let Some(next) = self.peek_nth_char(1) {
                if next.is_ascii_digit() {
                    is_float = true;
                    literal.push('.');
                    self.advance_char(); // consume '.'
                    while let Some(ch) = self.peek_char() {
                        if ch.is_ascii_digit() || ch == '_' {
                            literal.push(ch);
                            self.advance_char();
                        } else {
                            break;
                        }
                    }
                }
            }
        }
        if literal.is_empty() {
            let span = Span::new(start_index, self.index, start_line, start_col);
            return Err(LexError::InvalidNumber { span });
        }
        let kind = if is_float {
            TokenKind::FloatLiteral(literal)
        } else {
            TokenKind::IntegerLiteral(literal)
        };
        let span = Span::new(start_index, self.index, start_line, start_col);
        Ok(Token::new(kind, span))
    }

    fn lex_string(&mut self) -> Result<Token, LexError> {
        let start_index = self.index;
        let start_line = self.line;
        let start_col = self.column;
        self.advance_char(); // opening "
        let mut value = String::new();
        loop {
            match self.advance_char() {
                Some((_, '"')) => {
                    let span = Span::new(start_index, self.index, start_line, start_col);
                    return Ok(Token::new(TokenKind::StringLiteral(value), span));
                }
                Some((_, '\\')) => {
                    if let Some((_, esc)) = self.advance_char() {
                        let ch = match esc {
                            'n' => '\n',
                            'r' => '\r',
                            't' => '\t',
                            '\\' => '\\',
                            '"' => '"',
                            '\'' => '\'',
                            other => other,
                        };
                        value.push(ch);
                    } else {
                        let span = Span::new(start_index, self.index, start_line, start_col);
                        return Err(LexError::UnterminatedString { span });
                    }
                }
                Some((_, ch)) => value.push(ch),
                None => {
                    let span = Span::new(start_index, self.index, start_line, start_col);
                    return Err(LexError::UnterminatedString { span });
                }
            }
        }
    }

    fn lex_char_literal(&mut self) -> Result<Token, LexError> {
        let start_index = self.index;
        let start_line = self.line;
        let start_col = self.column;
        self.advance_char(); // opening '
        let value = match self.advance_char() {
            Some((_, '\\')) => match self.advance_char() {
                Some((_, esc)) => match esc {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    '\\' => '\\',
                    '"' => '"',
                    '\'' => '\'',
                    other => other,
                },
                None => {
                    let span = Span::new(start_index, self.index, start_line, start_col);
                    return Err(LexError::InvalidCharLiteral { span });
                }
            },
            Some((_, ch)) => ch,
            None => {
                let span = Span::new(start_index, self.index, start_line, start_col);
                return Err(LexError::InvalidCharLiteral { span });
            }
        };
        match self.advance_char() {
            Some((_, '\'')) => {
                let span = Span::new(start_index, self.index, start_line, start_col);
                Ok(Token::new(TokenKind::CharLiteral(value), span))
            }
            _ => {
                let span = Span::new(start_index, self.index, start_line, start_col);
                Err(LexError::InvalidCharLiteral { span })
            }
        }
    }

    fn skip_trivia(&mut self) -> Result<(), LexError> {
        loop {
            let mut ate = false;
            while matches!(self.peek_char(), Some(ch) if ch.is_whitespace()) {
                ate = true;
                self.advance_char();
            }
            if self.peek_char() == Some('/') {
                match self.peek_second_char() {
                    Some('/') => {
                        ate = true;
                        self.advance_char();
                        self.advance_char();
                        while let Some((_, ch)) = self.advance_char() {
                            if ch == '\n' {
                                break;
                            }
                        }
                    }
                    Some('*') => {
                        ate = true;
                        let start_span =
                            Span::new(self.index, self.index + 2, self.line, self.column);
                        self.advance_char();
                        self.advance_char();
                        let mut closed = false;
                        while let Some((_, ch)) = self.advance_char() {
                            if ch == '*' && self.peek_char() == Some('/') {
                                self.advance_char();
                                closed = true;
                                break;
                            }
                        }
                        if !closed {
                            return Err(LexError::UnterminatedBlockComment { span: start_span });
                        }
                    }
                    _ => {}
                }
            }
            if !ate {
                break;
            }
        }
        Ok(())
    }

    fn simple_token(&mut self, kind: TokenKind) -> Token {
        let start_index = self.index;
        let start_line = self.line;
        let start_col = self.column;
        self.advance_char();
        let span = Span::new(start_index, self.index, start_line, start_col);
        Token::new(kind, span)
    }

    fn multi_char_token(&mut self, len: usize, kind: TokenKind) -> Token {
        let start_index = self.index;
        let start_line = self.line;
        let start_col = self.column;
        for _ in 0..len {
            self.advance_char();
        }
        let span = Span::new(start_index, self.index, start_line, start_col);
        Token::new(kind, span)
    }

    fn peek_char(&self) -> Option<char> {
        self.source[self.index..].chars().next()
    }

    fn peek_second_char(&self) -> Option<char> {
        let mut iter = self.source[self.index..].chars();
        iter.next()?;
        iter.next()
    }

    fn peek_nth_char(&self, n: usize) -> Option<char> {
        let mut iter = self.source[self.index..].chars();
        for _ in 0..n {
            iter.next()?;
        }
        iter.next()
    }

    fn advance_char(&mut self) -> Option<(usize, char)> {
        if self.index >= self.source.len() {
            return None;
        }
        let ch = self.peek_char()?;
        let idx = self.index;
        self.index += ch.len_utf8();
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some((idx, ch))
    }

    fn skip_bom(&mut self) {
        if self.index == 0 && self.peek_char() == Some('\u{feff}') {
            self.advance_char();
        }
    }
}

fn is_ident_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_ident_char(ch: char) -> bool {
    is_ident_start(ch) || ch.is_ascii_digit()
}

fn keyword(ident: &str) -> Option<Keyword> {
    match ident {
        "import" => Some(Keyword::Import),
        "as" => Some(Keyword::As),
        "extern" => Some(Keyword::Extern),
        "fun" => Some(Keyword::Fun),
        "async" => Some(Keyword::Async),
        "let" => Some(Keyword::Let),
        "var" => Some(Keyword::Var),
        "struct" => Some(Keyword::Struct),
        "enum" => Some(Keyword::Enum),
        "trait" => Some(Keyword::Trait),
        "impl" => Some(Keyword::Impl),
        "return" => Some(Keyword::Return),
        "in" => Some(Keyword::In),
        "if" => Some(Keyword::If),
        "else" => Some(Keyword::Else),
        "while" => Some(Keyword::While),
        "for" => Some(Keyword::For),
        "switch" => Some(Keyword::Switch),
        "try" => Some(Keyword::Try),
        "catch" => Some(Keyword::Catch),
        "unsafe" => Some(Keyword::Unsafe),
        "assembly" => Some(Keyword::Assembly),
        "slice" => Some(Keyword::Slice),
        "tuple" => Some(Keyword::Tuple),
        "mut" => Some(Keyword::Mut),
        "await" => Some(Keyword::Await),
        "true" => Some(Keyword::True),
        "false" => Some(Keyword::False),
        _ => None,
    }
}
