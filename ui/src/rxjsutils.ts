import {
	type Dispatch,
	type SetStateAction,
	useEffect,
	useMemo,
	useState,
} from "react";
import { map, type Observable, Subject } from "rxjs";
import type { AjaxResponse } from "rxjs/ajax";

export function useObservable<T>(observable: Observable<T>) {
	const [state, setState] = useState<T>();
	useEffect(() => {
		const subscription = observable.subscribe((x) => setState(x));
		return () => subscription.unsubscribe();
	}, [observable]);
	return state;
}

export function unwrapResponse<T>() {
	return map(({ response }: AjaxResponse<T>) => response);
}

export function useSubject<T = undefined>(): [
	Observable<T>,
	(event: T) => void,
] {
	const subject = useMemo(() => new Subject<T>(), []);
	return [subject, (x: T) => subject.next(x)];
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
