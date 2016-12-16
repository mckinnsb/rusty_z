#[cfg(feature="web")]
extern crate webplatform;

fn main() {
    display("What's up world!");
}

#[cfg(feature="web")]
fn display(text: &str) {
    let document = webplatform::init();
    let content = document.element_query("section#content");

    match content {
        Some(_) => content.unwrap().html_set(text),
        None => println!("Couldn't find specfied element!"),
    }
}

#[cfg(not(feature="web"))]
fn display(text: &str) {
    println!("{}", text);
}
