use super::super::object_properties_view::*;
use super::super::zstring::*;
use super::opcode::*;
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
    // println!("pushing offset: {}", code.read_bytes);

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

    let (variable, value) = (code.operands[0].get_value(), code.operands[1].get_value() as i16);

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

    let (variable, value) = (code.operands[0].get_value(), 
                             code.operands[1].get_value() as i16);

    let mut current = machine.read_variable(variable as u8) as i16;
    current -= 1;

    machine.write_variable_in_place(variable as u8, current as u16);

    match current < value {
        false => code.result = 0,
        true => code.result = 1,
    }

    // done

}

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

pub fn get_child(code: &mut OpCode, machine: &mut ZMachine) {

    code.store = true;
    code.branch = true;

    let object = code.operands[0].get_value();

    code.result = machine.get_object_view(object).get_child();

}

pub fn get_parent(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}

pub fn get_prop(code: &mut OpCode, machine: &mut ZMachine) {

    code.store = true;

    let (object, property) = (code.operands[0].get_value(), code.operands[1].get_value());

    code.result = machine.get_object_view(object)
        .get_properties_table_view()
        .get_property(property as u8)
        .value;

}

pub fn get_prop_len(code: &mut OpCode, machine: &mut ZMachine) {

    code.store = true;

    let property_address = code.operands[0].get_value();
    let size_byte = machine.get_memory_view().read_at(property_address as u32);

    let ObjectPropertyInfo { size, .. } =
        ObjectPropertiesView::get_object_property_from_size_byte(size_byte);

    code.result = size as u16;

}

pub fn get_prop_addr(code: &mut OpCode, machine: &mut ZMachine) {

    code.store = true;

    let (object, property) = (code.operands[0].get_value(), code.operands[1].get_value());

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

    let size_byte = match property {
        0 => property_view.view.read_at_head(0),
        _ => {

            let info = property_view.get_property_info(property);

            let addr = match info.addr {
                Some(x) => x,
                None => panic!("attempted to find the next property of a non existant property!"),
            };

            let next_addr = addr + info.size as u32;
            property_view.view.read_at_head(next_addr)

        }
    };

    let ObjectPropertyInfo { id, .. } =
        ObjectPropertiesView::get_object_property_from_size_byte(size_byte);

    code.result = id as u16;

}

pub fn get_sibling(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}

pub fn inc(code: &mut OpCode, machine: &mut ZMachine) {

    let (variable, value) = (code.operands[0].get_value(), code.operands[1].get_value() as i16);

    let mut current = machine.read_variable(variable as u8) as i16;
    current += 1;

    machine.write_variable_in_place(variable as u8, current as u16);

}

pub fn inc_chk(code: &mut OpCode, machine: &mut ZMachine) {

    code.branch = true;

    let (variable, value) = (code.operands[0].get_value(), 
                             code.operands[1].get_value() as i16);

    let mut current = machine.read_variable(variable as u8) as i16;
    machine.write_variable_in_place(variable as u8, (current + 1)as u16);

    match current > value {
        false => code.result = 0,
        true => code.result = 1,
    }

}

pub fn input_stream(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}

//this code moves object to the first child of destination -
//basically what this does is it sets "child" of destination to this object,
//and whatever "child" was previously then becomes the "sibling" of this object,
//
//it should be noted we don't change the "parent" status of the previous object - 
//that remains ( all children of a parent have "parent" listed, it's just that
//they only refer to their next sibling )
//
//it also should be noted this is used in weird ways;
//you can do insert_obj 0, 1 to basically remove everything from a
//bag, and insert_obj 1, 0 to basically remove an object from a bag
//its more or less up to the author to decide what they want to use

pub fn insert_obj(code: &mut OpCode, machine: &mut ZMachine) {

    let (child, parent) = ( code.operands[0].get_value(),
                            code.operands[1].get_value() );

    //println!( "child: {}, parent: {}", child, parent );
    //println!( "address: {:x}, op_code:{}", machine.ip, code );

    if child != 0 {
        let child_view = machine.
                           get_object_view(child);

        //in the case of 0, this deparents
        child_view.set_parent(parent);
    }

    if parent != 0 {
        let parent_view = machine.
                            get_object_view(parent);

        //in the case of 0, this empties
        parent_view.set_child(child);
    }

    if parent != 0 && child != 0 {
        //we could make this more efficient, but it would get kind of ugly
        let child_view = machine.
                           get_object_view(child);

        let parent_view = machine.
                            get_object_view(parent);

        //since both are not zero, we are actually inserting
        child_view.set_sibling(parent_view.get_child());
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
pub fn jin(code: &mut OpCode, machine: &mut ZMachine) {

    code.branch = true;

    let (child, parent) = (code.operands[0].get_value(),
                           machine.get_object_view(code.operands[1].get_value()));

    // this will convert to 1
    code.result = (parent.get_child() == child) as u16;
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

    //we have to be careful about this cast or we will lose the negative value
    let offset = code.operands[0].get_value() as i16;
    let offset = offset as i32;

    //so because we know that machine.ip will always be lower than
    //2 billion, we can safely convert it to i32 ( and it will still be positive )
    //
    //we have to do this because offset may be negative
    //println!( "old code:{:x}", machine.ip );

    let new_ip = machine.ip as i32 + (offset);
    machine.ip = (new_ip as u32) + code.read_bytes - 2;

    //println!( "code read bytes:{}", code.read_bytes );
    //println!( "offset:{}", offset );
    //println!( "jumping to:{:x}", machine.ip );
    
    //reset read bytes so machine does not advance the code
    code.read_bytes = 0;
    //done

}

pub fn jz(code: &mut OpCode, machine: &mut ZMachine) {
    code.branch = true;
    code.result = (code.operands[0].get_value() == 0) as u16;
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

    let address = (start as u32) + (index as u32 * 2);

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

pub fn nop(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}

pub fn or(code: &mut OpCode, machine: &mut ZMachine) {
    code.store = true;
    code.result = code.operands[0].get_value() | code.operands[1].get_value();
    // done
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

    let view = machine.get_frame_view();
    let abbreviations_view = machine.get_abbreviations_view();

    let string = ZString::create(code.read_bytes, &view, &abbreviations_view);

    code.read_bytes += string.encoded_length;

    print!("{}", string);
    // println!("read bytes: {}", code.read_bytes);

}

pub fn print_addr(code: &mut OpCode, machine: &mut ZMachine) {

    let addr = code.operands[0].get_value() as u32;
    //let packed_addr = (addr as u32) * 2;

    let view = machine.get_memory_view();
    let abbreviations_view = machine.get_abbreviations_view();

    let string = ZString::create(addr, &view, &abbreviations_view);

    print!("{}", string);

}

pub fn print_char(code: &mut OpCode, machine: &mut ZMachine) {
    //let ch = (code.operands[0].get_value());
    //let mut ch_str = String::with_capacity(1);
    match ZString::decode_zscii(code.operands[0].get_value()) {
        Some(x) => print!("{}", x ),
        None => {}
    }

}

pub fn print_obj(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}

pub fn print_paddr(code: &mut OpCode, machine: &mut ZMachine) {

    let packed_addr = code.operands[0].get_value();

    //another thing that we will have to change in the future,
    //packed addresses vary based on version..
    //
    //also gotta be careful with these casts - they are packed for 
    //a reason ( they are 16bit representations of 32 bit word locations )
    
    let full_addr = (packed_addr as u32)*2; 

    println!( "printing whatever is at: {}", full_addr );

    let view = machine.get_memory_view();
    let abbreviations_view = machine.get_abbreviations_view();
    let string = ZString::create(full_addr, &view, &abbreviations_view);

    print!("{}", string);

}

pub fn print_num(code: &mut OpCode, machine: &mut ZMachine) {
    let num = (code.operands[0].get_value());
    print!("{}", num as i16);

}

//there are a lot of "macro commands" in the z-instruction set,
//probably to save on common tasks, such as this one,
//frequently used when you succeed in doing something
//(print success message, new line, and return true)
pub fn print_ret(code: &mut OpCode, machine: &mut ZMachine) {
    println!("printing and returning");
    print(code, machine);
    new_line(code, machine);
    rtrue(code, machine);
}

pub fn put_prop(code: &mut OpCode, machine: &mut ZMachine) {

    let (object, property, value) =
        (code.operands[0].get_value(), code.operands[1].get_value(), code.operands[2].get_value());

    // println!("object: {}", object);
    // println!("property: {}", property);
    // println!("value: {}", value);

    machine.get_object_view(object).
        get_properties_table_view().
        //its virtually assured property is always a byte value
        //otherwise, its an inform compiler bug
        write_property(property as u8, value);


}

//weirdly enough, this is not a store call
//i have no idea if its legal to pull and push
//back onto the stack, but i don't see why not
pub fn pull(code: &mut OpCode, machine: &mut ZMachine) {

    let destination = code.operands[0].get_value();
    let value = machine.call_stack.stack.pop();

    let value = match value {
        Some(x) => x,
        None => panic!( "stack underflow!" ),
    };

    machine.store_variable(destination as u8, value);
    //done
    
}

pub fn push(code: &mut OpCode, machine: &mut ZMachine) {
    let value = code.operands[0].get_value();
    machine.call_stack.stack.push(value);
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

    // println!("return offset is:{}", offset);

    // retrieve the lower and top parts of the address
    let address_uhalf = machine.call_stack.stack.pop();
    let address_lhalf = machine.call_stack.stack.pop();

    let address = match (address_uhalf, address_lhalf) {
        (Some(uhalf), Some(lhalf)) => ((((uhalf as u32) << 16) & 0xFF00) | (lhalf as u32)),
        _ => panic!("return call resulted in stack underflow!"),
    };

    // println!("address is: {}", address);

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

//pop the stack and return that value,
//similar to rtrue except we modify the stack
//the next three fucntions are all 0-op,
//so we don't have to worry too much about modifying operands
//to use ret ( 1-OP )
pub fn ret_popped(code: &mut OpCode, machine: &mut ZMachine) {
    let value = match machine.call_stack.stack.pop() {
        Some(x) => x,
        None => panic!( "stack underflow!" ),
    };

    code.operands[0] = Operand::LargeConstant{ value: value };
    ret(code, machine);
}

//return the value false
pub fn rfalse(code: &mut OpCode, machine: &mut ZMachine) {
    //similar to rtrue
    code.operands[0] = Operand::SmallConstant { value: 0 };
    ret(code, machine);
}

//return the value true
pub fn rtrue(code: &mut OpCode, machine: &mut ZMachine) {
    //so here we do some fudging, because we want to re-use ret
    //the alternative is to create an abstraction that "handled" returning, but
    //that seems like almost everything that ret does
    code.operands[0] = Operand::SmallConstant { value: 1 };
    ret(code, machine);
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

// stores that aren't stores trip me up, honestly
pub fn store(code: &mut OpCode, machine: &mut ZMachine) {

    let (variable, value) = (code.operands[0].get_value(), code.operands[1].get_value());

    machine.store_variable(variable as u8, value);
    // done

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

    code.branch = true;

    let (object, attribute) = (code.operands[0].get_value(), code.operands[1].get_value());

    code.result = machine.get_object_view(object)
        .has_attribute(attribute) as u16;

}

pub fn verify(code: &mut OpCode, machine: &mut ZMachine) {
    unimplemented!();
}
