import { createAccount, uuidRegex } from "./util.ts";
// As we directly import from playwright/test, we are not logged in automatically.
import { Page, test, expect } from "@playwright/test";

async function toUserSettings(page: Page, isMobile: boolean) {
  await page.goto("/");

  if (isMobile) {
    await page.getByRole('button').first().click();
  }

  // Navigate to user settings
  await page.getByRole("button", { name: "Playwright" }).click();
  await expect(page.getByRole("menu", { name: "Playwright" })).toBeVisible();
  await page.getByRole("menuitem", { name: "User settings" }).click();

  // Check we are on the user settings page
  {
    const expectedUrl = new RegExp(`${uuidRegex}/account`);
    await expect(page).toHaveURL(expectedUrl);
    await expect(page.getByRole("heading", { name: "User Settings" })).toBeVisible();
  }
}

test("Password change", async ({ page, isMobile }) => {
  await createAccount(page);
  await toUserSettings(page, isMobile);
  const currentEmail = await page.getByRole("textbox", { name: "Email" }).inputValue();

  // Change password
  await page.getByRole("textbox", { name: "Current password" }).fill("playwrighttest");
  await page.getByRole("textbox", { name: "New password", exact: true }).fill("newplaywrighttest");
  await page.getByRole("textbox", { name: "Repeat the new password", exact: true }).fill("newplaywrighttest");
  await page.getByRole("button", { name: "Update Password" }).click();

  // Check success banner shows up
  await expect(page.getByText("Updated", { exact: true })).toBeVisible();

  // log out
  await page.getByRole("button", { name: "Playwright" }).click();
  await expect(page.getByRole("menu", { name: "Playwright" })).toBeVisible();
  await page.getByRole("menuitem", { name: "Log out" }).click();

  // Check we are on the login page
  await expect(page).toHaveURL(/login/);
  await expect(page.getByText("Welcome! Login with:")).toBeVisible();
  // Try logging in with the old password
  await page.getByRole("textbox", { name: "Email" }).fill(currentEmail);
  await page.getByRole("textbox", { name: "Password" }).fill("playwrighttest");
  await page.getByRole("button", { name: "Login" }).click();
  // Check error message is shown
  await expect(page.getByText("Wrong username or password")).toBeVisible();

  // Try logging in with the new password
  await page.getByRole("textbox", { name: "Password" }).fill("newplaywrighttest");
  await page.getByRole("button", { name: "Login" }).click();

  // Check we are put on the projects
  {
    const expectedUrl = new RegExp(`${uuidRegex}/projects`);
    await expect(page).toHaveURL(expectedUrl);
    await expect(page.getByRole("button", { name: "New Project" })).toBeVisible();
  }
});

test("Email and name change", async ({ page, isMobile }) => {
  await createAccount(page);
  await toUserSettings(page, isMobile);
  const currentEmail = await page.getByRole("textbox", { name: "Email" }).inputValue();

  // Change to invalid email
  await page.getByRole("textbox", { name: "Email" }).fill("not-an-email");
  await page.getByRole("button", { name: "Save" }).click();

  // Check the browser's built-in validation message
  const validationMessage = await page.getByRole("textbox", { name: "Email" }).evaluate((element) => {
    return (element as HTMLInputElement).validationMessage;
  });
  expect(validationMessage).toContain("email");

  // Change to new, valid email
  await page.getByRole("textbox", { name: "Email" }).fill(`changed+${currentEmail}`);

  // Attempt to set invalid name (too short)
  await page.getByRole("textbox", { name: "Name" }).fill("a");
  await page.getByRole("button", { name: "Save" }).click();
  await expect(page.getByText("Name must have at least 3 letters")).toBeVisible();

  // Set new, valid name
  await page.getByRole("textbox", { name: "Name" }).fill("Updated Playwright User");
  await page.getByRole("button", { name: "Save" }).click();

  // Check success banner shows up
  await expect(page.getByText("Updated", { exact: true })).toBeVisible();
  // Verify updated name is shown in UI (top right user menu)
  await expect(page.getByRole("button", { name: "Updated Playwright User" })).toBeVisible();

  // Verify changes persisted
  await page.reload();
  await expect(page.getByRole("textbox", { name: "Email" })).toHaveValue(`changed+${currentEmail}`);
  await expect(page.getByRole("textbox", { name: "Name" })).toHaveValue("Updated Playwright User");

  // log out
  await page.getByRole("button", { name: "Playwright" }).click();
  await expect(page.getByRole("menu", { name: "Playwright" })).toBeVisible();
  await page.getByRole("menuitem", { name: "Log out" }).click();

  // Try logging in with the old email
  await page.getByRole("textbox", { name: "Email" }).fill(currentEmail);
  await page.getByRole("textbox", { name: "Password" }).fill("playwrighttest");
  await page.getByRole("button", { name: "Login" }).click();
  // Check error message is shown
  await expect(page.getByText("Wrong username or password")).toBeVisible();

  // Try logging in with the new email
  await page.getByRole("textbox", { name: "Email" }).fill(`changed+${currentEmail}`);
  await page.getByRole("button", { name: "Login" }).click();

  // Check we are put on the projects
  {
    const expectedUrl = new RegExp(`${uuidRegex}/projects`);
    await expect(page).toHaveURL(expectedUrl);
    await expect(page.getByRole("button", { name: "New Project" })).toBeVisible();
  }
});
