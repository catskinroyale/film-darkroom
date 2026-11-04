# Notes for Claude Code

Working notes for AI-assisted sessions on Colour Darkroom. Read alongside `README.md`, the
user guide (`docs/USER_GUIDE.md`), design 0001 (`docs/design/0001-colour-film-model.md`) and
the decision log (`docs/decisions.md`; record product and scope decisions there).

## What this is

A Rust colour film emulation engine and a macOS desktop app (`colour-darkroom`), the colour
sibling of BW Darkroom (github.com/catskinroyale/bnw-darkroom). Scene → layer exposure →
light effects → curves → interlayer → grain → printer lights → print → dyes are separate,
physically motivated stages. Every film and print stock is **illustrative**
(`crates/cd-film/src/stock.rs`): say so, never invent data, and label approximations.

It is a **separate app** (decision P1): never read or write BW Darkroom's settings, styles
(`bw-darkroom/styles.json`) or sidecars (`.bw.json`). Ours are `colour-darkroom/` and
`.cd.json`. The shared foundations were copied, not linked (P2).

## Layout

| Path | What |
|---|---|
| `crates/cd-core` (MIT) | Planes, `SceneImage`, geometry, curves, hash noise, blur, `gaussian_blur_wide` |
| `crates/cd-film` | `stock.rs` (film and print stocks), `lib.rs` (expose, develop, print, render) |
| `crates/cd-profiles` | `Style`, `builtin_styles`, `render`, `render_region`, `ExportRecord` |
| `crates/cd-io` | Loading, colour export with EXIF (`save_jpeg`, `save_png16`, `save_tiff16`) |
| `crates/cd-raw` | Raw input via LightCraft's `lightcraft-raw` (pinned git rev, same as BW Darkroom) |
| `apps/cd-desktop` | egui app: `app.rs`, `workflow.rs` (Film, Light, Grain, Print sections), `worker.rs`, the rest adapted from BW Darkroom |
| `apps/cd-cli` | `colour styles`, `colour render`, `colour sheet` |

Everything except `cd-core` is Apache-2.0. `theme.rs`, `widgets.rs` and `folders.rs` adapt
LightCraft code (MIT, credited in each file and the About window); Inter is OFL. Never use
ArtCraft branding, and never another company's product branding for the app itself.

## Commands

```sh
source ~/.cargo/env                      # cargo is not on PATH by default
cargo fmt && cargo clippy --release --all-targets -- -D warnings
cargo test --release --workspace         # keep clippy and tests clean
cargo run --release -p cd-desktop -- <folder-or-image>
cargo run --release -p cd-cli -- sheet <raw> --out sheet.jpg   # judge looks by eye
python3 -I tools/gen_licenses.py         # after dependency changes (About window list)
python3 -I tools/make_icon.py packaging/macos/icon_1024.png
```

## Model decisions to keep (design 0001)

- **Grey is sacred.** Printer lights are solved so the metered grey prints as a neutral
  18 % on every film × print pair (`metered_grey_prints_as_neutral_mid_grey_on_every_stock_pair`).
  Anything that could tint neutrals must keep that test passing.
- **Interimage effects act on log exposure**, floored at the toe (C3). On density they tinted
  greys; without the floor, shadow noise printed as coloured specks.
- **One curve shape** (`toe_line_shoulder`) for negatives, prints and slides (C4). A logistic
  put grey on its soft half and flattened every print.
- **Film exposure changes character, not brightness** for negatives (lab timing, C5).
- **Halation and bloom act on light** before the log (C2). Halos are red-orange.
- **Grain** is BW Darkroom's Selwyn model per layer, keyed on full-frame pixel coordinates
  (`Region::origin`), so the detail view matches the export.
- Stock looks are `look_points`, switchable ("Film's lab look"), never baked into curves.

## UI conventions (egui 0.37)

As BW Darkroom: `egui::Panel::…show(ui)`, `Id::unique`, zero-size `ui.interact` overlays in
resizable panels, `Response::container_contains_pointer`, advanced controls in
`widgets::flyout` rows inside `widgets::section`s, and a hover explanation on every control.

## Screenshot tests

`COLOUR_DARKROOM_SCREENSHOT=<png>` renders, saves a screenshot and quits; test runs never
save settings or touch the network. Options (prefix `COLOUR_DARKROOM_SCREENSHOT_`): `FRESH=1`,
`THEME=dark|light`, `VIEW=develop|styles`, `EDIT=1`, `CLOSE=image,film`, `DETAIL=x,y,zoom`,
`GRAIN=amount,size,roughness`, `THEN_PUSH=stops`, `COMPARE=side|original`,
`SHOW=bitmaps,jpegs`, `STYLEMENU=index`, `ABOUT=1`, `EXPORT=1`, `UPDATE=version` +
`UPDATE_DIALOG=1`, `WAIT=seconds`, `CLICKS=…`. Run each shot with a time limit
(`perl -e 'alarm 90; exec @ARGV' …`). Scripted clicks can rename or delete real user styles:
only open dialogs, never confirm.

## Git, changelog and releases

- Commit and push only when the user asks. End commit messages with the attribution line
  the session provides.
- Every push needs a CHANGELOG.md `[Unreleased]` entry.
- The GitHub repo `catskinroyale/colour-darkroom` does not exist yet; until it does, the
  update check reports a failed check (harmless). `tools/release.sh` and `updates.rs` point
  at it.
- Release (version only in `[workspace.package]`; starts at 0.0.0, so the first release is
  `tools/release.sh minor` → 0.1.0, which the script links to its tag):
  ```sh
  tools/release.sh patch|minor|major
  git push && git push origin vX.Y.Z
  tools/package-macos.sh                  # → dist/Colour-Darkroom-X.Y.Z-arm64.dmg
  gh release create vX.Y.Z dist/Colour-Darkroom-X.Y.Z-arm64.dmg --title "Colour Darkroom X.Y.Z" \
    --notes-file <changelog section> --latest
  ```
- Public images (README, releases) use only the author's own photos; check JPEGs for names,
  paths or GPS before publishing.
