#![allow(unused)]

extern crate blocktree;

use std::io::{Read, Write, BufReader, BufRead};
use blocktree::block::{BlockDevice, MemBlockDevice};
use blocktree::tree::BlobTree;
use blocktree::seq::{SeqWriter, SeqReader};

#[test]
fn seq_writer() {
    let mut device = Box::new(MemBlockDevice::new(4096));
    {
        let mut writer = SeqWriter::new(&mut *device);
        for i in 0 .. 10000 {
            writer.write(b"test\n");
        }

        let (block_num, count) = writer.finish().unwrap();
    }

    {
        let reader = SeqReader::new(&mut *device, 0);
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
