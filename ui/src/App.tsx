import "normalize.css";
import createCache from "@emotion/cache";
import { CacheProvider } from "@emotion/react";
import Box from "@mui/material/Box";
import { createTheme, ThemeProvider } from "@mui/material/styles";
import Toolbar from "@mui/material/Toolbar";
import { Empty } from "google-protobuf/google/protobuf/empty_pb";
import { lazy, useEffect, useMemo, useState } from "react";
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
import Bar from "./Bar.tsx";
import type { Response } from "./GoogleSignIn";
import MenuDrawer from "./MenuDrawer.tsx";
import { UserClient } from "./proto/UserServiceClientPb";
import { LoginRequest } from "./proto/user_pb";
import { useSubject } from "./rxjsutils";
import themeConfig from "./theme.ts";

const TodoList = lazy(() => import("./TodoList.tsx"));
const Accounting = lazy(() => import("./Accounting.tsx"));
const Intro = lazy(() => import("./Intro.tsx"));

const userClient = new UserClient("/api");

const theme = createTheme(themeConfig);

const App = () => {
	const [onLogin$, onLogin] = useSubject<Response>();
	const [promptLogin, setPromptLogin] = useState(false);
	const [username, setUsername] = useState<string>();
	const emotionCache = useMemo(
		() =>
			createCache({
				key: "mui",
				nonce: window.__webpack_nonce__,
				prepend: true,
			}),
		[],
	);
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
	const [drawerOpen, setDrawerOpen] = useState(false);

	return (
		<CacheProvider value={emotionCache}>
			<ThemeProvider theme={theme}>
				<BrowserRouter>
					<Bar
						openDrawer={() => setDrawerOpen(true)}
						username={username}
						promptLogin={promptLogin}
						onLogin={onLogin}
					></Bar>
					<MenuDrawer
						username={username}
						open={drawerOpen}
						onClose={() => setDrawerOpen(false)}
					/>
					<Box>
						<Toolbar />
						<Routes>
							<Route index element={<Intro />} />
							<Route path="/todo/*" element={<TodoList />} />
							<Route path="/accounting/*" element={<Accounting />} />
						</Routes>
					</Box>
				</BrowserRouter>
			</ThemeProvider>
		</CacheProvider>
	);
};

export default App;
