import "normalize.css";
import createCache from "@emotion/cache";
import { CacheProvider } from "@emotion/react";
import Box from "@mui/material/Box";
import { createTheme, ThemeProvider } from "@mui/material/styles";
import Toolbar from "@mui/material/Toolbar";
import { LocalizationProvider } from "@mui/x-date-pickers";
import { AdapterDateFns } from "@mui/x-date-pickers/AdapterDateFns";
import { Empty } from "google-protobuf/google/protobuf/empty_pb";
import { lazy, useEffect, useMemo, useState } from "react";
import { BrowserRouter, Route, Routes } from "react-router";
import Bar from "./Bar.tsx";
import GsiContextProvider from "./GsiContextProvider.tsx";
import MenuDrawer from "./MenuDrawer.tsx";
import { UserClient } from "./proto/UserServiceClientPb";
import RequireLogin from "./RequireLogin.tsx";
import themeConfig from "./theme.ts";

const TodoList = lazy(() => import("./TodoList.tsx"));
const Accounting = lazy(() => import("./Accounting.tsx"));
const Intro = lazy(() => import("./Intro.tsx"));

const theme = createTheme(themeConfig);

const App = () => {
	const emotionCache = useMemo(
		() =>
			createCache({
				key: "mui",
				nonce: window.__webpack_nonce__,
				prepend: true,
			}),
		[],
	);
	const [clientId, setClientId] = useState<string>();
	useEffect(() => {
		const userClient = new UserClient("/api");
		userClient.getParam(new Empty()).then((response) => {
			setClientId(response.getGoogleClientId());
		});
	}, []);
	const [drawerOpen, setDrawerOpen] = useState(false);

	return (
		<CacheProvider value={emotionCache}>
			<ThemeProvider theme={theme}>
				<LocalizationProvider dateAdapter={AdapterDateFns}>
					<GsiContextProvider clientId={clientId}>
						<BrowserRouter>
							<Bar openDrawer={() => setDrawerOpen(true)}></Bar>
							<MenuDrawer
								open={drawerOpen}
								onClose={() => setDrawerOpen(false)}
							/>
							<Box>
								<Toolbar />
								<Routes>
									<Route index element={<Intro />} />
									<Route
										path="/todo/*"
										element={
											<RequireLogin>
												<TodoList />
											</RequireLogin>
										}
									/>
									<Route
										path="/accounting/*"
										element={
											<RequireLogin>
												<Accounting />
											</RequireLogin>
										}
									/>
								</Routes>
							</Box>
						</BrowserRouter>
					</GsiContextProvider>
				</LocalizationProvider>
			</ThemeProvider>
		</CacheProvider>
	);
};

export default App;
