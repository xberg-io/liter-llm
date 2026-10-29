#!/usr/bin/env bash
#MISE description="Run schemathesis API contract tests against proxy Docker image"
set -euo pipefail

docker build -f docker/Dockerfile -t liter-llm:schemathesis .
container_id=$(docker run -d -p 4111:4000 -e LITER_LLM_MASTER_KEY=sk-schema-test liter-llm:schemathesis)
sleep 3
failed=0
schemathesis run http://localhost:4111/openapi.json \
  --max-examples=10 \
  --request-timeout=30000 \
  --header "Authorization: Bearer sk-schema-test" \
  --checks status_code_conformance,response_schema_conformance,negative_data_rejection || failed=1
docker stop "$container_id"
exit "$failed"
