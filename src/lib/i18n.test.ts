import { describe, expect, it } from "vitest";
import { translate } from "./i18n";

describe("translate", () => {
  it("returns English sidebar language label", () => {
    expect(translate("en", "sidebar.language")).toBe("Language");
  });

  it("returns Portuguese sidebar language label", () => {
    expect(translate("pt-BR", "sidebar.language")).toBe("Idioma");
  });

  it("interpolates count", () => {
    expect(translate("en", "jobs.audios", { count: 3 })).toBe("3 audios");
    expect(translate("pt-BR", "jobs.audios", { count: 3 })).toBe("3 áudios");
  });
});
