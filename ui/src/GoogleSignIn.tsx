import { useContext, useEffect } from "react";
import GsiContext from "./GsiContext";

export interface Response {
	credential: string;
}

export default function GoogleSignIn() {
	const gsiContext = useContext(GsiContext);
	useEffect(() => {
		if (!gsiContext.loaded) {
			gsiContext.load?.();
			return;
		}
		google.accounts.id.prompt();
	}, [gsiContext.loaded, gsiContext.load]);
	return null;
}
