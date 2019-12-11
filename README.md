# rusty_z

A ZMachine interpreter written in Rust.

It's based off of the standards specification available here: http://inform-fiction.org/zmachine/standards/z1point1/index.html

The machine will compile for most regular targets that have a CLI (using `termion` as a terminal output in those cases), and will also compile for the `asmjs-unknown-emscripten` target, producing a javascript file that will expose a `RustyZ` object to the window. An example on how to use it is included (`index.html` / `index.js` ).

This is still a work in progress, but it should implement all of the Version 3 opcodes except save and restore. 

Other known issues/planned work:

* Some Version 3 games will break on certain commands; this is apparently due to some interpreters actually resolving undefined behavior. One example I can think of is that some games test attribute 0 of object 0, which always resolved to false in some interpreters, but there is no object 0. (This happens in HGTTG).
* Quit prompts and confirm prompts for non-`asmjs` move the cursor to the wrong location, causing the confirm/dialog to be cut off (classic terminal issue).
* Restart doesn't do anything (halts the system) on `asmjs-unknown-emscripten`. 
* You can't pick the story file at this point in time; it's hard coded into the binary. This was done for expedience; there's no requirement that the story file be a static `Vec` or anything. This is one part of the CLI/web interface that's going to be pretty tricky to sort out, particularly for the `asmjs` target.

The target is currently `asmjs-unknown-emscripten`, but I'd like to bring it to WASM someday.




