// @vitest-environment jsdom
import { cleanup, renderHook } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { defaults } from "./appearance";
import { useAppearance } from "./useAppearance";

const { rpc, configureTheme, logError } = vi.hoisted(() => ({
  rpc: vi.fn().mockResolvedValue(null),
  configureTheme: vi.fn(),
  logError: vi.fn(),
}));
vi.mock("./api", () => ({ rpc }));
vi.mock("./editor", () => ({ configureTheme }));
vi.mock("./log", () => ({ logError }));

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.clearAllMocks();
  document.documentElement.removeAttribute("style");
  delete document.documentElement.dataset.appearance;
});

it.each(["Win32", "Linux x86_64"])(
  "matches the native frame and menu to the selected palette on %s",
  (platform) => {
    vi.spyOn(navigator, "platform", "get").mockReturnValue(platform);
    const theme = {
      ...defaults,
      dark: { ...defaults.dark, windowBackground: "#234567" },
    };
    const { rerender } = renderHook(
      ({ dark, opacity }) => useAppearance(theme, dark, opacity),
      { initialProps: { dark: true, opacity: 0.88 } },
    );
    expect(rpc).toHaveBeenLastCalledWith("apply_window_appearance", {
      dark: true,
      background: "#234567",
    });
    expect(
      document.documentElement.style.getPropertyValue("--windowBackground"),
    ).toBe("#234567");
    expect(document.documentElement.dataset.appearance).toBe("dark");

    rerender({ dark: true, opacity: 0.6 });
    expect(rpc).toHaveBeenCalledTimes(1);
    expect(document.documentElement.style.getPropertyValue("--opacity")).toBe(
      "0.6",
    );

    rerender({ dark: false, opacity: 0.6 });
    expect(rpc).toHaveBeenLastCalledWith("apply_window_appearance", {
      dark: false,
      background: defaults.light.windowBackground,
    });
  },
);

it("keeps macOS native translucency independent of the CSS palette", () => {
  vi.spyOn(navigator, "platform", "get").mockReturnValue("MacIntel");
  renderHook(() => useAppearance(defaults, true, 0.72));
  expect(rpc).not.toHaveBeenCalled();
  expect(configureTheme).toHaveBeenCalledWith(defaults.dark, true);
  expect(document.documentElement.style.getPropertyValue("--opacity")).toBe(
    "0.72",
  );
});

it("restores the current Windows palette on focus and removes obsolete listeners", () => {
  vi.spyOn(navigator, "platform", "get").mockReturnValue("Win32");
  const { rerender, unmount } = renderHook(
    ({ dark }) => useAppearance(defaults, dark, 1),
    { initialProps: { dark: true } },
  );
  rpc.mockClear();
  window.dispatchEvent(new Event("focus"));
  expect(rpc).toHaveBeenCalledExactlyOnceWith("apply_window_appearance", {
    dark: true,
    background: defaults.dark.windowBackground,
  });

  rerender({ dark: false });
  rpc.mockClear();
  window.dispatchEvent(new Event("focus"));
  expect(rpc).toHaveBeenCalledExactlyOnceWith("apply_window_appearance", {
    dark: false,
    background: defaults.light.windowBackground,
  });

  unmount();
  rpc.mockClear();
  window.dispatchEvent(new Event("focus"));
  expect(rpc).not.toHaveBeenCalled();
});
