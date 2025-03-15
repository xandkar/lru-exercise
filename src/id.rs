use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Hash, Eq, PartialEq, Default, Clone, Copy)]
pub struct Id(usize);

impl Id {
    pub fn next() -> Self {
        const ORDER: Ordering = Ordering::SeqCst;
        static ID: AtomicUsize = AtomicUsize::new(0);
        let id = ID.load(ORDER);
        ID.fetch_add(1, ORDER);
        Self(id)
    }
}
