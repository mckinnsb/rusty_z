extern crate rand;
use super::super::object_properties_view::*;
use super::super::zstring::*;
use super::super::memory_view::MemoryView;
use super::super::object_view::ObjectView;
use super::opcode::*;
use super::ZMachine;
use super::super::header::*;
use super::MachineState;
use super::Stack;

use std::cmp;

// for input flushing
use std::io;
use std::io::Write;
use std::str::SplitWhitespace;
use std::process;

use std::rc::*;

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

    // move program counter
    // address is actually multiplied by a constant, depending on the version #
    // we just support 3 here, so it is always 2
    let mut address = (code.operands[0].get_value() as u32) * 2;

    // println!("////////////// calling {:x}", address);

    // if 0 is given to call, we "return" false, and by return
    // i mean the opcode then stores 0 at the value on branch
    // so we fake the end of ret and return
    // we also don't touch read bytes or anything like that at all
    if address == 0 {
        code.result = 0;
        code.store = true;
        return;
    }

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
    // println!("pushing offset: {}", code.read_bytes);
    machine.call_stack.stack.push(code.read_bytes as u16);

    // change the instruction pointer
    machine.ip = address;

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
    //
    // this has a slightly different deconstruction than the vast majority
    // of opcodes because its variable ( can have any number of operands, truly ),
    // and this variable # effects how the call stack is formed

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

// this clears a bit in the attributes table
pub fn clear_attr(code: &mut OpCode, machine: &mut ZMachine) {
    let (object, attr) = (code.operands[0].get_value(), code.operands[1].get_value());

    machine.get_object_view(object).unset_attribute(attr);
    // done
}

pub fn dec(code: &mut OpCode, machine: &mut ZMachine) {

    let variable = code.operands[0].get_value();

    let mut current = machine.read_variable(variable as u8) as i16;
    current -= 1;

    machine.write_variable_in_place(variable as u8, current as u16);

}

// dec check decrements a variable and branches if the variable is now
// less than value
//
// this is not a store function, it actually operates under different
// rules in which the stack is not popped when read or written to

pub fn dec_chk(code: &mut OpCode, machine: &mut ZMachine) {

    code.branch = true;

    let (variable, value) = (code.operands[0].get_value(), code.operands[1].get_value() as i16);

    let mut current = machine.read_variable(variable as u8) as i16;
    current -= 1;

    machine.write_variable_in_place(variable as u8, current as u16);

    match current < value {
        false => code.result = 0,
        true => code.result = 1,
    }

    // done

}


// although not officially documented, this code exists, and was probably
// used for debugging. interpreters are allowed to:
// 1) ignore it
// 2) use it for debugging
//
// it's not a "no-op", strictly speaking, but for our purposes it is.
pub fn debug(code: &mut OpCode, machine: &mut ZMachine) {}

// signed division, should halt interpreter on divide by zero ( the inform
// compiler should guarantee that never happens )
pub fn div(code: &mut OpCode, machine: &mut ZMachine) {

    code.store = true;

    let (dividend, divisor) = (code.operands[0].get_value(), code.operands[1].get_value());

    if divisor == 0 {
        panic!("division by zero!");
    }

    code.result = (dividend as i16 / divisor as i16) as u16;

}

// gets the child id of the object
pub fn get_child(code: &mut OpCode, machine: &mut ZMachine) {

    code.store = true;
    code.branch = true;

    let object = code.operands[0].get_value();
    code.result = machine.get_object_view(object).get_child();
    // done

}

// gets the parent id of the object
pub fn get_parent(code: &mut OpCode, machine: &mut ZMachine) {

    code.store = true;

    let object = code.operands[0].get_value();
    code.result = machine.get_object_view(object).get_parent();
    // done

}

pub fn get_prop(code: &mut OpCode, machine: &mut ZMachine) {

    code.store = true;

    let (object, property) = (code.operands[0].get_value(), code.operands[1].get_value());

    let value = machine.get_object_view(object)
        .get_properties_table_view()
        .get_property(property as u8)
        .value;

    // println!("object:{}\nproperty:{}\nvalue:{}", object, property, value);
    // println!("****");

    code.result = value;

}

pub fn get_prop_len(code: &mut OpCode, machine: &mut ZMachine) {

    code.store = true;

    // we subtract one because the size byte itself is actually 1 below
    // the address given.
    //
    // this is kind of confusing until you realize that the value coming
    // into this operand is coming out of get_prop_addr

    let property_address = code.operands[0].get_value() - 1;
    let size_byte = machine.get_memory_view().read_at(property_address as u32);
    // println!( "sizebyte:{:x}", size_byte );

    let ObjectPropertyInfo { size, .. } =
        ObjectPropertiesView::get_object_property_from_size_byte(size_byte);

    // println!( "prop address:{:x}", property_address );
    // println!( "prop len:{}", size );

    code.result = size as u16;

}

pub fn get_prop_addr(code: &mut OpCode, machine: &mut ZMachine) {

    code.store = true;

    let (object, property) = (code.operands[0].get_value(), code.operands[1].get_value());

    // println!( "object: {}", object );
    // println!( "property: {}", property );

    code.result = machine.get_object_view(object)
        .get_properties_table_view()
        .get_property_addr(property as u8) as u16;

}

// this gets the next property of the property listed,
// if the property given is 0, it finds the first property of the object,
// otherwise, it finds the next property,
//
// and in either case, returns the id of the next property
//
// if the property specified is non-existant, the interpreter should halt.
pub fn get_next_prop(code: &mut OpCode, machine: &mut ZMachine) {

    code.store = true;

    let (object, property) = (code.operands[0].get_value(), code.operands[1].get_value() as u8);

    let property_view = machine.get_object_view(object).get_properties_table_view();

    // this was not appropriately being done before
    let size_byte = {

        let info = property_view.get_property_info(property);

        let addr = match info.addr {
            Some(x) => x,
            None => panic!("attempted to find the next property of a non existant property!"),
        };

        let next_addr = addr + info.size as u32;
        property_view.view.read_at_head(next_addr)

    };

    let ObjectPropertyInfo { id, .. } =
        ObjectPropertiesView::get_object_property_from_size_byte(size_byte);

    code.result = id as u16;

}

pub fn get_sibling(code: &mut OpCode, machine: &mut ZMachine) {

    code.store = true;
    code.branch = true;

    let object = code.operands[0].get_value();
    code.result = machine.get_object_view(object).get_sibling();

    // println!("slibling is {}", code.result);
    // done

}

pub fn inc(code: &mut OpCode, machine: &mut ZMachine) {

    let variable = code.operands[0].get_value();

    let mut current = machine.read_variable(variable as u8) as i16;
    current += 1;

    machine.write_variable_in_place(variable as u8, current as u16);

}

pub fn inc_chk(code: &mut OpCode, machine: &mut ZMachine) {

    code.branch = true;

    let (variable, value) = (code.operands[0].get_value(), code.operands[1].get_value() as i16);

    let mut current = machine.read_variable(variable as u8) as i16;

    current += 1;
    machine.write_variable_in_place(variable as u8, current as u16);

    match current > value {
        false => code.result = 0,
        true => code.result = 1,
    }

}

pub fn input_stream(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}

// this code moves object to the first child of destination -
// basically what this does is it sets "child" of destination to this object,
// and whatever "child" was previously then becomes the "sibling" of this object,
//
// it should be noted we don't change the "parent" status of the previous object -
// that remains ( all children of a parent have "parent" listed, it's just that
// they only refer to their next sibling )
//
// it also should be noted this is used in weird ways;
// you can do insert_obj 0, 1 to basically remove everything from a
// bag, and insert_obj 1, 0 to basically remove an object from a bag
// its more or less up to the author to decide what they want to use
pub fn insert_obj(code: &mut OpCode, machine: &mut ZMachine) {

    let (child, parent) = (code.operands[0].get_value(), code.operands[1].get_value());

    // child will never be 0, parent might be though
    let mut child_view = machine.get_object_view(child);
    let current_parent = child_view.get_parent();

    // we are done if they are the same
    if child_view.get_parent() == parent {
        return;
    }

    // if the current parent is not 0, we have to deparent
    if current_parent != 0 {
        unparent_object(&mut child_view, machine);
    }

    // in the case of 0, this deparents
    child_view.set_parent(parent);

    if parent != 0 {

        let parent_view = machine.get_object_view(parent);
        let old_child = parent_view.get_child();

        parent_view.set_child(child);

        if child != 0 {
            // we could make this more efficient, but it would get kind of ugly
            let child_view = machine.get_object_view(child);
            child_view.set_sibling(old_child);
        }

    }

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

    // println!( "je is branching:{}", condition );
    // println!( "{}",code );

    code.result = condition;

}

// this function jumps if greater than
pub fn jg(code: &mut OpCode, machine: &mut ZMachine) {
    // casting between signed and unsigned values should be OK
    code.branch = true;
    code.result = ((code.operands[0].get_value() as i16) >
                   (code.operands[1].get_value() as i16)) as u16;
}

// this function jumps if the object is a child of the other object,
//
// it seems like this function can also take 0 as an argument to parent,
// to ask the question "does the child have no parent?"
//
// i'm hoping that you cannot also ask, "what is the parent of nothing",
// since that question would make no sense, unless the answer was also
// "nothing", but that would just be like a jump( since its an unconditional
// branch
pub fn jin(code: &mut OpCode, machine: &mut ZMachine) {

    code.branch = true;

    let (child, parent) = (code.operands[0].get_value(), code.operands[1].get_value());

    let child = machine.get_object_view(child);
    code.result = (child.get_parent() == parent) as u16;
    // println!("result is:{}", code.result );

    // done!

}

// this function jumps if less than
pub fn jl(code: &mut OpCode, machine: &mut ZMachine) {
    code.branch = true;
    code.result = ((code.operands[0].get_value() as i16) <
                   (code.operands[1].get_value() as i16)) as u16;
}

// another one of the few instructions that just modifies ip,
// this time it does absolutely nothing to the call stack
pub fn jump(code: &mut OpCode, machine: &mut ZMachine) {

    // we have to be careful about this cast or we will lose the negative value
    let offset = code.operands[0].get_value() as i16;
    let offset = offset as i32;

    // so because we know that machine.ip will always be lower than
    // 2 billion, we can safely convert it to i32 ( and it will still be positive )
    //
    // we have to do this because offset may be negative
    // println!( "old code:{:x}", machine.ip );

    let new_ip = machine.ip as i32 + (offset);
    machine.ip = (new_ip as u32) + code.read_bytes - 2;

    // println!( "code read bytes:{}", code.read_bytes );
    // println!( "offset:{}", offset );
    // println!( "jumping to:{:x}", machine.ip );

    // reset read bytes so machine does not advance the code
    code.read_bytes = 0;
    // done

}

pub fn jz(code: &mut OpCode, machine: &mut ZMachine) {

    code.branch = true;
    code.result = (code.operands[0].get_value() == 0) as u16;
    // println!( "result of jz:{}", code.result );

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

    let address = (start as u32) + (index as u32) * 2;

    code.result = machine.get_memory_view()
        .read_u16_at(address as u32);

}

pub fn loadb(code: &mut OpCode, machine: &mut ZMachine) {

    code.store = true;

    let (array, byte_index) = (code.operands[0].get_value(), code.operands[1].get_value());

    let address = (array as u32) + (byte_index as u32);

    code.result = machine.get_memory_view()
        .read_at(address) as u16;
    // done

}

// signed multiplication
pub fn mul(code: &mut OpCode, machine: &mut ZMachine) {
    code.store = true;
    code.result = ((code.operands[0].get_value() as i16) *
                   (code.operands[1].get_value() as i16)) as u16;
    // done
}

// signed modulo
pub fn mod_fn(code: &mut OpCode, machine: &mut ZMachine) {

    code.store = true;
    code.result = ((code.operands[0].get_value() as i16) %
                   (code.operands[1].get_value() as i16)) as u16;
    // done
}

pub fn new_line(code: &mut OpCode, machine: &mut ZMachine) {
    println!("");
}

//uh... do nothing!
pub fn nop(code: &mut OpCode, machine: &mut ZMachine) {}

pub fn or(code: &mut OpCode, machine: &mut ZMachine) {
    code.store = true;
    code.result = code.operands[0].get_value() | code.operands[1].get_value();
    // done
}

pub fn output_stream(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}

pub fn quit(code: &mut OpCode, machine: &mut ZMachine) {
    println!( "Quitting.");
    process::exit(0);
}

pub fn pop(code: &mut OpCode, machine: &mut ZMachine) {
    machine.call_stack.stack.pop();
    //thats it
}

pub fn print(code: &mut OpCode, machine: &mut ZMachine) {

    let view = machine.get_frame_view();
    let abbreviations_view = machine.get_abbreviations_view();

    let string = ZString::create(code.read_bytes, &view, &abbreviations_view);

    code.read_bytes += string.encoded_length;

    // all print functions use print, instead of println
    print!("{}", string);

}

pub fn print_addr(code: &mut OpCode, machine: &mut ZMachine) {

    let addr = code.operands[0].get_value() as u32;
    // let packed_addr = (addr as u32) * 2;

    let view = machine.get_memory_view();
    let abbreviations_view = machine.get_abbreviations_view();

    let string = ZString::create(addr, &view, &abbreviations_view);

    machine.print_to_main(&format!("{}",string));

}

pub fn print_char(code: &mut OpCode, machine: &mut ZMachine) {
    // let ch = (code.operands[0].get_value());
    // let mut ch_str = String::with_capacity(1);
    //
    // this is similar to print_obj in that we do not println!
    match ZString::decode_zscii(code.operands[0].get_value()) {
        Some(x) => machine.print_to_main(&char::to_string(&x)),
        None => {}
    }

}

pub fn print_obj(code: &mut OpCode, machine: &mut ZMachine) {

    let object = code.operands[0].get_value();

    let view = machine.get_object_view(object).get_properties_table_view();

    // the string is offset by one because properties starts with the size byte,
    // then is followed by the short name of the object
    let string = ZString::create(1, &view.view, &machine.get_abbreviations_view());

    // we actually print instead of println here because
    // objects can handle carriage returns themselves ( a ZString has a newline
    // character )

    machine.print_to_main(&format!("{}", string));

}

pub fn print_paddr(code: &mut OpCode, machine: &mut ZMachine) {

    let packed_addr = code.operands[0].get_value();

    // another thing that we will have to change in the future,
    // packed addresses vary based on version..
    //
    // also gotta be careful with these casts - they are packed for
    // a reason ( they are 16bit representations of 32 bit word locations )

    let full_addr = (packed_addr as u32) * 2;

    let view = machine.get_memory_view();
    let abbreviations_view = machine.get_abbreviations_view();
    let string = ZString::create(full_addr, &view, &abbreviations_view);

    machine.print_to_main(&format!("{}", string));

}

pub fn print_num(code: &mut OpCode, machine: &mut ZMachine) {
    let num = (code.operands[0].get_value());
    machine.print_to_main(&format!("{}", num as i16));
}

// there are a lot of "macro commands" in the z-instruction set,
// probably to save on common tasks, such as this one,
// frequently used when you succeed in doing something
// print success message, new line, and return true)
pub fn print_ret(code: &mut OpCode, machine: &mut ZMachine) {
    print(code, machine);
    new_line(code, machine);
    rtrue(code, machine);
}

pub fn put_prop(code: &mut OpCode, machine: &mut ZMachine) {

    let (object, property, value) =
        (code.operands[0].get_value(), code.operands[1].get_value(), code.operands[2].get_value());

    // println!("************************************************");
    // println!("WRITING object: {}", object);
    // println!("property: {}", property);
    // println!("value: {}", value);
    // println!("****");
    //
    machine.get_object_view(object).
        get_properties_table_view().
        //its virtually assured property is always a byte value
        //otherwise, its an inform compiler bug
        write_property(property as u8, value);


}

// weirdly enough, this is not a store call
// i have no idea if its legal to pull and push
// back onto the stack, but i don't see why not
pub fn pull(code: &mut OpCode, machine: &mut ZMachine) {

    let destination = code.operands[0].get_value();
    let value = machine.call_stack.stack.pop();

    let value = match value {
        Some(x) => x,
        None => panic!("stack underflow!"),
    };

    machine.store_variable(destination as u8, value);
    // done

}

pub fn push(code: &mut OpCode, machine: &mut ZMachine) {
    let value = code.operands[0].get_value();
    machine.call_stack.stack.push(value);
}

pub fn random(code: &mut OpCode, machine: &mut ZMachine) {

    code.store = true;

    let (range, seed) = match code.operands[0].get_value() {
        x if x < 0 => (None, Some(x)),
        x @ _ => (Some(x), None),
    };

    //the function will only seed or return a random number, not both
    if let Some(seed_value) = seed {
        //if its a seed, the result is 0
        //and we do not generate a return value
        
        machine.random_generator.seed(seed_value);
        code.result = 0;
        return;
    };

    if let Some(range_value) = range {
        let random = machine.random_generator.next(range_value);
        code.result = random;
        return;
    };

}

pub fn remove_obj(code: &mut OpCode, machine: &mut ZMachine) {

    let obj = code.operands[0].get_value();
    let mut view = machine.get_object_view(obj);
    unparent_object(&mut view, machine);

}

pub fn restore(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}

pub fn restart(code: &mut OpCode, machine: &mut ZMachine) {

    machine.state = MachineState::Restarting;
    code.read_bytes = 0;

}

// ret
pub fn ret(code: &mut OpCode, machine: &mut ZMachine) {

    // return takes one operand, which is the address to return
    let value = code.operands[0].get_value();
    code.result = value;

    // drain the stack and restore the last call frame
    // this will leave our information at the top of the stack
    machine.call_stack.restore_last_frame();

    // retrieve the offset
    let offset = match machine.call_stack.stack.pop() {
        Some(x) => x as u32,
        None => panic!("stack underflow when restoring stack offset!"),
    };

    // so... return is not technically a store, but we are faking the end
    // of the "call" here
    //
    // im not sure how other interpretations do if they are actually
    // trying to generalize "store" and "branch"
    //
    // only return actually adequately deals with that - everything else just
    // jumps
    //
    // we add the offset to complete the "fake"

    code.read_bytes = offset;
    code.store = true;

    // retrieve the lower and top parts of the address
    let address_uhalf = machine.call_stack.stack.pop();
    let address_lhalf = machine.call_stack.stack.pop();

    let address = match (address_uhalf, address_lhalf) {
        (Some(uhalf), Some(lhalf)) => ((((uhalf as u32) << 16) & 0xFFFF0000) | (lhalf as u32)),
        _ => panic!("return call resulted in stack underflow!"),
    };

    // we don't do *2 on this version since we
    // stored the address in-system ( not as part of asm )
    machine.ip = address;

    // println!("returning to:{:x} in ret ********", address + offset);

    // we are done, machine handles store calls

}

// pop the stack and return that value,
// similar to rtrue except we modify the stack
// the next three fucntions are all 0-op,
// so we don't have to worry too much about modifying operands
// to use ret ( 1-OP )
pub fn ret_popped(code: &mut OpCode, machine: &mut ZMachine) {

    let value = match machine.call_stack.stack.pop() {
        Some(x) => x,
        None => panic!("stack underflow!"),
    };

    code.operands[0] = Operand::LargeConstant { value: value };
    ret(code, machine);

}

// return the value false
pub fn rfalse(code: &mut OpCode, machine: &mut ZMachine) {
    // similar to rtrue
    code.operands[0] = Operand::SmallConstant { value: 0 };
    ret(code, machine);
}

// return the value true
pub fn rtrue(code: &mut OpCode, machine: &mut ZMachine) {
    // so here we do some fudging, because we want to re-use ret
    // the alternative is to create an abstraction that "handled" returning, but
    // that seems like almost everything that ret does
    code.operands[0] = Operand::SmallConstant { value: 1 };
    ret(code, machine);
}

pub fn save(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}

// this sets a bit in the attributes table
pub fn set_attr(code: &mut OpCode, machine: &mut ZMachine) {

    // println!("{}",code);
    let (object, attr) = (code.operands[0].get_value(), code.operands[1].get_value());

    machine.get_object_view(object).set_attribute(attr);

    // done
}

pub fn set_window(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}

pub fn sound_effect(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}

// this is only for version 1-3
//
// in versions 4+ the game itself is responsible for this information,
// and interfaces actually become more complex with additional windows

// The below code will only appear in version 3, but it will be used by read regardless
// in 1-3.
//
// (In Version 3 only.) Display and update the status line now (don't wait until 
// the next keyboard input).(In theory this opcode is illegal in later Versions 
// but an interpreter should treat it as nop, because Version 5 Release 23 of 
// 'Wishbringer' contains this opcode by accident.)

// In Versions 1 to 3, a status line should be printed by the interpreter, as follows. In Version
// 3, it must set bit 4 of 'Flags 1' in the header if it is unable to produce a status line.
//
pub fn show_status(code: &mut OpCode, machine: &mut ZMachine) {

    // The short name of the object whose number is in the first global variable should be printed
    // on the left hand side of the line.
    
    //we can match against version 3 by also matching against the header flags, which 
    //will let us unwrap some necessary info

    if let &HeaderFlags::V1{ flags: HeaderFlagsV1{ ref status_line, .. } } = &machine.header.flags {

        let globals = machine.get_global_variables_view();

        let score_object = globals.read_global(0);
        let first_param = globals.read_global(1);
        let second_param = globals.read_global(2);

        let view = machine.get_object_view(score_object).get_properties_table_view();
        // the string is offset by one because properties starts with the size byte,
        // then is followed by the short name of the object
        let score_name = ZString::create(1, &view.view, &machine.get_abbreviations_view());

        let out = match (status_line, first_param > 12 ) {
            (&StatusLineType::Hours, false) => format!( "Time: {}:{} AM", first_param, second_param ),
            (&StatusLineType::Hours, true) => format!( "Time: {}:{} PM", first_param, second_param ),
            (&StatusLineType::Score, _) => format!( "Score: {} Turns: {}", first_param, second_param ),
        };

        machine.print_to_header(&format!("{}",score_name), &out);

    }

    
}

pub fn split_window(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}

pub fn sread(code: &mut OpCode, machine: &mut ZMachine) {

    show_status(code, machine);
    io::stdout().flush();

    let (text_buffer, parse_buffer) = (code.operands[0].get_value(), code.operands[1].get_value());

    // this a little cheat;
    //
    // so, we box up the function in an Rc,
    // which cannot lend mutable references
    //
    // as a result, we use a Fn, not FnMut, but to do that,
    // we need only non-mutable bindings to persist into the
    // closure; so we use a wrapped refcell
    //
    // when we de-cheat the memory view functions and force
    // mut for write functions that access borrow_mut( which also might
    // lead to the breaking up of the memory and the elimination of Rc<RefCell<Vec>>),
    // we will have to change this

    let view = machine.get_memory_view();
    let dictionary_view = machine.get_dictionary_view();
    let abbreviations_view = machine.get_abbreviations_view();
    let version = machine.header.version;

    let process_input = Rc::new(move |input: String| {

        // text and parse are addrs that indicate where

        // the text should be filled with the text input

        // the parse-buffer, if non-zero, should be filled
        // with tokenized words from the text buffer that
        // match the dictionary
        //
        // youll notice a slight difference between this
        // and the parse_buffer call - we want to avoid
        // splitting the string if the input is too large

        let mut cursor = text_buffer as u32;
        let max_length = view.read_at(cursor);

        // we also don't write over the max length,
        // i believe that stays the same all game

        cursor += 1;

        if input.len() as u8 > max_length {
            println!("response too large! try again");
            return;
        }

        // we have to double-let because it actually
        // goes from String to &str and back again
        //
        // im not 100% sure whats the best way to handle
        // that uniformly at this point

        let cleaned_input = input.trim();
        let cleaned_input = cleaned_input.to_lowercase();
        let split = cleaned_input.split_whitespace();

        // we use cursor because we had to increment the text buffer pos by 1
        // split is no longer ours
        let words = sread_write_to_text_buffer(&view, split, cursor);

        // if the parse buffer is 0, that means we don't parse at all
        if parse_buffer == 0 {
            return;
        }

        // we pass version so the word can be correctly encoded
        sread_write_to_parse_buffer(&view, &words, &dictionary_view, parse_buffer, version);

    });

    machine.state = MachineState::TakingInput { callback: process_input }

}

// private helper function for sread, takes a cleaned, split input
// and writes it to the text buffer at the position indicated,
// and returns a Vec of words in a list ( for use in the next step )
//
// this function takes ownership of split ( and technically text buffer, but thats Copy )
fn sread_write_to_text_buffer(view: &MemoryView,
                              mut split: SplitWhitespace,
                              mut text_buffer: u32)
                              -> Vec<String> {

    // 8 words is probably a reasonable upper bound
    let mut words = Vec::with_capacity(8);

    while let Some(word) = split.next() {

        for ch in word.chars() {
            view.write_at(text_buffer, ch as u8);
            text_buffer += 1;
        }

        view.write_at(text_buffer, ' ' as u8);
        text_buffer += 1;

        words.push(String::from(word));

    }

    words

}

// private helper function for sread, takes a vec of words, looks them up in the dictionary,
// then writes
//
// 1) address in dictionary in first two bytes( 0 if nothing ),
// 2) # of characters in second byte
// 3) position in string of first letter of the word
fn sread_write_to_parse_buffer(view: &MemoryView,
                               words: &Vec<String>,
                               dictionary_view: &MemoryView,
                               parse_buffer: u16,
                               version: u8) {

    let mut cursor = parse_buffer as u32;

    // whats the max # of parsed tokens?
    let max_tokens = view.read_at(cursor);

    // determine the count from the lower of the two - the # of words,
    // and the max count
    let token_count = cmp::min(max_tokens, words.len() as u8) as usize;

    // we don't write to this
    cursor += 1;

    // write the word(token) count in the first byte of the parse buffer
    view.write_at(cursor, token_count as u8);

    // we write to this
    cursor += 1;

    // we need a mutable letter cursor for this double loop,
    // which indicates the start of the word in the sentence
    // rather than the place in memory

    let mut letter_cursor = 1;

    for word in words[0..token_count].iter() {

        let encoded_word = ZString::encode_word(word, version);
        let address: Option<u32> = sread_find_word_in_dictionary(&encoded_word, &dictionary_view);

        // write the address of the abbreviations table from the dictionary

        // println!( "writing to: {:x}", cursor );

        match address {
            // this might look dangerous, but abbreviation strings
            // actually are in the lower 128k of memory ( after the header )
            // i think the maximum size allowed is 64k - 64 bytes ( the size
            // of the header )
            Some(x) => view.write_u16_at(cursor, x as u16),
            None => view.write_u16_at(cursor, 0),
        }

        view.write_at(cursor + 2, word.len() as u8);
        view.write_at(cursor + 3, letter_cursor);

        // increment the letter cursor, include a space
        letter_cursor += word.len() as u8 + 1;

        // increment the parse cursor by 4 bytes
        cursor += 4;

    }

}

// private helper function for sread, finds words in the dictionary
// i believe this is the only op - or part of the code - that uses it
fn sread_find_word_in_dictionary(string: &ZWord, dictionary: &MemoryView) -> Option<u32> {

    // so interestingly enough, the dictionary starts with a # and a list of
    // codes which correspond to keyboard input.
    let num_input_codes = dictionary.read_at_head(0) as u32;

    // after all the input codes, which are a byyte each, we have the entry length
    let entry_length = dictionary.read_at_head(num_input_codes + 1) as u32;

    // and one after that we have the # of entries in a word
    let dictionary_entries = dictionary.read_u16_at_head(num_input_codes + 2) as u32;

    // so the total offset is four bytes + num_input_codes
    let dictionary_header_offset = num_input_codes + 4;

    // and we more or less have to compute this each time, since its actually legal
    // to alter the dictionary

    // binary search
    // it ends up working because all characters are padded with the same
    // end character (5), and the numerical order of z-characters turns
    // out to also be the alphabetic one
    //
    // we basically take the first 4 or 6 bytes of the word and turn
    // it into a number, and that's the index of the abbreviations table

    let mut address = None;
    let mut lower = 0;
    let mut upper = dictionary_entries - 1;
    let mut pointer = 0;

    while lower <= upper {

        pointer = lower + (upper - lower) / 2;
        let offset = dictionary_header_offset + pointer * entry_length;
        let mut found: bool = false;

        // we need to both pull the encoded value out of the enum,
        // and the entry from the table itself, because if there
        // is no match, we have to change the upper/lower bounds
        // based on comparing the two values
        //
        // note that in the future these will probably need to be vecs,
        // because the match arms wont match types when v4 is implemented

        let (mut encoded_string, mut dictionary_entry) = match string {

            // we can "move" encoded out of zword here, because it has the copy trait
            // as an array of u8s
            &ZWord::V3 { encoded, .. } => {

                let encoded_entry = [dictionary.read_at_head(offset),
                                     dictionary.read_at_head(offset + 1),
                                     dictionary.read_at_head(offset + 2),
                                     dictionary.read_at_head(offset + 3)];

                found = encoded_entry == encoded;

                (encoded, encoded_entry)

            }
            // v4 not implemented yet
            _ => unimplemented!(),
        };

        if found {
            // println!("found the lookup!");
            address = Some(dictionary.pointer + offset);
            break;
        }

        // note this will not work if the entries are larger than
        // four words ( 64 bits ), but realistically we only deal
        // with 32 and 48 bit situations
        //
        // we take all the the bytes, and combine them into one entry,
        // and then compare the 32 or "48" bit number ( both housed
        // in a 64 bit number since modern cpus woo! )

        let encode_map = [encoded_string, dictionary_entry]
            .into_iter()
            .map(|container| {

                let mut total: u64 = 0;

                for (i, value) in container.iter().enumerate() {
                    let val = *value;
                    let shift = ((container.len() - i - 1) * 8) as u8;
                    total = total | ((val as u64) << shift);
                }

                total

                // turbofish!
            })
            .collect::<Vec<u64>>();

        let (mut encoded, mut encoded_entry) = (encode_map[0], encode_map[1]);

        // the ordering of the table corresponds to dictionary ordering,
        // and is sorted

        if encoded < encoded_entry {
            upper = pointer - 1;
        } else {
            lower = pointer + 1;
        }

    }

    address

}

// stores that aren't stores trip me up, honestly
pub fn store(code: &mut OpCode, machine: &mut ZMachine) {

    let (variable, value) = (code.operands[0].get_value(), code.operands[1].get_value());

    machine.store_variable(variable as u8, value);
    // done

}

// like storew, this operates on all memory, and is
// another store that isn't a store, as this stores
// in the opcode and not via the machine

pub fn storeb(code: &mut OpCode, machine: &mut ZMachine) {


    let (start, index, value) =
        (code.operands[0].get_value(), code.operands[1].get_value(), code.operands[2].get_value());

    let address = start + index;
    machine.get_memory_view()
        .write_at(address as u32, value as u8);

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

    code.branch = true;

    let (mask, flags) = (code.operands[0].get_value(), code.operands[1].get_value());

    code.result = (mask & flags == flags) as u16;
}

pub fn test_attr(code: &mut OpCode, machine: &mut ZMachine) {

    code.branch = true;

    let (object, attribute) = (code.operands[0].get_value(), code.operands[1].get_value());

    // println!( "object:{}", object);
    // println!( "attribute:{}", attribute);

    code.result = machine.get_object_view(object)
        .has_attribute(attribute) as u16;

    // println!( "result:{}", code.result);

}

fn unparent_object( obj_view: &mut ObjectView, machine: &mut ZMachine ) {

    let current_parent = obj_view.get_parent();

    if current_parent == 0 {
        return;
    }

    let parent_view = machine.get_object_view(current_parent);
    let mut current_child = parent_view.get_child();

    // i would try to generalize the logic, but as it turns out, you do completely
    // different things
    if current_child == obj_view.object_id {
        // if first child, set parent's new first child to child's sibling
        parent_view.set_child(obj_view.get_sibling());
    } else {
        // else, progress through children until child is found, then
        // set previous child to child's sibling
        let mut last_child = current_child;

        loop {

            last_child = current_child;
            current_child = machine.get_object_view(current_child).get_sibling();

            if current_child == obj_view.object_id {
                break;
            }

            if current_child == 0 {
                panic!("object tree badly formed - object marked as having a parent it \
                        does not");
            }

        }

        let new_sibling = machine.get_object_view(current_child).get_sibling();
        machine.get_object_view(last_child).set_sibling(new_sibling);

    }

}

//checksum always passes, for now
//
//this was used for piracy and fidelity reasons;
//these days? not sure what it could really be used for
//i don't know of any games that actually use it for a game purpose
//
//it would literally have to be a programming game or something, that
//involved creating a checksum
//
//so, we aren't going to bother here. if the story file is tampered with
//whatever, if it is tampered to the point of breaking, its going to crash
//anyway
pub fn verify(code: &mut OpCode, machine: &mut ZMachine) {
    code.branch = true;
    code.result = 1;
}

