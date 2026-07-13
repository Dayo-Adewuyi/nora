import { invoke, isTauri } from "@tauri-apps/api/core";

export interface RuntimeContract {
  productName: string;
  offline: boolean;
  supportedCadres: readonly ["JCHEW", "CHEW"];
}

const webContract: RuntimeContract = Object.freeze({
  productName: "CHEW Companion",
  offline: true,
  supportedCadres: ["JCHEW", "CHEW"] as const,
});

export async function loadRuntimeContract(): Promise<RuntimeContract> {
  if (!isTauri()) {
    return webContract;
  }

  return invoke<RuntimeContract>("runtime_contract");
}
