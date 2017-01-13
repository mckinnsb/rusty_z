#[cfg(target_os="emscripten")]
extern crate webplatform;

extern crate rusty_z;

use std::io::*;
use rusty_z::zmachine::main::ZMachine;
use rusty_z::zmachine::main::MachineState;
use rusty_z::zmachine::main::input_handler::*;

// use std::fs::File;
// use std::io::Read;

fn main() {

    // try to load the data

    // this is a static reference, we need a vec
    // we use the include_bytes! macro because it is cross-compatible
    // with asm.js
    //
    // we could probably split this out later using CFG to
    // lower the size of the desktop binary
    let data = include_bytes!("../data.z3");

    // this gets the file size from the static string
    let file_size = data.len();

    // we then get a reference to that static string as a slice
    let data_ref: &[u8] = &data[..];

    // we then copy it as a vector using std::slice
    let data_buffer = data_ref.to_vec();

    println!("file read was {} bytes long", file_size);

    if data_buffer.len() <= 0 {
        panic!("Could not read file!");
    }

    // machine now takes ownership of the data buffer
    // its mut, because next_instruction can change the
    // state of the machine. which makes complete sense
    //
    // we could also maybe hide this my starting a coroutine or something,
    // but we would just be wrapping a mutable reference somewhere

    let mut handler = input_handler();
    let mut machine = ZMachine::new(data_buffer);
    let status = machine.header.get_status();

    display(&status);

    // this might really need to change
    loop {
        let state = machine.state.clone();
        match state {
            MachineState::Running => machine.next_instruction(),
            MachineState::Stopped => break,
            MachineState::TakingInput { ref callback } => {
                machine.wait_for_input(&mut handler, callback.clone())
            }
        };
    }

    println!("zmachine exited");

}

fn input_handler() -> InputHandler<std::io::Stdin> {
    let reader = std::io::stdin();

    InputHandler { reader: reader }
}

#[cfg(target_os="emscripten")]
fn display(text: &str) {
    let document = webplatform::init();
    let content = document.element_query("section#content");

    match content {
        Some(_) => content.unwrap().html_set(text),
        None => println!("Couldn't find specfied element!"),
    }
}

#[cfg(not(target_os="emscripten"))]
fn display(text: &str) {
    println!("{}", text);
}
