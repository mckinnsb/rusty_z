pub trait ZInterface : Sized {
    fn quit(&self);
    fn clear(&self);
    fn read_next_line(&self, buf: &mut String) -> Option<usize>;
    fn print_to_main(&self, &str);

    // It's interesting, the ZMachine actually has this concept embedded in the opcodes;
    // show_status points to two objects that each must be displayed on the top left
    // and top right.
    //
    // This is interesting be cause it ALSO has opcodes for creating split/secondary
    // screens, and doesn't use this for that purpose; it's possible implementation
    // wasn't ready for Zork I/II/III (but other version 3 games make heavy use of it)
    fn print_to_header(&self, left_side: &str, right_side: &str);
    fn setup_logging(&self);
}

