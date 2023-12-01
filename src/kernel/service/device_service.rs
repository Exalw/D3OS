use alloc::boxed::Box;
use acpi::AcpiTables;
use spin::Mutex;
use crate::device::lfb_terminal::{CursorThread, LFBTerminal};
use crate::device::ps2::PS2;
use crate::device::serial;
use crate::device::serial::{BaudRate, ComPort, SerialPort};
use crate::device::speaker::Speaker;
use crate::device::terminal::Terminal;
use crate::kernel;
use crate::kernel::Service;
use crate::kernel::service::memory_service::AcpiHandler;
use crate::kernel::thread::thread::Thread;

pub struct DeviceService {
    speaker: Mutex<Speaker>,
    ps2: PS2,
    terminal: LFBTerminal,
    serial: Option<SerialPort>,
    acpi_tables: Option<AcpiTables<AcpiHandler>>
}

impl Service for DeviceService {}

impl DeviceService {
    pub const fn new() -> Self {
        Self {
            speaker: Mutex::new(Speaker::new()),
            ps2: PS2::new(),
            terminal: LFBTerminal::empty(),
            serial: None,
            acpi_tables: None
        }
    }

    pub fn init_keyboard(&mut self) {
        self.ps2.init_controller().unwrap_or_else(|err| panic!("Failed to initialize PS2 controller (Error: {:?})", err));
        self.ps2.init_keyboard().unwrap_or_else(|err| panic!("Failed to initialize PS2 keyboard (Error: {:?})", err));
        self.ps2.plugin_keyboard();
    }

    pub fn init_terminal(&mut self, buffer: *mut u8, pitch: u32, width: u32, height: u32, bpp: u8) {
        self.terminal = LFBTerminal::new(buffer, pitch, width, height, bpp);
        kernel::get_thread_service().ready_thread(Thread::new_kernel_thread(Box::new(|| {
            let terminal = &mut kernel::get_device_service().terminal;
            let mut cursor_thread = CursorThread::new(terminal);

            cursor_thread.run();
        })))
    }

    pub fn init_serial_port(&mut self) {
        let mut serial: Option<SerialPort> = None;
        unsafe {
            if serial::check_port(ComPort::Com1) {
                serial = Some(SerialPort::new(ComPort::Com1));
            } else if serial::check_port(ComPort::Com2) {
                serial = Some(SerialPort::new(ComPort::Com2));
            } else if serial::check_port(ComPort::Com3) {
                serial = Some(SerialPort::new(ComPort::Com3));
            } else if serial::check_port(ComPort::Com4) {
                serial = Some(SerialPort::new(ComPort::Com4));
            }
        }

        if serial.is_some() {
            unsafe { serial.as_mut().unwrap().init(128, BaudRate::Baud115200); }
            self.serial = Some(serial.unwrap());
        }

    }

    pub fn init_acpi_tables(&mut self, rsdp_addr: usize) {
        let handler = AcpiHandler::default();

        unsafe {
            let tables = AcpiTables::from_rsdp(handler, rsdp_addr);
            match tables {
                Ok(tables) => {
                    self.acpi_tables = Some(tables);
                }
                Err(_) => {
                    panic!("Failed to parse ACPI tables");
                }
            }
        }
    }

    pub fn get_speaker(&self) -> &Mutex<Speaker> {
        return &self.speaker;
    }

    pub fn get_ps2(&mut self) -> &mut PS2 {
        return &mut self.ps2;
    }

    pub fn get_terminal(&mut self) -> &mut dyn Terminal {
        return &mut self.terminal;
    }

    pub fn get_serial_port(&mut self) -> &mut Option<SerialPort> {
        return &mut self.serial;
    }

    pub fn get_acpi_tables(&mut self) -> &mut AcpiTables<AcpiHandler> {
        return self.acpi_tables.as_mut().expect("ACPI: Accessing tables before initialization!");
    }
}