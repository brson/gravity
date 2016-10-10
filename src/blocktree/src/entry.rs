use errors::*;
use block::BlockDevice;
use std::io::{Read, Write};
use std::io::Result as IoResult;
use std::io::ErrorKind as IoErrorKind;
use std::io::Error as IoError;
use byteorder::{LittleEndian, WriteBytesExt, ReadBytesExt};
use std::iter;
use std::u16;
use std::u32;
use std::cmp;
use std::ops::Deref;
use seq::{SeqWriter, SeqReader};

pub struct EntryWriter<'a, 'b: 'a> {
    seq_writer: &'a mut SeqWriter<'b>,
    buf: Vec<u8>,
}

impl<'a, 'b> EntryWriter<'a, 'b> {
    pub fn new(seq_writer: &'a mut SeqWriter<'b>) -> EntryWriter<'a, 'b> {
        EntryWriter {
            seq_writer: seq_writer,
            buf: Vec::new(),
        }
    }

    pub fn complete_entry(&mut self) -> Result<()> {
        assert!(self.buf.len() <= u16::MAX as usize);
        self.seq_writer.write_u16::<LittleEndian>(self.buf.len() as u16)?;
        self.seq_writer.write_all(&*self.buf)?;
        self.buf.truncate(0);
        Ok(())
    }
}

impl<'a, 'b> Write for EntryWriter<'a, 'b> {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        self.buf.extend(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> IoResult<()> { Ok(()) }
}

impl<'a, 'b> Drop for EntryWriter<'a, 'b> {
    fn drop(&mut self) {
        assert!(self.buf.is_empty());
    }
}

pub struct EntryReader<'a, 'b: 'a> {
    seq_reader: &'a mut SeqReader<'b>,
    entry_info: Option<(usize, usize)>,
}

impl<'a, 'b> EntryReader<'a, 'b> {
    pub fn new(seq_reader: &'a mut SeqReader<'b>) -> EntryReader<'a, 'b> {
        EntryReader {
            seq_reader: seq_reader,
            entry_info: None,
        }
    }

    pub fn enter_entry(&mut self) -> Result<bool> {
        if self.entry_info.is_some() {
            // Make sure we've read all the way to the end of the previous entry
            // before reading the next. FIXME: seek
            let ref mut buf = [0; 32];
            while self.read(buf)? != 0 { }
        }
        let entry_len = self.seq_reader.read_u16::<LittleEndian>();
        match entry_len {
            Ok(len) => {
                self.entry_info = Some((len as usize, 0));
                Ok(true)
            }
            Err(e) => {
                if e.kind() == IoErrorKind::UnexpectedEof {
                    self.entry_info = None;
                    Ok(false)
                } else {
                    Err(e.into())
                }
            }
        }
    }
}

impl<'a, 'b> Read for EntryReader<'a, 'b> {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        if let Some((len, offset)) = self.entry_info {
            let bytes_left = len - offset;
            if bytes_left != 0 {
                let bytes_to_read = cmp::min(bytes_left, buf.len());
                let buf = &mut buf[0 .. bytes_to_read];
                let read = self.seq_reader.read(buf)?;
                assert!(offset + read <= len);
                self.entry_info = Some((len, offset + read));
                Ok(read)
            } else {
                Ok(0)
            }
        } else {
            panic!("call enter_entry");
        }
    }
}

pub fn entry_size(key: &[u8]) -> usize {
    key.len() + 2
}
