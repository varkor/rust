#![allow(unused_lifetimes)]

fn main<'a>() {}
//~^ ERROR `main` function is not allowed to have generic parameters [E0131]
