
use super::opcode::*;
use super::ZMachine;
use super::Stack;

//short functions

//long functions

//variable length functions
//
//call is one of the few functions that cannot
//really avoid advancing or changing the program
//counter itself. other functions which are
//in-line to routines can be handled by zmachine,
//( even things like branching/storing )
//but call and it's attendants have to change the
//ip mid-instruction, since they are reading
//from high memory and setting up the call stack/
//program pointer for the next operation (i.e.,
//it's not a simple advance )
//
//i could try to abstract this, or make this
//part of zmachine, but its really still an op
//code, albiet a really effing powerful one,
//and that would get awkward abstraction-wise

pub fn call( code: &mut OpCode, machine: &mut ZMachine ) {


    //push this current address
    //we split it in two, because the stack holds words
    //note there is more than one way to do this  we could
    //also just store the packed address, or actually implement
    //call frames, but its easier to just implement everything
    //using the staack, and i try not to "massage" things as much as possible
    //
    //( packed addresses are really just a design around the limitation of a 16 bit
    //  system anyway )

    let address_lhalf = ( machine.ip & 0xFFFF ) as u16;
    let address_uhalf = ( machine.ip >> 16 ) as u16;

    machine.call_stack.stack.push( address_lhalf );
    machine.call_stack.stack.push( address_uhalf );

    //push the current offset to the stack
    machine.call_stack.stack.push( code.read_bytes );

    //move program counter
    let mut address = match code.operands[0] {
        Operand::SmallConstant{ value } => value as u16,
        Operand::LargeConstant{ value } |
        Operand::Variable{ value } => value,
        Operand::Omitted{..} => panic!( "you must supply an argument to call, even if 0" ),
    };

    //address is actually multiplied by a constant, depending on the version #
    //we just support 3 here, so it is always 2

    machine.ip = address as u32 * 2;

    //set the new call stack
    machine.call_stack.switch_to_new_frame();

    //its important to note here that the function
    //makes space for its arguments and those
    //are also considered "locals". locals left
    //over are initialized to the values specified in
    //those bytes

    let mut num_locals = machine.get_frame_view().read_at_head(0);
    let mut num_args = 0;

    //push local variables onto the call stack

    for op in code.operands[1..].iter() {
        match op {
            &Operand::Omitted => break,
            &Operand::SmallConstant { value } => {
                num_args += 1;
                machine.call_stack.stack.push( value as u16 );
            },
            &Operand::LargeConstant { value } |
            &Operand::Variable { value } => {
                num_args += 1;
                machine.call_stack.stack.push( value );
            },
        };
        num_locals -= 1
    };

    //advance the pointer. note that if there are
    //any default values for operands, these
    //will automatically be included in the locals
    //below ( we only read when we know we have no arg )
    //
    //also note these are words, so we skip two bytes
    //for each local

    if num_args > 0 {
        machine.ip = machine.ip + num_args*2;
    }

    if num_locals > 0 {
        for _ in 0..num_locals {
            let default_value = machine.get_frame_view().read_u16_at_head(0);
            machine.call_stack.stack.push( default_value );
            //advance the pointer
            machine.ip += 2;
        }
    }

    //done!

}

//this actually operates on the entirety of the dynamic
//memory, and can be used to alter things outside
//of the global variable table ( which seems to just be
//for convenience, but i think his can be used to alter
//abbreviations or things like that mid-game )

pub fn storew( code: &mut OpCode, machine: &mut ZMachine ) {

    let ( start, index, value ) = (
        code.operands[0].get_value(),
        code.operands[1].get_value(),
        code.operands[2].get_value()
    );

    let address = start + (index*2);
    machine.get_memory_view().write_u16_at(address, value);

    //thats all we wrote, its strange store calls don't
    //actually follow the "store" mechanism, but there
    //must have been an design reason

}

//ret
pub fn ret( code: &mut OpCode, machine: &mut ZMachine ) {

    //return takes one operand, which is the address to return
    let value = code.operands[0].get_value();
    code.result = value;

    machine.call_stack.restore_last_frame();

    //retrieve the offset
    let offset = machine.call_stack.stack.pop();

    //set the read bytes back to the offset, this is kind
    //of like mimicing the old instruction
    code.read_bytes = offset;

    //retrieve the lower and top parts of the address
    let address_lhalf = machine.call_stack.stack.pop();
    let address_uhalf = machine.call_stack.stack.pop();

    let address = match (address_uhalf, address_lhalf) {
        (Some(uhalf), Some(lhalf)) => {
            ( uhalf as u32 ) << 16 |
            ( lhalf as u32 )
        },
        _ => panic!( "return call resulted in stack underflow!" ),
    };


    //we don't do *2 on this version since we
    //stored the address in-system ( not as part of asm )
    machine.ip = address;

    //we are done, machine handles store calls

}

