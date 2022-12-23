mod ring_items;
use crate::ring_items::RingItem;
fn main() {
    let _item = RingItem {
        size: 0,
        type_id: 0,
        body_header_size: 0,
        payload: Vec::new(),
    };
    println!("Hello, world!");
}
