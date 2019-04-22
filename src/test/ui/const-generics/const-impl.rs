// run-pass

#![feature(const_generics)]
//~^ WARN the feature `const_generics` is incomplete and may cause the compiler to crash

trait Trait {
    fn test_a(&self) -> usize;

    fn test_b(&self) -> usize;
}

impl<T, const N: usize> Trait for [T; N] {
    fn test_a(&self) -> usize {
        0
    }

    fn test_b(&self) -> usize {
        self.len()
    }
}

fn main() {}
