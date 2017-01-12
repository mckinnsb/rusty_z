use std::io::*;

pub trait LineReader {
    fn read_next_line(&self, buf: &mut String ) -> Result<usize>;
}

impl LineReader for Stdin {
    fn read_next_line(&self, buf: &mut String ) -> Result<usize> {
        self.read_line(buf)
    }
}

pub struct InputHandler<T: LineReader> {
    pub reader: T,
}

impl<T: LineReader> InputHandler<T> {
    pub fn get_input(&mut self) -> Option<String> {

        // 64 characters is probably a pretty reasonable start
        let mut input = String::with_capacity(64);
        let result = self.reader.read_next_line(&mut input);

        let length = match result {
            Ok(x) => x,
            // we ignore the error here, for now
            // im guessing we might need to panic in the future
            Err(e) => 0,
        };

        // no input, so return None
        if length == 0 {
            return None;
        };

        // see if there's a '\n' at the end of the line
        // according to read_line, we should be guaranteed an EOF
        // or a \n - but i'm not sure if that's part of read_line or not

        // i also believe this MIGHT not work cross-OS. but
        let end_of_input = input.ends_with('\n');

        if !end_of_input {
            return None;
        };

        Some(input)

    }
}
