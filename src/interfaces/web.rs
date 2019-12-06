extern crate serde_derive;
extern crate futures;

use std::cell::*;
use std::io::*;
use std::rc::*;
use std::pin::Pin;

use stdweb::*;

use self::serde_derive::{Serialize, Deserialize};

use self::futures::*;
use self::futures::task::*;

use super::super::zmachine::input_handler::*;
use super::zinterface::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct WebUpdate {
    pub source: String,
    pub content: String
}

pub struct WebPublisher {
    terminated: bool,
    updates: Vec<WebUpdate>,
}

impl WebPublisher {
    pub fn new() -> WebPublisher {
        let publisher = WebPublisher { 
            updates: Vec::<WebUpdate>::new(),
            terminated: false 
        };

        // man I never thought I would use the object literal pattern again..
        //
        // We have to do the callback "subscription" in javascript because
        // TryFrom is not implemented for function, as far as I can tell
        // you can't pass a javascript function to a rust function and then
        // have the rust function call that javascript function without
        // doing some manual labor / hoop jumping like this
        
        js! { @(no_return)
            window.RustyZ = window.RustyZ || {
                subscribe: function(callbackFn) {
                    this.callbackFn = callbackFn;
                }
            };
        };

        publisher
    }

    pub fn send(&mut self, update: WebUpdate) {
        self.updates.push(update);
    }

    pub async fn subscribe(&mut self) {
        js! {
            console.log("subscribing..");
        }

        let sub = self.by_ref().for_each(|x| {
            js! { @(no_return)
                console.log("value received by subscribe");
                console.log(@{&x});
                window.RustyZ.callbackFn(@{&x});
            };

            future::ready(())
        });

        sub.await;
    }
}

impl Stream for WebPublisher {
    type Item = WebUpdate;

    fn poll_next(self: Pin<&mut Self>, _: &mut Context) -> Poll<Option<WebUpdate>> {
        js! {
            console.log("poll next");
        }

        if self.terminated { return Poll::Ready(None); }

        js! {
            console.log("trying to pop");
        }

        return match self.get_mut().updates.pop() {
            None => Poll::Pending,
            Some(x) => Poll::Ready(Some(x))
        }
    }
}

// We are using Rc<RefCell> for the publisher as well as the indicator
// because we want to maintain the same calling interface - realistically
// printing to main doesn't change the interface handle, but it does
// change the publisher's state. 

pub struct WebInterface {
    pub indicator: Rc<RefCell<WebInputIndicator>>,
    pub publisher: Rc<RefCell<WebPublisher>>,
}

impl WebInterface {
    pub fn new() -> WebInterface {
        let interface = WebInterface {
            indicator: Rc::new(RefCell::new(WebInputIndicator { input_sent: false })),
            publisher: Rc::new(RefCell::new(WebPublisher::new()))
        };

        interface
    }
}

impl ZInterface for WebInterface {
    fn clear(&self) {}

    fn print_to_main(&self, str: &str) {
        js!{
            console.log(@{str});
        }

        self.publisher.borrow_mut().send(WebUpdate {
            source: "main".to_string(),
            content: str.to_string()
        });
    }

    fn print_to_header(&self, left_side: &str, right_side: &str) {
        self.publisher.borrow_mut().send(WebUpdate {
            source: "left".to_string(),
            content: left_side.to_string()
        });

        self.publisher.borrow_mut().send(WebUpdate {
            source: "right".to_string(),
            content: right_side.to_string()
        });
    }

    fn read_next_line(&self, buf: &mut String) -> Option<usize> {

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

    fn setup_loop<F>(&self, mut main_loop: F) -> LoopState
    where
        F: 'static + FnMut() -> u8,
    {
        js! { @(no_return)
            let callback = @{main_loop};
            
            function loop(time) {
                requestAnimationFrame(loop);
                callback();
            }

            requestAnimationFrame(loop);
        };


        LoopState::Running
    }
}

pub struct WebInputIndicator {
    pub input_sent: bool,
}
