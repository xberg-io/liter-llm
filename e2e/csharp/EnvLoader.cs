// Hand-written harness hook: load liter-llm/.env from the repo root so
// smoke tests gated on OPENAI_API_KEY/ANTHROPIC_API_KEY/GEMINI_API_KEY
// pick them up automatically. Module initializers run before any tests.
using System.IO;
using System.Runtime.CompilerServices;

namespace Kreuzberg.E2eTests;

internal static class EnvLoader
{
    [ModuleInitializer]
    internal static void LoadEnv()
    {
        // Walk up from the assembly directory until we find the directory
        // that contains a .env file (the liter-llm repo root).
        var dir = new DirectoryInfo(System.AppContext.BaseDirectory);
        while (dir != null)
        {
            var candidate = Path.Combine(dir.FullName, ".env");
            if (File.Exists(candidate))
            {
                DotNetEnv.Env.Load(candidate);
                return;
            }
            dir = dir.Parent;
        }
    }
}
