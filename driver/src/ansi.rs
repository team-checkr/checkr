#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Span<'a> {
    pub text: &'a str,
    pub code: Option<Code>,
    pub fg: Option<Color>,
    pub bg: Option<Color>,
}

#[tracing::instrument(skip_all)]
pub fn parse_ansi(mut s: &str) -> Vec<Span> {
    let mut spans = Vec::new();

    while !s.is_empty() {
        if let Some((left, right)) = s.split_once('\u{001b}') {
            spans.push(Span {
                text: left,
                code: None,
                fg: None,
                bg: None,
            });

            if !right.starts_with('[') {
                s = &s[1..];
                continue;
            }

            if let Some(end_idx) =
                right[1..].find(|c: char| !(c.is_ascii_digit() || c == '[' || c == ';'))
            {
                for value in right[1..end_idx + 1].split(';') {
                    spans.push(Span {
                        text: "",
                        code: Some(Code::from_str(value)),
                        fg: None,
                        bg: None,
                    });
                }
                s = &right[end_idx + 2..];
            } else {
                todo!()
            }
        } else {
            spans.push(Span {
                text: s,
                code: None,
                fg: None,
                bg: None,
            });
            break;
        }
    }

    let mut fg = None;
    let mut bg = None;

    enum State {
        None,
        SawSet(ColorType, bool),
    }

    let mut state = State::None;

    for span in &mut spans {
        if let Some(code) = span.code {
            match (code, std::mem::replace(&mut state, State::None)) {
                (Code::Color(color, ColorType::Foreground), _) => fg = Some(color),
                (Code::Color(color, ColorType::Background), _) => bg = Some(color),
                (Code::Reset, _) => {
                    fg = None;
                    bg = None;
                }
                (Code::SetColor(ty), _) => {
                    state = State::SawSet(ty, false);
                }
                (Code::Unknown(5), State::SawSet(ty, false)) => {
                    state = State::SawSet(ty, true);
                }
                (_, State::SawSet(ty, true)) => {
                    use Color::*;
                    let colors = [
                        Black,
                        Red,
                        Green,
                        Yellow,
                        Blue,
                        Magenta,
                        Cyan,
                        White,
                        Default,
                        BrightBlack,
                        BrightRed,
                        BrightGreen,
                        BrightYellow,
                        BrightBlue,
                        BrightMagenta,
                        BrightCyan,
                        BrightWhite,
                    ];
                    let Some(color) = colors.get(code.value() as usize + 1) else {
                        continue;
                    };

                    match ty {
                        ColorType::Foreground => fg = Some(*color),
                        ColorType::Background => fg = Some(*color),
                    }
                }
                (Code::Unknown(value), _) => {
                    tracing::warn!(code=?value, "unhandled code");
                }
                _ => {}
            }
        } else {
            span.fg = fg;
            span.bg = bg;
        }
    }

    spans
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColorType {
    Foreground,
    Background,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Code {
    Reset,
    Bright,
    Dim,
    Inverse,
    NoBrightness,
    NoItalic,
    NoUnderline,
    NoInverse,
    SetColor(ColorType),
    NoColor(ColorType),
    Color(Color, ColorType),
    Unknown(u8),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    Default,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
}

macro_rules! impl_codes {
    ($($l:literal: {$($expr:tt)*},)*) => {
        impl Code {
            fn from_str(s: &str) -> Code {
                use Color::*;
                use ColorType::*;

                match s {
                    $(stringify!($l) => $($expr)*,)*
                    value => return Code::Unknown(value.parse().unwrap()),
                }
            }
            pub fn value(self) -> u8 {
                use Color::*;
                use ColorType::*;

                match self {
                    $($($expr)* => $l,)*
                    Code::Unknown(value) => value,
                }
            }
        }

    };
}
impl_codes!(
     0: {Code::Reset},
     1: {Code::Bright},
     2: {Code::Dim},
     7: {Code::Inverse},
    22: {Code::NoBrightness},
    23: {Code::NoItalic},
    24: {Code::NoUnderline},
    27: {Code::NoInverse},
    38: {Code::SetColor(Foreground)},
    39: {Code::NoColor(Foreground)},
    48: {Code::SetColor(Background)},
    49: {Code::NoColor(Background)},

    30: {Code::Color(Black, Foreground)},     40: {Code::Color(Black, Background)},
    31: {Code::Color(Red, Foreground)},       41: {Code::Color(Red, Background)},
    32: {Code::Color(Green, Foreground)},     42: {Code::Color(Green, Background)},
    33: {Code::Color(Yellow, Foreground)},    43: {Code::Color(Yellow, Background)},
    34: {Code::Color(Blue, Foreground)},      44: {Code::Color(Blue, Background)},
    35: {Code::Color(Magenta, Foreground)},   45: {Code::Color(Magenta, Background)},
    36: {Code::Color(Cyan, Foreground)},      46: {Code::Color(Cyan, Background)},
    37: {Code::Color(White, Foreground)},     47: {Code::Color(White, Background)},
    39: {Code::Color(Default, Foreground)},   49: {Code::Color(Default, Background)},

    90: {Code::Color(BrightBlack, Foreground)},   100: {Code::Color(BrightBlack, Background)},
    91: {Code::Color(BrightRed, Foreground)},     101: {Code::Color(BrightRed, Background)},
    92: {Code::Color(BrightGreen, Foreground)},   102: {Code::Color(BrightGreen, Background)},
    93: {Code::Color(BrightYellow, Foreground)},  103: {Code::Color(BrightYellow, Background)},
    94: {Code::Color(BrightBlue, Foreground)},    104: {Code::Color(BrightBlue, Background)},
    95: {Code::Color(BrightMagenta, Foreground)}, 105: {Code::Color(BrightMagenta, Background)},
    96: {Code::Color(BrightCyan, Foreground)},    106: {Code::Color(BrightCyan, Background)},
    97: {Code::Color(BrightWhite, Foreground)},   107: {Code::Color(BrightWhite, Background)},
);
