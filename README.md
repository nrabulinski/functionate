# Functionate
This crate provides an attribute macro to implement `Fn` traits for any struct/enum,
essentially enabling you to use it as a regular function/pass it like a closure or use it like an overloaded function.

~~(Please don't use this in any real-world project)~~
## Why shouldn't I use this?
If the fact that you have to enable unstable features still doesn't convince you,
here's why you should stay away from this (or any other simillar patters) in any real project:
- It introduces ambiguity - What does this code do? Where do I find the implementation? Why am I calling a struct?
- It promotes rust anti-patterns - Rust doesn't allow you to, for example, overload functions in classical sense for a reason. You shouldn't try to do it.

If you're still positive about using this in your project - I'm trurly sorry. And I hope you won't try to implement this in the real-world.

Any more reasons not to use this are welcome.

## Example
```rust
#![feature(unboxed_closures, fn_traits)]
use functionate::functionate;

#[derive(Debug)]
struct MyFunc {
    state: i32,
}

#[functionate]
impl MyFunc {
    fn get_state(&self) -> i32 {
        self.state
    }

    fn update(&mut self, new: i32) {
        self.state += new;
    }
}

fn main() {
    let mut my_func = MyFunc { state: 5 };
    println!("{}", my_func()); // 5
    my_func(3);
    println!("{}", my_func()); // 8
    my_func(-8);
    println!("{}", my_func()); // 0
}
```
Firstly, we need to enable both `unboxed_closures` and `fn_traits` features.
Once we have those, we can use `#[functionate]` on an `impl` block with methods (it's important for all the methods to have `self`, `&mut self` or `&self` argument).

## Warning
Generics are only supported on `impl` level so if you try to make a method generic expect stuff to break.

Async methods are **not** supported at all *(yet)*.