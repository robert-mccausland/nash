use std::fmt::Debug;

#[derive(Debug)]
pub(crate) struct Backtrackable<I: Iterator>
where
    I::Item: Debug + Copy,
{
    history: Vec<I::Item>,
    history_position: usize,
    source: I,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Checkpoint {
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
