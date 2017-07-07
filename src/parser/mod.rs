use pest::prelude::*;
use std::collections::HashMap;

/*
 * Parser for PDF COS object syntax. Think of COS kind of like a really
 * awkward, hard to read version of JSON.
 *
 * It is the base object structure for any semantic element in PDF. 
 *
 */

#[derive(Debug, PartialEq, Clone)]
pub enum DictNode {
    Dict(HashMap<String, DictNode>),
    Array(Vec<DictNode>),
    Str(String),
    Int(i64),
    ObjectReference(i64, i64),
}

impl_rdp! {
    grammar! {
        beginarray = { ["["] }
        endarray = { ["]"] }
        dictionary = { ["<"] ~ ["<"] ~ keypair* ~ [">"] ~ [">"] }
        keypair = { key ~ node }
        node = _{ (array | reference | string | key | int) }
        array = { beginarray ~ node+ ~ endarray }
        reference =  { int ~ int ~ ["R"] }
        key = @{ ["/"] ~ (!special ~ !whitespace ~ any)+ }
        string = @{ !int ~ (!special ~ !whitespace ~ any)+ }
        int    =  @{ ["-"]? ~ ['0'..'9']+ }
        whitespace = _{ [" "] | ["\t"] | ["\r"] | ["\n"] }
        special = { ["["] | ["]"] | (["<"] ~ ["<"]) | ([">"] ~ [">"]) | ["/"] }
    }

    process! {
        parse(&self) -> DictNode {
            (&int: int) => DictNode::Int(int.parse::<i64>().unwrap()),
            (&s: string) => DictNode::Str(s.to_string()),
            (_: reference, u1: parse(), u2: parse()) => {
                // this is fucking lame, given my grammar I know these are ints
                match (u1, u2) {
                    (DictNode::Int(a), DictNode::Int(b)) => DictNode::ObjectReference(a, b),
                    _ => unreachable!(),
                }
            },
            (_: array, _: beginarray, mut contents: _array()) => {
                contents.reverse();
                DictNode::Array(contents)
            }
        }

        _array(&self) -> Vec<DictNode> {
            (_: endarray) => Vec::new(),
            (head: parse(), mut tail: _array()) => {
                tail.push(head);
                tail
            },
        }
    }
}

#[test]
fn test_key() {
    let mut parser = Rdp::new(StringInput::new("/Hello"));
    assert!(parser.key());
    assert!(parser.end());

    let mut parser = Rdp::new(StringInput::new("\n\n /Hello\t"));
    parser.skip();
    assert!(parser.key());
    parser.skip();
    assert!(parser.end());
}

#[test]
fn test_int() {
    let mut parser = Rdp::new(StringInput::new("45678"));
    assert!(parser.int());
    assert!(parser.end());

    let mut parser = Rdp::new(StringInput::new("0"));
    assert!(parser.int());
    assert!(parser.end());

    let mut parser = Rdp::new(StringInput::new("-35"));
    assert!(parser.int());
    assert!(parser.end());
}

#[test]
fn test_string() {
    let mut parser = Rdp::new(StringInput::new("Bonjour124"));
    assert!(parser.string());
    assert!(parser.end());

    let mut parser = Rdp::new(StringInput::new("It was you Charlie!"));
    assert!(parser.string());
    parser.skip();
    assert!(parser.string());
    parser.skip();
    assert!(parser.string());
    parser.skip();
    assert!(parser.string());
    assert!(parser.end());
}

#[test]
fn test_object_reference() {
    let mut parser = Rdp::new(StringInput::new("34 0 R"));
    assert!(parser.reference());

    let queue = vec![
        Token::new(Rule::reference, 0, 6),
        Token::new(Rule::int, 0, 2),
        Token::new(Rule::int, 3, 4),
    ];
    assert_eq!(parser.queue(), &queue);
}

#[test]
fn test_array() {
    let mut parser = Rdp::new(StringInput::new("[ 342 -124 6421 ]"));
    assert!(parser.array());

    let queue = vec![
        Token::new(Rule::array, 0, 17),
        Token::new(Rule::beginarray, 0, 1),
        Token::new(Rule::int, 2, 5),
        Token::new(Rule::int, 6, 10),
        Token::new(Rule::int, 11, 15),
        Token::new(Rule::endarray, 16, 17),
    ];
    assert_eq!(parser.queue(), &queue);
}

#[test]
fn test_nested_array() {
    let mut parser = Rdp::new(StringInput::new("[ 342 [-124] ]"));
    assert!(parser.array());

    let queue = vec![
        Token::new(Rule::array, 0, 14),
        Token::new(Rule::beginarray, 0, 1),
        Token::new(Rule::int, 2, 5),
        Token::new(Rule::array, 6, 12),
        Token::new(Rule::beginarray, 6, 7),
        Token::new(Rule::int, 7, 11),
        Token::new(Rule::endarray, 11, 12),
        Token::new(Rule::endarray, 13, 14),
    ];
    assert_eq!(parser.queue(), &queue);
}


#[test]
fn test_keypair() {
    let mut parser = Rdp::new(StringInput::new("/Size 65"));
    assert!(parser.keypair());

    let queue = vec![
        Token::new(Rule::keypair, 0, 8),
        Token::new(Rule::key, 0, 5),
        Token::new(Rule::int, 6, 8),
    ];
    assert_eq!(parser.queue(), &queue);
}


#[test]
fn test_key_keypair() {
    // weirdly this is valid syntax in cos, equivalent to:
    // { Type: "/Font", Subtype: "/TrueType" }
    let mut parser = Rdp::new(StringInput::new("/Type/Font/Subtype/TrueType"));
    assert!(parser.keypair());
    assert!(parser.keypair());
    assert!(parser.end());

    let queue = vec![
        Token::new(Rule::keypair, 0, 10),
        Token::new(Rule::key, 0, 5),
        Token::new(Rule::key, 5, 10),
        Token::new(Rule::keypair, 10, 27),
        Token::new(Rule::key, 10, 18),
        Token::new(Rule::key, 18, 27),
    ];
    assert_eq!(parser.queue(), &queue);
}

#[test]
fn test_dictionary() {
    let dict = "<< /Length 5 0 R /Filter /FlateDecode >>";
    let mut parser = Rdp::new(StringInput::new(dict));
    assert!(parser.dictionary());
    assert!(parser.end());
    let queue = vec![
        Token::new(Rule::dictionary, 0, 40),
        Token::new(Rule::keypair, 3, 16),
        Token::new(Rule::key, 3, 10),
        Token::new(Rule::reference, 11, 16),
        Token::new(Rule::int, 11, 12),
        Token::new(Rule::int, 13, 14),
        Token::new(Rule::keypair, 17, 37),
        Token::new(Rule::key, 17, 24),
        Token::new(Rule::key, 25, 37),
    ];
    assert_eq!(parser.queue(), &queue);
}

#[test]
fn test_dictionary_with_array() {
    let dict = r#"
        << /Size 65 /Root 35 0 R /Info 1 0 R 
        /ID
        [<d83abc5b1b9bea6e1b372681e568f886><d83abc5b1b9bea6e1b372681e568f886>]
        >>
    "#;
    let mut parser = Rdp::new(StringInput::new(dict));
    parser.skip();
    assert!(parser.dictionary());
    parser.skip();
    assert!(parser.end());
}

#[test]
fn test_complex_dictionary() {
    let dict = r#"
        <</Type/FontDescriptor/FontName/CAAAAA+TimesNewRomanPSMT
        /Flags 6
        /FontBBox[-568 -306 2000 1007]/ItalicAngle 0
        /Ascent 891
        /Descent -216
        /CapHeight 1006
        /StemV 80
        /FontFile2 8 0 R
        >>
    "#;
    let mut parser = Rdp::new(StringInput::new(dict));
    parser.skip();
    assert!(parser.dictionary());
    parser.skip();
    assert!(parser.end());
}

#[test]
fn test_parsing_atoms() {
    let mut parser = Rdp::new(StringInput::new("56"));
    assert!(parser.int());
    let node = parser.parse();
    assert_eq!(node, DictNode::Int(56));

    let mut parser = Rdp::new(StringInput::new("Bonjour"));
    assert!(parser.string());
    let node = parser.parse();
    assert_eq!(node, DictNode::Str("Bonjour".to_string()));
}

#[test]
fn test_parsing_refs() {
    let mut parser = Rdp::new(StringInput::new("30 0 R"));
    assert!(parser.reference());
    let node = parser.parse();
    assert_eq!(node, DictNode::ObjectReference(30, 0));
}

#[test]
fn test_parsing_array() {
    let mut parser = Rdp::new(StringInput::new("[ 759 Something ]"));
    assert!(parser.array());
    let node = parser.parse();
    assert_eq!(node, DictNode::Array([
        DictNode::Int(759),
        DictNode::Str("Something".to_string())
    ].to_vec()));
}
