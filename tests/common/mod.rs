//! Shared helpers for comparing decoded cells with Quattro Pro's output.
use quattro_sqz::Record;
use std::collections::BTreeMap;
use std::path::Path;

pub(crate) type Cell = (u16, u8, Vec<u8>);

/// Reads Quattro's own WQ1 records, dropping the length byte it adds to labels.
pub(crate) fn quattro_cells(path: &Path) -> BTreeMap<(u16, u16), Cell> {
    let b = std::fs::read(path).unwrap();
    let (mut p, mut out) = (0, BTreeMap::new());
    while p + 4 <= b.len() {
        let t = u16::from_le_bytes([b[p], b[p + 1]]);
        let n = u16::from_le_bytes([b[p + 2], b[p + 3]]) as usize;
        let d = &b[p + 4..p + 4 + n];
        p += 4 + n;
        if (0x0c..=0x10).contains(&t) {
            let mut body = d[5..].to_vec();
            if t == 0x0f {
                while body.last() == Some(&0) {
                    body.pop();
                }
                if body.len() >= 2 && body[1] as usize == body.len() - 2 {
                    body.remove(1);
                }
            }
            let (c, r) = (
                u16::from_le_bytes([d[1], d[2]]),
                u16::from_le_bytes([d[3], d[4]]),
            );
            out.insert((c, r), (t, d[0], body));
        }
        if t == 1 {
            break;
        }
    }
    out
}

pub(crate) fn decoded_cells(records: &[Record]) -> BTreeMap<(u16, u16), Cell> {
    let mut out = BTreeMap::new();
    for r in records {
        if let Some((f, c, row)) = r.address() {
            let mut body = r.data[5..].to_vec();
            if r.rtype == 0x0f {
                while body.last() == Some(&0) {
                    body.pop();
                }
            }
            out.insert((c, row), (r.rtype, f, body));
        }
    }
    out
}
