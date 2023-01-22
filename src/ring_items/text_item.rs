use crate::ring_items;
use std::time;
///
///  This module provides support for ring items that have
/// payloads that consist of textual strings.  This ring items
/// are normally used for documentation purposes
///

/// More than one type of raw item may be a text item.  This
///
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TextItemType {
    PacketTypes,
    MonitoredVariables,
}

/// the TextItem struct is the internal representation of a textual
/// item.  The Option struct members are those for which, depending
/// on the actual manner of construction of the item (body header or no)
/// or depending on the NSCLDAQ version (V11/V12) may or may not be present
/// in the raw ring item.

pub struct TextItem {
    item_type: TextItemType,
    body_header: Option<ring_items::BodyHeader>,
    time_offset: u32,
    absolute_time: time::SystemTime,
    offset_divisor: u32,
    original_sid: Option<u32>,
    strings: Vec<String>,
}

impl TextItem {
    fn item_type_to_int(item_type: TextItemType) -> u32 {
        match item_type {
            TextItemType::PacketTypes => ring_items::PACKET_TYPES,
            TextItemType::MonitoredVariables => ring_items::MONITORED_VARIABLES,
        }
    }
    fn string_from_item_type(item_type: TextItemType) -> String {
        match item_type {
            TextItemType::PacketTypes => String::from("Packet Types"),
            TextItemType::MonitoredVariables => String::from("Monitored variables"),
        }
    }
    fn item_type_from_u32(type_id: u32) -> Option<TextItemType> {
        match type_id {
            ring_items::PACKET_TYPES => Some(TextItemType::PacketTypes),
            ring_items::MONITORED_VARIABLES => Some(TextItemType::MonitoredVariables),
            _ => None,
        }
    }
    /// Create a new item:

    pub fn new(
        type_id: TextItemType,
        body_header: Option<ring_items::BodyHeader>,
        offset: u32,
        timestamp: time::SystemTime,
        divisor: u32,
        orsid: Option<u32>,
        strings: &Vec<String>,
    ) -> TextItem {
        TextItem {
            item_type: type_id,
            body_header: body_header,
            time_offset: offset,
            absolute_time: timestamp,
            offset_divisor: divisor,
            original_sid: orsid,
            strings: strings.clone(),
        }
    }
    // Getters.

    pub fn get_item_type(&self) -> TextItemType {
        self.item_type
    }
    pub fn get_item_type_string(&self) -> String {
        Self::string_from_item_type(self.item_type)
    }
    pub fn get_body_header(&self) -> Option<ring_items::BodyHeader> {
        self.body_header
    }
    pub fn get_time_offset(&self) -> u32 {
        self.time_offset
    }
    pub fn get_offset_secs(&self) -> f32 {
        (self.time_offset as f32) / (self.offset_divisor as f32)
    }
    pub fn get_absolute_time(&self) -> time::SystemTime {
        self.absolute_time
    }
    pub fn get_original_sid(&self) -> Option<u32> {
        self.original_sid
    }
    pub fn get_string_count(&self) -> usize {
        self.strings.len()
    }
    pub fn get_strings(&self) -> Vec<String> {
        self.strings.clone()
    }
    pub fn get_string(&self, index: usize) -> Option<String> {
        if index < self.strings.len() {
            Some(self.strings[index].clone())
        } else {
            None
        }
    }
    /// add another string to the array of strings.
    /// returning a &mut Self supports chaining to other
    /// methods or multiple adds.
    pub fn add(&mut self, str: &str) -> &mut Self {
        self.strings.push(String::from(str));
        self
    }
    // Conversions.

    pub fn from_raw(raw: &ring_items::RingItem, vers: ring_items::RingVersion) -> Option<TextItem> {
        // figure out the correct value for the
        // type:

        if let Some(itype) = Self::item_type_from_u32(raw.type_id()) {
            let mut result = TextItem {
                item_type: itype,
                body_header: raw.get_bodyheader(),
                time_offset: 0,
                absolute_time: time::SystemTime::now(),
                offset_divisor: 1,
                original_sid: None,
                strings: Vec::new(),
            };
            // Now fill in the rest of the result with stuff
            // from the raw item.  The offset of the payload
            //  depends on the existence
            // or nonexistence of a body header

            let offset: usize = if result.body_header.is_some() {
                ring_items::body_header_size() as usize
            } else {
                0
            };
            let p = raw.payload().as_slice();
            result.time_offset = u32::from_ne_bytes(p[offset..offset + 4].try_into().unwrap());
            result.absolute_time = ring_items::raw_to_systime(u32::from_ne_bytes(
                p[offset + 4..offset + 8].try_into().unwrap(),
            ));
            let num_string = u32::from_ne_bytes(p[offset + 8..offset + 12].try_into().unwrap());
            result.offset_divisor =
                u32::from_ne_bytes(p[offset + 12..offset + 16].try_into().unwrap());
            let mut offset = offset + 16;
            if vers == ring_items::RingVersion::V12 {
                result.original_sid = Some(u32::from_ne_bytes(
                    p[offset..offset + 4].try_into().unwrap(),
                ));
                offset = offset + 4;
            }
            // offset is the offset of the first string.

            for _ in 0..num_string {
                result
                    .strings
                    .push(ring_items::get_c_string(&mut offset, &p));
            }

            Some(result)
        } else {
            None
        }
    }
    

    /// Covert to a raw type

    pub fn to_raw(&self) -> ring_items::RingItem {
        // Create the base raw item with the body header if needed.

        let type_id = Self::item_type_to_int(self.item_type);
        let mut result = if let Some(hdr) = self.body_header {
            ring_items::RingItem::new_with_body_header(
                type_id,
                hdr.timestamp,
                hdr.source_id,
                hdr.barrier_type,
            )
        } else {
            ring_items::RingItem::new(type_id)
        };
        // Add all the fields that a text item needs in the raw item:

        result.add(self.time_offset);
        result.add(ring_items::systime_to_raw(self.absolute_time));
        result.add(self.strings.len() as u32);
        result.add(self.offset_divisor);
        if let Some(sid) = self.original_sid {
            result.add(sid);
        }
        // Now add the strings with a null terimantor separating each:
        // note that into_bytes consumes the string so we clone
        // and that it does not have a null terminator so we add one

        for s in &self.strings {
            let mut bytes = String::into_bytes(s.clone());
            bytes.push(0); // Null terminator.
            result.add_byte_vec(&bytes);
        }

        result
    }
}
#[cfg(test)]
mod text_tests {
    use crate::ring_items::*;
    use crate::text_item::*;
    use std::mem::size_of;
    use std::time::SystemTime;

    #[test]
    fn new_1() {
        // No strings attached, V11, no body header.

        let strings = Vec::<String>::new();
        let t = SystemTime::now();
        let item = TextItem::new(TextItemType::PacketTypes, None, 10, t, 1, None, &strings);

        assert_eq!(TextItemType::PacketTypes, item.item_type);
        assert!(item.body_header.is_none());
        assert_eq!(10, item.time_offset);
        assert_eq!(t, item.absolute_time);
        assert_eq!(1, item.offset_divisor);
        assert!(item.original_sid.is_none());
        assert_eq!(0, strings.len());
    }
    #[test]
    fn new_2() {
        // no strings, V11 body header:

        let strings = Vec::<String>::new();
        let t = SystemTime::now();
        let bh = BodyHeader {
            timestamp: 0x123456789abcdef,
            source_id: 2,
            barrier_type: 0,
        };
        let item = TextItem::new(
            TextItemType::PacketTypes,
            Some(bh),
            10,
            t,
            1,
            None,
            &strings,
        );

        assert_eq!(TextItemType::PacketTypes, item.item_type);
        assert!(item.body_header.is_some());
        let ibh = item.body_header.unwrap(); // assertion  says this is ok.
        assert_eq!(bh.timestamp, ibh.timestamp);
        assert_eq!(bh.source_id, ibh.source_id);
        assert_eq!(bh.barrier_type, ibh.barrier_type);

        assert_eq!(10, item.time_offset);
        assert_eq!(t, item.absolute_time);
        assert_eq!(1, item.offset_divisor);
        assert!(item.original_sid.is_none());
        assert_eq!(0, strings.len());
    }
    #[test]
    fn new_3() {
        // no strings v12, body header.

        let strings = Vec::<String>::new();
        let t = SystemTime::now();
        let bh = BodyHeader {
            timestamp: 0x123456789abcdef,
            source_id: 2,
            barrier_type: 0,
        };
        let item = TextItem::new(
            TextItemType::PacketTypes,
            Some(bh),
            10,
            t,
            1,
            Some(5),
            &strings,
        );

        assert_eq!(TextItemType::PacketTypes, item.item_type);
        assert!(item.body_header.is_some());
        let ibh = item.body_header.unwrap(); // assertion  says this is ok.
        assert_eq!(bh.timestamp, ibh.timestamp);
        assert_eq!(bh.source_id, ibh.source_id);
        assert_eq!(bh.barrier_type, ibh.barrier_type);

        assert_eq!(10, item.time_offset);
        assert_eq!(t, item.absolute_time);
        assert_eq!(1, item.offset_divisor);
        assert!(item.original_sid.is_some());
        assert_eq!(5, item.original_sid.unwrap());
        assert_eq!(0, strings.len());
    }
    // These tests attach strings to the item.

    #[test]
    fn new_4() {
        // no body header, v11 with strings:

        let strings = vec![
            String::from("one"),
            String::from("two"),
            String::from("three"),
            String::from("Last one"),
        ];
        let t = SystemTime::now();

        let item = TextItem::new(
            TextItemType::MonitoredVariables,
            None,
            10,
            t,
            1,
            None,
            &strings,
        );

        assert_eq!(TextItemType::MonitoredVariables, item.item_type);
        assert!(item.body_header.is_none());
        assert_eq!(10, item.time_offset);
        assert_eq!(t, item.absolute_time);
        assert_eq!(1, item.offset_divisor);
        assert!(item.original_sid.is_none());
        assert_eq!(strings.len(), item.strings.len());
        for i in 0..strings.len() {
            assert_eq!(strings[i], item.strings[i]);
        }
    }
    #[test]
    fn new_5() {
        // body header, v11 with strings:
        let strings = vec![
            String::from("one"),
            String::from("two"),
            String::from("three"),
            String::from("Last one"),
        ];
        let t = SystemTime::now();
        let bh = BodyHeader {
            timestamp: 0x123456789abcdef,
            source_id: 2,
            barrier_type: 0,
        };
        let item = TextItem::new(
            TextItemType::MonitoredVariables,
            Some(bh),
            10,
            t,
            1,
            None,
            &strings,
        );

        assert_eq!(TextItemType::MonitoredVariables, item.item_type);
        assert!(item.body_header.is_some());
        let ibh = item.body_header.unwrap(); // assertion  says this is ok.
        assert_eq!(bh.timestamp, ibh.timestamp);
        assert_eq!(bh.source_id, ibh.source_id);
        assert_eq!(bh.barrier_type, ibh.barrier_type);

        assert_eq!(10, item.time_offset);
        assert_eq!(t, item.absolute_time);
        assert_eq!(1, item.offset_divisor);
        assert!(item.original_sid.is_none());
        assert_eq!(strings.len(), item.strings.len());
        for i in 0..strings.len() {
            assert_eq!(strings[i], item.strings[i]);
        }
    }
    #[test]
    fn new_6() {
        // body header and v12 format:

        let strings = vec![
            String::from("one"),
            String::from("two"),
            String::from("three"),
            String::from("Last one"),
        ];
        let t = SystemTime::now();
        let bh = BodyHeader {
            timestamp: 0x123456789abcdef,
            source_id: 2,
            barrier_type: 0,
        };
        let item = TextItem::new(
            TextItemType::MonitoredVariables,
            Some(bh),
            10,
            t,
            1,
            Some(5),
            &strings,
        );

        assert_eq!(TextItemType::MonitoredVariables, item.item_type);
        assert!(item.body_header.is_some());
        let ibh = item.body_header.unwrap(); // assertion  says this is ok.
        assert_eq!(bh.timestamp, ibh.timestamp);
        assert_eq!(bh.source_id, ibh.source_id);
        assert_eq!(bh.barrier_type, ibh.barrier_type);

        assert_eq!(10, item.time_offset);
        assert_eq!(t, item.absolute_time);
        assert_eq!(1, item.offset_divisor);
        assert!(item.original_sid.is_some());
        assert_eq!(5, item.original_sid.unwrap());
        assert_eq!(strings.len(), item.strings.len());
        for i in 0..strings.len() {
            assert_eq!(strings[i], item.strings[i]);
        }
    }
    // test getters:

    #[test]
    fn getters_1() {
        // body header and v12 format:

        let strings = vec![
            String::from("one"),
            String::from("two"),
            String::from("three"),
            String::from("Last one"),
        ];
        let t = SystemTime::now();
        let bh = BodyHeader {
            timestamp: 0x123456789abcdef,
            source_id: 2,
            barrier_type: 0,
        };
        let item = TextItem::new(
            TextItemType::MonitoredVariables,
            Some(bh),
            10,
            t,
            2,
            Some(5),
            &strings,
        );

        assert_eq!(TextItemType::MonitoredVariables, item.get_item_type());
        assert_eq!(
            String::from("Monitored variables"),
            item.get_item_type_string()
        );

        assert!(item.get_body_header().is_some());
        let ibh = item.get_body_header().unwrap();
        assert_eq!(bh.timestamp, ibh.timestamp);
        assert_eq!(bh.source_id, ibh.source_id);
        assert_eq!(bh.barrier_type, ibh.barrier_type);

        assert_eq!(10, item.get_time_offset());
        assert_eq!(5.0, item.get_offset_secs());
        assert_eq!(t, item.get_absolute_time());

        assert!(item.get_original_sid().is_some());
        assert_eq!(5, item.get_original_sid().unwrap());
        
        assert_eq!(strings.len(), item.get_string_count());
        let istrings = item.get_strings();
        for i  in 0..strings.len() {
            assert_eq!(strings[i], istrings[i]);
            assert_eq!(strings[i], item.get_string(i).unwrap());
        }
        assert!(item.get_string(strings.len()).is_none())


    }
}
