use errors::*;
use std::cmp::Ordering;

use block::BlockDevice;

pub struct BlobTree<'a> {
    device: &'a mut BlockDevice,
    root_page: Option<usize>,
}

impl<'a> BlobTree<'a> {
    pub fn new(device: &mut BlockDevice, root_page: Option<usize>) -> Result<BlobTree> {
        Ok(BlobTree {
            device: device,
            root_page: root_page,
        })
    }

    pub fn insert<C>(&mut self, key: &[u8], value: &[u8], cmp: C) -> Result<()>
        where C: Fn(&[u8]) -> Result<Ordering>
    {
        if let Some(root_page) = self.root_page {
            panic!()
        } else {
            panic!()
        }
        panic!()
    }

    pub fn delete<C>(&mut self, key: &[u8], cmp: C) -> Result<()>
        where C: Fn(&[u8]) -> Result<Ordering>
    {
        panic!()
    }

    pub fn find<C>(&mut self, key: &[u8], cmp: C) -> Result<()>
        where C: Fn(&[u8]) -> Result<Ordering>
    {
        panic!()
    }

    pub fn root_page(&self) -> Option<usize> {
        self.root_page
    }
}
