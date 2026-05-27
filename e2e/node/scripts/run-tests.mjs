// Hand-written harness wrapper: best-effort load of liter-llm/.env from the
// repo root before launching vitest, so smoke tests gated on
// OPENAI_API_KEY/ANTHROPIC_API_KEY/GEMINI_API_KEY pick them up automatically.
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const envPath = resolve(here, "..", "..", "..", ".env");

if (existsSync(envPath)) {
	const { config } = await import("dotenv");
	config({ path: envPath });
}

const vitestBin = resolve(here, "..", "node_modules", ".bin", "vitest");
const args = ["run", ...process.argv.slice(2)];

const child = spawn(vitestBin, args, {
	stdio: "inherit",
	env: process.env,
});

child.on("exit", (code, signal) => {
	if (signal) {
		process.kill(process.pid, signal);
	} else {
		process.exit(code ?? 0);
	}
});
