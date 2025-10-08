import { expect, test } from "@playwright/test";

const loginPromptMessage = "請先透過Google登入";

test("accounting page require login", async ({ page }) => {
	await page.goto("http://localhost:3000/accounting");
	await expect(page.getByText(loginPromptMessage)).toBeVisible();
});

test("accounting page require insight", async ({ page }) => {
	await page.goto("http://localhost:3000/insight");
	await expect(page.getByText(loginPromptMessage)).toBeVisible();
});

test("accounting page require todo", async ({ page }) => {
	await page.goto("http://localhost:3000/todo");
	await expect(page.getByText(loginPromptMessage)).toBeVisible();
});
