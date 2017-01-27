#![feature(drop_types_in_const)]

//its a little strange FPS is a signed value, not
//sure what - would mean?
//
//values larger than 1000 seem to have no effect
//we might want to figure out a way to run more than one opcode
//at a time, potentially taking away control between input/sreads

const FPS: i32 = 1000;

pub mod zmachine;

//extern crate rusty_z;
extern crate rand;
use self::rand::*;

use std::io::*;
use std::rc::Rc;
use std::cell::*;
use std::process;
use std::thread;
use std::time;

use zmachine::main::*;
use zmachine::main::input_handler::*;

#[cfg(target_os="emscripten")]
extern crate webplatform;
#[cfg(target_os="emscripten")]
use webplatform::*;

#[cfg(not(target_os="emscripten"))]
#[macro_use]
extern crate log;
#[cfg(not(target_os="emscripten"))]
extern crate log4rs;

#[cfg(not(target_os="emscripten"))]
use log::LogLevelFilter;
#[cfg(not(target_os="emscripten"))]
use log4rs::append::file::*;
#[cfg(not(target_os="emscripten"))]
use log4rs::config::{Appender, Config, Logger, Root};



// this is unsafe - but the current implementation of emscripten using webplatform
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

static mut machine: Option<ZMachine<'static>> = None;
static mut data_buffer: Option<Vec<u8>> = None;
static mut input_config: Option<InputConfiguration<'static>> = None;

#[cfg(target_os="emscripten")]
static mut handler: Option<InputHandler<WebReader<'static>>> = None;

#[cfg(not(target_os="emscripten"))]
static mut handler: Option<InputHandler<std::io::Stdin>> = None;

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

    unsafe {

        //we then copy it, using to_vec
        data_buffer = Some(data_ref.to_vec());

        if data_buffer.as_ref().unwrap().len() <= 0 {
            panic!("Could not read file!");
        }

        // we can't get away from this cfg! call , it's the only way we really
        // use the same signature for both platforms
        // and not force desktop envs to install webplatform

        input_config = create_options();

        // machine now takes ownership of the cloned data buffer
        // its mut, because next_instruction can change the
        // state of the machine. which makes complete sense

        machine = Some(ZMachine::new(data_buffer.as_ref().unwrap().clone()));
        machine.as_mut().unwrap().clear();

        handler = Some(input_handler(input_config.as_ref().unwrap()));

    }

    set_loop();

}

#[cfg(not(target_os="emscripten"))]
fn create_options<'a>() -> Option<InputConfiguration<'a>> {
    Some(InputConfiguration::Standard)
}

#[cfg(target_os="emscripten")]
fn create_options<'a>() -> Option<InputConfiguration<'a>> {
    Some(InputConfiguration::HTMLDocument {
        html_doc: webplatform::init(),
        form_selector: String::from("form"),
        input_selector: String::from("#player_input"),
    })
}

#[cfg(not(target_os="emscripten"))]
fn input_handler(config: &InputConfiguration) -> InputHandler<std::io::Stdin> {
    let reader = std::io::stdin();
    InputHandler { reader: reader }
}

#[cfg(target_os="emscripten")]
fn input_handler<'a>(config: &InputConfiguration<'a>) -> InputHandler<WebReader<'a>> {

    match config {

        &InputConfiguration::HTMLDocument { ref html_doc,
                                            ref form_selector,
                                            ref input_selector } => {

            let form = html_doc.element_query(form_selector);
            let player_input = html_doc.element_query(input_selector);

            let reader = match (form, player_input) {
                (Some(form_element), Some(input_element)) => {
                    WebReader {
                        form: form_element,
                        player_input: input_element,
                        //we explicitly want something that will complain if used
                        current_input: String::with_capacity(0),
                        initialized: false,
                        indicator: Rc::new(RefCell::new(WebInputIndicator { input_sent: false })),
                    }
                }
                _ => panic!("element not found!"),
            };

            InputHandler { reader: reader }

        }

        _ => panic!("emscripten was given a non-html config!"),

    }

}

pub extern "C" fn main_loop() {

    unsafe {

        let machina = machine.as_mut().unwrap();

        while let x @ MachineState::Running = machina.state.clone() {
            warn!( "IP: {:x}", machina.current_ip() );
            machina.next_instruction();
        }

        match machina.state.clone() {
            MachineState::Restarting => {
                machine = Some(ZMachine::new(data_buffer.as_ref().unwrap().clone()));
                warn!( "IP: {:x}", machine.as_ref().unwrap().current_ip() );
                machine.as_mut().unwrap().next_instruction();
            }
            MachineState::Stopped => process::exit(0),
            MachineState::TakingInput { ref callback } => {
                machina.wait_for_input(handler.as_mut().unwrap(), callback.clone());
            },
            //this shouldn't happen
            _ => (),
        };

    }
}

#[cfg(not(target_os="emscripten"))]
fn set_loop() {

    //setup logger

    let logger = FileAppender::builder().build("log/dev.log").unwrap();
    let expanded = FileAppender::builder().build("log/expanded.log").unwrap();

    let config = Config::builder().
        appender(Appender::builder().build("main", Box::new(logger))).
        appender(Appender::builder().build("expanded", Box::new(expanded))).
        logger(Logger::builder().appender("expanded").additive(false).build("rusty_z::zmachine::main", LogLevelFilter::Info)).
        build(Root::builder().appender("main").build(LogLevelFilter::Warn)).
        unwrap();
    
    let handle = log4rs::init_config(config).unwrap();

    warn!( "log started" );

    loop {
        main_loop();
    }
}

#[cfg(target_os="emscripten")]
fn set_loop() {
    //emscripten_set_main_loop takes three parameters
    //
    //1) a function
    //
    //   this function needs to be:
    //
    //   1) extern
    //   2) have no parameters
    //
    //2) an integer, fps
    //
    //   if 0, will equiv. to using requestAnimationFrame.,
    //   this is actually not really desireable here as it will lock the loop
    //   to our refresh rate, which is going to be about 60hz_144hz, which means
    //   we will be running at 1/20th the speed of an apple IIe
    //
    //3) is infinite loop
    //
    //   honestly, this one is kind of weird ( because 0 will still be infinite loop )
    //   but it basically means "do you want main()'s context to remain when
    //   main_loop is called". so if false, every call will be static
    //   and without any environment or context ( and main is just used
    //   to set up this function, probably ), and if true, things like static
    //   variables and stuff allocated to the heap will stick around.
    unsafe {
        webplatform::emscripten_set_main_loop(main_loop, FPS, 1);
    }
}
