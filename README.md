# learning-rust


### Traits
#### Derive vs Impl traits  
https://stackoverflow.com/questions/64393455/what-is-difference-between-derive-attribute-and-implementing-traits-for-structur  
https://www.reddit.com/r/rust/comments/h8bpj6/a_very_basic_question_in_regard_to_derive_and_impl/  
##### Explanation:
Derive is using a macro to implement trait into struct- cannot do it for Display because there is no one-way to display signed integers for example


### Generics
```rust
struct MyBox<T>(T);

impl<T> MyBox<T> {
    fn new(x: T) -> MyBox<T> {
        MyBox(x)
    }
}
```

#### Explanation:

```rust
// Declaration of a struct that takes in a generic and creates a tuple with that particular generic type
// {} are missing because everything can be done in a single line declaration
struct MyBox<T>(T);

// impl<T> declares the use of generic parameter
//       here is where you can enter generics (either lifetimes or types, eg 'a, T, etc)
// MyBox<T> declares what type the below methods are implemented for
impl<T> MyBox<T> {
    fn new(x: T) -> MyBox<T> {
        MyBox(x)
    }
}
```
