#![crate_type = "lib"]

pub unsafe fn size_of_units<T: Sized>() -> [(); std::mem::size_of::<T>()] {
    //~^ ERROR the size for values of type `T` cannot be known at compilation time
    [(); std::mem::size_of::<T>()]
}
