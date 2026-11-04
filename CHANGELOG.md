# Changelog

All notable changes to this project are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added
- **Colour film engine** (`cd-film`): three dye layers with spectral sensitivity,
  per-layer toe / straight line / shoulder curves, interimage (DIR) effects that saturate
  colour without tinting greys, edge effects, per-layer Selwyn grain keyed on full-frame
  coordinates, halation, bloom and vignette.
- **Lab printing**: printer lights timed so mid-grey prints neutral on every film and print,
  with print exposure, red/green/blue printer points and each film's lab look on top.
- **Stocks** (all illustrative): Portra 160/400/800, Ektar 100, Gold 200, UltraMax 400,
  Pro 400H, Superia 400, Vision3 250D/500T, CineStill 800T; Ektachrome E100, Velvia 50,
  Provia 100F slides; neutral scan, Endura and Crystal Archive paper, 2383 and 3513 print
  film.
- **Desktop app** (`colour-darkroom`), adapted from BW Darkroom: folder tree, style
  previews, comparison and 1:1 detail views, Develop panel (Image, Film, Light, Grain,
  Print), styles, export to 16-bit TIFF/PNG or JPEG with EXIF and a JSON recipe.
- **Film compression**: rolls highlights off towards the mid-tones before the film, with
  Impact, Tonal range, White point and Colour density, and a curve of what it does. New
  built-in style *Portra 400 · film compression*; `colour render --compression`.
- **Defringe** (Image section): removes the coloured fringes lens chromatic aberration leaves
  along bright, high-contrast edges, before the film exaggerates them.
- **Command line** (`colour`): `styles`, `render` and `sheet` (every style on one sheet);
  `render` also takes `--defringe`, `--interlayer` and `--original`.

[Unreleased]: https://github.com/catskinroyale/colour-darkroom/commits/main
