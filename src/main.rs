mod ring_items;
use std::fs::File;

fn main() {
    let item = ring_items::RingItem::new(1);
    let item2 = ring_items::RingItem::new_with_body_header(2, 0x123456789, 2, 0);

    println!("Size: {}", item.size());
    println!("Type: {}", item.type_id());
    println!("Has body header: {}", item.has_body_header());

    println!("Size: {}", item2.size());
    println!("Type: {}", item2.type_id());
    println!("Has body header:{}", item2.has_body_header());
    let hdr = item2.get_bodyheader().unwrap();
    println!(" timestamp: {:#08x}", hdr.timestamp);
    println!("  sid     : {}", hdr.source_id);
    println!(" barrier  : {}", hdr.barrier_type);

    if let Ok(mut f) = File::open("run-0000-00.evt") {
        dump_items(&mut f);
    } else {
        println!("Failed to open input file");
    }
}
fn dump_items(f: &mut File) {
    println!("Dumping");
    loop {
        if let Ok(item) = ring_items::RingItem::read_item(f) {
            println!("----------------------");
            println!("Size: {}", item.size());
            println!("type: {}", item.type_id());

            if item.has_body_header() {
                dump_body_header(&item);
            }
        } else {
            println!("done");
            break;
        }
    }
}
fn dump_body_header(item: &ring_items::RingItem) {
    let header = item.get_bodyheader().unwrap();
    println!("Body header:");
    println!("   timestamp: {:#08x}", header.timestamp);
    println!("   sourceid:  {}", header.source_id);
    println!("   barrier:   {}", header.barrier_type);
}
