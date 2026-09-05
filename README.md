# Terra Launcher

A custom Minecraft launcher written in Rust.

- UI: egui / eframe (dark theme by default, runtime-themeable via JSON presets — planned)
- Plugins: WASM-based plugin tabs (planned)
- Mod loaders: Fabric / Forge / Quilt (planned)
- Friends: modpack sharing via serializable manifests (planned)

## Development

```
cargo run
```

CI runs `cargo check` and `cargo test` on every push (see `.github/workflows/ci.yml`).
