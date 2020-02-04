use std::io;
use std::fmt;
pub type XousArgumentCode = u32;
pub type XousSize = u32;

#[macro_export]
macro_rules! make_type {
    ($fcc:expr) => {
        {
            let mut c: [u8; 4] = Default::default();
            c.copy_from_slice($fcc.as_bytes());
            XousArgumentCode::from_le_bytes(c)
        }
    };
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

pub struct XousArguments<'a> {
    arguments: Vec<&'a dyn XousArgument>,
}

impl<'a> fmt::Display for XousArguments<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Xous Arguments with {} parameters", self.arguments.len())?;
        for arg in &self.arguments {
            write!(f, "{}", arg)?;
        }
        Ok(())
    }
}

impl<'a> XousArguments<'a> {
    pub fn new() -> XousArguments<'a> {
        XousArguments { arguments: vec![] }
    }

    pub fn add(&mut self, arg: &'a dyn XousArgument) {
        self.arguments.push(arg);
    }

    pub fn write<T>(&self, mut w: T) -> io::Result<()> where T: io::Write {
        let mut total_length = 12; // 'XASZ' plus tag length total length
        for arg in &self.arguments {
            total_length = total_length + arg.length() + 8;
        }

        w.write(&make_type!("XASZ").to_le_bytes())?;
        w.write(&0u16.to_le_bytes())?;
        w.write(&1u16.to_le_bytes())?;
        w.write(&((total_length/4) as u32).to_le_bytes())?;

        for arg in &self.arguments {
            w.write(&arg.code().to_le_bytes())?;
            w.write(&0u16.to_le_bytes())?;

            let advertised_len = arg.length() as u32;
            w.write(&((advertised_len/4) as u16).to_le_bytes())?;

            let actual_len = arg.serialize(&mut w)? as u32;
            assert_eq!(advertised_len, actual_len, "argument advertised it would write {} bytes, but it wrote {} bytes", advertised_len, actual_len);
        }
        Ok(())
    }
}