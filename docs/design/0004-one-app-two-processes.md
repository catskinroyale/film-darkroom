# Design 0004: one app, two processes

Status: implemented (unreleased). Decision P7 in the [decision log](../decisions.md).

Film Darkroom combines BW Darkroom (black and white, released to 0.11.1) and Colour
Darkroom (colour, unreleased) in one code base and one app. The two photographic models
stay separate; everything around them is shared.

## 1. Layout

```text
                 fd-core   planes, SceneImage, geometry, curves, noise, blur, film compression
                    │
      ┌─────────────┼──────────────┐
    fd-io        fd-raw         engines
 (load/export)  (camera raw)      │
                       ┌──────────┴───────────┐
                  mono-film                colour-film
                  mono-developer           (stocks, exposure, light,
                  mono-negative             curves, interlayer, grain,
                  mono-darkroom             printing, dyes)
                       └──────────┬───────────┘
                             fd-profiles     Style = Mono | Colour; render, render_region,
                                │            ModelSummary, ExportRecord, NameParts
                      ┌─────────┴─────────┐
                 fd-desktop            fd-cli
              (film-darkroom)          (film)
```

- **Shared crates** (`fd-core`, `fd-io`, `fd-raw`) were the two apps' copies merged. The
  colour copies were supersets (`SceneImage` in core, wide blur, colour export, Defringe);
  `fd-io` keeps the black-and-white greyscale writers as `save_grey_*`.
- **Engines are untouched**: the black-and-white crates were renamed (`bw-*` →
  `mono-*`), `cd-film` became `colour-film`, and their code did not change.
- **`fd-profiles`** holds both styles libraries unchanged as `mono` and `colour`, and a
  thin layer that dispatches on the process.

The merge was checked render for render: every built-in style of both apps, on three raw
files, hashed bit for bit before the merge and after each step (147 renders, identical).

## 2. Decisions

| # | Decision | Rationale |
|---|---|---|
| U1 | **A style belongs to one process.** `Style` is an enum, `Mono(mono::Style)` or `Colour(colour::Style)`, serialised with a `"process"` tag. Untagged JSON (from the single-process apps) is read by shape: a `developer` field means black and white. | The two recipes share few fields (format, enlargement, grain, seed, exposure compensation), and those mean slightly different things in each model. Wrapping the existing structs kept both serialisations, both test suites and every saved style working unchanged. |
| U2 | **The process is chosen in the app, not per section.** A Colour / B&W switch filters the style list, picker and previews and picks the Develop sections. Each process keeps its style and unsaved edits while the other is shown. | A mixed list of ~50 styles was hard to scan, and rendering every preview in both processes doubled the work. The switch makes "colour or mono" the first choice, as it is with film. |
| U3 | **Per-photo adjustments are shared** between processes, in one `<file>.fd.json` sidecar. | Exposure, white balance, rotation and highlight handling describe the photo, not the print. Separate copies per process would drift apart (rotate a photo in colour and it would still be sideways in black and white). |
| U4 | **Read the old apps' data, never write it.** Styles: until `film-darkroom/styles.json` exists, the user styles of `bw-darkroom/` and `colour-darkroom/` are read in. Sidecars: without a `.fd.json`, the newest `.bw.json` or `.cd.json` is read; resetting writes a default `.fd.json` rather than letting the old one reappear. | Users of either app keep their work on day one, and both old apps keep working on the same folders. |
| U5 | **Film compression is shared** (`fd_core::Compression`). Black and white applies the luminance curve to the scene before development; colour density is colour-only. Off by default, so existing black-and-white renders are unchanged. | It is a scene-referred highlight curve with nothing colour-specific in it (design 0003 C10). |
| U6 | **The CLI follows the same rules**: `film render` takes any style; `--film` picks a style of that film's process; process-specific flags on the wrong process are errors, not silently ignored. | One tool for both, without guessing. |

## 3. What stays per process

| | Black and white | Colour |
|---|---|---|
| Model | Film → developer → negative → paper (designs 0001, 0002) | Three dye layers → interlayer → printer lights → print stock or scan (design 0003) |
| Calibration | Kodak and Ilford datasheet fits (Levels 1–2) and illustrative | Illustrative |
| Develop sections | Film, Development, Film compression, Grain, Print | Film, Film compression, Light, Grain, Print |
| Output | Greyscale (TIFF 16, PNG 16, JPEG) | sRGB colour (TIFF 16, PNG 16, JPEG) |
| Style groups | Films & processes, Photographers, My styles | Negative films, Slide films, My styles |
