// run-pass

#![feature(destructuring_assignment)]

struct TupleStruct<S, T>(S, T);

fn main() {
    let (mut a, mut b);
    TupleStruct(a, b) = TupleStruct(0, 1);
    assert_eq!((a,b), (0,1));
    TupleStruct(a, .., b) = TupleStruct(1,2);
    assert_eq!((a,b), (1,2));
}
