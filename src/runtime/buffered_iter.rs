use std::iter::FusedIterator;

pub struct BufferedIter<I: Iterator> {
    iter: I,
    buffer: Option<I::Item>,
}

impl<I: Iterator> BufferedIter<I> {
    pub fn new(iter: I) -> Self {
        Self { iter, buffer: None }
    }

    pub fn peek(&mut self) -> Option<&mut I::Item> {
        if self.buffer.is_none() {
            self.buffer = self.iter.next();
        }

        self.buffer.as_mut()
    }
}

impl<I: Iterator> Iterator for BufferedIter<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.buffer.take().or_else(|| self.iter.next())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self.buffer {
            None => self.iter.size_hint(),
            Some(_) => {
                let (lower, upper) = self.iter.size_hint();
                (lower + 1, upper.map(|bound| bound + 1))
            }
        }
    }
}

impl<I: FusedIterator> FusedIterator for BufferedIter<I> {}
