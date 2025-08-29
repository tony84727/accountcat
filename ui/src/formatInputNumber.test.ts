import { describe, expect, test } from "@rstest/core";
import formatInputNumber from "./formatInputNumber";

describe("formatInputNumber", () => {
	test("should allow input 0", () => {
		expect(formatInputNumber("0")).toBe("0");
	});
	test("should format non-number input to 0", () => {
		expect(formatInputNumber("hello")).toBe("0");
	});
	test("should not change integer input", () => {
		expect(formatInputNumber("10")).toBe("10");
	});
	test("should remove leading zeros", () => {
		expect(formatInputNumber("010")).toBe("10");
		expect(formatInputNumber("010.0")).toBe("10.0");
	});
	test("should allow on-going decimal input", () => {
		expect(formatInputNumber("10.")).toBe("10.");
	});
	test("should remove charaters other that digits and deciaml seperator", () => {
		expect(formatInputNumber("10,0")).toBe("100");
		expect(formatInputNumber("12b3ac")).toBe("123");
	});
});
