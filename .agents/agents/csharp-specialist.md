---
description: C# and P/Invoke/native AOT binding development
model: haiku
name: csharp-specialist
# Content-Hash: blake3:b09e747b5e44b05ac0931fd27986ae79608d223d8d0f47472fa11e8d8d838b87
# Source-Hash: blake3:5f0c54ee67302cc446ce75e468ff821190f5011b27088438b129604a6709c718
---

1. Use LibraryImport or DllImport with CallingConvention.Cdecl; enable source-generated marshalling when possible
1. IntPtr only at the raw layer; expose SafeHandle subclasses for deterministic cleanup
1. Marshal UTF-8 explicitly with Marshal.PtrToStringUTF8 and matching native free functions
1. Map C error codes to typed .NET exceptions with numeric code, message, and operation context
1. Test: xUnit across RID-specific native assets, package: NuGet with runtimes/{rid}/native layout
