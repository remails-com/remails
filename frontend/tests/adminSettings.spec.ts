// As we directly import from playwright/test, we are not logged in automatically.
import { test, expect } from "@playwright/test";

test("manage API user", async ({ page }) => {
  await page.goto("http://localhost:3000/login");

  // Use login as super admin
  await page.getByRole("textbox", { name: "Email" }).fill("sudo@remails");
  await page.getByRole("textbox", { name: "Password" }).fill("unsecure123");
  await page.getByRole("button", { name: "Login" }).click();

  await page.getByRole('link', { name: 'Admin' }).click();
  await expect(page.getByRole("tabpanel", { name: "Config" })).toBeVisible();

  await page.getByRole("tab", { name: "Users" }).click();

  // make sure all operations take place in the correct row
  const row = page.getByRole("row").filter({ hasText: "Test API User 5" });
  const overlay = page.getByRole('dialog', { name: 'Manage user Test API User 5' });

  // make user admin
  await row.getByRole("button").locator(".tabler-icon.tabler-icon-edit").click();
  await overlay.getByLabel("Global role").click();
  await page.getByRole("option", { name: "admin" }).click();
  await overlay.getByRole("button", { name: "Save" }).click();
  await expect(page.getByText("User updated")).toBeVisible();
  await page.locator('.mantine-Modal-overlay').click(); // close overlay

  // check the user is actually admin now
  const nameCell = row.getByRole("cell").nth(1);
  await expect(nameCell).toContainText("Admin");

  // make user non-admin and block them
  await row.getByRole("button").locator(".tabler-icon.tabler-icon-edit").click();
  await overlay.getByLabel("Global role").click();
  await page.getByRole("option", { name: "admin" }).click();
  await overlay.getByLabel("Block user from accessing Remails").click();
  await overlay.getByRole("button", { name: "Save" }).click();
  await page.locator('.mantine-Modal-overlay').click(); // close overlay

  // check again
  await expect(nameCell).not.toContainText("Admin");
  await expect(nameCell).toContainText("Blocked");

  // unblock user again
  await row.getByRole("button").locator(".tabler-icon.tabler-icon-edit").click();
  await overlay.getByLabel("Block user from accessing Remails").click();
  await overlay.getByRole("button", { name: "Save" }).click();
  await page.locator('.mantine-Modal-overlay').click(); // close overlay

  // check again
  await expect(nameCell).not.toContainText("Admin");
  await expect(nameCell).not.toContainText("Blocked");
});
