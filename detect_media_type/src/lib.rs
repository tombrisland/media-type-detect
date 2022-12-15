use std::cmp::min;

use log::{info, Level, Metadata, Record};

use rule_def::{Match, MediaTypeRegistry, Rule};
use rule_gen::load_type_registry;

pub struct Logger;

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!("{} - {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}

static NO_TYPE_FILTER: Vec<String> = vec![];

fn detect_type(file_name: &String, registry: &MediaTypeRegistry) {
    // TODO can't recurse
    match run_all_rules(file_name, &registry) {
        Some(parent_type) => {
            // Are there any children of this type?
            if let Some(children) = registry.child_types.get(&parent_type) {
                run_filtered_rules(file_name, registry, children);
            } else {
                Some(parent_type)
            }
        }
        None => None
    }
}

fn run_rule_set(file_name: &String, rules: &Vec<Rule>) -> bool {
    return rules.iter()
        // Iterate each rule within the media type
        .map(|rule| match rule {
            // Check if the filename glob matches
            Rule::Glob(glob) => file_name.contains(&glob.pattern),
            Rule::Magic(_) => false,
        })
        .any(|result| result == true);
}

fn run_filtered_rules(file_name: &String, registry: &MediaTypeRegistry, type_filter: &Vec<String>) -> Option<String> {
    for media_type in type_filter {
        if let Some(rule_set) = registry.rules_registry.get(media_type) {
            info!("Checking rule for {}", media_type);

            if run_rule_set(&file_name, rule_set) {
                info!("Matched item as media type {}", media_type);

                return Some(media_type.clone());
            }
        }
    }

    // No match found within the filtered rules
    None
}

fn run_all_rules(file_name: &String, media_type_registry: &MediaTypeRegistry) -> Option<String> {
    for (media_type, rule_set) in &media_type_registry.rules_registry {
        info!("Checking rule for {}", media_type);

        if run_rule_set(&file_name, rule_set) {
            info!("Matched item as media type {}", media_type);

            return Some(media_type.clone());
        }
    }

    // No rules matched
    None
}

#[cfg(test)]
mod tests {
    use log::LevelFilter;

    use rule_def::MediaTypeRegistry;
    use rule_gen::load_type_registry;

    use crate::{Logger, run_filtered_rules};

    static LOG: Logger = Logger;

    #[test]
    fn it_works() {
        log::set_logger(&LOG)
            .map(|()| log::set_max_level(LevelFilter::Info))
            .expect("Logger failed to initialise");

        let registry: MediaTypeRegistry = load_type_registry();

        let file_name: String = "file.json".into();

        let option = run_filtered_rules(file_name, &registry);

        println!("Loaded {:?} rules; matched {:?}", registry.rules_registry.len(), option)
    }
}
