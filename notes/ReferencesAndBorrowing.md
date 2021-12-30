# References and Borrowing in Rust

## Problem
When we pass variables to different functions, we also have to transfer ownership, thus making previous pointers invalid.

This can be quite tedious if we want to use the variable we passed after the function terminates.

One fix for this would be to have the function return the value we lent, so that we can use it again, but this is cumbersome

**Is there a way for us to pass a variable to a function without transferring ownership?**

**Answer: Yes.** We do this with references.


In Rust, we use `&x` symbols to denote a reference to a variable to `x`.

Here is an example of how we use references.

```rust
fn main(){
    let s1 = String::from("hello");

    let len = calculate_length(&s1);

    println!("The length of '{}' is {}.", s1, len);
}

fn calculate_length(s: &String){
    s.len();
}
```
In this case, `s` holds a pointer which points to the pointer held in `s1`.

The `&s1` creates a reference that refers to the value of `s1`.


We call the action of creating a reference **borrowing**.


We can **not** modify something we are borrowing without making the reference mutable (you also have to make the original variable mutable).

- This comes with a catch: You can only have **one** mutable reference to a single piece of data at a time within the same scope.
- You also can't have mutable and immutable references existing within the same scope.

So the following code will fail

```rust
let mut s = String::from("hello");

let r1 = &mut s;
let r2 = &mut s;

```


The benefit of this restriction is that it prevents data races at compile time.

## The Rules of References

1. At any given time, you can either have one mutable reference or any
number of immutable References
2. References must always be valid.

