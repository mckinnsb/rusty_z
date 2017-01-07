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
    pub object_id: u16, 
    // how long are the attributes in bytes?
    pub attributes_length: u32,
    // this is the property defaults view - mostly
    // used to create object property views ( which are in different places in memory )
    pub defaults_view: MemoryView,
    // how long is the parent/child/sibling? (in bytes)
    // its assumed that all are the same size
    pub related_obj_length: u32,
    // the memory view for this struct
    // we expect that this has the proper offset
    // for the global variables table
    pub view: MemoryView,
}

impl ObjectView {
    // an object can have only one child - everything else in the "bag" is a sibling
    // of the child
    pub fn get_child(&self) -> u16 {

        // first we start from the beginning of the object table
        //then offset by attribute length + all relatives length
        let pointer_position = self.attributes_length +
                               //the order is parent, sibling, child
                               self.related_obj_length * 2;

        let child_id = self.view.read_u16_at_head(pointer_position);

        child_id

    }

    pub fn get_parent(&self) -> u16 {

        // first we start from the beginning of the object table
        //then offset by attribute length + all relatives length
        let pointer_position = self.attributes_length;
        let parent_id = self.view.read_u16_at_head(pointer_position);

        parent_id

    }

    pub fn get_sibling(&self) -> u16 {

        // first we start from the beginning of the object table
        //then offset by attribute length + all relatives length
        let pointer_position = self.attributes_length +
                               //the order is parent, sibling, child
                               self.related_obj_length;

        let sibling_id = self.view.read_u16_at_head(pointer_position);

        sibling_id

    }
    

    pub fn get_properties_table_view(&self) -> ObjectPropertiesView {

        // println!("starting: {}", self.view.pointer);
        // first we start from the beginning of the object table
        // then offset by attribute length + all relatives length
        let pointer_position = self.attributes_length +
                               self.related_obj_length * 3;

        // we should now be at the properties table address
        // object addresses are not packed and are in dynamic mem
        let pointer = self.view.read_u16_at_head(pointer_position) as u32;

        ObjectPropertiesView::create(pointer, &self.defaults_view, &self.view)

    }

    pub fn has_attribute(&self, attribute: u16) -> bool {
        // this will also have to change with the new version
        // v4 may have up to 48
        match attribute {
            i @ 0...15 => ObjectView::is_bit_set_in_u16(i as u8, self.view.read_u16_at(0)),
            i @ 16...31 => ObjectView::is_bit_set_in_u16((i as u8) - 16, self.view.read_u16_at(1)),
            _ => panic!("attempt to read an invalid attribute"),
        }
    }

    pub fn is_bit_set(num: u8, byte: u8) -> bool {
        num << 1 & byte != 0
    }

    pub fn is_bit_set_in_u16(num: u8, word: u16) -> bool {
        num << 1 & word as u8 != 0
    }

    pub fn set_child( &self, child_id: u16 ) {
        // first we start from the beginning of the object table
        //then offset by attribute length + all relatives length
        let pointer_position = self.attributes_length +
                               //the order is parent, sibling, child
                               self.related_obj_length * 2;

        self.view.write_u16_at_head( pointer_position, child_id );
        
    }

    pub fn set_parent( &self, parent_id: u16 ) {
        // first we start from the beginning of the object table
        //then offset by attribute length + all relatives length
        let pointer_position = self.attributes_length;
                               //the order is parent, sibling, child
                               //so parent has no relative offset after attributes

        self.view.write_u16_at_head( pointer_position, parent_id );
        
    }

    pub fn set_sibling( &self, sibling_id: u16 ) {
        // first we start from the beginning of the object table
        //then offset by attribute length + all relatives length
        let pointer_position = self.attributes_length +
                               //the order is parent, sibling, child
                               self.related_obj_length;
                               //so parent has no relative offset after attributes

        self.view.write_u16_at_head( pointer_position, sibling_id );
        
    }


}
