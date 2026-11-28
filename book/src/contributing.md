# Contributing

How to contribute is in [CONTRIBUTING.md](https://github.com/walteraandrade/ginga/blob/main/CONTRIBUTING.md), and the rules for a change are in [AGENTS.md](https://github.com/walteraandrade/ginga/blob/main/AGENTS.md), for people and coding agents alike. The checks to run before pushing:

```sh
cargo fmt --all
cargo clippy --all-targets -- -D warnings                 # the engine
cargo test                                                # the engine, in seconds
cargo clippy -p ginga-gpui --all-targets -- -D warnings   # the GPUI adapter; the first build takes minutes
cargo test -p ginga-gpui                                  # the adapter on GPUI's test platform, no display
```

The engine needs Rust 1.87 or newer and the adapter Rust 1.95, the oldest that builds its GPUI. CI checks each on that version, on every pull request.

Everyone taking part follows the [code of conduct](https://github.com/walteraandrade/ginga/blob/main/CODE_OF_CONDUCT.md); report a security problem as [SECURITY.md](https://github.com/walteraandrade/ginga/blob/main/SECURITY.md) says.

## License

Either of [Apache-2.0](https://github.com/walteraandrade/ginga/blob/main/LICENSE-APACHE) or [MIT](https://github.com/walteraandrade/ginga/blob/main/LICENSE-MIT), at your option. Unless you say otherwise, a contribution you submit for inclusion, as defined in the Apache-2.0 license, is offered under the same two, with no other terms.
