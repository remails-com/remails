// based on https://playwright.dev/docs/auth#moderate-one-account-per-parallel-worker

import { test as baseTest, expect } from '@playwright/test';
import fs from 'fs';
import path from 'path';
import { v4 as uuid } from "uuid";

export * from '@playwright/test';
export const test = baseTest.extend<object, { workerStorageState: string }>({
  // Use the same storage state for all tests in this worker.
  // eslint-disable-next-line react-hooks/rules-of-hooks
  storageState: ({ workerStorageState }, use) => use(workerStorageState),

  // Authenticate once per worker with a worker-scoped fixture.
  workerStorageState: [async ({ browser }, use) => {
    // Use parallelIndex as a unique identifier for each worker.
    const id = test.info().parallelIndex;
    const fileName = path.resolve(test.info().project.outputDir, `.auth/${id}.json`);

    if (fs.existsSync(fileName)) {
      // Reuse existing authentication state if any.
      await use(fileName);
      return;
    }

    // Important: make sure we authenticate in a clean environment by unsetting storage state.
    const page = await browser.newPage({ storageState: undefined });

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
      await page.getByRole("main").getByRole("button").filter({ hasText: /^$/ }).click();
      await expect(page.getByText("Organization", { exact: true })).toBeVisible();
    }).toPass();
    // End of authentication steps.

    await page.context().storageState({ path: fileName });
    await page.close();
    await use(fileName);
  }, { scope: 'worker' }],
});