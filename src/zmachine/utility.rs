pub struct MemoryTools;

impl MemoryTools {
    // it takes up to a u32 because the largest story file allowed
    // is 512kbytes, and u32 is the lowest integer that can represent
    // 512,000 locations in memory is u32
    //
    // it should be noted that this is actually  2^16 * some multiplier,
    // depending on the version;
    //
    // 1-3: 2, or a max size of 128k
    // 4-5: 4, or a max size of 256k
    // 6-8: 8, or a max size of 512k

    // i don't really care about the integer size; you are on your own
    // buddy ( regarding overflows )
    //
    // i could make struct/tuple struct that maybe verifies that the
    // value is below some multiple of 2^16, but that feels like overkill
    // for now

    pub fn get_u16_at_position(buf: &[u8], index: usize) -> u16 {
        let x: u16 = (buf[index] as u16) << 8 | (buf[index + 1] as u16);
        x
    }
}
