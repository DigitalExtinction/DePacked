# DePacked

[![crates.io](https://img.shields.io/crates/v/depacked.svg)](https://crates.io/crates/depacked)
[![crates.io](https://img.shields.io/crates/d/depacked.svg)](https://crates.io/crates/depacked)

## Example

```rust
use depacked::PackedData;

struct NeedToPack(u32);

fn main() {
    let mut packed = PackedData::with_max_capacity(1000);

    // Insertin is fast but not as CPU cache friendly.
    let first_item = packed.insert(NeedToPack(0));
    let second_item = packed.insert(NeedToPack(1));

    // Getting (mutable) references is fast and CPU cache friendly.
    let first_ref = packed.get(first_item);
    let second_ref_mut = packed.get_mut(second_item);

    // Removing might be slower.
    let first = packed.remove(first_item);
}
```

## License

DePacked is free and open source! All code in this repository is dual-licensed
under either:

* MIT License ([LICENSE-MIT](docs/LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))
* Apache License, Version 2.0 ([LICENSE-APACHE](docs/LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))

at your option.
