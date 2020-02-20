extern crate log;
extern crate log4rs;
extern crate termion;

use std::process;

use self::log::LogLevelFilter;
use self::log4rs::append::file::*;
use self::log4rs::config::{Appender, Config, Logger, Root};
use self::termion::{clear, color, cursor, style};

use super::zinterface::*;

pub struct CliInterface {}

impl ZInterface for CliInterface {
    fn clear(&self) {
        println!("{}", clear::All);
    }

    fn print_to_main(&self, str: &str) {
        print!("{}", str);
    }

    fn print_to_header(&self, left_side: &str, right_side: &str) {
        //terminals start at 1,1 so, keep that in mind
        //this could panic... but if it can't get the terminal size,
        //there's a good reason to
        let (x, y) = termion::terminal_size().unwrap();

        let top_left = cursor::Goto(1, 1);
        //padding is 4 chars
        let margin_padding = "    ";
        let center_size = (x as usize) - left_side.len() - right_side.len() - 4 * 2;
        let center_padding: String = (0..center_size).into_iter().map(|_| " ").collect();
        let bottom = cursor::Goto(2, y);

        let header = format!(
            "{}{}{}{}{}{}{}{}{}{}{}{}",
            cursor::Goto(1, 1),
            color::Bg(color::LightWhite),
            color::Fg(color::Black),
            clear::CurrentLine,
            top_left,
            margin_padding,
            left_side,
            center_padding,
            right_side,
            margin_padding,
            style::Reset,
            bottom
        );

        print!("{}", header);
    }

    fn read_next_line(&self, buf: &mut String) -> Option<usize> {
        match std::io::stdin().read_line(buf) {
            Ok(x) => Some(x),
            //discard the error
            Err(_) => None,
        }
    }

    fn quit(&self) {
        process::exit(0);
    }

    fn setup_logging(&self) {
        let logger = FileAppender::builder().build("log/dev.log").unwrap();
        let expanded = FileAppender::builder().build("log/expanded.log").unwrap();

        let config = Config::builder()
            .appender(Appender::builder().build("main", Box::new(logger)))
            .appender(Appender::builder().build("expanded", Box::new(expanded)))
            .logger(
                Logger::builder()
                    .appender("expanded")
                    .additive(false)
                    .build("rusty_z::zmachine", LogLevelFilter::Info),
            )
            .build(Root::builder().appender("main").build(LogLevelFilter::Warn))
            .unwrap();

        log4rs::init_config(config).unwrap();
    }

    fn setup_loop<F>(&self, mut main_loop: F) -> LoopState
    where
        F: 'static + FnMut() -> u8,
    {
        let mut result;

        loop {
            let raw = main_loop();
            result = raw.into();

            match result {
                LoopState::Quitting => break,
                LoopState::Restarting => break,
                LoopState::Error => break,
                LoopState::Running => (),
            }
        }

        return result;
    }
}
