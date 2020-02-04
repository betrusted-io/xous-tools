use crate::xous_arguments::{XousArgument, XousArgumentCode, XousSize};
use std::fmt;
use std::io;

#[derive(Debug)]
pub struct XousKernel {
    /// Address of PID1 in RAM (i.e. SPI flash)
    load_offset: u32,

    /// Size of PID1
    load_size: u32,

    /// Virtual address of .text section in RAM
    text_offset: u32,

    /// Virtual address of .data and .bss section in RAM
    data_offset: u32,

    /// Size of .data and .bss section
    data_size: u32,

    /// Virtual address trap handler
    trap_handler: u32,
}

impl fmt::Display for XousKernel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "XousKernel: {} bytes long, loaded from {:08x} to {:08x} with trap_handler @ {:08x}, and {} bytes of data @ {:08x}",
            self.load_size, self.load_offset, self.text_offset, self.trap_handler,
            self.data_size, self.data_offset)
    }
}

impl XousKernel {
    pub fn new(
        load_offset: u32,
        load_size: u32,
        text_offset: u32,
        data_offset: u32,
        data_size: u32,
        trap_handler: u32,
    ) -> XousKernel {
        XousKernel {
            load_offset,
            load_size,
            text_offset,
            data_offset,
            data_size,
            trap_handler,
        }
    }
}

impl XousArgument for XousKernel {
    fn code(&self) -> XousArgumentCode {
        make_type!("XKRN")
    }
    fn length(&self) -> XousSize {
        std::mem::size_of::<Self>() as XousSize
    }
    fn serialize(&self, output: &mut dyn io::Write) -> io::Result<usize> {
        let mut written = 0;
        written += output.write(&self.load_offset.to_le_bytes())?;
        written += output.write(&self.load_size.to_le_bytes())?;
        written += output.write(&self.text_offset.to_le_bytes())?;
        written += output.write(&self.data_offset.to_le_bytes())?;
        written += output.write(&self.data_size.to_le_bytes())?;
        written += output.write(&self.trap_handler.to_le_bytes())?;
        Ok(written)
    }
}