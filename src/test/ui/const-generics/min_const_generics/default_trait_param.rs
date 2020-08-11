#![crate_type = "lib"]
#![feature(const_generic_defaults)]

trait Foo<const KIND: bool = true> {}
