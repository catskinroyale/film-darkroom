# Colour Darkroom user guide

Colour Darkroom turns your raw files into colour film prints: pick a film, expose and
process it, then print it on paper or cinema print film, or scan it. If you know BW Darkroom,
the window works the same way.

## The window

- **Left**: the folder tree, and a column of style previews of the open photo. Click a
  preview to apply that style.
- **Centre**: the photo, with the comparison toolbar (single, side by side, original only),
  the detail view (**D**, then click to inspect grain at 1:1) and the image strip below.
- **Right**: the **Develop** panel and the **Styles** list (tool strip on the far right).

Every control has a hover explanation. Double-click a slider to reset it; click its value
to type one.

## Develop

### Image

Per-photo adjustments before the film sees the light: exposure, scene contrast, white
balance and highlight recovery, saved in a sidecar next to the photo (`<file>.cd.json`;
BW Darkroom's `.bw.json` sidecars are separate).

**Defringe** removes the yellow, blue, purple or green edges lens chromatic aberration
leaves along bright, high-contrast edges (railings, branches against the sky), which the
film would otherwise exaggerate. It only touches colour at strong edges.

White balance decides what the film treats as neutral, as if the film were correctly
filtered for the light. For tungsten film's blue look in daylight (Vision3 500T, CineStill
800T), set white balance to **Daylight**; under lamps, use **Tungsten** for neutral colour.

### Film

- **Film**: the stock, negatives first, then slides. Each has a one-line description.
- **Format** and **Enlarge**: how big the film is and how much of it you use. Bigger film
  means finer grain and tighter halation.
- **Push / pull**: development only. A push adds contrast, saturation and grain.
- **Exposure**: how much light the film got. The lab still prints mid-grey as mid-grey, so
  on negatives this changes the *character*: +1 gives the dense, clean, saturated
  "overexposed Portra" look; −2 gives a thin, muddy, grainy negative. Slides simply get
  lighter or darker, as real slides do.
- **Colour layers → Interlayer**: how strongly the film's dye layers hold each other back.
  More gives purer colour; greys never change.
- **Characteristic curves**: the red, green and blue layers' density against exposure.

### Light

- **Halation**: the red-orange glow film gets round bright lights, from light bouncing off
  the film base. Most films have a backing that keeps it faint; CineStill 800T has none.
- **Bloom**: a soft highlight glow, like a diffusion filter.
- **Vignette**: lens fall-off in the corners.

### Grain

Size, amount and roughness relative to the film's own grain, as in BW Darkroom. The readout
shows the granularity at this enlargement. Use the detail view to judge it.

### Print

- **Print**: *Neutral scan* (the film's own colour), *Endura* or *Crystal Archive* paper
  (softer blacks), or *2383* / *3513* cinema print film (contrasty, dense blacks).
- **Print exposure**: darker or lighter than the lab's timing.
- **Printer lights**: red, green and blue in printer points. Positive adds that colour;
  12 points is about a stop.
- **Film's lab look**: the rendering each film is known for (Gold's warmth, 400H's cool
  cast). Turn it off for strictly neutral greys.

Slides have no print: the colour sliders become colour-compensating filters on the camera,
and print exposure becomes light-box brightness.

## Styles

A style is the whole recipe: film, exposure, push, light, grain and print. Built-in styles
are read-only; **Save as…** keeps your settings as your own style. Styles are stored in
`~/Library/Application Support/colour-darkroom/styles.json` and can be exported and
imported as `.json` files.

## Export

The export button (bottom of the tool strip) writes a full-resolution 16-bit TIFF, 16-bit
PNG or JPEG, in sRGB, with the camera's EXIF kept and the recipe in the file's comment, plus
an optional `.json` recipe beside it. By default exports go into a `colour-exports` folder
next to each photo. File names come from a template: `{name}`, `{style}`, `{film}`,
`{print}`, `{date}`, `{time}`, `{n}`.

## The command line

```sh
colour styles                                          # styles, films and prints
colour render photo.NEF --style "Gold 200 · scan" --out gold.jpg
colour render photo.NEF --film ektar-100 --print endura --ev 1 --edge 3000 --out ektar.tif
colour sheet photo.NEF --out sheet.jpg                 # every built-in style on one sheet
```

## About the film stocks

Every stock is **illustrative**: its parameters give the character photographers describe
for the named film, judged by eye, not measured from its datasheet. The names describe the
intended look only.
