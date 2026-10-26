# Fitted calibrations

Level-2 development calibrations, fitted to manufacturer sensitometry and compiled into
`bw-negative` (`fitted.rs`). Each file is written by `tools/fit_development.py`; its header
states the model. Design 0002 §2 gives the method and the validation.

| File | Film + developer | Fitted to |
|---|---|---|
| `tri-x_d76-stock.csv` | Tri-X 400 + D-76 stock | Kodak F-4017 D-76 family, 6/8/10/12 min, large tank (`data/sensitometry/kodak-f4017/`) |
| `tmax-400_d76-stock.csv` | T-Max 400 + D-76 stock | Kodak F-4043 D-76 family, 6/8/11 min, small tank |
| `tmax-100_d76-stock.csv` | T-Max 100 + D-76 stock | Kodak F-4016 D-76 family, 6/7½/10 min, small tank |
| `developer-rates.csv` | Tri-X and T-Max 400 in HC-110 B and D-76 1+1 | Kodak's CI-vs-time lines: the D-76 curve at a fitted rate (Level 2−) |
| `ilford/*.csv` | HP5 Plus, FP4 Plus, 100 Delta, Delta 400, Delta 3200 | Ilford's curves, Level 1: borrowed growth (Delta 3200: its own DD-X family), ISO-anchored axis (`data/sensitometry/ilford/`) |

Refit (needs numpy and scipy):

```sh
K=data/sensitometry/kodak-f4017/tri-x-400
python3 -I tools/fit_development.py --normal-time 7.75 --holdout 8 \
  --curve 6=${K}_35mm_d76-stock_large-tank_06min.csv --curve 8=${K}_35mm_d76-stock_large-tank_08min.csv \
  --curve 10=${K}_35mm_d76-stock_large-tank_10min.csv --curve 12=${K}_35mm_d76-stock_large-tank_12min.csv \
  --ci ${K}_contrast-index_d76-stock_small-tank.csv --ci-time-factor 1.155 \
  --out data/calibrations/tri-x_d76-stock.csv
```

T-Max 400 / T-Max 100: `--normal-time 7.5` / `6.5`, small-tank curves, `--holdout 8` / `7.5`.
Developer rates: add `--ci-time-factor 1.155` (Tri-X only) and `--rate hc110-b=<CI line>`.

Ilford (Level 1), e.g. HP5 Plus:

```sh
python3 -I tools/fit_development.py --curve 6.5=data/sensitometry/ilford/hp5-plus_ilfotec-hc-1+31_6.5min.csv \
  --normal-time 6.5 --borrow data/calibrations/tri-x_d76-stock.csv --pin-base --anchor-iso 400 \
  --out data/calibrations/ilford/hp5.csv
```

FP4 Plus: 8 min, ISO 125, Tri-X growth. 100 Delta: 8.5, ISO 100, T-Max 100. Delta 400: 8,
ISO 400, T-Max 400. Delta 3200: its four DD-X curves, `--normal-time 9.5 --holdout 12
--pin-base --anchor-iso 1000`, no borrowing.
