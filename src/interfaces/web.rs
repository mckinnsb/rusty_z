extern crate futures;
extern crate serde_derive;

use std::{cell::*, pin::Pin, rc::*};

use stdweb::*;

use self::{
    futures::task::*,
    futures::*,
    serde_derive::{Deserialize, Serialize},
};

use super::zinterface::*;

pub struct WebInputIndicator {
    pub input_sent: bool,
}

// We are using Rc<RefCell> for the publisher as well as the indicator
// because we need to be able to create a weak reference to the publisher
// directly, the entire interface owned by the ZMachine as an Rc, so if we
// want to only grab the publisher, we need to set it up in memory so that
// it maintains it's own static reference to a mutable object, this way,
// we can create Weak references to publisher by just passing around the Rc

pub struct WebInterface {
    pub indicator: Rc<RefCell<WebInputIndicator>>,
    pub publisher: Rc<RefCell<WebPublisher>>,
}

pub struct WebPublisher {
    updates: Vec<WebUpdate>,
}

pub struct WebStream {
    terminated: bool,
    index: u8,
    stream: Weak<RefCell<WebPublisher>>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct WebUpdate {
    pub source: String,
    pub content: String,
}

impl WebInterface {
    pub fn new() -> WebInterface {
        let interface = WebInterface {
            indicator: Rc::new(RefCell::new(WebInputIndicator { input_sent: false })),
            publisher: Rc::new(RefCell::new(WebPublisher::new())),
        };

        interface
    }
}

impl ZInterface for WebInterface {
    fn clear(&self) {}

    fn print_to_main(&self, str: &str) {
        js! { console.log("print to main:" + @{str}); }

        self.publisher.borrow_mut().send(WebUpdate {
            source: "main".to_string(),
            content: str.to_string(),
        });
    }

    fn print_to_header(&self, left_side: &str, right_side: &str) {
        js! { console.log("print to right header:" + @{right_side}); }

        self.publisher.borrow_mut().send(WebUpdate {
            source: "left".to_string(),
            content: left_side.to_string(),
        });

        self.publisher.borrow_mut().send(WebUpdate {
            source: "right".to_string(),
            content: right_side.to_string(),
        });
    }

    fn read_next_line(&self, buf: &mut String) -> Option<usize> {
        return None;
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

impl WebPublisher {
    pub fn new() -> WebPublisher {
        let publisher = WebPublisher {
            updates: Vec::<WebUpdate>::new(),
        };

        // man I never thought I would use the object literal pattern again..
        //
        // We have to do the callback "subscription" in javascript because
        // TryFrom is not implemented for function, as far as I can tell
        // you can't pass a javascript function to a rust function and then
        // have the rust function call that javascript function without
        // doing some manual labor / hoop jumping like this

        js! { @(no_return)
            window.RustyZ = window.RustyZ || {};
            window.RustyZ.subscribe = function(callbackFn) {
                this.callbackFn = callbackFn;
            };

            window.RustyZ.subscribe(function(e) {
                console.log(e);
            });
        };

        publisher
    }

    pub fn send(&mut self, update: WebUpdate) {
        self.updates.push(update);
    }
}

impl WebStream {
    pub fn new(publisher: &Rc<RefCell<WebPublisher>>) -> WebStream {
        WebStream {
            terminated: false,
            index: 0,
            stream: Rc::downgrade(publisher),
        }
    }

    pub async fn subscribe(&mut self) {
        self.by_ref()
            .for_each(|x| {
                js! { @(no_return)
                    window.RustyZ.callbackFn(@{&x});
                };

                future::ready(())
            })
            .await
    }
}

impl Stream for WebStream {
    type Item = WebUpdate;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context) -> Poll<Option<WebUpdate>> {
        if self.terminated {
            return Poll::Ready(None);
        }

        let update = {
            let stream = match self.stream.upgrade() {
                None => return Poll::Ready(None),
                Some(x) => x,
            };

            let result = match stream.borrow().updates.get(self.index as usize) {
                None => None,
                Some(x) => {
                    self.get_mut().index += 1;
                    Some(x.clone())
                }
            };

            result
        };

        match update {
            None => {
                let waker = Rc::new(context.waker().clone());
                let wake_up = move || {
                    waker.wake_by_ref();
                };

                // note we are doing context re-awakening almost entirely in
                // javascript

                js! {
                    window.RustyZ = window.RustyZ || {};

                    var RustyZ = window.RustyZ;

                    if (RustyZ.wakeId) {
                        if (RustyZ.wakeFunction) {
                            RustyZ.wakeFunction.drop();
                        }

                        window.cancelAnimationFrame(RustyZ.wakeId);
                    }

                    RustyZ.wakeFunction = @{wake_up};

                    RustyZ.wakeId = window.requestAnimationFrame(function() {
                        RustyZ.wakeFunction();
                        RustyZ.wakeFunction.drop();
                        RustyZ.wakeFunction = null;
                        RustyZ.wakeId = null;
                    });
                }

                Poll::Pending
            }
            Some(x) => Poll::Ready(Some(x)),
        }
    }
}
