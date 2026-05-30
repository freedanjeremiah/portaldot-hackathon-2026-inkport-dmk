/* POST /api/translate — fast codegen-only preview (no cargo build).
   Wire this to the real inkport-translate binary when running on the Linux dev machine:
     1. Write body.solidity to /tmp/inkport-playground/<sessionId>/<name>.sol
     2. Run: inkport-translate <file> --target seal --out <tmpdir>/build/<name>/
     3. Read back src/lib.rs + metadata.json
     4. Return { rust, metadata } */
export async function POST(_request: Request) {
  return Response.json(
    { error: 'Backend not connected. Run the playground on the same machine as inkport (Linux dev box).' },
    { status: 501 }
  );
}
