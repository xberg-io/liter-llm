---
name: create-e2e-fixture
description: "Create a new e2e test fixture for cross-language test generation"
# Content-Hash: blake3:ddefe75eaa45d6cd64c916352085396dc80e0a53e8041fbf893729c80ef8d748
# Source-Hash: blake3:5f0c54ee67302cc446ce75e468ff821190f5011b27088438b129604a6709c718
---

# Create E2E Fixture

## Steps

1. **Identify the test case**: What behavior needs testing across language bindings?
1. **Choose category**: smoke, basic, parsing, edge-case, error-handling, or platform-specific.
1. **Write fixture JSON**:

   ```json
   {
     "id": "descriptive_snake_case_name",
     "category": "basic",
     "description": "Tests that X produces Y",
     "source_code": "input data or code",
     "assertions": {
       "tree_not_null": true,
       "expect_error": false
     },
     "tags": ["regression"]
   }
   ```

1. **Add to fixtures directory**: Place in appropriate category file or subdirectory.
1. **Regenerate**: Run `task generate:e2e`.
1. **Verify**: Run `task test:e2e` to confirm all languages pass.
1. **Commit**: Include fixture + regenerated test files in same commit.

## Guidelines

- IDs must be unique across all fixture files
- Include both positive and negative test cases
- Add skip conditions for platform-specific behavior
- Keep source_code minimal — test one thing per fixture
