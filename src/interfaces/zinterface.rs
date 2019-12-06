extern crate serde_derive;

use self::serde_derive::{Serialize};

#[derive(Copy, Clone, Serialize)]
#[repr(u8)]
pub enum LoopState {
    Running = 0,
    Restarting = 1,
    Quitting = 2,
    Error = 3,
}

impl From<u8> for LoopState {
    fn from(orig: u8) -> Self {
        match orig {
            0 => return LoopState::Running,
            1 => return LoopState::Restarting,
            2 => return LoopState::Quitting,
            3 => return LoopState::Error,
            _ => return LoopState::Error,
        }
    }
}

pub trait ZInterface: Sized {
    fn quit(&self);
    fn clear(&self);
    fn read_next_line(&self, buf: &mut String) -> Option<usize>;
    fn print_to_main(&self, main: &str);

    // It's interesting, the ZMachine actually has this concept embedded in the opcodes;
    // show_status points to two objects that each must be displayed on the top left
    // and top right.
    //
    // This is interesting be cause it ALSO has opcodes for creating split/secondary
    // screens, and doesn't use this for that purpose; it's possible implementation
    // wasn't ready for Zork I/II/III (but other version 3 games make heavy use of it)
    fn print_to_header(&self, left_side: &str, right_side: &str);
    fn setup_logging(&self);
    fn setup_loop<F>(&self, main_loop: F) -> LoopState
    where
        F: 'static + FnMut() -> u8;
}
