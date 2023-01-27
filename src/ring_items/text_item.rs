use crate::ring_items;
use humantime;
use std::fmt;
use std::slice::Iter;
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
    /// Support iteration of the strings:

    pub fn iter(&self) -> Iter<String> {
        self.strings.iter()
    }

    /// add another string to the array of strings.
    /// returning a &mut Self supports chaining to other
    /// methods or multiple adds.
    pub fn add(&mut self, str: &str) -> &mut Self {
        self.strings.push(String::from(str));
        self
    }
}

impl fmt::Display for TextItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Text Item: \n").unwrap();
        write!(f, "  type: {}\n", self.get_item_type_string()).unwrap();
        if let Some(bh) = self.body_header {
            write!(f, "Body header: \n {}\n", bh).unwrap();
        }
        write!(
            f,
            "  Offset {} secs , time {}\n",
            self.get_offset_secs(),
            humantime::format_rfc3339(self.get_absolute_time())
        )
        .unwrap();
        if let Some(sid) = self.get_original_sid() {
            write!(f, "Original sid:  {}\n", sid).unwrap();
        }
        for i in 0..self.get_string_count() {
            write!(f, "String: {} : {}\n", i, self.get_string(i).unwrap()).unwrap();
        }
        write!(f, "")
    }
}

impl ring_items::ToRaw for TextItem {
    fn to_raw(&self) -> ring_items::RingItem {
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

impl ring_items::FromRaw<TextItem> for ring_items::RingItem {
    fn to_specific(&self, vers: ring_items::RingVersion) -> Option<TextItem> {
        // figure out the correct value for the
        // type:

        if let Some(itype) = TextItem::item_type_from_u32(self.type_id()) {
            let mut result = TextItem {
                item_type: itype,
                body_header: self.get_bodyheader(),
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
            let p = self.payload().as_slice();
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
        for i in 0..strings.len() {
            assert_eq!(strings[i], istrings[i]);
            assert_eq!(strings[i], item.get_string(i).unwrap());
        }
        assert!(item.get_string(strings.len()).is_none());
    }
    #[test]
    pub fn getters_2() {
        // no body header v11:
        // body header and v12 format:

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
            2,
            None,
            &strings,
        );

        assert_eq!(TextItemType::MonitoredVariables, item.get_item_type());
        assert_eq!(
            String::from("Monitored variables"),
            item.get_item_type_string()
        );

        assert!(item.get_body_header().is_none());

        assert_eq!(10, item.get_time_offset());
        assert_eq!(5.0, item.get_offset_secs());
        assert_eq!(t, item.get_absolute_time());

        assert!(item.get_original_sid().is_none());

        assert_eq!(strings.len(), item.get_string_count());
        let istrings = item.get_strings();
        for i in 0..strings.len() {
            assert_eq!(strings[i], istrings[i]);
            assert_eq!(strings[i], item.get_string(i).unwrap());
        }
        assert!(item.get_string(strings.len()).is_none());
    }
    #[test]
    fn add_1() {
        // add string to an item:

        let mut strings = vec![
            String::from("one"),
            String::from("two"),
            String::from("three"),
            String::from("Last one"),
        ];
        let t = SystemTime::now();

        let mut item = TextItem::new(
            TextItemType::MonitoredVariables,
            None,
            10,
            t,
            2,
            None,
            &strings,
        );

        let next = "One more string";
        item.add(&next);
        strings.push(String::from(next));

        assert_eq!(strings.len(), item.get_string_count());
        for i in 0..strings.len() {
            assert_eq!(strings[i], item.get_string(i).unwrap());
        }
        assert!(item.get_string(strings.len()).is_none());
    }
    #[test]
    fn add_2() {
        // check chaining:

        // add string to an item:

        let mut strings = vec![
            String::from("one"),
            String::from("two"),
            String::from("three"),
            String::from("Last one"),
        ];
        let t = SystemTime::now();

        let mut item = TextItem::new(
            TextItemType::MonitoredVariables,
            None,
            10,
            t,
            2,
            None,
            &strings,
        );

        let next = "One more string";
        let n = item.add(&next).get_string_count();
        strings.push(String::from(next));
        assert_eq!(strings.len(), n);
    }
    #[test]
    fn iter_1() {
        // test iteration:

        // check chaining:

        // add string to an item:

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
            2,
            None,
            &strings,
        );

        for (i, s) in item.iter().enumerate() {
            assert_eq!(strings[i], *s);
        }
    }
    // as with our other types; we test to_raw() first and then
    // use it to generate raw items to test with from_raw.

    #[test]
    fn to_raw_1() {
        // no strings, no body header, v11:

        let strings = Vec::<String>::new();
        let t = SystemTime::now();
        let item = TextItem::new(TextItemType::PacketTypes, None, 10, t, 1, None, &strings);
        let raw = item.to_raw();
        assert_eq!(PACKET_TYPES, raw.type_id());
        assert!(!raw.has_body_header());

        let p = raw.payload().as_slice();
        let u32s = size_of::<u32>();
        let mut offset = 0;

        // Time offset:

        assert_eq!(
            10,
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;

        // absolute timestamp:

        assert_eq!(
            systime_to_raw(t),
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;

        // No strings:

        assert_eq!(
            0,
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;

        // offset divisor:

        assert_eq!(
            1,
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;

        // In V11 there should be no more bytes:

        assert_eq!(offset, p.len());
    }
    #[test]
    fn to_raw_2() {
        // no strings. v11 body header:

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
        let raw = item.to_raw();
        assert_eq!(PACKET_TYPES, raw.type_id());
        assert!(raw.has_body_header());
        let ibh = raw.get_bodyheader().unwrap();
        assert_eq!(bh.timestamp, ibh.timestamp);
        assert_eq!(bh.source_id, ibh.source_id);
        assert_eq!(bh.barrier_type, ibh.barrier_type);

        let p = raw.payload().as_slice();
        let u32s = size_of::<u32>();
        let mut offset = body_header_size();

        // Time offset:

        assert_eq!(
            10,
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;

        // absolute timestamp:

        assert_eq!(
            systime_to_raw(t),
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;

        // No strings:

        assert_eq!(
            0,
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;

        // offset divisor:

        assert_eq!(
            1,
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;

        // In V11 there should be no more bytes:

        assert_eq!(offset, p.len());
    }
    #[test]
    fn to_raw_3() {
        // No strings, v12, body header:

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
        let raw = item.to_raw();
        assert_eq!(PACKET_TYPES, raw.type_id());
        assert!(raw.has_body_header());
        let ibh = raw.get_bodyheader().unwrap();
        assert_eq!(bh.timestamp, ibh.timestamp);
        assert_eq!(bh.source_id, ibh.source_id);
        assert_eq!(bh.barrier_type, ibh.barrier_type);

        let p = raw.payload().as_slice();
        let u32s = size_of::<u32>();
        let mut offset = body_header_size();

        // Time offset:

        assert_eq!(
            10,
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;

        // absolute timestamp:

        assert_eq!(
            systime_to_raw(t),
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;

        // No strings:

        assert_eq!(
            0,
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;

        // offset divisor:

        assert_eq!(
            1,
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;

        // V12 - original sid:
        assert_eq!(
            5,
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;

        // In V12 there should be no more bytes:

        assert_eq!(offset, p.len());
    }
    #[test]
    fn to_raw_4() {
        // No body header, v11 with strings:

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
        let raw = item.to_raw();

        assert_eq!(MONITORED_VARIABLES, raw.type_id());
        assert!(!raw.has_body_header());

        let p = raw.payload().as_slice();
        let u32s = size_of::<u32>();
        let mut offset = 0;

        // Time offset:

        assert_eq!(
            10,
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;

        // absolute timestamp:

        assert_eq!(
            systime_to_raw(t),
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;

        // No strings:

        assert_eq!(
            strings.len() as u32,
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;

        // offset divisor:

        assert_eq!(
            1,
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;

        // strings start here:

        for i in 0..strings.len() {
            assert_eq!(strings[i], get_c_string(&mut offset, p));
        }

        // Should be at the item end

        assert_eq!(offset, p.len());
    }
    #[test]
    fn to_raw_5() {
        // Body header, strings v11.

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
        let raw = item.to_raw();
        assert_eq!(MONITORED_VARIABLES, raw.type_id());

        assert!(raw.has_body_header());
        let ibh = raw.get_bodyheader().unwrap();
        assert_eq!(bh.timestamp, ibh.timestamp);
        assert_eq!(bh.source_id, ibh.source_id);
        assert_eq!(bh.barrier_type, ibh.barrier_type);

        let p = raw.payload().as_slice();
        let u32s = size_of::<u32>();
        let mut offset = body_header_size();

        // Time offset:

        assert_eq!(
            10,
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;

        // absolute timestamp:

        assert_eq!(
            systime_to_raw(t),
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;

        // No strings:

        assert_eq!(
            strings.len() as u32,
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;

        // offset divisor:

        assert_eq!(
            1,
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;

        // strings start here:

        for i in 0..strings.len() {
            assert_eq!(strings[i], get_c_string(&mut offset, p));
        }

        // Should be at the item end

        assert_eq!(offset, p.len());
    }
    #[test]
    fn to_raw_6() {
        // body header, v12 and strings

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
        let raw = item.to_raw();

        assert_eq!(MONITORED_VARIABLES, raw.type_id());

        assert!(raw.has_body_header());
        let ibh = raw.get_bodyheader().unwrap();
        assert_eq!(bh.timestamp, ibh.timestamp);
        assert_eq!(bh.source_id, ibh.source_id);
        assert_eq!(bh.barrier_type, ibh.barrier_type);

        let p = raw.payload().as_slice();
        let u32s = size_of::<u32>();
        let mut offset = body_header_size();

        // Time offset:

        assert_eq!(
            10,
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;

        // absolute timestamp:

        assert_eq!(
            systime_to_raw(t),
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;

        // No strings:

        assert_eq!(
            strings.len() as u32,
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;

        // offset divisor:

        assert_eq!(
            1,
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;

        // Before the strings is an original sid in V12:

        assert_eq!(
            5,
            u32::from_ne_bytes(p[offset..offset + u32s].try_into().unwrap())
        );
        offset += u32s;

        // strings start here:

        for i in 0..strings.len() {
            assert_eq!(strings[i], get_c_string(&mut offset, p));
        }

        // Should be at the item end

        assert_eq!(offset, p.len());
    }
    // we're on a firm footing with to_raw so we can use it to
    // test from _raw:

    #[test]
    fn from_raw_1() {
        // no strings, v11 no body header:

        let strings = Vec::<String>::new();
        let t = SystemTime::now();
        let item = TextItem::new(TextItemType::PacketTypes, None, 10, t, 1, None, &strings);
        let raw = item.to_raw();
        let recons: Option<TextItem> = raw.to_specific(RingVersion::V11);
        assert!(recons.is_some());
        let recons = recons.unwrap();

        assert_eq!(item.get_item_type(), recons.get_item_type());
        assert!(recons.get_body_header().is_none());
        assert_eq!(item.get_time_offset(), recons.get_time_offset());
        assert_eq!(
            // must use raw format since to_raw truncates to seconds
            systime_to_raw(item.get_absolute_time()),
            systime_to_raw(recons.get_absolute_time())
        );
        assert!(recons.get_original_sid().is_none());
        assert_eq!(0, recons.get_string_count());
    }
    #[test]
    fn from_raw_2() {
        // no strings, v11, body header.

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

        let raw = item.to_raw();
        let recons: Option<TextItem> = raw.to_specific(RingVersion::V11);
        assert!(recons.is_some());
        let recons = recons.unwrap();

        assert_eq!(item.get_item_type(), recons.get_item_type());
        assert!(recons.get_body_header().is_some());
        let ibh = recons.get_body_header().unwrap();
        assert_eq!(bh.timestamp, ibh.timestamp);
        assert_eq!(bh.source_id, ibh.source_id);
        assert_eq!(bh.barrier_type, ibh.barrier_type);

        assert_eq!(item.get_time_offset(), recons.get_time_offset());
        assert_eq!(
            // must use raw format since to_raw truncates to seconds
            systime_to_raw(item.get_absolute_time()),
            systime_to_raw(recons.get_absolute_time())
        );
        assert!(recons.get_original_sid().is_none());
        assert_eq!(0, recons.get_string_count());
    }
    #[test]
    fn from_raw_3() {
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
        let raw = item.to_raw();
        let recons: Option<TextItem> = raw.to_specific(RingVersion::V12);
        assert!(recons.is_some());
        let recons = recons.unwrap();

        assert_eq!(item.get_item_type(), recons.get_item_type());
        assert!(recons.get_body_header().is_some());
        let ibh = recons.get_body_header().unwrap();
        assert_eq!(bh.timestamp, ibh.timestamp);
        assert_eq!(bh.source_id, ibh.source_id);
        assert_eq!(bh.barrier_type, ibh.barrier_type);

        assert_eq!(item.get_time_offset(), recons.get_time_offset());
        assert_eq!(
            // must use raw format since to_raw truncates to seconds
            systime_to_raw(item.get_absolute_time()),
            systime_to_raw(recons.get_absolute_time())
        );
        assert!(recons.get_original_sid().is_some());
        assert_eq!(5, recons.get_original_sid().unwrap());
        assert_eq!(0, recons.get_string_count());
    }
    #[test]
    fn from_raw_4() {
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
        let raw = item.to_raw();
        let recons: Option<TextItem> = raw.to_specific(RingVersion::V11);

        assert!(recons.is_some());
        let recons = recons.unwrap();

        assert_eq!(item.get_item_type(), recons.get_item_type());
        assert!(recons.get_body_header().is_none());

        assert_eq!(item.get_time_offset(), recons.get_time_offset());
        assert_eq!(
            // must use raw format since to_raw truncates to seconds
            systime_to_raw(item.get_absolute_time()),
            systime_to_raw(recons.get_absolute_time())
        );
        assert!(recons.get_original_sid().is_none());
        assert_eq!(item.get_string_count(), recons.get_string_count());

        // This is a more rusty way to compare strings than
        // using enumerate I think?

        for it in item.iter().zip(recons.iter()) {
            assert_eq!(*it.0, *it.1);
        }
    }
    #[test]
    fn from_raw_5() {
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
        let raw = item.to_raw();
        let recons: Option<TextItem> = raw.to_specific(RingVersion::V11);

        assert!(recons.is_some());
        let recons = recons.unwrap();

        assert_eq!(item.get_item_type(), recons.get_item_type());
        assert!(recons.get_body_header().is_some());
        let ibh = recons.get_body_header().unwrap();
        assert_eq!(bh.timestamp, ibh.timestamp);
        assert_eq!(bh.source_id, ibh.source_id);
        assert_eq!(bh.barrier_type, ibh.barrier_type);

        assert_eq!(item.get_time_offset(), recons.get_time_offset());
        assert_eq!(
            // must use raw format since to_raw truncates to seconds
            systime_to_raw(item.get_absolute_time()),
            systime_to_raw(recons.get_absolute_time())
        );
        assert!(recons.get_original_sid().is_none());
        assert_eq!(item.get_string_count(), recons.get_string_count());

        // This is a more rusty way to compare strings than
        // using enumerate I think?

        for it in item.iter().zip(recons.iter()) {
            assert_eq!(*it.0, *it.1);
        }
    }
    #[test]
    fn from_raw_6() {
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

        let raw = item.to_raw();
        let recons: Option<TextItem> = raw.to_specific(RingVersion::V12);

        assert!(recons.is_some());
        let recons = recons.unwrap();

        assert_eq!(item.get_item_type(), recons.get_item_type());
        assert!(recons.get_body_header().is_some());
        let ibh = recons.get_body_header().unwrap();
        assert_eq!(bh.timestamp, ibh.timestamp);
        assert_eq!(bh.source_id, ibh.source_id);
        assert_eq!(bh.barrier_type, ibh.barrier_type);

        assert_eq!(item.get_time_offset(), recons.get_time_offset());
        assert_eq!(
            // must use raw format since to_raw truncates to seconds
            systime_to_raw(item.get_absolute_time()),
            systime_to_raw(recons.get_absolute_time())
        );
        assert!(recons.get_original_sid().is_some());
        assert_eq!(5, recons.get_original_sid().unwrap());
        assert_eq!(item.get_string_count(), recons.get_string_count());

        // This is a more rusty way to compare strings than
        // using enumerate I think?

        for it in item.iter().zip(recons.iter()) {
            assert_eq!(*it.0, *it.1);
        }
    }
    #[test]
    fn from_raw_7() {
        // Must be a valid item type:

        let raw = RingItem::new(BEGIN_RUN); // not a text item.
        let recons : Option<TextItem> = raw.to_specific(RingVersion::V11);
        assert!(recons.is_none());
        let recons : Option<TextItem> = raw.to_specific(RingVersion::V12);
        assert!(recons.is_none());
    }
}
