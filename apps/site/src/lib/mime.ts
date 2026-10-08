/** Content types for files in game builds, by extension. */
const TYPES: Record<string, string> = {
  html: "text/html; charset=utf-8",
  htm: "text/html; charset=utf-8",
  js: "text/javascript; charset=utf-8",
  mjs: "text/javascript; charset=utf-8",
  css: "text/css; charset=utf-8",
  json: "application/json",
  map: "application/json",
  wasm: "application/wasm",
  txt: "text/plain; charset=utf-8",
  md: "text/plain; charset=utf-8",
  csv: "text/csv; charset=utf-8",
  xml: "application/xml",
  png: "image/png",
  jpg: "image/jpeg",
  jpeg: "image/jpeg",
  gif: "image/gif",
  webp: "image/webp",
  avif: "image/avif",
  svg: "image/svg+xml",
  ico: "image/x-icon",
  bmp: "image/bmp",
  ktx2: "image/ktx2",
  mp3: "audio/mpeg",
  ogg: "audio/ogg",
  oga: "audio/ogg",
  opus: "audio/ogg",
  wav: "audio/wav",
  m4a: "audio/mp4",
  aac: "audio/aac",
  flac: "audio/flac",
  mp4: "video/mp4",
  m4v: "video/mp4",
  webm: "video/webm",
  ttf: "font/ttf",
  otf: "font/otf",
  woff: "font/woff",
  woff2: "font/woff2",
  glb: "model/gltf-binary",
  gltf: "model/gltf+json",
  // Engine data: Godot packs, Unity data, emscripten memory files, and
  // other binary assets.
  pck: "application/octet-stream",
  data: "application/octet-stream",
  mem: "application/octet-stream",
  bin: "application/octet-stream",
  unityweb: "application/octet-stream",
  symbols: "application/octet-stream",
  basis: "application/octet-stream",
  hdr: "application/octet-stream",
  obj: "text/plain; charset=utf-8",
  mtl: "text/plain; charset=utf-8",
  glsl: "text/plain; charset=utf-8",
  wgsl: "text/plain; charset=utf-8",
  vert: "text/plain; charset=utf-8",
  frag: "text/plain; charset=utf-8",
  atlas: "text/plain; charset=utf-8",
  fnt: "text/plain; charset=utf-8",
};

const ENCODINGS: Record<string, string> = { br: "br", gz: "gzip" };

function extensionOf(name: string): string {
  const base = name.split("/").pop() ?? name;
  const dot = base.lastIndexOf(".");
  return dot > 0 ? base.slice(dot + 1).toLowerCase() : "";
}

/** Whether a build may contain this file. Files without an extension are allowed. */
export function allowedInBuild(name: string): boolean {
  const extension = extensionOf(name);
  if (!extension) return true;
  if (ENCODINGS[extension]) {
    const inner = extensionOf(name.slice(0, -(extension.length + 1)));
    return !inner || inner in TYPES;
  }
  return extension in TYPES;
}

/**
 * The headers that describe a build file. Unity's precompressed files, such
 * as game.wasm.br, are sent as their inner type with a matching encoding.
 */
export function describe(name: string): { type: string; encoding?: string } {
  const extension = extensionOf(name);
  const encoding = ENCODINGS[extension];
  if (encoding) {
    const inner = extensionOf(name.slice(0, -(extension.length + 1)));
    return { type: TYPES[inner] ?? "application/octet-stream", encoding };
  }
  return { type: TYPES[extension] ?? "application/octet-stream" };
}
