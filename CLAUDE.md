# Notes for Claude Code

Working notes for AI-assisted sessions on Film Darkroom. Read alongside `README.md`, the
user guide (`docs/USER_GUIDE.md`), the designs (`docs/design/`: 0001 and 0002 black and
white, 0003 colour, 0004 the combined app) and the decision log (`docs/decisions.md`;
record product and scope decisions there).

## What this is

A Rust film emulation engine with two processes, black and white (`mono`) and colour, and
a macOS desktop app (`film-darkroom`) that switches between them. It combines BW Darkroom
(github.com/catskinroyale/bnw-darkroom, released to 0.11.1, whose history this repo
continues) and Colour Darkroom (github.com/catskinroyale/colour-darkroom, unreleased,
merged in). Decision P7, design 0004.

- **Black and white:** film → developer → negative → print, separate physically motivated
  stages. Tri-X, T-Max and the papers are fitted to Kodak's curves, Ilford films to
  Ilford's (Level 1), the rest **illustrative** (`crates/mono-negative/src/illustrative.rs`).
  Photographer presets are documented interpretations (`docs/research/photographer-presets.md`).
- **Colour:** scene → layer exposure → light effects → curves → interlayer → grain →
  printer lights → print → dyes. Every film and print stock is **illustrative**
  (`crates/colour-film/src/stock.rs`).

Say so, never invent data, and label approximations.

**Never write the old apps' data.** Film Darkroom reads BW Darkroom's and Colour Darkroom's
styles (`bw-darkroom/styles.json`, `colour-darkroom/styles.json`) and sidecars (`.bw.json`,
`.cd.json`) as a starting point, and never writes, renames or deletes them. Ours are
`film-darkroom/styles.json` and `<photo>.fd.json`.

## Layout

| Path | What |
|---|---|
| `crates/fd-core` (MIT) | Planes, `SceneImage`, geometry, curves, hash noise, blur (`gaussian_blur_wide`), film compression |
| `crates/fd-io` | Loading; export with EXIF, greyscale (`save_grey_*`) and colour (`save_jpeg`, `save_png16`, `save_tiff16`) |
| `crates/fd-raw` | Raw input via LightCraft's `lightcraft-raw` (pinned git rev); `RawAdjust` incl. defringe |
| `crates/mono-film`, `mono-developer` | Black-and-white film stocks; developer behaviour |
| `crates/mono-negative` | Exposure, development, adjacency, grain; `times.rs` reads `data/development/times.csv`; `fitted.rs` Level-2 calibrations from `data/calibrations/`; `combination()` prefers fitted over illustrative |
| `crates/mono-darkroom` | Printing: measured paper grade family (`data/papers/`) |
| `crates/colour-film` | `stock.rs` (film and print stocks), `lib.rs` (expose, develop, print, render) |
| `crates/fd-profiles` | `mono` and `colour` style modules (the two apps' profiles, unchanged) and `lib.rs`: `Style = Mono \| Colour` (tag `"process"`), `render`, `render_region`, `ModelSummary`, `ExportRecord`, `NameParts`, `CompressionSpec`, `builtin_styles` |
| `apps/fd-desktop` | egui app: `app.rs` (state, panels, process switch), `workflow/{mod,mono,colour}.rs` (Develop sections), `image_panel.rs`, `source.rs` (sidecars), `styles.rs` (store + first-launch import), `worker.rs`, `export.rs`, `widgets.rs` + `theme.rs` (LightCraft-derived UI) |
| `apps/fd-cli` | `film styles`, `film render`, `film sheet`, `film variations` |

Everything except `fd-core` is Apache-2.0. `theme.rs`, `widgets.rs` and `folders.rs` adapt
LightCraft code (MIT, credited in each file and the About window); Inter is OFL. Never use
ArtCraft branding, and never another company's product branding for the app itself.

## Commands

```sh
source ~/.cargo/env                      # cargo is not on PATH by default
cargo fmt && cargo clippy --release --all-targets -- -D warnings
cargo test --release --workspace         # keep clippy and tests clean
cargo test --release -p fd-desktop -- --ignored live   # real GitHub update check (network)
cargo run --release -p fd-desktop -- <folder-or-image>
cargo run --release -p fd-cli -- sheet <raw> --out sheet.jpg   # judge looks by eye
python3 -I tools/gen_licenses.py         # after dependency changes (About window list)
python3 -I tools/make_icon.py packaging/macos/icon_1024.png
```

Datasheet calibration (design 0002; needs a venv with pymupdf, numpy, scipy): read vector
curves with `tools/datasheet_curves.py` (inspect → extract → overlay to check by eye), fit
film families with `tools/fit_development.py`, build papers with `tools/paper_family.py`.
Each `data/` folder's README has the exact commands. Only Kodak's PDFs are vector so far;
Ilford's curves are raster images (`tools/raster_curve.py`).

`gh` is in `/opt/homebrew/bin` (`export PATH="/opt/homebrew/bin:$PATH"`).

**Refactors that should not change renders**: hash every built-in style's render of a few
raw files before and after (as the merge did, design 0004 §1) and require bit-identical
output.

## Model decisions to keep

Black and white (designs 0001, 0002):

- **Development moves the curve** (D12): `SpeedGain` shifts log H by `s·ln(τ/τ_ref)`;
  illustrative calibrations keep `s = 0`. A two-stop push gains about one stop.
- **The paper is measured** (Kodak Polymax II RC, filters −1…5+; in-between grades blend
  horizontally). Grade = Kodak filter number.
- **Push/pull change development only.** Styles expose the film at box speed
  (`Style::exposure_ei`); `exposure_index` picks the published time.
- **Prints are exposed for the shadows** (P3, design 0002 §7): zone I prints at the
  density normal development gives it (`mono::Style::print_settings`),
  `crates/fd-profiles/tests/push.rs`.
- **Calibration levels**: Level 2 = Kodak D-76 families and CI-line developer rates;
  Level 1 = Ilford films; the rest illustrative.
- Development times: whole-stop EIs only, from the Massive Dev Chart extract.

Colour (design 0003):

- **Grey is sacred.** Printer lights are solved so the metered grey prints as a neutral
  18 % on every film × print pair
  (`metered_grey_prints_as_neutral_mid_grey_on_every_stock_pair`).
- **Interimage effects act on log exposure**, floored at the toe (C3).
- **One curve shape** (`toe_line_shoulder`) for negatives, prints and slides (C4).
- **Film exposure changes character, not brightness** for negatives (lab timing, C5).
- **Halation and bloom act on light** before the log (C2). Halos are red-orange.
- Stock looks are `look_points`, switchable ("Film's lab look"), never baked into curves.

Both:

- **Grain** is keyed on **full-frame pixel coordinates**, so 1:1 detail crops match the
  export; `GrainAdjust` (size, amount, roughness) defaults to the film's own grain.
- **Film compression** (`fd_core::Compression`, design 0003 C10, 0004 U5) is off by
  default; black and white uses the luminance curve only.
- **Per-photo adjustments are shared** between processes (U3); styles belong to one (U1).

## UI conventions (egui 0.37)

- `App::ui(ui)` and `egui::Panel::left/right/top/bottom(..).show(ui)`; `Id::unique`
  (`Id::new` and `ui.id()` are deprecated).
- A widget wider than its row inside a resizable panel makes the panel grow forever: lay
  overlays out with zero-size `ui.interact` + painting.
- For "pointer inside a container" use `Response::container_contains_pointer`; plain
  `contains_pointer` is false over child widgets (this broke the style menu once).
- Right side: tool strip (`Module::Develop` / `Module::Styles`) + tools panel. Advanced
  controls go in `widgets::flyout` rows inside `widgets::section`s, not separate views.
- The process switch (`App::set_process`) filters styles, previews and sections; lists
  iterate `App::visible()`, and gallery tiles of the other process are `Tile::Skipped`.
- Every control gets a one- or two-line hover explanation.

## Screenshot tests (checking UI without the user's screen)

`FILM_DARKROOM_SCREENSHOT=<png>` renders, saves a screenshot and quits; test runs never
save settings or touch the network. Options (prefix `FILM_DARKROOM_SCREENSHOT_`):
`PROCESS=mono|colour`, `FRESH=1` (default layout), `THEME=dark|light`,
`VIEW=develop|styles`, `EDIT=1` (open all sections), `CLOSE=image,film`,
`DETAIL=x,y,zoom`, `GRAIN=amount,size,roughness`, `THEN_PUSH=stops`,
`COMPARE=side|original`, `SHOW=bitmaps,jpegs`, `STYLEMENU=index`, `ABOUT=1`, `EXPORT=1`,
`UPDATE=version` + `UPDATE_DIALOG=1` (fake update), `WAIT=seconds`, and
`CLICKS=t:x,y;t:x,y>x2,y2` (scripted clicks and drags through egui input). Run each shot
with a time limit (`perl -e 'alarm 90; exec @ARGV' …`; macOS has no `timeout`) and retry:
a window opened behind others may never repaint. Scripted clicks can rename or delete real
user styles (`~/Library/Application Support/film-darkroom/styles.json`): only open
dialogs, never confirm.

## Git, changelog and releases

- Commit and push only when the user asks.
- **Every push needs a CHANGELOG.md `[Unreleased]` entry.** A hook
  (`.claude/hooks/require-changelog.sh`) blocks `git push` otherwise; edit the changelog in
  a separate command before the push command.
- End commit messages with the attribution line the session provides.
- **Every release refreshes the README first**: take a screenshot as
  `docs/images/screenshot-X.Y.Z.jpg` (a new name per release: caches keep old images),
  `git rm` the previous one, and update the README text and image alt text to match,
  committed before `tools/release.sh`. Take the shot with `HOME` set to a scratch folder
  holding only an approved photo (e.g. `Pictures/Darkroom/DSC_6438.NEF` plus neutral
  empty folders) so the folder tree shows nothing personal, then check the JPEG for
  names, paths or GPS (`strings`).
- Release (version only in `[workspace.package]` of `Cargo.toml`; Film Darkroom continues
  BW Darkroom's numbering, so the first release is `tools/release.sh minor` → 0.12.0):
  ```sh
  tools/release.sh patch|minor|major      # bumps version, dates the changelog, tags
  git push && git push origin vX.Y.Z
  tools/package-macos.sh                  # → dist/Film-Darkroom-X.Y.Z-arm64.dmg
  gh release create vX.Y.Z dist/Film-Darkroom-X.Y.Z-arm64.dmg --title "Film Darkroom X.Y.Z" \
    --notes-file <changelog section> --latest
  ```
  Before 1.0: minor = features or behaviour changes, patch = fixes.
- Public images (README, releases) use only the author's own photos.
