# learning-rust

### QoL
#### Cargo Watch
```powershell
cargo install cargo-watch
cargo watch -q -c -w src/ -x 'run'
cargo watch -q -c -w src/ -x 'test'
```

#### Cargo Watch Alias
```powershell
# Using notepad popup below, place script within
notepad $profile

function cargoWatchR {
    cargo watch -q -c -w src/ -x 'run'
}

function cargoWatchT {
    cargo watch -q -c -w src/ -x 'test'
}

function cargoWatchTP($a) {
    cargo watch -q -c -w src/ -x "test $a"
}

function cargoRun {
    cargo run
}

function wasmPackWeb {
    wasm-pack build --target web
}

Set-Alias cwr cargoWatchR
Set-Alias cwt cargoWatchT
Set-Alias cwtp cargoWatchTP
New-Alias cr cargoRun
New-Alias wpw wasmPackWeb

# enable ps scripts in windows
```

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

### Parameters
#### Turbofish
```rust
fn main() {
    let s = "Hello, World!";
// ::<String> is called turbofish- used to tell rust what type this function should be converted into
    let string = s.into::<String>();
}
```


#### Windows + WSL
Need to install this before being able to build anything
```zsh
sudo apt-get update
sudo apt install build-essential libssl-dev pkg-config
```
