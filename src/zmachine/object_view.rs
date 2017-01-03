use std::rc::*;
use std::cell::RefCell;
use super::memory_view::*;

//this wraps a memory view and is specifically
//for the object table in the zmachine

//this is true for all versions
//
//this is the size of the addr of the properties pointer
//technically it can be anywhere in dynamic memory

const properties_length : u32 = 2;

pub struct ObjectView {

    //how long is an objects, attribute field, in bytes?
    attributes_length: u8,
    //how long is an object total, in bytes?
    object_length: u8,
    //how long is the property defaults table length in bytes?
    //zmachine standards doc gives this in 16-bit words
    property_defaults_length: u8,
    //how long is the parent/child/sibling? (in bytes)
    //its assumed that all are the same size
    related_obj_length: u8,
    //the memory view for this struct
    //we expect that this has the proper offset
    //for the global variables table
    view: MemoryView,

}

impl ObjectView {

    pub fn get_properties_table_view( &self, index: u16 ) -> MemoryView {

        let offset = index * self.object_length;
        let pointer = self.view.pointer + offset + self.property_defaults_length;

        MemoryView {
            pointer: pointer,
            memory: self.memory.clone(),
        }
    }

}

