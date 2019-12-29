use arsc::{Chunk, ChunkIterator};
use std::env;
use std::fs::File;
use std::io::prelude::*;

fn iterate(iter: ChunkIterator, depth: usize) {
    let indent = (0..depth * 4).map(|_| " ").collect::<String>();
    for chunk in iter {
        match chunk {
            Chunk::Table(table, _) => {
                println!("{}Table packages={}", indent, table.package_count.value())
            }
            Chunk::Package(pkg, _) => println!("{}Package id={:#02x}", indent, pkg.id.value()),
            Chunk::StringPool(sp, _) => println!(
                "{}StringPool strings={} styles={}",
                indent,
                sp.string_count.value(),
                sp.style_count.value()
            ),
            Chunk::Spec(spec, _) => println!(
                "{}Spec id={:#02x} entires={}",
                indent,
                spec.id.value(),
                spec.entry_count.value()
            ),
            Chunk::Type(type_, _) => println!(
                "{}Type id={:#02x} entries={}",
                indent,
                type_.id.value(),
                type_.entry_count.value()
            ),
            Chunk::Error(ref msg) => println!("{}Error: {}", indent, msg),
        }
        if let Some(child_iter) = chunk.iter() {
            iterate(child_iter, depth + 1);
        }
    }
}

fn main() {
    let mut file = File::open(env::args().nth(1).unwrap()).unwrap();
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).unwrap();
    let iter = ChunkIterator::new(&buf);
    iterate(iter, 0);
}
