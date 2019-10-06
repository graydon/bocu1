extern crate cc;

fn main() {
    cc::Build::new().file("src/bocu1.c").compile("IBMbocu1");
}
