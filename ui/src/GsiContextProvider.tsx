import { Empty } from "google-protobuf/google/protobuf/empty_pb";
import { type ReactNode, useEffect, useMemo, useState } from "react";
import type { Response } from "./GoogleSignIn";
import GsiContext from "./GsiContext";
import { UserClient } from "./proto/UserServiceClientPb";
import { LoginRequest } from "./proto/user_pb";

interface GsiContextProviderProps {
	clientId?: string;
	children: ReactNode;
}

export default function GsiContextProvider({
	clientId,
	children,
}: GsiContextProviderProps) {
	const [username, setUsername] = useState<string>();
	const [loaded, setLoaded] = useState(false);
	const [isAdmin, setIsAdmin] = useState(false);
	const context = useMemo(
		() => ({
			load: () => {
				if (loaded) {
					return Promise.resolve();
				}
				const userClient = new UserClient("/api");
				if (!clientId) {
					return Promise.resolve();
				}
				const clientScriptTag = document.createElement("script");
				clientScriptTag.src = "https://accounts.google.com/gsi/client";
				clientScriptTag.async = true;
				clientScriptTag.nonce = window.__webpack_nonce__;

				const callback = async (response: Response) => {
					const loginRequest = new LoginRequest();
					loginRequest.setToken(response.credential);
					const loginResponse = await userClient.login(loginRequest);
					setUsername(loginResponse.getName());
					setIsAdmin(loginResponse.getIsAdmin());
				};
				const loading = new Promise((resolved) => {
					clientScriptTag.onload = () => {
						google.accounts.id.initialize({
							client_id: clientId,
							callback: callback,
						});
						setLoaded(true);
						resolved(undefined);
					};
					document.body.appendChild(clientScriptTag);
				});
				return loading;
			},
			username,
			loaded,
			isAdmin,
		}),
		[clientId, username, loaded, isAdmin],
	);
	useEffect(() => {
		const userClient = new UserClient("/api");
		userClient.getProfile(new Empty()).then((response) => {
			setUsername(response.getName());
			setIsAdmin(response.getIsAdmin());
		});
	}, []);
	return <GsiContext value={context}>{children}</GsiContext>;
}
