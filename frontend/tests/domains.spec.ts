import { expect, test } from "./fixtures.ts";
import { createProject, deleteProject, uuidRegex } from "./util.ts";
import { Page } from "@playwright/test";
import { v4 as uuid } from "uuid";

async function toDomains(page: Page) {
  // Navigate to domains page
  await page.getByRole("link", { name: "Domains", exact: true }).click();

  // Check we are on the user domains page
  {
    const expectedUrl = new RegExp(`${uuidRegex}/domains`);
    await expect(page).toHaveURL(expectedUrl);
    await expect(page.getByRole("button", { name: "New domain" })).toBeVisible();
  }
}

async function createDomain(page: Page): Promise<string> {
  const domain = `${uuid()}.com`;

  await page.getByRole("button", { name: "New domain" }).click();
  await expect(page.getByRole("dialog", { name: "Add new domain" })).toBeVisible();

  await page.getByRole("textbox", { name: "Domain name" }).fill(domain);
  await page.getByRole("button", { name: "Next" }).click();
  await expect(page.getByRole("heading", { name: "DKIM Public Key" })).toBeVisible();
  await page.getByRole("button", { name: "Verify", exact: true }).click();
  await expect(page.getByText("DKIM error: could not")).toBeVisible();
  await page.getByRole("button", { name: `Show ${domain}` }).click();

  await toDomains(page);

  // Check new Domain is visible and has the right attributes in the table
  const rows = page.getByRole("table").getByRole("row");
  const targetRow = rows.filter({ hasText: domain });
  // No associated projects because we didn't add any
  await expect(targetRow.getByText("no projects")).toBeVisible();
  // DNS status
  await expect(targetRow.getByText("error")).toBeVisible();

  return domain;
}

test("basic domain lifecycle", async ({ page }) => {
  await page.goto("/");

  await toDomains(page);
  const domain = await createDomain(page);

  // go to settings
  await page
    .getByRole("table")
    .getByRole("row")
    .filter({ hasText: domain })
    .getByRole("link")
    .locator(".tabler-icon.tabler-icon-edit")
    .click();

  // delete domain
  await page.getByRole("button", { name: "Delete" }).click();
  const modal = page.getByLabel('Please confirm your action');
  await expect(modal.getByRole("strong")).toContainText(domain);
  await modal.getByRole('button', { name: 'Delete' }).click();
  await expect(page.getByText("Domain deleted")).toBeVisible();

  // Check we are back on the domains page
  {
    const expectedUrl = new RegExp(`${uuidRegex}/domains`);
    await expect(page).toHaveURL(expectedUrl);
  }

  // check the domain is actually gone
  await expect(page.getByRole("cell", { name: domain })).not.toBeVisible();
});

test("attach project afterward", async ({ page }) => {
  await page.goto("/");

  const project = await createProject(page);

  await toDomains(page);
  const domain = await createDomain(page);

  // go to settings
  await page
    .getByRole("table")
    .getByRole("row")
    .filter({ hasText: domain })
    .getByRole("link")
    .locator(".tabler-icon.tabler-icon-edit")
    .click();

  // Click dropdown
  await page.getByRole("textbox", { name: "Usable by" }).click();
  await expect(page.getByRole("listbox", { name: "Usable by" })).toBeVisible();

  // select project
  await page.getByRole("option", { name: project }).click();
  await page.getByRole("textbox", { name: "Usable by" }).blur();
  await page.getByRole("button", { name: "Save" }).click();

  await expect(page.getByText("Domain updated")).toBeVisible();

  // go back using the breadcrumbs
  await page.getByRole("link", { name: "domains", exact: true }).click();

  // check table row
  await expect(
    page.getByRole("table").getByRole("row").filter({ hasText: domain }).getByRole("cell", { name: project })
  ).toBeVisible();

  await deleteProject(page);
});

test("create domain with project", async ({ page }) => {
  await page.goto("/");

  const project = await createProject(page);
  await toDomains(page);

  const domain = `${uuid()}.com`;

  await page.getByRole("button", { name: "New domain" }).click();
  await expect(page.getByRole("dialog", { name: "Add new domain" })).toBeVisible();

  await page.getByRole("textbox", { name: "Domain name" }).fill(domain);

  // select project
  await page.getByRole("textbox", { name: "Usable by" }).click();
  await expect(page.getByRole("listbox", { name: "Usable by" })).toBeVisible();
  await page.getByRole("option", { name: project }).click();

  await page.getByRole("textbox", { name: "Usable by" }).blur();
  await page.getByRole("button", { name: "Next" }).click();
  await page.getByRole("button", { name: "Configure later" }).click();

  // check table row
  await expect(
    page.getByRole("table").getByRole("row").filter({ hasText: domain }).getByRole("cell", { name: project })
  ).toBeVisible();

  await deleteProject(page);
});

test("domain must have TLD", async ({ page }) => {
  await page.goto("/");

  await toDomains(page);

  const domain = `${uuid()}`;

  await page.getByRole("button", { name: "New domain" }).click();
  await expect(page.getByRole("dialog", { name: "Add new domain" })).toBeVisible();

  await page.getByRole("textbox", { name: "Domain name" }).fill(domain);
  await page.getByRole("button", { name: "Next" }).click();
  await expect(page.getByText("Domain must include a top")).toBeVisible();
});

test("cancel button deletes temporary domain", async ({ page }) => {
  await page.goto("/");

  await toDomains(page);

  const domain = `${uuid()}.com`;

  // create new domain
  await page.getByRole("button", { name: "New domain" }).click();
  await expect(page.getByRole("dialog", { name: "Add new domain" })).toBeVisible();
  await page.getByRole("textbox", { name: "Domain name" }).fill(domain);
  await page.getByRole("button", { name: "Next" }).click();

  // check domain gets added to table
  await expect(page.getByRole("table").getByRole("row").filter({ hasText: domain })).toBeVisible();

  // cancel wizard
  await page.getByRole("button", { name: "Cancel" }).click();

  // make sure it's not shown in the table
  await expect(page.getByRole("table").getByRole("row").filter({ hasText: domain })).not.toBeVisible();
});
