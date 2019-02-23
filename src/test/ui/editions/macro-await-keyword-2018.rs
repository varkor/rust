// edition:2018
// run-pass

macro_rules! m {
    ($($t:tt)*) => {};
}
m! { async try dyn await }

fn main() {}
