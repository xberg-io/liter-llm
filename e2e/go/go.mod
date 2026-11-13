module github.com/xberg-io/liter-llm/packages/go/e2e

go 1.27

require (
	github.com/stretchr/testify v1.12.1
	github.com/xberg-io/liter-llm/packages/go/v2 v2.7.1
)

require go.yaml.in/yaml/v3 v3.0.5 // indirect

replace github.com/xberg-io/liter-llm/packages/go/v2 => ../../packages/go
