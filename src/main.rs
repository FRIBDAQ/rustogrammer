mod ring_items;
use humantime;
use ring_items::abnormal_end;
use ring_items::analysis_ring_items;
use ring_items::event_item;
use ring_items::format_item;
use ring_items::glom_parameters;
use ring_items::scaler_item;
use ring_items::state_change;
use ring_items::text_item;
use ring_items::triggers_item;
use std::fs::File;

fn main() {
    if let Ok(mut f) = File::open("run-0088-00.evt") {
        dump_items(&mut f);
    } else {
        println!("Failed to open input file");
    }
}
fn dump_items(f: &mut File) {
    println!("Dumping");
    loop {
        if let Ok(item) = ring_items::RingItem::read_item(f) {
            println!("---------");
            if let Some(fmt) = format_item::FormatItem::from_raw(&item) {
                println!("{}", fmt);
            }
            if let Some(state) =
                state_change::StateChange::from_raw(&item, ring_items::RingVersion::V11)
            {
                println!("{}", state);
            }
            if let Some(sc) = scaler_item::ScalerItem::from_raw(&item, ring_items::RingVersion::V11)
            {
                println!("{}", sc);
            }
            if let Some(t) = text_item::TextItem::from_raw(&item, ring_items::RingVersion::V11) {
                dump_text(&t);
                let raw = t.to_raw();
                println!("Recreated size {} type: {}", raw.size(), raw.type_id());
            }
            if let Some(e) = event_item::PhysicsEvent::from_raw(&item) {
                println!("{}", e);
            }
            if let Some(count) =
                triggers_item::PhysicsEventCountItem::from_raw(&item, ring_items::RingVersion::V11)
            {
                dump_count_item(&count);
                let raw = count.to_raw();
                println!("Recreate size: {} type: {}", raw.size(), raw.type_id());
            }
            if let Some(gp) = glom_parameters::GlomParameters::from_raw(&item) {
                println!("{}", gp);
            }
            if let Some(ae) = abnormal_end::AbnormalEnd::from_raw(&item) {
                println!("{}", ae);
            }
            if let Some(pd) = analysis_ring_items::ParameterDefinitions::from_raw(&item) {
                println!("{}", pd);
            }
            if let Some(vd) = analysis_ring_items::VariableValues::from_raw(&item) {
                println!("{}", vd);
            }
            if let Some(pv) = analysis_ring_items::ParameterItem::from_raw(&item) {
                println!("{}", pv);
            }
        } else {
            println!("done");
            break;
        }
    }
}

fn dump_text(t: &text_item::TextItem) {
    println!("Text Item: ");
    println!("  type: {}", t.get_item_type_string());
    println!(
        "  Offset {} secs , time {} ",
        t.get_offset_secs(),
        humantime::format_rfc3339(t.get_absolute_time())
    );
    if let Some(sid) = t.get_original_sid() {
        println!("Original sid:  {}", sid);
    }
    for i in 0..t.get_string_count() {
        println!("String: {} : {}", i, t.get_string(i).unwrap());
    }
}

fn dump_count_item(c: &triggers_item::PhysicsEventCountItem) {
    println!("Trigger count information: ");
    if let Some(bh) = c.get_bodyheader() {
        println!(
            "bodyheader : ts {:0>8x} sid {} barrier {}",
            bh.timestamp, bh.source_id, bh.barrier_type
        );
    }
    println!(
        "{} Seconds in the run at {} : {} Triggers",
        c.get_offset_time(),
        humantime::format_rfc3339(c.get_absolute_time()),
        c.get_event_count()
    );
    if let Some(sid) = c.get_original_sid() {
        println!("Original sid: {}", sid);
    }
}
