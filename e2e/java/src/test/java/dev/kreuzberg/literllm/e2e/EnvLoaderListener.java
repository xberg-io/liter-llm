// Hand-written harness hook: load liter-llm/.env from the repo root so
// smoke tests gated on OPENAI_API_KEY/ANTHROPIC_API_KEY/GEMINI_API_KEY
// pick them up automatically. Registered via JUnit Platform's
// LauncherSessionListener service-loader entry under
// META-INF/services/org.junit.platform.launcher.LauncherSessionListener.
package dev.kreuzberg.literllm.e2e;

import io.github.cdimascio.dotenv.Dotenv;
import org.junit.platform.launcher.LauncherSession;
import org.junit.platform.launcher.LauncherSessionListener;

public final class EnvLoaderListener implements LauncherSessionListener {

    @Override
    public void launcherSessionOpened(final LauncherSession session) {
        // Walk up from the working directory until a .env is found
        // (handles maven surefire's workingDirectory pointing at
        // test_documents while .env lives at the liter-llm repo root).
        java.io.File dir = new java.io.File(System.getProperty("user.dir")).getAbsoluteFile();
        while (dir != null) {
            final java.io.File candidate = new java.io.File(dir, ".env");
            if (candidate.isFile()) {
                final Dotenv dotenv = Dotenv.configure()
                    .directory(dir.getAbsolutePath())
                    .ignoreIfMissing()
                    .load();
                dotenv.entries().forEach(entry -> {
                    if (System.getProperty(entry.getKey()) == null) {
                        System.setProperty(entry.getKey(), entry.getValue());
                    }
                });
                return;
            }
            dir = dir.getParentFile();
        }
    }
}
