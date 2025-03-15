use std::rc::Rc;

pub mod id;
pub mod list;
pub mod lru;

#[derive(Debug)]
pub struct RefCount {
    pub strong: usize,
    pub weak: usize,
}

impl RefCount {
    pub fn of<T>(r: &Rc<T>) -> Self {
        Self {
            strong: Rc::strong_count(r),
            weak: Rc::weak_count(r),
        }
    }
}
