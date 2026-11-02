# Colour Darkroom

A colour film emulation engine in Rust, with a desktop app and a command-line tool. It
turns digital raw files (NEF, DNG, ARW) into colour negative, slide and print looks by
modelling the film rather than grading the image: three dye layers with their own spectral
sensitivity and curves, interlayer effects, halation, per-layer grain, a lab that times each
print to grey, and paper, cinema print film or a neutral scan.

It is the colour sibling of [BW Darkroom](https://github.com/catskinroyale/bnw-darkroom)
and shares its approach and interface, but it is a separate app with its own settings,
styles and sidecars.

> **Status:** early prototype, not yet released. Every film and print stock is **illustrative**: its
> parameters were chosen to give the character photographers describe for the named product,
> not fitted to its datasheet, and the names describe the intended look only. See
> [design 0001](docs/design/0001-colour-film-model.md).

## Films and prints

| Negatives | Slides | Prints |
|---|---|---|
| Portra 160, 400, 800 · Ektar 100 · Gold 200 · UltraMax 400 · Pro 400H · Superia 400 · Vision3 250D, 500T · CineStill 800T | Ektachrome E100 · Velvia 50 · Provia 100F | Neutral scan · Endura paper · Crystal Archive paper · 2383 and 3513 print film |

## Layout

| Path | Contents |
|---|---|
| `crates/cd-core` | Planes, scenes, geometry, curves, deterministic noise, blur (wide blur for halation and bloom) |
| `crates/cd-film` | The colour model: stocks (`stock.rs`), exposure, light effects, curves, interlayer, grain, printing, dyes |
| `crates/cd-profiles` | Styles (film + process + light + grain + print), built-ins, render entry points, export recipe |
| `crates/cd-io` | Image loading and colour export (TIFF, JPEG, PNG with EXIF) |
| `crates/cd-raw` | Camera raw input via LightCraft's `lightcraft-raw` |
| `apps/cd-desktop` | `colour-darkroom` desktop app (egui) |
| `apps/cd-cli` | `colour` command-line tool |
| `docs/` | User guide, design, decision log |

## Build and run

Requires the Rust toolchain pinned in `rust-toolchain.toml` (installed automatically by rustup).

```sh
cargo test --release                                      # all tests
cargo run --release -p cd-desktop -- ~/Pictures           # desktop app (optional folder or image)
cargo run --release -p cd-cli -- sheet photo.NEF --out sheet.jpg
tools/package-macos.sh                                    # → dist/Colour-Darkroom-<version>-arm64.dmg
```

Set `COLOUR_RAW_SAMPLE=/path/to/file.NEF` to include a real raw file in `cargo test`.
Nikon High Efficiency (HE/HE★) NEFs are not supported; use lossless compressed NEF.

To use the app, see the [user guide](docs/USER_GUIDE.md).

## Licensing

| Part | Licence |
|---|---|
| `crates/cd-core` | MIT ([LICENSE-MIT](LICENSE-MIT)) |
| All other crates and apps | Apache-2.0 ([LICENSE-APACHE](LICENSE-APACHE)) |

Third-party code keeps its own licence: `lightcraft-raw` and its supporting crates are MIT
OR Apache-2.0; the interface adapts LightCraft's (MIT); Inter is SIL OFL 1.1. The About
window lists every third-party component compiled into the app (`tools/gen_licenses.py`).
