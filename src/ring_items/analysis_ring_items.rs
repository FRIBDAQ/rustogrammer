use crate::ring_items;
use std::mem;
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
            for _ in 0..num {
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
    /// provide an iterator over the variable defs.
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
    /// Provide an iterator over the variable value records.

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
                offset += mem::size_of::<f64>();
                let mut off = offset;
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
    /// Add a new variable value/def.

    pub fn add_def(&mut self, def: VariableValue) -> &mut Self {
        self.defs.push(def);
        self
    }
}
//---------------------------------------------------------------
// Parameter values from an event:
//---------------------------------------------------------------
#[derive(Clone, Copy)]
pub struct ParameterValue {
    id: u32,
    value: f64,
}

impl ParameterValue {
    pub fn new(id: u32, value: f64) -> ParameterValue {
        ParameterValue {
            id: id,
            value: value,
        }
    }
    pub fn id(&self) -> u32 {
        self.id
    }
    pub fn value(&self) -> f64 {
        self.value
    }
}

pub struct ParameterItem {
    trigger: u64,
    parameters: Vec<ParameterValue>,
}

impl ParameterItem {
    pub fn new(trigger: u64) -> ParameterItem {
        ParameterItem {
            trigger: trigger,
            parameters: Vec::<ParameterValue>::new(),
        }
    }
    /// Create a new item from a raw ring item if possible.

    pub fn from_raw(raw: &ring_items::RingItem) -> Option<ParameterItem> {
        if raw.type_id() == ring_items::PARAMETER_DATA {
            let payload = raw.payload().as_slice();
            let trigger: u64 = u64::from_ne_bytes(payload[0..8].try_into().unwrap());
            let mut result = Self::new(trigger);
            let num = u32::from_ne_bytes(payload[8..12].try_into().unwrap());
            let mut offset = 12; // First id/value pair.
            for _ in 0..num {
                let id = u32::from_ne_bytes(
                    payload[offset..offset + mem::size_of::<u32>()]
                        .try_into()
                        .unwrap(),
                );
                offset += mem::size_of::<u32>();
                let value = f64::from_ne_bytes(
                    payload[offset..offset + mem::size_of::<f64>()]
                        .try_into()
                        .unwrap(),
                );
                result.parameters.push(ParameterValue::new(id, value));
                offset = offset + mem::size_of::<f64>();
            }

            Some(result)
        } else {
            None
        }
    }
    /// Convert a parameter values item into a raw one.
    pub fn to_raw(&self) -> ring_items::RingItem {
        // Never any body header so:

        let mut result = ring_items::RingItem::new(ring_items::PARAMETER_DATA);
        result.add(self.trigger).add(self.parameters.len() as u32);
        for p in &self.parameters {
            result.add(p.id()).add(p.value());
        }

        result
    }
    pub fn add(&mut self, id: u32, value: f64) -> &mut ParameterItem {
        self.parameters.push(ParameterValue::new(id, value));
        self
    }
    pub fn add_parameter(&mut self, p: ParameterValue) -> &mut ParameterItem {
        self.parameters.push(p);
        self
    }
    pub fn iter(&self) -> Iter<'_, ParameterValue> {
        self.parameters.iter()
    }
    pub fn trigger(&self) -> u64 {
        self.trigger
    }
}
#[cfg(test)]
mod test_paramdef {
    use crate::analysis_ring_items::ParameterDefinition;
    #[test]
    fn new_1() {
        let def = ParameterDefinition::new(12, "Item");
        assert_eq!(12, def.id);
        assert_eq!(String::from("Item"), def.name);
    }
    #[test]
    fn getter_1() {
        let def = ParameterDefinition::new(12, "Item");
        assert_eq!(String::from("Item"), def.name());
    }
    #[test]
    fn getter_2() {
        let def = ParameterDefinition::new(12, "Item");
        assert_eq!(12, def.id());
    }
}
#[cfg(test)]
mod test_paramdefs {
    use crate::analysis_ring_items::*;
    use crate::ring_items::*;
    use std::mem::size_of;
    #[test]
    fn new_1() {
        let item = ParameterDefinitions::new();
        assert_eq!(0, item.defs.len());
    }
    #[test]
    fn add_1() {
        let mut item = ParameterDefinitions::new();
        let def = ParameterDefinition::new(1, "Item");
        item.add_definition(def);
        assert_eq!(1, item.defs.len());
        assert_eq!(1, item.defs[0].id());
        assert_eq!(String::from("Item"), item.defs[0].name());
    }
    #[test]
    fn add_2() {
        let mut item = ParameterDefinitions::new();
        item.add_definition(ParameterDefinition::new(1, "item1"))
            .add_definition(ParameterDefinition::new(2, "item2"));
        assert_eq!(2, item.defs.len());

        assert_eq!(1, item.defs[0].id());
        assert_eq!(2, item.defs[1].id());
        assert_eq!(String::from("item1"), item.defs[0].name());
        assert_eq!(String::from("item2"), item.defs[1].name());
    }
    #[test]
    fn to_raw_1() {
        // empty (no defs).

        let item = ParameterDefinitions::new();
        let raw = item.to_raw();
        assert_eq!(PARAMETER_DEFINITIONS, raw.type_id());
        assert!(!raw.has_body_header());
        // Body should say there are no items.
        assert_eq!(
            0,
            u32::from_ne_bytes(raw.payload().as_slice()[0..4].try_into().unwrap())
        );
        // Size:

        assert_eq!(4 * size_of::<u32>() as u32, raw.size());
    }
    #[test]
    fn to_raw_2() {
        // Put in two defs:

        let mut item = ParameterDefinitions::new();
        item.add_definition(ParameterDefinition::new(1, "item1"))
            .add_definition(ParameterDefinition::new(2, "item2"));
        let raw = item.to_raw();

        // Since add is used we'll assume the fields are right and only
        // look at the payload:

        let p = raw.payload().as_slice();
        assert_eq!(2, u32::from_ne_bytes(p[0..4].try_into().unwrap()));

        // First def:

        let mut o = 4;
        assert_eq!(1, u32::from_ne_bytes(p[o..o + 4].try_into().unwrap()));
        o += 4; // Name offset:
        assert_eq!(String::from("item1"), get_c_string(&mut o, p));

        // Second def:

        assert_eq!(2, u32::from_ne_bytes(p[o..o + 4].try_into().unwrap()));
        o += 4;
        assert_eq!(String::from("item2"), get_c_string(&mut o, p));
    }
    // Now that to_raw works we can use it to generate items for
    // from _raw.

    #[test]
    fn from_raw_1() {
        let item = ParameterDefinitions::new();
        let raw = item.to_raw();
        let recons = ParameterDefinitions::from_raw(&raw);
        assert!(recons.is_some());
        let recons = recons.unwrap();
        assert_eq!(0, recons.defs.len());
    }
    #[test]
    fn from_raw_2() {
        let mut item = ParameterDefinitions::new();
        item.add_definition(ParameterDefinition::new(1, "item1"))
            .add_definition(ParameterDefinition::new(2, "item2"));
        let raw = item.to_raw();
        let recons = ParameterDefinitions::from_raw(&raw);
        assert!(recons.is_some());
        let recons = recons.unwrap();
        assert_eq!(2, recons.defs.len());
        assert_eq!(item.defs[0].id(), recons.defs[0].id());
        assert_eq!(item.defs[0].name(), recons.defs[0].name());
        assert_eq!(item.defs[1].id(), recons.defs[1].id());
        assert_eq!(item.defs[1].name(), recons.defs[1].name());
    }
    #[test]
    fn from_raw_3() {
        // wrong  type -> None:

        let raw = ring_items::RingItem::new(PARAMETER_DEFINITIONS - 1);
        assert!(ParameterDefinitions::from_raw(&raw).is_none());
    }
    #[test]
    fn getdef_1() {
        let mut item = ParameterDefinitions::new();
        item.add_definition(ParameterDefinition::new(1, "item1"))
            .add_definition(ParameterDefinition::new(2, "item2"));

        let mut i = 0;
        for p in item.iter() {
            assert_eq!(item.defs[i].id(), p.id());
            assert_eq!(item.defs[i].name(), p.name());

            i += 1;
        }
    }
}
#[cfg(test)]
mod test_vars {
    use crate::analysis_ring_items::*;
    use crate::ring_items::*;
    use std::mem::size_of;

    #[test]
    fn newvar_1() {
        let val = VariableValue::new(3.1416, "Angle", "radians");
        assert_eq!(3.1416, val.value());
        assert_eq!(String::from("Angle"), val.name());
        assert_eq!(String::from("radians"), val.units());
    }
    #[test]
    fn newvars_1() {
        let vars = VariableValues::new();
        assert_eq!(0, vars.defs.len());
    }
    #[test]
    fn add_1() {
        let mut vars = VariableValues::new();
        vars.add_def(VariableValue::new(3.1416, "Angle", "radians"))
            .add_def(VariableValue::new(1.5, "Measure", "mm"));
        assert_eq!(2, vars.defs.len());

        assert_eq!(3.1416, vars.defs[0].value());
        assert_eq!(String::from("Angle"), vars.defs[0].name());
        assert_eq!(String::from("radians"), vars.defs[0].units());

        assert_eq!(1.5, vars.defs[1].value());
        assert_eq!(String::from("Measure"), vars.defs[1].name());
        assert_eq!(String::from("mm"), vars.defs[1].units());
    }
    #[test]
    fn iter_1() {
        let mut vars = VariableValues::new();
        vars.add_def(VariableValue::new(3.1416, "Angle", "radians"))
            .add_def(VariableValue::new(1.5, "Measure", "mm"));

        let mut i = 0;
        for v in vars.iter() {
            assert_eq!(vars.defs[i].value(), v.value());
            assert_eq!(vars.defs[i].name(), v.name());
            assert_eq!(vars.defs[i].units(), v.units());

            i += 1;
        }
    }
    #[test]
    fn to_raw_1() {
        // Empty item:

        let vars = VariableValues::new();
        let raw = vars.to_raw();

        assert_eq!(VARIABLE_VALUES, raw.type_id());
        assert_eq!(
            0,
            u32::from_ne_bytes(raw.payload().as_slice()[0..4].try_into().unwrap())
        );
    }
    #[test]
    fn to_raw_2() {
        // Item with def/values:

        let mut vars = VariableValues::new();
        vars.add_def(VariableValue::new(3.1416, "Angle", "radians"))
            .add_def(VariableValue::new(1.5, "Measure", "mm"));

        let raw = vars.to_raw();
        assert_eq!(
            2,
            u32::from_ne_bytes(raw.payload().as_slice()[0..4].try_into().unwrap())
        );
        let mut offset = 4;
        let p = raw.payload.as_slice();
        for i in 0..2 {
            assert_eq!(
                vars.defs[i].value(),
                f64::from_ne_bytes(p[offset..offset + size_of::<f64>()].try_into().unwrap())
            );
            offset += size_of::<f64>();
            let mut o = offset; // units are fixed size:
            assert_eq!(vars.defs[i].units(), get_c_string(&mut o, &p));
            offset += MAX_UNITS_LENGTH;
            assert_eq!(vars.defs[i].name(), get_c_string(&mut offset, &p));
        }
    }

    // With to_raw tested we can use it to generate raw items for
    // from_raw for testing:

    #[test]
    fn from_raw_1() {
        // empty defs.

        let vars = VariableValues::new();
        let raw = vars.to_raw();
        let recons = VariableValues::from_raw(&raw);
        assert!(recons.is_some());
        let recons = recons.unwrap();
        assert_eq!(0, recons.defs.len());
    }
    #[test]
    fn from_raw_2() {
        // with defs.
        let mut vars = VariableValues::new();
        vars.add_def(VariableValue::new(3.1416, "Angle", "radians"))
            .add_def(VariableValue::new(1.5, "Measure", "mm"));

        let raw = vars.to_raw();
        let recons = VariableValues::from_raw(&raw);
        assert!(recons.is_some());
        let recons = recons.unwrap();

        assert_eq!(vars.defs.len(), recons.defs.len());
        for i in 0..vars.defs.len() {
            assert_eq!(vars.defs[i].value(), recons.defs[i].value());
            assert_eq!(vars.defs[i].units(), recons.defs[i].units());
            assert_eq!(vars.defs[i].name(), recons.defs[i].name());
        }
    }
    #[test]
    fn from_raw_3() {
        // wrong type of raw item.

        let raw = RingItem::new(VARIABLE_VALUES + 1); // wrong type.
        assert!(VariableValues::from_raw(&raw).is_none());
    }
}
#[cfg(test)]
mod param_tests {
    use crate::analysis_ring_items::*;
    use crate::ring_items::*;
    use std::mem::size_of;
    // Tests for ParameterValue type

    #[test]
    fn pv_new() {
        let p = ParameterValue::new(2, 1.234);
        assert_eq!(2, p.id());
        assert_eq!(1.234, p.value());
    }

    // Tests for ParameterItem type:

    #[test]
    fn pi_new() {
        let item = ParameterItem::new(12345);
        assert_eq!(12345, item.trigger);
        assert_eq!(0, item.parameters.len());
    }
    #[test]
    fn trigger() {
        let item = ParameterItem::new(12345);
        assert_eq!(12345, item.trigger()); // getter.
    }
    // add method
    #[test]
    fn add_1() {
        let mut item = ParameterItem::new(2345);
        item.add(1, 3.14);
        item.add(3, 5.7);

        assert_eq!(2, item.parameters.len());
        assert_eq!(1, item.parameters[0].id());
        assert_eq!(3.14, item.parameters[0].value());
        assert_eq!(3, item.parameters[1].id());
        assert_eq!(5.7, item.parameters[1].value());
    }
    #[test]
    fn add_2() {
        // chaining adds:

        let mut item = ParameterItem::new(234560);
        item.add(1, 3.14).add(3, 5.7);

        assert_eq!(2, item.parameters.len());
        assert_eq!(1, item.parameters[0].id());
        assert_eq!(3.14, item.parameters[0].value());
        assert_eq!(3, item.parameters[1].id());
        assert_eq!(5.7, item.parameters[1].value());
    }
    // add_parameter method:

    #[test]
    fn add_3() {
        let mut item = ParameterItem::new(111);

        item.add_parameter(ParameterValue::new(1, 3.14));
        item.add_parameter(ParameterValue::new(3, 5.7));

        assert_eq!(2, item.parameters.len());
        assert_eq!(1, item.parameters[0].id());
        assert_eq!(3.14, item.parameters[0].value());
        assert_eq!(3, item.parameters[1].id());
        assert_eq!(5.7, item.parameters[1].value());
    }
    #[test]
    fn add_4() {
        // chaining:

        let mut item = ParameterItem::new(111);

        item.add_parameter(ParameterValue::new(1, 3.14))
            .add_parameter(ParameterValue::new(3, 5.7));

        assert_eq!(2, item.parameters.len());
        assert_eq!(1, item.parameters[0].id());
        assert_eq!(3.14, item.parameters[0].value());
        assert_eq!(3, item.parameters[1].id());
        assert_eq!(5.7, item.parameters[1].value());
    }
    #[test]
    fn add_5() {
        // chaining

        let mut item = ParameterItem::new(111);

        item.add_parameter(ParameterValue::new(1, 3.14)).add(3, 5.7);

        assert_eq!(2, item.parameters.len());
        assert_eq!(1, item.parameters[0].id());
        assert_eq!(3.14, item.parameters[0].value());
        assert_eq!(3, item.parameters[1].id());
        assert_eq!(5.7, item.parameters[1].value());
    }
    #[test]
    fn add_6() {
        // chaining:

        let mut item = ParameterItem::new(111);

        item.add(1, 3.14).add_parameter(ParameterValue::new(3, 5.7));

        assert_eq!(2, item.parameters.len());
        assert_eq!(1, item.parameters[0].id());
        assert_eq!(3.14, item.parameters[0].value());
        assert_eq!(3, item.parameters[1].id());
        assert_eq!(5.7, item.parameters[1].value());
    }
    #[test]
    fn iter() {
        let mut item = ParameterItem::new(111);

        item.add(1, 3.14).add_parameter(ParameterValue::new(3, 5.7));
        let mut i = 0;

        for p in item.iter() {
            assert_eq!(item.parameters[i].id(), p.id());
            assert_eq!(item.parameters[i].value(), p.value());
            i += 1;
        }
    }
    // Tests for to_raw;  Once that workw we can test from_raw using to_raw
    // to painlessly create our raw items.
    #[test]
    fn to_raw_1() {
        let item = ParameterItem::new(12345);
        let raw = item.to_raw();

        assert!(!raw.has_body_header());
        assert_eq!(PARAMETER_DATA, raw.type_id());

        // Trigger count:

        let p = raw.payload().as_slice();
        assert_eq!(
            12345,
            u64::from_ne_bytes(p[0..size_of::<u64>()].try_into().unwrap())
        );
        assert_eq!(
            0,
            u32::from_ne_bytes(
                p[size_of::<u64>()..size_of::<u64>() + size_of::<u32>()]
                    .try_into()
                    .unwrap()
            )
        );
    }
    #[test]
    fn to_raw_2() {
        let mut item = ParameterItem::new(111);
        item.add(1, 3.14).add_parameter(ParameterValue::new(3, 5.7));
        let raw = item.to_raw();

        let p = raw.payload().as_slice();
        let mut offset = 0;

        // Trigger number:

        assert_eq!(
            111,
            u64::from_ne_bytes(p[offset..offset + size_of::<u64>()].try_into().unwrap())
        );
        // Number of parameter items:

        offset += size_of::<u64>();
        assert_eq!(
            2,
            u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
        );
        offset += size_of::<u32>();
        // The items:

        for i in 0..2 {
            assert_eq!(
                item.parameters[i].id(),
                u32::from_ne_bytes(p[offset..offset + size_of::<u32>()].try_into().unwrap())
            );
            offset += size_of::<u32>();
            assert_eq!(
                item.parameters[i].value(),
                f64::from_ne_bytes(p[offset..offset + size_of::<f64>()].try_into().unwrap())
            );
            offset += size_of::<f64>();
        }
    }
    // from_raw tests can use to_raw to produce the raw item that is the
    // source of the conversion. We'll also need to be sure that wrong
    // type gives None back.

    #[test]
    fn from_raw_1() {
        let orig = ParameterItem::new(124);
        let raw = orig.to_raw();
        let copy = ParameterItem::from_raw(&raw);

        assert!(copy.is_some());
        let copy = copy.unwrap();

        assert_eq!(orig.trigger(), copy.trigger());
        assert_eq!(0, copy.parameters.len());
    }
    #[test]
    fn from_raw_2() {
        let mut orig = ParameterItem::new(12345);
        orig.add(1, 1.2345).add(65, 5.555);
        let raw = orig.to_raw();
        let copy = ParameterItem::from_raw(&raw);

        assert!(copy.is_some());
        let copy = copy.unwrap();

        assert_eq!(orig.trigger(), copy.trigger());
        assert_eq!(orig.parameters.len(), copy.parameters.len());

        for i in 0..orig.parameters.len() {
            assert_eq!(orig.parameters[i].id(), copy.parameters[i].id());
            assert_eq!(orig.parameters[i].value(), copy.parameters[i].value());
        }
    }
    #[test]
    fn from_raw_3() {
        // Bad type gives None:

        let raw = RingItem::new(PARAMETER_DATA+1);
        assert!(ParameterItem::from_raw(&raw).is_none());
    }
}
