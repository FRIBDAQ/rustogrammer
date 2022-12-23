// This is an raw ring item.   Raw in the
// sense that the payload is just a soup of bytes.
// However it wil have methods that allow conversion of this item
// to more structured ring items based on the 'type' field.
//
pub struct RingItem {
    pub size: u32,
    pub type_id: u32,
    pub body_header_size: u32,
    pub payload: Vec<u8>,
}
