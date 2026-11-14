//! Keeps saves in order: an older snapshot never lands after a newer one.

/// Hands out a rising generation per snapshot and remembers the newest one
/// written.
#[derive(Debug, Default)]
pub struct SaveOrder {
    issued: u64,
    committed: u64,
}

impl SaveOrder {
    /// The generation of a snapshot taken now: one more than the last.
    pub fn issue(&mut self) -> u64 {
        self.issued += 1;
        self.issued
    }

    /// Records that `generation` is being written; false, and nothing
    /// recorded, when a newer one already was.
    pub fn commit(&mut self, generation: u64) -> bool {
        if generation < self.committed {
            return false;
        }
        self.committed = generation;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::SaveOrder;

    #[test]
    fn generations_rise_from_one() {
        let mut saves = SaveOrder::default();
        assert_eq!(saves.issue(), 1);
        assert_eq!(saves.issue(), 2);
    }

    #[test]
    fn an_older_generation_is_refused_after_a_newer_one() {
        let mut saves = SaveOrder::default();
        let older = saves.issue();
        let newer = saves.issue();
        assert!(saves.commit(newer));
        assert!(!saves.commit(older));
        assert!(saves.commit(newer));
        assert!(saves.commit(saves.issued + 1));
    }

    #[test]
    fn generations_written_in_order_are_all_accepted() {
        let mut saves = SaveOrder::default();
        let first = saves.issue();
        let second = saves.issue();
        assert!(saves.commit(first));
        assert!(saves.commit(second));
        assert!(!saves.commit(first));
    }
}
