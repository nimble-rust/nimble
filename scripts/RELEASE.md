# Release

## Release example

```console
cargo release "0.0.0-dev" -v --workspace --no-publish --no-tag --no-push --execute
```

```console
cargo release "0.0.13-dev" -v --workspace --no-tag --no-push --execute
```

Publish order:

nimble-steps
nimble-assent
nimble-blob-stream
nimble-participant
seq-map
nimble-step-types
nimble-sample-step
nimble-protocol
nimble-client-logic
nimble-host-logic
nimble-ordered-datagram
nimble-protocol-header
nimble-layer
nimble-host
nimble-seer
nimble-rectify
nimble-sample-game
time-tick
nimble-client
nimble-wrapped-step
nimble-rust

## Check line counts

```console
tokei crates/ --files --sort lines --type rust
```
