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

// mod parser;

#[derive(Debug)]
enum DictNode {
    Dict(HashMap<String, DictNode>),
    Array(Vec<DictNode>),
    Str(String),
    ObjectReference(u64, u64),
}

const  CHUNK_SIZE: i64 = 10240;

fn tokens_to_dict_node(tokens: &Vec<String>) -> DictNode {
    println!("OK {:?}", tokens);
    if tokens.len() == 1 {
        return DictNode::Str(tokens[0].clone());
    }
    else if tokens.len() == 3 && tokens[2] == "R" {
        return DictNode::ObjectReference(
            tokens[0].parse::<u64>().unwrap(),
            tokens[1].parse::<u64>().unwrap(),
        );
    } else {
        let nodes = tokens.iter().map(|token| DictNode::Str(token.to_string()));
        return DictNode::Array(nodes.collect());
    }
}

fn parse_xref(file: &mut File, offset: u64) -> DictNode {
    match file.seek(SeekFrom::Start(offset)) {
        Err(_) => panic!("couldn't seek to xref"),
        Ok(_) => (),
    };
    let file_reader = BufReader::new(file);
    let mut xref = Vec::new();
    for line in file_reader.lines().skip(2) {
        let unwrapped = line.unwrap();
        if unwrapped == "trailer" {
            break;
        }
        let vec: Vec<&str> = unwrapped.split_whitespace().collect();
        xref.push(DictNode::Str(vec[0].to_string()));
    }

    return DictNode::Array(xref);
}

fn parse_dict(buffer: &Vec<u8>) -> DictNode {
    let space = ' ' as u8;
    let mut in_dict = false;
    let mut in_array = false;
    let mut dict = HashMap::new();
    let mut values_so_far: Vec<String> = Vec::new();
    let mut array_so_far: Vec<String> = Vec::new();
    let mut last_key: Option<String> = None;

    for token in buffer.split(|byte| *byte == space) {
        let str_token = match String::from_utf8(token.to_vec()) {
            Err(_) => "non-utf8 binary".to_string(),
            Ok(s) => s,
        };
        println!("token: {}", str_token);
        if !in_dict {
            continue
        } else if str_token == "<<" {
            in_dict = true
        } else if str_token.starts_with(">>") {
            break
        } else if str_token == "[" {
            in_array = true;
            array_so_far.clear();
        } else if str_token == "]" {
            in_array = false;
            values_so_far.append(&mut array_so_far);
        } else if str_token.starts_with("/") {
            if last_key.is_some() {
                dict.insert(
                    last_key.unwrap().clone(),
                    tokens_to_dict_node(&values_so_far)
                );
            }
            values_so_far.clear();
            last_key = Some(str_token);
        } else {
            if in_array {
                array_so_far.push(str_token);
            } else {
                values_so_far.push(str_token);
            }
        }
    }

    return DictNode::Dict(dict);
}

fn get_doc_metadata(file: &mut File) -> (DictNode, DictNode) {
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
        else if found_xref && xref_offset == 0 {
            let string = match String::from_utf8(line.to_vec()) {
                Err(_) => "non-utf8 binary".to_string(),
                Ok(line) => line,
            };
            println!("xref: {}", string);
            xref_offset = string.parse::<u64>().unwrap()
        }
    }

    let trailer = parse_dict(&trailer);
    let xref = parse_xref(file, xref_offset);
    return (trailer, xref)
}

fn cat_object(file: &mut File, object_offset: u64) {
    match file.seek(SeekFrom::Start(object_offset)) {
        Err(_) => panic!("couldn't seek to object"),
        Ok(_) => (),
    };

    let newline = '\n' as u8;
    let mut found_stream = false;
    let mut obj_dict_buffer = Vec::new();
    let mut stream_buffer = Vec::new();
    let mut file_buffer = Vec::new();
    file.take(CHUNK_SIZE as u64).read_to_end(&mut file_buffer).unwrap();

    for line in file_buffer.split(|byte| *byte == newline) {
        if line == "stream".as_bytes() {
            found_stream = true;
        } else if line == "endstream".as_bytes() {
            break
        } else if !found_stream {
            obj_dict_buffer.extend_from_slice(line);
        } else {
            stream_buffer.extend_from_slice(line);
        }
    }

    let obj_dict = parse_dict(&obj_dict_buffer);
    // let stream = inflate_bytes(&stream_buffer).unwrap();
    println!("Object: {:?}", obj_dict);
    // println!("{}", from_utf8(&stream).unwrap());
}


fn cat_xref_table(file: &mut File, xref: &Vec<DictNode>) {
    for (id, offset_node) in xref.iter().skip(1).enumerate() {
        if let DictNode::Str(ref offset_str) = *offset_node {
            println!("Object ID {} @ offset {}", id + 1, offset_str);
            let offset = offset_str.parse::<u64>().unwrap();
            cat_object(file, offset);
            break;
        }
    }
}
// This is the main function
fn main() {
    let path = Path::new("i-92.pdf");
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

    if let DictNode::Array(array) = xref {
        cat_xref_table(&mut file, &array);
    }
}


mod parser;
#[test]
fn calculator() {
    assert!(parser::parse_Term("0x22").is_ok());
    assert!(parser::parse_Term("(0x2f)").is_ok());
    assert!(parser::parse_Term("((((0x4EF))))").is_ok());
    assert!(parser::parse_Term("((22)").is_err());
}
