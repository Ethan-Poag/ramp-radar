// beforeBuildCommand may start in the repo root or in src-tauri, depending
// on how `tauri build` was invoked. Resolve web/ from this file's location.
const { execSync } = require("child_process");
const path = require("path");
const web = path.resolve(__dirname, "../web");
execSync("npm ci && npm run build", { cwd: web, stdio: "inherit", shell: true });
