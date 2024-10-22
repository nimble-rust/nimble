# Release

## Release example

### Bump version of all local crates

```console
cargo release "0.0.0-dev" -v --workspace --no-publish --no-tag --no-push --no-confirm --execute
```

### Publish

```console
cargo deps-order --workspace-only --exec "cargo publish" --wait 5
```

## Check line counts

```console
tokei crates/ --files --sort code --type rust
```
