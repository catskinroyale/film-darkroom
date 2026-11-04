# Papers

Printing papers compiled into `mono-darkroom`: one characteristic curve per variable-contrast
filter grade, from the manufacturer's datasheet (design 0002 §3). Written by
`tools/paper_family.py`; columns `filter,log_h,density`.

| File | Paper | Source |
|---|---|---|
| `kodak-polymax-ii-rc.csv` | Kodak Polymax II RC, F surface, Polymax filters −1…5+, Dektol 1:2 | Kodak G-26 (`data/sensitometry/kodak-g26/`) |

Rebuild:

```sh
G=data/sensitometry/kodak-g26/polymax-ii-rc_f_dektol-1+2_filter
python3 -I tools/paper_family.py --out data/papers/kodak-polymax-ii-rc.csv --comment "..." -- \
  "-1=${G}-minus1.csv:140" "0=${G}-0.csv:140" "1=${G}-1.csv:130" "1.5=${G}-1.5.csv:120" \
  "2=${G}-2.csv:120" "3=${G}-3.csv:100" "3.5=${G}-3.5.csv:80" "4=${G}-4.csv:70" "5=${G}-5plus.csv:40"
```
