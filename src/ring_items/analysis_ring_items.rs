use crate::ring_items;
use std::slice::Iter;
///  This module contains definitions and implementations for the internal
///  structure of

//------------------------------------------------------------
// Parameter defiition items:
//------------------------------------------------------------

/// This struct defines the corresopndence between a parameter id and
/// a parameter name.
pub struct ParameterDefinition {
    id: u32,
    name: String,
}
/// The PARAMETER_DEFINITIONS ring item type is really just
/// a vector of ParameterDefinitions
///
pub struct ParameterDefinitions {
    defs: Vec<ParameterDefinition>,
}

impl ParameterDefinition {
    pub fn new(id: u32, name: &str) -> ParameterDefinition {
        ParameterDefinition {
            id: id,
            name: String::from(name),
        }
    }
    pub fn name(&self) -> String {
        self.name.clone()
    }
    pub fn id(&self) -> u32 {
        self.id
    }
}

impl ParameterDefinitions {
    pub fn new() -> ParameterDefinitions {
        ParameterDefinitions {
            defs: Vec::<ParameterDefinition>::new(),
        }
    }
    /// Make a ParameterDefinitions from a raw ring item if possible.
    /// Note that parameter definitions never have body headers.
    ///
    pub fn from_raw(raw: &ring_items::RingItem) -> Option<ParameterDefinitions> {
        if raw.type_id() == ring_items::PARAMETER_DEFINITIONS {
            let mut result = ParameterDefinitions::new();
            let payload = raw.payload().as_slice();
            let num = u32::from_ne_bytes(payload[0..4].try_into().unwrap());
            let mut offset = 4;
            for _ in 1..num {
                result
                    .defs
                    .push(Self::get_parameter_def(&mut offset, &payload));
            }
            Some(result)
        } else {
            None
        }
    }
    /// Convert the set of definitions to a raw ring item.
    ///
    pub fn to_raw(&self) -> ring_items::RingItem {
        let mut result = ring_items::RingItem::new(ring_items::PARAMETER_DEFINITIONS);
        result.add(self.defs.len());
        for def in &self.defs {
            result.add(def.id);
            let mut bytes = String::into_bytes(def.name.clone());
            bytes.push(0);
            result.add_byte_vec(&bytes);
        }

        result
    }

    pub fn iter(&self) -> Iter<'_, ParameterDefinition> {
        self.defs.iter()
    }

    pub fn add_definition(&mut self, def: ParameterDefinition) -> &mut Self {
        self.defs.push(def);
        self
    }

    // Private methods.

    fn get_parameter_def(offset: &mut usize, bytes: &[u8]) -> ParameterDefinition {
        let id = u32::from_ne_bytes(bytes[*offset..*offset + 4].try_into().unwrap());
        *offset = offset.checked_add(4).unwrap();
        let name = ring_items::get_c_string(offset, &bytes);
        ParameterDefinition::new(id, &name)
    }
}
