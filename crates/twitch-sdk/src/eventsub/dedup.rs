use std::collections::{HashSet, VecDeque};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Occurrence {
    First,
    Repeated,
}

pub(crate) struct RecentIds {
    capacity: usize,
    order: VecDeque<String>,
    seen: HashSet<String>,
}

impl RecentIds {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            order: VecDeque::with_capacity(capacity),
            seen: HashSet::with_capacity(capacity),
        }
    }

    pub fn record(&mut self, id: &str) -> Occurrence {
        if self.seen.contains(id) {
            return Occurrence::Repeated;
        }

        if self.is_full() {
            self.forget_oldest();
        }

        self.order.push_back(id.to_owned());
        self.seen.insert(id.to_owned());
        Occurrence::First
    }

    fn is_full(&self) -> bool {
        self.order.len() == self.capacity
    }

    fn forget_oldest(&mut self) {
        if let Some(oldest) = self.order.pop_front() {
            self.seen.remove(&oldest);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_repeated_ids_and_forgets_the_oldest() {
        let mut ids = RecentIds::new(2);

        assert_eq!(ids.record("a"), Occurrence::First);
        assert_eq!(ids.record("a"), Occurrence::Repeated);
        assert_eq!(ids.record("b"), Occurrence::First);
        assert_eq!(ids.record("c"), Occurrence::First);
        assert_eq!(ids.record("a"), Occurrence::First);
    }
}
