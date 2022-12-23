mod ring_items;

fn main() {
    let item = ring_items::RingItem::new(1);
    println!("Size: {}", item.size());
    println!("Type: {}", item.type_id());
    println!("Has body header: {}", item.has_body_header());
}
