# Decision log

Product and scope decisions for Colour Darkroom, newest first. Model decisions (C1–C9) are
in [design 0001](design/0001-colour-film-model.md#2-decisions).

| # | Date | Decision | Rationale |
|---|------|----------|-----------|
| P3 | 2026-11-02 | **Stocks start illustrative; datasheet calibration follows.** Every film and print stock ships with parameters chosen by eye and labelled illustrative, in the About window, the curves panel and every export recipe. | A working colour pipeline with the right structure comes first. Kodak publishes per-layer curves, sensitivities and dye spectra for most of these stocks, so they can be fitted one at a time (design 0001 §4) without changing the model. |
| P2 | 2026-11-02 | **Copy, don't share, BW Darkroom's foundations.** `cd-core` (planes, curves, noise, blur), `cd-io`, `cd-raw` and the desktop shell (theme, widgets, folders, browser, worker, export) started as copies of BW Darkroom's crates. | The two apps evolve independently: neither can break the other, and each releases on its own schedule. If they drift apart in ways worth reconciling, a shared crate can be extracted later. |
| P1 | 2026-11-02 | **A separate app from BW Darkroom.** Colour Darkroom has its own name, bundle id, settings, styles file (`~/Library/Application Support/colour-darkroom/`), sidecars (`<file>.cd.json`) and export folder (`colour-exports`). | The user asked for a colour film app separate from the black-and-white one. Separate sidecars mean both apps can edit the same raw files without overwriting each other's adjustments. |
