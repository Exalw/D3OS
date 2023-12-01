use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::ptr;
use crate::kernel;
use crate::library::graphic::ansi;
use crate::library::io::stream::OutputStream;

#[derive(Copy, Clone, PartialEq, PartialOrd)]
pub enum LogLevel {
    TRACE,
    DEBUG,
    INFO,
    WARN,
    ERROR
}

pub struct LogService {
    level: LogLevel,
    streams: Vec<Box<&'static mut dyn OutputStream>>
}

impl LogService {
    pub const fn new() -> Self {
        Self { level: LogLevel::INFO, streams: Vec::new() }
    }

    pub fn log(&mut self, level: LogLevel, name: &String, msg: &str) {
        if level < self.level {
            return;
        }

        let ms = kernel::get_time_service().get_systime_ms();
        let seconds = ms / 1000;
        let fraction = ms % 1000;

        let string = format!("{}[{}.{:0>3}]{}[{}]{}[{}] {}", ansi::FOREGROUND_CYAN, seconds, fraction, ansi_color(level), level_as_string(level), ansi::FOREGROUND_DEFAULT, name, msg);

        for i in 0 .. self.streams.len() {
            let stream = &mut self.streams[i];
            stream.write_str(&string);
            stream.write_byte('\n' as u8);
        }
    }

    pub fn register(&mut self, stream: &'static mut dyn OutputStream) {
        self.streams.push(Box::new(stream));
    }

    pub fn remove(&mut self, stream: &mut dyn OutputStream) {
        self.streams.retain(|element| ptr::from_ref(*element.as_ref()) != ptr::from_ref(stream));
    }
}

fn level_as_string(level: LogLevel) -> &'static str {
    match level {
        LogLevel::TRACE => "TRC",
        LogLevel::DEBUG => "DBG",
        LogLevel::INFO => "INF",
        LogLevel::WARN => "WRN",
        LogLevel::ERROR => "ERR"
    }
}

fn ansi_color(level: LogLevel) -> &'static str {
    match level {
        LogLevel::TRACE => ansi::FOREGROUND_BRIGHT_WHITE,
        LogLevel::DEBUG => ansi::FOREGROUND_BRIGHT_GREEN,
        LogLevel::INFO => ansi::FOREGROUND_BRIGHT_BLUE,
        LogLevel::WARN => ansi::FOREGROUND_BRIGHT_YELLOW,
        LogLevel::ERROR => ansi::FOREGROUND_BRIGHT_RED
    }
}