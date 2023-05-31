use std::{collections::HashMap, string::ToString};

use anyhow::Result;
use turbopath::RelativeUnixPathBuf;
use turborepo_lockfile::Lockfile;

use crate::{
    cli::EnvMode,
    commands::CommandBase,
    env::{BySource, DetailedMap, EnvironmentVariableMap},
    package_json::PackageJson,
    package_manager::PackageManager,
};

static DEFAULT_ENV_VARS: [String; 1] = ["VERCEL_ANALYTICS_ID".to_string()];

#[derive(Default)]
struct GlobalHashableInputs {
    global_cache_key: &'static str,
    global_file_hash_map: HashMap<RelativeUnixPathBuf, String>,
    root_external_deps_hash: String,
    env: Vec<String>,
    resolved_env_vars: DetailedMap,
    pass_through_env: Vec<String>,
    env_mode: EnvMode,
    framework_inference: bool,
    dot_env: Vec<RelativeUnixPathBuf>,
}

fn get_global_hash_inputs(
    _base: &mut CommandBase,
    _root_package_json: &PackageJson,
    _package_manager: PackageManager,
    _lockfile: Box<dyn Lockfile>,
    _global_file_dependencies: Vec<String>,
    env_at_execution_start: &EnvironmentVariableMap,
    global_env: Vec<String>,
    _global_pass_through_env: Vec<String>,
    _env_mode: EnvMode,
    _framework_inference: bool,
    _dot_env: Vec<RelativeUnixPathBuf>,
) -> Result<GlobalHashableInputs> {
    let default_env_var_map = env_at_execution_start.from_wildcards(&DEFAULT_ENV_VARS[..])?;

    let user_env_var_set = env_at_execution_start.from_wildcards_unresolved(&global_env)?;

    let mut all_env_var_map = EnvironmentVariableMap::default();
    all_env_var_map.union(&user_env_var_set.inclusions);
    all_env_var_map.union(&default_env_var_map);
    all_env_var_map.difference(&user_env_var_set.exclusions);

    let mut explicit_env_var_map = EnvironmentVariableMap::default();
    explicit_env_var_map.union(&user_env_var_set.inclusions);
    explicit_env_var_map.difference(&user_env_var_set.exclusions);

    let mut matching_env_var_map = EnvironmentVariableMap::default();
    matching_env_var_map.union(&default_env_var_map);
    matching_env_var_map.difference(&user_env_var_set.exclusions);

    let global_hashable_env_vars = DetailedMap {
        all: all_env_var_map,
        by_source: BySource {
            explicit: explicit_env_var_map,
            matching: matching_env_var_map,
        },
    };

    Ok(GlobalHashableInputs {
        resolved_env_vars: global_hashable_env_vars,
        ..GlobalHashableInputs::default()
    })
}
