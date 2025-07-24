import "normalize.css";
import { useCallback, useState } from "react";
import GoogleSignIn, { type Response } from "./GoogleSignIn";
import Nav from "./Nav";

interface Identity {
	name: string;
}

const App = () => {
	const [username, setUsername] = useState<string | undefined>(undefined);
	const login = useCallback(async (response: Response) => {
		const r = await fetch("/api/login", {
			method: "post",
			body: response.credential,
		});
		const identity: Identity = await r.json();
		setUsername(identity.name);
	}, []);
	return (
		<div>
			<Nav username={username} />
			<GoogleSignIn loginCallback={login} />
		</div>
	);
};

export default App;
