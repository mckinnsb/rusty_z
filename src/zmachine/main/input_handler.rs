use std::io::*;
use std::rc::Rc;
use std::cell::*;
use std::borrow::Borrow;
use std::clone::Clone;

#[cfg(target_os="emscripten")]
extern crate webplatform;
#[cfg(target_os="emscripten")]
use self::webplatform::*;

pub trait LineReader {
    fn read_next_line(&mut self, buf: &mut String) -> Option<usize>;
}

impl LineReader for Stdin {
    fn read_next_line(&mut self, buf: &mut String) -> Option<usize> {
        match self.read_line(buf) {
            Ok(x) => Some(x),
            //discard the error
            Err(e) => None,
        }
    }
}

pub struct InputHandler<T: LineReader> {
    pub reader: T,
}

//im not sure how i feel about this right now;
//so this enum has two forms, one has a lifetime,
//one does not, but this means that all forms
//of this enum have to be treated as if they have
//lifetimes. no big deal - we use this at the top
//of the main loop, so this should survive for a while
//
//this sort of solves the problem of having different
//polymorphic inputs given different configurations,
//without having to load both libs on both targets
//( particularly problematic with emscripten, since lots of crates
//  don't even work - i doubt termion would compile ).

pub enum InputConfiguration<'a> {
    Standard,
    HTMLDocument {
        html_doc: Document<'a>,
        form_selector: String,
        input_selector: String,
    },
}

//this is PURE evil
//we are basically mocking out a type provided by emscripten to allow
//for the enum above to be used

#[cfg(not(target_os="emscripten"))]
pub struct Document<'a> {
    //i just picked this, it could be any type with a lifetime
    pub refer: Option<Ref<'a, String>>,
}

pub struct WebInputIndicator {
    pub input_sent: bool,
}


#[cfg(target_os="emscripten")]
pub struct WebReader<'a> {
    pub form: HtmlNode<'a>,
    pub player_input: HtmlNode<'a>,
    pub current_input: String,
    pub initialized: bool,

    //we need a bit of shared, mutable state between the callback
    //and the reader; the object needs to be shared and available
    //on the heap, so Rc<RefCell> it is
    pub indicator: Rc<RefCell<WebInputIndicator>>,
}

#[cfg(target_os="emscripten")]
impl<'a> LineReader for WebReader<'a> {
    fn read_next_line(&mut self, buf: &mut String) -> Option<usize> {

        //i have no idea why i have to do it this way -
        //in memory view we just use borrow and borrow_mut -
        //wondering if its the underlying data type (my own struct vs. vec)
        let input_sent = {
            let indicator: &RefCell<WebInputIndicator> = self.indicator.borrow();
            indicator.borrow().input_sent
        };

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

                self.form.on("submit",
                             move |_| { indicator.borrow_mut().input_sent = true; });

                None

            }

            _ => {

                //focus!
                self.player_input.focus();
                None

            }

        }

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
