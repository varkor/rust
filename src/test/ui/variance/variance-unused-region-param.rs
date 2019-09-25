// Test that we report an error for unused type parameters in types.

struct SomeStruct<'a> { x: u32 } //~ ERROR parameter `'a` is never used
enum SomeEnum<'a> { Nothing } //~ ERROR parameter `'a` is never used
trait SomeTrait<'a> { fn foo(&self); } //~ WARN parameter `'a` is never used

fn main() {}
