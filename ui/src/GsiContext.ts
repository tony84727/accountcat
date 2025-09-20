import { createContext } from "react";

export interface GsiContext {
	load?(): Promise<unknown>;
	loaded: boolean;
	username?: string;
	isAdmin?: boolean;
}

const context = createContext<GsiContext>({
	load: undefined,
	username: undefined,
	loaded: false,
	isAdmin: true,
});
export default context;
