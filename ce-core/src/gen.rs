mod gcl_gen;

use rand::Rng;

pub trait Generate: 'static {
    type Context;

    fn gen<R: Rng>(cx: &mut Self::Context, rng: &mut R) -> Self;
}

impl<T> Generate for Box<T>
where
    T: Generate,
{
    type Context = T::Context;

    fn gen<R: Rng>(cx: &mut Self::Context, rng: &mut R) -> Self {
        Box::new(T::gen(cx, rng))
    }
}
