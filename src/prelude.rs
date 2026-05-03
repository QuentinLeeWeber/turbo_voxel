use std::{collections::HashMap, hash::Hash};

pub trait HashMapExt<I, T>
where
    I: Hash + Eq + Clone,
{
    fn retain_filter<F>(&mut self, f: F) -> Vec<T>
    where
        F: FnMut(&T) -> bool;
}

impl<I, T> HashMapExt<I, T> for HashMap<I, T>
where
    I: Hash + Eq + Clone,
{
    /// Retains only the elements for which the predicate evaluates to true,
    /// returning the removed elements as a vector.
    fn retain_filter<F>(&mut self, mut f: F) -> Vec<T>
    where
        F: FnMut(&T) -> bool,
    {
        let mut filtered: Vec<T> = Vec::new();
        for i in (0..self.len()).rev().collect::<Vec<usize>>() {
            let should_filter = f(&self.values().nth(i).unwrap());
            if should_filter {
                let k = self.keys().nth(i).unwrap().clone();
                let entry = self.remove(&k).unwrap();
                filtered.push(entry);
            }
        }
        filtered
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn retain_filter_removes_and_returns_matching_values() {
        let mut hm: HashMap<i32, i32> = HashMap::new();
        hm.insert(1, 10);
        hm.insert(2, 20);
        hm.insert(3, 30);
        hm.insert(4, 40);

        let removed = hm.retain_filter(|v| *v >= 25);

        assert_eq!(removed.len(), 2);
        assert!(removed.contains(&30));
        assert!(removed.contains(&40));

        assert_eq!(hm.len(), 2);
        assert_eq!(hm.get(&1), Some(&10));
        assert_eq!(hm.get(&2), Some(&20));
        assert!(hm.get(&3).is_none());
        assert!(hm.get(&4).is_none());
    }

    #[test]
    fn retain_filter_returns_empty_when_nothing_matches() {
        let mut hm: HashMap<&str, i32> = HashMap::new();
        hm.insert("a", 1);
        hm.insert("b", 2);

        let removed = hm.retain_filter(|&v| v > 10);

        assert!(removed.is_empty());
        assert_eq!(hm.len(), 2);
        assert_eq!(hm.get("a"), Some(&1));
        assert_eq!(hm.get("b"), Some(&2));
    }

    #[test]
    fn retain_filter_removes_all_when_all_match() {
        let mut hm: HashMap<u8, u8> = HashMap::new();
        hm.insert(1, 5);
        hm.insert(2, 6);
        hm.insert(3, 7);

        let removed = hm.retain_filter(|&v| v >= 5);

        assert_eq!(removed.len(), 3);
        assert!(hm.is_empty());

        assert!(removed.contains(&5));
        assert!(removed.contains(&6));
        assert!(removed.contains(&7));
    }

    #[test]
    fn retain_filter_works_with_non_copy_values() {
        let mut hm: HashMap<&str, String> = HashMap::new();
        hm.insert("m", "markus".to_string());
        hm.insert("e", "elon".to_string());
        hm.insert("j", "jeremy".to_string());

        let removed = hm.retain_filter(|s| s.starts_with('e'));

        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0], "elon".to_string());
        assert_eq!(hm.len(), 2);
        assert!(hm.values().any(|v| v == "markus"));
        assert!(hm.values().any(|v| v == "jeremy"));
    }
}
