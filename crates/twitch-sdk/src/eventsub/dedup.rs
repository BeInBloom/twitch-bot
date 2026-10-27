use std::collections::{HashSet, VecDeque};

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

    /// Returns `false` if the id was already seen recently.
    pub fn insert(&mut self, id: &str) -> bool {
        if self.seen.contains(id) {
            return false;
        }

        if self.order.len() == self.capacity
            && let Some(oldest) = self.order.pop_front()
        {
            self.seen.remove(&oldest);
        }

        self.order.push_back(id.to_owned());
        self.seen.insert(id.to_owned());
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_repeated_ids_and_forgets_the_oldest() {
        let mut ids = RecentIds::new(2);

        assert!(ids.insert("a"));
        assert!(!ids.insert("a"));
        assert!(ids.insert("b"));
        assert!(ids.insert("c"));
        assert!(ids.insert("a"));
    }
}
