require "./spec_helper"

describe LiterLlm do
  describe "error-handling" do
    pending "401 Authentication error returned by the Anthropic API when the API key is invalid"
    pending "401 Unauthorized error when API key is invalid or missing"
    pending "Azure OpenAI returns a 401 Unauthorized error when the API key is missing or invalid — uses Azure's error envelope shape with code AccessDenied"
    pending "400 Bad Request error when a parameter value is invalid"
    pending "AWS Bedrock returns 403 Forbidden (not 401) when credentials are missing, expired, or the IAM role lacks bedrock:InvokeModel permission — verifies the error is mapped to Authentication"
    pending "400 error when a request is rejected due to content policy"
    pending "400 error when the prompt exceeds the model's maximum context length"
    pending "200 OK response with an empty JSON object body, missing required fields"
    pending "403 Forbidden error when the API key does not have access to the requested resource"
    pending "504 Gateway Timeout error when the upstream service times out"
    pending "401 Authentication error returned by the GitHub Copilot API when the token is invalid or expired"
    pending "404 Not Found error when requesting a model that does not exist"
    pending "429 Too Many Requests error when the rate limit is exceeded"
    pending "500 Internal Server Error from the upstream API"
    pending "502 Bad Gateway error when the upstream service is unavailable"
    pending "408 Request Timeout error when the API request takes too long to complete"
    pending "Google Vertex AI returns 401 Unauthorized when the OAuth2 token is missing, expired, or the service account lacks aiplatform.endpoints.predict permission — verifies the error is mapped to Authentication"
  end
end
