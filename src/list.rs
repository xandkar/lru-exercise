use std::{
    cell::RefCell,
    collections::HashSet,
    fmt::Debug,
    rc::{Rc, Weak},
    sync::{Mutex, OnceLock},
};

use crate::{id::Id, RefCount};

static NODES: OnceLock<Mutex<HashSet<Id>>> = OnceLock::new();

/// View currently allocated nodes.
pub fn nodes() -> HashSet<Id> {
    let nodes = NODES
        .get_or_init(|| Mutex::new(HashSet::new()))
        .lock()
        .unwrap();
    nodes.clone()
}

#[derive(Debug, Default)]
pub struct Node<T: Debug> {
    id: Id, // For debugging, tracking (de)allocations.

    pub data: T,

    // XXX To demo what happens if we use Rc instead of Weak.
    // prev: Option<Rc<RefCell<Node<T>>>>,
    prev: Option<Weak<RefCell<Node<T>>>>,
    next: Option<Rc<RefCell<Node<T>>>>,
}

impl<T: Debug> Node<T> {
    fn new(data: T) -> Self {
        let id = Id::next();
        let mut nodes = NODES
            .get_or_init(|| Mutex::new(HashSet::new()))
            .lock()
            .unwrap();
        nodes.insert(id);
        Self {
            data,
            id,
            prev: None,
            next: None,
        }
    }
}

// To demonstrate the leak.
// When using Rc instead of Weak for prev - this never gets called:
impl<T: Debug> Drop for Node<T> {
    fn drop(&mut self) {
        eprintln!("[debug] DROPPING list node={self:?}.");
        let mut nodes = NODES
            .get()
            .unwrap_or_else(|| {
                unreachable!(
                    "De-allocating a list node which was never allocated.\
                    node={self:?}"
                )
            })
            .lock()
            .unwrap();
        eprintln!("[debug] DROPPING list node: {:?} of {:?}", self.id, nodes);
        assert!(
            nodes.remove(&self.id),
            "Removing a node that we previously added."
        );
        eprintln!();
    }
}

#[derive(Debug, Default)]
pub struct List<T: Debug> {
    head: Option<Rc<RefCell<Node<T>>>>,
    tail: Option<Rc<RefCell<Node<T>>>>,
}

// TODO Remove unnecessary clones.
impl<T: Clone + Debug> List<T> {
    pub fn new() -> Self {
        Self {
            head: None,
            tail: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        match (&self.head, &self.tail) {
            (None, None) => true,
            (Some(_), Some(_)) => false,
            (None, Some(_)) => unreachable!("Headless tail."),
            (Some(_), None) => unreachable!("Tailless head."),
        }
    }

    pub fn pop_back(&mut self) -> Option<T> {
        let mut data = None;
        if let Some(old_tail) = self.tail.take() {
            data = Some(old_tail.borrow().data.clone());
            match old_tail.borrow_mut().prev.take() {
                None => {
                    let old_head = self.head.take().unwrap_or_else(|| {
                        unreachable!(
                            "Old tail has no predecessor, so it must be head,\
                            yet we have no head!"
                        )
                    });
                    assert!(
                        Rc::ptr_eq(&old_tail, &old_head),
                        "Old tail has no predecessor, so it must be head,\
                        yet it is not the head!"
                    );
                }

                // XXX When prev = Rc:
                // Some(old_prev) => {
                //     old_prev.borrow_mut().next = None;
                //     self.tail = Some(old_prev); // Old previous is new tail.
                // }

                // XXX When prev = Weak:
                Some(old_prev_weak) => {
                    if let Some(old_prev) = old_prev_weak.upgrade() {
                        old_prev.borrow_mut().next = None;
                        self.tail = Some(old_prev); // Old previous is new tail.
                    }
                }
            }
        }
        data
    }

    pub fn pop_front(&mut self) -> Option<T> {
        self.head.take().map(|old_head| {
            let data = old_head.borrow().data.clone();
            if let Some(next) = old_head.borrow_mut().next.take() {
                next.borrow_mut().prev = None;
                self.head = Some(next);
            }
            if self.head.is_none() {
                self.tail = None;
            }
            data
        })
    }

    pub fn push_front(&mut self, data: T) -> Rc<RefCell<Node<T>>> {
        // 1. Allocate.
        let new = Rc::new(RefCell::new(Node::new(data)));

        // 2. Relink.
        // new->old
        // new<-old
        new.borrow_mut().next = self.head.clone();
        match self.head.take() {
            // Empty, so new node will also be tail, in addition to head.
            None => {
                self.tail = Some(new.clone());
            }
            // Non-empty, so old head should link back to new head: new<-old.
            // Note that we already linked forward new->old in above allocation.
            Some(old) => {
                // new<-old

                // XXX When prev = Weak:
                old.borrow_mut().prev = Some(Rc::downgrade(&new));

                // XXX When prev = Rc:
                // old.borrow_mut().prev = Some(new.clone());
            }
        }
        // New node is the new head.
        self.head = Some(new.clone());

        // 3. Return (for use in move_to_front).
        new
    }

    // Given:
    //   - node=B
    //   - list=[A, B, C]
    // Make:
    //   - list=[B, A, C]
    pub fn move_to_front(&mut self, b_node: &Rc<RefCell<Node<T>>>) {
        // Plan:
        // - [x] -<-B B.prev = None
        // - [x] B->A B.next = Some(A) | None // None if list was empty.
        // - [x] A->C A.next = Some(C)
        // - [x] A<-C C.prev = Some(A)
        // - [x] C->* C.next = C.next // No change.
        // - [x] B<-H old_head.prev = Some(B) // Old head isn't necessarily A.
        // - [x]      head = Some(B) // New head.
        // - [x]      tail = tail | Some(A) // No change unless B was tail.
        // Old head is not A if B was further to the right, for example:
        // [X, A, B, C] <-- hear we'd need to relink to B<-X

        if let Some(head) = &self.head {
            if Rc::ptr_eq(head, b_node) {
                // Node is already at the front.
                return;
            }
        }

        let a_node_opt =
            // XXX When prev = Rc:
            // b_node.borrow().prev.clone().map(|a| a);

            // XXX When prev = Weak:
            b_node.borrow().prev.clone().map(|a| a.upgrade()).flatten();

        let c_node_opt = b_node.borrow().next.clone();

        // A->C
        if let Some(a_node) = &a_node_opt {
            a_node.borrow_mut().next = c_node_opt.clone();
        }
        // A<-C
        if let Some(c_node) = &c_node_opt {
            c_node.borrow_mut().prev =
                // XXX When prev = Rc:
                // a_node_opt.clone();

                // XXX When prev = Weak:
                a_node_opt.clone().map(|a| Rc::downgrade(&a));
        }

        // If we actually had [A, B] and now [B, A], then make A the tail.
        // T = A | T
        if let Some(tail) = &self.tail {
            if Rc::ptr_eq(tail, b_node) {
                self.tail = a_node_opt;
            }
        }

        // Put B at the head.
        {
            let mut b_node_mut = b_node.borrow_mut();
            b_node_mut.prev = None;
            match self.head.take() {
                // Non-empty, so: B<-H0, B->H0.
                Some(old_head) => {
                    // XXX When prev = Weak:
                    old_head.borrow_mut().prev = Some(Rc::downgrade(b_node));

                    // XXX When prev = Rc:
                    // old_head.borrow_mut().prev = Some(b_node.clone());

                    b_node_mut.next = Some(old_head);
                }
                // Empty, so: B->-
                None => {
                    b_node_mut.next = None;
                }
            }
        }
        // H = B
        self.head = Some(b_node.clone());
    }

    pub fn to_vec(&self) -> Vec<T> {
        let mut xs: Vec<T> = Vec::new();
        let mut head = self.head.clone();
        while let Some(node) = head {
            let x = node.borrow().data.clone();
            xs.push(x);
            head = node.borrow().next.clone();
        }
        xs
    }

    pub fn to_ref_counts(&self) -> Vec<(Id, T, RefCount)> {
        let mut xs = Vec::new();
        let mut head = self.head.clone();
        while let Some(node) = head {
            let mut ref_count = RefCount::of(&node);
            // XXX -1 because we just made a temp clone which
            //     will be dropped when this method returns.
            ref_count.strong -= 1;

            let id = node.borrow().id;
            let x = node.borrow().data.clone();
            xs.push((id, x, ref_count));
            head = node.borrow().next.clone();
        }
        xs
    }
}

#[cfg(test)]
mod tests {
    use super::List;

    impl List<char> {
        pub fn from_str(s: &str) -> Self {
            let mut selph = Self::new();
            for c in s.chars().rev() {
                selph.push_front(c);
            }
            selph
        }

        pub fn to_string(&self) -> String {
            String::from_iter(self.to_vec())
        }
    }

    #[test]
    fn string_round_trip() {
        let s = "abcdefg";
        assert_eq!(s, List::from_str(s).to_string());
    }

    #[test]
    fn move_to_front_1() {
        let mut list: List<char> = List::new();
        assert!(list.is_empty());
        let a_node = list.push_front('a');
        assert!(!list.is_empty());
        list.move_to_front(&a_node);
        assert_eq!("a", list.to_string());
    }

    #[test]
    fn move_to_front_2() {
        let mut list: List<char> = List::new();
        let a_node = list.push_front('a');
        let _b_node = list.push_front('b');
        assert_eq!("ba", list.to_string());
        list.move_to_front(&a_node);
        assert_eq!("ab", list.to_string());
    }

    #[test]
    fn move_to_front_3() {
        let mut list: List<char> = List::new();
        let _c_node = list.push_front('c');
        let b_node = list.push_front('b');
        let _a_node = list.push_front('a');
        list.move_to_front(&b_node);
        assert_eq!("bac", list.to_string());
    }

    #[test]
    fn move_to_front() {
        let mut list: List<char> = List::new();
        assert_eq!("", list.to_string());
        assert!(list.is_empty());

        let e_node = list.push_front('e');
        let d_node = list.push_front('d');
        let c_node = list.push_front('c');
        let b_node = list.push_front('b');
        let a_node = list.push_front('a');
        assert_eq!("abcde", list.to_string());

        list.move_to_front(&b_node);
        assert_eq!("bacde", list.to_string());
        assert!(!list.is_empty());

        list.move_to_front(&c_node);
        assert_eq!("cbade", list.to_string());
        assert!(!list.is_empty());

        list.move_to_front(&d_node);
        assert_eq!("dcbae", list.to_string());
        assert!(!list.is_empty());

        list.move_to_front(&e_node);
        assert_eq!("edcba", list.to_string());
        assert!(!list.is_empty());

        list.move_to_front(&a_node);
        assert_eq!("aedcb", list.to_string());
        assert!(!list.is_empty());
    }

    #[test]
    fn pop_back() {
        let mut list = List::from_str("abc");
        assert_eq!("abc", list.to_string());
        assert!(!list.is_empty());

        assert_eq!('c', list.pop_back().unwrap());
        assert_eq!("ab", list.to_string());
        assert!(!list.is_empty());

        assert_eq!('b', list.pop_back().unwrap());
        assert_eq!("a", list.to_string());
        assert!(!list.is_empty());

        assert_eq!('a', list.pop_back().unwrap());
        assert_eq!("", list.to_string());
        assert!(list.is_empty());
    }

    #[test]
    fn pop_front() {
        let mut list = List::from_str("abc");
        assert_eq!("abc", list.to_string());
        assert!(!list.is_empty());

        assert_eq!('a', list.pop_front().unwrap());
        assert_eq!("bc", list.to_string());
        assert!(!list.is_empty());

        assert_eq!('b', list.pop_front().unwrap());
        assert_eq!("c", list.to_string());
        assert!(!list.is_empty());

        assert_eq!('c', list.pop_front().unwrap());
        assert_eq!("", list.to_string());
        assert!(list.is_empty());

        assert!(list.pop_front().is_none());
    }
}
