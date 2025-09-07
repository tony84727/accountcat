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
		favicon: "./src/assets/favicon.ico",
	},
	server: {
		proxy: {
			"/api": "http://localhost:3000",
		},
	},
	security: {
		nonce: "__CSP_NONCE__",
	},
	tools: {
		rspack: {
			experiments: {
				lazyBarrel: false,
			},
		},
	},
});
