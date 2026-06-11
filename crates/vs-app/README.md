# vs-app

`vs-app` is the cross-platform desktop GUI for the [`vs`](../../README.md) runtime
version manager. It is built with [gpui](https://crates.io/crates/gpui) and
[gpui-component](https://crates.io/crates/gpui-component) and drives the same
`vs-core` engine as the `vs` CLI.

## Run

```bash
cargo run -p vs-app
```

## Linux build dependencies

gpui needs system libraries on Linux (X11/Wayland/Vulkan/fontconfig/ALSA). See the
`checks` job in `.github/workflows/ci.yml` for the exact apt package list.
