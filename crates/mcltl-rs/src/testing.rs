#[macro_export]
macro_rules! expect_ {
    ($($t:tt)*) => {
        |res| ::insta::assert_snapshot!(res, $($t)*)
    };
}

pub use expect_ as expect;
