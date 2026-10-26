# Ilford (HARMAN) film datasheets

Ilford publishes one characteristic curve per film (Delta 3200: a family) on a **relative**
exposure axis. The curves are Ilford's; the files here are their plotted coordinates. Axes:
`log_h` relative log exposure as plotted, `density` diffuse density. Retrieved 2026-10-24.

| File | Datasheet | Curve | How read |
|---|---|---|---|
| `hp5-plus_ilfotec-hc-1+31_6.5min.csv` | HP5 PLUS Technical Information, Nov 2018, p. 5 ([ilfordphoto.com](https://www.ilfordphoto.com/amfile/file/download/file/1903/product/691/product_datasheet_hp5plus.pdf)) | ILFOTEC HC 1+31, 6½ min, 20 °C | raster image, traced |
| `fp4-plus_ilfotec-hc-1+31_8min.csv` | FP4 PLUS Technical Information, Nov 2018, p. 5 ([ilfordphoto.com](https://www.ilfordphoto.com/amfile/file/download/file/1919/product/690/product_datasheet_fp4plus.pdf)) | ILFOTEC HC 1+31, 8 min, 20 °C | raster image, traced |
| `delta-400_id-11-stock_8min-24c.csv` | DELTA 400 Professional Technical Information, Nov 2018, p. 6 (ilfordphoto.com download 1915/424, read via the Internet Archive) | ID-11 stock, 8 min, **24 °C** | raster image, traced |
| `delta-100_id-11-stock_8.5min.csv` | 100 DELTA Professional fact sheet, 2010, p. 4 ([macodirect.de copy](https://www.macodirect.de/media/pdf/1d/5d/2c/ID108_Datenblatt_e.pdf)) | ID-11 stock, 8½ min, 20 °C | image strips; page region rendered at 400 dpi and traced |
| `delta-3200_ddx-1+4_{07,09,12,16}min.csv` | DELTA 3200 Professional fact sheet, April 1999, p. 6 ([MIT copy](https://stuff.mit.edu/afs/sipb.mit.edu/user/bert/debug/root/mit/gamadrid/delta3200.pdf)) | ILFOTEC DD-X 1+4, 7/9/12/16 min, 20 °C | vector paths, exact |

Tracing (`tools/raster_curve.py`): all plots share one layout, with grid line 0 at relative
log H 0 and line 8 at 4 (`--x-lines 0:0,8:4`), and density 2.0 on line 2 and 0 on line 6
(`--y-lines 2:2,6:0`). Each trace was checked on an overlay; uncertainty about ±0.02 D.
Delta 3200 (vector): frame 104.99,87.93,280.1,203.32 with x 0.0197…4.5008 and y −0.002…2.995
from the grid's tick lines; curves identified by density order (7 < 9 < 12 < 16 min).

Used by `data/calibrations/ilford/` (design 0002 §6, Level 1).
