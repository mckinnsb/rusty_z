// represents the current zmachine
use super::header::*;

use std::rc::*;
use std::cell::RefCell;

pub struct ZMachine {
    // ALL of the memory, this represents the entire state of the machine
    // this is loaded in at first , then modified by save files, then
    // the game is run and dynamic memory then changes during play
    //
    // its broken into three parts: dynamic, static, and high.
    //
    // dynamic: all things that can change in game, including object trees
    // and inventory
    //
    // static: this contains grammar, actions, preactions, adjectives, and
    // the dictionary- basically defining the language of the game
    //
    // high: routines and static strings meant to be used by the machine
    //
    // the machine owns a reference to the memory, and typically
    // is the only person who asks for a mutable reference
    memory: Rc<RefCell<Vec<u8>>>,

    // the header, which actually reads the first 64 bytes in memory
    // above,
    //
    // everyone has access to it
    pub header: Header,
}

impl ZMachine {
    pub fn new(data: Vec<u8>) -> ZMachine {

        // we have to create an immutably reference
        // counted mutable reference in order to
        //
        // allow the parent to write and the children to
        // read from the same memory, since they both effectively
        // own it, this gives the memory over to a ref cell
        // which is wrapped by an rc ( making the ref cell immutable ),
        // and then passed around. the ref cell can only be accessed
        // read-only, and the inner vector data can only be mutably
        // borrowed if no one else is currently borrowing it;
        //
        // this means rust will not protect us from one
        // thread borrowing a mut when another borrows it immutably!
        //
        // we should be able to avoid this, however, by simply
        // using immutable calls in the child, and mutable calls
        // only in the parent

        let memory = Rc::new(RefCell::new(data));

        // we are going to give the reference to the header,
        // so it can read it

        let header = Header::create(memory.clone());

        ZMachine {
            memory: memory,
            header: header,
        }
    }

    pub fn get_version(&self) -> u8 {
        self.header.version
    }
}
