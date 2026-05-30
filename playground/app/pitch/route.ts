import { readFileSync } from "node:fs";
import { join } from "node:path";

// Serve the self-contained pitch deck (playground/pitch/index.html) at /pitch.
export const dynamic = "force-static";

export function GET() {
  const html = readFileSync(join(process.cwd(), "pitch", "index.html"), "utf8");
  return new Response(html, {
    headers: { "content-type": "text/html; charset=utf-8" },
  });
}
