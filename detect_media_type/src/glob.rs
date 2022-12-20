use rule_def::GlobRule;

pub(crate) fn run_glob(resource_name: &str, glob_rule: &GlobRule) -> bool {
    // TODO impl other globtypes inc regex
    resource_name.ends_with(&glob_rule.pattern)
}