import type { PortId, PortTelemetry } from "./ports";

export type MockDeviceNetworkInfo = {
  ip: string;
  hostname: string;
  mac: string;
  mcu_unique_id: string;
};

function hashString(input: string): number {
  let hash = 5381;
  for (let i = 0; i < input.length; i += 1) {
    hash = (hash << 5) + hash + input.charCodeAt(i);
  }
  return hash >>> 0;
}

function hexByte(value: number): string {
  return (value & 0xff).toString(16).padStart(2, "0");
}

export function mockPortTelemetry(
  deviceId: string,
  portId: PortId,
  powerEnabled: boolean,
): PortTelemetry {
  const sample_uptime_ms =
    typeof performance !== "undefined"
      ? Math.floor(performance.now())
      : Date.now();

  if (!powerEnabled) {
    return {
      status: "ok",
      voltage_mv: 0,
      current_ma: 0,
      power_mw: 0,
      sample_uptime_ms,
    };
  }

  const seed = hashString(`${deviceId}:${portId}`);
  const voltage_mv = 5000 + (seed % 301) - 150; // 4.85V ~ 5.15V
  const current_ma = 250 + ((seed >> 8) % 1500); // 250mA ~ 1749mA
  const power_mw = Math.max(0, Math.round((voltage_mv * current_ma) / 1000));

  return {
    status: "ok",
    voltage_mv,
    current_ma,
    power_mw,
    sample_uptime_ms,
  };
}

export function mockDeviceNetworkInfo(deviceId: string): MockDeviceNetworkInfo {
  const seed = hashString(deviceId);
  const ip = `192.168.${(seed >> 8) & 0xff}.${((seed >> 16) & 0xfe) + 1}`;
  const hostname = `isolarail-${deviceId.slice(0, 6) || "device"}.local`;
  const mac = [
    0x02,
    seed & 0xff,
    (seed >> 8) & 0xff,
    (seed >> 16) & 0xff,
    (seed >> 24) & 0xff,
    (seed ^ 0xa5) & 0xff,
  ]
    .map(hexByte)
    .join(":");
  const mcu_unique_id = `ESP32-S3-${seed.toString(16).padStart(8, "0")}`;

  return { ip, hostname, mac, mcu_unique_id };
}
