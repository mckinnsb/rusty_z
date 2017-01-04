use std::rc::*;
use std::cell::RefCell;
use super::memory_view::*;
use super::object_properties_view::*;

// this wraps a memory view and is specifically
// for the object table in the zmachine

// this is true for all versions
//
// this is the size of the addr of the properties pointer
// technically it can be anywhere in dynamic memory

const properties_length: u32 = 2;

pub struct ObjectView {
    // how long is an objects attribute field, in bytes?
    pub attributes_length: u32,
    // how long is an object total, in bytes?
    pub object_length: u32,
    // how long is the property defaults table length in bytes?
    // zmachine standards doc gives this in 16-bit words
    pub property_defaults_length: u32,
    // how long is the parent/child/sibling? (in bytes)
    // its assumed that all are the same size
    pub related_obj_length: u32,
    // the memory view for this struct
    // we expect that this has the proper offset
    // for the global variables table
    pub view: MemoryView,
}

impl ObjectView {
    pub fn get_properties_table_view(&self, index: u16) -> ObjectPropertiesView {

        println!("starting: {}", self.view.pointer);

        let offset = ((index as u32 - 1) * self.object_length);
        // first we start from the beginning of the object table
        let pointer_position = self.view.pointer +
                               //then offset be default properties table
                               self.property_defaults_length +
                               //then offset by index of object by size
                               offset +
                               //then offset by attribute length + all relatives length
                               self.attributes_length +
                               self.related_obj_length * 3;

        // now, actually read the address
        // object addresses are not packed and are in dynamic mem

        println!("reading: {}", pointer_position);
        let pointer = self.view.read_u16_at(pointer_position) as u32;
        println!("read: {}", pointer);

        ObjectPropertiesView::create(pointer, &self.view)

    }
}
