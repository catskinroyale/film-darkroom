# Changelog

All notable changes to this project are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

## [0.1.0] - 2026-11-04

Film Darkroom: BW Darkroom and Colour Darkroom in one app and one code base. Black and
white continues from BW Darkroom 0.11.1; colour arrives from Colour Darkroom (unreleased).

### Added
- **Colour and black and white in one app.** A **Colour | B&W** switch in the top bar (or
  **M**) chooses the process: the style list, the column of previews and the Develop
  panel follow it. Each process keeps its style and unsaved edits while the other is
  shown, and the last style of each is remembered.
- **Colour film** (from Colour Darkroom; all stocks illustrative): three dye layers with
  spectral sensitivity, per-layer curves, interimage effects that saturate colour without
  tinting greys, edge effects, per-layer grain, halation, bloom and vignette; a lab that
  times every print to grey, with print exposure, printer points and each film's lab look.
  Portra 160/400/800, Ektar 100, Gold 200, UltraMax 400, Pro 400H, Superia 400,
  Vision3 250D/500T, CineStill 800T; Ektachrome E100, Velvia 50, Provia 100F slides;
  neutral scan, Endura and Crystal Archive paper, 2383 and 3513 print film.
- **Film compression**, for both processes: rolls highlights off towards the mid-tones
  before the film, with Impact, Tonal range, White point (and, in colour, Colour density)
  and a curve of what it does. Off by default. New built-in style *Portra 400 · film
  compression*.
- **Defringe** (Image section): removes the coloured fringes lens chromatic aberration
  leaves along bright, high-contrast edges.
- **`{print}` export token**: the paper, print stock, scan, or "slide". `{developer}`
  gives C-41, ECN-2 or E-6 for colour.
- **One command-line tool, `film`**: `styles`, `render` (any style; `--film`, `--print`,
  `--push`, `--ev`, `--compression`, plus `--developer` and `--grade` for black and white
  and `--interlayer` for colour), `sheet` (`--process mono|colour`) and `variations`.

### Changed
- **New name and home**: the app is Film Darkroom (`film-darkroom`), from
  github.com/catskinroyale/film-darkroom. Styles are kept in
  `film-darkroom/styles.json`, per-photo adjustments in `<photo>.fd.json`, and exports go
  to `film-exports`.
- **Your BW Darkroom and Colour Darkroom work carries over.** Until Film Darkroom saves
  styles of its own, it starts with your styles from both apps; a photo without a
  `.fd.json` opens with its `.bw.json` or `.cd.json` adjustments. Neither app's files are
  ever changed, so both keep working.
- The `bw` and `colour` command-line tools are replaced by `film`.
- **New app icon**: a strip of 35 mm film on a lightbox, holding the same sunset in a
  colour frame and a black-and-white frame.

## [BW Darkroom 0.11.1] - 2026-10-26

Film Darkroom's black-and-white process is BW Darkroom, whose releases follow, newest
first (github.com/catskinroyale/bnw-darkroom).

### Fixed
- **Export file names follow your changes.** After you change a style's film, developer
  or push, `{style}` in the file name gives the film, developer and EI actually used
  instead of the saved style's name.

## [BW Darkroom 0.11.0] - 2026-10-26

### Changed
- **More films fitted to published curves.** T-Max 100 and T-Max 400 in D-76 are fitted
  to Kodak's curve families (level 2). Tri-X and T-Max 400 in HC-110 B and D-76 1+1 are
  calibrated from Kodak's contrast charts; their rates agree with Kodak's time tables.
- **Ilford films from Ilford's curves.** HP5 Plus, FP4 Plus, 100 Delta, Delta 400 and
  Delta 3200 take their curve shape from Ilford's datasheets (level 1: approximation) in
  every developer. Kentmere and Double-X publish no curves and stay illustrative.
- **Prints are exposed for the shadows.** The darkest tone with detail prints the same
  whatever the development, instead of clear film. Normal development looks as before; a
  push now holds the shadows and brightens the mid-tones and highlights, rather than
  turning the whole print pale.
- **Tri-X 400 in D-76 is now fitted to Kodak's published curves** (level 2:
  sensitometric). The four curves of Kodak's D-76 family (F-4017) are read exactly from
  the datasheet PDF and fitted with the engine's own model. The fit predicts a held-out
  curve within 0.014 density and reproduces Kodak's contrast index and ISO speed. The
  Calibration line under Development details says *level 2: sensitometric* for this
  combination; the other films and developers stay illustrative.
- **Longer development now makes the film faster**, as Kodak's curves show: a two-stop
  push gains about one stop of real speed. (Only fitted calibrations use this.)
- **A real paper.** Printing uses Kodak Polymax II RC's published curves for each
  contrast filter (G-26), replacing the placeholder paper. **Paper grade** runs from −1
  to 5 (Kodak's 5+), and the Print section shows the paper and its ISO R range. Prints
  at grade 2 are a little softer than before: Kodak's filter 2 is R119.

### Added
- **An article about the app** in `docs/article/` (Markdown, and HTML with the
  screenshots inline).
- The About window says which films are fitted and credits Kodak's and Ilford's datasheets.
- **A matte paper**: Polycontrast IV-like (Kodak G-4037, N surface), with blacks about 1.7
  instead of glossy's 2.1. Choose it under **Paper** in the Print section.
- **Preflash** in the Print section: an even pre-exposure that brings detail into bright
  highlights.
- `tools/raster_curve.py` traces a curve from a datasheet's raster image, for Ilford's
  sheets.
- Tools to read characteristic curves straight from datasheet PDFs and fit them
  (`tools/datasheet_curves.py`, `fit_development.py`, `paper_family.py`), with the
  extracted Kodak curves in `data/sensitometry/`. Design 0002 describes the method and the
  validation.

### Fixed
- The README showed an out-of-date screenshot: GitHub and browsers kept a cached copy
  of the reused file name. Each release's screenshot now has its own name.

## [BW Darkroom 0.10.0] - 2026-10-23

### Changed
- **The style gallery is a column on the left**, beside the folder tree, so larger
  previews no longer take height from the photo. Previews are twice their earlier size,
  in the photo's own shape, with their names wrapped to the preview's width beneath. The
  column fits snugly around them. A **Size** slider at the column's base (styled like the
  Develop panel's sliders) makes them up to twice as large again; the column's width
  follows it and the size is remembered. The previews are rendered larger to stay
  sharp. The image strip stays at the bottom.
- The style column is shown and hidden with a grid icon at the left of the toolbar
  (or **G**), instead of the **Gallery** button in the top bar.
- The README screenshot shows the style column, and its folder tree shows only neutral
  example folders.
- `CLAUDE.md`: every release now refreshes the README and its screenshot first.

## [BW Darkroom 0.9.0] - 2026-10-23

### Added
- **Side-by-side comparison.** A toolbar of icons between the photo and the style gallery
  switches the image area between **Treated** (a filled frame; the default), **Side by
  side** (two frames: original and treated next to each other) and **Original** (an
  empty frame). Clicking **Side by side** again goes back
  to Treated, and **C** steps through the three views. The Detail window follows the view,
  so side by side it shows the original and treated crops together, at 100% or 200%, and
  dragging moves both.

### Changed
- **A tidier top bar.** The folder tree is shown and hidden with a sidebar icon at the far
  left. A single theme icon at the far right steps through Auto, Light and Dark. The
  "BW Darkroom" title (already in the window's title bar), the **Export…** button and the
  **Workflow** toggle (both in the tool strip) and **Open folder…** (now **Browse
  Folder…** in the folder tree) are gone.
- **Show original** and **Detail 1:1** moved from above the photo to the new toolbar.
  Detail is now a magnifying-glass icon, and a book icon shows or hides the file name and
  capture details beside it.

### Fixed
- **Portrait photos are upright in the image strip.** Raw thumbnails come from the camera's
  embedded preview, which is stored sideways; the raw file's orientation tag is now
  applied. JPEG, TIFF and PNG files also follow their EXIF orientation, in thumbnails and
  when opened. (A rendered file you had already turned with **Rotate** may need turning
  back.)
- Folder tree rows line up with the **Folders** header: chevrons under its chevron, and
  each level indented by one icon.

## [BW Darkroom 0.8.0] - 2026-10-23

### Changed
- **Finding a folder and choosing a photo are now separate.**
  - **Folders** (left): a collapsible folder tree, following LightCraft's. It starts from
    Pictures, Desktop, Home and any memory card or external drive, opens down to the
    folder you're in, and ends with **Browse Folder…**. Click a folder to show its photos;
    click its chevron to show or hide its subfolders. The tree lists folders only.
  - **Images** (bottom): a sideways-scrolling strip of the chosen folder's photos. Click
    one to open it. The **RAW**, **TIFF/PNG** and **JPEG** filters and the photo count
    moved here. **I** shows or hides the strip; the arrow keys still step through it.
  - The style gallery sits above the image strip, now labelled **Styles**.
- Apps and other macOS packages (`.app`, Photos and Lightroom libraries, and so on) are no
  longer listed as folders.
- Thumbnails are decoded larger, so they stay sharp in the image strip.
- The README screenshot shows the 0.9 interface, side by side.

### Removed
- The file list's **Up** button and **Thumbs** option; the folder tree and image strip
  replace them.

### Added
- `docs/decisions.md`: a log of product and scope decisions. First entry: the digital look
  is deferred, and work concentrates on film, chemicals and paper.

## [BW Darkroom 0.7.1] - 2026-10-23

### Added
- `CLAUDE.md`: working notes for AI-assisted development (layout, commands, model
  decisions to keep, UI conventions, screenshot-test hooks, release steps).

### Fixed
- **Grain now moves with the image in the Detail window.** It used to be generated
  relative to the visible crop, so dragging the view re-rolled the pattern in place. Grain
  is now tied to each pixel's position in the whole frame. The Detail view therefore shows
  exactly the grain of the full-size export, and it stays aligned as you drag.

## [BW Darkroom 0.7.0] - 2026-10-22

### Changed
- **A new interface, following LightCraft's design.**
  - **Look:** the Inter typeface; LightCraft's Charcoal (dark) and Silver (light) themes;
    the photo on a darker canvas.
  - **Tool strip:** a column of icons at the far right chooses the panel beside it
    (**Develop**, **Styles**) and holds **Export** and **About**.
  - **Develop panel:** a header with **Save**, **Save as…** and **Revert**, a **Style**
    picker, and accordion sections (**Image**, **Film**, **Development**, **Grain**,
    **Print**) that remember whether they're open.
  - **Advanced controls** sit in expandable rows instead of a separate Detailed view:
    **Temperature & highlights**, **Development details** (EI, exposure compensation,
    time, temperature, agitation) and **Characteristic curve**.
  - **Styles panel:** styles in collapsible groups (**Films & processes**,
    **Photographers**, **My styles**), with the **…** menu on hover and on right-click.
    The photographer presets live here, and hovering one shows its look and what is
    documented.
  - **Sliders:** label and value above a thin track with a ring knob. Shift-drag for fine
    control, double-click to reset, click the value to type one.
  - **Removed:** the Simple / Detailed / Photographers switch, and the separate style
    editor window. **Edit…** now opens every section of the Develop panel.
  - **Credits:** the About window credits LightCraft (MIT) and Inter (SIL Open Font
    License) and includes both licences.

### Added
- **Drag inside the Detail window** to move the area you're inspecting, as if moving the
  photo under a loupe. The view follows the pointer straight away and sharpens when the
  new render arrives; the area stays inside the photo.
- **Update notices.** Once a day at start-up the app asks GitHub for the latest release.
  If it's newer, an **Update** button appears in the top bar, opening a Software Update
  window with the release notes and **Download…**, **Later** and **Skip This Version**.
  **BW Darkroom ▸ Check for Updates…** and a button in the About window check straight
  away, and the daily check can be turned off. Nothing is downloaded or installed
  automatically, and nothing is sent beyond the request itself.

### Changed
- The README no longer has a Versioning section.

## [BW Darkroom 0.6.2] - 2026-10-22

### Added
- **User guide** in [docs/USER_GUIDE.md](docs/USER_GUIDE.md): installing, the window,
  opening photos, image adjustments, styles and presets, the workflow views, push and
  pull, grain, the Detail window, printing, managing styles, exporting, shortcuts, where
  files are stored and troubleshooting. The README links to it.
- An app screenshot at the top of the README, showing one of the author's own photos.
- A **Download** section in the README linking to the latest release's macOS app, with
  install steps, for people who don't want to compile.

### Fixed
- The export dialog's `{ei}` token help now says "developed for" rather than "rated at",
  matching push and pull as development changes.

## [BW Darkroom 0.6.1] - 2026-10-22

### Fixed
- The style "…" menu's items can be clicked again. A press on any item except the one
  nearest the "…" counted as a click outside the menu, so the menu closed before the
  click finished.

## [BW Darkroom 0.6.0] - 2026-10-22

### Added
- **Grain controls** in the Simple and Detailed views and the style editor:
  - **Size** changes how coarse the grain clumps are, with strength kept the same.
  - **Amount** changes the grain's strength (0 removes it).
  - **Roughness** goes from softer and smoother (below 0) to grittier, with more
    isolated dark and bright specks (above 0).
  - **Randomise** picks a new pattern and random settings within a usable range;
    **Film's grain** resets; **New pattern** is unchanged.
  - The defaults are the film's own grain, and the grain readout says when it has been
    adjusted. Styles saved without adjustments are unchanged.

### Changed
- **The Detail window keeps up while you adjust.** Its crop now renders before the full
  preview, so any workflow or image change shows there first. A spinner shows while an
  update is on its way. The window now opens over the left of the image, leaving the
  Styles and Workflow panels free.

### Fixed
- Switching the Detail window to 200% no longer doubles its width and leaves the
  magnified crop on the left. The window also returns to its normal size if an earlier
  version saved it oversized.

## [BW Darkroom 0.5.0] - 2026-10-21

### Added
- **File-type filter in the file browser.** Raw files (NEF, DNG, ARW) are always listed,
  and are all that's listed by default. **TIFF/PNG** and **JPEG** toggles in the panel
  header add lossless bitmaps and compressed JPEGs. The header counts the hidden files.
  A folder with no raw files offers **Show them**. Arrow-key stepping skips hidden files,
  and the toggles are remembered.

### Changed
- **Push and pull now change development only.** Your photo is already correctly
  exposed, so the film is exposed at box speed. Push or pull picks the published time for
  the pushed or pulled EI: Zone System N+ / N− development. Exposure compensation in
  Detailed view still gives an underexposed push. Detailed view's "Exposure index" is
  now labelled "Develop for EI".
- **The print holds its black point.** Prints used to place mid-grey at the same density
  for every negative, so a push turned existing blacks grey (Tri-X +2, zone 0 went from
  1.86 to 1.51). The enlarger exposure now keeps clear film printing at the black of the
  normally developed negative, as a printer setting "minimum time for maximum black"
  would. Normal development prints exactly as before. A push keeps blacks, opens the
  shadows slightly and brightens the highlights (HP5 HC-110 B +2: zone III goes from
  1.46 to 1.25 and zone VII from 0.17 to 0.03, with zone 0 unchanged). A pull does the
  reverse. Styles saved with a pushed EI change look accordingly.

## [BW Darkroom 0.4.1] - 2026-10-21

### Changed
- **Compact file browser.** The left panel is now a one-line-per-file list (default
  width 170 px, down from about 260 px) instead of a list of thumbnails:
  - Long names are shortened in the middle so the start and the end, including the
    number, stay visible (for example "Reykjav…042.NEF"), as the Mac Finder and Apple's
    interface guidelines recommend. Hover a row to see the full name.
  - Each row has a short type tag on the right (NEF, DNG, JPG, PNG, TIF), and folders
    have a "Folder" tag.
  - Hovering a file shows a larger thumbnail beside the list.
  - A **Thumbs** toggle in the panel header brings back small inline thumbnails. The
    setting is remembered.
  - The header shows the folder name (full path on hover) and the number of images and
    folders.

### Fixed
- In Detailed view, showing the white-balance sliders no longer switches an image from
  "As shot" to Custom or marks its adjustments as edited. It changes only when you move
  a slider.

## [BW Darkroom 0.4.0] - 2026-10-21

### Added
- **Style menu (…)** on every row of the Styles list, and on right-click of a gallery
  tile: **Edit…** (an editor window with every setting and a live preview), **Duplicate**,
  **Rename…**, **Export…** (a `.json` file), **Move up / Move down** (your own styles) and
  **Delete…** (with confirmation). Built-in styles are read-only: rename, move and delete
  are disabled with an explanation; duplicate one to change it.
- **Import…** in the Styles section adds styles from exported `.json` files (one style or a
  list), renaming any that clash.

### Fixed
- Screenshot test runs no longer save panel sizes and window positions into the user's
  settings.

## [BW Darkroom 0.3.1] - 2026-10-21

### Added
- Detail view zoom: **100%** (one image pixel per screen pixel, exact on Retina displays)
  and **200%** (each image pixel as a crisp 2×2 block for judging grain).
- The main image shows an outline of exactly the area the detail view displays.

### Fixed
- The detail view was not consistently centred on the clicked point: clicks were measured
  against the whole preview panel rather than the drawn image, so the error varied with the
  image's aspect ratio and the panel size. Clicks outside the image are now ignored.

### Changed
- The detail window opens beside the image instead of over it.

## [BW Darkroom 0.3.0] - 2026-10-21

### Added
- **Kentmere Pan 100, 200 and 400** films (illustrative models; Harman publishes no curves
  or granularity), with 85 Massive Dev Chart rows and Harman data-sheet findings
  (`docs/research/kentmere.md`). Kentmere HC-110 rows labelled "1+31" count as dilution B.
- **Photographer presets** as a third Workflow view, **Photographers**: Don McCullin,
  Ansel Adams, Bill Brandt, Josef Koudelka and Edward Weston. Each preset is an ordinary
  film/developer/process/print recipe with a note separating documented practice from
  interpretation (`docs/research/photographer-presets.md`); presets also appear as
  built-in styles.
- **Contrast** in the Image panel: scene contrast around mid-grey (−100…+100), applied
  before the film on luminance so colour-to-grey conversion is unchanged; saved in sidecars
  (older sidecars load as 0). Presets can set it.
- **Dark mode**: Auto / Light / Dark toggle in the top bar, remembered between sessions.
- **8×10** film format (203 mm), for contact-print workflows.

### Changed
- The 56 mm format is labelled "6×6 / 645".
- Development-time table: 293 curated rows.

## [BW Darkroom 0.2.0] - 2026-10-20

### Added
- **Camera raw support** (`crates/bw-raw`): Nikon NEF, DNG (including Leica M Monochrom)
  and Sony ARW, decoded with LightCraft's `lightcraft-raw` (MIT OR Apache-2.0, pinned to
  commit `d999e38`). AHD demosaic, DNG colour model, highlight reconstruction and
  embedded-preview thumbnails.
- Colour for NEF and ARW from matrices fitted to measured camera spectral sensitivities
  (rawtoaces-data, Academy Software Foundation), including the Nikon Z f.
- **Image panel** in the desktop app for per-image adjustments before development:
  exposure (with an averaging-meter Auto), white balance (as shot, auto, presets, custom
  temperature and tint), highlight reconstruct/clip, rotation and reset. Adjustments are
  stored non-destructively in a sidecar next to each image (`<file>.bw.json`).
- **Export dialog** (⌘E): JPEG with quality setting, 16-bit TIFF, 16-bit PNG or the
  original's format; full size or a long-edge resize; file-name templates with tokens
  (`{name}`, `{style}`, `{film}`, `{developer}`, `{ei}`, `{date}`, `{time}`, `{n}`);
  incremental numbering with zero padding; add-suffix or overwrite on conflict; export
  folder; optional recipe. Settings persist between sessions.
- Export recipes record the image adjustments and camera details.
- **EXIF preserved on export** (JPEG, 16-bit TIFF, 16-bit PNG) from JPEG, PNG, TIFF and
  raw sources (NEF, DNG, ARW), via `kamadak-exif`: camera, lens, exposure, ISO, focal
  length, capture dates, GPS, artist and copyright. Orientation is reset to 1 (rotation is
  applied to the pixels), pixel dimensions follow the export, colour space is sRGB,
  Software names BW Darkroom, and a UserComment records the development recipe.
  MakerNote, thumbnails and source image-structure tags are dropped.
- `bw-io`: JPEG and 16-bit PNG writers, Lanczos long-edge resize, orientation transforms.
- macOS packaging: `tools/package-macos.sh` builds `dist/BW Darkroom.app` and a
  drag-to-Applications DMG (Apple Silicon, ad-hoc signed; `SIGN_IDENTITY` for a Developer
  ID), with an app icon generated by `tools/make_icon.py` and a stripped `dist` build profile.
- **About window** from a native macOS app menu (BW Darkroom ▸ About BW Darkroom, plus
  Services, Hide, Quit and a Window menu, via `muda`): version with git commit, the
  project's licences with full texts, data sources, and a filterable list of all
  third-party components and their licences (`tools/gen_licenses.py`).
- **Semantic versioning:** the workspace version is the single source; builds embed the git
  commit; `tools/release.sh patch|minor|major` bumps the version, dates the changelog
  section, commits and tags `vX.Y.Z`.
- Claude Code project hook (`.claude/hooks/require-changelog.sh`) that blocks `git push`
  unless CHANGELOG.md is updated with the changes being pushed.

### Changed
- **Licensing:** `bw-core` is MIT; every other crate, the desktop app, the CLI and plugins
  are Apache-2.0 (previously all declared "MIT OR Apache-2.0" without licence files).
  Added `LICENSE-MIT` and `LICENSE-APACHE`; both are bundled in the macOS app.
- Export recipes are named after the full file (`print.jpg.json`) so a JPEG and a TIFF
  with the same name keep separate recipes.
- The export folder moved from the top bar into the export dialog.
- README documents macOS packaging and licensing.

### Known limitations
- Nikon High Efficiency NEFs (HE / HE★, JPEG XS) cannot be decoded; shoot lossless
  compressed NEF.
- `{date}` and `{time}` in export names are UTC.
- Film and developer parameters are still illustrative, not fitted to sensitometry.

## [BW Darkroom 0.1.0] - 2026-10-20

### Added
- Design 0001 for the negative simulation model, with verified source research.
- Engine crates: `bw-core` (planes, film geometry, monotone curves, deterministic noise,
  blur), `bw-film`, `bw-developer`, `bw-negative` (exposure, development-growth curves,
  adjacency, Poisson/Selwyn grain), `bw-darkroom`, `bw-profiles`, `bw-io`.
- Films: Tri-X 400, T-Max 100/400, HP5 Plus, FP4 Plus, Delta 100/400/3200, Double-X 5222.
  Developers: D-76 (stock, 1+1), D-96, HC-110 (B, E, H, G), Rodinal (1+25, 1+50, 1+100),
  FX-55 (1+9).
- Development-time table curated from Massive Dev Chart data (whole-stop EIs), with
  `tools/curate_times.py`.
- `bw` CLI with a variations contact sheet.
- BW Darkroom desktop app: folder browser, preview, style gallery, simple and detailed
  workflow views with hover help, push/pull in stops, grain from format and enlargement,
  1:1 detail view, full-resolution TIFF export.

[Unreleased]: https://github.com/catskinroyale/film-darkroom/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/catskinroyale/film-darkroom/releases/tag/v0.1.0
[BW Darkroom 0.11.1]: https://github.com/catskinroyale/bnw-darkroom/compare/v0.11.0...v0.11.1
[BW Darkroom 0.11.0]: https://github.com/catskinroyale/bnw-darkroom/compare/v0.10.0...v0.11.0
[BW Darkroom 0.10.0]: https://github.com/catskinroyale/bnw-darkroom/compare/v0.9.0...v0.10.0
[BW Darkroom 0.9.0]: https://github.com/catskinroyale/bnw-darkroom/compare/v0.8.0...v0.9.0
[BW Darkroom 0.8.0]: https://github.com/catskinroyale/bnw-darkroom/compare/v0.7.1...v0.8.0
[BW Darkroom 0.7.1]: https://github.com/catskinroyale/bnw-darkroom/compare/v0.7.0...v0.7.1
[BW Darkroom 0.7.0]: https://github.com/catskinroyale/bnw-darkroom/compare/v0.6.2...v0.7.0
[BW Darkroom 0.6.2]: https://github.com/catskinroyale/bnw-darkroom/compare/v0.6.1...v0.6.2
[BW Darkroom 0.6.1]: https://github.com/catskinroyale/bnw-darkroom/compare/v0.6.0...v0.6.1
[BW Darkroom 0.6.0]: https://github.com/catskinroyale/bnw-darkroom/compare/v0.5.0...v0.6.0
[BW Darkroom 0.5.0]: https://github.com/catskinroyale/bnw-darkroom/compare/v0.4.1...v0.5.0
[BW Darkroom 0.4.1]: https://github.com/catskinroyale/bnw-darkroom/compare/v0.4.0...v0.4.1
[BW Darkroom 0.4.0]: https://github.com/catskinroyale/bnw-darkroom/compare/v0.3.1...v0.4.0
[BW Darkroom 0.3.1]: https://github.com/catskinroyale/bnw-darkroom/compare/v0.3.0...v0.3.1
[BW Darkroom 0.3.0]: https://github.com/catskinroyale/bnw-darkroom/compare/v0.2.0...v0.3.0
[BW Darkroom 0.2.0]: https://github.com/catskinroyale/bnw-darkroom/compare/49d00c7...v0.2.0
[BW Darkroom 0.1.0]: https://github.com/catskinroyale/bnw-darkroom/commit/49d00c7
