//go:build rust
// +build rust

package run

import (
	"github.com/vercel/turbo/cli/internal/env"

	"github.com/vercel/turbo/cli/internal/ffi"
)

// `getGlobalHashableEnvVars` calculates env var dependencies
func getGlobalHashableEnvVars(envAtExecutionStart env.EnvironmentVariableMap, globalEnv []string) (env.DetailedMap, error) {
	respDetailedMap, err := ffi.GetGlobalHashableEnvVars(envAtExecutionStart, globalEnv)
	if err != nil {
		return env.DetailedMap{}, err
	}
	detailedMap := env.DetailedMap{
		All: respDetailedMap.GetAll(),
		BySource: env.BySource{
			Explicit: respDetailedMap.GetBySource().GetExplicit(),
			Matching: respDetailedMap.GetBySource().GetMatching(),
		},
	}
	return detailedMap, nil
}
