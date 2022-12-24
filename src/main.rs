mod ring_items;

fn main() {
    let item = ring_items::RingItem::new(1);
    let item2 = ring_items::RingItem::new_with_body_header(2, 0x123456789, 2, 0);

    println!("Size: {}", item.size());
    println!("Type: {}", item.type_id());
    println!("Has body header: {}", item.has_body_header());

    println!("Size: {}", item2.size());
    println!("Type: {}", item2.type_id());
    println!("Has body header:{}", item2.has_body_header());
}
