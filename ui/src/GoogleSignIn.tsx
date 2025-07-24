import { useEffect } from "react";

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

export default function (props: Props) {
	useEffect(() => {
		window.onGoogleLogin = (response) => {
			props.loginCallback(response);
		};
		return () => {
			delete window.onGoogleLogin;
		};
	}, [props.loginCallback]);
	return (
		<div
			id="g_id_onload"
			data-client_id={import.meta.env.PUBLIC_GOOGLE_CLIENT_ID}
			data-context="use"
			data-callback="onGoogleLogin"
			data-nonce=""
			data-itp_support="true"
		></div>
	);
}
