#[cfg(feature = "no_std")]
trait Abs {
    fn abs(&self) -> Self;
}

#[cfg(feature = "no_std")]
impl Abs for f32 {
    fn abs(&self) -> f32 {
        if *self < 0. {
            -*self
        } else {
            *self
        }
    }
}

#[cfg(feature = "parse-expressions")]
#[derive(Debug, PartialEq)]
pub enum Either<T, U> {
    Left(T),
    Right(U),
}
