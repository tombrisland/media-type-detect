use std::cmp::min;
use log::debug;
use rule_def::{MagicRule, Match};

// TODO split into globs and magic so can be run separately

pub(crate) fn run_magic(buf: &[u8], magic_rule: &MagicRule) -> bool {
    return magic_rule.conditions.iter().any(|condition: &Match| {
        match condition {
            // TODO implement the multi behaviour
            Match::Multi(_) => false,
            Match::Single(single) => {
                let from: usize = single.offset.from as usize;

                // TODO could do partial magic matches if file isn't long enough
                if buf.len() > from {
                    let to: usize = min(single.bytes.len() + from, buf.len());
                    let slice: &[u8] = &buf[from..to];

                    debug!("Comparing buffers {:?} to {:?}", slice, &single.bytes);

                    vector_contains(slice, &single.bytes)
                } else {
                    // If buffer ends before magic starts can't be a match
                    false
                }
            }
        }
    });
}

fn vector_contains<T: Eq>(a: &[T], b: &Vec<T>) -> bool {
    for idx in 0..a.len() {
        if a[idx] != b[idx] {
            return false;
        }
    }

    return true;
}