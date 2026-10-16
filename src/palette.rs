//! The mIRC colour palette.

/// mIRC colours 0-98 (https://modern.ircdocs.horse/formatting#colors).
pub const MIRC_RGB: [u32; 99] = [
    0xffffff, 0x000000, 0x00007f, 0x009300, 0xff0000, 0x7f0000, 0x9c009c, 0xfc7f00, //
    0xffff00, 0x00fc00, 0x009393, 0x00ffff, 0x0000fc, 0xff00ff, 0x7f7f7f, 0xd2d2d2, //
    0x470000, 0x472100, 0x474700, 0x324700, 0x004700, 0x00472c, 0x004747, 0x002747, //
    0x000047, 0x2e0047, 0x470047, 0x47002a, 0x740000, 0x743a00, 0x747400, 0x517400, //
    0x007400, 0x007449, 0x007474, 0x004074, 0x000074, 0x4b0074, 0x740074, 0x740045, //
    0xb50000, 0xb56300, 0xb5b500, 0x7db500, 0x00b500, 0x00b571, 0x00b5b5, 0x0063b5, //
    0x0000b5, 0x7500b5, 0xb500b5, 0xb5006b, 0xff0000, 0xff8c00, 0xffff00, 0xb2ff00, //
    0x00ff00, 0x00ffa0, 0x00ffff, 0x008cff, 0x0000ff, 0xa500ff, 0xff00ff, 0xff0098, //
    0xff5959, 0xffb459, 0xffff71, 0xcfff60, 0x6fff6f, 0x65ffc9, 0x6dffff, 0x59b4ff, //
    0x5959ff, 0xc459ff, 0xff66ff, 0xff59bc, 0xff9c9c, 0xffd39c, 0xffff9c, 0xe2ff9c, //
    0x9cff9c, 0x9cffdb, 0x9cffff, 0x9cd3ff, 0x9c9cff, 0xdc9cff, 0xff9cff, 0xff94d3, //
    0x000000, 0x131313, 0x282828, 0x363636, 0x4d4d4d, 0x656565, 0x818181, 0x9f9f9f, //
    0xbcbcbc, 0xe2e2e2, 0xffffff,
];

/// mIRC's first sixteen colours, by the names
/// https://modern.ircdocs.horse/formatting#colors gives them.
pub mod name {
    pub const WHITE: u8 = 0;
    pub const BLACK: u8 = 1;
    pub const BLUE: u8 = 2;
    pub const GREEN: u8 = 3;
    pub const RED: u8 = 4;
    pub const BROWN: u8 = 5;
    pub const MAGENTA: u8 = 6;
    pub const ORANGE: u8 = 7;
    pub const YELLOW: u8 = 8;
    pub const LIGHT_GREEN: u8 = 9;
    pub const CYAN: u8 = 10;
    pub const LIGHT_CYAN: u8 = 11;
    pub const LIGHT_BLUE: u8 = 12;
    pub const PINK: u8 = 13;
    pub const GREY: u8 = 14;
    pub const LIGHT_GREY: u8 = 15;
}

use name::*;

/// The eight SGR colours (black, red, green, yellow, blue, magenta, cyan,
/// white) as the PC drew them, each as the mIRC colour of the same name and
/// shade. The PC's dark red (#aa0000) is mIRC's brown (#7f0000), its dark
/// yellow is the brown #aa5500 that mIRC calls orange, and its dark white is
/// light grey.
pub const ANSI_NORMAL: [u8; 8] = [BLACK, BROWN, GREEN, ORANGE, BLUE, MAGENTA, CYAN, LIGHT_GREY];

/// The bright eight: dark grey, and the light shade of each colour.
pub const ANSI_BRIGHT: [u8; 8] = [GREY, RED, LIGHT_GREEN, YELLOW, LIGHT_BLUE, PINK, LIGHT_CYAN, WHITE];

fn channels(rgb: u32) -> (i32, i32, i32) {
    (
        ((rgb >> 16) & 255) as i32,
        ((rgb >> 8) & 255) as i32,
        (rgb & 255) as i32,
    )
}

/// The mIRC colour closest to an RGB value.
pub fn nearest(rgb: u32) -> u8 {
    let (r, g, b) = channels(rgb);
    let mut best = (1u8, i32::MAX);
    for (i, &c) in MIRC_RGB.iter().enumerate() {
        let (cr, cg, cb) = channels(c);
        let d = (cr - r).pow(2) + (cg - g).pow(2) + (cb - b).pow(2);
        if d < best.1 {
            best = (i as u8, d);
        }
    }
    best.0
}

/// How far apart two mIRC colours look, as RGB distance.
pub fn distance(a: u8, b: u8) -> f64 {
    let (ar, ag, ab) = channels(MIRC_RGB[a as usize]);
    let (br, bg, bb) = channels(MIRC_RGB[b as usize]);
    (((ar - br).pow(2) + (ag - bg).pow(2) + (ab - bb).pow(2)) as f64).sqrt()
}

/// An xterm 256-colour index as RGB.
pub fn xterm_rgb(n: u8) -> u32 {
    if n < 16 {
        let table = if n >= 8 { &ANSI_BRIGHT } else { &ANSI_NORMAL };
        return MIRC_RGB[table[(n % 8) as usize] as usize];
    }
    if n < 232 {
        let n = (n - 16) as u32;
        let levels = [0u32, 95, 135, 175, 215, 255];
        return (levels[(n / 36) as usize] << 16) | (levels[((n / 6) % 6) as usize] << 8) | levels[(n % 6) as usize];
    }
    let g = 8 + 10 * (n as u32 - 232);
    (g << 16) | (g << 8) | g
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The PC's sixteen colours.
    const PC: [u32; 16] = [
        0x000000, 0xaa0000, 0x00aa00, 0xaa5500, 0x0000aa, 0xaa00aa, 0x00aaaa, 0xaaaaaa, //
        0x555555, 0xff5555, 0x55ff55, 0xffff55, 0x5555ff, 0xff55ff, 0x55ffff, 0xffffff,
    ];

    #[test]
    fn each_dark_colour_is_its_nearest_of_the_sixteen() {
        for (i, &rgb) in PC[..8].iter().enumerate() {
            let (r, g, b) = channels(rgb);
            let nearest = (0..16u8)
                .min_by_key(|&c| {
                    let (cr, cg, cb) = channels(MIRC_RGB[c as usize]);
                    (cr - r).pow(2) + (cg - g).pow(2) + (cb - b).pow(2)
                })
                .unwrap();
            assert_eq!(ANSI_NORMAL[i], nearest, "colour {i}");
        }
    }

    #[test]
    fn bright_and_dark_are_different_colours() {
        for i in 0..8 {
            assert_ne!(ANSI_NORMAL[i], ANSI_BRIGHT[i]);
        }
        let mut all: Vec<u8> = ANSI_NORMAL.iter().chain(&ANSI_BRIGHT).copied().collect();
        all.sort();
        assert_eq!(all, (0..16).collect::<Vec<u8>>());
    }
}
