export async function submitDecision(params: {
  action: string;
  symbol?: string;
  side?: string;
  size_pct?: number;
  reasoning: string;
}): Promise<{ ok: boolean; error?: string }> {
  try {
    const response = await fetch("/api/brain/decide", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(params),
    });
    if (response.ok) {
      return { ok: true };
    }
    return { ok: false, error: `HTTP ${response.status}` };
  } catch (e) {
    return { ok: false, error: (e as Error).message };
  }
}
