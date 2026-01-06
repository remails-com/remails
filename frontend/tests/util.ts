import { Page } from "@playwright/test";
import { v4 as uuid } from "uuid";
import { expect } from "playwright/test";

export const uuidRegex = "[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}";

export async function createProject(page: Page): Promise<string> {
  const projectUuid = uuid();

  // Create a new project
  await expect(page.getByRole("button", { name: "New Project" })).toBeVisible();
  await page.getByRole("button", { name: "New Project" }).click();
  await page.getByRole("textbox", { name: "Name" }).fill(projectUuid);
  await page.getByRole("button", { name: "Save" }).click();

  return projectUuid;
}

export async function deleteProject(page: Page) {
  await page.goto("/");
  await page.getByRole("row").getByRole("button").locator(".tabler-icon.tabler-icon-edit").click();
  await page.getByRole("button", { name: "Delete" }).click();
  await page.getByRole("button", { name: "Confirm" }).click();
}

export async function createAccount(page: Page) {
  await page.goto("http://localhost:3000/login?type=register");

  await page.getByRole("textbox", { name: "Name" }).fill("Playwright");
  await page.getByRole("textbox", { name: "Email" }).fill(`${uuid()}@playwrighttest.com`);
  await page.getByRole("textbox", { name: "Password" }).fill("playwrighttest");
  await page.getByRole("button", { name: "Register" }).click();
  await expect(page.getByRole("textbox", { name: "Name" })).toBeFocused();
  await page.getByRole("textbox", { name: "Name" }).fill("test-organization");
  await page.getByRole("button", { name: "Create Organization" }).click();
  await page.getByRole("button", { name: "Choose your subscription" }).click();

  await expect(async () => {
    await page.getByRole("main").getByRole("button").locator(".tabler-icon.tabler-icon-reload").click();
    await expect(page.getByText("Organization", { exact: true })).toBeVisible();
  }).toPass();
}
