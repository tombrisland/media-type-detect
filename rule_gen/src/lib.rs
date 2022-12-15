extern crate rule_def;

use rule_def::*;

// Include generated code for media type rules
pub fn load_type_registry() -> MediaTypeRegistry {
    return include!(concat!(env!("OUT_DIR"), "/rules.rs"));
}