import { useEffect, useRef } from "react";

export interface Response {
	credential: string;
}
interface Props {
	loginCallback(response: Response): void;
}

declare global {
	interface Window {
		onGoogleLogin?(response: Response): void;
	}
}

export default function GoogleSignIn(props: Props) {
	const button = useRef(null);
	useEffect(() => {
		const clientScriptTag = document.createElement("script");
		clientScriptTag.src = "https://accounts.google.com/gsi/client";
		clientScriptTag.async = true;
		clientScriptTag.onload = () => {
			google.accounts.id.initialize({
				client_id: import.meta.env.PUBLIC_GOOGLE_CLIENT_ID,
				callback: props.loginCallback,
			});
			if (button.current) {
				google.accounts.id.renderButton(button.current, {
					theme: "outline",
					size: "medium",
					type: "standard",
				});
			}

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
	}, [props.loginCallback]);
	return;
}
