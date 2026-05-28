# frozen_string_literal: true

Gem::Specification.new do |spec|
  spec.name = "liter_llm"
  spec.version = "1.4.0.pre.rc.43"
  spec.authors       = ["Na'aman Hirschfeld"]
  spec.summary       = "Universal LLM API client with Rust-powered polyglot bindings."
  spec.description   = "Universal LLM API client with Rust-powered polyglot bindings."
  spec.homepage      = "https://github.com/kreuzberg-dev/liter-llm"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.2.0"
  spec.metadata["keywords"] = %w[anthropic api-client llm openai].join(",")
  spec.metadata["rubygems_mfa_required"] = "true"

  spec.files = Dir.glob(%w[README* LICENSE* lib/**/* ext/**/* sig/**/* Steepfile]).reject do |f|
    f.include?("/native/target/") || f.include?("/native/tmp/")
  end
  spec.require_paths = ["lib"]
  spec.extensions    = ["ext/liter_llm_rb/extconf.rb"]

  spec.add_dependency "rb_sys", "~> 0.9"
  spec.add_dependency "sorbet-runtime", "~> 0.5"
end
