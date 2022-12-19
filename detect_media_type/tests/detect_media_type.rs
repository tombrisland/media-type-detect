use std::path::Path;
use detect_media_type;

use std::time::Instant;

use log::{Level, LevelFilter, Metadata, Record};
use detect_media_type::detect_file_type;
use rule_def::MediaTypeRegistry;
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

static LOG: Logger = Logger;

#[test]
fn it_works() {
    log::set_logger(&LOG)
        .map(|()| log::set_max_level(LevelFilter::Info))
        .expect("Logger failed to initialise");

    let start = Instant::now();

    let registry: MediaTypeRegistry = load_type_registry();

    let after_load = Instant::now();

    let path = Path::new("./tests/data/image_jpeg");

    let option = detect_file_type(path, &registry);

    println!("Loaded {:?} rules in {:?}; matched {:?} in {:?}", registry.rules_registry.len(), start.elapsed(), option, after_load.elapsed())
}