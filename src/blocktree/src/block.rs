use std::ops::{Deref, Range};
use std::marker::PhantomData;
use errors::*;

pub trait BlockDevice {
    fn block_size(&self) -> usize;
    fn num_blocks(&self) -> usize;
    fn get_block(&mut self, num: usize) -> Result<Block>;
    fn push(&mut self, block: &[u8]) -> Result<()>;
    fn pop(&mut self) -> Result<()>;
    fn replace(&mut self, num: usize, block: &[u8]) -> Result<()>;
}

pub struct Block<'a> {
    data: &'a [u8],
    phantom_mut: PhantomData<&'a mut ()>,
}

impl<'a> Deref for Block<'a> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.data
    }
}

pub struct MemBlockDevice {
    block_size: usize,
    buf: Vec<u8>,
}

impl MemBlockDevice {
    pub fn new(block_size: usize) -> MemBlockDevice {
        assert!(block_size != 0);
        assert!(block_size.is_power_of_two());
        MemBlockDevice {
            block_size: block_size,
            buf: Vec::new(),
        }
    }
}

impl BlockDevice for MemBlockDevice {
    fn block_size(&self) -> usize {
        self.block_size
    }

    fn num_blocks(&self) -> usize {
        assert!(self.buf.len() % self.block_size == 0);
        self.buf.len() / self.block_size
    }

    fn get_block(&mut self, num: usize) -> Result<Block> {
        let offset = num * self.block_size;
        assert!(offset + self.block_size <= self.buf.len());
        Ok(Block {
            data: &self.buf[offset .. offset + self.block_size],
            phantom_mut: PhantomData,
        })
    }

    fn push(&mut self, block: &[u8]) -> Result<()> {
        assert!(block.len() == self.block_size);
        self.buf.extend(block);
        Ok(())
    }

    fn pop(&mut self) -> Result<()> {
        assert!(self.buf.len() % self.block_size == 0);
        if self.buf.is_empty() {
            panic!("popping empty block device");
        } else {
            let newlen = self.buf.len() - self.block_size;
            self.buf.truncate(newlen);
            Ok(())
        }
    }

    fn replace(&mut self, num: usize, block: &[u8]) -> Result<()> {
        assert!(block.len() == self.block_size);
        let offset = num * self.block_size;
        assert!(offset + self.block_size <= self.buf.len());
        let buf = &mut self.buf[offset .. offset + self.block_size];
        buf.copy_from_slice(block);

        Ok(())
    }
}
