extern crate rand;
use self::rand::*;

use super::super::interfaces::zinterface::*;
use super::opcode::*;

// represents the current zmachine
use super::global_variables_view::*;
use super::header::*;
use super::memory_view::*;
use super::object_view::*;

use std::cell::*;
use std::rc::*;

// once this is a FnMut or FnOnce, I don't think we
// can clone it anymore.
#[derive(Clone)]
pub enum MachineState {
    Stopped,
    Restarting,
    Running,
    // input finished takes ownership of the string
    TakingInput { callback: Rc<dyn Fn(String)> },
}

pub struct RandomGen<T> {
    generator: T,
    pub randoms_predictable: bool,
    pub randoms_predictable_next: u16,
    pub random_seed: u16,
}

impl<T: rand::SeedableRng<[u32; 4]>> RandomGen<T> {
    pub fn seed(&mut self, value: u16) {
        self.random_seed = value;

        //if seed < 1000, we set the generator into predictable mode,
        //and begin counting up from 1
        self.randoms_predictable = value < 1000;

        if self.randoms_predictable {
            self.randoms_predictable_next = 1;
        } else {
            let seed = match value {
                // if s is 0, all future random numbers will be "really random"
                // so we just generate a random number generator at random
                0 => [
                    rand::thread_rng().gen::<u32>(),
                    rand::thread_rng().gen::<u32>(),
                    rand::thread_rng().gen::<u32>(),
                    rand::thread_rng().gen::<u32>(),
                ],
                _ => {
                    //else, use the seed to make a random number generator,
                    //such that the same seed will generate the same #s every time
                    let val = value as u32;
                    //overflow is fine
                    [val, val + 1, val + 2, val + 3]
                }
            };

            self.generator = T::from_seed(seed);
        }
    }

    pub fn next(&mut self, range: u16) -> u16 {
        if self.randoms_predictable {
            self.randoms_predictable_next += 1;

            if self.randoms_predictable_next > range {
                self.randoms_predictable_next = 1;
            }

            self.randoms_predictable_next
        } else {
            //bits will be lost, but its random
            //note that zmachine is actually inclusive in its range,
            //and, interestingly, no random may be 0..

            self.generator.gen_range(1, range + 1)
        }
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
    top_of_frame: usize,
    pub stack: Vec<u16>,
}

impl Stack {
    // strictly speaking, can only be 1..15 ( 14 total )
    // gotta be careful here because you would normally think you would have
    // to offset by index because of where top_of_frame is, but as it turns
    // out, starting out with "1" prevents that
    pub fn get_local_variable(&self, num: u8) -> u16 {
        // here we will cast because i do want some restrictions
        // around get local variable
        let offset = num as usize;
        let index = self.top_of_frame + offset;

        // println!("num is: {}", num);
        // println!("getting index: {}", index);

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
        // println!("just pushed top of frame:{}", self.top_of_frame);
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

        // println!("top of frame:{}", self.top_of_frame);
    }

    pub fn top_of_stack(&self) -> usize {
        self.stack.len() - 1
    }
}

pub struct ZMachine<T: ZInterface> {
    // the call stack, which are 2-byte words (u16)
    //
    // this also mixes in the local stack,
    // more like a traditional implementation ,
    // where we push pointer values to the top
    // of the stack frame before moving to the next ip,
    // and pop them to return
    //
    // the ZMachine spec says we don't have to implement it like this;
    // we can actually have a stack of addresses in memory and also
    // a separate memory stack if we want ( they are distinct concepts to
    // interface with in the ZMachine ), but almost everyone and
    // implements it this way because its straightforward
    // and mirrors "actual" stack frames.
    pub call_stack: Stack,

    // the header, which actually reads the first 64 bytes in memory
    // everyone has access to it, its mostly configuration stuff
    // and version info
    pub header: Header,

    // the interface to the GUI, so ZMachine can print/take input
    pub zinterface: Rc<T>,

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
    pub memory: Rc<RefCell<Vec<u8>>>,

    // the stack pointer/program counter, technically this can be 0-512k,
    // closest representation is u32
    //
    // note that this is one of the very few 'u32' things here
    pub ip: u32,

    //we use XorShiftRng because there are no real security concerns here
    pub random_generator: RandomGen<XorShiftRng>,

    // are we still running? keep processing.
    pub state: MachineState,
}

impl<T: ZInterface> ZMachine<T> {
    //creates a new zmachine from the data given
    pub fn new(data: Vec<u8>, interface: T) -> ZMachine<T> {
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
        let interface = Rc::new(interface);

        let machine = ZMachine::<T> {
            call_stack: Stack {
                top_of_frame: 0,
                stack: Vec::new(),
            },
            header: header,
            ip: pc_start,
            memory: memory,
            zinterface: interface,
            random_generator: RandomGen {
                generator: XorShiftRng::from_seed([
                    rand::thread_rng().gen::<u32>(),
                    rand::thread_rng().gen::<u32>(),
                    rand::thread_rng().gen::<u32>(),
                    rand::thread_rng().gen::<u32>(),
                ]),
                random_seed: 1,
                randoms_predictable: false,
                randoms_predictable_next: 1,
            },
            state: MachineState::Running,
        };

        //does nothing in desktop
        machine
    }

    //anyone can read this, just not mut/set it
    pub fn current_ip(&self) -> u32 {
        self.ip
    }

    //actually executes the instruction
    fn execute_instruction(&mut self, op_code: &mut OpCode<T>) {
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
                self.handle_branch(op_code);
            }
            false => {
                // println!("code does not branch");
                self.ip += op_code.read_bytes;
            }
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

    pub fn get_dictionary_view(&self) -> MemoryView {
        MemoryView {
            memory: self.memory.clone(),
            pointer: self.header.dictionary_location as u32,
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

    pub fn get_global_variables_view(&self) -> GlobalVariablesView {
        GlobalVariablesView {
            view: MemoryView {
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
            },
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

    // object id is a u16 because in future versions, there can be up
    // to 65k objects. id rather standardize that ahead of time because
    // it will be all over the instruction set
    pub fn get_object_view(&self, object_id: u16) -> ObjectView {
        // we will have to change the values for this in the future when we support
        // newer versions of the ZMachine ( particularly version 4 )

        // we should really be getting this from the header - ill save it 
        // for when we move to version 4+ (that's when this changes)
        let object_length = 9;
        let property_defaults_length = 62;

        // calculate offset and object location
        let offset = (object_id as u32 - 1) * object_length;

        let object_location =
            self.header.object_table_location as u32 + property_defaults_length as u32 + offset;

        ObjectView {
            object_id: object_id,
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

    // handle a branch opcode - this happens after instructions are executed
    pub fn handle_branch(&mut self, op_code: &mut OpCode<T>) {
        let view = self.get_frame_view();
        let condition = op_code.result;
        let true_mask = 0b10000000;
        let branch_on_true = (view.read_at_head(op_code.read_bytes) & true_mask) != 0;

        let two_bits_mask = 0b01000000;
        let one_bit = (view.read_at_head(op_code.read_bytes) & two_bits_mask) != 0;

        // we branch when the value is non-zero;
        // this is helpful for get child and other branches which
        // also return values
        let branch = (branch_on_true && condition > 0) || (!branch_on_true && condition == 0);

        // branch byte offset
        let branch_byte_offset = match one_bit {
            true => 1,
            false => 2,
        };

        if branch {
            let offset: (bool, i16) = match one_bit {
                // we have to mask against the control bits, here
                //
                true => {
                    // this should still be a positive # since we do not prop the bytes
                    // it can be from 0 to 63
                    (
                        true,
                        (view.read_at_head(op_code.read_bytes) & 0b00111111) as i16,
                    )
                }
                false => {
                    let mut fourteen_bit =
                        view.read_u16_at_head(op_code.read_bytes) & 0b0011111111111111;

                    //println!( "fourteen bit is:{:b}", fourteen_bit );

                    if fourteen_bit & 0x2000 != 0 {
                        // propagate the sign
                        fourteen_bit = fourteen_bit | (1 << 15);
                        fourteen_bit = fourteen_bit | (1 << 14);
                    }

                    //println!( "corrected fourteen bit is:{}", fourteen_bit );
                    //println!( "converted fourteen bit is:{}", fourteen_bit as i16 ){

                    (false, fourteen_bit as i16)
                }
            };

            match offset {
                // the below only applies when the branch form is one byte - i mistakenly
                // assumed that 0 or 1 would be encoded as two bytes, which, in retrospect,
                // does not make much sense ( although im not sure why it wouldn't be allowed -
                // it does actually seem like this introduces a control dependency ).
                //
                // in the case of 1 or 0, we return true or false from the function,
                // which is actually "in" the instruction set,
                // so this part is a little messy;
                //
                // we could move "return" up to the zmachine, and away from call/ret,
                // but for now we will just get it done - its high time for zork
                //
                // this works because return does not actually let zmachine
                // increment the code, so by calling it here, it modifies the ip for
                // us and at the end, we should be in the right spot
                (true, 0) => {
                    let mut rfalse = OpCode::form_rfalse();
                    //println!("returning from branch false");
                    self.execute_instruction(&mut rfalse);
                }

                (true, 1) => {
                    let mut rtrue = OpCode::form_rtrue();
                    //println!("returning from branch true");
                    self.execute_instruction(&mut rtrue);
                }

                // branch address is defined as "address after branch data",
                // or self.ip + op_code.read_bytes + offset
                // "-2, + branch offset"
                // not entirely sure why they felt the -2 was necessary?
                // maybe it makes sense in inform syntax
                (_, x) => {
                    //println!("read at:{:x}", view.pointer + op_code.read_bytes);
                    //println!("diff was:{}", x){

                    let difference =
                        (op_code.read_bytes as i16) + x + (branch_byte_offset as i16) - 2;

                    self.ip = ((self.ip as i32) + (difference as i32)) as u32;

                    //println!("branching to :{:x}", self.ip);
                }
            }
        } else {
            let difference = op_code.read_bytes + branch_byte_offset;
            self.ip += difference;

            //print!("branch failed, moving to : ");
        }
    }

    //grabs the next instruction and executes it
    pub fn next_instruction(&mut self) {
        // a non-mutable memory view,
        // reads from the same memory as zmachine
        let view = self.get_frame_view();
        let globals = self.get_global_variables_view();

        // the top two bytes of the instruction
        // will give all of the information needed for the instruction;
        //
        // note that not all instructions use the top two bytes

        let word = view.peek_at_instruction();
        let mut op_code = OpCode::form_opcode(word);

        op_code.ip = self.ip;
        // println!( "ip: {:x}", op_code.ip );

        {
            let code_ref = &mut op_code;
            OpCode::assign_instruction(code_ref);
        }

        // we get a mutable reference to the call stack
        // because variables can augment them
        //
        // we do this in its own scope to drop the mutable
        // reference after we are done
        {
            let stack = &mut self.call_stack;

            // have the view.
            op_code.read_variables(view, globals, stack);
        }

        self.execute_instruction(&mut op_code);
    }

    // this JUST reads a variable, but does not modify the stack in any way
    // its different from the opcode functions, which we may merge into zmachine,
    // or may not

    // i thought this would be non-mutating at first but as it turns out, the
    // stack (or stack pointer) is mutated by reading the stack pointer
    pub fn read_variable(&mut self, address: u8) -> u16 {
        match address {
            // so it turns out that the stack does "pop" the value
            // in most z-machine interpreters, but what is different is
            // that the value is not deleted - it remains in the array

            // its possible some programs used "compiler wizardry"
            // to optimize some things to access data that was already "popped"

            // in that case I may not have much of an option but to re-think
            // the data store behind the stack, i might need to switch to Array
            // w/ a pointer

            // 0, its the stack, pop it and return
            0 => match self.call_stack.stack.pop() {
                Some(value) => value,
                None => panic!("stack underflow!"),
            },
            // 1 to 15, its a local
            i @ 0x01..=0x0f => self.call_stack.get_local_variable(i),
            // 16 to 255, it's a global variable.
            global @ 0x10..=0xff => {
                let index = global - 0x10;
                self.get_global_variables_view().read_global(index as u16)
            }
        }
    }

    // the machine always stores variables during or at the end of instruction calls,
    // and accesses variables before processing the call;
    pub fn store_variable(&mut self, address: u8, value: u16) {
        // println!( "storing: {} at {}", value, address );

        match address {
            0 => self.call_stack.stack.push(value),
            index @ 0x01..=0x0f => self.call_stack.store_local_variable(index, value),
            global @ 0x10..=0xff => {
                // offset by 16 to get the global "index"
                let index = global - 0x10;
                self.get_global_variables_view()
                    .write_global(index as u16, value);
            }
        }
    }
    // wait for input, and on input, hand it to whatever code/op was waiting
    // for it
    pub fn wait_for_input(&mut self, callback: Rc<dyn Fn(String)>) {
        let mut buf = String::new();
        let result = match self.zinterface.read_next_line(&mut buf) {
            Some(_) => {
                callback(buf.to_string());
                true
            }
            _ => false,
        };

        if result {
            self.state = MachineState::Running;
        }
    }

    // this writes a variable in place - it really only specializes on the stack,
    // otherwise it wraps store_variable
    pub fn write_variable_in_place(&mut self, address: u8, value: u16) {
        match address {
            0 => {
                let last = self.call_stack.stack.len() - 1;
                self.call_stack.stack[last] = value;
            }
            _ => self.store_variable(address, value),
        }
    }
}
