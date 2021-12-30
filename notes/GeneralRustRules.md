# General Language Rules in Rust

1. Variables are immutable by default, we can make them mutable with the `mut` keyword.
2. If you are creating a binary executable, add Cargo.lock to git. If not, add
it to the .gitignore.
3. `String` is the dynamic heap string type. Use it when you need to own or modify your string data.
4. `&str` points us to the beginning of a chunk of a string.
5. The Rust compiler needs to know the size of each variable at compile time.
6. String literals are string slices
- Take this code for example
```rust
let s = "Hello, World!";
```
- Recall that string literals are stored directly into the binary
- The type of `s` here is `&str`, so it's pointing to specific chunk in the binary
`&str` is an immutable reference.
7. References to `String` are also equivalent to whole slices of `String`