use std::{cell::RefCell, collections::HashMap, hash::Hash, rc::Rc};

use crate::list::{List, Node};

pub struct LRU<Key, Val> {
    cap: usize,
    siz: usize,
    index: HashMap<Key, Rc<RefCell<Node<(Key, Val)>>>>,
    order: List<(Key, Val)>,
}

impl<K, V> LRU<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    pub fn new(cap: usize) -> Self {
        Self {
            cap,
            siz: 0,
            index: HashMap::new(),
            order: List::new(),
        }
    }

    pub fn set(&mut self, key: K, val: V) {
        match self.index.get(&key) {
            Some(node) => {
                self.order.move_to_front(node);
                node.borrow_mut().data = (key, val);
            }
            None => {
                if self.cap == 0 {
                    return;
                }
                if self.siz == self.cap {
                    let (key_to_evict, _) =
                        self.order.pop_back().unwrap_or_else(|| {
                            unreachable!("Empty order with size>0")
                        });
                    self.index.remove(&key_to_evict);
                    self.siz -= 1;
                }
                let node = self.order.push_front((key.clone(), val));
                self.index.insert(key, node);
                self.siz += 1;
            }
        }
    }

    pub fn get(&mut self, key: &K) -> Option<V> {
        self.index.get(key).map(|node| {
            self.order.move_to_front(node);
            node.borrow().data.1.clone()
        })
    }

    pub fn del(&mut self, key: &K) {
        if let Some(node) = self.index.get(key) {
            self.order.move_to_front(node);
            self.order.pop_front();
        }
    }

    pub fn export(&self) -> Vec<(K, V)> {
        self.order.to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::LRU;

    #[test]
    fn basic() {
        let mut cache = LRU::new(2);
        assert!(cache.export().is_empty());

        cache.set("a", 1);
        cache.set("b", 2);
        assert_eq!(vec![("b", 2), ("a", 1)], cache.export());

        assert_eq!(cache.get(&"a"), Some(1));
        assert_eq!(vec![("a", 1), ("b", 2)], cache.export());

        assert_eq!(cache.get(&"b"), Some(2));
        assert_eq!(vec![("b", 2), ("a", 1)], cache.export());

        cache.set("a", 3);
        assert_eq!(vec![("a", 3), ("b", 2)], cache.export());

        assert_eq!(cache.get(&"b"), Some(2));
        assert_eq!(vec![("b", 2), ("a", 3)], cache.export());

        cache.set("a", 4);
        assert_eq!(vec![("a", 4), ("b", 2)], cache.export());

        cache.set("b", 5);
        assert_eq!(vec![("b", 5), ("a", 4)], cache.export());

        // At capacity - last is removed.
        cache.set("c", 6);
        assert_eq!(vec![("c", 6), ("b", 5)], cache.export());

        cache.del(&"c");
        assert_eq!(vec![("b", 5)], cache.export());

        cache.del(&"b");
        assert!(cache.export().is_empty());
    }

    #[test]
    fn zero_cap() {
        let mut cache = LRU::new(0);
        assert!(cache.export().is_empty());
        cache.set("a", 1);
        cache.set("b", 2);
        cache.set("c", 3);
        assert!(cache.export().is_empty());
    }
}
