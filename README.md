# rusty_z

A ZMachine interpreter written in Rust.

It's based off of the standards specification available here: http://inform-fiction.org/zmachine/standards/z1point1/index.html

The machine will compile for most regular targets that have a CLI (using `termion` as a terminal output in those cases), and will also compile for the `asmjs-unknown-emscripten` target, producing a javascript file that will expose a `RustyZ` object to the window. An example on how to use it is included (`index.html` / `index.js` ).

This is still a work in progress, but it should implement all of the Version 3 opcodes except save and restore. This means it should be able to play most Version 3 games that used Inform compilers that used opcodes in a "standard" manner. For instance, I know you can finish Zork I, and I'm fairly certain you can finish II and III as well. However, some games used their own tweaked interpeters to handle behavior that would normally be undefined.

Games known to not work:
* Hitchhiker's Guide To The Galaxy (this game tests attribute 0 of object 0, which returned false in patched interpreters, but there is no object 0 as object indexes start at 1, and thus panics on a standard interpreter)

Other known issues/planned work:

* Other workarounds for opcodes being used in strange ways.
* Quit prompts and confirm prompts for non-`asmjs` move the cursor to the wrong location, causing the confirm/dialog to be cut off (classic terminal issue).
* Restart doesn't do anything (halts the system) on `asmjs-unknown-emscripten`. 
* You can't pick the story file at this point in time; it's hard coded into the binary. This was done for expedience; there's no requirement that the story file be a static `Vec` or anything. This is one part of the CLI/web interface that's going to be pretty tricky to sort out, particularly for the `asmjs` target.
* There's technically a maximum number of commands you can enter at this point in time; this is the maximum size of the stream/stack, which is actually not truncated at runtime (yet). There are a few ways to do this, I'm currently thinking about a good solution.



