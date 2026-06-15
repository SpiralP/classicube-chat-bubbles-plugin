use classicube_sys::{Convert_CP437ToUnicode, Convert_CodepointToCP437};

use super::{BARS, CORNER, DOT};

#[test]
fn icon_glyphs_round_trip_through_cp437() {
    for g in [DOT, CORNER, BARS] {
        let byte = Convert_CodepointToCP437(g as _);
        assert_ne!(byte, b'?', "glyph not representable in CP437");
        assert_eq!(u32::from(Convert_CP437ToUnicode(byte)), g as u32);
    }
}
