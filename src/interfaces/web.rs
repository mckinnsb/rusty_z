use super::zinterface::ZInterface;

pub struct WebInterface {}

impl ZInterface for WebInterface {
    fn clear(&self) {}

    fn print_to_main(&self, str: &str) {}

    fn print_to_header(&self, left_side: &str, right_side: &str) {}

    fn read_next_line(&self, buf: &mut String) -> Option<usize> {
        None
    }

    fn quit(&self) {}

    fn set_loop(&self) {}

    fn setup_logging(&self) {}
}
