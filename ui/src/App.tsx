import "normalize.css";
import { defer } from "rxjs";
import { ajax } from "rxjs/ajax";
import { map, mergeWith, share, startWith, switchMap } from "rxjs/operators";
import type { Response } from "./GoogleSignIn";
import Nav from "./Nav";
import { unwrapResponse, useObservable, withSubject } from "./rxjsutils";

const [onLogin$, onLogin] = withSubject<Response>();
const username$ = defer(() => ajax.get<string>("/api/name")).pipe(
	unwrapResponse(),
	mergeWith(
		onLogin$.pipe(
			switchMap(({ credential }) =>
				ajax.post<{ name: string }>("/api/login", credential).pipe(
					unwrapResponse(),
					map(({ name }) => name),
				),
			),
		),
	),
	share(),
);
const promptLogin$ = username$.pipe(
	map((x) => !x),
	startWith(false),
);

const App = () => {
	const promptLogin = useObservable(promptLogin$);
	const username = useObservable(username$);
	return (
		<div>
			<Nav username={username} onLogin={onLogin} promptLogin={promptLogin} />
		</div>
	);
};

export default App;
