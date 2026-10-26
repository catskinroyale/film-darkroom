# Kodak F-4017: TRI-X 320 and 400 Films

Source: KODAK PROFESSIONAL TRI-X 320 and 400 Films, Technical Data F-4017, February 2016
(Kodak Alaris), <https://imaging.kodakalaris.com/sites/default/files/files/products/f4017_TriX.pdf>,
retrieved 2026-10-24. The curves are Kodak's; the files here are their plotted coordinates.

Extracted from the PDF's vector paths with `tools/datasheet_curves.py` (exact plotted points;
checked by overlaying them on the page). Page 8.

| File | Plot | Process | Axes |
|---|---|---|---|
| `tri-x-400_35mm_d76-stock_large-tank_{06,08,10,12}min.csv` | Characteristic Curves, 400TX, 35 mm | D-76, large tank, agitation every minute, 20 °C; diffuse visual density | `log_h` log10 lux-seconds, `density` diffuse |
| `tri-x-400_contrast-index_d76-stock_small-tank.csv` | Contrast Index Curves, 400TX (D-76 line) | small tank (30 s agitation), 20 °C | `time_min`, `contrast_index` |

Extraction commands (frame and legend in PDF points):

```sh
python3 -I tools/datasheet_curves.py extract f4017.pdf --page 8 \
  --frame 81.4,63.5,265.8,248.0 --legend 92.0,119.8,175.1,156.5 --x=-4,1 --y=0,4 --out DIR \
  tri-x-400_35mm_d76-stock_large-tank_12min=16.2 ..._10min=6.5 ..._08min=3.2 ..._06min=solid
python3 -I tools/datasheet_curves.py extract f4017.pdf --page 8 \
  --frame 353.9,71.0,538.3,255.5 --legend 470,160,540,250 --x=0,30 --y=0,1.2 \
  --columns time_min,contrast_index --out DIR tri-x-400_contrast-index_d76-stock_small-tank=solid
```

Used by `data/calibrations/tri-x_d76-stock.csv` (design 0002 §2).
