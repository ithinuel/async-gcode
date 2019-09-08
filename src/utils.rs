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

#[cfg(any(
    feature = "parse-expressions",
    feature = "parse-parameters",
    feature = "parse-comments"
))]
pub(crate) type Stack<T> = crate::std::vec::Vec<T>;

#[cfg(not(any(
    feature = "parse-expressions",
    feature = "parse-parameters",
    feature = "parse-comments"
)))]
#[derive(Debug)]
pub(crate) struct Stack<T>(Option<T>);
#[cfg(not(any(
    feature = "parse-expressions",
    feature = "parse-parameters",
    feature = "parse-comments"
)))]
impl<T> Stack<T> {
    pub fn new() -> Self {
        Self(None)
    }
    pub fn push(&mut self, val: T) {
        if self.0.is_none() {
            self.0 = Some(val);
        } else {
            panic!("stack is full");
        }
    }
    pub fn pop(&mut self) -> Option<T> {
        self.0.take()
    }
    pub fn clear(&mut self) {
        self.0 = None
    }
}
