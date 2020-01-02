use arsc::{Chunk, ChunkIterator, Package, Spec, StringPool, Table, Type};
use clap::{value_t, App, Arg};
use memmap::MmapOptions;
use std::fs::File;
use zip::{CompressionMethod, ZipArchive};

#[allow(clippy::transmute_ptr_to_ptr)]
fn iterate(iter: ChunkIterator, depth: usize) {
    let indent = (0..depth * 4).map(|_| " ").collect::<String>();
    for chunk in iter {
        match chunk {
            Chunk::Table(bytes) => {
                let table: &Table = unsafe { std::mem::transmute(&bytes[0]) };
                println!("{}Table packages={}", indent, table.package_count.value());
            }
            Chunk::Package(bytes) => {
                let pkg: &Package = unsafe { std::mem::transmute(&bytes[0]) };
                println!("{}Package id={:#02x}", indent, pkg.id.value());
            }
            Chunk::StringPool(bytes) => {
                let sp: &StringPool = unsafe { std::mem::transmute(&bytes[0]) };
                println!(
                    "{}StringPool strings={} styles={}",
                    indent,
                    sp.string_count.value(),
                    sp.style_count.value()
                );
            }
            Chunk::Spec(bytes) => {
                let spec: &Spec = unsafe { std::mem::transmute(&bytes[0]) };
                println!(
                    "{}Spec id={:#02x} entires={}",
                    indent,
                    spec.id.value(),
                    spec.entry_count.value()
                );
            }
            Chunk::Type(bytes) => {
                let type_: &Type = unsafe { std::mem::transmute(&bytes[0]) };
                println!(
                    "{}Type id={:#02x} entries={}",
                    indent,
                    type_.id.value(),
                    type_.entry_count.value()
                );
            }
            Chunk::Error(ref msg) => println!("{}Error: {}", indent, msg),
        }
        if let Some(child_iter) = chunk.iter() {
            iterate(child_iter, depth + 1);
        }
    }
}

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

    // traverse chunks
    let iter = ChunkIterator::new(buf);
    iterate(iter, 0);
}
