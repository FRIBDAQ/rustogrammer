mod ring_items;
use ring_items::abnormal_end;
use ring_items::analysis_ring_items;
use ring_items::event_item;
use ring_items::format_item;
use ring_items::glom_parameters;
use ring_items::scaler_item;
use ring_items::state_change;
use ring_items::text_item;
use ring_items::triggers_item;
use ring_items::FromRaw;
use std::fs::File;

fn main() {
    if let Ok(mut f) = File::open("run-0088-00.evt") {
        dump_items(&mut f);
    } else {
        println!("Failed to open input file");
    }
}
// TODO:  In the code below, for to_specific, we should default
// to a specific version (e.g. V11) but allow format item contents
// to modify accordingly.
fn dump_items(f: &mut File) {
    println!("Dumping");
    loop {
        if let Ok(item) = ring_items::RingItem::read_item(f) {
            println!("---------");
            let f: Option<format_item::FormatItem> = item.to_specific(ring_items::RingVersion::V11);
            if let Some(fmt) = f {
                println!("{}", fmt);
            }
            let sc: Option<state_change::StateChange> = item.to_specific(ring_items::RingVersion::V11);
            if let Some(state) = sc {
                println!("{}", state);
            }
            let s: Option<scaler_item::ScalerItem> = item.to_specific(ring_items::RingVersion::V11);
            if let Some(sc) = s {
                println!("{}", sc);
            }
            let ti : Option<text_item::TextItem> = item.to_specific(ring_items::RingVersion::V11);
            if let Some(t) = ti {
                println!("{}", t);
            }
            let ev: Option<event_item::PhysicsEvent> =
                item.to_specific(ring_items::RingVersion::V11);
            if let Some(e) = ev {
                println!("{}", e);
            }
            if let Some(count) =
                triggers_item::PhysicsEventCountItem::from_raw(&item, ring_items::RingVersion::V11)
            {
                println!("{}", count);
            }
            let g: Option<glom_parameters::GlomParameters> =
                item.to_specific(ring_items::RingVersion::V11);
            if let Some(gp) = g {
                println!("{}", gp);
            }
            let a: Option<abnormal_end::AbnormalEnd> =
                item.to_specific(ring_items::RingVersion::V11);
            if let Some(ae) = a {
                println!("{}", ae);
            }
            let p: Option<analysis_ring_items::ParameterDefinitions> =
                item.to_specific(ring_items::RingVersion::V11);
            if let Some(pd) = p {
                println!("{}", pd);
            }
            let v: Option<analysis_ring_items::VariableValues> =
                item.to_specific(ring_items::RingVersion::V11);
            if let Some(vd) = v {
                println!("{}", vd);
            }
            let p: Option<analysis_ring_items::ParameterItem> =
                item.to_specific(ring_items::RingVersion::V11);
            if let Some(pv) = p {
                println!("{}", pv);
            }
        } else {
            println!("done");
            break;
        }
    }
}
