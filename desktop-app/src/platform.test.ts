import { expect, it } from "vitest";
import { fileManagerName } from "./platform";

it("names Finder on macOS and a generic file manager on Linux", () => {
  expect(
    fileManagerName(
      "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko)",
    ),
  ).toBe("Finder");
  expect(
    fileManagerName(
      "Mozilla/5.0 (X11; Ubuntu; Linux x86_64) AppleWebKit/605.1.15 (KHTML, like Gecko)",
    ),
  ).toBe("File Manager");
});
