import { afterEach, describe, expect, it, vi } from "vitest";
import { api } from "./client";
import type { RunModelSelection } from "./contracts";

describe("system usage api", () => {
  afterEach(() => vi.unstubAllGlobals());

  it("requests usage for the selected chat model", async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify({}), {
      status: 200,
      headers: { "Content-Type": "application/json" }
    }));
    vi.stubGlobal("fetch", fetchMock);
    const selection: RunModelSelection = { providerId: "provider-a", model: "model/large" };

    await api.system.usage(selection);

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/system/usage?provider_id=provider-a&model=model%2Flarge",
      expect.objectContaining({ credentials: "same-origin" })
    );
  });
});
