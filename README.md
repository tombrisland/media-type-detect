# Media type detection

Media type detection in Rust. Makes use of the magic bytes and glob definitions from the Apache tika project.

These are parsed ahead of time and compiled into the library.

## Next

* Find a way to store &str references in the structs. Could then move to a const value rather than an export function in `/rule_gen/lib.rs`.
* In the same vein move to slices instead of vectors for conditions + the rule lists
* Implement detectors to cover the remainder of the functionality
* AWS lambda function implementation
* Shuffle recently used rules to the top of the vector