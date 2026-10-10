import fs from "node:fs";
import path from "node:path";
import { BUILDS_DIR, buildFile } from "./builds";
import { SITE_ORIGIN } from "./config";

/** Exact files downloaded by the browser, excluding macOS transfer metadata. */
export function buildDownloadManifest(build: string) {
  if (!buildFile(build, ["index.html"])) return {files:[], bytes:0};
  const files: {path:string; bytes:number}[] = [];
  const root = path.join(BUILDS_DIR, build);
  const walk = (directory: string, prefix: string) => {
    for (const entry of fs.readdirSync(directory, {withFileTypes:true})) {
      if (entry.name.startsWith("._") || [".DS_Store", "__MACOSX", "Thumbs.db"].includes(entry.name)) continue;
      const relative = prefix + entry.name;
      if (entry.isDirectory()) walk(path.join(directory, entry.name), relative + "/");
      else if (entry.isFile()) files.push({path:relative, bytes:fs.statSync(path.join(directory, entry.name)).size});
    }
  };
  walk(root, "");
  return {files, bytes:files.reduce((sum,file) => sum + file.bytes, 0)};
}

/** Helpers share the build's origin and storage partition with its eventual game frame. */
export function downloadResponse(build: string, mode: string, method: string): Response | null {
  if (!buildFile(build, ["index.html"])) return null;
  const headers: Record<string, string> = {"Cache-Control":"no-store", "X-Content-Type-Options":"nosniff"};
  let body: string;
  if (mode === "manifest") {
    body = JSON.stringify(buildDownloadManifest(build));
    headers["Content-Type"] = "application/json";
  } else if (mode === "worker" || mode === "loader") {
    body = fs.readFileSync(path.join(process.cwd(), "public", "download", `${mode}.js`), "utf8");
    headers["Content-Type"] = "text/javascript; charset=utf-8";
  } else if (mode === "download") {
    const origin = SITE_ORIGIN.replace(/&/g,"&amp;").replace(/"/g,"&quot;").replace(/</g,"&lt;");
    body = `<!doctype html><html><head><meta charset="utf-8"><meta name="parent-origin" content="${origin}"><title>Downloading game</title></head><body><script src="index.html?couch=loader"></script></body></html>`;
    headers["Content-Type"] = "text/html; charset=utf-8";
    headers["Content-Security-Policy"] = `default-src 'none'; script-src 'self'; connect-src 'self'; worker-src 'self'; frame-ancestors ${SITE_ORIGIN}`;
  } else return null;
  return new Response(method === "HEAD" ? null : body, {headers});
}
