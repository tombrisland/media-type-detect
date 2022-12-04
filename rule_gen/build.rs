#![feature(let_chains)]
extern crate rule_def;
extern crate xml;

use std::collections::VecDeque;
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use xml::attribute::OwnedAttribute;
use xml::EventReader;
use xml::name::OwnedName;
use xml::reader::XmlEvent;

use rule_def::{GlobRule, GlobType, MagicRule, Match, MediaTypeRule, Multi, Offset, Rule, Single};

const TIKA_MIMETYPES_PATH: &str = "./tika-mimetypes.xml";
const OUT_FILE: &str = "rules.rs";

// The un-eval library sometimes adds unnecessary snippets
const REMOVE_FROM_OUTPUT: [&str; 1] = [
    ".into_iter().collect()"
];
const EMPTY: &str = "";

type XmlElement = (OwnedName, Vec<OwnedAttribute>);

fn main() {
    let out_dir: String = env::var("OUT_DIR").unwrap();

    // Read in tika-mimetypes.xml
    let reader: BufReader<File> =
        BufReader::new(File::open(TIKA_MIMETYPES_PATH)
            .expect("Could not find tika-mimetypes.xml"));

    // Initialise parser and extract rules from XML
    let media_type_rules: Vec<MediaTypeRule> = parse_xml_rules(EventReader::new(reader));

    let path: PathBuf = Path::new(&out_dir).join(OUT_FILE);

    let mut open_options: OpenOptions = OpenOptions::new();
    open_options.write(true).append(false);

    // Create a new file for the output or truncate existing
    let out_file: File = File::create(path.as_path())
        .expect("Failed to create output file for rust source");

    let mut writer: BufWriter<File> = BufWriter::new(out_file);

    let mut output_string: String = uneval::to_string(media_type_rules)
        .expect("Failed to serialise rules as rust source");

    // Remove any extra code to reduce binary size
    for remove_str in REMOVE_FROM_OUTPUT {
        output_string = output_string.replace(remove_str, EMPTY);
    }

    writer.write_all(output_string.as_bytes()).unwrap();
}

const MIME_TYPE_ELEMENT: &str = "mime-type";
const MIME_TYPE_FIELD: &str = "type";

const GLOB_ELEMENT: &str = "glob";
const PATTERN_FIELD: &str = "pattern";
const IS_REGEX_FIELD: &str = "isregex";

const MAGIC_ELEMENT: &str = "magic";
const PRIORITY_FIELD: &str = "priority";

const MATCH_ELEMENT: &str = "match";
const OFFSET_FIELD: &str = "offset";
const VALUE_FIELD: &str = "value";
const MIN_SHOULD_MATCH_FIELD: &str = "minShouldMatch";

// Create a match condition from an XML attribute
fn create_match_condition(attributes: &Vec<OwnedAttribute>) -> Match {
    match extract_xml_field(&attributes, MIN_SHOULD_MATCH_FIELD) {
        // Some match clauses have a minShouldMatch condition with nested match statements
        Some(str) => {
            let min_to_match: u8 = u8::from_str(str.as_str()).unwrap();

            Match::Multi(Multi { min_to_match, conditions: vec![] })
        }
        // Regular magic condition
        None => {
            let bytes: Vec<u8> =
                extract_xml_field(&attributes, VALUE_FIELD).unwrap().as_bytes().to_vec();

            Match::Single(Single {
                offset: Offset::from_attr(extract_xml_field(&attributes, OFFSET_FIELD)),
                bytes,
                conditions: vec![],
            })
        }
    }
}

// Create a glob condition from an XML attribute
fn create_glob_rule(attributes: &Vec<OwnedAttribute>) -> GlobRule {
    let glob_type = match extract_xml_field(&attributes, IS_REGEX_FIELD) {
        Some(_) => GlobType::Regex,
        // Default to equals if field unset
        _ => GlobType::Equals
    };

    GlobRule {
        // Mandatory field on all glob entries
        pattern: extract_xml_field(&attributes, PATTERN_FIELD).unwrap(),
        glob_type,
    }
}

// Create a magic rule from an XML attribute and any nested match blocks
fn create_magic_rule(attributes: &Vec<OwnedAttribute>) -> MagicRule {
    let priority: u8 = match extract_xml_field(&attributes, PRIORITY_FIELD) {
        Some(str) => u8::from_str(str.as_str()).unwrap(),
        // Default priority to zero if not populated
        None => 0
    };

    MagicRule { priority, conditions: vec![] }
}

fn parse_xml_rules(event_reader: EventReader<BufReader<File>>) -> Vec<MediaTypeRule> {
    let mut media_type_rules: Vec<MediaTypeRule> = vec![];

    // Parent XML elements
    let mut elements: VecDeque<XmlElement> = Default::default();

    // Temporary store of rules per media type
    let mut curr_rules: Vec<Rule> = vec![];
    let mut curr_magic: Option<MagicRule> = None;

    // Parent nested match blocks
    let mut nested_match_blocks: Vec<Match> = vec![];

    for event in event_reader {
        match event {
            Ok(XmlEvent::StartElement { name, attributes, .. }) => {
                match name.local_name.as_str() {
                    // Glob rules can be added immediately
                    GLOB_ELEMENT => curr_rules.push(Rule::Glob(create_glob_rule(&attributes))),
                    // Push match elements onto the stack to support deep nesting
                    MATCH_ELEMENT => nested_match_blocks.push(create_match_condition(&attributes)),
                    // Create a magic entry to add nested rules onto
                    MAGIC_ELEMENT => curr_magic = Some(create_magic_rule(&attributes)),
                    _ => {}
                }

                // Makes it easier to search for direct parents
                elements.push_front((name, attributes));
            }
            Ok(XmlEvent::EndElement { .. }) => {
                // There must be an element on the stack
                let (name, attributes) = elements.pop_front().unwrap();

                match name.local_name.as_str() {
                    // When the match is closed we know there is no more nesting
                    MATCH_ELEMENT => {
                        let current_match: Match = nested_match_blocks.pop().unwrap();

                        // Check if there is a direct parent match element
                        if let Some((parent_name, ..)) = elements.front() && parent_name.local_name == MATCH_ELEMENT {
                            let parent_match: &mut Match = nested_match_blocks.last_mut().unwrap();

                            parent_match.add_child_condition(current_match);
                        } else {
                            // This entry must be directly under a Magic block so add it
                            if let Some(magic) = &mut curr_magic {
                                magic.conditions.push(current_match);
                            }
                        }
                    }
                    // Add this completed magic block to the rules
                    MAGIC_ELEMENT => {
                        curr_rules.push(Rule::Magic(curr_magic.unwrap()));

                        curr_magic = None;
                    }
                    // Once we've collected all the rules for this mime-type
                    MIME_TYPE_ELEMENT => {
                        // Mime type is a mandatory field
                        let media_type: String = extract_xml_field(&attributes, MIME_TYPE_FIELD).unwrap();

                        media_type_rules.push(MediaTypeRule { media_type, rules: curr_rules.clone() });

                        // Clear rules for the next entry
                        curr_rules.clear();
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    };

    media_type_rules
}

// Retrieve the value of an XML field from an attribute
fn extract_xml_field(attributes: &Vec<OwnedAttribute>, key: &str) -> Option<String> {
    attributes.iter().find(|attr| attr.name.local_name == key).map(|attr| attr.value.clone())
}