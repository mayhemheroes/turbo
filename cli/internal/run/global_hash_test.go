package run

import (
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/vercel/turbo/cli/internal/env"
)

func TestGetGlobalHashableEnvVars(t *testing.T) {
	testCases := []struct {
		envAtExecutionStart env.EnvironmentVariableMap
		globalEnv           []string
		expectedMap         env.DetailedMap
	}{
		{
			envAtExecutionStart: env.EnvironmentVariableMap{
				"FOO":     "bar",
				"BAR_BAT": "baz",
			},
			globalEnv: []string{
				"FOO*",
				"!BAR*",
			},
			expectedMap: env.DetailedMap{},
		},
	}

	for _, testCase := range testCases {
		result, err := getGlobalHashableEnvVars(testCase.envAtExecutionStart, testCase.globalEnv)
		assert.NoError(t, err)
		assert.Equal(t, testCase.expectedMap, result)
	}
}
