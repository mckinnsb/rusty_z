pub mod zinterface;

#[cfg(not(target_os = "emscripten"))]
pub mod cli;

#[cfg(target_os = "emscripten")]
pub mod web;

