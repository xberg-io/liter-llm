use std::path::Path;

fn main() {
    // Re-run whenever any Rust source changes.
    println!("cargo:rerun-if-changed=src");

    // Optional FRB codegen: regenerate flutter_rust_bridge artifacts when the
    // tool is on PATH. Missing tool is not fatal — committed generated sources
    // are checked in, and CI environments without FRB still build cleanly.
    match std::process::Command::new("flutter_rust_bridge_codegen")
        .args(["generate", "--config-file", "flutter_rust_bridge.yaml"])
        .status()
    {
        Ok(status) if status.success() => {
            // FRB v2.12+ emits `use` lists in an order rustfmt 2024 edition rewrites
            // (e.g. `{transform_result_dco, Lifetimeable, Lockable}` →
            // `{Lifetimeable, Lockable, transform_result_dco}`). Run rustfmt against
            // the generated file so committed output is fmt-clean and `cargo fmt --check`
            // stays green in CI.
            match std::process::Command::new("rustfmt")
                .args(["--edition", "2024", "src/frb_generated.rs"])
                .status()
            {
                Ok(s) if s.success() => {}
                Ok(s) => println!("cargo:warning=rustfmt on src/frb_generated.rs exited {s}"),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                    println!(
                        "cargo:warning=rustfmt not on PATH — skipping post-FRB format. Install rustfmt via rustup to keep generated bridge sources fmt-clean."
                    );
                }
                Err(err) => println!("cargo:warning=failed to spawn rustfmt: {err}"),
            }

            // Patch the generated Dart entrypoint so the published package resolves
            // its native library from its own installed location.
            patch_published_loader();
        }
        Ok(status) => panic!("flutter_rust_bridge_codegen generate failed (exit code: {status})"),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            println!(
                "cargo:warning=flutter_rust_bridge_codegen not on PATH — skipping codegen. Install via `dart pub global activate flutter_rust_bridge_codegen` to regenerate FRB artifacts at build time."
            );
        }
        Err(err) => panic!("failed to spawn flutter_rust_bridge_codegen: {err}"),
    }
}

const FRB_GENERATED_DART: &str = "../lib/src/liter_llm_bridge_generated/frb_generated.dart";
const LOADER_MARKER: &str = "_alefResolveExternalLibrary";
const FRB_INIT_PROLOGUE: &str = "  /// Initialize flutter_rust_bridge\n  static Future<void> init({\n    RustLibApi? api,\n    BaseHandler? handler,\n    ExternalLibrary? externalLibrary,\n    bool forceSameCodegenVersion = true,\n  }) async {\n";
const FRB_INIT_REPLACEMENT: &str = r#"  /// Resolve the prebuilt native library from environment variable,
  /// package-relative location, or defer to flutter_rust_bridge's default loader.
  /// Returns `null` to defer to flutter_rust_bridge's default loader.
  ///
  /// Checks in order:
  /// 1. FRB_DART_LOAD_EXTERNAL_LIBRARY_NATIVE_LIB_DIR environment variable
  ///    (allows test harnesses to point to development build paths)
  /// 2. Package-installed location (lib/src/liter_llm_bridge_generated/)
  ///    (for published pub.dev packages with bundled native libraries)
  /// 3. Returns null (flutter_rust_bridge falls back to its default loader)
  static Future<ExternalLibrary?> _alefResolveExternalLibrary() async {
    try {
      // On Linux the published package ships arch-specific .so files named
      // libliter_llm_dart_x86_64.so and libliter_llm_dart_aarch64.so.
      // Detect arch from Platform.version (e.g. "linux_x64", "linux_arm64").
      final linuxArch = Platform.version.contains('arm64') ||
              Platform.version.contains('aarch64')
          ? '_aarch64'
          : '_x86_64';
      final candidates = <String>[
        if (Platform.isMacOS) 'libliter_llm_dart.dylib',
        if (Platform.isLinux) 'libliter_llm_dart$linuxArch.so',
        if (Platform.isLinux) 'libliter_llm_dart.so',
        if (Platform.isWindows) 'liter_llm_dart.dll',
      ];

      // Check FRB_DART_LOAD_EXTERNAL_LIBRARY_NATIVE_LIB_DIR env var first.
      // This allows test harnesses to override library location for development.
      final envDir = Platform.environment['FRB_DART_LOAD_EXTERNAL_LIBRARY_NATIVE_LIB_DIR'];
      if (envDir != null && envDir.isNotEmpty) {
        final libDir = Directory(envDir);
        if (libDir.existsSync()) {
          for (final candidate in candidates) {
            final libPath = '$envDir/$candidate';
            if (File(libPath).existsSync()) {
              return ExternalLibrary.open(libPath);
            }
          }
        }
      }

      // Check package-installed location.
      final packageRoot =
          await Isolate.resolvePackageUri(Uri.parse('package:liter_llm/liter_llm.dart'));
      if (packageRoot != null) {
        final libDir = packageRoot.resolve('src/liter_llm_bridge_generated/');
        for (final candidate in candidates) {
          final libPath = libDir.resolve(candidate).toFilePath();
          if (File(libPath).existsSync()) {
            return ExternalLibrary.open(libPath);
          }
        }
      }
    } catch (_) {
      // Fall through to the default loader on any resolution failure.
    }
    return null;
  }

  /// Initialize flutter_rust_bridge
  static Future<void> init({
    RustLibApi? api,
    BaseHandler? handler,
    ExternalLibrary? externalLibrary,
    bool forceSameCodegenVersion = true,
  }) async {
    externalLibrary ??= await _alefResolveExternalLibrary();
"#;

/// Inject the published-package native-library loader into `frb_generated.dart`.
/// Idempotent: a no-op when the marker is already present or the FRB entrypoint
/// signature is absent.
fn patch_published_loader() {
    let path = Path::new(FRB_GENERATED_DART);
    let Ok(source) = std::fs::read_to_string(path) else {
        println!(
            "cargo:warning=published-loader patch skipped: {} not found",
            FRB_GENERATED_DART
        );
        return;
    };
    if source.contains(LOADER_MARKER) {
        return;
    }
    if !source.contains(FRB_INIT_PROLOGUE) {
        println!("cargo:warning=published-loader patch skipped: FRB init prologue not found");
        return;
    }

    let mut patched = source.replacen(FRB_INIT_PROLOGUE, FRB_INIT_REPLACEMENT, 1);

    // Ensure the helper's `File`/`Isolate` dependencies are imported.
    for (probe, line) in [
        ("import 'dart:io';", "import 'dart:io';\n"),
        ("import 'dart:isolate';", "import 'dart:isolate';\n"),
    ] {
        if patched.contains(probe) {
            continue;
        }
        if let Some(pos) = patched.find("\nimport ") {
            patched.insert_str(pos + 1, line);
        } else {
            patched.insert_str(0, line);
        }
    }

    if patched != source {
        if let Err(err) = std::fs::write(path, &patched) {
            println!("cargo:warning=failed to write published-loader patch: {err}");
            return;
        }
        match std::process::Command::new("dart")
            .args(["format", FRB_GENERATED_DART])
            .status()
        {
            Ok(s) if s.success() => {}
            Ok(s) => println!("cargo:warning=dart format on {} exited {}", FRB_GENERATED_DART, s),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                println!(
                    "cargo:warning=dart not on PATH — skipping post-patch format. Install Dart SDK to keep generated FRB Dart sources fmt-clean."
                );
            }
            Err(err) => println!("cargo:warning=failed to spawn dart format: {err}"),
        }
    }
}
