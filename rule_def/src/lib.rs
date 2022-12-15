extern crate serde_derive;

use std::collections::HashMap;
use serde_derive::Serialize;
use std::str::FromStr;

#[derive(Clone, Serialize, Debug)]
pub struct MediaTypeRegistry {
    pub rules_registry: HashMap<String, Vec<Rule>>,
    pub child_types: HashMap<String, Vec<String>>,
}

#[derive(Clone, Serialize, Debug)]
pub enum Rule {
    Glob(GlobRule),
    Magic(MagicRule),
}

#[derive(Clone, Serialize, Debug)]
pub struct MagicRule {
    // Priority over other magic rules
    pub priority: u8,
    pub conditions: Vec<Match>,
}

#[derive(Clone, Serialize, Debug)]
pub enum Match {
    Multi(Multi),
    Single(Single),
}

impl Match {
    // Build nested match heirachies
    pub fn add_child_condition(&mut self, condition: Match) {
        // Only support child single conditions
        if let Match::Single(single_condition) = condition {
            match self {
                Match::Multi(multi) => multi.conditions.push(single_condition),
                Match::Single(single) => single.conditions.push(single_condition)
            }
        }
    }
}

#[derive(Clone, Serialize, Debug)]
pub struct Multi {
    // Minimum conditions to match
    pub min_to_match: u8,
    pub conditions: Vec<Single>,
}

#[derive(Clone, Serialize, Debug)]
pub struct Single {
    pub offset: Offset,
    // A sequence of magic bytes
    pub bytes: Vec<u8>,
    // Any OR'ed conditions with this magic
    pub conditions: Vec<Single>,
}

#[derive(Clone, Serialize, Debug)]
pub struct GlobRule {
    pub pattern: String,
    pub glob_type: GlobType,
}

#[derive(Clone, Serialize, Debug)]
pub enum GlobType {
    Regex,
    EndsWith,
    StartsWith,
    Contains,
}

#[derive(Clone, Serialize, Debug)]
pub struct Offset {
    // The byte to start at
    pub from: u32,
    // Count to check to
    pub count: u32,
}

impl Offset {
    pub fn from_attr(attr: Option<String>) -> Offset {
        match attr {
            // Default offset looks at the start of the file
            None => Offset {
                from: 0,
                count: 0,
            },
            Some(offset) => {
                match offset.split_once(":") {
                    // Some offset have a start and a count to check
                    Some((start, count)) => Offset {
                        from: u32::from_str(start).unwrap(),
                        count: u32::from_str(count).unwrap(),
                    },
                    // Others just have a start value
                    None => Offset {
                        from: u32::from_str(offset.as_str()).unwrap(),
                        count: 0,
                    }
                }
            }
        }
    }
}