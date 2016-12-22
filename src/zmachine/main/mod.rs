pub mod opcode;

// represents the current zmachine
use super::header::*;
use self::opcode::*;

use std::rc::*;
use std::cell::RefCell;

// a "view" into environment memory. mostly used to split off the memory
// from zmachine so we can give the memory and stack in different states
// immutable/mutable )

pub struct MemoryView {
    memory: Rc<RefCell<Vec<u8>>>,
    // the ip at the time this view was created -
    // note this is only valid
    ip: u16,
}

impl MemoryView {
    pub fn read_at_offset(&self, offset: u16) -> u8 {
        let address = self.program_offset(offset);
        self.memory.borrow()[address]
    }

    pub fn read_at_offset_from_head(&self, offset: u16) -> u8 {
        self.read_at_offset(self.ip + offset)
    }

    pub fn read_u16_at_offset(&self, offset: u16) -> u16 {
        let result = (self.read_at_offset(offset) as u16) << 8 |
                     self.read_at_offset(offset + 1) as u16;
        result
    }

    pub fn read_u16_at_offset_from_head(&self, offset: u16) -> u16 {
        self.read_u16_at_offset(self.ip + offset)
    }

    fn program_offset(&self, offset: u16) -> usize {
        (self.ip + (offset)) as usize
    }
}

// wraps a Vec with some other information
pub struct Stack {
    // this holds the top of the last frame,
    // this is important because we tuck addresses under here
    //
    // we don't have a "current pointer" because the stack
    // is going to grow until our system just can't take it anymore,
    // and current pointer will always be stack.len()-1
    top_of_frame: u16,
    pub stack: Vec<u16>,
}

impl Stack {
    // strictly speaking, can only be 0..15
    pub fn get_local_variable(&self, num: u8) -> u16 {

        // here we will cast because i do want some restrictions
        // around get local variable

        let offset = num as u16;
        let index = self.top_of_frame + offset + 1;
        self.stack[index as usize]

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
    // closest representation is u16
    ip: u16,

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

        let pc_start = header.pc_start;

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

    pub fn get_view(&self) -> MemoryView {
        MemoryView {
            memory: self.memory.clone(),

            // note this will only be accurate per-instruction;
            // don't try to use the old instructions memory view
            // to pass to opcodes
            ip: self.ip,
        }
    }

    pub fn next_instruction(&mut self) {

        let op_id = self.peek_at_instruction();

        let mut opCode = OpCode::form_opcode(op_id);

        let stack = &mut self.call_stack;
        let view = self.get_view();

        // have the view.
        opCode.read_variables(view, stack);

        println!("{}", opCode);

        self.running = false;

    }

    // this peeks at the top of the stack and copies the first two bytes
    // into an array, and returns
    fn peek_at_instruction(&self) -> [u8; 2] {
        // these are u8s, so they are copied
        let x = [self.read_at_offset(0), self.read_at_offset(1)];
        x
    }
}
