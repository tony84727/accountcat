import { defineConfig, loadEnv } from "@rsbuild/core";
import { pluginReact } from "@rsbuild/plugin-react";
import { pluginSass } from "@rsbuild/plugin-sass";

const { publicVars } = loadEnv();

export default defineConfig({
	plugins: [pluginReact(), pluginSass()],
	html: {
		title: "AccountCat",
		tags: [
			{
				tag: "script",
				attrs: {
					async: true,
					src: "https://accounts.google.com/gsi/client",
				},
			},
		],
	},
	source: {
		define: publicVars,
	},
	server: {
		proxy: {
			"/api": "http://localhost:3000",
		},
	},
});
