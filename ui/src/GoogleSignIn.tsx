import { Empty } from "google-protobuf/google/protobuf/empty_pb";
import { useEffect, useState } from "react";
import { UserClient } from "./proto/UserServiceClientPb";

export interface Response {
	credential: string;
}
interface Props {
	clientId: string;
	loginCallback(response: Response): void;
}

declare global {
	interface Window {
		onGoogleLogin?(response: Response): void;
	}
}

export default function GoogleSignIn(props: Props) {
	const [clientId, setClientId] = useState<string>();
	useEffect(() => {
		const userClient = new UserClient("/api");
		userClient
			.getParam(new Empty())
			.then((param) => setClientId(param.getGoogleClientId()));
	}, []);
	useEffect(() => {
		if (!clientId) {
			return;
		}
		const clientScriptTag = document.createElement("script");
		clientScriptTag.src = "https://accounts.google.com/gsi/client";
		clientScriptTag.async = true;
		clientScriptTag.nonce = window.__webpack_nonce__;
		clientScriptTag.onload = () => {
			google.accounts.id.initialize({
				client_id: clientId,
				callback: props.loginCallback,
			});
			google.accounts.id.prompt();
			window.onGoogleLogin = (response) => {
				props.loginCallback(response);
			};
		};
		document.body.appendChild(clientScriptTag);

		return () => {
			if (window.google) {
				google.accounts.id.cancel();
			}
			clientScriptTag.remove();
			delete window.onGoogleLogin;
		};
	}, [props.loginCallback, clientId]);
	return null;
}
