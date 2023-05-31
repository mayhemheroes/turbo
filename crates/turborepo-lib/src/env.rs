use std::{collections::BTreeMap, env};

use petgraph::visit::Walker;
use regex::Regex;
use sha2::{Digest, Sha256};

// TODO: Consider using immutable data structures here
#[derive(Debug, Default)]
pub struct EnvironmentVariableMap(BTreeMap<String, String>);

// BySource contains a map of environment variables broken down by the source
pub struct BySource {
    pub explicit: EnvironmentVariableMap,
    pub matching: EnvironmentVariableMap,
}

// DetailedMap contains the composite and the detailed maps of environment
// variables All is used as a taskhash input (taskhash.CalculateTaskHash)
// BySoure is used to print out a Dry Run Summary
pub struct DetailedMap {
    pub all: EnvironmentVariableMap,
    pub by_source: BySource,
}

// EnvironmentVariablePairs is a list of "k=v" strings for env variables and
// their values
type EnvironmentVariablePairs = Vec<String>;

// WildcardMaps is a pair of EnvironmentVariableMaps.
struct WildcardMaps {
    pub inclusions: EnvironmentVariableMap,
    pub exclusions: EnvironmentVariableMap,
}

impl WildcardMaps {
    // Resolve collapses a WildcardSet into a single EnvironmentVariableMap.
    fn resolve(&self) -> EnvironmentVariableMap {
        let mut output = self.inclusions.clone();
        for (key, _) in &self.exclusions {
            output.remove(key);
        }
        output
    }
}

impl EnvironmentVariableMap {
    pub fn infer() -> Self {
        EnvironmentVariableMap(env::vars().iter().collect())
    }

    // Takes another EnvironmentVariableMap and adds it into `self`
    // Overwrites values if they already exist.
    pub fn union(&mut self, another: &EnvironmentVariableMap) {
        for (key, value) in another {
            self.0.insert(key.clone(), value.clone());
        }
    }

    // Takes another EnvironmentVariableMap and removes matching keys
    // from `self`
    pub fn difference(evm: &mut EnvironmentVariableMap, another: &EnvironmentVariableMap) {
        for key in another.keys() {
            evm.remove(key);
        }
    }

    pub fn insert(&mut self, key: String, value: String) {
        self.0.insert(key, value);
    }

    // Returns a sorted list of env var names from `self`
    fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = evm.keys().cloned().collect();
        names.sort();
        names
    }

    // Returns a deterministically sorted set of EnvironmentVariablePairs
    // from `self`. It takes a function to operate on each
    // key-value pair and return a string
    fn map_to_pair<F>(&self, transformer: F) -> EnvironmentVariablePairs
    where
        F: Fn(&str, &str) -> String,
    {
        let mut pairs: EnvironmentVariablePairs =
            evm.iter().map(|(k, v)| transformer(k, v)).collect();
        pairs.sort();
        pairs
    }

    // Returns a deterministically sorted set of EnvironmentVariablePairs
    // from an EnvironmentVariableMap This is the value used to print out
    // the task hash input, so the values are cryptographically hashed
    fn to_secret_hashable(&self) -> EnvironmentVariablePairs {
        self.map_to_pair(|k, v| {
            if !v.is_empty() {
                let hashed_value = Sha256::digest(v.as_bytes());
                format!("{}={:x}", k, hashed_value)
            } else {
                format!("{}=", k)
            }
        })
    }

    // ToHashable returns a deterministically sorted set of EnvironmentVariablePairs
    // from an EnvironmentVariableMap This is the value that is used upstream as a
    // task hash input, so we need it to be deterministic
    fn to_hashable(&self) -> EnvironmentVariablePairs {
        self.map_to_pair(|k, v| format!("{}={}", k, v))
    }

    // from_wildcards returns a wildcardSet after processing wildcards against it.
    fn wildcard_map_from_wildcards(
        &self,
        wildcard_patterns: &[String],
    ) -> Result<WildcardMaps, regex::Error> {
        let mut output = WildcardMaps {
            inclusions: EnvironmentVariableMap::new(),
            exclusions: EnvironmentVariableMap::new(),
        };

        let mut include_patterns = Vec::new();
        let mut exclude_patterns = Vec::new();

        for wildcard_pattern in wildcard_patterns {
            if wildcard_pattern.starts_with('!') {
                let exclude_pattern = wildcard_to_regex_pattern(&wildcard_pattern[1..]);
                exclude_patterns.push(exclude_pattern);
            } else if wildcard_pattern.starts_with('\\')
                && wildcard_pattern.chars().nth(1) == Some('!')
            {
                let include_pattern = wildcard_to_regex_pattern(&wildcard_pattern[1..]);
                include_patterns.push(include_pattern);
            } else {
                let include_pattern = wildcard_to_regex_pattern(&wildcard_pattern);
                include_patterns.push(include_pattern);
            }
        }

        let include_regex_string = format!("^({})$", include_patterns.join("|"));
        let exclude_regex_string = format!("^({})$", exclude_patterns.join("|"));

        let include_regex = Regex::new(&include_regex_string)?;
        let exclude_regex = Regex::new(&exclude_regex_string)?;

        for (env_var, env_value) in evm {
            if !include_patterns.is_empty() && include_regex.is_match(env_var) {
                output.inclusions.insert(env_var.clone(), env_value.clone());
            }
            if !exclude_patterns.is_empty() && exclude_regex.is_match(env_var) {
                output.exclusions.insert(env_var.clone(), env_value.clone());
            }
        }

        Ok(output)
    }

    // Returns an EnvironmentVariableMap containing the variables
    // in the environment which match an array of wildcard patterns.
    pub fn from_wildcards(
        &self,
        wildcard_patterns: &[String],
    ) -> Result<EnvironmentVariableMap, regex::Error> {
        if wildcard_patterns.is_empty() {
            return Ok(EnvironmentVariableMap::new());
        }

        let mut resolved_set = self.from_wildcards(wildcard_patterns)?;
        Ok(resolved_set.resolve())
    }

    // FromWildcardsUnresolved returns a wildcardSet specifying the inclusions and
    // exclusions discovered from a set of wildcard patterns. This is used to ensure
    // that user exclusions have primacy over inferred inclusions.
    pub fn from_wildcards_unresolved(
        &self,
        wildcard_patterns: &[String],
    ) -> Result<WildcardMaps, regex::Error> {
        if wildcard_patterns.is_empty() {
            return Ok(WildcardMaps {
                inclusions: EnvironmentVariableMap::new(),
                exclusions: EnvironmentVariableMap::new(),
            });
        }

        self.from_wildcards(wildcard_patterns)
    }
}

const WILDCARD: char = '*';
const WILDCARD_ESCAPE: char = '\\';
const REGEX_WILDCARD_SEGMENT: &str = ".*";

fn wildcard_to_regex_pattern(pattern: &str) -> String {
    let mut regex_string = Vec::new();
    let mut previous_index = 0;
    let mut previous_char: Option<char> = None;

    for (i, char) in pattern.chars().enumerate() {
        if char == WILDCARD {
            if previous_char == Some(WILDCARD_ESCAPE) {
                // Found a literal *
                // Replace the trailing "\*" with just "*" before adding the segment.
                regex_string.push(format!(
                    "{}*",
                    regex::escape(&pattern[previous_index..i - 1])
                ));
            } else {
                // Found a wildcard
                // Add in the static segment since the last wildcard. Can be zero length.
                regex_string.push(regex::escape(&pattern[previous_index..i]));

                // Add a dynamic segment if it isn't adjacent to another dynamic segment.
                if let Some(last_segment) = regex_string.last() {
                    if last_segment != &REGEX_WILDCARD_SEGMENT {
                        regex_string.push(REGEX_WILDCARD_SEGMENT.to_string());
                    }
                } else {
                    regex_string.push(REGEX_WILDCARD_SEGMENT.to_string());
                }
            }

            // Advance the pointer.
            previous_index = i + 1;
        }
        previous_char = Some(char);
    }

    // Add the last static segment. Can be zero length.
    regex_string.push(regex::escape(&pattern[previous_index..]));

    regex_string.join("")
}
