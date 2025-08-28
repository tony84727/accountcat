import { createContext } from "react";

export interface GsiContext {
	load?(): Promise<unknown>;
	loaded: boolean;
	username?: string;
}

const context = createContext<GsiContext>({
	load: undefined,
	username: undefined,
	loaded: false,
});
export default context;
