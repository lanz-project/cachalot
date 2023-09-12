# Cachalot
#### ... *cache a lot*.

## Description
Caches the result of a function for a given range of indexes.

## Usage 
```rust
use cachalot::{cachalot, try_cachalot};

#[cachalot(root = ".my_store")]
pub async fn source(..keys, range: Range<u128>) -> impl Stream<Item = MyItem> {
    // your code
}

#[cachalot(root = ".my_store")]
pub async fn try_source(..keys, range: Range<u128>) -> impl Stream<Item = Result<MyItem, Err>> {
    // your code
}
```

## Requirements

```rust
// abstract function signature:
pub async fn my_fn(..keys, range: Range<u128>) -> impl Stream<Item = MyItem> {
    // your code
}
```

+ The function must be asynchronous.
+ The return value must implement [Stream](https://docs.rs/futures/latest/futures/stream/trait.Stream.html).
+ "MyItem" must implement [NoUninit](https://docs.rs/bytemuck/latest/bytemuck/trait.NoUninit.html) and [AnyBitPattern](https://docs.rs/bytemuck/latest/bytemuck/trait.AnyBitPattern.html) from [bytemuck](https://github.com/Lokathor/bytemuck).
+ "..keys" is a list of arguments that will be used as a storage key - each argument must implement Hash + Send + Sync + Copy (the best solution is to use ref or function-poiner as an argument).
+ The function must always satisfy the following requirement:
```rust
let a = // any
let b = // any
let k = // between a .. b
let m = // between a .. b

let left = my_fn(..keys, a..b);
let right = my_fn(..keys, a..k) + my_fn(..keys, k..m) + my_fn(..keys, m..b);

assert_eq!(left, right) 
```
