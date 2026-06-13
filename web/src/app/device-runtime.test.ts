import { describe, expect, test } from "bun:test";

import { LocalUsbAgentHttpError } from "../domain/hardwareConsole";
import {
  localUsbErrorToDeviceApiError,
  shouldResetLocalUsbConnectionCache,
} from "./device-runtime";

describe("localUsbErrorToDeviceApiError", () => {
  test("preserves structured devd busy errors", () => {
    const error = localUsbErrorToDeviceApiError(
      new LocalUsbAgentHttpError("device busy", 409, "busy", true),
    );

    expect(error).toEqual({
      kind: "busy",
      message: "device busy",
      retryable: true,
    });
  });

  test("preserves structured devd API errors instead of marking offline", () => {
    const error = localUsbErrorToDeviceApiError(
      new LocalUsbAgentHttpError(
        "connected device firmware version `0.0.1` is incompatible",
        400,
        "bad_request",
        false,
      ),
    );

    expect(error).toEqual({
      kind: "api_error",
      status: 400,
      code: "bad_request",
      message: "connected device firmware version `0.0.1` is incompatible",
      retryable: false,
    });
  });
});

describe("shouldResetLocalUsbConnectionCache", () => {
  test("keeps cached agent and device links for structured devd errors", () => {
    expect(
      shouldResetLocalUsbConnectionCache(
        new LocalUsbAgentHttpError("device busy", 409, "busy", true),
      ),
    ).toBe(false);
  });

  test("resets cache for transport-level failures", () => {
    expect(shouldResetLocalUsbConnectionCache(new Error("fetch failed"))).toBe(
      true,
    );
  });
});
