import { createAccount, uuidRegex } from "./util.ts";
// As we directly import from playwright/test, we are not logged in automatically.
import { Page, test, expect } from "@playwright/test";

async function toUserSettings(page: Page) {
  await page.goto("/");

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

test("Password change", async ({ page }) => {
  await createAccount(page);
  await toUserSettings(page);
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

test("Email and name change", async ({ page }) => {
  await createAccount(page);
  await toUserSettings(page);
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

test("Password reset link", async ({ browser }) => {
  const context1 = await browser.newContext({ storageState: undefined });
  const context2 = await browser.newContext({ storageState: undefined });
  const page1 = await context1.newPage();
  const page2 = await context2.newPage();

  await page1.goto("http://localhost:3000/login");
  await page2.goto("http://localhost:3000/login");

  // Initiate password reset email
  await page1.getByRole("button", { name: "Forgot your password?" }).click();

  await page1.getByRole("textbox", { name: "Email" }).fill("test-api@user-2");
  await page1.getByRole("button", { name: "Reset password" }).click();
  await expect(page1.getByText("Please check your inbox")).toBeVisible();

  // Use admin account to find the password reset email
  await page2.getByRole("textbox", { name: "Email" }).fill("admin@example.com");
  await page2.getByRole("textbox", { name: "Password" }).fill("unsecure123");
  await page2.getByRole("button", { name: "Login" }).click();
  await expect(page2.getByRole("row", { name: "Project 1 Organization 1" })).toBeVisible();
  await page2.getByText("Project 1 Organization").click();
  await page2.getByRole("button", { name: "Email from noreply@remails.com" }).first().click();
  await page2.getByRole("button", { name: "View email" }).click();

  // extract link from email
  const email = await page2.getByText("Dear Test API User 2").textContent();
  expect(email).toBeTruthy();
  const regex = new RegExp(/https:\/\/[^/]*\/([^\s)]*)/);
  const reset_link = email!.match(regex)![1];
  expect(reset_link).toBeTruthy();

  // Use password reset link
  await page1.goto(`http://localhost:3000/${reset_link}`);
  await expect(page1.getByRole("img", { name: "Remails logo" })).toBeVisible();

  await page1.getByRole("textbox", { name: "Password" }).fill("thisismynewpassword");
  await page1.getByRole("button", { name: "Reset password" }).click();
  await expect(page1.getByRole("link", { name: "Github" })).toBeVisible();

  await page1.getByRole("textbox", { name: "Password" }).fill("thisismynewpassword");
  await page1.getByRole("textbox", { name: "Email" }).fill("test-api@user-2");
  await page1.getByRole("button", { name: "Login" }).click();
  await expect(page1.getByRole("button", { name: "Test API User 2" })).toBeVisible();
});
