use pest::prelude::*;

/*
 * Parser for PDF X-Ref table. The X-Ref table is basically a table of contents of
 * indirect object (or XObjects), storing their exact byte offset in a file.
 *
 * It's analogous to heap allocated memory. For example, imagine if I have a 30 page
 * PhD thesis with the same picture of my cat in each header. Rather than storing the
 * image in each page object, the page instructions will make an indirect reference to
 * the cat 'XObject', then the parser will consult the xref table to pull it out.
 */

#[derive(Debug, PartialEq, Clone)]
pub struct XRefTable(Vec<XRefEntry>);

#[derive(Debug, PartialEq, Clone)]
pub struct XRefEntry {
    offset: u64,
    generation_id: u64,
    is_free: bool,
}

impl_rdp! {
    grammar! {
        xref = { xref_begin ~ xref_header ~ xref_entry+ ~ xref_end }
        xref_begin = { newline* ~ ["xref\n"] }
        xref_end = { newline* ~ ["trailer\n"] }
        xref_header = { newline* ~ int ~ int ~ newline }
        xref_entry = { newline* ~ int ~ int ~ usage ~ newline }
        usage = @{ ["f"] | ["n"] }  // f == free n == in-use
        int = @{ ['0'..'9']+ }
        whitespace = _{ [" "] | ["\t"] }
        newline = _{ ["\n"] }
   }

   process! {
        parse(&self) -> XRefTable {
            (mut entries: _parse_xref()) => {
                entries.reverse();
                XRefTable(entries)
            }
        }

        _parse_xref(&self) -> Vec<XRefEntry> {
            (_: xref, _: xref_begin, tail: _parse_xref()) => tail,
            (_: xref_header, _: int, _:int, tail: _parse_xref()) => tail,
            (_: xref_end) => Vec::new(),
            (entry: _parse_xref_entry(), mut tail: _parse_xref()) => {
                tail.push(entry);
                tail
            },
        }

        _parse_xref_entry(&self) -> XRefEntry {
            (_: xref_entry, &o: int, &g: int, &u: usage) => {
                XRefEntry{
                    offset: o.parse::<u64>().unwrap(),
                    generation_id: g.parse::<u64>().unwrap(),
                    is_free: u == "f"
                }
            }
        }

   }
}

#[test]
fn test_parsing_int() {
    let mut parser = Rdp::new(StringInput::new("0"));
    assert!(parser.int());
    assert!(parser.end());

    let mut parser = Rdp::new(StringInput::new("0000118424"));
    assert!(parser.int());
    assert!(parser.end());
}

#[test]
fn test_parsing_xref_elements() {
    let mut parser = Rdp::new(StringInput::new("1 2 f \n"));
    assert!(parser.xref_entry());
    assert!(parser.end());

    let queue = vec![
        Token::new(Rule::xref_entry, 0, 7),
        Token::new(Rule::int, 0, 1),
        Token::new(Rule::int, 2, 3),
        Token::new(Rule::usage, 4, 5),
    ];
    assert_eq!(parser.queue(), &queue);

    let mut parser = Rdp::new(StringInput::new("xref\n"));
    assert!(parser.xref_begin());
    assert!(parser.end());

    let mut parser = Rdp::new(StringInput::new("trailer\n"));
    assert!(parser.xref_end());
    assert!(parser.end());

    let mut parser = Rdp::new(StringInput::new("0 65\n"));
    assert!(parser.xref_header());
    assert!(parser.end());
}


#[test]
fn test_parsing_xref() {
    let xref = "\n    xref\n  0 65\n 0000000000 65535 f\n 0000118424 00000 n\ntrailer\n";
    let expected_xref = XRefTable([
        XRefEntry{ offset: 0, generation_id: 65535, is_free: true},
        XRefEntry{ offset: 118424, generation_id: 0, is_free: false},
    ].to_vec());

    let mut parser = Rdp::new(StringInput::new(xref));
    parser.skip();
    assert!(parser.xref());
    let xref = parser.parse();
    assert_eq!(xref, expected_xref);
}
