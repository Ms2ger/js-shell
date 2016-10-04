#[macro_use]
extern crate js;

mod script;

fn main() {
    let thread = script::start();
    script::evaluate_script(&thread, r#"println("Hello, world!")"#);
    script::shutdown(thread);
}
