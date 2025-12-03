import { expect, test } from "../playwright/fixtures.ts";
import { createProject, uuidRegex } from "./util.ts";

test("Project lifecycle", async ({ page }) => {
  await page.goto("/");

  const projectUuid = await createProject(page);

  // Check success banner shows up
  await expect(page.getByText("Project created")).toBeVisible();

  // Check we are put on the credentials page
  {
    const expectedUrl = new RegExp(`${uuidRegex}/projects/${uuidRegex}/credentials`);
    await expect(page).toHaveURL(expectedUrl);
    await expect(page.getByRole("button", { name: "New Credential" })).toBeVisible();
  }

  // Back to projects list
  await page.locator("a").filter({ hasText: "Projects" }).click();

  // Check the new project is listed
  await expect(page.getByRole("cell", { name: projectUuid })).toBeVisible();

  // click edit button
  await page
    .getByRole("row", { name: projectUuid })
    .getByRole("button")
    .locator(".tabler-icon.tabler-icon-edit")
    .click();

  // Check we are on the edit project page
  {
    const expectedUrl = new RegExp(`${uuidRegex}/projects/${uuidRegex}/settings`);
    await expect(page).toHaveURL(expectedUrl);
    await expect(page.getByRole("heading", { name: "Project Settings" })).toBeVisible();
  }

  await expect(page.getByRole("textbox", { name: "Name" })).toHaveValue(projectUuid);

  // rename project
  await page.getByRole("textbox", { name: "Name" }).fill("renamed project");
  await page.getByRole("button", { name: "Save" }).click();

  // Check success banner shows up
  await expect(page.getByText("Project updated")).toBeVisible();

  // check new name is visible
  await expect(page.getByRole("heading", { name: "renamed project" })).toBeVisible();

  // Use breadcrumb to go back to projects list
  await page.getByRole("button", { name: "projects" }).click();

  // Check the renamed project is listed
  await expect(page.getByRole("cell", { name: "renamed project" })).toBeVisible();

  // Back to edit project page
  await page
    .getByRole("row", { name: "renamed project" })
    .getByRole("button")
    .locator(".tabler-icon.tabler-icon-edit")
    .click();

  // Delete the project
  await page.getByRole("button", { name: "Delete" }).click();
  // Confirm deletion dialog is visible and shows the correct project name
  await expect(page.getByLabel("Please confirm your action").getByText("renamed project")).toBeVisible();

  // Click "confirm deletion"
  await page.getByRole("button", { name: "Confirm" }).click();

  // Confirm success message is visible
  await expect(page.getByText("Project deleted", { exact: true })).toBeVisible();

  // Check we are on the project list page
  {
    const expectedUrl = new RegExp(`${uuidRegex}/projects`);
    await expect(page).toHaveURL(expectedUrl);
  }

  // Check the project is no longer listed
  await expect(page.getByRole("cell", { name: "renamed project" })).not.toBeVisible();
});

test("Credentials lifecycle", async ({ page }) => {
  await page.goto("/");
  await createProject(page);

  // Create new SMTP credential
  await page.getByRole("button", { name: "New Credential" }).click();
  await expect(page.getByRole("dialog", { name: "Create new SMTP credential" })).toBeVisible();

  // Fill in details
  await page.getByRole("textbox", { name: "Username" }).fill("playwright-smtp-user");
  await page.getByRole("textbox", { name: "Description" }).fill("This is created by Playwright");
  await page.getByRole("button", { name: "Create", exact: true }).click();

  // Check that credential name has the expected format
  await expect(page.getByLabel("Create new SMTP credential")).toContainText(/[0-9a-f]{8}-playwright-smtp-user/);
  await page.getByRole("button", { name: "Done" }).click();

  // Check that the new credential is listed
  await expect(page.getByLabel("Credentials")).toContainText(/[0-9a-f]{8}-playwright-smtp-user/);

  // Check the description is correct
  await expect(page.getByLabel("Credentials")).toContainText("This is created by Playwright");

  // Go to credential edit page
  await page.getByRole("table").getByRole("button").locator(".tabler-icon.tabler-icon-edit").click();

  // Check we are on the credentials edit page
  {
    const expectedUrl = new RegExp(`${uuidRegex}/projects/${uuidRegex}/credentials/${uuidRegex}`);
    await expect(page).toHaveURL(expectedUrl);
  }

  // Edit description
  await page.getByRole("textbox", { name: "Description" }).fill("This is made by Playwright");
  await page.getByRole("button", { name: "Save" }).click();

  // Ensure success message is visible
  await expect(page.getByText("SMTP credential updated")).toBeVisible();

  // Use breadcrumb to go back to the credentials list
  await page.getByRole("button", { name: "credentials" }).click();

  // Ensure updated description is visible
  await expect(page.getByLabel("Credentials")).toContainText("This is made by Playwright");

  // Back to credential edit page
  await page.getByRole("table").getByRole("button").locator(".tabler-icon.tabler-icon-edit").click();

  // Delete the credential
  await page.getByRole("button", { name: "Delete" }).click();
  await expect(page.getByRole("strong")).toContainText(/[0-9a-f]{8}-playwright-smtp-user/);
  await page.getByRole("button", { name: "Confirm" }).click();

  // Ensure success message is visible
  await expect(page.getByText("Credential deleted")).toBeVisible();

  // Check we are on the credentials list page
  {
    const expectedUrl = new RegExp(`${uuidRegex}/projects/${uuidRegex}/credentials`);
    await expect(page).toHaveURL(expectedUrl);
  }

  // Ensure the credential is no longer listed
  await expect(page.getByLabel("Credentials")).not.toContainText(/[0-9a-f]{8}-playwright-smtp-user/);
});
