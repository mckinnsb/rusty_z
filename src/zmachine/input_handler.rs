use std::io::*;
use std::cell::*;
use std::rc::*;

#[cfg(target_os = "emscripten")]
extern crate stdweb;

pub trait LineReader {
    fn read_next_line(&mut self, buf: &mut String) -> Option<usize>;
}

impl LineReader for Stdin {
    fn read_next_line(&mut self, buf: &mut String) -> Option<usize> {
        match self.read_line(buf) {
            Ok(x) => Some(x),
            //discard the error
            Err(_) => None,
        }
    }
}

pub struct InputHandler<T: LineReader> {
    pub reader: T,
}


// we are going to change it so the emscriptem version outputs to a stream; 
// it's no longer going to pull an html element from the page
pub enum InputConfiguration {
    Standard,
    HTMLDocument
}

pub struct WebInputIndicator {
    pub input_sent: bool,
}

#[cfg(target_os = "emscripten")]
pub struct WebReader {
    //we need a bit of shared, mutable state between the callback
    //and the reader; the object needs to be shared and available
    //on the heap, so Rc<RefCell> it is
    pub indicator: Rc<RefCell<WebInputIndicator>>,
}

#[cfg(target_os = "emscripten")]
impl LineReader for WebReader {
    fn read_next_line(&mut self, _: &mut String) -> Option<usize> {

        //i have no idea why i have to do it this way -
        //in memory view we just use borrow and borrow_mut -
        //wondering if its the underlying data type (my own struct vs. vec)

        return None;

        /*
         * it's likely we will still need this mutex-like structure
         
        let input_sent = {
            let indicator: &RefCell<WebInputIndicator> = self.indicator.borrow();
            indicator.borrow().input_sent
        };

         */


        /* 
         * but using this is kind of unlikely
         
        match (self.initialized, input_sent) {
            (true, true) => {
                self.indicator.borrow_mut().input_sent = false;

                self.current_input = self.player_input.data_get("value").unwrap();

                buf.push_str(&self.current_input);
                Some(buf.len())
            }

            (false, _) => {
                self.initialized = true;

                let indicator = self.indicator.clone();

                self.form.on("submit", move |_| {
                    indicator.borrow_mut().input_sent = true;
                });

                None
            }

            _ => {
                //focus!
                self.player_input.focus();
                None
            }
        }

        */
    }
}

impl<T: LineReader> InputHandler<T> {
    pub fn get_input(&mut self) -> Option<String> {
        // 64 characters is probably a pretty reasonable start
        let mut input = String::with_capacity(64);
        let result = self.reader.read_next_line(&mut input);

        let length = match result {
            Some(x) => x,
            // we ignore the error here, for now
            // im guessing we might need to panic in the future
            None => 0,
        };

        // no input, so return None
        if length == 0 {
            return None;
        };

        //we don't need to check for new line -
        //input handler takes care of that for us by dealing
        //with std:;in::io ( blocks until new line ) and
        //htmlevent (submits on return)
        //warn!( "READ INPUT:{}", input );
        Some(input)
    }
}
