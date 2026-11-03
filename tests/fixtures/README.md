# Test fixtures

- `original/`: the 25 `.WQ!` spreadsheets and the `README` from the data diskettes of *Personnel Attrition Rates in
  Historical Land Combat Operations* (US Army Concepts Analysis Agency; DTIC ADM000368).
- `quattro-wq1/`: each sheet opened in Quattro Pro 4.0 and saved as `.WQ1`. Quattro changes some formula
  results slightly on loading, so these aren't an exact copy of the originals.
- `same-session/`: CRETE, ADE, and KRIVO saved as both `.WQ1` and `.WQ!` from one Quattro session, so the two
  match exactly.
- `variants/`: the same three sheets (`C`, `A`, `K`) with SQZ! options, and the password `Secret1`.
  `BL` removes blanks, `AP` stores approximate values, `RM` removes values, `PW` has a password, and `V1`,
  `VA`, and `PV` use the older `Z15` format: exact values, approximate values, and a password.

The repo's license covers the code, not these data files. They aren't part of the published crate.
