use std::fmt;
use super::memory_view::*;

// ah, the ZString!
//
// such a wonder of nature!.
//
// it's a comppressed string. basically each zstring is encoded
// as a set of pair of bytes ( or 16-bit words ), such that
// 3 characters use 5 bits each, and the remaining bit indicates
// when the line ends.
//
// there are some special encoding rules too; zstrings can
// encode abbreviations (for further compression), menu clicks,
// single clicks and double clicks! ( i don't think this is used output wise, however )


pub enum Alphabet {
    A0,
    A1,
    A2,
}

impl Alphabet {
    fn next_alphabet(&self, shift_char: u8) -> Alphabet {
        match (self, shift_char) {
            (&Alphabet::A0, 4) => Alphabet::A1,
            (&Alphabet::A0, 5) => Alphabet::A2,
            (&Alphabet::A1, 4) => Alphabet::A2,
            (&Alphabet::A1, 5) => Alphabet::A0,
            (&Alphabet::A2, 4) => Alphabet::A0,
            (&Alphabet::A2, 5) => Alphabet::A1,
            _ => panic!("non-shift character given to next alphabet!"),
        }
    }
}

enum BigChar {
    None,
    Building,
    Partial { upper: u8 }, /* note these are really limited to 5 bits, so the total representation
                            * of a big char is 1024 */
}

enum Abbreviation {
    None,
    Partial { z: u8 },
}

pub struct ZString {
    // the "array" of ZSCII codes
    bytes: Vec<u8>,
    // the string itself
    string: String,
    // how long was this string in bytes
    pub encoded_length: u32,
}

impl ZString {
    // create a zstring, located at offset from memory view's pointer
    pub fn create(offset: u32, view: &MemoryView, abbreviations_view: &MemoryView) -> ZString {

        // we legitimately don't know the length of this
        let mut bytes = Vec::new();

        // implicit copy
        let mut pointer = offset;

        //println!("pointer: {:x}", view.pointer + pointer);

        let mask: u16 = 0x8000;

        loop {
            let byte: u16 = view.read_u16_at_head(pointer);
            bytes.push(byte);

            if mask & byte > 0 {
                break;
            }

            pointer += 2;
        }

        let mut z_string = ZString {
            //each byte has 3 z-chars, which corresponds to 3 chars (even with
            //special chars, rust treats chars more like runes ).
            
            bytes: Vec::with_capacity(bytes.len() * 3),
            string: String::with_capacity(bytes.len() * 3),
            encoded_length: (bytes.len() as u32) * 2,
        };

        for word in bytes {
            z_string.bytes.push(((word >> 10) as u8) & 0x1F);
            z_string.bytes.push(((word >> 5) as u8) & 0x1F);
            z_string.bytes.push((word as u8) & 0x1F);
        }

        //println!("num of chars: {}", z_string.bytes.len());

        // we always start out with A0
        // this can shift for one character only, so we have to keep track of it
        let mut alphabet = Alphabet::A0;
        // this we also have to keep track of as its multi-char
        let mut printing_big_char = BigChar::None;
        // this we also have to keep track of as its multi-char (but is an abbreviation)
        let mut printing_abbreviation = Abbreviation::None;

        for ch in z_string.bytes.iter() {

            // copy the byte, its fine. i don't care. ill probably end up doing it anyway.
            //
            // so this is a crazy match, but it honestly shows how rust sort of flattens
            // if-then madness in to more sensible cases that can persist state
            // over an iteration - perfect for decoding, as it turns out
            match (*ch, &alphabet, &printing_big_char, &printing_abbreviation) {

                //if six and in alphabet A2, start a bigchar, but only if
                //we are not starting an abbreviation
                (6, &Alphabet::A2, &BigChar::None, &Abbreviation::None) => printing_big_char = BigChar::Building,

                //if building bigchar, this char is upper half of bigchar
                (upper, _, &BigChar::Building, _) => {
                    printing_big_char = BigChar::Partial { upper: upper }
                }

                //if partially building bigchar, this char is lower half of bigchar,
                //and finish building the big char
                (lower, _, &BigChar::Partial { upper }, _) => {
                    let big_char = ((upper as u16) << 5 & 0b0000001111100000) |
                                   ((lower as u16) & 0b0000000000011111);
                    z_string.string.push(ZString::decode_big_char(big_char));
                    printing_big_char = BigChar::None;
                    alphabet = Alphabet::A0;
                }

                //if we have a character between 1 and 3, start building an abbreviation
                //if we aren't already.
                (z @ 1...3, _, _, &Abbreviation::None) => printing_abbreviation = Abbreviation::Partial { z: z },

                //if we are building an abbreviation, the next # is the index
                //find the abbreviation and print
                (i, _, _, &Abbreviation::Partial { z }) => {
                    let string = format!("{}",
                                         ZString::find_abbreviation(i, z, abbreviations_view));
                    z_string.string.push_str(&string);
                    printing_abbreviation = Abbreviation::None;
                    //im not sure if you are supposed to do anything w/alphabet here
                    alphabet = Alphabet::A0;
                }

                //if we have a character between 4 and 5, switch the alphabet
                //(for the next char only) if we aren't building an abbreviation
                (z @ 4...5, _, _, &Abbreviation::None) => alphabet = alphabet.next_alphabet(z),

                //the default case, actually print the string
                (x, _, _, _) => {
                    z_string.string.push(ZString::decode_char(x, &alphabet));
                    alphabet = Alphabet::A0;
                }
            }

        }

        z_string

    }

    // chars are runes in rust, sortof, kind of
    pub fn decode_char(ch: u8, alphabet: &Alphabet) -> char {
        match (ch, alphabet) {
            (0x6, &Alphabet::A0) => 'a',
            (0x7, &Alphabet::A0) => 'b',
            (0x8, &Alphabet::A0) => 'c',
            (0x9, &Alphabet::A0) => 'd',
            (0xA, &Alphabet::A0) => 'e',
            (0xB, &Alphabet::A0) => 'f',
            (0xC, &Alphabet::A0) => 'g',
            (0xD, &Alphabet::A0) => 'h',
            (0xE, &Alphabet::A0) => 'i',
            (0xF, &Alphabet::A0) => 'j',
            (0x10, &Alphabet::A0) => 'k',
            (0x11, &Alphabet::A0) => 'l',
            (0x12, &Alphabet::A0) => 'm',
            (0x13, &Alphabet::A0) => 'n',
            (0x14, &Alphabet::A0) => 'o',
            (0x15, &Alphabet::A0) => 'p',
            (0x16, &Alphabet::A0) => 'q',
            (0x17, &Alphabet::A0) => 'r',
            (0x18, &Alphabet::A0) => 's',
            (0x19, &Alphabet::A0) => 't',
            (0x1A, &Alphabet::A0) => 'u',
            (0x1B, &Alphabet::A0) => 'v',
            (0x1C, &Alphabet::A0) => 'w',
            (0x1D, &Alphabet::A0) => 'x',
            (0x1E, &Alphabet::A0) => 'y',
            (0x1F, &Alphabet::A0) => 'z',
            (0x6, &Alphabet::A1) => 'A',
            (0x7, &Alphabet::A1) => 'B',
            (0x8, &Alphabet::A1) => 'C',
            (0x9, &Alphabet::A1) => 'D',
            (0xA, &Alphabet::A1) => 'E',
            (0xB, &Alphabet::A1) => 'F',
            (0xC, &Alphabet::A1) => 'G',
            (0xD, &Alphabet::A1) => 'H',
            (0xE, &Alphabet::A1) => 'I',
            (0xF, &Alphabet::A1) => 'J',
            (0x10, &Alphabet::A1) => 'K',
            (0x11, &Alphabet::A1) => 'L',
            (0x12, &Alphabet::A1) => 'M',
            (0x13, &Alphabet::A1) => 'N',
            (0x14, &Alphabet::A1) => 'O',
            (0x15, &Alphabet::A1) => 'P',
            (0x16, &Alphabet::A1) => 'Q',
            (0x17, &Alphabet::A1) => 'R',
            (0x18, &Alphabet::A1) => 'S',
            (0x19, &Alphabet::A1) => 'T',
            (0x1A, &Alphabet::A1) => 'U',
            (0x1B, &Alphabet::A1) => 'V',
            (0x1C, &Alphabet::A1) => 'W',
            (0x1D, &Alphabet::A1) => 'X',
            (0x1E, &Alphabet::A1) => 'Y',
            (0x1F, &Alphabet::A1) => 'Z',
            (0x7, &Alphabet::A2) => '^',
            (0x8, &Alphabet::A2) => '0',
            (0x9, &Alphabet::A2) => '1',
            (0xA, &Alphabet::A2) => '2',
            (0xB, &Alphabet::A2) => '3',
            (0xC, &Alphabet::A2) => '4',
            (0xD, &Alphabet::A2) => '5',
            (0xE, &Alphabet::A2) => '6',
            (0xF, &Alphabet::A2) => '7',
            (0x10, &Alphabet::A2) => '8',
            (0x11, &Alphabet::A2) => '9',
            (0x12, &Alphabet::A2) => '.',
            (0x13, &Alphabet::A2) => ',',
            (0x14, &Alphabet::A2) => '!',
            (0x15, &Alphabet::A2) => '?',
            (0x16, &Alphabet::A2) => '_',
            (0x17, &Alphabet::A2) => '#',
            (0x18, &Alphabet::A2) => '\'',
            (0x19, &Alphabet::A2) => '"',
            (0x1A, &Alphabet::A2) => '/',
            (0x1B, &Alphabet::A2) => '\\',
            (0x1C, &Alphabet::A2) => '-',
            (0x1D, &Alphabet::A2) => ':',
            (0x1E, &Alphabet::A2) => '(',
            (0x1F, &Alphabet::A2) => ')',
            (0, _) => ' ',
            _ => panic!("could not match character : {}", ch),
        }
    }

    // these are all unicode characters...
    // not sure if i want to do this yet....
    //
    // its a lot of entries.. a WHOLLLLE lot
    pub fn decode_big_char(ch: u16) -> char {
        '?'
    }

    pub fn find_abbreviation(i: u8, z: u8, view: &MemoryView) -> ZString {

        
        let address_offset = (32 * (z - 1) + i) * 2;
        let packed_address = view.read_u16_at_head(address_offset as u32);
        let address = (packed_address as u32 * 2);

        let mut new_view = view.clone();
        new_view.pointer = address;

        // we should be ok to feed this back in,
        // note that technically abbreviations cant abbreviate, but
        // it would be fine if they did, anyway ( i'm certainly not preventing it,
        // and can't see any reason why it would fail )
        
        ZString::create(0, &new_view, view)

    }
}

impl fmt::Display for ZString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.string)
    }
}
