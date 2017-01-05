pub mod opcode;
pub mod instruction_set;

// represents the current zmachine
use super::header::*;
use self::opcode::*;
use super::memory_view::*;
use super::object_view::*;

use std::rc::*;
use std::cell::RefCell;

// wraps a Vec with some other information
pub struct Stack {
    // this holds the top of the last frame,
    // this is important because we tuck addresses under here
    //
    // we don't have a "current pointer" because the stack
    // is going to grow until our system just can't take it anymore,
    // and current pointer will always be stack.len()-1
    top_of_frame: usize,
    pub stack: Vec<u16>,
}

impl Stack {
    // strictly speaking, can only be 1..16
    // gotta be careful here because you would normally think you would have
    // to offset by index because of where top_of_frame is, but as it turns
    // out, starting out with "1" prevents that
    pub fn get_local_variable(&self, num: u8) -> u16 {

        // here we will cast because i do want some restrictions
        // around get local variable
        let offset = num as usize;
        let index = self.top_of_frame + offset;

        //println!("num is: {}", num);
        //println!("getting index: {}", index);

        self.stack[index as usize]

    }

    pub fn store_local_variable(&mut self, num: u8, value: u16) {
        let offset = num as usize;
        let index = self.top_of_frame + offset;
        self.stack[index as usize] = value;
    }

    pub fn switch_to_new_frame(&mut self) {

        // the stack will never exceed 64,000 entries - i believe
        // the recommended # given by infocom is somewhere in the hundreds
        // the total stack size wont exceed 1024 entries,
        // or about ~16k
        self.stack.push(self.top_of_frame as u16);
        //println!("just pushed top of frame:{}", self.top_of_frame);
        self.top_of_frame = self.top_of_stack();

    }

    pub fn restore_last_frame(&mut self) {

        // dump everything after the top of the frame
        self.stack.truncate(self.top_of_frame + 1);

        // restore the top of the frame ( hidden )
        self.top_of_frame = match self.stack.pop() {
            Some(frame) => frame as usize,
            _ => panic!("restoring last frame resulted in stack underflow!"),
        };

        //println!("top of frame:{}", self.top_of_frame);
    }

    pub fn top_of_stack(&self) -> usize {
        self.stack.len() - 1
    }
}

pub struct ZMachine {
    // the call stack, which are 2-byte words (u16)
    //
    // this also mixes in the local stack,
    // more like a traditional implementation ,
    // where we push pointer values to the top
    // of the stack frame before moving to the next ip,
    // and pop them to return
    //
    // the zmachine spec says we don't have to implement it like this;
    // we can actually have a stack of addresses in memory and also
    // a seperate memory stack if we want ( they are distinct concepts to
    // interface with in the ZMachine ), but almost everyone and
    // their mother implements it this way because its straightforward
    // and mirrors "actual" stack frames. whatever that means.
    pub call_stack: Stack,

    // the header, which actually reads the first 64 bytes in memory
    // everyone has access to it, its mostly configuration stuff
    // and version info
    pub header: Header,

    // ALL of the memory, this represents the entire state of the machine
    // this is loaded in at first , then modified by save files, then
    // the game is run and dynamic memory then changes during play
    //
    // its broken into three parts: dynamic, static, and high.
    //
    // dynamic: all things that can change in game, including object trees
    // and inventory
    //
    // static: this contains grammar, actions, preactions, adjectives, and
    // the dictionary- basically defining the language of the game
    //
    // high: routines and static strings meant to be used by the machine
    //
    // the machine owns a reference to the memory, and typically
    // is the only person who asks for a mutable reference
    memory: Rc<RefCell<Vec<u8>>>,

    // the stack pointer/program counter, technically this can be 0-512k,
    // closest representation is u32
    //
    // note that this is one of the very few 'u32' things here
    ip: u32,

    // are we still running? keep processing.
    pub running: bool,
}

impl ZMachine {
    pub fn new(data: Vec<u8>) -> ZMachine {

        // we have to create an immutably reference
        // counted mutable reference in order to
        //
        // allow the parent to write and the children to
        // read from the same memory, since they both effectively
        // own it, this gives the memory over to a ref cell
        // which is wrapped by an rc ( making the ref cell immutable ),
        // and then passed around. the ref cell can only be accessed
        // read-only, and the inner vector data can only be mutably
        // borrowed if no one else is currently borrowing it;
        //
        // this means rust will not protect us from one
        // thread borrowing a mut when another borrows it immutably!
        //
        // we should be able to avoid this, however, by simply
        // using immutable calls in the child, and mutable calls
        // only in the parent

        let memory = Rc::new(RefCell::new(data));

        // we are going to give the reference to the header,
        // so it can read it

        let header = Header::create(memory.clone());

        // we have to copy it here before we move
        // if this were not a u16( or a type that didn't implement Copy ),
        // we would get an error later if we tried to access header.pc_start,
        // but since it implements the Copy trait, we are actually just copying
        // the value

        // note that pc_start is a u16, but our pointer is a u32. this is because

        let pc_start = header.pc_start as u32;

        ZMachine {
            call_stack: Stack {
                top_of_frame: 0,
                stack: Vec::new(),
            },
            header: header,
            ip: pc_start,
            memory: memory,
            running: true,
        }

    }

    pub fn get_version(&self) -> u8 {
        self.header.version
    }

    pub fn get_abbreviations_view(&self) -> MemoryView {
        MemoryView {
            memory: self.memory.clone(),

            // note this will only be accurate per-instruction;
            // don't try to use the old instructions memory view
            // to pass to opcodes
            pointer: self.header.abbreviations_location as u32,
        }
    }
    // gets a view into the current program
    // stack
    pub fn get_frame_view(&self) -> MemoryView {
        MemoryView {
            memory: self.memory.clone(),

            // note this will only be accurate per-instruction;
            // don't try to use the old instructions memory view
            // to pass to opcodes
            pointer: self.ip,
        }
    }

    pub fn get_global_variables_view(&self) -> MemoryView {
        MemoryView {
            memory: self.memory.clone(),

            //this will be accurate for the lifetime of the program
            //we cast here; it wont be of any consequence because
            //while the memory/pointer is represented by s u32,
            //the global variables are in the lower half of memory
            //represented by a u16
            //
            //( basically, the addresses are all two byte words but are multiplied
            //by 2 to access "high memory", or non dynamic memory" )
            pointer: self.header.global_vars_table_location as u32,
        }
    }

    pub fn get_object_view(&self, object_id: u16) -> ObjectView {

        // we will have to change the values for this in the future when we support
        // newer versions of the ZMachine ( particularly version 4 )

        // we should really be getting this from the header - ill save it for a future
        // refactor, minor issue right now
        let object_length = 9;
        let property_defaults_length = 62;

        // calculate offset and object location
        let offset = ((object_id as u32 - 1) * object_length);
        let object_location =
            self.header.object_table_location as u32 + property_defaults_length as u32 + offset;


        ObjectView {
            attributes_length: 4,
            defaults_view: MemoryView {
                memory: self.memory.clone(),
                pointer: self.header.object_table_location as u32,
            },
            view: MemoryView {
                memory: self.memory.clone(),

                // this should be accurate for the lifetime of the
                // program - i believe the tables interiors may be
                // changed but the boundries cannot be overridden, these
                // are set by the compiler, i think in inform you "declare"
                // the size of properties ahead of time
                pointer: object_location,
            },
            // 3 relatives, 1 byte each
            related_obj_length: 1,
        }

    }

    // the memory view for the whole env
    pub fn get_memory_view(&self) -> MemoryView {
        MemoryView {
            memory: self.memory.clone(),
            // the start of memory
            pointer: 0,
        }
    }

    pub fn next_instruction(&mut self) {

        //println!("next instruction! pointer: {:x}", self.ip);

        // a non-mutable memory view,
        // reads from the same memory as zmachine
        let view = self.get_frame_view();
        let globals = self.get_global_variables_view();

        // the top two bytes of the instruction
        // will give all of the information needed for the instruction;
        //
        // note that not all instructions use the top two bytes

        let word = view.peek_at_instruction();
        //println!("raw word: {:x}", word[0]);
        //println!("raw word: {:x}", word[1]);

        let mut op_code = OpCode::form_opcode(word);

        // we get a mutable reference to the call stack
        // because variables can augment them
        //
        // we do this in its own scope to drop the mutable
        // reference after we are done
        {
            let stack = &mut self.call_stack;
            // have the view.
            op_code.read_variables(view, globals, stack);
            println!("{:x}", self.ip);
            println!("{}", op_code);
        }

        op_code.execute(self);

        // technically, store and branch cannot happen at the same time
        // i will not make any enforcement here because the zmachine makes no such
        // requirement of an interpreter, but given how the opcodes are defined,
        // that never happens.

        if op_code.store {

            // we have to make a new view, here,
            // because we could have changed the pointer
            // after execute

            let view = self.get_frame_view();
            let destination = view.read_at_head(op_code.read_bytes);
            self.store_variable(destination, op_code.result);
            op_code.read_bytes += 1

        }

        // if the op code branched or branches,
        // we rely on the op to set the ip,
        // otherwise we just increment it

        match op_code.branch {
            true => {

                let condition = op_code.result;
                let true_mask = 0b10000000;
                let view = self.get_frame_view();
                let branch_on_true = (view.read_at_head(op_code.read_bytes) & true_mask) ==
                                     true_mask;

                // we branch when the value is non-zero;
                // this is helpful for get child and other branches which
                // also return values

                let branch = (branch_on_true && condition > 0) ||
                             (!branch_on_true && condition == 0);

                if (branch) {

                    let two_bits_mask = 0b01000000;
                    let one_bit = (view.read_at_head(op_code.read_bytes) & two_bits_mask) ==
                                  two_bits_mask;

                    let mut offset: u16 = if (one_bit) {
                        (view.read_at_head(op_code.read_bytes) & 0b00111111) as u16
                    } else {
                        view.read_u16_at_head(op_code.read_bytes) & 0b0011111111111111
                    };

                    self.ip = self.ip + offset as u32;
                } else {
                    self.ip += op_code.read_bytes + 1;
                }

            }
            false => self.ip += op_code.read_bytes,
        }


        // self.running = false;

    }

    //this JUST reads a variable, but does not modify the stack in any way
    //its different from the opcode functions, which we may merge into zmachine,
    //or may not
    pub fn read_variable(&self, address: u8) -> u16 {
        match address {
            // 0, its the stack, pop it and return
            0 => self.call_stack.stack[self.call_stack.stack.len() -1],
            // 1 to 15, its a local
            i @ 0x01...0x0f => self.call_stack.get_local_variable(i),
            // 16 to 255, it's a global variable.
            global @ 0x10...0xff => self.
                                      get_global_variables_view().
                                      read_u16_at_head(global as u32),
            _ => unreachable!(),
        }
    }

    //this writes a variable in place - it really only specializes on the stack,
    //otherwise it wraps store_variable
    pub fn write_variable_in_place(&mut self, address: u8, value: u16) {
        match address {
            0 => {
                let last = self.call_stack.stack.len()-1;
                self.call_stack.stack[last] = value;
            }
            _ => self.store_variable(address, value),
        }
    }

    // the machine always stores variables during or at the end of instruction calls,
    // and accesses variables before processing the call;
    pub fn store_variable(&mut self, address: u8, value: u16) {
        match address {
            0 => self.call_stack.stack.push(value),
            index @ 0x01...0x0f => self.call_stack.store_local_variable(index - 1, value),
            index @ 0x10...0xff => {
                self.get_global_variables_view()
                    .write_u16_at_head((index as u32 - 1) * 2, value)
            }
            _ => unreachable!(),
        }
    }


}
