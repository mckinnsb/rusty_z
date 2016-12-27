use super::memory_view::*;

use std::rc::*;
use std::cell::RefCell;
use std::cell::Ref;
use std::borrow::Borrow;

// this is the "zmachine header" , its in dynamic memory
// and is up to 64 bytes, described on page 1 and extensively on 11 of the
// zmachine standards at:
//
// http://inform-fiction.org/zmachine/standards/z1point1/sect04.html

pub struct Header {
    memory: Rc<RefCell<Vec<u8>>>,
    // this is the version of the game, we support up to 3 so far
    pub version: u8,
    // these are flags set for the game;
    pub flags: HeaderFlags,
    // where does "high memory", where most coroutines and strings are kept, start?
    pub hi_memory_start: u16,
    // where does the program counter (pc) start?
    pub pc_start: u16,
    // where is the dictionary?
    pub dictionary_location: u16,
    // where are those objects boy?
    pub object_table_location: u16,
    // where are the global variables at?
    pub global_vars_table_location: u16,
    // where does static memory begin?
    pub static_memory_start_location: u16,
}

impl Header {
    // these are dynamic attributes of the header,
    // and so may be written to and read from during
    // game play
    //
    // interpreter settable attributes", such as options,
    // can be set directly to the struct

    pub fn fixed_font(&self) -> bool {
        let memory = self.get_memory();
        (memory[0x10] | 1 << 1) > 0
    }

    pub fn transcripting(&self) -> bool {
        let memory = self.get_memory();
        (memory[0x10] | 1 << 0) > 0
    }

    fn get_memory(&self) -> Ref<Vec<u8>> {
        let header_reference: &RefCell<Vec<u8>> = self.memory.borrow();
        let header_memory: Ref<Vec<u8>> = header_reference.borrow();
        header_memory
    }

    pub fn create(memory: Rc<RefCell<Vec<u8>>>) -> Header {

        //make two clones - one will be dropped by the end of the scope
        let memory_for_struct = memory.clone();
        let memory_for_header = memory.clone();

        let view = MemoryView {
            memory: memory_for_struct,
            //the header pointer is the top of the memory
            pointer: 0,
        };

        let version = view.read_at(0x0);

        let obj = Header {
            //header needs its own reference to build views, and also mutate values
            //
            //the header can only be modified by the interpreter - not the zmachine
            //itself
            memory: memory_for_header,
            version: version,
            flags: HeaderFlags::process_header(&view, version),
            hi_memory_start: view.read_u16_at(0x4),
            pc_start: view.read_u16_at(0x6),
            dictionary_location: view.read_u16_at(0x8),
            object_table_location: view.read_u16_at(0xA),
            global_vars_table_location: view.read_u16_at(0xC),
            static_memory_start_location: view.read_u16_at(0xE),
        };

        obj

    }

    pub fn get_status(&self) -> String {

        let header_flags = match self.flags {
            HeaderFlags::V1 { ref flags } => flags,
        };

        let status_type: &str = match header_flags.status_line {
            StatusLineType::Hours => "Show hours and minutes in status line",
            StatusLineType::Score => "Show score and moves in status line",
        };

        let flags_string: String = format!("status: {}\nsplit_story: {}\nshow_status_line: \
                                            {}\nsplit_screen: {}\n\
                                            variable_pitch_font: {}\n",
                                           status_type,
                                           header_flags.split_story,
                                           header_flags.split_screen,
                                           header_flags.show_status_line,
                                           header_flags.variable_pitch_font);
        let debug: String = format!("version: {}\n{}\nhi_memory_start: {:x}\npc_start: \
                                     {:x}\ndictionary_location:{:x}\nobject_table_location: \
                                     {:x}\nglobal_vars_table_location: \
                                     {:x}\nstatic_memory_start_location: {:x}\n",
                                    self.version,
                                    &flags_string,
                                    self.hi_memory_start,
                                    self.pc_start,
                                    self.dictionary_location,
                                    self.object_table_location,
                                    self.global_vars_table_location,
                                    self.static_memory_start_location);

        debug

    }
}

// just one enum for now
pub enum HeaderFlags {
    V1 { flags: HeaderFlagsV1 },
}

impl HeaderFlags {
    fn process_header(view: &MemoryView, version: u8) -> HeaderFlags {

        match version {
            1...3 => HeaderFlags::process_v1_header(view),
            _ => panic!("This interpreter only supports up to ZMachine version 3 at this time."),
        }

    }

    fn process_v1_header(view: &MemoryView) -> HeaderFlags {

        let flag_byte = view.read_at(0x1);

        HeaderFlags::V1 {
            flags: HeaderFlagsV1 {
                // first bit, if set, means we use the
                // hours status line instead of score
                status_line: match flag_byte & 0x1 {
                    0x1 => StatusLineType::Hours,
                    // this is 0 and the default
                    _ => StatusLineType::Score,
                },

                // bit 2 of byte 1
                split_story: flag_byte & 0x2 > 0,
                // go ahead, skip a bit and fuck with everyones head
                // bit 4 of byte 1
                show_status_line: flag_byte & 0x4 > 0,
                // bit 5 of byte 1
                split_screen: flag_byte & 0x5 > 0,
                // bit 6 of byte 1
                variable_pitch_font: flag_byte & 0x6 > 0,
            },
        }

    }
}

// there are two versions of header flags, 1-3 has one version,
// 4+ has another
//
// the one in 4+ makes a lot more sense, im not sure how many
// of these will be used in our interpretation ( they seem
// to be based around older architectures )

pub struct HeaderFlagsV1 {
    pub status_line: StatusLineType,

    // is this split across two disks? ( this is not used )
    pub split_story: bool,

    // is the status line available( the real option is is it NOT available,
    // but, you know, thinking in positives is good
    pub show_status_line: bool,

    // screen splitting available
    pub split_screen: bool,

    // do we use a variable pitch font by default
    pub variable_pitch_font: bool,
}

pub enum StatusLineType {
    Score,
    Hours,
}
