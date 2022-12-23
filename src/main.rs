mod ring_items;

fn main() {
    let _item = ring_items::RingItem {
        size: 0,
        type_id: 0,
        body_header_size: 0,
        payload: Vec::new(),
    };
    println!("Hello, world!");
}
