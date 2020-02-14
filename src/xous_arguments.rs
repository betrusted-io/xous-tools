use std::fmt;
use std::io;
pub type XousArgumentCode = u32;
pub type XousSize = u32;

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
    fn serialize(&self, output: &mut dyn std::io::Write) -> io::Result<usize>;
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
    pub fn new(ram_start: XousSize, ram_length: XousSize, ram_name: &str) -> XousArguments {
        XousArguments {
            ram_start,
            ram_length,
            ram_name: make_type!(ram_name),
            arguments: vec![],
        }
    }

    pub fn add<T: 'static>(&mut self, arg: T)
    where
        T: XousArgument + Sized {
        self.arguments.push(Box::new(arg));
    }

    pub fn write<T>(&self, mut w: T) -> io::Result<()>
    where
        T: io::Write,
    {
        let total_length = self.len();

        // XArg tag header
        w.write(&make_type!("XArg").to_le_bytes())?;
        w.write(&0u16.to_le_bytes())?; // CRC16
        w.write(&5u16.to_le_bytes())?; // Size (in words)

        // XArg tag contents
        w.write(&((total_length / 4) as u32).to_le_bytes())?;
        w.write(&1u32.to_le_bytes())?; // Version
        w.write(&(self.ram_start as u32).to_le_bytes())?;
        w.write(&(self.ram_length as u32).to_le_bytes())?;
        w.write(&(self.ram_name as u32).to_le_bytes())?;

        // Write out each subsequent argument
        for arg in &self.arguments {
            w.write(&arg.code().to_le_bytes())?;
            w.write(&0u16.to_le_bytes())?;

            let advertised_len = arg.length() as u32;
            w.write(&((advertised_len / 4) as u16).to_le_bytes())?;

            let actual_len = arg.serialize(&mut w)? as u32;
            assert_eq!(
                advertised_len, actual_len,
                "argument advertised it would write {} bytes, but it wrote {} bytes",
                advertised_len, actual_len
            );
        }
        Ok(())
    }

    pub fn len(&self) -> u32 {
        let mut total_length = 20 + 8; // 'XArg' plus tag length total length
        for arg in &self.arguments {
            total_length = total_length + arg.length() + 8;
        }
        total_length
    }
}
