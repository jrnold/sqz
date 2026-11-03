//! Reader for SQZ! compressed Quattro Pro spreadsheets (`.WQ!`).
//!
//! SQZ! was a 1980s spreadsheet compressor, built into Quattro Pro for DOS. [`decode`] turns a
//! `.WQ!` file into the WK1-style records it compresses, which [`to_csv`] and [`to_wq1`] can
//! write out. The format is documented in
//! [`FORMAT.md`](https://github.com/jrnold/sqz/blob/main/FORMAT.md).
//!
//! ```no_run
//! let raw = std::fs::read("CRETE.WQ!")?;
//! for record in quattro_sqz::decode(&raw)? {
//!     if let (Some((_, column, row)), Some(value)) = (record.address(), record.value()) {
//!         println!("{column},{row}: {value:?}");
//!     }
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use std::collections::HashMap;
use std::fmt;

// Generated from the driver's table; the bit strings read better without separators.
#[allow(clippy::unreadable_literal)]
mod text_code;
use text_code::TEXT_CODE;

/// Record type of an empty, formatted cell.
pub const BLANK: u16 = 0x0c;
/// Record type of a 16-bit integer cell.
pub const INTEGER: u16 = 0x0d;
/// Record type of a double-precision number cell.
pub const NUMBER: u16 = 0x0e;
/// Record type of a text cell.
pub const LABEL: u16 = 0x0f;
/// Record type of a formula cell.
pub const FORMULA: u16 = 0x10;
/// Record type that ends a file.
pub const EOF: u16 = 0x01;

/// Why a file could not be decoded.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The file doesn't start with a supported SQZ! header.
    UnsupportedHeader(String),
    /// The file is password protected and no password was given.
    PasswordRequired,
    /// The password doesn't match the file's check value.
    WrongPassword,
    /// A record is longer than the driver allows.
    RecordTooLong(usize),
    /// The formula table has an invalid id or length.
    BadFormulaTable,
    /// The data ended early, at this bit offset.
    Truncated(usize),
    /// A label contains an invalid text code.
    BadTextCode,
    /// A row number is out of range.
    BadRow(u32),
    /// A cell refers to a formula missing from the table.
    BadFormulaId(u32),
    /// A cell repeats from a column with no earlier cell.
    BadRepeat(u32),
    /// A column number is out of range.
    BadColumn(u32),
    /// A cell record is too short to hold an address.
    BadCell,
    /// The decoded records would exceed the limit, which suggests a malformed file.
    OutputTooLarge,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::UnsupportedHeader(h) => write!(f, "not a supported SQZ! file: header {h:?}"),
            Error::PasswordRequired => write!(f, "this file is password protected"),
            Error::WrongPassword => write!(f, "wrong password"),
            Error::RecordTooLong(n) => write!(f, "record of {n} bytes is over the driver's limit"),
            Error::BadFormulaTable => write!(f, "invalid formula table"),
            Error::Truncated(p) => write!(f, "file ended early at bit {p}"),
            Error::BadTextCode => write!(f, "invalid text code"),
            Error::BadRow(r) => write!(f, "row {r} is out of range"),
            Error::BadFormulaId(i) => write!(f, "formula id {i} is not in the table"),
            Error::BadRepeat(c) => write!(f, "repeat from empty column {c}"),
            Error::BadColumn(c) => write!(f, "column {c} is out of range"),
            Error::BadCell => write!(f, "a cell record is too short"),
            Error::OutputTooLarge => write!(f, "decoded data is over the size limit"),
        }
    }
}
impl std::error::Error for Error {}

/// One decoded record: a WK1-style type and payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// Record type.
    pub rtype: u16,
    /// Record payload.
    pub data: Vec<u8>,
}

impl Record {
    /// Cell address as `(format, column, row)`, for cell records.
    #[must_use]
    pub fn address(&self) -> Option<(u8, u16, u16)> {
        if !matches!(self.rtype, BLANK | INTEGER | NUMBER | LABEL | FORMULA) || self.data.len() < 5
        {
            return None;
        }
        Some((
            self.data[0],
            u16::from_le_bytes([self.data[1], self.data[2]]),
            u16::from_le_bytes([self.data[3], self.data[4]]),
        ))
    }
    /// Cell contents, or `None` if this isn't a well-formed cell record.
    #[must_use]
    pub fn value(&self) -> Option<Value> {
        self.address()?;
        let p = &self.data[5..];
        match self.rtype {
            BLANK => Some(Value::Blank),
            INTEGER => {
                let bytes = p.first_chunk::<2>()?;
                Some(Value::Integer(i16::from_le_bytes(*bytes)))
            }
            LABEL => {
                let (prefix, rest) = match p.first() {
                    Some(b'\\' | b'\'' | b'"' | b'^') => (Some(p[0] as char), &p[1..]),
                    _ => (None, p),
                };
                let end = rest.iter().position(|&b| b == 0).unwrap_or(rest.len());
                Some(Value::Label {
                    prefix,
                    text: cp437(&rest[..end]),
                })
            }
            _ => Value::from_double(p),
        }
    }
    /// Formula bytecode, as stored in a WK1-style formula record.
    #[must_use]
    pub fn formula_code(&self) -> Option<&[u8]> {
        if self.rtype == FORMULA {
            self.data.get(15..)
        } else {
            None
        }
    }
}

/// A cell's contents. A formula yields its stored result.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// An empty cell.
    Blank,
    /// A 16-bit integer.
    Integer(i16),
    /// A number, or a formula's result.
    Number(f64),
    /// Text, with its alignment prefix if it has one.
    Label {
        /// One of `\\`, `'`, `"`, or `^`.
        prefix: Option<char>,
        /// The text, decoded from code page 437.
        text: String,
    },
    /// An `ERR` or `NA` stored by the spreadsheet: a double with an all-ones exponent.
    Special,
}

impl Value {
    fn from_double(p: &[u8]) -> Option<Value> {
        let v = f64::from_le_bytes(*p.first_chunk::<8>()?);
        Some(if v.is_finite() {
            Value::Number(v)
        } else {
            Value::Special
        })
    }
}

fn cp437(b: &[u8]) -> String {
    const HIGH: &str = "ÇüéâäàåçêëèïîìÄÅÉæÆôöòûùÿÖÜ¢£¥₧ƒáíóúñÑªº¿⌐¬½¼¡«»░▒▓│┤╡╢╖╕╣║╗╝╜╛┐└┴┬├─┼╞╟╚╔╩╦╠═╬╧╨╤╥╙╘╒╓╫╪┘┌█▄▌▐▀αßΓπΣσµτΦΘΩδ∞φε∩≡±≥≤⌠⌡÷≈°∙·√ⁿ²■\u{a0}";
    let high: Vec<char> = HIGH.chars().collect();
    b.iter()
        .map(|&c| {
            if c < 0x80 {
                c as char
            } else {
                high[c as usize - 0x80]
            }
        })
        .collect()
}

/// Byte source: undoes modem armour (0x2E96) and the password cipher (0x2FC9).
struct Source<'a> {
    raw: &'a [u8],
    pos: usize,
    block_start: usize,
    block_size: usize,
    modem: bool,
    groups: u8,
    password: Vec<u8>,
    key: u16,
    pw_index: usize,
    rot16: u32,
    rot8: u32,
}

impl Source<'_> {
    /// Seeks and starts a new driver read buffer, resetting the cipher position (0x2F5F).
    fn enter(&mut self, pos: usize, block_size: usize) {
        self.pos = pos;
        self.block_start = pos;
        self.block_size = block_size;
        self.groups = 0;
        self.pw_index = 0;
        self.rot16 = 9;
        self.rot8 = 5;
    }
    fn raw_byte(&mut self) -> Result<u8, Error> {
        if self.pos - self.block_start >= self.block_size {
            self.block_start = self.pos;
        }
        let b = *self
            .raw
            .get(self.pos)
            .ok_or(Error::Truncated(self.pos * 8))?;
        self.pos += 1;
        Ok(b)
    }
    fn byte(&mut self) -> Result<u8, Error> {
        let mut c = self.raw_byte()?;
        if self.modem {
            c = self.unarmour(c)?;
        }
        if !self.password.is_empty() {
            c = self.decrypt(c);
        }
        Ok(c)
    }
    /// Three data bytes travel as four printable ones, 18 groups to a line. The first byte of each
    /// group holds the top two bits of the other three.
    #[allow(clippy::verbose_bit_mask)] // Group position, as in the driver; clearer than trailing_zeros.
    fn unarmour(&mut self, first: u8) -> Result<u8, Error> {
        let mut c = first;
        if (self.pos - 1 - self.block_start) & 3 == 0 {
            if self.groups == 0x12 {
                for _ in 0..4 {
                    self.raw_byte()?;
                }
                self.groups = 0;
            }
            self.groups += 1;
            c = self.raw_byte()?;
        }
        let index = self.pos - 1 - self.block_start;
        let head = self.raw[self.block_start + (index & !3)].wrapping_sub(0x20);
        let high = head.checked_shl((index as u32 & 3) * 2).unwrap_or(0) & 0xc0;
        Ok(c.wrapping_sub(0x20) | high)
    }
    fn decrypt(&mut self, c: u8) -> u8 {
        let [low, high] = self.key.to_le_bytes();
        let turned = u16::from_le_bytes([low, high.rotate_left(self.rot8)]).rotate_left(self.rot16);
        let i = self.pw_index;
        let mask = self.password[i].rotate_left(i as u32 % 8) ^ turned.to_be_bytes()[0];
        let out = (c ^ mask).rotate_left(i as u32 % 8);
        self.pw_index += 1;
        if self.pw_index == self.password.len() {
            self.pw_index = 0;
            self.rot8 = (self.rot8 + 1) & 7;
            if self.rot8 == 0 {
                self.rot16 = (self.rot16 + 1) & 15;
            }
        }
        out
    }
    fn word(&mut self) -> Result<u16, Error> {
        let low = self.byte()?;
        Ok(u16::from_le_bytes([low, self.byte()?]))
    }
}

/// Bit reader, most significant bit first.
struct Bits<'a> {
    src: Source<'a>,
    acc: u32,
    count: u32,
}

impl Bits<'_> {
    fn read(&mut self, n: u32) -> Result<u32, Error> {
        let mut v = 0;
        for _ in 0..n {
            if self.count == 0 {
                self.acc = u32::from(self.src.byte()?);
                self.count = 8;
            }
            self.count -= 1;
            v = (v << 1) | ((self.acc >> self.count) & 1);
        }
        Ok(v)
    }
    fn bytes(&mut self, n: usize) -> Result<Vec<u8>, Error> {
        (0..n).map(|_| self.read(8).map(|v| v as u8)).collect()
    }
}

/// Reader over a text bit string. Past the end, peeks see zeros, as in the driver.
struct TextBits<'a> {
    d: &'a [u8],
    p: usize,
}
impl TextBits<'_> {
    fn bit(&self, i: usize) -> u32 {
        self.d
            .get(i >> 3)
            .map_or(0, |b| u32::from((b >> (7 - (i & 7))) & 1))
    }
    fn peek(&self, n: usize) -> u32 {
        (0..n).fold(0, |v, i| (v << 1) | self.bit(self.p + i))
    }
}

// `t`, `k`, `c`, `n`, and `v` follow the driver's registers and the format's own notation.
#[allow(clippy::many_single_char_names)]
fn decode_text(buf: &[u8], nbits: usize, code: &HashMap<(u8, u16), u8>) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    if nbits < 3 {
        return Ok(out);
    }
    let mut t = TextBits { d: buf, p: 0 };
    let mode = t.peek(3);
    t.p = 3;
    if mode == 0 {
        // Byte-oriented run coding, starting at the second byte.
        let (mut i, mut n, mut last) = (1usize, (nbits - 3) >> 3, 0u8);
        while n > 0 {
            let c = *buf.get(i).ok_or(Error::BadTextCode)?;
            i += 1;
            if c == 0 {
                n = n.saturating_sub(1);
                let k = *buf.get(i).ok_or(Error::BadTextCode)?;
                i += 1;
                out.extend(std::iter::repeat_n(last, run_length(k)));
            } else {
                out.push(c);
                last = c;
            }
            if out.len() > TEXT_LIMIT {
                break;
            }
            n = n.saturating_sub(1);
        }
        return Ok(out);
    }
    let (mut last, mut first, mut word) = (0u8, true, true);
    while t.p < nbits {
        let mut sym = None;
        for k in 1..=16usize {
            if let Some(&s) = code.get(&(k as u8, t.peek(k) as u16)) {
                sym = Some((s, k));
                break;
            }
        }
        let (sym, k) = sym.ok_or(Error::BadTextCode)?;
        t.p += k;
        if t.p > nbits {
            break;
        }
        match sym {
            0x1f => {
                let v = t.peek(5) as u8;
                t.p += 5;
                if t.p > nbits {
                    break;
                }
                if v != 0 {
                    out.push(v);
                    last = v;
                } else {
                    let n = t.peek(8) as u8;
                    t.p += 8;
                    out.extend(std::iter::repeat_n(last, run_length(n)));
                }
            }
            0x80 => {
                let c = 0x80 | t.peek(7) as u8;
                t.p += 7;
                if t.p > nbits {
                    break;
                }
                out.push(c);
                last = c;
            }
            mut c => {
                let low = c.is_ascii_lowercase();
                match mode {
                    2 if low => c -= 32,
                    3 if low && first => {
                        c -= 32;
                        first = false;
                    }
                    4 if low && word => {
                        c -= 32;
                        word = false;
                    }
                    5 if low => {
                        if t.p >= nbits {
                            break;
                        }
                        if t.bit(t.p) == 1 {
                            c -= 32;
                        }
                        t.p += 1;
                    }
                    _ => {}
                }
                out.push(c);
                last = c;
                if c == b' ' {
                    word = true;
                }
            }
        }
        if out.len() > TEXT_LIMIT {
            break;
        }
    }
    Ok(out)
}

/// The driver stops decoding a string once it has written more than this (0x4490).
const TEXT_LIMIT: usize = 0xf4;
/// The driver's limit on a record's length (0x3A8B).
const RECORD_LIMIT: usize = 0x80f;
/// The largest worksheet position: 256 columns and 8,192 rows.
const MAX_COLUMN: u32 = 0xff;
const MAX_ROW: u32 = 0x1fff;
/// Repeats cost a few bits each, so a small file could expand a long way. No real sheet comes close.
const OUTPUT_LIMIT: usize = 256 * 1024 * 1024;

/// The count a run byte stands for: one less, with zero meaning 256 (0x4531).
fn run_length(k: u8) -> usize {
    match k.wrapping_sub(1) {
        0 => 256,
        n => n as usize,
    }
}

struct Slot {
    rtype: u16,
    fmt: u8,
    payload: Vec<u8>,
}

struct Decoder<'a> {
    b: Bits<'a>,
    old_version: bool,
    table: HashMap<u32, Vec<u8>>,
    code: HashMap<(u8, u16), u8>,
    no_values: bool,
    raw_doubles: bool,
    last_row: u32,
    last_col: u32,
    last_fmt: u32,
    cur_row: u32,
    cur_col: u32,
    slots: HashMap<u32, Slot>,
}

impl Decoder<'_> {
    fn fmt(&mut self) -> Result<u8, Error> {
        let v = match self.b.read(2)? {
            0 => self.last_fmt,
            2 => 0xff,
            _ => self.b.read(8)?,
        };
        self.last_fmt = v;
        Ok(v as u8)
    }
    fn col(&mut self) -> Result<u32, Error> {
        let v = match self.b.read(2)? {
            0 => 0,
            1 => self.last_col + 1,
            2 => self.b.read(2)? + 2 + self.last_col,
            _ => self.b.read(8)?,
        } & 0xff;
        self.last_col = v;
        Ok(v)
    }
    fn row(&mut self) -> Result<u32, Error> {
        let v = if self.b.read(1)? == 1 {
            self.b.read(13)?
        } else {
            self.last_row
        };
        if v >= 0x2000 {
            return Err(Error::BadRow(v));
        }
        self.last_row = v;
        Ok(v)
    }
    fn addr(&mut self) -> Result<(u8, u32, u32), Error> {
        Ok((self.fmt()?, self.col()?, self.row()?))
    }
    fn small(&mut self) -> Result<u32, Error> {
        if self.b.read(1)? == 1 {
            self.b.read(16)
        } else {
            self.b.read(7)
        }
    }
    fn constant(&mut self) -> Result<Vec<u8>, Error> {
        if self.old_version {
            return self.b.bytes(8);
        }
        let d = self.b.read(3)?;
        let sign = u64::from(self.b.read(1)?);
        let e = u64::from(if d & 4 != 0 {
            self.b.read(6)? + 0x3e0
        } else {
            self.b.read(11)?
        });
        let mut m = u64::from(self.b.read(8)?) << 44;
        let n = d & 3;
        if n >= 1 {
            m |= u64::from(self.b.read(8)?) << 36;
        }
        if n >= 2 {
            m |= u64::from(self.b.read(8)?) << 28;
        }
        if n >= 3 {
            m |= u64::from(self.b.read(12)?) << 16;
            m |= u64::from(self.b.read(16)?);
        }
        Ok(((sign << 63) | ((e & 0x7ff) << 52) | m)
            .to_le_bytes()
            .to_vec())
    }
    /// A formula's stored result. In a file without results, the driver fills a first cell with
    /// zero and a repeated one with an all-ones exponent (0x3982, 0x3BD5).
    fn result(&mut self, repeated: bool) -> Result<Vec<u8>, Error> {
        if self.no_values {
            let top = if repeated { 0xfff0u16 } else { 0 };
            return Ok(u64::from(top).wrapping_shl(48).to_le_bytes().to_vec());
        }
        if self.raw_doubles {
            return self.b.bytes(8);
        }
        let sign = u64::from(self.b.read(1)?);
        let e = self.b.read(8)?;
        let e = u64::from(if e == 0xff {
            0x7ff
        } else if e != 0 {
            e + 0x380
        } else {
            0
        });
        let hi = u64::from(self.b.read(12)?);
        let lo = u64::from(self.b.read(11)?);
        Ok(((sign << 63) | (e << 52) | (hi << 40) | (lo << 29))
            .to_le_bytes()
            .to_vec())
    }
    fn string(&mut self) -> Result<Vec<u8>, Error> {
        if self.old_version {
            return self.old_string();
        }
        let v = self.b.read(11)?;
        let (n, r) = ((v >> 3) as usize, v & 7);
        let whole = n.saturating_sub(1);
        let mut buf = self.b.bytes(whole)?;
        if r != 0 {
            buf.push((self.b.read(r)? << (8 - r)) as u8);
        }
        decode_text(&buf, whole * 8 + r as usize, &self.code)
    }
    /// Before `Z20`: 7- or 8-bit characters ending in 0, with 2 as a run escape (0x334A, 0x348D).
    fn old_string(&mut self) -> Result<Vec<u8>, Error> {
        let width = if self.b.read(1)? == 1 { 8 } else { 7 };
        let (mut out, mut last) = (Vec::new(), 0u8);
        loop {
            match self.b.read(width)? as u8 {
                0 => return Ok(out),
                2 => {
                    match self.b.read(width)? as usize {
                        0 => out.push(2),
                        n => out.extend(std::iter::repeat_n(last, n)),
                    }
                    last = 2;
                }
                c => {
                    out.push(c);
                    last = c;
                }
            }
            if out.len() > RECORD_LIMIT {
                return Err(Error::RecordTooLong(out.len()));
            }
        }
    }
    fn label(&mut self) -> Result<Vec<u8>, Error> {
        let prefix: &[u8] = match self.b.read(3)? {
            0 => b"\\",
            1 => b"'",
            2 => b"\"",
            3 => b"^",
            _ => b"",
        };
        let mut out = prefix.to_vec();
        out.extend(self.string()?);
        out.push(0);
        Ok(out)
    }
    fn formula_code(&mut self) -> Result<Vec<u8>, Error> {
        let code = if self.b.read(1)? == 1 {
            let n = self.small()? as usize;
            if n >= RECORD_LIMIT {
                return Err(Error::RecordTooLong(n));
            }
            self.b.bytes(n)?
        } else {
            let i = self.small()?;
            self.table.get(&i).ok_or(Error::BadFormulaId(i))?.clone()
        };
        if code.len() + 0xf > RECORD_LIMIT {
            return Err(Error::RecordTooLong(code.len() + 0xf));
        }
        let mut out = (code.len() as u16).to_le_bytes().to_vec();
        out.extend(code);
        Ok(out)
    }
    fn record(rtype: u16, fmt: u8, c: u32, r: u32, payload: &[u8]) -> Record {
        let mut data = vec![fmt];
        data.extend((c as u16).to_le_bytes());
        data.extend((r as u16).to_le_bytes());
        data.extend(payload);
        Record { rtype, data }
    }
    fn cell(&mut self, rtype: u16, fmt: u8, c: u32, r: u32, payload: Vec<u8>) -> Record {
        let rec = Self::record(rtype, fmt, c, r, &payload);
        self.slots.insert(
            c,
            Slot {
                rtype,
                fmt,
                payload,
            },
        );
        (self.cur_row, self.cur_col) = (r, c);
        rec
    }
    fn formula(
        &mut self,
        fmt: u8,
        c: u32,
        r: u32,
        code: Vec<u8>,
        repeated: bool,
    ) -> Result<Record, Error> {
        let mut payload = self.result(repeated)?;
        payload.extend(&code);
        let rec = Self::record(FORMULA, fmt, c, r, &payload);
        self.slots.insert(
            c,
            Slot {
                rtype: FORMULA,
                fmt,
                payload: code,
            },
        );
        (self.cur_row, self.cur_col) = (r, c);
        Ok(rec)
    }
    fn repeat(&mut self, src: u32, c: u32, r: u32) -> Result<Record, Error> {
        if c > MAX_COLUMN {
            return Err(Error::BadColumn(c));
        }
        if r > MAX_ROW {
            return Err(Error::BadRow(r));
        }
        let (rtype, fmt, payload) = {
            let s = self.slots.get(&src).ok_or(Error::BadRepeat(src))?;
            (s.rtype, s.fmt, s.payload.clone())
        };
        if rtype == FORMULA {
            self.formula(fmt, c, r, payload, true)
        } else {
            Ok(self.cell(rtype, fmt, c, r, payload))
        }
    }
    fn next(&mut self) -> Result<Record, Error> {
        match self.b.read(3)? {
            1 => {
                let (f, c, r) = self.addr()?;
                let v = self.small()? as u16;
                Ok(self.cell(INTEGER, f, c, r, v.to_le_bytes().to_vec()))
            }
            2 => {
                let (f, c, r) = self.addr()?;
                let v = self.constant()?;
                Ok(self.cell(NUMBER, f, c, r, v))
            }
            3 => {
                let (f, c, r) = self.addr()?;
                let v = self.label()?;
                // 0x38FE: a label over 0x14 bytes skips the repeat cache and the current position.
                if 5 + v.len() > 0x14 {
                    Ok(Self::record(LABEL, f, c, r, &v))
                } else {
                    Ok(self.cell(LABEL, f, c, r, v))
                }
            }
            4 => {
                let (f, c, r) = self.addr()?;
                let code = self.formula_code()?;
                self.formula(f, c, r, code, false)
            }
            5 => self.generic(),
            6 => {
                let (r, c) = (self.cur_row, self.cur_col);
                self.repeat(c, c + 1, r)
            }
            0 => {
                let (r, c) = (self.cur_row, self.cur_col);
                self.repeat(c + 1, c + 1, r)
            }
            _ => {
                let (lr, lc) = (self.cur_row, self.cur_col);
                let c = self.col()?;
                self.repeat(c, c, if c <= lc { lr + 1 } else { lr })
            }
        }
    }
    fn generic(&mut self) -> Result<Record, Error> {
        let t = self.b.read(8)? as u16;
        match t {
            BLANK => {
                let (f, c, r) = self.addr()?;
                Ok(self.cell(BLANK, f, c, r, Vec::new()))
            }
            0x25..=0x27 => {
                let mut s = self.string()?;
                s.resize(if t == 0x27 { 0x28 } else { 0xf2 }, 0);
                Ok(Record { rtype: t, data: s })
            }
            0x47 => {
                // 0x3AB3: a 16-byte name, then a block as two cell addresses and a flag.
                let mut data = vec![0; 0x19];
                for i in 0..2 {
                    let (c, r) = (self.col()? as u16, self.row()? as u16);
                    data[0x10 + 4 * i..0x12 + 4 * i].copy_from_slice(&c.to_le_bytes());
                    data[0x12 + 4 * i..0x14 + 4 * i].copy_from_slice(&r.to_le_bytes());
                }
                data[0x18] = self.b.read(1)? as u8;
                let s = self.string()?;
                let n = s.len().min(0x10);
                data[..n].copy_from_slice(&s[..n]);
                Ok(Record { rtype: t, data })
            }
            _ => {
                let n = self.small()? as usize;
                if n > RECORD_LIMIT {
                    return Err(Error::RecordTooLong(n));
                }
                let data = self.b.bytes(n)?;
                if matches!(t, INTEGER | NUMBER | LABEL | FORMULA) {
                    check_address(&data)?;
                }
                Ok(Record { rtype: t, data })
            }
        }
    }
}

/// How a file's formula results were stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueStorage {
    /// Stored in full.
    Exact,
    /// Rounded to a 23-bit mantissa and an 8-bit exponent.
    Approximate,
    /// Not stored; the spreadsheet must recalculate them.
    Removed,
}

/// The options in a file's header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileInfo {
    /// The two version digits, such as `"20"` for SQZ! Plus or `"15"` for SQZ!.
    pub version: String,
    /// Whether empty, formatted cells were dropped.
    pub blanks_removed: bool,
    /// How formula results are stored.
    pub values: ValueStorage,
    /// Whether the data is armoured as printable text.
    pub modem_armour: bool,
    /// Whether the file needs a password.
    pub password_protected: bool,
}

/// Reads a file's header without decoding it.
///
/// # Errors
///
/// Returns [`Error::UnsupportedHeader`] if this isn't a supported SQZ! file.
pub fn inspect(raw: &[u8]) -> Result<FileInfo, Error> {
    let digits = raw
        .get(1..3)
        .is_some_and(|v| v.iter().all(u8::is_ascii_digit));
    if raw.len() < 0x2e || raw[0] != b'Z' || !digits || (raw[1], raw[2]) > (b'2', b'1') {
        let head = raw.get(..13).unwrap_or(raw);
        return Err(Error::UnsupportedHeader(
            String::from_utf8_lossy(head).into(),
        ));
    }
    let values = match (raw[4] == b'Y', raw[7] != b'N') {
        (true, _) => ValueStorage::Removed,
        (false, true) => ValueStorage::Approximate,
        (false, false) => ValueStorage::Exact,
    };
    Ok(FileInfo {
        version: String::from_utf8_lossy(&raw[1..3]).into(),
        blanks_removed: raw[3] == b'Y',
        values,
        modem_armour: raw[5] != b'N',
        password_protected: raw[6] != b'N',
    })
}

/// The password as the driver holds it: upper-cased bytes (0x2B9A). One over 15 bytes is ignored.
fn normalize_password(password: &str) -> Vec<u8> {
    let b = password.as_bytes();
    if b.len() > 0xf {
        return Vec::new();
    }
    b.iter()
        .map(|&c| if c >= 0x61 { c.wrapping_sub(0x20) } else { c })
        .collect()
}

/// The value the header holds to check a password (0x2E3C).
fn password_key(password: &[u8]) -> u16 {
    let start = u16::from((password.len() as u8) << 4) * 0x100;
    password.iter().enumerate().fold(start, |key, (i, &c)| {
        key.wrapping_add(u16::from(c) * (i as u16 + 1))
    })
}

/// Decodes a `.WQ!` file into WK1-style records, ending with the end-of-file record.
///
/// ```no_run
/// let raw = std::fs::read("CRETE.WQ!")?;
/// let csv = quattro_sqz::to_csv(&quattro_sqz::decode(&raw)?);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// Returns an [`Error`] if the file is malformed or password protected.
pub fn decode(raw: &[u8]) -> Result<Vec<Record>, Error> {
    decode_with_password(raw, None)
}

/// Decodes a `.WQ!` file that may be password protected. Passwords ignore case.
///
/// # Errors
///
/// Returns an [`Error`] if the file is malformed or the password is missing or wrong.
pub fn decode_with_password(raw: &[u8], password: Option<&str>) -> Result<Vec<Record>, Error> {
    decode_limited(raw, password, OUTPUT_LIMIT)
}

fn decode_limited(raw: &[u8], password: Option<&str>, limit: usize) -> Result<Vec<Record>, Error> {
    let info = inspect(raw)?;
    let mut src = Source {
        raw,
        pos: 0,
        block_start: 0,
        block_size: 0,
        modem: info.modem_armour,
        groups: 0,
        password: Vec::new(),
        key: 0,
        pw_index: 0,
        rot16: 9,
        rot8: 5,
    };
    let mut expected = None;
    if info.password_protected {
        let pw = normalize_password(password.ok_or(Error::PasswordRequired)?);
        if pw.is_empty() {
            return Err(Error::WrongPassword);
        }
        src.key = password_key(&pw);
        expected = Some(src.key);
        src.password = pw;
    }
    // 0x3D7B: the password check and table offset, in a 12-byte read.
    src.enter(0x18, 0xc);
    let check = src.word()?;
    if expected.is_some_and(|key| key != check) {
        return Err(Error::WrongPassword);
    }
    let low = u32::from(src.word()?);
    let table_offset = (low | u32::from(src.word()?) << 16) as usize;
    src.enter(table_offset, 0x200);
    let table = read_table(&mut src)?;
    src.enter(0x2e, 0x200);
    let code = TEXT_CODE.iter().map(|&(l, c, s)| ((l, c), s)).collect();
    let mut d = Decoder {
        b: Bits {
            src,
            acc: 0,
            count: 0,
        },
        old_version: raw[1] < b'2',
        table,
        code,
        no_values: info.values == ValueStorage::Removed,
        raw_doubles: info.values != ValueStorage::Approximate,
        last_row: 0xfff0,
        last_col: 0xfff0,
        last_fmt: 0xfff0,
        cur_row: 0x7fff,
        cur_col: 0,
        slots: HashMap::new(),
    };
    let (mut out, mut total) = (Vec::new(), 0usize);
    loop {
        let r = d.next()?;
        total += r.data.len() + size_of::<Record>();
        if total > limit {
            return Err(Error::OutputTooLarge);
        }
        let done = r.rtype == EOF;
        out.push(r);
        if done {
            return Ok(out);
        }
    }
}

/// A cell record from tag 5 holds its address as raw bytes, so it needs the same limits.
fn check_address(data: &[u8]) -> Result<(), Error> {
    let [_, c0, c1, r0, r1, ..] = data else {
        return Err(Error::BadCell);
    };
    let (c, r) = (
        u16::from_le_bytes([*c0, *c1]),
        u16::from_le_bytes([*r0, *r1]),
    );
    if u32::from(c) > MAX_COLUMN {
        return Err(Error::BadColumn(c.into()));
    }
    if u32::from(r) > MAX_ROW {
        return Err(Error::BadRow(r.into()));
    }
    Ok(())
}

/// Reads the formula table: a count, then ids, lengths, and bytes (0x3DE1, 0x3E2E).
fn read_table(src: &mut Source<'_>) -> Result<HashMap<u32, Vec<u8>>, Error> {
    let count = u32::from(src.word()?);
    let mut table = HashMap::new();
    for _ in 0..count {
        let id = u32::from(src.word()?);
        let len = src.word()? as usize;
        if id >= count || len > 0x800 {
            return Err(Error::BadFormulaTable);
        }
        let body = (0..len)
            .map(|_| src.byte())
            .collect::<Result<Vec<u8>, Error>>()?;
        table.insert(id, body);
    }
    Ok(table)
}

/// Serializes records in the layout Quattro Pro reads as `.WQ1`.
///
/// Quattro's labels hold a length byte after the alignment prefix, which the decoded records lack.
#[must_use]
pub fn to_wq1(records: &[Record]) -> Vec<u8> {
    let mut out = Vec::new();
    for r in records {
        let data = if r.rtype == LABEL {
            wq1_label(&r.data)
        } else {
            r.data.clone()
        };
        out.extend(r.rtype.to_le_bytes());
        out.extend((data.len() as u16).to_le_bytes());
        out.extend(&data);
    }
    out
}

fn wq1_label(data: &[u8]) -> Vec<u8> {
    if data.len() < 5 {
        return data.to_vec();
    }
    let (address, body) = data.split_at(5);
    let (prefix, rest) = match body.first() {
        Some(p @ (b'\\' | b'\'' | b'"' | b'^')) => (*p, &body[1..]),
        _ => (b'\'', body),
    };
    let end = rest.iter().position(|&b| b == 0).unwrap_or(rest.len());
    let text = &rest[..end.min(255)];
    let mut out = address.to_vec();
    out.extend([prefix, text.len() as u8]);
    out.extend(text);
    out
}

/// Options for [`to_csv_with`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct CsvOptions {
    /// Leaves formula cells empty, for a file whose results were removed.
    pub omit_results: bool,
    /// Writes label text exactly, without protecting spreadsheet programs from it. See [`to_csv`].
    pub exact_labels: bool,
}

/// Renders the cells as CSV: label text without its alignment prefix, a formula's stored
/// result, and `ERR` for a stored `ERR` or `NA`.
///
/// A spreadsheet program may run a CSV cell that looks like a formula, and labels come from the
/// file, so a label that could be one gets a leading `'`. Lines made of rule characters, such as
/// `-----`, stay as they are. Use [`to_csv_with`] and [`CsvOptions::exact_labels`] to turn this off.
#[must_use]
pub fn to_csv(records: &[Record]) -> String {
    to_csv_with(records, CsvOptions::default())
}

/// Whether a spreadsheet program could read this text as a formula.
fn looks_like_formula(s: &str) -> bool {
    let is_rule = s
        .chars()
        .all(|c| matches!(c, '-' | '+' | '=' | '|' | '_' | '*' | ' '));
    !is_rule && s.starts_with(['=', '+', '-', '@', '\t', '\r'])
}

/// Renders the cells as CSV, with options.
#[must_use]
pub fn to_csv_with(records: &[Record], options: CsvOptions) -> String {
    let mut grid: Vec<Vec<String>> = Vec::new();
    for r in records {
        let (Some((_, c, row)), Some(v)) = (r.address(), r.value()) else {
            continue;
        };
        let text = match v {
            _ if options.omit_results && r.rtype == FORMULA => String::new(),
            Value::Blank => String::new(),
            Value::Integer(i) => i.to_string(),
            Value::Number(x) => x.to_string(),
            Value::Label { text, .. } if !options.exact_labels && looks_like_formula(&text) => {
                format!("'{text}")
            }
            Value::Label { text, .. } => text,
            Value::Special => "ERR".to_string(),
        };
        if u32::from(c) > MAX_COLUMN || u32::from(row) > MAX_ROW {
            continue;
        }
        let (c, row) = (c as usize, row as usize);
        if grid.len() <= row {
            grid.resize(row + 1, Vec::new());
        }
        if grid[row].len() <= c {
            grid[row].resize(c + 1, String::new());
        }
        grid[row][c] = text;
    }
    let width = grid.iter().map(Vec::len).max().unwrap_or(0);
    let mut out = String::new();
    for row in &grid {
        let fields: Vec<String> = (0..width)
            .map(|i| {
                let s = row.get(i).map_or("", String::as_str);
                if s.contains([',', '"', '\n', '\r']) {
                    format!("\"{}\"", s.replace('"', "\"\""))
                } else {
                    s.to_string()
                }
            })
            .collect();
        out.push_str(&fields.join(","));
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn code() -> HashMap<(u8, u16), u8> {
        TEXT_CODE.iter().map(|&(l, c, s)| ((l, c), s)).collect()
    }

    /// Packs `0`/`1` text into bytes, returning them and the bit count.
    fn pack(bits: &str) -> (Vec<u8>, usize) {
        let digits: Vec<u8> = bits
            .bytes()
            .filter(|b| *b != b' ')
            .map(|b| b - b'0')
            .collect();
        let bytes = digits
            .chunks(8)
            .map(|c| c.iter().enumerate().fold(0, |v, (i, b)| v | (b << (7 - i))));
        (bytes.collect(), digits.len())
    }

    fn text(bits: &str) -> String {
        let (buf, n) = pack(bits);
        String::from_utf8(decode_text(&buf, n, &code()).unwrap()).unwrap()
    }

    #[test]
    fn text_code_is_a_complete_prefix_code() {
        let kraft: f64 = TEXT_CODE
            .iter()
            .map(|&(l, ..)| (-f64::from(l)).exp2())
            .sum();
        assert!((kraft - 1.0).abs() < f64::EPSILON);
        assert_eq!(TEXT_CODE.len(), 72);
    }

    #[test]
    fn case_modes_change_letters() {
        // "e", space, "e" with e = 1100 and space = 111.
        assert_eq!(text("001 1100 111 1100"), "e e");
        assert_eq!(text("010 1100 111 1100"), "E E");
        assert_eq!(text("011 1100 111 1100"), "E e");
        assert_eq!(text("100 1100 111 1100"), "E E");
        assert_eq!(text("101 1100 1 111 1100 0"), "E e");
    }

    #[test]
    fn run_length_counts_one_less_and_wraps() {
        assert_eq!(run_length(5), 4);
        assert_eq!(run_length(1), 256);
        assert_eq!(run_length(0), 255);
    }

    #[test]
    fn passwords_are_upper_cased_and_limited() {
        assert_eq!(normalize_password("aB1"), b"AB1");
        assert_eq!(normalize_password("sixteen-letters!"), Vec::<u8>::new());
    }

    #[test]
    fn password_key_weights_each_byte_by_position() {
        assert_eq!(password_key(b"AB"), 0x2000 + 0x41 + 2 * 0x42);
    }

    #[test]
    fn malformed_cell_payloads_have_no_value() {
        let short = |rtype| Record {
            rtype,
            data: vec![0; 5],
        };
        assert_eq!(short(INTEGER).value(), None);
        assert_eq!(short(NUMBER).value(), None);
        assert_eq!(short(FORMULA).value(), None);
        assert_eq!(short(BLANK).value(), Some(Value::Blank));
        assert_eq!(
            Record {
                rtype: LABEL,
                data: vec![0; 2]
            }
            .address(),
            None
        );
    }

    #[test]
    fn non_finite_results_are_special() {
        let record = |v: f64| {
            let mut data = vec![0; 5];
            data.extend(v.to_le_bytes());
            Record {
                rtype: NUMBER,
                data,
            }
        };
        assert_eq!(record(1.5).value(), Some(Value::Number(1.5)));
        assert_eq!(record(f64::NAN).value(), Some(Value::Special));
        assert_eq!(record(f64::NEG_INFINITY).value(), Some(Value::Special));
    }

    #[test]
    fn csv_quotes_fields_that_need_it() {
        let label = |c: u16, s: &str| {
            let mut data = vec![0xff];
            data.extend(c.to_le_bytes());
            data.extend(0u16.to_le_bytes());
            data.extend(s.bytes());
            Record { rtype: LABEL, data }
        };
        let csv = to_csv(&[label(0, "'a,b"), label(1, "'say \"hi\"")]);
        assert_eq!(csv, "\"a,b\",\"say \"\"hi\"\"\"\n");
    }

    fn label_record(s: &str) -> Record {
        let mut data = vec![0xff, 0, 0, 0, 0];
        data.extend(s.bytes());
        Record { rtype: LABEL, data }
    }

    #[test]
    fn csv_protects_labels_that_could_run_as_formulas() {
        let csv = |s: &str| to_csv(&[label_record(s)]);
        for s in ["'=HYPERLINK(1)", "'@SUM(A1)", "'+1+1", "'-1+1"] {
            assert_eq!(csv(s), format!("'{}\n", &s[1..]));
        }
        for s in ["'-----", "'--------|", "'+---------", "'=====", "'plain"] {
            assert_eq!(csv(s), format!("{}\n", &s[1..]));
        }
        let exact = CsvOptions {
            exact_labels: true,
            ..CsvOptions::default()
        };
        assert_eq!(to_csv_with(&[label_record("'=1+1")], exact), "=1+1\n");
    }

    #[test]
    fn csv_keeps_negative_numbers() {
        let data = [&[0xff, 0, 0, 0, 0][..], &(-5i16).to_le_bytes()].concat();
        assert_eq!(
            to_csv(&[Record {
                rtype: INTEGER,
                data
            }]),
            "-5\n"
        );
    }

    #[test]
    fn csv_ignores_addresses_outside_a_worksheet() {
        let data = [&[0xff, 0xff, 0xff, 0xff, 0xff][..], &[1, 0]].concat();
        assert_eq!(
            to_csv(&[Record {
                rtype: INTEGER,
                data
            }]),
            ""
        );
    }

    /// One integer cell, then many repeats of it at very little cost in the file.
    #[test]
    fn output_limit_stops_expansion() {
        let mut bits = String::from("001 10 00 1 0000000000000 0 0000001");
        bits.push_str(&" 111 00".repeat(3000));
        let (stream, _) = pack(&bits);
        let mut raw = vec![0u8; 0x2e];
        raw[..3].copy_from_slice(b"Z20");
        raw[3..8].fill(b'N');
        let table = u32::try_from(0x2e + stream.len()).unwrap();
        raw[0x1a..0x1e].copy_from_slice(&table.to_le_bytes());
        raw.extend(&stream);
        raw.extend([0, 0]);
        assert_eq!(decode_limited(&raw, None, 1000), Err(Error::OutputTooLarge));
    }

    #[test]
    fn wq1_labels_get_a_length_byte() {
        let data = [&[0xff, 0, 0, 0, 0][..], b"^abc\0"].concat();
        let out = to_wq1(&[Record { rtype: LABEL, data }]);
        assert_eq!(&out[9..], b"^\x03abc");
    }

    #[test]
    fn cp437_maps_high_bytes() {
        assert_eq!(cp437(&[b'a', 0x80, 0xe1, 0xff]), "aÇß\u{a0}");
    }
}
