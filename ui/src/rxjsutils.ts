import type { Dispatch, SetStateAction } from "react";
import { type Observable, Subject } from "rxjs";

export function createNotifier(
	updateDispatch: Dispatch<SetStateAction<(() => void) | undefined>>,
) {
	const event$ = new Subject();
	updateDispatch(() => () => event$.next(undefined));
	return event$;
}

export function createCallback<T>(
	updateDispatch: Dispatch<SetStateAction<((event: T) => void) | undefined>>,
): Observable<T> {
	const event$ = new Subject<T>();
	updateDispatch(() => (event: T) => event$.next(event));
	return event$;
}

export function createMultiArgumentCallback<T extends unknown[]>(
	updateDispatch: Dispatch<SetStateAction<((...event: T) => void) | undefined>>,
) {
	const event$ = new Subject<T>();
	updateDispatch(
		() =>
			(...event: T) =>
				event$.next(event),
	);
	return event$;
}
