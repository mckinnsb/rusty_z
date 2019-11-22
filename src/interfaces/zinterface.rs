pub trait ZInterface {
   fn quit();
   fn read_next_line(buf: &mut String) -> Option<usize>;
   fn print_to_main(str: &str);
   fn print_to_header(str: &str);
}

