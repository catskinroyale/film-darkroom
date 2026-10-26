# Kodak G-26: POLYMAX II RC Paper

Source: KODAK PROFESSIONAL POLYMAX II RC Paper, Technical Data G-26, June 2005 (Eastman
Kodak), copy at <https://125px.com/docs/paper/kodak/g26.pdf>, retrieved 2026-10-24. The
curves are Kodak's; the files here are their plotted coordinates.

Page 7, "Characteristic Curves", F surface, KODAK POLYMAX Filters, KODAK DEKTOL Developer
(1:2), 60 s, 20 °C, continuous agitation. Axes: `log_h` log10 exposure as plotted
(lux-seconds), `density` reflection density. One file per filter; `minus1` is filter −1 and
`5plus` is Kodak's 5+. `white` is the white-light curve (drawn identically in all three
plots).

Extracted from the PDF's vector paths with `tools/datasheet_curves.py`. The curves are all
drawn alike and labelled with leader lines, so each was identified by its ISO 6846 range and
speed against Kodak's table on p. 3, then checked against the leader lines on an overlay
(design 0002 §3). Plots (frame in PDF points, axes −3…1 log H, 0…4 D):

| Plot | Frame | Curves (path order) |
|---|---|---|
| top left | 95.0,116.8,279.5,301.3 | @0 filter 3, @1 filter 1, @2 white, @3 filter 5+ |
| bottom left | 88.5,354.4,273.0,538.9 | @2 filter 4, @3 white, @4 filter 0, @5 filter 2 (@0, @1 are ticks) |
| bottom right | 368.2,348.9,552.7,533.4 | @0 filter 1½, @1 filter −1, @2 white, @3 filter 3½ |

Combined into `data/papers/kodak-polymax-ii-rc.csv` by `tools/paper_family.py`.
