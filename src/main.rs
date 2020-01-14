use arsc::Table;
use clap::{value_t, App, Arg};
use memmap::MmapOptions;
use std::fs::File;
use zip::{CompressionMethod, ZipArchive};

fn main() {
    // parse command line arguments
    let opts = App::new("arsc")
        .arg(Arg::with_name("apk").takes_value(true).required(true))
        .get_matches();

    // memory map APK
    let apk = value_t!(opts.value_of("apk"), String).unwrap();
    let file = File::open(apk).expect("failed to open file");
    let mmap = unsafe { MmapOptions::new().map(&file).unwrap() };

    // read zip header, entry header
    let reader = std::io::Cursor::new(mmap.as_ref());
    let mut zip = ZipArchive::new(reader).expect("failed to open zip");
    let entry = zip
        .by_name("resources.arsc")
        .expect("failed to unzip resources.arsc");
    if entry.compression() != CompressionMethod::Stored {
        panic!("resources.arsc compressed");
    }

    // "extract" the non-compressed entry
    let begin = entry.data_start() as usize;
    let end = begin + entry.size() as usize;
    let buf = &mmap[begin..end];

    // parse resource table
    let table = Table::parse(buf).unwrap();
    for resid in table.resid_iter() {
        println!("resid={:?}", resid);
    }
}
