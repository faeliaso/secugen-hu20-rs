# Hardware validation guide

Do not include fingerprint images, raw frame bytes, or image hashes in source
control, CI artifacts, issue reports, or logs.

## Before protocol implementation

Follow the physical-validation gate in
[the protocol record](PROTOCOL_REVERSE_ENGINEERING.md). Record only USB
metadata and command semantics.

## After protocol implementation

1. Run `make validate` on Apple Silicon. Linux x86_64 and Linux aarch64 are
   verified by the CI pipeline without a physical scanner.
2. Confirm `cargo run --example list_devices` reports only `1162:2200`.
3. Run `cargo run --example led_probe` and confirm the LED turns on then off.
4. Run `cargo run --example capture_metadata` and confirm 300×400 at 500 DPI.
5. Run `cargo run --example capture_pgm -- /secure/local/path/frame.pgm`.
6. Visually inspect the locally stored image, then securely handle it according
   to the operator's biometric-data policy.

Record OS/version, crate version, USB topology, PID, observed geometry,
success/failure, and sanitized logs only.
