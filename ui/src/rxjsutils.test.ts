import { describe, expect, test } from "@rstest/core";
import { createNotifier, createCallback, createMultiArgumentCallback } from "./rxjsutils";
import type { Dispatch, SetStateAction } from "react";

describe("rxjsutils", () => {
	describe("createNotifier", () => {
		test("should set dispatch and emit when called", () => {
			let dispatchAction: SetStateAction<(() => void) | undefined> | undefined;
			const mockDispatch: Dispatch<SetStateAction<(() => void) | undefined>> = (action) => {
				dispatchAction = action;
			};

			const event$ = createNotifier(mockDispatch);

			expect(typeof event$.next).toBe("function");
			expect(typeof event$.subscribe).toBe("function");
			expect(dispatchAction).toBeDefined();

			const setFunction = dispatchAction as () => () => void;
			const notifier = setFunction();

			let emitted = false;
			let emittedValue: any = "unemitted";
			event$.subscribe((val) => {
				emitted = true;
				emittedValue = val;
			});

			notifier();

			expect(emitted).toBe(true);
			expect(emittedValue).toBe(undefined);
		});
	});

    describe("createCallback", () => {
		test("should set dispatch and emit event when called", () => {
			let dispatchAction: any;
			const mockDispatch: any = (action: any) => {
				dispatchAction = action;
			};

			const event$ = createCallback<string>(mockDispatch);

			expect(dispatchAction).toBeDefined();

			const setFunction = dispatchAction as () => (event: string) => void;
			const callback = setFunction();

			let emitted = false;
			let emittedValue: any;
			event$.subscribe((val) => {
				emitted = true;
				emittedValue = val;
			});

			callback("test-value");

			expect(emitted).toBe(true);
			expect(emittedValue).toBe("test-value");
		});
	});

    describe("createMultiArgumentCallback", () => {
		test("should set dispatch and emit array of arguments when called", () => {
			let dispatchAction: any;
			const mockDispatch: any = (action: any) => {
				dispatchAction = action;
			};

			const event$ = createMultiArgumentCallback<[string, number]>(mockDispatch);

			expect(dispatchAction).toBeDefined();

			const setFunction = dispatchAction as () => (...event: [string, number]) => void;
			const callback = setFunction();

			let emitted = false;
			let emittedValue: any;
			event$.subscribe((val) => {
				emitted = true;
				emittedValue = val;
			});

			callback("test-arg", 42);

			expect(emitted).toBe(true);
			expect(emittedValue).toEqual(["test-arg", 42]);
		});
	});
});
