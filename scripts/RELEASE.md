# Release

## Release example
```console
cargo release "0.0.0-dev" -v --workspace --no-publish --no-tag --no-push --execute
```


```console
cargo release "0.0.13-dev" -v --workspace --no-tag --no-push --execute
```

Publish order:

* nimble-steps
* nimble-assent
* nimble-blob-stream
* nimble-participant
* nimble-step-types
* nimble-protocol
* nimble-participant-steps
* nimble-seer
* nimble-rectify
* nimble-sample-step
* nimble-client-logic
* nimble-host
* nimble-ordered-datagram
* datagram-pinger
* nimble-client
* nimble-rust


## Check line counts

```console
tokei crates/ --files --sort lines --type rust
```
