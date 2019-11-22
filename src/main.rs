//its a little strange FPS is a signed value, not
//sure what a negative value would mean?
//
//values larger than 1000 seem to have no effect
//we might want to figure out a way to run more than one opcode
//at a time, potentially taking away control between input/sreads

pub mod zmachine;

//extern crate rusty_z;
extern crate rand;

use zmachine::input_handler::*;
use zmachine::zmachine::*;

#[cfg(target_os = "emscripten")]
extern crate stdweb;
#[cfg(target_os = "emscripten")]
use std::rc::Rc;
#[cfg(target_os = "emscripten")]
use std::cell::*;

#[cfg(not(target_os = "emscripten"))]
use std::process;
#[cfg(not(target_os = "emscripten"))]
extern crate log;
#[cfg(not(target_os = "emscripten"))]
extern crate log4rs;

#[cfg(not(target_os = "emscripten"))]
use log::LogLevelFilter;
#[cfg(not(target_os = "emscripten"))]
use log4rs::append::file::*;
#[cfg(not(target_os = "emscripten"))]
use log4rs::config::{Appender, Config, Logger, Root};

// this is unsafe - but the current implementation of emscripten on rust
// does not allow passing or currying arguments to the main loop callback function,
// and rather than try to go down that rabbit hole, we are going to use a static
// mut zmachine
//
// at first rust was very against things living "oustide" of main - this has
// since changed, but its still a good idea to keep things in main when possible
//
// things like GL and JS/hardware integration change that though,
// when you are interacting with a C api that expects you to set an extern
// callback in main and then wait for instructions - if you loop
// endlessly in emscripten, for instance, that creates an endless spin
// of doom

// its actually probably far more standard to use Option<Box<T>>s.
// might change over to that soon

static mut MACHINE: Option<ZMachine> = None;
static mut DATA_BUFFER: Option<Vec<u8>> = None;

#[cfg(target_os = "emscripten")]
static mut HANDLER: Option<InputHandler<WebReader>> = None;

#[cfg(not(target_os = "emscripten"))]
static mut HANDLER: Option<InputHandler<std::io::Stdin>> = None;

fn main() {
    // try to load the data

    // this is a static reference, we need a vec
    // we use the include_bytes! macro because it is cross-compatible
    // with asm.js
    //
    // we could probably split this out later using CFG to
    // lower the size of the desktop binary
    let data = include_bytes!("../Zork1.dat");

    // we then get a reference to that static string as a slice
    let data_ref: &[u8] = &data[..];

    unsafe {
        //we then copy it, using to_vec
        DATA_BUFFER = Some(data_ref.to_vec());

        if DATA_BUFFER.as_ref().unwrap().len() <= 0 {
            panic!("Could not read file!");
        }

        // machine now takes ownership of the cloned data buffer
        // its mut, because next_instruction can change the
        // state of the machine. which makes complete sense

        MACHINE = Some(ZMachine::new(DATA_BUFFER.as_ref().unwrap().clone()));
        MACHINE.as_mut().unwrap().clear();

        HANDLER = Some(input_handler(create_options().as_ref().unwrap()));
    }

    set_loop();
}

#[cfg(not(target_os = "emscripten"))]
fn create_options<'a>() -> Option<InputConfiguration> {
    Some(InputConfiguration::Standard)
}

#[cfg(target_os = "emscripten")]
fn create_options<'a>() -> Option<InputConfiguration> {
    Some(InputConfiguration::HTMLDocument)
}

#[cfg(not(target_os = "emscripten"))]
fn input_handler(_: &InputConfiguration) -> InputHandler<std::io::Stdin> {
    let reader = std::io::stdin();
    InputHandler { reader: reader }
}

#[cfg(target_os = "emscripten")]
fn input_handler(config: &InputConfiguration) -> InputHandler<WebReader> {
    match config {
        &InputConfiguration::HTMLDocument {} => {
            let reader = WebReader {
                indicator: Rc::new(RefCell::new(WebInputIndicator { input_sent: false })),
            };

            InputHandler { reader: reader }
        }

        _ => panic!("emscripten was given a non-html config!"),
    }
}

pub extern "C" fn main_loop() {
    unsafe {
        let machina = MACHINE.as_mut().unwrap();

        while let MachineState::Running = machina.state.clone() {
            machina.next_instruction();
        }

        match machina.state.clone() {
            MachineState::Restarting => {
                MACHINE = Some(ZMachine::new(DATA_BUFFER.as_ref().unwrap().clone()));
                MACHINE.as_mut().unwrap().next_instruction();
            }
            MachineState::Stopped => {
                quit();
            }
            MachineState::TakingInput { ref callback } => {
                machina.wait_for_input(HANDLER.as_mut().unwrap(), callback.clone());
            }
            //this shouldn't happen
            _ => (),
        };
    }
}

#[cfg(not(target_os = "emscripten"))]
fn quit() {
    process::exit(0);
}

#[cfg(target_os = "emscripten")]
fn quit() {
    // UPDATE: not sure what we will replace this with re: stdweb, maybe exit runtime will be available

    // this is a no-op in emscripten because in our current version
    // (webplatform, which is super old and we need to transition away from),
    // we can't use EXIT_RUNTIME=1 (which allows a WASM program to end the WASM runtime)
    //
    // the issue here, i believe, is that normally, multiple WASM programs run
    // in the same runtime, and if you allow one of them to shut it down, you allow that
    // process to be able to end all other processes, and so this has to be explicit.
    //
    // so we just pause it.. forever.
}

#[cfg(not(target_os = "emscripten"))]
fn set_loop() {
    //setup logger

    let logger = FileAppender::builder().build("log/dev.log").unwrap();
    let expanded = FileAppender::builder().build("log/expanded.log").unwrap();

    let config = Config::builder()
        .appender(Appender::builder().build("main", Box::new(logger)))
        .appender(Appender::builder().build("expanded", Box::new(expanded)))
        .logger(
            Logger::builder()
                .appender("expanded")
                .additive(false)
                .build("rusty_z::zmachine", LogLevelFilter::Info),
        )
        .build(Root::builder().appender("main").build(LogLevelFilter::Warn))
        .unwrap();

    log4rs::init_config(config).unwrap();

    loop {
        main_loop();
    }
}

#[cfg(target_os = "emscripten")]
fn set_loop() {
}
