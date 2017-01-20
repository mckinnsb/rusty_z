use super::memory_view::*;
use super::object_view::*;

pub struct ObjectProperty {
    pub value: u16,
    pub info: ObjectPropertyInfo,
}

#[derive(Clone)]
#[derive(Copy)]
pub struct ObjectPropertyInfo {
    // this is an option, because if not found, it will be none
    pub addr: Option<u32>,
    pub id: u8,
    // this is an option, because if not found, it will be none
    // in this case however, get_prop should panic
    pub size: u8,
}

pub struct ObjectPropertiesView {
    pub object_id: u16,
    // this will be an even # of bytes, but is just a byte
    pub text_size: u8,
    // this is global, we will have to use global read functions
    // to easily read the defaults table from any one object property view
    pub defaults_view: MemoryView,
    pub view: MemoryView,
}

impl ObjectPropertiesView {

    pub fn create(object_id: u16,
                  pointer_position: u32,
                  defaults_view: &MemoryView,
                  memory: &MemoryView)
                  -> ObjectPropertiesView {

        let mut view = memory.clone();
        view.pointer = pointer_position;

        let text_size = view.read_at(pointer_position);

        ObjectPropertiesView {
            object_id: object_id,
            defaults_view: defaults_view.clone(),
            text_size: text_size,
            view: view,
        }

    }

    pub fn get_object_property_from_size_byte(size_byte: u8) -> ObjectPropertyInfo {
        // the size byte packs the size by encoding it as
        // byte = 32(l-1) + id
        ObjectPropertyInfo {
            // careful now - don't try to feed this to the wrong place
            addr: None,
            size: (size_byte / 32) + 1,
            id: (size_byte) % 32,
        }
    }

    // gets the property address ( full address )
    pub fn get_property_addr(&self, property_index: u8) -> u32 {
        let info = self.get_property_info(property_index);
        self.get_property_addr_from_info(info)
    }

    // gets the property address from an info object ( helpful if you already
    // have the info around )
    pub fn get_property_addr_from_info(&self, info: ObjectPropertyInfo) -> u32 {
        match info.addr {
            None => 0,
            Some(x) => x + self.view.pointer,
        }
    }

    // gets the property info, which includes address, size, and id
    pub fn get_property_info(&self, property_index: u8) -> ObjectPropertyInfo {

        // we skip the text and the text size byte
        let mut pointer_cursor = 2 * (self.text_size as u32) + 1;

        let mut info = ObjectPropertyInfo {
            addr: None,
            id: property_index,
            size: 0,
        };

        let find_first = property_index == 0;

        // could use a while, but thats sort of not using destructuring
        loop {

            let size_byte = self.view.read_at_head(pointer_cursor);

            if size_byte == 0 {
                // terminate on size byte of 0
                break;
            }

            let found_info = ObjectPropertiesView::get_object_property_from_size_byte(size_byte);

            if find_first || found_info.id == info.id {
                info.size = found_info.size;
                info.addr = Some(pointer_cursor + 1);
                break;
            }

            pointer_cursor += (found_info.size as u32) + 1;

        }

        info

    }

    // gets the property default for this property
    pub fn get_property_default(&self, property_index: u8) -> u16 {
        self.defaults_view.read_u16_at_head((property_index as u32 - 1) * 2)
    }

    // note that this is a little inefficient. but whatever
    // if this really ends up being a performance problem, we can come back to it
    // this will return the default value if the property is not found
    pub fn get_property(&self, property_index: u8) -> ObjectProperty {

        let info = self.get_property_info(property_index);

        let value = match info.addr {
            None => {
                // println!( "reading default for:{}", property_index );
                self.get_property_default(property_index)
            }
            Some(addr) => {
                // println!( "found addr: {}", addr );
                // println!( "+ pointer: {}", self.get_property_addr_from_info(info));
                match info.size {
                    1 => self.view.read_at_head(addr) as u16,
                    2 => self.view.read_u16_at_head(addr),
                    _ => {
                        panic!("you have an address but no size, or are trying to read a property \
                                of length > 2")
                    }
                }
            }
        };

        ObjectProperty {
            info: info,
            value: value,
        }

    }

    pub fn write_property(&self, property_index: u8, value: u16) {

        let property = self.get_property(property_index);
        let ObjectPropertyInfo { size, addr, .. } = property.info;

        match (size, addr) {
            (1, Some(addr)) => self.view.write_at_head(addr + 1, value as u8),
            (2, Some(addr)) => self.view.write_u16_at_head(addr + 1, value),
            _ => {
                panic!("you are attempting to write a property to memory that is greater than 2 \
                        bytes, or doesnt exist")
            }
        }

    }
}
