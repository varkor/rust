// edition:2018

#![deny(keyword_idents)]

macro_rules! r#await {
    () => { println!("Hello, world!") }
}

fn main() {
    await!()
    //~^ ERROR `await` is a keyword
    //~^^ ERROR this was previously accepted by the compiler but is being phased out
}
