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
    ObjectReference(i64, i64),
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
}

impl_rdp! {
    grammar! {
        begindict = { ["<"] ~ ["<"] }
        enddict = { [">"] ~ [">"] }
        beginarray = { ["["] }
        endarray = { ["]"] }
        dictionary = {  begindict ~ keypair* ~ enddict }
        keypair = { key ~ node }
        node = _{ (array | reference | string |key | int | float | boolean | dictionary) }
        array = { beginarray ~ node* ~ endarray }
        reference =  { int ~ int ~ ["R"] }
        key = @{ ["/"] ~ (!special ~ !whitespace ~ any)+ }
        string = @{ (["("] ~ acceptable_string* ~ [")"]) | (["<"] ~ acceptable_string+ ~ [">"])}
        acceptable_string = _{ (whitespace | ["/"] | ['a'..'z'] | ['A'..'Z'] | ['0'..'9'] | [":"] | ["."] | ["@"] | ["'"] ) }
        int =  @{ !float ~ ["-"]? ~ ['0'..'9']+ }
        float =  @{ ["-"]? ~ ['0'..'9']+ ~ ["."] ~ ['0'..'9']* }
        boolean = @{ ["true"] | ["false"] }
        whitespace = _{ [" "] | ["\t"] | ["\r"] | ["\n"] | ["endobj"] }
        special = { beginarray | begindict | endarray | enddict | ["\\"]| ["/"] | ["("] | [")"] }
    }

    process! {
        parse(&self) -> DictNode {
            (&int: int) => DictNode::Int(int.parse::<i64>().unwrap()),
            (&float: float) => DictNode::Float(float.parse::<f64>().unwrap()),
            (&s: string) => DictNode::Str(s.to_string()),
            (&b: boolean) => DictNode::Bool(b.parse::<bool>().unwrap()),
            (&k: key) => DictNode::Str(k.to_string()),
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
            },
            (_: dictionary, _: begindict, mut contents: _dict()) => {
                DictNode::Dict(contents)
            }
        }

        _array(&self) -> Vec<DictNode> {
            (_: endarray) => Vec::new(),
            (head: parse(), mut tail: _array()) => {
                tail.push(head);
                tail
            },
        }

        _dict(&self) -> HashMap<String, DictNode> {
            (_: enddict) => HashMap::new(),
            (_: keypair, &key: key, value: parse(), mut tail: _dict()) => {
                tail.insert(key[1..].to_string(), value);
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
fn test_float() {
    let mut parser = Rdp::new(StringInput::new("3.14"));
    assert!(parser.float());
    assert!(parser.end());

    let mut parser = Rdp::new(StringInput::new("-214.946"));
    assert!(parser.float());
    assert!(parser.end());

    let mut parser = Rdp::new(StringInput::new("0.02"));
    assert!(parser.float());
    assert!(parser.end());
}

#[test]
fn test_string() {
    let mut parser = Rdp::new(StringInput::new("(A)"));
    assert!(parser.string());
    assert!(parser.end());

    let mut parser = Rdp::new(StringInput::new("<d83abc5b1b9bea6e1b372681e568f886><d83abc5b1b9bea6e1b372681e568f886>"));
    assert!(parser.string());
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
fn test_empty_array() {
    let mut parser = Rdp::new(StringInput::new("[  ]"));
    assert!(parser.array());

    let queue = vec![
        Token::new(Rule::array, 0, 4),
        Token::new(Rule::beginarray, 0, 1),
        Token::new(Rule::endarray, 3, 4),
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
    // { "Type": "/Font", "Subtype": "/TrueType" }
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
        Token::new(Rule::begindict, 0, 2),
        Token::new(Rule::keypair, 3, 16),
        Token::new(Rule::key, 3, 10),
        Token::new(Rule::reference, 11, 16),
        Token::new(Rule::int, 11, 12),
        Token::new(Rule::int, 13, 14),
        Token::new(Rule::keypair, 17, 37),
        Token::new(Rule::key, 17, 24),
        Token::new(Rule::key, 25, 37),
        Token::new(Rule::enddict, 38, 40),
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

    let mut parser = Rdp::new(StringInput::new("(Bonjour)"));
    assert!(parser.string());
    let node = parser.parse();
    assert_eq!(node, DictNode::Str("(Bonjour)".to_string()));

    let mut parser = Rdp::new(StringInput::new("true"));
    assert!(parser.boolean());
    let node = parser.parse();
    assert_eq!(node, DictNode::Bool(true));
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
    let mut parser = Rdp::new(StringInput::new("[ 759 -124 ]"));
    assert!(parser.array());
    let node = parser.parse();
    assert_eq!(node, DictNode::Array([
        DictNode::Int(759),
        DictNode::Int(-124)
    ].to_vec()));
}

#[test]
fn test_parsing_dict() {
    let dict = "<< /Length 5 0 R /Filter /FlateDecode >>";
    let corresponding_map = hashmap!{
        "Length".to_string() => DictNode::ObjectReference(5, 0),
        "Filter".to_string() => DictNode::Str("/FlateDecode".to_string()),
    };
    let mut parser = Rdp::new(StringInput::new(dict));
    assert!(parser.dictionary());
    let node = parser.parse();
    assert_eq!(node, DictNode::Dict(corresponding_map));
}


#[test]
fn test_parsing_complex_dictionary() {
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
    let bounding_box = DictNode::Array([
        DictNode::Int(-568),
        DictNode::Int(-306),
        DictNode::Int(2000),
        DictNode::Int(1007),
    ].to_vec());
    let corresponding_map = hashmap!{
        "Type".to_string() => DictNode::Str("/FontDescriptor".to_string()),
        "FontName".to_string() => DictNode::Str("/CAAAAA+TimesNewRomanPSMT".to_string()),
        "Flags".to_string() => DictNode::Int(6),
        "FontBBox".to_string() => bounding_box,
        "ItalicAngle".to_string() => DictNode::Int(0),
        "Ascent".to_string() => DictNode::Int(891),
        "Descent".to_string() => DictNode::Int(-216),
        "CapHeight".to_string() => DictNode::Int(1006),
        "StemV".to_string() => DictNode::Int(80),
        "FontFile2".to_string() => DictNode::ObjectReference(8, 0),
    };
    let mut parser = Rdp::new(StringInput::new(dict));
    parser.skip();
    assert!(parser.dictionary());
    let node = parser.parse();
    assert_eq!(node, DictNode::Dict(corresponding_map));
}


#[test]
fn test_parsing_real_world_dictionary() {
    let dict = "<</Type/Page/Parent 7 0 R/Resources 24 0 \
               R/MediaBox[0 0 612 792]/Annots[4 0 R 5 0 R \
               6 0 R ]/Group<</S/Transparency/CS/DeviceRGB/I \
               true>>/Contents 2 0 R>>";
    let mut parser = Rdp::new(StringInput::new(dict));
    assert!(parser.dictionary());
}

#[test]
fn test_parsing_uri_value() {
    let dict = "<</Type/Annot/Subtype/Link/Border[0 0 0] \
                /Rect[92.5 701.5 236.8 714.2]/A<</Type \
                /Action/S/URI/URI(mailto:human@alumni.ubc.ca)>> \
                >>";
    let mut parser = Rdp::new(StringInput::new(dict));
    assert!(parser.dictionary());
}

#[test]
fn test_whitespace_value() {
    let dict = "<</Producer(GNU Ghostscript 7.05)>>";
    let mut parser = Rdp::new(StringInput::new(dict));
    assert!(parser.dictionary());
}

#[test]
fn test_floating_point_in_dict() {
    let dict = "<</Type/ExtGState/Name/R4/TR/Identity/OPM 1/SM 0.02>>";
    let mut parser = Rdp::new(StringInput::new(dict));
    assert!(parser.dictionary());
}

#[test]
fn test_special_chars_in_string() {
    let dict = "<</Flags(/fi/fl/foo)>>";
    let mut parser = Rdp::new(StringInput::new(dict));
    assert!(parser.dictionary());
}
