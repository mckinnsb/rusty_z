use super::opcode::*;
use super::super::object_properties_view::*;
use super::ZMachine;
use super::Stack;

// short functions

// long functions

// variable length functions
//
// call is one of the few functions that cannot
// really avoid advancing or changing the program
// counter itself. other functions which are
// in-line to routines can be handled by zmachine,
// even things like branching/storing )
// but call and it's attendants have to change the
// ip mid-instruction, since they are reading
// from high memory and setting up the call stack/
// program pointer for the next operation (i.e.,
// it's not a simple advance )
//
// i could try to abstract this, or make this
// part of zmachine, but its really still an op
// code, albiet a really effing powerful one,
// and that would get awkward abstraction-wise

pub fn and(code: &mut OpCode, machine: &mut ZMachine) {
    code.store = true;
    code.result = code.operands[0].get_value() & code.operands[1].get_value();
    // done
}

// maaaybe the most self explanatory jam in the book
// add two numbers. store it in result
//
// zmachine takes care of the 'storing' part

pub fn add(code: &mut OpCode, machine: &mut ZMachine) {

    code.store = true;
    code.result = ((code.operands[0].get_value() as i16) +
                   (code.operands[1].get_value() as i16)) as u16;
    // yay!
}

pub fn call(code: &mut OpCode, machine: &mut ZMachine) {

    // push this current address
    // we split it in two, because the stack holds words
    // note there is more than one way to do this  we could
    // also just store the packed address, or actually implement
    // call frames, but its easier to just implement everything
    // using the staack, and i try not to "massage" things as much as possible
    //
    // packed addresses are really just a design around the limitation of a 16 bit
    //  system anyway )

    let address_lhalf = (machine.ip & 0xFFFF) as u16;
    let address_uhalf = (machine.ip >> 16) as u16;

    machine.call_stack.stack.push(address_lhalf);
    machine.call_stack.stack.push(address_uhalf);

    // push the current offset to the stack
    // note its highly unlikely the conversion will be a problem here;
    // offset is not likely to be greater than 100
    println!("pushing offset: {}", code.read_bytes);
    machine.call_stack.stack.push(code.read_bytes as u16);

    // move program counter
    let mut address = match code.operands[0] {
        Operand::SmallConstant { value } => value as u16,
        Operand::LargeConstant { value } |
        Operand::Variable { value } => value,
        Operand::Omitted { .. } => panic!("you must supply an argument to call, even if 0"),
    };

    // address is actually multiplied by a constant, depending on the version #
    // we just support 3 here, so it is always 2

    machine.ip = address as u32 * 2;

    // set the new call stack
    machine.call_stack.switch_to_new_frame();

    // so here, we are going to set the read bytes of the code to #,
    // this is to prevent the zmachine from advancing the pointer,
    // since we are manually taking care of that here

    code.read_bytes = 0;

    // its important to note here that the function
    // makes space for its arguments and those
    // are also considered "locals". locals left
    // over are initialized to the values specified in
    // those bytes

    let mut num_locals = machine.get_frame_view().read_at_head(0);
    let mut num_args = 0;

    // push local variables onto the call stack

    for op in code.operands[1..].iter() {
        match op {
            &Operand::Omitted => break,
            &Operand::SmallConstant { value } => {
                num_args += 1;
                machine.call_stack.stack.push(value as u16);
            }
            &Operand::LargeConstant { value } |
            &Operand::Variable { value } => {
                num_args += 1;
                machine.call_stack.stack.push(value);
            }
        };
        num_locals -= 1
    }

    // first advance the pointer by the header of the function
    machine.ip += 1;

    // advance the pointer. note that if there are
    // any default values for operands, these
    // will automatically be included in the locals
    // below ( we only read when we know we have no arg )
    //
    // also note these are words, so we skip two bytes
    // for each local

    if num_args > 0 {
        machine.ip = machine.ip + num_args * 2;
    }

    if num_locals > 0 {
        for _ in 0..num_locals {
            let default_value = machine.get_frame_view().read_u16_at_head(0);
            machine.call_stack.stack.push(default_value);
            // advance the pointer
            machine.ip += 2;
        }
    }

    // done!

}

pub fn call_1s(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}
pub fn clear_attr(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}

pub fn dec(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}
pub fn dec_chk(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}
pub fn div(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}

pub fn get_child(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}
pub fn get_parent(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}

pub fn get_prop(code: &mut OpCode, machine: &mut ZMachine) {

    code.store = true;

    let (object, property) = (code.operands[0].get_value(), code.operands[1].get_value());

    code.result = machine.get_object_view()
        .get_properties_table_view(object)
        .get_property(property as u8);

}

pub fn get_prop_len(code: &mut OpCode, machine: &mut ZMachine) {

    code.store = true;

    let property_address = code.operands[0].get_value();
    let size_byte = machine.get_memory_view().read_at(property_address as u32);

    let ObjectProperty { size, .. } =
        ObjectPropertiesView::get_object_property_from_size_byte(size_byte);

    code.result = size as u16;

}

pub fn get_prop_addr(code: &mut OpCode, machine: &mut ZMachine) {

    code.store = true;

    let (object, property) = (code.operands[0].get_value(), code.operands[1].get_value());

    code.result = machine.get_object_view()
        .get_properties_table_view(object)
        .get_property_addr(property as u8) as u16;

}

pub fn get_next_prop(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}
pub fn get_sibling(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}

pub fn inc(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}
pub fn inc_chk(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}
pub fn input_stream(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}
pub fn insert_obj(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}


// je is the only jump expression which can take 4 arguments
// as a variable-encoded expression
//
// some aspects of the design are kind of confusing, im wondering
// if this was more for the lexer
pub fn je(code: &mut OpCode, machine: &mut ZMachine) {

    code.branch = true;

    // god i love rust, watch this

    let candidate = code.operands[0].get_value();
    let mut condition = 0;

    for operand in code.operands[1..].iter() {
        match operand {
            &Operand::Omitted => break,
            _ => {
                if candidate == operand.get_value() {
                    condition = 1;
                    break;
                }
            }
        }
    }

    code.result = condition;

}


pub fn jg(code: &mut OpCode, machine: &mut ZMachine) {
    // casting between signed and unsigned values should be OK
    code.branch = true;
    code.result = ((code.operands[0].get_value() as i16) >
                   (code.operands[1].get_value() as i16)) as u16;
}

pub fn jin(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}

pub fn jl(code: &mut OpCode, machine: &mut ZMachine) {
    code.branch = true;
    code.result = ((code.operands[0].get_value() as i16) <
                   (code.operands[1].get_value() as i16)) as u16;
}

// another one of the few instructions that just modifies ip,
// this time it does absolutely nothing to the call stack
pub fn jump(code: &mut OpCode, machine: &mut ZMachine) {

    let offset = code.operands[0].get_value() as u32;
    machine.ip += offset + code.read_bytes;

}

pub fn jz(code: &mut OpCode, machine: &mut ZMachine) {
    code.branch = true;
    code.result = (code.operands[0].get_value() == 1) as u16;
}

pub fn load(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}

// this also actually operates on the entirety of dynamic memory, and
// can be used to load things outside the global variables table or stack
// so again, we actually use the entire memory view here (just like storew)

pub fn loadw(code: &mut OpCode, machine: &mut ZMachine) {

    code.store = true;

    let (start, index) = (code.operands[0].get_value(), code.operands[1].get_value());

    let address = start + (index * 2);

    code.result = machine.get_memory_view()
        .read_u16_at(address as u32);

}

pub fn loadb(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}

pub fn mul(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}
pub fn mod_fn(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}
pub fn new_line(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}
pub fn nop(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}
pub fn or(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}
pub fn output_stream(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}
pub fn quit(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}
pub fn pop(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}
pub fn print(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}
pub fn print_addr(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}
pub fn print_char(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}
pub fn print_obj(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}
pub fn print_paddr(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}
pub fn print_num(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}
pub fn print_ret(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}

pub fn put_prop(code: &mut OpCode, machine: &mut ZMachine) {

    let (object, property, value) =
        (code.operands[0].get_value(), code.operands[1].get_value(), code.operands[2].get_value());

    println!("object: {}", object);
    println!("property: {}", property);
    println!("value: {}", value);

    machine.get_object_view().
        get_properties_table_view(object).
        //its virtually assured property is always a byte value
        //otherwise, its an inform compiler bug
        write_property(property as u8, value);


}

pub fn pull(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}
pub fn push(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}
pub fn random(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}

// ret
pub fn ret(code: &mut OpCode, machine: &mut ZMachine) {

    // return takes one operand, which is the address to return
    let value = code.operands[0].get_value();
    code.result = value;

    machine.call_stack.restore_last_frame();

    // retrieve the offset
    let offset = match machine.call_stack.stack.pop() {
        Some(x) => x,
        None => panic!("stack underflow when restoring stack offset!"),
    };

    // so... return is not technically a store, but we are faking the end
    // of the "call" here
    //
    // only return actually adequately deals with that - everything else just
    // jumps
    //
    // we add the offset to complete the "fake"

    code.read_bytes = offset as u32;
    code.store = true;

    println!("return offset is:{}", offset);

    // retrieve the lower and top parts of the address
    let address_uhalf = machine.call_stack.stack.pop();
    let address_lhalf = machine.call_stack.stack.pop();

    let address = match (address_uhalf, address_lhalf) {
        (Some(uhalf), Some(lhalf)) => ((((uhalf as u32) << 16) & 0xFF00) | (lhalf as u32)),
        _ => panic!("return call resulted in stack underflow!"),
    };

    println!("address is: {}", address);

    // we don't do *2 on this version since we
    // stored the address in-system ( not as part of asm )
    machine.ip = address;

    // we are done, machine handles store calls

}

pub fn remove_obj(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}
pub fn restore(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}
pub fn restart(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}
pub fn ret_popped(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}
pub fn rfalse(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}

pub fn rtrue(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}
pub fn save(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}
pub fn set_attr(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}
pub fn set_window(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}
pub fn sound_effect(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}
pub fn show_status(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}
pub fn split_window(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}
pub fn sread(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}
pub fn store(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}
pub fn storeb(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}

// this actually operates on the entirety of the dynamic
// memory, and can be used to alter things outside
// of the global variable table ( which seems to just be
// for convenience, but i think his can be used to alter
// abbreviations or things like that mid-game )
pub fn storew(code: &mut OpCode, machine: &mut ZMachine) {

    let (start, index, value) =
        (code.operands[0].get_value(), code.operands[1].get_value(), code.operands[2].get_value());

    let address = start + (index * 2);
    machine.get_memory_view()
        .write_u16_at(address as u32, value);

    // thats all we wrote, its strange store calls don't
    // actually follow the "store" mechanism, but there
    // must have been an design reason

}

pub fn sub(code: &mut OpCode, machine: &mut ZMachine) {
    code.store = true;
    code.result = ((code.operands[0].get_value() as i16) -
                   (code.operands[1].get_value() as i16)) as u16;
}

pub fn test(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}
pub fn test_attr(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}

pub fn verify(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}
