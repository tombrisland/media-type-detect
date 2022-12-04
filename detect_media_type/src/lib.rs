use std::cmp::min;
use rule_def::{Match, MediaTypeRule, Rule};
use rule_gen::load_media_type_rules;

pub fn detect(file_name: String, buf: [u8; 1024]) {
    let vec : Vec<MediaTypeRule> = load_media_type_rules();

    for rule in vec {
        for x in rule.rules {
            let matches : bool = match x {
                // Run glob pattern against filename
                Rule::Glob(glob) => file_name.ends_with(glob.pattern.as_str()),
                // Check magic bytes
                Rule::Magic(magic) => {
                    for match_condition in magic.conditions {
                        match match_condition {
                            Match::Multi(_) => false,
                            Match::Single(single) => {
                                let end : usize = min(buf.len(), single.bytes.len() + single.offset.from);

                                for i in single.offset.from..end {
                                    if buf[i] != single.bytes.get(i) {
                                        false
                                    }
                                }

                                true
                            }
                        }
                    }
                }
            };

            println!("A rule for {:?} returned {:?}", &rule.media_type, matches);
        }
    }
}

fn run_globs() {

}

#[cfg(test)]
mod tests {
    use rule_def::MediaTypeRule;
    use rule_gen::load_media_type_rules;

    #[test]
    fn it_works() {
        let vec : Vec<MediaTypeRule> = load_media_type_rules();

        println!("Loaded {:?} rules", vec.len())
    }
}
