use std::fmt::Debug;

#[derive(Debug)]
pub struct Backtrackable<I: Iterator>
where
    I::Item: Debug + Copy,
{
    history: Vec<I::Item>,
    history_position: usize,
    source: I,
}

#[derive(Debug, Clone, Copy)]
pub struct Checkpoint {
    history_position: usize,
}

impl<I: Iterator> Backtrackable<I>
where
    I::Item: Debug + Copy,
{
    pub fn new(source: I) -> Self {
        Self {
            history: Vec::new(),
            history_position: 0,
            source,
        }
    }

    pub fn backtrack(&mut self, checkpoint: Checkpoint) {
        self.history_position = checkpoint.history_position;
    }

    pub fn checkpoint(&mut self) -> Checkpoint {
        Checkpoint {
            history_position: self.history_position,
        }
    }

    pub fn peek(&mut self) -> Option<I::Item> {
        let result = if self.history_position == self.history.len() {
            if let Some(next) = self.source.next() {
                self.history.push(next);
                next
            } else {
                return None;
            }
        } else {
            self.history[self.history_position]
        };
        return Some(result);
    }
}

impl<I: Iterator> Iterator for Backtrackable<I>
where
    I::Item: Copy + Debug,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(next) = self.peek() {
            self.history_position += 1;
            Some(next)
        } else {
            None
        }
    }
}

pub fn is_duplicates<I: IntoIterator>(value: I) -> Option<I::Item>
where
    I::Item: PartialEq,
{
    let mut iter = value.into_iter();
    let previous = iter.next()?;

    while let Some(value) = iter.next() {
        if value != previous {
            return None;
        }
    }

    return Some(previous);
}

mod test {
    use super::*;

    #[test]
    fn are_duplicates_should_return_none_for_empty_collection() {
        assert_eq!(is_duplicates(Vec::<()>::new()), None)
    }

    #[test]
    fn are_duplicates_should_return_none_for_collection_with_non_duplicate_values() {
        assert_eq!(is_duplicates(vec![1, 2, 3]), None)
    }

    #[derive(Debug, Clone)]
    struct TestItem<T> {
        value: T,
        _marker: u32,
    }

    impl<T> PartialEq for TestItem<T>
    where
        T: Eq,
    {
        fn eq(&self, other: &Self) -> bool {
            self.value == other.value
        }
    }

    #[test]
    fn are_duplicates_should_return_first_item_for_collection_with_duplicate_values() {
        let item1 = TestItem::<u32> {
            value: 1,
            _marker: 1,
        };
        let item2 = TestItem::<u32> {
            value: 1,
            _marker: 2,
        };

        let result = is_duplicates(vec![item1.clone(), item2])
            .expect("Expected are_duplicates to return Some");
        assert_eq!(
            result._marker, item1._marker,
            "Expected item returned by are_duplicates to be the first item of the array"
        );
    }
}
