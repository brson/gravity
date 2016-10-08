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

// The size of data in each block
const DATA_LEN_HEADER_SIZE: usize = 2;
// A u32 at the end of each block that links to the next
const LINK_FOOTER_SIZE: usize = 4;

pub struct SeqWriter<'a> {
    device: &'a mut BlockDevice,
    buf: Vec<u8>,
    finished: bool,
}

impl<'a> SeqWriter<'a> {
    pub fn new(device: &'a mut BlockDevice) -> SeqWriter<'a> {
        SeqWriter {
            device: device,
            buf: Vec::new(),
            finished: false,
        }
    }

    pub fn blocks_used(&self) -> usize {
        panic!();
    }

    pub fn free_space_in_block(&self) -> usize {
        panic!();
    }

    pub fn finish(mut self) -> Result<(usize, usize)> {
        assert!(!self.buf.is_empty());
        assert!(!self.finished);
        let first_block = self.device.num_blocks();
        let chunk_size = self.device.block_size()
            - DATA_LEN_HEADER_SIZE - LINK_FOOTER_SIZE;
        let block_count = self.buf.chunks(chunk_size).count();
        let mut block_buf = Vec::with_capacity(self.device.block_size());
        for (i, block_data) in self.buf.chunks(chunk_size).enumerate() {
            assert!(block_data.len() <= u16::MAX as usize);
            // Write the block header, the length of the data
            block_buf.write_u16::<LittleEndian>(block_data.len() as u16);
            // Write the block data
            block_buf.extend(block_data);
            // The last block may not be filled with data, so pad it
            let padding = chunk_size - block_data.len();
            block_buf.extend(iter::repeat(0).take(padding));
            if i + 1 < block_count {
                // Write the block number of the next block
                let next_block = self.device.num_blocks() + 1;
                assert!(next_block < u32::MAX as usize); // <, not <= to account for the
                                                         // sentinel value u32::MAX
                block_buf.write_u32::<LittleEndian>(next_block as u32)?;
            } else {
                // For the last block write a sentinel value
                block_buf.write_u32::<LittleEndian>(u32::MAX)?;
            }
            assert!(block_buf.len() == self.device.block_size());
            self.device.push(&block_buf)?;
            block_buf.truncate(0);
        }
        self.finished = true;
        Ok((first_block, block_count))
    }
}

impl<'a> Drop for SeqWriter<'a> {
    fn drop(&mut self) {
        assert!(self.finished);
    }
}

impl<'a> Write for SeqWriter<'a> {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        self.buf.extend(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> IoResult<()> { Ok(()) }
}

pub struct SeqReader<'a> {
    device: &'a mut BlockDevice,
    block_num: usize,
    offset: usize,
}

impl<'a> SeqReader<'a> {
    pub fn new(device: &'a mut BlockDevice, block_num: usize) -> SeqReader<'a> {
        SeqReader {
            device: device,
            block_num: block_num,
            offset: 0,
        }
    }
}

impl<'a> Read for SeqReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        let block_size = self.device.block_size();
        let block = self.device.get_block(self.block_num)
            .map_err(|e| IoError::new(IoErrorKind::Other, format!("{}", e)))?;
        let data_len = block.deref().read_u16::<LittleEndian>()?;
        assert!(self.offset <= u16::MAX as usize);
        let bytes_left = data_len as usize - self.offset;
        let bytes_to_read = cmp::min(bytes_left, buf.len());
        let offset = DATA_LEN_HEADER_SIZE + self.offset;
        let src = &block[offset .. offset + bytes_to_read];
        let dst = &mut buf[ .. bytes_to_read];
        dst.copy_from_slice(src);
        self.offset += bytes_to_read;
        if bytes_to_read == bytes_left {
            let footer = &block[block_size - LINK_FOOTER_SIZE .. block_size];
            let next_block = footer.deref().read_u32::<LittleEndian>()?;
            if next_block != u32::MAX {
                self.block_num = next_block as usize;
                self.offset = 0;
            }
        }
        Ok(bytes_to_read)
    }
}
