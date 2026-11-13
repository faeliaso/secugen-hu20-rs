# Release pipeline

Do not publish before physical protocol validation is complete.

1. Update the manifest version and changelog.
2. Run `make validate`, `cargo test --doc`, and `cargo build --release`.
3. Merge a reviewed release change, tag it as `vX.Y.Z`, and wait for Release
   Check to pass.
4. Run the manual Publish workflow with the same tag and confirmation text
   `PUBLISH`. Store `CARGO_REGISTRY_TOKEN` only in the protected `crates-io`
   GitHub environment.
