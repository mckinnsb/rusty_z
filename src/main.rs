pub mod interfaces;
pub mod zmachine;

extern crate rand;
#[cfg(target_os = "emscripten")]
extern crate stdweb;

use std::{cell::*, rc::*};

use interfaces::zinterface::*;
use zmachine::zmachine::*;

#[cfg(target_os = "emscripten")]
use {
    interfaces::web::{WebInterface, WebStream, WebUpdate},
    stdweb::*,
};

#[cfg(target_os = "emscripten")]
js_serializable!(WebUpdate);

#[cfg(not(target_os = "emscripten"))]
use interfaces::cli::CliInterface;

fn main() {
    // machine now takes ownership of the cloned data buffer
    // its mut, because next_instruction can change the
    // state of the machine. which makes complete sense
    let data = get_program();

    let mut interface = get_interface();
    interface.clear();

    let mut machine = ZMachine::new(data, interface);
    let interface = Rc::clone(&machine.zinterface);

    // the loop setup has to happen in main() or a function called from main()
    // if we are using a closure, because of the static lifetime requirement
    // when we use a reference, which is automatic when passing a function
    //
    // note: cli blocks here for now
    interface.setup_loop(move || main_loop(&mut machine));

    #[cfg(target_os = "emscripten")]
    spawn_local(async move {
        let mut stream = WebStream::new(&interface.publisher);
        stream.subscribe().await;
    });
}

#[cfg(not(target_os = "emscripten"))]
pub fn get_interface() -> CliInterface {
    CliInterface {}
}

#[cfg(target_os = "emscripten")]
pub fn get_interface() -> WebInterface {
    WebInterface::new()
}

pub fn get_program() -> Vec<u8> {
    // we use the include_bytes! macro because it is cross-compatible
    // with asm.js - this embeds the bytes in the js file.
    //
    // we could probably split this out later using CFG to
    // lower the size of the desktop binary,
    // and maybe look at loading a file remotely in the future

    let data = include_bytes!("../Zork1.dat");

    // we then get a reference to that static array of bytes as a slice
    let data_ref: &[u8] = &data[..];

    //we then copy it, using to_vec
    let data_vec = data_ref.to_vec();

    if data_vec.len() <= 0 {
        panic!("Could not read file!");
    }

    data_vec
}

pub fn main_loop<T: ZInterface>(machina: &mut ZMachine<T>) -> u8 {
    while let MachineState::Running = machina.state.clone() {
        machina.next_instruction();
    }

    match machina.state.clone() {
        MachineState::Restarting => {
            return LoopState::Restarting as u8;
        }
        MachineState::Stopped => {
            return LoopState::Quitting as u8;
        }
        MachineState::TakingInput { ref callback } => {
            machina.wait_for_input(callback.clone());
        }
        _ => (),
    };

    return LoopState::Running as u8;
}
