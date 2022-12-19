#![feature(let_chains)]
extern crate core;
extern crate rule_def;
extern crate xml;

use std::{env, u8};
use std::collections::{HashMap, VecDeque};
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use xml::attribute::OwnedAttribute;
use xml::EventReader;
use xml::name::OwnedName;
use xml::reader::XmlEvent;

use rule_def::{GlobRule, GlobType, MagicRule, Match, MediaTypeRegistry, Multi, Offset, Rule, Single};

const TIKA_MIMETYPES_PATH: &str = "./tika-mimetypes.xml";

const RULE_REGISTRY_FILE: &str = "rule_registry.rs";

type XmlElement = (OwnedName, Vec<OwnedAttribute>);

fn main() {
    let out_dir: String = env::var("OUT_DIR").unwrap();

    // Read in tika-mimetypes.xml
    let reader: BufReader<File> =
        BufReader::new(File::open(TIKA_MIMETYPES_PATH)
            .expect("Could not find tika-mimetypes.xml"));

    // Initialise parser and extract rules from XML
    let type_registry: MediaTypeRegistry = parse_xml_rules(EventReader::new(reader));

    let path: PathBuf = Path::new(&out_dir).join(RULE_REGISTRY_FILE);

    let mut open_options: OpenOptions = OpenOptions::new();
    open_options.write(true).append(false);

    // Create a new file for the output or truncate existing
    let out_file: File = File::create(path.as_path())
        .expect("Failed to create output file for rust source");

    let mut writer: BufWriter<File> = BufWriter::new(out_file);

    let output_string: String = uneval::to_string(type_registry)
        .expect("Failed to serialise rule registry as rust source");

    writer.write_all(output_string.as_bytes()).unwrap();
}

const MIME_TYPE_ELEMENT: &str = "mime-type";
const MIME_TYPE_FIELD: &str = "type";

const GLOB_ELEMENT: &str = "glob";
const PATTERN_FIELD: &str = "pattern";
const ASTERISK: &str = "*";
const IS_REGEX_FIELD: &str = "isregex";

const MAGIC_ELEMENT: &str = "magic";
const PRIORITY_FIELD: &str = "priority";

const MATCH_ELEMENT: &str = "match";
const OFFSET_FIELD: &str = "offset";
const VALUE_FIELD: &str = "value";
const MIN_SHOULD_MATCH_FIELD: &str = "minShouldMatch";

const SUB_CLASS_ELEMENT: &str = "sub-class-of";

const HEX_PREFIX: &str = "0x";

const BACKSLASH_INT: u8 = 92;
const BACKSLASH_CHAR: char = '\\';

fn parse_match(value: &String) -> Vec<u8> {
    // Value must be a single hex value
    if value.starts_with(HEX_PREFIX) {
        let (_, hex_str) = value.split_once(HEX_PREFIX).unwrap();

        let x: Vec<u8> = (0..hex_str.len())
            .step_by(2)
            .map(|idx| u8::from_str_radix(&hex_str[idx..idx + 2], 16).unwrap())
            .collect();

        // TODO match every hex byte just by knowing the length
        return x;
    }

    let chars: Vec<char> = value.chars().collect();

    let mut decoded_bytes: Vec<u8> = vec![];

    let mut idx: usize = 0;
    let max: usize = chars.len();

    while idx < max {
        let current: &char = chars.get(idx).unwrap();

        if current == &BACKSLASH_CHAR {
            match chars.get(idx + 1).unwrap() {
                // If we have a double backslash
                &BACKSLASH_CHAR => {
                    // Decode as a single
                    decoded_bytes.push(BACKSLASH_INT);
                    idx += 1;
                }
                // Decode following characters as a hex value
                &'x' => {
                    let hex_str: String = chars.as_slice()[idx + 2..idx + 4].iter().collect();
                    let number: u8 = u8::from_str_radix(&hex_str, 16).unwrap();

                    decoded_bytes.push(number);

                    idx += 3;
                }
                // TODO impl the other mental formats
                _ => {}
            };
        } else {
            for b in current.to_string().as_bytes() {
                decoded_bytes.push(*b);
            }
        }
        idx += 1;
    }

    decoded_bytes
}

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
            let string: String = extract_xml_field(&attributes, VALUE_FIELD).unwrap();

            Match::Single(Single {
                offset: Offset::from_attr(extract_xml_field(&attributes, OFFSET_FIELD)),
                bytes: parse_match(&string),
                conditions: vec![],
            })
        }
    }
}

const EMPTY: &str = "";

// Create a glob condition from an XML attribute
fn create_glob_rule(attributes: &Vec<OwnedAttribute>) -> GlobRule {
    // Mandatory field on all glob entries
    let pattern: String = extract_xml_field(&attributes, PATTERN_FIELD).unwrap();

    let glob_type = match extract_xml_field(&attributes, IS_REGEX_FIELD) {
        Some(_) => GlobType::Regex,
        // Default to contains if field unset
        _ => {
            // Use position of asterisks to determine GlobType
            if pattern.starts_with(ASTERISK) {
                if pattern.ends_with(ASTERISK) {
                    GlobType::Contains
                } else {
                    GlobType::EndsWith
                }
            } else {
                GlobType::StartsWith
            }
        }
    };

    GlobRule {
        // Asterisk behaviour is covered by GlobType
        pattern: pattern.replace(ASTERISK, EMPTY),
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

fn parse_xml_rules(event_reader: EventReader<BufReader<File>>) -> MediaTypeRegistry {
    let mut rules_registry: HashMap<String, Vec<Rule>> = Default::default();
    let mut root_types: Vec<String> = vec![];
    let mut sub_types: HashMap<String, Vec<String>> = Default::default();

    // Parent XML elements
    let mut elements: VecDeque<XmlElement> = Default::default();

    // Temporary store of rules per media type
    let mut curr_rules: Vec<Rule> = vec![];
    let mut curr_magic: Option<MagicRule> = None;
    // If this type has a parent
    let mut curr_parent: Option<String> = None;

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
                    // Add a relationship into the children map
                    SUB_CLASS_ELEMENT => curr_parent = Some(extract_xml_field(&attributes, MIME_TYPE_FIELD).unwrap()),
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

                        if let Some(parent) = curr_parent.clone() {
                            // Insert a new entry or add to the existing one
                            if let Some(children) = sub_types.get_mut(&parent) {
                                children.push(media_type.clone());
                            } else {
                                sub_types.insert(parent, vec![media_type.clone()]);
                            }
                        } else {
                            // No parent makes this a root type
                            root_types.push(media_type.clone());
                        }

                        rules_registry.insert(media_type, curr_rules.clone());

                        // Clear rules for the next entry
                        curr_rules.clear();
                        curr_parent = None;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    };

    MediaTypeRegistry {
        root_types,
        sub_types,

        rules_registry,
    }
}

// Retrieve the value of an XML field from an attribute
fn extract_xml_field(attributes: &Vec<OwnedAttribute>, key: &str) -> Option<String> {
    attributes.iter().find(|attr| attr.name.local_name == key).map(|attr| attr.value.clone())
}