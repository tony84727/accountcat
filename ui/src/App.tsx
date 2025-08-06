import "normalize.css";
import { Empty } from "google-protobuf/google/protobuf/empty_pb";
import { lazy, useEffect, useState } from "react";
import { BrowserRouter, Route, Routes } from "react-router";
import { defer, from, Subject } from "rxjs";
import {
	map,
	mergeWith,
	share,
	startWith,
	switchMap,
	takeUntil,
} from "rxjs/operators";
import type { Response } from "./GoogleSignIn";
import Nav from "./Nav";
import { UserClient } from "./proto/UserServiceClientPb";
import { LoginRequest } from "./proto/user_pb";
import { useSubject } from "./rxjsutils";

const TodoList = lazy(() => import("./TodoList.tsx"));
const Accounting = lazy(() => import("./Accounting.tsx"));

const userClient = new UserClient("/api");

const App = () => {
	const [onLogin$, onLogin] = useSubject<Response>();
	const [promptLogin, setPromptLogin] = useState(false);
	const [username, setUsername] = useState<string>();
	useEffect(() => {
		const username$ = defer(() => userClient.getName(new Empty())).pipe(
			map((response) => response.getName()),
			mergeWith(
				onLogin$.pipe(
					switchMap(({ credential }) => {
						const request = new LoginRequest();
						request.setToken(credential);
						return from(userClient.login(request)).pipe(
							map((response) => response.getName()),
						);
					}),
				),
			),
			share(),
		);
		const promptLogin$ = username$.pipe(
			map((x) => !x),
			startWith(false),
		);
		const bye$ = new Subject();
		promptLogin$.pipe(takeUntil(bye$)).subscribe(setPromptLogin);
		username$.pipe(takeUntil(bye$)).subscribe(setUsername);

		return () => bye$.next(undefined);
	}, [onLogin$]);
	return (
		<BrowserRouter>
			<div>
				<Nav username={username} onLogin={onLogin} promptLogin={promptLogin} />
			</div>
			<Routes>
				<Route path="/todo/*" element={<TodoList />} />
				<Route path="/accounting/*" element={<Accounting />} />
			</Routes>
		</BrowserRouter>
	);
};

export default App;
