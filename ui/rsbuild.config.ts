import { defineConfig, loadEnv } from "@rsbuild/core";
import { pluginReact } from "@rsbuild/plugin-react";
import { pluginSass } from "@rsbuild/plugin-sass";

const { publicVars } = loadEnv();

export default defineConfig({
	plugins: [pluginReact(), pluginSass()],
	html: {
		title: "AccountCat",
		meta: {
			nonce: "__CSP_NONCE__",
		},
	},
	source: {
		define: publicVars,
	},
	server: {
		proxy: {
			"/api": "http://localhost:3000",
		},
	},
	security: {
		nonce: "__CSP_NONCE__",
	},
});
