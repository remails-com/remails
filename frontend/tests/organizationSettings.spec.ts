import { test, expect } from "./fixtures.ts";
import { test as baseTest } from "@playwright/test";
import { createAccount, uuidRegex } from "./util.ts";
import { Page } from "@playwright/test";

test("rename organization", async ({ page }) => {
  await page.goto("/");

  // Navigate to organization settings
  await page.locator("a").filter({ hasText: "Settings" }).click();
  await expect(page.getByRole("tabpanel", { name: "Subscription" })).toBeVisible();

  // rename organization
  await page.getByRole("heading").filter({ hasNotText: "Your subscription" }).click();
  await page.getByRole("textbox").fill("renamed organization");
  await expect(page.locator(".tabler-icon.tabler-icon-check")).toBeVisible();
  await page.locator(".tabler-icon.tabler-icon-check").click();
  await expect(page.getByRole("main")).toContainText("renamed organization");
});

async function toOrganizationMembers(page: Page) {
  await page.goto("/");

  // Navigate to organization settings
  await page.locator("a").filter({ hasText: "Settings" }).click();
  await expect(page.getByRole("tabpanel", { name: "Subscription" })).toBeVisible();

  // Open members tab
  await page.getByRole("tab", { name: "Members" }).click();
  await expect(page.getByRole("heading", { name: "Organization members" })).toBeVisible();
}

// Create and delete an organization invite
test("manage organization invite", async ({ page }) => {
  await toOrganizationMembers(page);

  // Create new invite link
  await page.getByRole("button", { name: "New invite link" }).click();
  await expect(page.getByRole("dialog", { name: "Create new invite link" })).toBeVisible();

  // Fill in invite details
  await page.getByRole("textbox", { name: "Organization role" }).click();
  await expect(page.getByRole("listbox", { name: "Organization role" })).toBeVisible();

  await page.getByRole("option", { name: "Maintainer" }).click();
  await page.getByRole("button", { name: "Create", exact: true }).click();

  // Confirm invite links table is visible
  await page.getByRole("button", { name: "Done" }).click();
  await expect(page.getByRole("heading", { name: "Organization invites" })).toBeVisible();

  // Confirm the new invite is listed and has the correct role
  await expect(page.getByRole("cell", { name: "Maintainer" })).toBeVisible();
  // And correct 'created by' user
  const count = await page.getByRole("cell", { name: "Playwright", exact: true }).count();
  expect(count).toBe(2);

  // Delete the invite
  await page.locator(".tabler-icon.tabler-icon-trash").click();
  await expect(page.getByRole("dialog", { name: "Please confirm your action" })).toBeVisible();

  await page.getByRole("button", { name: "Confirm" }).click();
});

// Create and accept an organization invite
// Using 'baseTest' to avoid using an already signed-in state
baseTest("accept organization invite", async ({ browser }) => {
  test.slow();

  const context1 = await browser.newContext({ storageState: undefined });
  const context2 = await browser.newContext({ storageState: undefined });
  const page1 = await context1.newPage();
  const page2 = await context2.newPage();

  await createAccount(page1);
  const projectPage = page1.url();

  await createAccount(page2);

  // Create invite link with first account
  await toOrganizationMembers(page1);
  await page1.getByRole("button", { name: "New invite link" }).click();
  await expect(page1.getByRole("dialog", { name: "Create new invite link" })).toBeVisible();
  await page1.getByRole("textbox", { name: "Organization role" }).click();

  await page1.getByRole("option", { name: "Maintainer" }).click();
  await page1.getByRole("button", { name: "Create", exact: true }).click();

  const inviteLink = await page1.getByText("/invite/").textContent();

  await page1.getByRole("button", { name: "Done" }).click();

  await expect(page1.getByRole("dialog", { name: "Create new invite link" })).not.toBeVisible();

  await page2.goto(inviteLink!);

  // Confirm joining the organization
  await page2.getByRole("button", { name: "Join organization" }).click();

  // Check that the second user could join the organization of the first user
  await expect(page2).toHaveURL((url) => projectPage.startsWith(url.toString()));
  await expect(page2.getByRole("button", { name: "New Project" })).toBeVisible();
  // Check that it is allowed to create new projects
  await expect(page2.getByRole("button", { name: "New Project" })).not.toBeDisabled();
});

test("organization API key", async ({ page }) => {
  await page.goto("/");

  // Navigate to organization settings
  await page.locator("a").filter({ hasText: "Settings" }).click();
  await expect(page.getByRole("tabpanel", { name: "Subscription" })).toBeVisible();

  // Open API keys tab
  await page.getByRole("tab", { name: "API Keys" }).click();
  await expect(page.getByRole("button", { name: "New API Key" })).toBeVisible();

  // Create new API key
  await page.getByRole("button", { name: "New API Key" }).click();
  await expect(page.getByRole("dialog", { name: "Create new API key" })).toBeVisible();

  // Configure access level
  await page.getByRole("textbox", { name: "Access level" }).click();
  await expect(page.getByRole("listbox", { name: "Access level" })).toBeVisible();
  await page.getByRole("option", { name: "Read-only" }).click();

  // Fill in description
  await page.getByRole("textbox", { name: "Description" }).click();
  await page.getByRole("textbox", { name: "Description" }).fill("Playwright test API key");

  await page.getByRole("button", { name: "Create", exact: true }).click();

  // Confirm API key password is visible
  await expect(page.getByText("Password", { exact: true })).toBeVisible();
  const key_id = (await page.locator("div").locator("pre").first().textContent()) || "";

  // Confirm dialog
  await page.getByRole("button", { name: "Done" }).click();

  // Check we are put on the API keys page
  {
    const expectedUrl = new RegExp(`${uuidRegex}/settings/api_keys`);
    await expect(page).toHaveURL(expectedUrl);
    await expect(page.getByRole("button", { name: "New API Key" })).toBeVisible();
  }

  // Confirm the new API key is listed with correct details
  const row = page.getByRole("row").filter({ hasText: key_id });
  await expect(row.getByRole("cell", { name: "Read-only" })).toBeVisible();
  await expect(row.getByRole("cell", { name: "Playwright test API key", exact: true })).toBeVisible();

  // Open API key details
  await row.getByRole("link").locator(".tabler-icon.tabler-icon-edit").click();

  // Check we are put on the API key details page
  {
    const expectedUrl = new RegExp(`${uuidRegex}/settings/api_keys/${uuidRegex}`);
    await expect(page).toHaveURL(expectedUrl);
  }

  // Confirm details are correct
  await expect(page.getByLabel("Description")).toContainText("Playwright test API key");
  await expect(page.getByRole("textbox", { name: "Access level" })).toHaveValue("Read-only");

  // Delete the API key
  await page.getByRole("button", { name: "Delete" }).click();

  // Confirm deletion dialog is visible and shows the correct API key name
  await expect(page.getByLabel("Please confirm your action").getByText("Playwright test API key")).toBeVisible();

  // Confirm deletion
  await page.getByRole("button", { name: "Confirm" }).click();

  // confirm success message is visible
  await expect(page.getByText("API key deleted", { exact: true })).toBeVisible();

  // Confirm API key is no longer listed
  await expect(page.getByRole("cell", { exact: true, name: key_id })).not.toBeVisible();
});
