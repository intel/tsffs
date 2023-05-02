use raffl_macro::callback_wrappers;

struct Test {}

#[callback_wrappers]
impl Test {
    pub fn test(&self, a: i32, b: i32) -> i32 {
        a + b
    }
}

fn main() {}
