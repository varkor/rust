#![feature(const_generic_defaults)]

fn foo<const SIZE: usize = 5>() {}
                      //~^ ERROR expected one of `!`, `(`, `+`, `,`, `::`, `<`, or `>`, found `=`

fn main() {}
