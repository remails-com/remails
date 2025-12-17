// As we directly import from playwright/test, we are not logged in automatically.
import { test, expect } from "@playwright/test";

test("change global role", async ({ page }) => {
  await page.goto("http://localhost:3000/login");

  // Use login as super admin
  await page.getByRole("textbox", { name: "Email" }).fill("sudo@remails");
  await page.getByRole("textbox", { name: "Password" }).fill("unsecure123");
  await page.getByRole("button", { name: "Login" }).click();

  await page.locator("a").filter({ hasText: "Admin" }).click();
  await expect(page.getByRole("tabpanel", { name: "Config" })).toBeVisible();

  await page.getByRole("tab", { name: "Users" }).click();

  // make sure all operations take place in the correct row
  const row = page.getByRole("row", { name: "Test API User 5" });

  // click role dropdown
  await row.getByRole("cell").nth(3).getByRole("textbox").click();

  // make user admin
  await page.getByRole("option", { name: "admin" }).click();

  // check the user is actually admin now
  await expect(row.getByRole("cell").nth(3).getByRole("textbox")).toHaveValue("admin");

  // make user non-admin again
  await row.getByRole("cell").nth(3).locator("svg").first().click();

  // check again
  await expect(row.getByRole("cell").nth(3).getByRole("textbox")).toBeEmpty();
});
