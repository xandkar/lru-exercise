use lru_exercise::{
    list::{self, List},
    RefCount,
};

fn main() {
    eprintln!("[debug] Allocated nodes: {:?}", list::nodes());
    {
        let mut xs = List::new();
        xs.push_front(3);
        xs.push_front(2);
        xs.push_front(1);
        dbg!(&xs.to_ref_counts());
        eprintln!("[debug] Allocated nodes: {:?}", list::nodes());
    }
    eprintln!("[debug] Out of scope.");
    eprintln!("[debug] Allocated nodes: {:?}", list::nodes());
    let mut xs = List::new();
    dbg!(&xs.to_ref_counts());
    let node_3 = xs.push_front(3);
    eprintln!("[debug] Allocated nodes: {:?}", list::nodes());
    let node_2 = xs.push_front(2);
    eprintln!("[debug] Allocated nodes: {:?}", list::nodes());
    let node_1 = xs.push_front(1);
    eprintln!("[debug] Allocated nodes: {:?}", list::nodes());
    dbg!(&xs.to_ref_counts());
    dbg!(RefCount::of(&node_1));
    dbg!(RefCount::of(&node_2));
    dbg!(RefCount::of(&node_3));
    eprintln!(
        "[debug] >>> COMOEDIA FINITA EST <<< Allocated list nodes: {:?}",
        list::nodes()
    );
}
