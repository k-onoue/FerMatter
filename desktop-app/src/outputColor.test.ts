import { expect, it } from "vitest";
import {
  cardOutputColor,
  isBrightColor,
  isHexColor,
  parseHexInput,
} from "./outputColor";

it("accepts optional '#' in the picker but keeps saved card and history formats strict", () => {
  expect(parseHexInput("AbC123")).toBe("#abc123");
  expect(parseHexInput("#AbC123")).toBe("#abc123");
  expect(cardOutputColor("AbC123")).toBe("#AbC123");
  expect(cardOutputColor("#AbC123")).toBeNull();
  expect(isHexColor("#AbC123")).toBe(true);
  expect(isHexColor("AbC123")).toBe(false);
});

it("does not turn incomplete, alpha or invalid saved values into output colors", () => {
  for (const value of [
    "",
    "#abc",
    "abcd",
    "#abcd1234",
    "#gggggg",
    " #123456",
    "#123456 ",
    "##123456",
  ])
    expect(parseHexInput(value)).toBeNull();
  for (const value of [null, undefined, 123456, {}, "abc", "12345678"])
    expect(cardOutputColor(value)).toBeNull();
});

it("preserves the preview backdrop choice for light and dark ink", () => {
  expect(isBrightColor("#ffffff")).toBe(true);
  expect(isBrightColor("#DDDDDD")).toBe(true);
  expect(isBrightColor("#dbdbdb")).toBe(false);
  expect(isBrightColor("#2563eb")).toBe(false);
  expect(isBrightColor("#000000")).toBe(false);
});
