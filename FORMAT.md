# The SQZ! `.WQ!` format (version `Z20`)

This describes files written by SQZ!, Synex Systems' spreadsheet compressor, as shipped inside Quattro
Pro 4.0 for DOS. It comes from a disassembly of the reader driver `FRWQ!.TRN` ("Copyright (C) 1987,1988
Synex Systems Corp. Release 2.00"). Hex addresses are offsets in that file.

The format is not a general compressor. It reencodes each WK1-style spreadsheet record with
field-specific codes, a fixed character code, and references to earlier cells.

## File layout

| Offset | Contents |
|---|---|
| `0x00` | `Z20` magic and version |
| `0x03` | Option bytes, `N` when off. `0x04`: `Y` omits formula results. `0x05`: modem armour. `0x06`: password. `0x07`: `N` stores formula results as raw doubles |
| `0x18` | u16 checksum, verified only for password files |
| `0x1A` | u32 file offset of the formula table |
| `0x2E` | Start of the record bit stream |
| After the stream | `SFTSP!\r\nZ`, then the formula table, `!!\r\nZAPPTR!\r\nZ`, a pointer block, `0xEF` padding, and `Z***** End of SQZ! *****!\r\n` |

The formula table is byte-aligned: a u16 count, then for each entry a u16 id, a u16 length, and that
many bytes of formula record. Each entry's body is stored uncompressed.

## Bit stream

Bits are read most significant first (`0x3097`). Raw bytes in the stream are not byte-aligned. Every
record starts with a 3-bit tag (`0x37A1`):

| Tag | Meaning |
|---|---|
| 1 | Integer: address, then small integer |
| 2 | Number: address, then packed constant |
| 3 | Label: address, then 3-bit alignment prefix and string |
| 4 | Formula: address, formula reference, then result |
| 5 | Any other record: 8-bit type, then its contents |
| 6 | Repeat: next column, same row, copied from the current column |
| 0 | Repeat: next column, same row, copied from the last cell in that next column |
| 7 | Repeat: column field, copied from the last cell in that column; the row advances if the column did not |

A repeated formula reuses its code, but a new result follows in the stream. Each column remembers its
last cell, except that a label record over `0x14` bytes doesn't update that memory or the current
position (`0x38FE`). The record stream ends with tag 5, type 1.

### Address (`0x3405`)

State starts at `0xFFF0` and each field remembers its last value. Tag 7 updates the column state too.

- **Format:** 2 bits. `0` same as last, `2` is `0xFF`, otherwise 8 bits follow.
- **Column:** 2 bits. `0` is column 0, `1` is last + 1, `2` is last + 2 + a 2-bit value, `3` is an 8-bit value.
- **Row:** 1 bit. `0` same as the last explicit row, `1` means 13 bits follow.

### Numbers

- **Small integer (`0x32DC`):** 1 bit, then 7 bits if it is 0 or 16 bits if it is 1.
- **Packed constant (`0x35C4`):** 3 bits `d`, 1 sign bit, then the IEEE exponent: 6 bits + `0x3E0` if `d & 4`,
  otherwise 11 bits. Mantissa bits follow from the top: 8 always, then 8 if `d & 3 >= 1`, 8 if `>= 2`,
  and 12 + 16 if `== 3`. Omitted bits are zero.
- **Formula result (`0x34E8`):** 8 raw bytes, little-endian, when byte `0x07` is `N`.

### Formula reference (`0x391B`)

1 bit. `1`: small-integer length, then the formula bytes inline. `0`: small-integer table id. Either
way the record gets a u16 length, then the formula bytes.

### Label text (`0x38AA`, `0x32F7`, `0x447D`)

The 3-bit prefix is `0` `\`, `1` `'`, `2` `"`, `3` `^`, otherwise none. An 11-bit field then gives the
size: `v >> 3` is one more than the count of whole bytes, and `v & 7` is the count of extra bits.

The text starts with a 3-bit mode, then prefix-coded characters from the table in `src/text_code.rs`, a
complete code with 72 symbols and at most 16 bits. Mode 0 instead uses bytes, with `0` meaning "repeat the
last byte, next byte minus one times".

| Mode | Case rule for lowercase letters |
|---|---|
| 1 | Unchanged |
| 2 | All uppercase |
| 3 | First letter uppercase |
| 4 | First letter of each word uppercase |
| 5 | One bit after each letter; `1` means uppercase |

Two symbols escape. `0x1F`: 5 bits give a control character, or if they are 0, 8 bits `n` repeat the
last character `n - 1` times. `0x80`: 7 bits give a character with the high bit set.

## Options

- **Remove Blanks** (byte `0x03`) drops empty, formatted cells. Decoding is unaffected.
- **Storage of Values** sets byte `0x04` (Remove) or byte `0x07` (Approximate). With Remove, a first formula
  gets a zero result and a repeated one gets `00 00 00 00 00 00 F0 FF`. With Approximate, a result is 1 sign
  bit, 8 exponent bits (`0` is zero, `0xFF` is `0x7FF`, otherwise add `0x380`), then 23 mantissa bits (`0x34E8`).
- **Version:** `Z15` stores constants as 8 raw bytes, and each string as 1 width bit, then 7- or 8-bit
  characters ending in 0. A 2 is an escape: the next character repeats the last character that many times, or
  if 0 is a literal 2. Either way, 2 becomes the last character (`0x348D`). The writer in Quattro Pro 4.0 jumps
  past its version check at `0x2959`, so it always writes `Z20`, whatever the menu says.
- **Password** (byte `0x06`): upper-cased, at most 15 bytes. The word at `0x18` is `(length << 12)` plus each
  byte times its 1-based position, and is itself encrypted. Each data byte `c` at password index `i` becomes
  `rol8(c ^ rol8(p[i], i) ^ k, i)`. `k` is the high byte of the check word after rotating its high byte left by
  `r8` and the whole word left by `r16`. `r8` starts at 5 and `r16` at 9; `r8` advances after each pass through
  the password, and `r16` when `r8` wraps. Position resets at each section: the words at `0x18`, the table,
  and the stream.
- **Modem armour** (byte `0x05`): each group of 4 printable bytes holds 3 data bytes as 6 bits + `0x20`, and the
  first holds their top 2 bits. After 18 groups, 4 bytes of line break follow. It applies beneath the password
  layer, to everything from offset `0x18` on.

## Other records

Record `0x47` (`0x3AB3`) holds two cell addresses, a 1-bit flag, and a string, and expands to a 16-byte name,
the addresses, and the flag.

## Not covered

- The driver's 2,500-byte repeat cache (`0x46BD`) isn't emulated. The writer only refers to cells it cached, so
  remembering more can't change how a valid file decodes. A malformed file may be accepted that the driver rejects.
- `.WK!` and `.WK$` use a different driver, which hasn't been studied. `FRWKZ.TRN` is byte-identical to `FRWQ!.TRN`.

## How each was tested

| Feature | Evidence |
|---|---|
| Blanks, Approximate, Remove, password | Files written by Quattro Pro 4.0 |
| `Z15` | Files from Synex's writer with one jump patched, since the build in Quattro Pro 4.0 can't write it |
| Modem armour, record `0x47` | Synthetic files built from this description, so they only show self-consistency |
