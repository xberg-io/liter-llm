---
type: Reference
title: Scoop Manifest Layout
paths:
  - "scripts/publish/liter-llm.json.tmpl"
  - "scripts/publish/scoop.json"
  - ".github/workflows/publish.yaml"
x-ai-rulez:
  kind: rule
  id: scoop-manifest
  metadata:
    priority: medium
---

- The Windows zip nests the binary in a directory whose name carries the version, so `extract_dir` is `liter-llm-${version}-x86_64-pc-windows-msvc` and must stay templated on the version. The layout is set by the `Stage archive` step in .github/workflows/publish.yaml.
