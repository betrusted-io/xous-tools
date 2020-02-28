use std::fmt;
use std::io::{Cursor, Write, Result};
pub type XousArgumentCode = u32;
pub type XousSize = u32;
use crc::{crc16, Hasher16};

#[macro_export]
macro_rules! make_type {
    ($fcc:expr) => {{
        let mut c: [u8; 4] = Default::default();
        c.copy_from_slice($fcc.as_bytes());
        u32::from_le_bytes(c)
    }};
}

pub trait XousArgument: fmt::Display {
    /// A fourcc code of this tag
    fn code(&self) -> XousArgumentCode;

    /// The total size of this argument, not including the code and the length.
    fn length(&self) -> XousSize;

    /// Write the contents of this argument to the specified writer.
    /// Return the number of bytes written.
    fn serialize(&self, output: &mut dyn Write) -> Result<usize>;
}

pub struct XousArguments {
    ram_start: XousSize,
    ram_length: XousSize,
    ram_name: u32,
    arguments: Vec<Box<dyn XousArgument>>,
}

impl fmt::Display for XousArguments {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Xous Arguments with {} parameters", self.arguments.len())?;

        let tag_name_bytes = self.ram_name.to_le_bytes();
        let tag_name = String::from_utf8_lossy(&tag_name_bytes);
        writeln!(
            f,
            "    Main RAM \"{}\" ({:08x}): {:08x} - {:08x}",
            tag_name,
            self.ram_name,
            self.ram_start,
            self.ram_start + self.ram_length
        )?;

        for arg in &self.arguments {
            write!(f, "{}", arg)?;
        }
        Ok(())
    }
}

impl XousArguments {
    pub fn new(ram_start: XousSize, ram_length: XousSize, ram_name: u32) -> XousArguments {
        XousArguments {
            ram_start,
            ram_length,
            ram_name,
            arguments: vec![],
        }
    }

    pub fn add<T: 'static>(&mut self, arg: T)
    where
        T: XousArgument + Sized,
    {
        self.arguments.push(Box::new(arg));
    }

    pub fn write<T>(&self, mut w: T) -> Result<()>
    where
        T: Write,
    {
        let total_length = self.len();


        // XArg tag contents
        let mut tag_data = Cursor::new(Vec::new());
        tag_data.write(&((total_length / 4) as u32).to_le_bytes())?;
        tag_data.write(&1u32.to_le_bytes())?; // Version
        tag_data.write(&(self.ram_start as u32).to_le_bytes())?;
        tag_data.write(&(self.ram_length as u32).to_le_bytes())?;
        tag_data.write(&(self.ram_name as u32).to_le_bytes())?;

        assert!((tag_data.get_ref().len() & 3) == 0, "tag data was not a multiple of 4 bytes!");

        let mut digest = crc16::Digest::new(crc16::X25);
        // XArg tag header
        w.write(&make_type!("XArg").to_le_bytes())?;
        digest.write(tag_data.get_ref());
        w.write(&digest.sum16().to_le_bytes())?; // CRC16
        w.write(&((tag_data.get_ref().len() / 4) as u16).to_le_bytes())?; // Size (in words)
        w.write(tag_data.get_ref())?;

        // Write out each subsequent argument
        for arg in &self.arguments {

            let mut tag_data = Cursor::new(Vec::new());
            let advertised_len = arg.length() as u32;
            let actual_len = arg.serialize(&mut tag_data)? as u32;
            assert_eq!(
                advertised_len, actual_len,
                "argument advertised it would write {} bytes, but it wrote {} bytes",
                advertised_len, actual_len
            );
            assert_eq!(
                tag_data.get_ref().len() as u32, actual_len,
                "argument said it wrote {} bytes, but it actually wrote {} bytes",
                actual_len, tag_data.get_ref().len()
            );

            let mut digest = crc16::Digest::new(crc16::X25);
            // XArg tag header
            w.write(&arg.code().to_le_bytes())?;
            digest.write(tag_data.get_ref());
            w.write(&digest.sum16().to_le_bytes())?; // CRC16
            w.write(&((tag_data.get_ref().len() / 4) as u16).to_le_bytes())?; // Size (in words)
            w.write(tag_data.get_ref())?;
        }
        Ok(())
    }

    pub fn len(&self) -> u32 {
        let mut total_length = 20 + self.header_len() as u32; // 'XArg' plus tag length total length
        for arg in &self.arguments {
            total_length += arg.length() + 8;
        }
        total_length
    }

    pub fn header_len(&self) -> usize {
        8
    }
}
