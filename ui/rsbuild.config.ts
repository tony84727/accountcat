import { defineConfig } from "@rsbuild/core";
import { pluginReact } from "@rsbuild/plugin-react";
import { pluginSass } from "@rsbuild/plugin-sass";

export default defineConfig({
	plugins: [pluginReact(), pluginSass()],
	html: {
		title: "AccountCat",
		meta: {
			nonce: "__CSP_NONCE__",
		},
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
