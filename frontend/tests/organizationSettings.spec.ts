import { test, expect } from "./fixtures.ts";
import { test as baseTest } from "@playwright/test";
import { createAccount, uuidRegex } from "./util.ts";
import { Page } from "@playwright/test";

test("rename organization", async ({ page }) => {
  await page.goto("/");

  // Navigate to organization settings
  await page.getByRole('link', { name: 'Organization', exact: true }).click();
  await page.getByRole('link', { name: 'Subscription', exact: true }).click();

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
  await page.getByRole('link', { name: 'Organization', exact: true }).click();

  // Open members tab
  await page.getByRole('link', { name: 'Members', exact: true }).click();
  await expect(page.getByRole("heading", { name: "Organization members" })).toBeVisible();
}

// Create and delete an organization invite
test("manage organization invite", async ({ page }) => {
  await toOrganizationMembers(page);

  // Create new invite link
  await page.getByRole("button", { name: "New invite link" }).click();
  await expect(page.getByRole("dialog", { name: "Create new invite link" })).toBeVisible();

  // Fill in invite details
  await page.getByRole("combobox", { name: "Organization role" }).click();
  await expect(page.getByRole("combobox", { name: "Organization role" })).toBeVisible();

  await page.getByRole("option", { name: "Maintainer" }).click();
  await page.getByRole("button", { name: "Create", exact: true }).click();

  // Confirm invite links table is visible
  await page.getByRole("button", { name: "Done" }).click();
  await expect(page.getByRole("heading", { name: "Organization invites" })).toBeVisible();

  // Confirm the new invite is listed and has the correct role
  await expect(page.getByRole("cell", { name: "Maintainer" })).toBeVisible();
  // And correct 'created by' user
  const count = await page.getByRole("cell", { name: "Playwright" }).count();
  expect(count).toBe(2);

  // Delete the invite
  await page.locator(".tabler-icon.tabler-icon-trash").click();
  await expect(page.getByRole("dialog", { name: "Please confirm your action" })).toBeVisible();

  await page.getByLabel('Please confirm your action').getByRole("button", { name: "Delete" }).click();
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
  await page1.getByRole("combobox", { name: "Organization role" }).click();

  await page1.getByRole("option", { name: "Read-only" }).click();
  await page1.getByRole("button", { name: "Create", exact: true }).click();

  const inviteLink = await page1.getByText("/invite/").textContent();

  await page1.getByRole("button", { name: "Done" }).click();

  await expect(page1.getByRole("dialog", { name: "Create new invite link" })).not.toBeVisible();

  await page2.goto(inviteLink!);

  // Confirm joining the organization
  await page2.getByRole("button", { name: "Join organization" }).click();

  // Check that the second user could join the organization of the first user
  await expect(page2).toHaveURL((url) => url.toString().startsWith(projectPage));
  await expect(page2.getByRole("button", { name: "New project" })).toBeVisible();

  // Check that it is not allowed to create new projects because it is read-only
  await expect(page2.getByRole("button", { name: "New project" })).toBeDisabled();
});

test("organization API key", async ({ page }) => {
  await page.goto("/");

  // Navigate to organization settings
  await page.getByRole('link', { name: 'Organization', exact: true }).click();

  // Open API keys tab
  await page.getByRole('link', { name: 'API keys', exact: true }).click();
  await expect(page.getByRole("button", { name: "New API key" })).toBeVisible();

  // Create new API key
  await page.getByRole("button", { name: "New API key" }).click();
  await expect(page.getByRole("dialog", { name: "Create new API key" })).toBeVisible();

  // Configure access level
  await page.getByRole("combobox", { name: "Access level" }).click();
  await expect(page.getByRole("combobox", { name: "Access level" })).toBeVisible();
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
    const expectedUrl = new RegExp(`${uuidRegex}/organization/api-keys`);
    await expect(page).toHaveURL(expectedUrl);
    await expect(page.getByRole("button", { name: "New API key" })).toBeVisible();
  }

  // Confirm the new API key is listed with correct details
  const row = page.getByRole("row").filter({ hasText: key_id.split("-")[0] });
  await expect(row.getByRole("cell", { name: "Read-only" })).toBeVisible();
  await expect(row.getByRole("cell", { name: "Playwright test API key", exact: true })).toBeVisible();

  // Open API key details
  await row.getByRole("link").locator(".tabler-icon.tabler-icon-edit").click();

  // Check we are put on the API key details page
  {
    const expectedUrl = new RegExp(`${uuidRegex}/organization/api-keys/${uuidRegex}`);
    await expect(page).toHaveURL(expectedUrl);
  }

  // Confirm details are correct
  await expect(page.getByLabel("Description")).toContainText("Playwright test API key");
  await expect(page.getByRole("combobox", { name: "Access level" })).toHaveValue("Read-only");

  // Delete the API key
  await page.getByRole("button", { name: "Delete" }).click();

  // Confirm deletion dialog is visible and shows the correct API key name
  const modal = page.getByLabel("Please confirm your action");
  await expect(modal.getByText("Playwright test API key")).toBeVisible();

  // Confirm deletion
  await modal.getByRole("button", { name: "Delete" }).click();

  // confirm success message is visible
  await expect(page.getByText("API key deleted", { exact: true })).toBeVisible();

  // Confirm API key is no longer listed
  await expect(page.getByRole("cell", { exact: true, name: key_id })).not.toBeVisible();
});
