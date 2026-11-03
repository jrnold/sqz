# Quattro Pro test rig

These scripts drive Quattro Pro 4.0 for DOS under DOSBox-X on macOS, which is how the fixtures in
`tests/fixtures` were made. Quattro Pro isn't included; you need your own copy.

1. `brew install dosbox-x nasm`, then pick a rig folder: `export QP_RIG=~/qp-rig`, and
   `mkdir -p "$QP_RIG/c/SRC" "$QP_RIG/c/WORK" "$QP_RIG/shots"`.
2. Copy both floppy disks into `$QP_RIG/c/SRC`, and the sheets into `$QP_RIG/c/WORK`.
3. `nasm -f bin -o "$QP_RIG/c/KEYS.COM" keys.asm`. This is a small keystroke feeder for DOS, which avoids the
   127-character command limit of DOSBox-X's own `AUTOTYPE`.
4. `swiftc -O wid.swift -o "$QP_RIG/wid"`. It finds the emulator's window, for screenshots.
5. Run `INSTALL` from `C:\SRC` once through `key.sh`, reading each screen from `$QP_RIG/shots`.

To save a sheet in both forms, in one session:

```bash
./key.sh save '{w8}{d0.5}/fa{w1}OUT.WQ1{enter}{w4}/fa{w1}OUT.WQ!{enter}{w5}' 'cd \WORK;;C:\QPRO\Q IN.WQ!' 30
```

Save in the current folder: a path typed into the Save As prompt failed. `/fus` opens the SQZ! options. For a
password, add ` P` after the file name. See `mk.py` for the key syntax.

`FRWQ!.TRN` in the Quattro Pro folder is the SQZ! reader, and `FSWQ!.TRN` the writer. Its build always writes
`Z20`, so the `Z15` fixtures came from a copy with the jump at `0x2959` changed to `EB 15`.
