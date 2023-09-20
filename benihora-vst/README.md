# Benihora VST

## Build

After installing [Rust](https://rustup.rs/), you can compile as follows:

```shell
cargo xtask bundle benihora-vst --release
```

The artifact will be placed in `target/bundled/benihora.vst3`.

## Install

Copy the `benihora.vst3` directory to your VST3 Plugins folder.

## TODO

- [ ] Key binding
- [ ] Improve intensity control
- [ ] Tongue and constiction point editor
- [ ] Improve routine editor
- [ ] Documentation
- [ ] Polyphonic
- [ ] Text input

## License

GPL-3.0 license
