use super::memory_view::*;

pub struct GlobalVariablesView {
    // its expected this will point to the table
    pub view: MemoryView,
}

impl GlobalVariablesView {
    // keep in mind this has to be 0..240, not 10..255
    pub fn read_global(&self, index: u16) -> u16 {
        // each global is 2 bytes, so we multiply the offset by 2
        // after subtracting one
        let offset = index * 2;
        self.view.read_u16_at_head(offset as u32)
    }

    pub fn write_global(&self, index: u16, value: u16) {
        let offset = index * 2;
        self.view.write_u16_at_head(offset as u32, value);
    }
}
