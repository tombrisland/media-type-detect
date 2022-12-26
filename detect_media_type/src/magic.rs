use std::cmp::min;

use log::{debug, info};

use rule_def::{MagicRule, Match, Single};

pub(crate) fn run_magic(buf: &[u8], magic_rule: &MagicRule) -> bool {
    return magic_rule.conditions.iter().any(|condition: &Match| {
        match condition {
            Match::Multi(multi) => {
                let mut curr_matches: u8 = 0;

                for match_clause in &multi.conditions {
                    if evaluate_match(buf, match_clause) {
                        curr_matches += 1;
                    }

                    if curr_matches == multi.min_to_match {
                        return true;
                    }
                }

                false
            }
            Match::Single(single) => evaluate_match(buf, single)
        }
    });
}

fn evaluate_match(buf: &[u8], match_clause: &Single) -> bool {
    let from: usize = match_clause.offset.from as usize;

    if buf.len() > from {
        let to: usize = min(match_clause.bytes.len() + from, buf.len());
        let slice: &[u8] = &buf[from..to];

        debug!("Comparing buffers {:?} to {:?}", slice, &match_clause.bytes);

        slice == match_clause.bytes.as_slice() &&
            // Either there are no nested conditions
            (match_clause.conditions.is_empty() ||
                // Or at least one child condition must match
                match_clause.conditions.iter().any(|child_clause| evaluate_match(buf, child_clause)))
    } else {
        // If buffer ends before magic starts it can't be a match
        false
    }
}