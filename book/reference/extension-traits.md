# Extension traits

In Grug, we make use of the **extension trait** pattern, which is well explained by [this video](https://youtu.be/qrf52BVaZM8).

To put it simply, a Rust library has two options on how to ship a functionality: _to ship a function_, or _to ship a trait_.

For instance, suppose our library needs to ship the functionality of converting Rust values to strings.

## Shipping a function

The library exports a function:

```rust
pub fn to_json_string<T>(data: &T) -> String
where
    T: serde::Serialize,
{
    serde_json::to_string(data).unwrap_or_else(|err| {
        panic!("failed to serialize to JSON string: {err}");
    })
}
```

The consumer imports the function:

```rust
use grug::to_json_string;

let my_string = to_json_string(&my_data)?;
```

## Shipping a trait

The library exports a trait, and implements the trait for all eligible types.

The trait is typically named `{...}Ext` where "Ext" stands for _extension_, because the effectively extends the functionality of types that implement it.

```rust
pub trait JsonSerExt {
    fn to_json_string(&self) -> String;
}

impl<T> JsonSerExt for T
where
    T: serde::Serialize,
{
    fn to_json_string(&self) -> String {
        serde_json::to_string(data).unwrap_or_else(|err| {
            panic!("failed to serialize to JSON string: {err}");
        })
    }
}
```

The consumer imports the trait:

```rust
use grug::JsonSerExt;

let my_string = my_data.to_json_string()?;
```

## Extension traits in Grug

We think the consumer's syntax with extension traits is often more readable than with functions. Therefore we use this pattern extensively in Grug.

In [grug-types](../../crates/types), we define functionalities related to hashing and serialization with following traits:

- `Borsh{Ser,De}Ext`
- `Proto{Ser,De}Ext`
- `Json{Ser,De}Ext`
- `HashExt`

Additionally, there are the following in [grug-apps](../../crates/apps), which provides gas metering capability to storage primitives including `Item` and `Map`, but they are only for internal use and not exported:

- `MeteredStorage`
- `MeteredItem`
- `MeteredMap`
- `MeteredIterator`
