const SECURITY = "/usr/bin/security";
const APPLE_DEVELOPMENT_PREFIX = "Apple Development:";

if (process.platform !== "darwin") {
  console.error("Signed development bundles are supported only on macOS.");
  process.exit(1);
}

const identityProbe = Bun.spawnSync([SECURITY, "find-identity", "-v", "-p", "codesigning"], {
  stdout: "pipe",
  stderr: "pipe",
});

if (identityProbe.exitCode !== 0) {
  console.error(new TextDecoder().decode(identityProbe.stderr).trim() || "Could not inspect the macOS code-signing identities.");
  process.exit(identityProbe.exitCode || 1);
}

const identity = new TextDecoder()
  .decode(identityProbe.stdout)
  .split("\n")
  .map((line) => line.match(/^\s*\d+\)\s+([A-F0-9]{40})\s+"([^"]+)"\s*$/))
  .find((match) => match?.[2].startsWith(APPLE_DEVELOPMENT_PREFIX));

if (!identity) {
  console.error("No valid Apple Development code-signing identity was found. Create or import one before running this command.");
  process.exit(1);
}

const [, identityHash, identityName] = identity;
console.log(`Building with ${identityName} (${identityHash}).`);

const configOverride = JSON.stringify({
  bundle: {
    macOS: {
      signingIdentity: identityHash,
    },
  },
});

const build = Bun.spawn(
  ["bun", "run", "tauri", "build", "--debug", "--bundles", "app", "--config", configOverride],
  { stdin: "inherit", stdout: "inherit", stderr: "inherit" },
);

process.exit(await build.exited);
