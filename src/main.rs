mod ring_items;
use humantime;
use ring_items::format_item;
use ring_items::scaler_item;
use ring_items::state_change;
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
            if let Some(fmt) = format_item::FormatItem::from_raw(&item) {
                dump_format(&fmt);
                let raw = fmt.to_raw();
                println!(
                    "When re-created size: {} type: {} {}",
                    raw.size(),
                    raw.type_id(),
                    raw.has_body_header()
                );
            }
            if let Some(state) =
                state_change::StateChange::from_raw(&item, ring_items::RingVersion::V11)
            {
                dump_state_change(&state);
                let raw = state.to_raw();
                println!(
                    "Recereated size: {} type: {} {}",
                    raw.size(),
                    raw.type_id(),
                    raw.has_body_header()
                );
                let s = state_change::StateChange::from_raw(&raw, ring_items::RingVersion::V11)
                    .unwrap();
                dump_state_change(&s);
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

fn dump_format(fmt: &format_item::FormatItem) {
    println!("Format Item: {}.{}", fmt.major(), fmt.minor());
}

fn dump_state_change(state: &state_change::StateChange) {
    println!("State Change: {}", state.change_type_string());
    println!(
        " run: {} offset {} seconds  ",
        state.run_number(),
        state.time_offset(),
    );
    if let Some(osid) = state.original_sid() {
        println!(" original sid: {}", osid);
    }
    println!("Title: {}", state.title());
    println!(
        " Stamp {}",
        humantime::format_rfc3339(state.absolute_time())
    );
}
