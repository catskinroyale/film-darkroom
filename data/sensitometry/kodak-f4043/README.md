# Kodak F-4043: T-MAX 400 Film

Source: KODAK PROFESSIONAL T-MAX 400 Film, Technical Data F-4043, February 2016 (Kodak
Alaris), <https://imaging.kodakalaris.com/sites/default/files/files/products/f4043_TMax_400.pdf>,
retrieved 2026-10-24. The curves are Kodak's; the files here are their plotted coordinates,
read from the PDF's vector paths with `tools/datasheet_curves.py` and checked on an overlay.

Page 8.

| File | Plot | Notes |
|---|---|---|
| `tmax-400_d76-stock_small-tank_{06,08,11}min.csv` | Characteristic Curves, D-76, small tank, 20 °C, diffuse visual | frame 83.9,56.5,268.3,241.0; −4…1 log H, 0…4 D; 11 min solid, 8 min dash 2.9, 6 min dash 1.4 |
| `tmax-400_contrast-index_{d76-stock,hc110-b,d76-1+1}_small-tank.csv` | Contrast Index Curves, small tank, 20 °C | frame 358.3,327.2,542.7,511.6; 2…16 min, CI 0.3…0.9; drawing order @1 D-76, @0 HC-110 B, @2 D-76 1:1 (identified by start time and overlay) |
