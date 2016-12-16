#[cfg(target_os="emscripten")]
extern crate webplatform;

fn main() {
    display("What's up world!");
}


#[cfg(target_os="emscripten")]
fn display(text: &str) {
    let document = webplatform::init();
    let content = document.element_query("section#content");

    match content {
        Some(_) => content.unwrap().html_set(text),
        None => println!("Couldn't find specfied element!"),
    }
}

#[cfg(not(target_os="emscripten"))]
fn display(text: &str) {
    println!("{}", text);
}
