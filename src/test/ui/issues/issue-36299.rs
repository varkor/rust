#![allow(unused_lifetimes)]

struct Foo<'a, A> {}
//~^ ERROR parameter `'a` is never used
//~| ERROR parameter `A` is never used

fn main() {}
