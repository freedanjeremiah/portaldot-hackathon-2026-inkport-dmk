/* POST /api/call — Contracts.call (mutating extrinsic) or dry-run (view).
   Body: { address: string, metadata: object, message: string, args: string[], sessionId: string }
   Returns: { result: any, events: object[] } or { error: string }

   Wire this by calling portaldot.py's call()/dry-run() with SCALE-encoded args decoded via test_contract.py. */
export async function POST(_request: Request) {
  return Response.json(
    { error: 'Backend not connected. Run the playground on the same machine as inkport.' },
    { status: 501 }
  );
}
