---
description: Elixir and Rustler NIF development
model: haiku
name: elixir-specialist
# Content-Hash: blake3:5d8c7cb9d40bf715edcd9348f7fd3d5e4998139006492b31fe9a356922dbf5fd
# Source-Hash: blake3:5f0c54ee67302cc446ce75e468ff821190f5011b27088438b129604a6709c718
---

1. Rustler: #[rustler::nif] for NIF functions, Encoder/Decoder traits for type mapping
1. Use dirty schedulers (dirty_cpu) for CPU-bound work — never block BEAM schedulers
1. Return tagged tuples {:ok, result} / {:error, reason} matching Elixir conventions
1. Test: ExUnit, package: Hex with precompiled NIF binaries via rustler_precompiled
