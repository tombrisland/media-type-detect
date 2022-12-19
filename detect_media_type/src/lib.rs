#![feature(let_chains)]

use std::ffi::OsStr;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use log::{debug, info};

use rule_def::{MediaTypeRegistry, Rule};

use crate::glob::run_glob;
use crate::magic::run_magic;

mod magic;
mod glob;

pub fn detect_file_type(path: &Path, media_type_registry: &MediaTypeRegistry) -> Option<String> {
    let file_name: Option<String> = path.file_name()
        // Convert to String representation
        .map(|name: &OsStr| name.to_string_lossy().into_owned());

    let mut buf: Vec<u8> = vec![0; 1024];

    File::open(&path).map(|mut file: File| {
        file.read(buf.as_mut_slice()).expect("Failed to read bytes from file");

        detect_type(&file_name, buf.as_slice(), media_type_registry)
    }).unwrap()
}

fn detect_type(
    file_name: &Option<String>,
    buf: &[u8],
    registry: &MediaTypeRegistry,
) -> Option<String> {
    match run_rules(file_name, buf, &registry, &registry.root_types) {
        Some(parent_type) => {
            // Are there any children of this type?
            let child_type: Option<String> = registry.sub_types.get(&parent_type)
                .and_then(|children| run_rules(file_name, buf, registry, children));

            // If no child was found return parent
            child_type.or(Some(parent_type))
        }
        // Nothing detected
        None => None
    }
}

fn run_rules(
    file_name: &Option<String>,
    buf: &[u8],
    type_registry: &MediaTypeRegistry,
    types: &Vec<String>) -> Option<String> {
    for media_type in types {
        debug!("Checking rule for {}", media_type);

        if let Some(rule_set) = type_registry.rules_registry.get(media_type) {
            if run_rule_set(file_name, buf, rule_set) {
                info!("Matched item as media type {}", media_type);

                return Some(media_type.clone());
            }
        }
    }

    // No rules matched
    None
}

// TODO will glob be useful - is there an extension?
// TODO config - trust extension?

fn run_rule_set(file_name: &Option<String>, buf: &[u8], rules: &Vec<Rule>) -> bool {
    return rules.iter()
        // Iterate each rule within the media type
        .map(|rule| match rule {
            // Check if the filename glob matches
            Rule::Glob(glob) => run_glob(file_name, &glob),
            Rule::Magic(magic) => run_magic(buf, &magic),
        })
        .any(|result| result == true);
}
