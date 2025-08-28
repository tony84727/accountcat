import type { ReactNode } from "react";
import GsiContext from "./GsiContext";
import Login from "./Login";

interface Props {
	username?: string;
	children: ReactNode;
}

export default function RequireLogin({ children }: Props) {
	return (
		<GsiContext.Consumer>
			{(gsi) => (gsi.username ? children : <Login />)}
		</GsiContext.Consumer>
	);
}
