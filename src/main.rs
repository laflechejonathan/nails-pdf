#![recursion_limit = "80"]

extern crate inflate;

use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::io::SeekFrom;
use std::path::Path;
use std::collections::HashMap;
use std::io::BufReader;
use std::str::from_utf8;
use inflate::inflate_bytes;
use pest::prelude::*;

#[macro_use] extern crate pest;
#[macro_use] extern crate maplit;

mod parsers;

const CHUNK_SIZE: i64 = 10240;

fn parse_xref(file: &mut File, offset: u64) -> parsers::xref::XRefTable {
    match file.seek(SeekFrom::Start(offset)) {
        Err(_) => panic!("couldn't seek to xref"),
        Ok(_) => (),
    };

    let newline = "\n".to_string();
    let file_reader = BufReader::new(file);
    let mut xref_str: String = "".to_owned();
    for line in file_reader.lines() {
        let unwrapped = line.unwrap();
        xref_str.push_str(&unwrapped);
        xref_str.push_str(&newline);
        if unwrapped == "trailer" {
            break;
        }
    }

    let mut xref_parser = parsers::xref::Rdp::new(StringInput::new(&xref_str));
    xref_parser.xref();
    return xref_parser.parse();
}

fn get_doc_metadata(file: &mut File) -> (parsers::cos::DictNode, parsers::xref::XRefTable) {
    let mut buffer = Vec::new();
    let mut trailer = Vec::new();

    match file.seek(SeekFrom::End(-CHUNK_SIZE)) {
        Err(_) => panic!("couldn't seek to eof"),
        Ok(_) => (),
    }
    file.take(CHUNK_SIZE as u64).read_to_end(&mut buffer).unwrap();

    let mut found_xref = false;
    let mut found_trailer = false;
    let mut xref_offset= 0;
    let newline = '\n' as u8;

    for line in buffer.split(|byte| *byte == newline) {
        if line == "trailer".as_bytes() {
            found_trailer = true;
        }
        else if line == "startxref".as_bytes() {
            found_xref = true;
        } else if found_trailer && !found_xref {
            trailer.extend_from_slice(line);
        }
        else if found_xref {
            let string = String::from_utf8(line.to_vec()).unwrap();
            xref_offset = string.parse::<u64>().unwrap();
            break;
        }
    }

    let trailer_str = String::from_utf8(trailer).unwrap();
    let mut trailer_parser = parsers::cos::Rdp::new(StringInput::new(&trailer_str));
    trailer_parser.node();
    let trailer = trailer_parser.parse();
    let xref = parse_xref(file, xref_offset);
    return (trailer, xref);
}

fn cat_xobject(file: &mut File, xref_entry: parsers::xref::XRefEntry) {
    match file.seek(SeekFrom::Start(xref_entry.offset)) {
        Err(_) => panic!("couldn't seek to object"),
        Ok(_) => (),
    };

    let newline = '\n' as u8;
    let mut obj_dict_buffer = Vec::new();
    let mut file_buffer = Vec::new();
    file.take(CHUNK_SIZE as u64).read_to_end(&mut file_buffer).unwrap();

    for line in file_buffer.split(|byte| *byte == newline).skip(1) {
        if line == "stream".as_bytes() {
            break
        } else if line == "endstream".as_bytes() {
            break
        } else if line == "endobj".as_bytes() {
            break
        } else {
            obj_dict_buffer.extend_from_slice(line);
        }
    }

    let dict_str = String::from_utf8(obj_dict_buffer).unwrap();
    let mut dict_parser = parsers::cos::Rdp::new(StringInput::new(&dict_str));
    dict_parser.node();
    let obj_dict = dict_parser.parse();

    println!("Object: {:?}", obj_dict);
}


// This is the main function
fn main() {
    let path = Path::new("michaelnguyen.pdf");
    let display = path.display();

    // Open the path in read-only mode, returns `io::Result<File>`
    let mut file = match File::open(&path) {
        Err(why) => panic!("couldn't open {}: {}", display,
                           why.description()),
        Ok(file) => file,
    };

    let (trailer, xref) = get_doc_metadata(&mut file);

    println!("Trailer:\n{:?}", trailer);
    println!("Xref:\n{:?}", xref);

    for (index, entry) in xref.into_iter().enumerate() {
        if !entry.is_free {
            println!("cat XObject {} at offset {}", index, entry.offset);
            cat_xobject(&mut file, entry);
        }
    }
}
