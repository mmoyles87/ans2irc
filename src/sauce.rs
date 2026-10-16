//! SAUCE records: the metadata block at the end of most ANSI files.

pub struct Sauce {
    pub width: Option<usize>,
    pub ice: Option<bool>,
}

/// Split a file into its drawing and what its SAUCE record says, if it has
/// one. Comments and everything after the DOS end-of-file byte are dropped.
pub fn split(data: &[u8]) -> (&[u8], Sauce) {
    let mut sauce = Sauce { width: None, ice: None };
    let mut body = data;
    if data.len() >= 128 && &data[data.len() - 128..data.len() - 123] == b"SAUCE" {
        let rec = &data[data.len() - 128..];
        let comments = rec[104] as usize;
        if rec[94] == 1 {
            // character data
            let w = rec[96] as usize | (rec[97] as usize) << 8;
            sauce.width = if w == 0 { None } else { Some(w) };
            sauce.ice = Some(rec[105] & 1 == 1);
        }
        body = &data[..data.len() - 128];
        let block = 5 + 64 * comments;
        if comments > 0 && body.len() >= block && &body[body.len() - block..body.len() - block + 5] == b"COMNT" {
            body = &body[..body.len() - block];
        }
    }
    if let Some(eof) = body.iter().position(|&b| b == 0x1a) {
        body = &body[..eof];
    }
    (body, sauce)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(width: u16, ice: bool) -> Vec<u8> {
        let mut rec = vec![0u8; 128];
        rec[..5].copy_from_slice(b"SAUCE");
        rec[94] = 1;
        rec[95] = 1;
        rec[96..98].copy_from_slice(&width.to_le_bytes());
        rec[105] = ice as u8;
        rec
    }

    #[test]
    fn reads_width_and_ice() {
        let mut data = b"art".to_vec();
        data.push(0x1a);
        data.extend(record(132, true));
        let (body, sauce) = split(&data);
        assert_eq!(body, b"art");
        assert_eq!(sauce.width, Some(132));
        assert_eq!(sauce.ice, Some(true));
    }

    #[test]
    fn no_record() {
        let (body, sauce) = split(b"plain\x1ajunk");
        assert_eq!(body, b"plain");
        assert!(sauce.width.is_none() && sauce.ice.is_none());
    }
}
