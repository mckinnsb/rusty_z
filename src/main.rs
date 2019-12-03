pub mod interfaces;
pub mod zmachine;

extern crate rand;

use interfaces::zinterface::ZInterface;
use zmachine::input_handler::*;
use zmachine::zmachine::*;

#[cfg(target_os = "emscripten")]
use interfaces::web::WebInterface;

#[cfg(not(target_os = "emscripten"))]
use interfaces::cli::CliInterface;

fn main() {
    // machine now takes ownership of the cloned data buffer
    // its mut, because next_instruction can change the
    // state of the machine. which makes complete sense
    let data = get_program();

    #[cfg(not(target_os = "emscripten"))]
    let machine = ZMachine::new(data, CliInterface {});

    #[cfg(target_os = "emscripten")]
    let machine = ZMachine::new(
        data,
        WebInterface::new()
    );

    machine.zinterface.clear();
    machine.zinterface.set_loop();
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

#[cfg(not(target_os = "emscripten"))]
pub fn main_loop<T: ZInterface>(machina: &mut ZMachine<CliInterface>) {
    while let MachineState::Running = machina.state.clone() {
        machina.next_instruction();
    }

    match machina.state.clone() {
        MachineState::Restarting => {
            let data = get_program();
            *machina = ZMachine::new(data, CliInterface {});
            machina.next_instruction();
        }
        MachineState::Stopped => {
            machina.zinterface.quit();
        }
        MachineState::TakingInput { ref callback } => {
            machina.wait_for_input(callback.clone());
        }
        _ => (),
    };
}

#[cfg(target_os = "emscripten")]
pub fn main_loop<T: ZInterface>(machina: &mut ZMachine<WebInterface>) {
    while let MachineState::Running = machina.state.clone() {
        machina.next_instruction();
    }

    match machina.state.clone() {
        MachineState::Restarting => {
            let data = get_program();
            *machina = ZMachine::new(data, WebInterface::new());
            machina.next_instruction();
        }
        MachineState::Stopped => {
            machina.zinterface.quit();
        }
        MachineState::TakingInput { ref callback } => {
            machina.wait_for_input(callback.clone());
        }
        _ => (),
    };
}
