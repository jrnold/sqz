# How the decoder was verified

The decoder was written from a disassembly of the SQZ! reader that ships with Quattro Pro 4.0 for DOS. Quattro Pro,
running under DOSBox-X, then served as ground truth. `cargo test` repeats these checks using saved files.

| Check | Result |
|---|---|
| `.WQ!` and `.WQ1` saved from one Quattro session, for 3 sheets | 10,292 of 10,292 cells identical, byte for byte |
| 25 original files against Quattro's `.WQ1` | All 127,651 cells have the same position, type, format, and formula; 126,507 are byte-identical |

## The differing cells

Of the 1,144 cells that differ, 1,092 are formula results that differ by a relative 1e-9 or less, and 52 are
stored `ERR` or `NA` values with different payload bytes. The same-session saves show why: Quattro changes these
results itself when it loads a file, and a re-saved file differs from its original on the same number of cells. So
a decoded `.WQ!` is closer to what was stored than a `.WQ1` made with Quattro is. The cause inside Quattro isn't
known, and the sheet's calculation mode doesn't fully explain it.

## Other records

Quattro 4.0 writes more non-cell records than these files hold: 217 across the 25 files.

| Type | Count | What's known |
|---|---|---|
| `0xCB`, `0xD6`, `0xD7` | 39 each | One per graph, starting with its name. The other fields are unknown |
| `0x1D` | 25 | A second copy in each file, usually with a different last byte. Meaning unknown |
| `0xC4`, `0xCC`, `0xD1` | 25 each | One per file, with the same value in all 25. Meaning unknown |

The same types with different bytes: `0x07`, `0x0B`, `0x25`, `0x26`, `0x27`, `0x33`, `0x9A`, and `0xA2`. The decoder
pads `0x25` to `0x27` to fixed sizes; the rest haven't been studied. So a `.WQ1` from this tool isn't byte-identical to
Quattro's.

## Loading the `.WQ1` output

These are load tests, because WQ1 has no public specification to validate against.

- **Quattro Pro 4.0** opens CRETE, and it looks right. Its labels are byte-identical to Quattro's for that sheet.
- **LibreOffice 26.8** opens all 25, with CSV output identical to that from Quattro's own `.WQ1`. CRETE's formulas import
  as formulas. LibreOffice showed about 8 significant digits, so this check is coarse. Don't use its values as exact.

## Recovered values

CSV exports of these sheets made through LibreOffice in 2013 had 276 error cells. 226 of them have numeric results
in the `.WQ!` files. The other 50 are formulas whose stored result is itself `ERR` or `NA`. In CRETE, the casualty totals
decode to 6000.03 and 21,299.99, against the sheet's targets of 6,000 and 21,300.
