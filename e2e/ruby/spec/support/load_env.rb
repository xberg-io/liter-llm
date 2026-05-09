# frozen_string_literal: true

# Hand-written harness hook: load .env from the liter-llm repo root so
# smoke specs that gate on OPENAI_API_KEY/ANTHROPIC_API_KEY/GEMINI_API_KEY
# pick them up automatically when present.
require 'dotenv'

env_path = File.expand_path('../../../.env', __FILE__)
Dotenv.load(env_path) if File.exist?(env_path)
