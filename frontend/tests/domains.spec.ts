import { expect, test } from "../playwright/fixtures.ts";
import { createProject, uuidRegex } from "./util.ts";
import { Page } from "@playwright/test";
import { v4 as uuid } from "uuid";

async function toDomains(page: Page) {
  // Navigate to domains page
  await page.locator("a").filter({ hasText: "Domains" }).click();

  // Check we are on the user domains page
  {
    const expectedUrl = new RegExp(`${uuidRegex}/domains`);
    await expect(page).toHaveURL(expectedUrl);
    await expect(page.getByRole("button", { name: "New Domain" })).toBeVisible();
  }
}

async function createDomain(page: Page): Promise<string> {
  const domain = `${uuid()}.com`;

  await page.getByRole("button", { name: "New Domain" }).click();
  await expect(page.getByRole("dialog", { name: "Create New Domain" })).toBeVisible();

  await page.getByRole("textbox", { name: "Domain Name" }).fill(domain);
  await page.getByRole("button", { name: "Next" }).click();
  await expect(page.getByRole("heading", { name: "DKIM Public Key" })).toBeVisible();
  await page.getByRole("button", { name: "Verify", exact: true }).click();
  await expect(page.getByText("DKIM error: could not")).toBeVisible();
  await page.getByRole("button", { name: `Show ${domain}` }).click();

  await toDomains(page);

  // Check new Domain is visible and has the right attributes in the table
  const rows = page.getByRole("table").getByRole("row");
  const targetRow = rows.filter({ hasText: domain });
  // Associated project
  await expect(targetRow.getByText("any project")).toBeVisible();
  // DNS status
  await expect(targetRow.getByText("error")).toBeVisible();

  return domain;
}

test("basic domain lifecycle", async ({ page }) => {
  await page.goto("/");

  await toDomains(page);
  const domain = await createDomain(page);

  // go to settings
  await page.getByRole("table").getByRole("row").filter({ hasText: domain }).getByRole("button").click();

  // delete domain
  await page.getByRole("button", { name: "Delete" }).click();
  await expect(page.getByRole("strong")).toContainText(domain);
  await page.getByRole("button", { name: "Confirm" }).click();
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
  await page.getByRole("table").getByRole("row").filter({ hasText: domain }).getByRole("button").click();

  // Click dropdown
  await page.getByRole("textbox", { name: "Usable by" }).click();
  await expect(page.getByRole("listbox", { name: "Usable by" })).toBeVisible();

  // select project
  await page.getByRole("option", { name: project }).click();
  await page.getByRole("button", { name: "Save" }).click();

  await expect(page.getByText("Domain updated")).toBeVisible();

  // go back using the breadcrumbs
  await page.getByRole("button", { name: "domains" }).click();

  // check table row
  await expect(
    page.getByRole("table").getByRole("row").filter({ hasText: domain }).getByRole("cell", { name: project })
  ).toBeVisible();
});

test("create domain with project", async ({ page }) => {
  await page.goto("/");

  const project = await createProject(page);
  await toDomains(page);

  const domain = `${uuid()}.com`;

  await page.getByRole("button", { name: "New Domain" }).click();
  await expect(page.getByRole("dialog", { name: "Create New Domain" })).toBeVisible();

  await page.getByRole("textbox", { name: "Domain Name" }).fill(domain);

  // select project
  await page.getByRole("textbox", { name: "Usable by" }).click();
  await expect(page.getByRole("listbox", { name: "Usable by" })).toBeVisible();
  await page.getByRole("option", { name: project }).click();

  await page.getByRole("button", { name: "Next" }).click();
  await page.getByRole("button", { name: "Configure later" }).click();

  // check table row
  await expect(
    page.getByRole("table").getByRole("row").filter({ hasText: domain }).getByRole("cell", { name: project })
  ).toBeVisible();
});

test("domain must have TLD", async ({ page }) => {
  await page.goto("/");

  await toDomains(page);

  const domain = `${uuid()}`;

  await page.getByRole("button", { name: "New Domain" }).click();
  await expect(page.getByRole("dialog", { name: "Create New Domain" })).toBeVisible();

  await page.getByRole("textbox", { name: "Domain Name" }).fill(domain);
  await page.getByRole("button", { name: "Next" }).click();
  await expect(page.getByText("Domain must include a top")).toBeVisible();
});

test("cancel button deletes temporary domain", async ({ page }) => {
  await page.goto("/");

  await toDomains(page);

  const domain = `${uuid()}.com`;

  // create new domain
  await page.getByRole("button", { name: "New Domain" }).click();
  await expect(page.getByRole("dialog", { name: "Create New Domain" })).toBeVisible();
  await page.getByRole("textbox", { name: "Domain Name" }).fill(domain);
  await page.getByRole("button", { name: "Next" }).click();

  // check domain gets added to table
  await expect(page.getByRole("table").getByRole("row").filter({ hasText: domain })).not.toBeVisible();

  // cancel wizard
  await page.getByRole("button", { name: "Cancel" }).click();

  // make sure it's not shown in the table
  await expect(page.getByRole("table").getByRole("row").filter({ hasText: domain })).not.toBeVisible();
});
