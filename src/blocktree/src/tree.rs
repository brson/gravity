use errors::*;
use std::cmp::Ordering;
use block::BlockDevice;
use seq::{SeqWriter, SeqReader};
use std::io::{Write, Read};
use std::u32;
use entry::{EntryWriter, EntryReader};
use byteorder::{LittleEndian, WriteBytesExt, ReadBytesExt};

const NODE_TYPE_INTERNAL: u8 = 1;
const NODE_TYPE_LEAF: u8 = 2;

pub struct BlobTree<'a> {
    device: &'a mut BlockDevice,
    root_page: Option<u32>,
}

impl<'a> BlobTree<'a> {
    pub fn new(device: &mut BlockDevice, root_page: Option<u32>) -> BlobTree {
        BlobTree {
            device: device,
            root_page: root_page,
        }
    }

    pub fn insert<C>(&mut self, key: &[u8], value: &[u8], cmp: C) -> Result<()>
        where C: Fn(&[u8], &[u8]) -> Result<Ordering>
    {
        let (page, index) = self.insert_value(value)?;
        if let Some(root_page) = self.root_page {
            let block_info = seq::info(self.device, root_page as usize)?;
            let (block_capacity, block_len, block_continues) = block_info;
            let ref mut seq_reader = SeqReader::new(self.device, root_page as usize);
            let ref mut entry_reader = EntryReader::new(seq_reader);
            entry_reader.enter_entry()?;
            let node_type = entry_reader.read_u8()?;
            if node_type == NODE_TYPE_LEAF {
                let mut index = 0;
                let mut matched = false;
                let mut matched_entry_size = 0;
                let mut count = 0;
                loop {
                    let ref mut candidate_key = Vec::new();
                    if !read_leaf_entry(entry_reader, candidate_key)? {
                        break;
                    }
                    match cmp(key, candidate_key) {
                        Ordering::Less => {
                            index += 1;
                        }
                        Ordering::Equal => {
                            matched = true;
                            matched_entry_size = entry::entry_size(candidate_key);
                        }
                        Ordering::Greater => { }
                    }
                    count += 1;
                }
                let new_entry_size = entry::entry_size(key);
                // Now figure out what to do for the insert
                if !block_continues {
                    // This is a single block sequence
                    let room_for_entry = block_capacity - block_len >= new_entry_size;
                    if matched {
                        if new_entry_size <= matched_entry_size {
                            insert_action = LeafReplace(root_page, index)
                        } else if room_for_entry {
                            insert_action = LeafReplace(root_page, index);
                        } else {
                            insert_action = LeafSplit(root_page, index, count);
                        }
                    } else {
                        if room_for_entry {
                            insert_action = LeafInsert(root_page, index);
                        } else {
                            insert_action = LeafSplit(root_page, index, count);
                        }
                    }
                } else {
                    panic!()
                }
            } else {
                panic!()
            }
        } else {
            // There's no root page yet
            let mut seq_writer = SeqWriter::new(self.device);
            {
                let ref mut entry_writer = EntryWriter::new(&mut seq_writer);
                entry_writer.write_u8(NODE_TYPE_LEAF)?;
                entry_writer.complete_entry();
                write_leaf_entry(entry_writer, key, page, index)?;
            }
            let (root_page, _) = seq_writer.finish()?;
            self.root_page = Some(root_page as u32);
            return Ok(());
        }
    }

    pub fn delete<C>(&mut self, key: &[u8], cmp: C) -> Result<()>
        where C: Fn(&[u8], &[u8]) -> Result<Ordering>
    {
        panic!()
    }

    pub fn find<C>(&mut self, key: &[u8], buf: &mut Vec<u8>, cmp: C) -> Result<bool>
        where C: Fn(&[u8], &[u8]) -> Result<Ordering>
    {
        buf.truncate(0);
        let page_and_index;
        if let Some(root_page) = self.root_page {
            let mut seq_reader = SeqReader::new(self.device, root_page as usize);
            let ref mut entry_reader = EntryReader::new(&mut seq_reader);
            if !entry_reader.enter_entry()? {
                panic!("no header in index block");
            }
            let node_type = entry_reader.read_u8()?;
            if node_type == NODE_TYPE_LEAF {
                let ref mut candidate_key = Vec::new();
                loop {
                    let next_entry = read_leaf_entry(entry_reader, candidate_key)?;
                    if let Some(candidate_page_and_index) = next_entry {
                        if cmp(key, candidate_key)? == Ordering::Equal {
                            // We've found the matching key
                            page_and_index = candidate_page_and_index;
                            break;
                        }
                    } else {
                        return Ok(false)
                    }
                }
            } else {
                panic!()
            }
        } else {
            // No root page, no data
            return Ok(false);
        }

        let (page, index) = page_and_index;
        self.retrieve_value(page, index, buf)?;
        Ok(true)
    }

    pub fn root_page(&self) -> Option<u32> {
        self.root_page
    }

    fn insert_value(&mut self, value: &[u8]) -> Result<(u32, u16)> {
        let mut seq_writer = SeqWriter::new(self.device);
        seq_writer.write_all(value)?;
        let (block, _) = seq_writer.finish()?;
        assert!(block <= u32::MAX as usize);
        Ok((block as u32, 0))
    }

    fn retrieve_value(&mut self, page: u32, index: u16,
                      buf: &mut Vec<u8>) -> Result<()> {
        let mut seq_reader = SeqReader::new(self.device, page as usize);
        seq_reader.read_to_end(buf)?;
        Ok(())
    }
}

fn write_leaf_entry(entry_writer: &mut EntryWriter, key: &[u8],
                    page: u32, index: u16) -> Result<()> {
    entry_writer.write_u32::<LittleEndian>(page)?;
    entry_writer.write_u16::<LittleEndian>(index)?;
    entry_writer.write_all(key)?;
    entry_writer.complete_entry()?;
    Ok(())
}

fn read_leaf_entry(entry_reader: &mut EntryReader,
                   key: &mut Vec<u8>) -> Result<Option<(u32, u16)>> {
    if !entry_reader.enter_entry()? {
        return Ok(None);
    }

    let page = entry_reader.read_u32::<LittleEndian>()?;
    let index = entry_reader.read_u16::<LittleEndian>()?;
    entry_reader.read_to_end(key)?;
    Ok(Some((page, index)))
}

enum InsertAction {
    LeafReplace(u32, u16),
    LeafInsert(u32, u16),
}
   
