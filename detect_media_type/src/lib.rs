#![feature(let_chains)]

extern crate core;

use std::ffi::OsStr;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use log::debug;

use rule_def::MediaTypeRegistry;

use crate::glob::run_glob;
use crate::magic::run_magic;

mod magic;
mod glob;

const EXTENSION_DOT: &str = ".";

pub struct DetectorConfig {
    pub enable_glob: bool,
    pub enable_magic: bool,
    pub prioritise_glob: bool,
    pub max_concurrency: u8,
    pub default_type: &'static str,
}

pub struct MediaTypeDetector {
    pub registry: MediaTypeRegistry,

    pub config: DetectorConfig,
}

impl MediaTypeDetector {
    // Detect the media type of a file on the file system
    pub fn detect_file_type(&self, path: &Path) -> Option<String> {
        let file_name: Option<String> = path.file_name()
            // Convert to String representation
            .map(|name: &OsStr| name.to_string_lossy().into_owned());

        // Buffer to hold a portion of the file to test magic against
        // TODO generate a const value in build.rs to be the max size of this vec
        let mut buf: Vec<u8> = vec![0; 1024];

        File::open(&path).map(|mut file: File| {
            file.read(buf.as_mut_slice()).expect("Failed to read bytes from file");

            self.detect_type(&file_name, buf.as_slice())
        }).unwrap()
    }

    // TODO recursive child types

    fn detect_type(
        &self,
        resource_name: &Option<String>,
        buf: &[u8],
    ) -> Option<String> {
        // Run all rules and return possible matches
        let matches: Vec<String> = self.run_magic_rules(buf);

        // Try to narrow down matches via glob rules
        if let Some(resource_name) = resource_name {
            let glob_match: Option<String> = self.run_glob_rules(resource_name);

            if glob_match.is_some() {
                return glob_match;
            }
        }

        // The last result will be the best match
        matches.last().map(|media_type| media_type.clone())
    }

    fn is_sub_type(&self, parent: &String, potential_child: &String) -> bool {
        match self.registry.sub_types.get(parent) {
            Some(types) => {
                if types.contains(potential_child) {
                    true
                } else {
                    // Recurse through the children of this parent
                    types.iter().any(|child| self.is_sub_type(child, potential_child))
                }
            }
            None => false
        }
    }

    fn run_magic_rules(
        &self,
        buf: &[u8],
    ) -> Vec<String> {
        let mut possible_types: Vec<String> = vec![];

        // The priority of the last match
        let mut last_match_priority: u8 = 0;

        for magic_rule in &self.registry.magic_rules {
            let media_type: &String = &magic_rule.media_type;

            if magic_rule.priority < last_match_priority {
                // None of the following clauses will have higher priority
                return possible_types;
            }

            // Skip rule if we've already matched a child type
            if possible_types.iter().any(|possible|
                self.is_sub_type(media_type, possible)) {
                debug!("Skipping check for {} as there's already a more specific match", media_type);

                continue;
            }

            // TODO order rules in order of priority - highest to lowest
            // Then stop iteration once we hit the first priority lower than one we've matched
            // TODO if isn't child + higher prio match don't run it

            // If the magic succeeds add to the possible types list
            if run_magic(buf, magic_rule) {
                possible_types.push(magic_rule.media_type.clone());

                last_match_priority = magic_rule.priority;
            }
        }

        possible_types
    }

    fn run_glob_rules(&self, resource_name: &String) -> Option<String> {
        // TODO parsing of HTTP paths etc
        // Only bother if the filename has an extension

        resource_name.split_once(EXTENSION_DOT).and_then(|(_, _ext)| {
            for glob_rule in &self.registry.glob_rules {
                let media_type: &String = &glob_rule.media_type;

                if run_glob(resource_name, glob_rule) {
                    return Some(media_type.clone());
                }
            }

            None
        })
    }
}