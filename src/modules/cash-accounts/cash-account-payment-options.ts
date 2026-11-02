import { invoke } from "@tauri-apps/api/core";

export type PaymentMethod = "cash" | "mtn_mobile_money" | "airtel_money" | "bank_transfer";

export interface CashAccountPaymentOption {
  id: string;
  accountType: "cash_drawer" | "bank" | "mtn_wallet" | "airtel_wallet" | "other";
  name: string;
  availableBalance: number;
}

export async function loadCashAccountPaymentOptions(
  businessId: string,
): Promise<CashAccountPaymentOption[]> {
  return invoke<CashAccountPaymentOption[]>("cash_account_payment_options", { businessId });
}

export function accountTypeForPaymentMethod(method: PaymentMethod): CashAccountPaymentOption["accountType"] {
  switch (method) {
    case "cash":
      return "cash_drawer";
    case "bank_transfer":
      return "bank";
    case "mtn_mobile_money":
      return "mtn_wallet";
    case "airtel_money":
      return "airtel_wallet";
  }
}

export function accountsForPaymentMethod(
  accounts: CashAccountPaymentOption[],
  method: PaymentMethod,
): CashAccountPaymentOption[] {
  const accountType = accountTypeForPaymentMethod(method);
  return accounts.filter((account) => account.accountType === accountType);
}
