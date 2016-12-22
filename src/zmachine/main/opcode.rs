// opcode struct

use std::fmt;
use super::MemoryView;
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
    SmallConstant { value: u8 },
    Variable { value: u16 },
    Omitted,
}

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "{}",
               match self {
                   &Operand::LargeConstant { .. } => "Large Constant",
                   &Operand::SmallConstant { .. } => "Small Constant",
                   &Operand::Variable { .. } => "Variable",
                   &Operand::Omitted => "Omitted",
               })
    }
}

// enum OpType {
//
// }
//

pub struct OpCode {
    // unsigned code, 0-255, the "index op code"
    // note this is not really used at all, its just for reference
    // on the table
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

    // does this code jump?
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

    // internally used, how many bytes have we read until the instruction is executed
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
    //
    read_bytes: u16,
}

impl OpCode {
    // opcode can be several bytes long, but in the
    // form section we always allow the function to peek
    // at the top two bytes of the program stack
    //
    // sometimes variable will need the second one,
    // and we trust the opcode itself, since the length is variable,
    // to move the pc

    pub fn form_opcode(code: [u8; 2]) -> OpCode {

        // set some defaults and do stuff we will have to do anyway,
        // like filling out the operands table

        let mut opCode: OpCode = OpCode {
            code: 0,
            branch: false,
            store: false,
            print: false,
            input: false,
            form: OpForm::Short,
            operands: [Operand::Omitted {},
                       Operand::Omitted {},
                       Operand::Omitted {},
                       Operand::Omitted {}],
            operand_count: 0,
            read_bytes: 0,
        };


        // make a closure here to let rust know when we want to drop
        // the mutable reference
        {
            // borrow a mutable reference to form the rest of the code

            let codeRef = &mut opCode;

            match code[0] {
                id @ 0x00...0x7f => OpCode::form_long_opcode(codeRef, id),
                // the fallthrough for be , the code for extended opcodes,
                // falls through in form_short_opcode
                id @ 0x80...0xbf => OpCode::form_short_opcode(codeRef, id),
                id @ 0xc0...0xff => OpCode::form_variable_opcode(codeRef, id, code[1]),
                // this should not be reachable
                _ => unreachable!(),
            }
        }

        // since we dropped the mutable reference, you can have it now,
        // caller
        opCode

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

        code.form = OpForm::Variable;
        // we read the first 2 here, as indicated by second byte above
        code.read_bytes = 2;

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

                println!("operand flag: {:#b}", second_byte);

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

    pub fn read_variables(&mut self, memory: MemoryView, call_stack: &mut Stack) {

        if (self.operand_count == 0) {
            return;
        }

        for i in 0..self.operand_count {
            let op = &mut self.operands[0];
            match op {
                &mut Operand::LargeConstant { mut value } => {
                    value = memory.read_u16_at_offset_from_head(self.read_bytes);
                    self.read_bytes += 2;
                }
                &mut Operand::SmallConstant { mut value } => {
                    value = memory.read_at_offset_from_head(self.read_bytes);
                    self.read_bytes += 1;
                }
                &mut Operand::Variable { mut value } => {
                    let addr = memory.read_at_offset_from_head(self.read_bytes);

                    match addr {
                        // 0, its the stack, pop it and return
                        0 => {
                            value = match call_stack.stack.pop() {
                                Option::Some(x) => x,
                                Option::None => panic!("stack underflow!"),
                            }
                        }

                        // 1 to 15, its a local
                        i @ 0x01...0x0f => value = call_stack.get_local_variable(i),
                        // 16 to 255, it's a global variable.
                        0x10...0xff => panic!("not implemented!"),
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
               "form: {}\n code: {}\n operands: {}\n operand_count: {}\n",
               self.form,
               self.code,
               format!("0: {}, 1: {}, 2: {}, 3: {}\n",
                       self.operands[0],
                       self.operands[1],
                       self.operands[2],
                       self.operands[3]),
               self.operand_count)
    }
}
