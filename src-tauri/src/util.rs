pub struct Unique<T, U> {
    iter: T,
    evaluated: Vec<U>,
}

impl<I, T> Iterator for Unique<I, T>
where
    I: Iterator<Item = T>,
    T: std::cmp::PartialEq + Clone,
{
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        let next_item = self.iter.next()?;
        if self.evaluated.contains(&next_item) {
            self.next()
        } else {
            self.evaluated = self
                .evaluated
                .iter()
                .cloned()
                .chain(vec![next_item.clone()])
                .collect();
            Some(next_item)
        }
    }
}

pub trait UniqueIterator<T>: Iterator<Item = T> + Sized {
    fn unique(self) -> Unique<Self, T> {
        Unique {
            iter: self,
            evaluated: Vec::new(),
        }
    }
}

impl<T, I: Iterator<Item = T>> UniqueIterator<T> for I {}
