# Benihora VST

Benihora VST is a voice synthesis plugin that uses a vocal tract model. It was developed with the motivation of using [Pink Trombone](https://experiments.withgoogle.com/pink-trombone) in a DAW. Pink Trombone is a web application that allows you to interactively manipulate the vocal tract to simulate speech, but Benihora VST allows you to control it with MIDI input.

Online demo: https://carrotflakes.github.io/benihora/

This is under development and features are subject to change.

## Usage

To start using the plugin, you need to obtain the plugin and install it in your DAW.
You can obtain it by downloading it or building it yourself.

### Download

You can get the latest build from the following links:

https://nightly.link/carrotflakes/benihora/workflows/vst/main/benihora-vst_windows.zip
https://nightly.link/carrotflakes/benihora/workflows/vst/main/benihora-vst_macos-universal.zip
https://nightly.link/carrotflakes/benihora/workflows/vst/main/benihora-vst_ubuntu.zip

### Install

Copy the `benihora.vst3` directory to the VST3 plugin folder specified by your DAW.
Scan the VST from your DAW and you will be able to use Benihora VST.

## Build

```shell
cargo xtask bundle benihora-vst --release
```

The artifact will be placed in `target/bundled/benihora.vst3`.

## TODO

- [ ] Key binding
- [ ] Improve intensity control
- [ ] Tongue and constiction point editor
- [ ] Improve routine editor
- [ ] Documentation
- [ ] Polyphonic
- [ ] Text input
- [ ] Preset

## License

GPL-3.0 license
