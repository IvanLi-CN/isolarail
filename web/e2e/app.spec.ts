import { expect, test } from "@playwright/test";

async function mockCompanionUnavailable(
  page: Parameters<typeof test>[0]["page"],
) {
  await page.route(/\/api\/v1\/bootstrap$/, async (route) => {
    await route.fulfill({
      status: 404,
      contentType: "application/json",
      body: JSON.stringify({ error: "not found" }),
    });
  });
}

test("renders devices list and mock dashboard", async ({ page }) => {
  const storageKey = "isolarail.devices";
  const device = {
    id: "demo",
    name: "Demo Hub",
    baseUrl: "http://192.168.1.23",
  };

  await mockCompanionUnavailable(page);
  await page.addInitScript(
    ({ storageKey, device }) => {
      window.localStorage.clear();
      window.localStorage.setItem(storageKey, JSON.stringify([device]));
    },
    { storageKey, device },
  );

  await page.goto("/");

  await expect(page.getByTestId("device-list")).toBeVisible();
  await expect(page.getByTestId("device-card-demo")).toBeVisible();

  await page.getByTestId("device-card-demo").click();
  await expect(page.getByTestId("device-dashboard-page")).toBeVisible();
  await expect(page.getByTestId("device-dashboard")).toBeVisible();

  await expect(page.getByTestId("port-card-port1")).toBeVisible();
  await expect(page.getByTestId("port-card-port2")).toBeVisible();
  await expect(page.getByTestId("port-card-port3")).toBeVisible();
  await expect(page.getByTestId("port-card-port4")).toBeVisible();
});

test("uses canonical settings/info device routes", async ({ page }) => {
  const storageKey = "isolarail.devices";
  const device = {
    id: "demo",
    name: "Demo Hub",
    baseUrl: "http://192.168.1.23",
  };

  await mockCompanionUnavailable(page);
  await page.addInitScript(
    ({ storageKey, device }) => {
      window.localStorage.clear();
      window.localStorage.setItem(storageKey, JSON.stringify([device]));
    },
    { storageKey, device },
  );

  await page.goto("/devices/demo");
  await expect(page.getByTestId("device-tabs")).toBeVisible();

  await page.getByRole("tab", { name: "Settings" }).click();
  await expect(page).toHaveURL(/\/devices\/demo\/settings$/);
  await expect(page.getByTestId("device-settings-page")).toBeVisible();

  await page.goto("/devices/demo/details");
  await expect(page).toHaveURL(/\/devices\/demo\/info$/);
  await expect(page.getByTestId("device-info-page")).toBeVisible();

  await page.goto("/devices/demo/overview");
  await expect(page).toHaveURL(/\/devices\/demo$/);
  await expect(page.getByTestId("device-dashboard-page")).toBeVisible();
});

test("opens add device modal with supported connection methods (web)", async ({
  page,
}) => {
  const storageKey = "isolarail.devices";
  await mockCompanionUnavailable(page);
  await page.addInitScript(
    ({ storageKey }) => {
      window.localStorage.clear();
      window.localStorage.setItem(storageKey, JSON.stringify([]));
    },
    { storageKey },
  );

  await page.goto("/");

  await page
    .getByTestId("device-list")
    .getByRole("button", { name: "+ Add" })
    .click();

  const dialog = page.getByTestId("add-device-dialog");
  await expect(dialog).toBeVisible();

  await expect(
    dialog.getByText("Auto discovery", { exact: true }),
  ).toBeVisible();
  await expect(
    dialog.getByText("Service discovery: Local companion only", {
      exact: true,
    }),
  ).toBeVisible();
  await expect(
    dialog.getByText("IP scan (advanced)", { exact: true }),
  ).toBeVisible();

  await dialog.getByRole("tab", { name: /Web Serial/ }).click();
  await expect(
    dialog.getByText("Add by Web Serial", { exact: true }),
  ).toBeVisible();
  await expect(
    dialog.getByRole("button", { name: "Connect and add" }),
  ).toBeVisible();

  await dialog.getByRole("tab", { name: /Local USB/ }).click();
  await expect(
    dialog.getByText("Add by Local USB", { exact: true }),
  ).toBeVisible();
  await expect(
    dialog.getByText(
      "Use the explicit isolarail-devd web companion to read the connected hub over Local USB and add it here.",
      { exact: true },
    ),
  ).toBeVisible();
});
