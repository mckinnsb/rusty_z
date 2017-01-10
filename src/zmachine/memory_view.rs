use std::rc::*;
use std::cell::RefCell;

// a "view" into environment memory. mostly used to split off the memory
// from zmachine so we can give the memory and stack in different states
// immutable/mutable )
//
// we use this in a lot of different ways, its kind of a catch all utility
// wrapper around the Rc<RefCell<Vec<u8>>> ( the memory map ), which we would
// be passing around anyway
//
// sometimes we can just use it directly when the memory structure is very
// simple ( as in global variables ), but in other cases where we have situations
// such as objects and their property tables, the structure becomes complex
// and to a large degree variable ( objects may be many bytes in size ),
//
// so in those instances, we wrap a memoryview around another object and
// expose a different api

// should have done this a while ago
#[derive(Clone)]
pub struct MemoryView {
    pub memory: Rc<RefCell<Vec<u8>>>,

    // the pointer at the time this view was created -
    // note this is only valid after its created for
    // things like the zmachine, but for other things
    // like views into global memory or object tables,
    // this will always be valid
    //
    // my advice to you , is not to mess with this,
    // even if given a mutable ref
    //
    // it takes up to a u32 because the largest story file allowed
    // is 512kbytes, and u32 is the lowest integer that can represent
    // 512,000 locations in memory is u32
    //
    // it should be noted that this is actually  2^16 * some multiplier,
    // depending on the version;
    //
    // 1-3: 2, or a max size of 128k
    // 4-5: 4, or a max size of 256k
    // 6-8: 8, or a max size of 512k
    //
    // i don't really care about the integer size; you are on your own
    // buddy ( regarding overflows )
    //
    // i could make struct/tuple struct that maybe verifies that the
    // value is below some multiple of 2^16, but that feels like overkill
    // for now
    pub pointer: u32,
}

impl MemoryView {
    // i could return a usize here, but that means more
    // unnecessary casting
    fn program_offset(&self, offset: u32) -> u32 {
        (self.pointer + (offset))
    }

    // this peeks at the top of the stack and copies the first two bytes
    // into an array, and returns
    pub fn peek_at_instruction(&self) -> [u8; 2] {
        let x = [self.read_at_head(0), self.read_at_head(1)];
        x
    }

    pub fn read_at(&self, address: u32) -> u8 {
        // println!("address: {}", address);
        self.memory.borrow()[address as usize]
    }

    pub fn read_at_head(&self, offset: u32) -> u8 {
        self.read_at(self.program_offset(offset))
    }

    pub fn read_u16_at(&self, address: u32) -> u16 {
        let upper_half = (self.read_at(address) as u16) << 8;
        let lower_half = self.read_at(address + 1) as u16;
        let result = upper_half | lower_half;

        // println!( "upper_half: {}", upper_half );
        // println!( "lower_half: {}", lower_half );
        // println!( "result: {}", result );

        result
    }

    pub fn read_u32_at(&self, address: u32) -> u32 {
        let upper_half = (self.read_u16_at(address) as u32) << 16;
        let lower_half = self.read_u16_at(address + 2) as u32;
        let result = upper_half | lower_half;

        result
    }

    pub fn read_u16_at_head(&self, offset: u32) -> u16 {
        self.read_u16_at(self.pointer + offset)
    }

    pub fn read_u32_at_head(&self, offset: u32) -> u32 {
        self.read_u32_at(self.pointer + offset)
    }


    // in the future, i would actually like to make this "mut self"
    //
    // while no property of the memory view itself is changing ( the pointer or Rc<RefCell<Vec>> ),
    // the Vec is. i know that im sort of getting away with a shared reference through the pointer
    // and specifically this access method, but it might be better to communicate that on a higher
    // level through the view anyway, even if no property of the view is property changed,
    // forcing mut to access mut might be a good pattern

    pub fn write_at(&self, address: u32, value: u8) {
        let mut memory = self.memory.borrow_mut();
        memory[address as usize] = value;
    }

    pub fn write_at_head(&self, offset: u32, value: u8) {
        self.write_at(self.pointer + offset, value);
    }

    pub fn write_u16_at(&self, address: u32, value: u16) {
        let upper_half = (value >> 8 & 0xFF) as u8;
        let lower_half = (value & 0xFF) as u8;

        // println!( "writing:{}" , value );
        // println!( "upper:{}" , upper_half );
        // println!( "lower:{}" , lower_half );

        self.write_at(address, upper_half);
        self.write_at(address + 1, lower_half);
    }

    pub fn write_u16_at_head(&self, offset: u32, value: u16) {
        self.write_u16_at(self.pointer + offset, value);
    }
}
