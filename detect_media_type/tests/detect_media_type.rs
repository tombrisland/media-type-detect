use std::collections::HashMap;
use std::path::Path;
use detect_media_type;

use std::time::Instant;

use log::{Level, LevelFilter, Metadata, Record};
use detect_media_type::{DetectorConfig, MediaTypeDetector};
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

    println!("Loaded {:?} magic and {:?} globs in {:?}",
             registry.magic_rules.len(), registry.glob_rules.len(), start.elapsed());

    let detector = MediaTypeDetector {
        registry,
        config: DetectorConfig {
            enable_glob: false,
            enable_magic: false,
            prioritise_glob: false,
            max_concurrency: 0,
            default_type: "",
        },
    };

    let mut expected_types: HashMap<String, Option<String>> = HashMap::new();

    expected_types.insert("image_heic".into(), Some("image/heic".into()));
    expected_types.insert("image_jpeg".into(), Some("image/jpeg".into()));
    expected_types.insert("image.png".into(), Some("image/png".into()));
    expected_types.insert("image_png".into(), Some("image/png".into()));
    expected_types.insert("file.json".into(), Some("application/json".into()));

    for (file_name, media_type) in expected_types {
        let after_load = Instant::now();

        let str: String = format!("./tests/data/{}", file_name);

        let path = Path::new(str.as_str());

        let option = detector.detect_file_type(path);

        println!("Matched {:?} in {:?}", option, after_load.elapsed());

        assert_eq!(option, media_type);
    }
}