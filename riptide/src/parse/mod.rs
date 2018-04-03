mod lexer;
pub mod parser;

/// Helper struct that provides the ability to peek from an iterator.
#[derive(Clone, Debug)]
pub struct Peekable<I: Iterator> {
    iter: I,
    peeked: Option<Option<I::Item>>,
}

impl<I: Iterator> Peekable<I> {
    pub fn new(iter: I) -> Self {
        Self {
            iter: iter,
            peeked: None,
        }
    }

    #[inline]
    pub fn peek(&mut self) -> Option<&I::Item> {
        if self.peeked.is_none() {
            self.peeked = Some(self.iter.next());
        }
        match self.peeked {
            Some(Some(ref value)) => Some(value),
            Some(None) => None,
            _ => unreachable!(),
        }
    }
}

impl<I: Iterator> AsRef<I> for Peekable<I> {
    fn as_ref(&self) -> &I {
        &self.iter
    }
}

impl<I: Iterator> Iterator for Peekable<I> {
    type Item = I::Item;

    #[inline]
    fn next(&mut self) -> Option<I::Item> {
        match self.peeked.take() {
            Some(v) => v,
            None => self.iter.next(),
        }
    }
}
