This macro will determine the path to a folder or file in the generated assets folder at compile time.

# Example
```rust
    // Will resolve to '$ASSETS_PATH/a/b/c'
    asset_path!("a", "b", "c");

    // Will resolve to '$ASSETS_PATH/path/to/my/asset.json'
    asset_path!("path", "to", "my", "asset.json");
```