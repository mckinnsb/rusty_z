// opcode struct

use std::fmt;
use super::instruction_set;
use super::super::memory_view::*;
use super::ZMachine;
use super::Stack;

// the "form" of the opcode, which dictates how the first byte(s) are read,
//
// if short or long, there is one byte, if variable, two ( in later versions
// of zmachine, this becomes 2-4, hence the name variable, and a new form
// extended is added, which we do not deal with here yet ( but is somewhat similar to
// variable )

pub enum OpForm {
    Short,
    Long,
    Variable,
}

impl fmt::Display for OpForm {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "{}",
               match self {
                   &OpForm::Short => "Short",
                   &OpForm::Long => "Long",
                   &OpForm::Variable => "Variable",
               })
    }
}

// the "operand" of the opcodes are a,b in a+b ( obviously ),
// but the operand type determines how large it is, and if it
// is an address or a raw value
//
// large constants are two bytes, everything is is 1 byte
//
// note that in the zmachine spec "omitted altogether" is an allowed
// type, but we do not use it here, because we use this as an array
// of types to pull after the opcode

pub enum Operand {
    LargeConstant { value: u16 },
    // the address itself is a byte, but the value is a u16,
    // we evaluate it before we store it here
    Variable { value: u16 },
    SmallConstant { value: u8 },
    Omitted,
}

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {

        let formatted = match self {
            &Operand::LargeConstant { value } => format!("Large Constant: {:x}", value),
            &Operand::SmallConstant { value } => format!("Small Constant: {:x}", value),
            &Operand::Variable { value } => format!("Variable : {:x}", value),
            &Operand::Omitted => format!("Omitted"),
        };

        write!(f, "{}", formatted)

    }
}

impl Operand {
    // it should be noted that some operands can be encoded as 8 bytes by the compiler,
    // but for the most part, they will be stored as u16s either a) in the stack or b)
    // as a global variable. im not sure if all interpreters or any interpreter
    // actually bothers to write small constants to the local call frame (instead of on the stack),
    // or bothers implementing a stack in bytes just to accomodate one data type

    pub fn get_value(&self) -> u16 {
        match self {
            // not 100% if this is a good idea at this point
            &Operand::Omitted => panic!("tried to get the value of an omitted operand!"),
            &Operand::SmallConstant { value } => value as u16,
            &Operand::LargeConstant { value } |
            &Operand::Variable { value } => value,
        }
    }
}

pub struct OpCode {
    // the actual opcode, this is determined
    pub code: u8,

    // the "form" of this opcode, or how it encodes the first byte(s)
    pub form: OpForm,

    // a list of the operands. when initialized, it will
    // be an array of Omitted{}, then the opcode will figure out
    // what it is, change the structs to what is necessary,
    // and then when asked, pull the right information from the buffer
    pub operands: [Operand; 4],

    // the # of operands present, 0..3
    // the operands field is always a 4 byte
    // array to make things simple ( we don't need
    // dynamic sizing ), this lets us know how many
    // to pull before realizing the rest will be Omitted
    pub operand_count: u8, /* the instruction itself
                            * pub instruction: OpType, */

    // does this code jump
    //
    // note that jumping does nothing to the stack, so its possible previous
    // state will get wiped out of it jumps to another routine (this is sometimes
    // desired, like when you die)
    pub branch: bool,

    // does this code store a value?
    pub store: bool,

    // does this code print something?
    pub print: bool,

    // does this code pause for input?
    pub input: bool,

    // this is the actual opcode instruction, its hidden behind "execute"
    instruction: fn(&mut OpCode, &mut ZMachine),

    // how many bytes have we read until the instruction is executed
    // ( and the stack pointer potentially changes )?
    //
    // we return this back to the zmachine so it can increment its PC after
    // this intstruction is executed, but before the stack frame changes.
    //
    // this value critically refers to where any stored variable/branch will
    // be when any calling function returns
    //
    // note that this occurs at the end of an instruction finishing,
    // so things like "call" actually finish before a a subroutine executes.
    //
    // the "local stack pointer" stays "mid-way" through the call, until the
    // routine returns, at which point the rest of the call's opcode is read
    // ( and the ip is incremented ), depending on if there is anything to
    // store or branch ( im not sure if there are void calls? )
    //
    // we probbbbably don't need all this memory, but
    // 1) it is to avoid unnecessary casts
    // and 2) technically speaking it could be legal, you could have an inform
    // script that is just one MASSIVE print command
    pub read_bytes: u32,

    // the result of the operation - if this was
    // a branch operation, 0 is false, 1 is true
    pub result: u16,
}

impl OpCode {
    fn assign_instruction(code: &mut OpCode) {

        // the form is an object that does not copy, so we need a reference
        // to it
        // behold, the match to rule them all
        //
        // not sure if this is better nested, or split apart
        // at all
        let instruction = match (&code.form, code.operand_count, code.code) {
            (&OpForm::Long, _, 0x1) => instruction_set::je,
            (&OpForm::Long, _, 0x2) => instruction_set::jl,
            (&OpForm::Long, _, 0x3) => instruction_set::jg,
            (&OpForm::Long, _, 0x4) => instruction_set::dec_chk,
            (&OpForm::Long, _, 0x5) => instruction_set::inc_chk,
            (&OpForm::Long, _, 0x6) => instruction_set::jin,
            (&OpForm::Long, _, 0x7) => instruction_set::test,
            (&OpForm::Long, _, 0x8) => instruction_set::or,
            (&OpForm::Long, _, 0x9) => instruction_set::and,
            (&OpForm::Long, _, 0xA) => instruction_set::test_attr,
            (&OpForm::Long, _, 0xB) => instruction_set::set_attr,
            (&OpForm::Long, _, 0xC) => instruction_set::clear_attr,
            (&OpForm::Long, _, 0xD) => instruction_set::store,
            (&OpForm::Long, _, 0xE) => instruction_set::insert_obj,
            (&OpForm::Long, _, 0xF) => instruction_set::loadw,
            (&OpForm::Long, _, 0x10) => instruction_set::loadb,
            (&OpForm::Long, _, 0x11) => instruction_set::get_prop,
            (&OpForm::Long, _, 0x12) => instruction_set::get_prop_addr,
            (&OpForm::Long, _, 0x13) => instruction_set::get_next_prop,
            (&OpForm::Long, _, 0x14) => instruction_set::add,
            (&OpForm::Long, _, 0x15) => instruction_set::sub,
            (&OpForm::Long, _, 0x16) => instruction_set::mul,
            (&OpForm::Long, _, 0x17) => instruction_set::div,
            (&OpForm::Long, _, 0x18) => instruction_set::mod_fn,
            (&OpForm::Short, 1, 0xB) => instruction_set::ret,
            (&OpForm::Variable, _, 0x0) => instruction_set::call,
            (&OpForm::Variable, _, 0x1) => instruction_set::storew,
            (&OpForm::Variable, _, 0x3) => instruction_set::put_prop,
            _ => panic!("Instruction not found!"),
        };

        code.instruction = instruction;

    }

    pub fn execute(&mut self, env: &mut ZMachine) {
        (self.instruction)(self, env);
    }

    // opcode can be several bytes long, but in the
    // form section we always allow the function to peek
    // at the top two bytes of the program stack
    //
    // sometimes variable will need the second one,
    // and we trust the opcode itself, since the length is variable,
    // to move the pc

    pub fn form_opcode(word: [u8; 2]) -> OpCode {

        // set some defaults and do stuff we will have to do anyway,
        // like filling out the operands table

        let mut op_code: OpCode = OpCode {
            code: 0,
            branch: false,
            store: false,
            print: false,
            input: false,
            instruction: OpCode::null_instruction,
            form: OpForm::Short,
            operands: [Operand::Omitted {},
                       Operand::Omitted {},
                       Operand::Omitted {},
                       Operand::Omitted {}],
            operand_count: 0,
            read_bytes: 0,
            result: 0,
        };


        // make a closure here to let rust know when we want to drop
        // the mutable reference
        {
            // borrow a mutable reference to form the rest of the code

            let code_ref = &mut op_code;

            // the top byte of the instruction
            // while not giving the exact opcode
            // were designed to 'mark' aspects of the
            // opcode, such as what form it is and
            // how many variables it takes

            match word[0] {
                // here, id is matched as the first byte, so we can access the opcode
                0x00...0x7f => OpCode::form_long_opcode(code_ref, word[0]),
                // the fallthrough for be , the code for extended opcodes,
                // falls through in form_short_opcode
                0x80...0xbf => OpCode::form_short_opcode(code_ref, word[0]),
                0xc0...0xff => OpCode::form_variable_opcode(code_ref, word[0], word[1]),
                // this should not be reachable
                _ => unreachable!(),
            }
        }

        {
            let code_ref = &mut op_code;
            OpCode::assign_instruction(code_ref);
        }
        // since we dropped the mutable reference, you can have it now,
        // caller
        op_code

    }

    // each of these functions has a match block which basically sets
    // up the "form" of their operands, but doesn't set the values, that
    // happens later, this just sets up basic information about it determined
    // from the first few bits, which is necessary to process the information
    //
    // it's a two-step opcode process, and one of the reasons why its a VM )
    //
    // this expects a "shell" opcode, which it modifies

    fn form_long_opcode(code: &mut OpCode, id: u8) {

        // we include the header byte now
        code.read_bytes = 1;
        code.form = OpForm::Long;

        // all long opcodes have two operands. lucky us
        code.operand_count = 2;
        // mask out the top three bits, and we have our opcode
        // (bottom five bits)
        code.code = id & 0b00011111;

        match id {
            0x00...0x1f => {
                code.operands[0] = Operand::SmallConstant { value: 0 };
                code.operands[1] = Operand::SmallConstant { value: 0 };
            }
            0x20...0x3f => {
                code.operands[0] = Operand::SmallConstant { value: 0 };
                // should note that writing to "0" is writing to the stack
                // pointer, so the default is to pull a variable from the stack
                code.operands[1] = Operand::Variable { value: 0 };
            }
            0x40...0x5f => {
                code.operands[0] = Operand::Variable { value: 0 };
                code.operands[1] = Operand::SmallConstant { value: 0 };
            }
            0x60...0x7f => {
                code.operands[0] = Operand::Variable { value: 0 };
                code.operands[1] = Operand::Variable { value: 0 };
            }
            // this should not be reachable
            _ => unreachable!(),
        }

    }

    fn form_short_opcode(code: &mut OpCode, id: u8) {

        // we include the header byte now
        code.read_bytes = 1;
        code.form = OpForm::Short;

        // mask out the top four bits, and we have our opcode
        // (bottom four bits)
        code.code = id & 0b00001111;

        match id {
            0x80...0x8f => {
                code.operand_count = 1;
                code.operands[0] = Operand::LargeConstant { value: 0 };
            }
            0x90...0x9f => {
                code.operand_count = 1;
                code.operands[0] = Operand::SmallConstant { value: 0 };
            }
            0xa0...0xaf => {
                code.operand_count = 1;
                code.operands[0] = Operand::Variable { value: 0 };
            }
            0xbe => panic!("Extended opcodes not supported!"),
            0xb0...0xbd | 0xbf => {
                code.operand_count = 0;
            }
            // this should not be reachable
            _ => unreachable!(),
        }

    }

    fn form_variable_opcode(code: &mut OpCode, id: u8, second_byte: u8) {

        // we read the first 2 here, as indicated by second byte above
        code.read_bytes = 2;
        code.form = OpForm::Variable;

        // mask out the top three bits, and we have our opcode
        // (bottom five bits)
        code.code = id & 0b00011111;

        match id {
            0xc0...0xdf => {

                code.operand_count = 2;

                // we shift the bits in the second byte to get
                // 2-bit flags which sign the type of each upcoming
                // operand, here we are just getting the byte by
                // shifting and anding against a binary value 11

                let t0 = (second_byte >> 6) & 0b11;
                let t1 = (second_byte >> 4) & 0b11;

                code.operands[0] = OpCode::get_type_for_bit(t0);
                code.operands[1] = OpCode::get_type_for_bit(t1);

            }
            0xe0...0xff => {

                // this is variable, so really we want to see
                // how many there are
                //
                // the first "omitted" result means we are done

                for i in 0..code.operands.len() {

                    let t = (second_byte >> (6 - (i * 2))) & 0b11;

                    if t == 0b11 {
                        break;
                    }

                    code.operands[i] = OpCode::get_type_for_bit(t);
                    code.operand_count = code.operand_count + 1;

                }

            }
            // this should not be reachable
            _ => unreachable!(),

        }

    }

    // this will be a 2-bit value, but we treat it as u8 because that's
    // probably the only reasonable value
    fn get_type_for_bit(type_bits: u8) -> Operand {

        let operand: Operand = match type_bits {
            0b00 => Operand::LargeConstant { value: 0 },
            0b01 => Operand::SmallConstant { value: 0 },
            0b10 => Operand::Variable { value: 0 },
            0b11 => Operand::Omitted {},
            _ => panic!("got something that was not two bytes!"),
        };

        operand

    }

    fn null_instruction(code: &mut OpCode, env: &mut ZMachine) {
        // doooo nothin
    }

    pub fn read_variables(&mut self,
                          frame_view: MemoryView,
                          globals: MemoryView,
                          call_stack: &mut Stack) {

        if self.operand_count == 0 {
            return;
        }

        for i in 0..self.operand_count {
            let op = &mut self.operands[i as usize];
            match op {
                &mut Operand::LargeConstant { ref mut value } => {
                    *value = frame_view.read_u16_at_head(self.read_bytes);
                    self.read_bytes += 2;
                }
                &mut Operand::SmallConstant { ref mut value } => {
                    *value = frame_view.read_at_head(self.read_bytes);
                    self.read_bytes += 1;
                }
                &mut Operand::Variable { ref mut value } => {

                    let addr = frame_view.read_at_head(self.read_bytes);

                    match addr {
                        // 0, its the stack, pop it and return
                        0 => {
                            *value = match call_stack.stack.pop() {
                                Some(x) => x,
                                None => panic!("stack underflow!"),
                            }
                        }

                        // 1 to 15, its a local
                        i @ 0x01...0x0f => *value = call_stack.get_local_variable(i),
                        // 16 to 255, it's a global variable.
                        global @ 0x10...0xff => *value = globals.read_u16_at_head(global as u32),
                        _ => unreachable!(),
                    }

                    self.read_bytes += 1;


                }
                &mut Operand::Omitted => break,
            };
        }

    }
}

impl fmt::Display for OpCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "form: {}\n opcode: {}\n operands:\n{}\n operand_count: {}\n",
               self.form,
               self.code,
               format!("0: {},\n1: {},\n2: {},\n3: {}\n",
                       self.operands[0],
                       self.operands[1],
                       self.operands[2],
                       self.operands[3]),
               self.operand_count)
    }
}
