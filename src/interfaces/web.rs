extern crate stdweb;

use std::cell::*;
use std::io::*;
use std::rc::*;

use super::super::zmachine::input_handler::*;
use super::zinterface::ZInterface;

pub struct WebInterface {
    pub indicator: Rc<RefCell<WebInputIndicator>>,
}

impl WebInterface {
    pub fn new() -> WebInterface {
        WebInterface {
            indicator: Rc::new(RefCell::new(WebInputIndicator { input_sent: false })),
        }
    }
}

impl ZInterface for WebInterface {
    fn clear(&self) {}

    fn print_to_main(&self, str: &str) {}

    fn print_to_header(&self, left_side: &str, right_side: &str) {}

    fn read_next_line(&self, buf: &mut String) -> Option<usize> {
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

    fn quit(&self) {}

    fn setup_logging(&self) {}
}

pub struct WebInputIndicator {
    pub input_sent: bool,
}
