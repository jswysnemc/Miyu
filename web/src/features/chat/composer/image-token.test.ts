import { describe, expect, it } from "vitest";
import { deleteImageTokenAtomically, insertImageToken, nextImageToken, removeImageToken } from "./image-token";

describe("image token", () => {
  it("reuses the smallest free image number", () => {
    expect(nextImageToken("[IMAGE-1] [IMAGE-3]", [])).toBe("[IMAGE-2]");
  });

  it("inserts a token with readable spacing", () => {
    expect(insertImageToken("前后", 1, 1, "[IMAGE-1]")).toEqual({
      value: "前 [IMAGE-1] 后",
      caret: 12
    });
  });

  it("deletes a whole token with backspace", () => {
    expect(deleteImageTokenAtomically("说明 [IMAGE-1] 后续", 12, 12, "Backspace")).toEqual({
      value: "说明 后续",
      caret: 2
    });
  });

  it("expands a partial selection over a token", () => {
    expect(deleteImageTokenAtomically("a [IMAGE-1] b", 4, 7, "Delete")).toEqual({ value: "a  b", caret: 2 });
  });

  it("removes a preview token from text", () => {
    expect(removeImageToken("查看 [IMAGE-1] 内容", "[IMAGE-1]")).toBe("查看 内容");
  });
});
