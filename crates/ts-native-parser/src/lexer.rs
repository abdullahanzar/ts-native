use crate::{ParseError, Span};

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Export,
    Const,
    Let,
    Function,
    Return,
    While,
    True,
    False,
    IntType,
    DoubleType,
    BoolType,
    VoidType,
    Identifier(String),
    IntegerLiteral(i64),
    DoubleLiteral(f64),
    Colon,
    Comma,
    Dot,
    Semicolon,
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Assign,
    Plus,
    Minus,
    Star,
    Slash,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    EqualEqual,
    BangEqual,
    EndOfFile,
}

impl TokenKind {
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Export => "'export'".to_owned(),
            Self::Const => "'const'".to_owned(),
            Self::Let => "'let'".to_owned(),
            Self::Function => "'function'".to_owned(),
            Self::Return => "'return'".to_owned(),
            Self::While => "'while'".to_owned(),
            Self::True => "'true'".to_owned(),
            Self::False => "'false'".to_owned(),
            Self::IntType => "'int'".to_owned(),
            Self::DoubleType => "'double'".to_owned(),
            Self::BoolType => "'bool'".to_owned(),
            Self::VoidType => "'void'".to_owned(),
            Self::Identifier(identifier) => format!("identifier '{identifier}'"),
            Self::IntegerLiteral(value) => format!("integer literal '{value}'"),
            Self::DoubleLiteral(value) => format!("double literal '{value}'"),
            Self::Colon => "':'".to_owned(),
            Self::Comma => "','".to_owned(),
            Self::Dot => "'.'".to_owned(),
            Self::Semicolon => "';'".to_owned(),
            Self::LeftParen => "'('".to_owned(),
            Self::RightParen => "')'".to_owned(),
            Self::LeftBrace => "'{'".to_owned(),
            Self::RightBrace => "'}'".to_owned(),
            Self::Assign => "'='".to_owned(),
            Self::Plus => "'+'".to_owned(),
            Self::Minus => "'-'".to_owned(),
            Self::Star => "'*'".to_owned(),
            Self::Slash => "'/'".to_owned(),
            Self::Less => "'<'".to_owned(),
            Self::LessEqual => "'<='".to_owned(),
            Self::Greater => "'>'".to_owned(),
            Self::GreaterEqual => "'>='".to_owned(),
            Self::EqualEqual => "'=='".to_owned(),
            Self::BangEqual => "'!='".to_owned(),
            Self::EndOfFile => "end of file".to_owned(),
        }
    }
}

pub fn tokenize(source: &str) -> Result<Vec<Token>, ParseError> {
    let mut lexer = Lexer::new(source);
    lexer.tokenize()
}

struct Lexer<'a> {
    source: &'a str,
    bytes: &'a [u8],
    current: usize,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            bytes: source.as_bytes(),
            current: 0,
        }
    }

    fn tokenize(&mut self) -> Result<Vec<Token>, ParseError> {
        let mut tokens = Vec::new();

        while let Some(byte) = self.peek() {
            match byte {
                b' ' | b'\t' | b'\r' | b'\n' => {
                    self.current += 1;
                }
                b'/' if self.peek_next() == Some(b'/') => {
                    self.skip_line_comment();
                }
                b':' => tokens.push(self.single(TokenKind::Colon)),
                b',' => tokens.push(self.single(TokenKind::Comma)),
                b'.' => tokens.push(self.single(TokenKind::Dot)),
                b';' => tokens.push(self.single(TokenKind::Semicolon)),
                b'(' => tokens.push(self.single(TokenKind::LeftParen)),
                b')' => tokens.push(self.single(TokenKind::RightParen)),
                b'{' => tokens.push(self.single(TokenKind::LeftBrace)),
                b'}' => tokens.push(self.single(TokenKind::RightBrace)),
                b'+' => tokens.push(self.single(TokenKind::Plus)),
                b'-' => tokens.push(self.single(TokenKind::Minus)),
                b'*' => tokens.push(self.single(TokenKind::Star)),
                b'/' => tokens.push(self.single(TokenKind::Slash)),
                b'=' => tokens.push(self.match_equals(TokenKind::Assign, TokenKind::EqualEqual)),
                b'<' => tokens.push(self.match_equals(TokenKind::Less, TokenKind::LessEqual)),
                b'>' => tokens.push(self.match_equals(TokenKind::Greater, TokenKind::GreaterEqual)),
                b'!' => {
                    if self.peek_next() == Some(b'=') {
                        let start = self.current;
                        self.current += 2;
                        tokens.push(Token {
                            kind: TokenKind::BangEqual,
                            span: Span::new(start, self.current),
                        });
                    } else {
                        return Err(ParseError::new(
                            self.source,
                            Span::new(self.current, self.current + 1),
                            "unexpected '!'",
                        ));
                    }
                }
                b'0'..=b'9' => tokens.push(self.lex_number()?),
                b'a'..=b'z' | b'A'..=b'Z' | b'_' => tokens.push(self.lex_identifier()),
                _ => {
                    return Err(ParseError::new(
                        self.source,
                        Span::new(self.current, self.current + 1),
                        format!("unexpected character '{}'", char::from(byte)),
                    ));
                }
            }
        }

        tokens.push(Token {
            kind: TokenKind::EndOfFile,
            span: Span::new(self.current, self.current),
        });

        Ok(tokens)
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.current).copied()
    }

    fn peek_next(&self) -> Option<u8> {
        self.bytes.get(self.current + 1).copied()
    }

    fn single(&mut self, kind: TokenKind) -> Token {
        let start = self.current;
        self.current += 1;

        Token {
            kind,
            span: Span::new(start, self.current),
        }
    }

    fn match_equals(&mut self, short: TokenKind, long: TokenKind) -> Token {
        let start = self.current;
        self.current += 1;

        let kind = if self.peek() == Some(b'=') {
            self.current += 1;
            long
        } else {
            short
        };

        Token {
            kind,
            span: Span::new(start, self.current),
        }
    }

    fn skip_line_comment(&mut self) {
        while let Some(byte) = self.peek() {
            self.current += 1;
            if byte == b'\n' {
                break;
            }
        }
    }

    fn lex_number(&mut self) -> Result<Token, ParseError> {
        let start = self.current;

        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.current += 1;
        }

        let is_double = self.peek() == Some(b'.') && matches!(self.peek_next(), Some(b'0'..=b'9'));
        if is_double {
            self.current += 1;
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.current += 1;
            }
        }

        let slice = &self.source[start..self.current];
        let kind = if is_double {
            let value = slice.parse::<f64>().map_err(|_| {
                ParseError::new(
                    self.source,
                    Span::new(start, self.current),
                    format!("invalid floating-point literal '{slice}'"),
                )
            })?;
            TokenKind::DoubleLiteral(value)
        } else {
            let value = slice.parse::<i64>().map_err(|_| {
                ParseError::new(
                    self.source,
                    Span::new(start, self.current),
                    format!("invalid integer literal '{slice}'"),
                )
            })?;
            TokenKind::IntegerLiteral(value)
        };

        Ok(Token {
            kind,
            span: Span::new(start, self.current),
        })
    }

    fn lex_identifier(&mut self) -> Token {
        let start = self.current;
        self.current += 1;

        while matches!(
            self.peek(),
            Some(b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_')
        ) {
            self.current += 1;
        }

        let slice = &self.source[start..self.current];
        let kind = match slice {
            "export" => TokenKind::Export,
            "const" => TokenKind::Const,
            "let" => TokenKind::Let,
            "function" => TokenKind::Function,
            "return" => TokenKind::Return,
            "while" => TokenKind::While,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "int" => TokenKind::IntType,
            "double" => TokenKind::DoubleType,
            "bool" => TokenKind::BoolType,
            "void" => TokenKind::VoidType,
            _ => TokenKind::Identifier(slice.to_owned()),
        };

        Token {
            kind,
            span: Span::new(start, self.current),
        }
    }
}
