import { defineConfig } from "@rstest/core";

export default defineConfig({
	exclude: ["**/node_modules/**", "tests/**"],
});
