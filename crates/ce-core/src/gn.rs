mod gcl_gen;

pub use gcl_gen::Context as GclGenContext;
use rand::Rng;

pub trait Generate: 'static {
    type Context;

    /// Generate a value of this type.
    fn gn<R: Rng>(cx: &mut Self::Context, rng: &mut R) -> Self;
}

impl<T> Generate for Box<T>
where
    T: Generate,
{
    type Context = T::Context;

    fn gn<R: Rng>(cx: &mut Self::Context, rng: &mut R) -> Self {
        Box::new(T::gn(cx, rng))
    }
}
