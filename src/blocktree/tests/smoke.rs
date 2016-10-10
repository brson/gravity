#![allow(unused)]

extern crate blocktree;

use std::io::{Read, Write, BufReader, BufRead};
use blocktree::block::{BlockDevice, MemBlockDevice};
use blocktree::tree::BlobTree;
use blocktree::seq::{SeqWriter, SeqReader};
use std::cmp::Ordering;
use blocktree::errors::*;

#[test]
fn seq_writer() {
    let ref mut device = MemBlockDevice::new(4096);
    {
        let mut writer = SeqWriter::new(device);
        for i in 0 .. 10000 {
            writer.write(b"test\n").unwrap();
        }

        let (block_num, count) = writer.finish().unwrap();
    }

    {
        let reader = SeqReader::new(device, 0);
        let mut reader = BufReader::new(reader);
        let mut count = 0;
        for line in reader.lines() {
            let line = line.unwrap();
            assert!(line == "test");
            count += 1;
        }
        assert!(count == 10000);
    }
}

fn bin_cmp(a: &[u8], b: &[u8]) -> Result<Ordering> {
    Ok(a.cmp(b))
}

#[test]
fn tree_insert() {
    let ref mut device = MemBlockDevice::new(32);
    let ref mut tree = BlobTree::new(device, None);
    let key = b"a";
    let value = b"b";
    tree.insert(key, value, bin_cmp).unwrap();
    let ref mut buf = Vec::new();
    tree.find(key, buf, bin_cmp).unwrap();
    assert!(value == &**buf);
    let ref mut buf = Vec::new();
    tree.find(b"c", buf, bin_cmp).unwrap();
    assert!(value != &**buf);
}
