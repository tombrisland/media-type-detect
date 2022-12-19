use rule_def::GlobRule;

pub(crate) fn run_glob(file_name: &Option<String>, glob_rule: &GlobRule) -> bool {
    match file_name {
        None => false,
        Some(name) => name.contains(&glob_rule.pattern)
    }
}