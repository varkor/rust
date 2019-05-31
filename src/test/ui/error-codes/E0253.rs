// run-pass

mod foo {
    pub trait MyTrait {
        fn do_something();
    }
}

use foo::MyTrait::do_something; // ok

fn main() {}
