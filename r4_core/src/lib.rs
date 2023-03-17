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


pub struct DetectorConfig {
    pub enable_glob: bool,
    pub enable_magic: bool,
    pub prioritise_glob: bool,
    pub max_concurrency: u8,
    pub default_type: &'static str,
}

pub struct BinarySignature {}

pub struct 

pub struct R4 {
    pub registry: MediaTypeRegistry,

    pub config: DetectorConfig,
}

impl R4 {
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

}