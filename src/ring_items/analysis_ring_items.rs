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
        result.add(self.defs.len() as u32);
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
//-------------------------------------------------------------
// Variable values.
//-------------------------------------------------------------
const MAX_UNITS_LENGTH: usize = 32;
///
/// Each variable has a record that describes its value, name and units:
///
pub struct VariableValue {
    value: f64,
    name: String,
    units: String,
}

impl VariableValue {
    pub fn new(value: f64, name: &str, units: &str) -> VariableValue {
        VariableValue {
            value: value,
            name: String::from(name),
            units: String::from(units),
        }
    }

    pub fn value(&self) -> f64 {
        self.value
    }
    pub fn name(&self) -> String {
        self.name.clone()
    }
    pub fn units(&self) -> String {
        self.units.clone()
    }
}

///
/// The variable item is really just a sequence of variable values:
///
pub struct VariableValues {
    defs: Vec<VariableValue>,
}
impl VariableValues {
    pub fn new() -> VariableValues {
        VariableValues {
            defs: Vec::<VariableValue>::new(),
        }
    }
    pub fn iter(&self) -> Iter<'_, VariableValue> {
        self.defs.iter()
    }
    /// Convert from raw if possible:

    pub fn from_raw(raw: &ring_items::RingItem) -> Option<VariableValues> {
        if raw.type_id() == ring_items::VARIABLE_VALUES {
            let mut result = Self::new();
            let payload = raw.payload().as_slice();
            let nvars = u32::from_ne_bytes(payload[0..4].try_into().unwrap());
            let mut offset = 4;
            for _ in 0..nvars {
                let value: f64 =
                    f64::from_ne_bytes(payload[offset..offset + 8].try_into().unwrap());
                let mut off = offset + 8;
                let units = ring_items::get_c_string(&mut off, &payload);
                offset = offset + MAX_UNITS_LENGTH;
                let name = ring_items::get_c_string(&mut offset, &payload);
                result.defs.push(VariableValue::new(value, &name, &units));
            }
            Some(result)
        } else {
            None
        }
    }
    /// Convert to a raw ring item.

    pub fn to_raw(&self) -> ring_items::RingItem {
        // These never have a body  header:

        let mut result = ring_items::RingItem::new(ring_items::VARIABLE_VALUES);
        result.add(self.defs.len() as u32);
        for def in self.defs.iter() {
            result.add(def.value);
            // build a units string padded with nulls out to MAX_UNITS_LENGTH:

            let mut units_bytes = String::into_bytes(def.units.clone());
            while units_bytes.len() < MAX_UNITS_LENGTH {
                units_bytes.push(0);
            }
            result.add_byte_vec(&units_bytes);

            let mut title_bytes = String::into_bytes(def.name.clone());
            title_bytes.push(0);
            result.add_byte_vec(&title_bytes);
        }
        result
    }
}
