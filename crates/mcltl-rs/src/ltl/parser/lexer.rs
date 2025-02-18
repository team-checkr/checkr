use plex::lexer;

#[derive(Debug, Clone)]
pub enum Token {
    Ident(String),
    Whitespace,
    LParen,
    RParen,

    True,
    False,
    Not,
    And,
    Or,
    G,
    F,
    U,
    V,
    R,
}

lexer! {
    fn next_token(text: 'a) -> Token;
    r#"[ \t\r\n]+"# => Token::Whitespace,

    r#"\("# => Token::LParen,
    r#"\)"# => Token::RParen,
    r#"true|True|T|TRUE"# => Token::True,
    r#"false|False|F|FALSE"# => Token::False,

    r#"\~|not"# => Token::Not,
    r#"\/\\|and"# => Token::And,
    r#"\\\/|or"# => Token::Or,
    r#"G"# => Token::G,
    r#"F"# => Token::F,
    r#"U"# => Token::U,
    r#"V"# => Token::V,
    r#"R"# => Token::R,

    r#"[a-z_][a-z0-9_]*"# => Token::Ident(text.to_owned()),

    r#"."# => panic!("unexpected character: {}", text),
}

pub struct Lexer<'a> {
    original: &'a str,
    remaining: &'a str,
}

impl<'a> Lexer<'a> {
    pub fn new(s: &'a str) -> Lexer<'a> {
        Lexer {
            original: s,
            remaining: s,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Span {
    pub lo: usize,
    pub hi: usize,
}

impl Iterator for Lexer<'_> {
    type Item = (Token, Span);
    fn next(&mut self) -> Option<(Token, Span)> {
        loop {
            let (tok, span) = if let Some((tok, new_remaining)) = next_token(self.remaining) {
                let lo = self.original.len() - self.remaining.len();
                let hi = self.original.len() - new_remaining.len();
                self.remaining = new_remaining;
                (tok, Span { lo, hi })
            } else {
                return None;
            };
            match tok {
                Token::Whitespace => {
                    continue;
                }
                tok => {
                    return Some((tok, span));
                }
            }
        }
    }
}
