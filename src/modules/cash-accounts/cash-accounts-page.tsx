import { invoke } from "@tauri-apps/api/core";
import { Landmark, LoaderCircle, Plus, Wallet } from "lucide-react";
import { useEffect, useState, type FormEvent } from "react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { PageHeader } from "@/components/shared/page-header";
import { SelectField } from "@/components/shared/select-field";
import { useAuthentication } from "@/modules/auth/auth-gate";

type AccountType = "cash_drawer" | "bank" | "mtn_wallet" | "airtel_wallet" | "other";

interface CashAccount {
  id: string;
  business_id: string;
  account_type: AccountType;
  name: string;
  responsible_user_id: string | null;
  responsible_name: string | null;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

interface CashAccountPaymentOption {
  id: string;
  accountType: AccountType;
  name: string;
  availableBalance: number;
}

interface CashTransfer {
  id: string;
  fromAccountId: string;
  toAccountId: string;
  amount: number;
  status: "pending" | "confirmed" | "cancelled";
  confirmationReference: string | null;
  occurredAt: string;
  createdAt: string;
}

const ACCOUNT_TYPES: { value: AccountType; label: string }[] = [
  { value: "cash_drawer", label: "Cash drawer" },
  { value: "bank", label: "Bank account" },
  { value: "mtn_wallet", label: "MTN Mobile Money" },
  { value: "airtel_wallet", label: "Airtel Money" },
  { value: "other", label: "Other account" },
];

function errorText(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

function accountTypeLabel(type: AccountType): string {
  return ACCOUNT_TYPES.find((accountType) => accountType.value === type)?.label ?? type;
}

function parseMoney(value: string): number | null {
  const match = /^(\d+)(?:\.(\d{1,2}))?$/.exec(value.trim());
  if (!match || value.length > 17) {
    return null;
  }
  const amount = BigInt(match[1]) * 100n + BigInt((match[2] ?? "").padEnd(2, "0"));
  return amount <= BigInt(Number.MAX_SAFE_INTEGER) ? Number(amount) : null;
}

function formatMoney(scaledAmount: number): string {
  if (!Number.isSafeInteger(scaledAmount) || scaledAmount < 0) {
    return "Invalid amount";
  }
  const amount = BigInt(scaledAmount);
  const whole = amount / 100n;
  const fraction = amount % 100n;
  return `${new Intl.NumberFormat("en-RW").format(Number(whole))}${
    fraction === 0n ? "" : `.${fraction.toString().padStart(2, "0")}`
  } RWF`;
}

export function CashAccountsPage() {
  const { user } = useAuthentication();
  const [accounts, setAccounts] = useState<CashAccount[]>([]);
  const [paymentOptions, setPaymentOptions] = useState<CashAccountPaymentOption[]>([]);
  const [transfers, setTransfers] = useState<CashTransfer[]>([]);
  const [accountType, setAccountType] = useState<AccountType>("cash_drawer");
  const [name, setName] = useState("");
  const [openingAmounts, setOpeningAmounts] = useState<Record<string, string>>({});
  const [fromAccountId, setFromAccountId] = useState("");
  const [toAccountId, setToAccountId] = useState("");
  const [transferAmount, setTransferAmount] = useState("");
  const [confirmationReferences, setConfirmationReferences] = useState<Record<string, string>>({});
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [savingAccountId, setSavingAccountId] = useState<string | null>(null);
  const [openingAccountId, setOpeningAccountId] = useState<string | null>(null);
  const [isSavingTransfer, setIsSavingTransfer] = useState(false);
  const [confirmingTransferId, setConfirmingTransferId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const showResponsibleUser = accountType === "cash_drawer";

  useEffect(() => {
    let active = true;
    Promise.all([
      invoke<CashAccount[]>("cash_account_list", { businessId: user.business_id }),
      invoke<CashAccountPaymentOption[]>("cash_account_payment_options", { businessId: user.business_id }),
      invoke<CashTransfer[]>("cash_transfer_list", { businessId: user.business_id }),
    ])
      .then(([result, balances, transferResult]) => {
        if (active) {
          setAccounts(result);
          setPaymentOptions(balances);
          setTransfers(transferResult);
          setError(null);
        }
      })
      .catch((cause: unknown) => {
        if (active) {
          setError(`Could not load cash accounts. ${errorText(cause)}`);
        }
      })
      .finally(() => {
        if (active) {
          setIsLoading(false);
        }
      });
    return () => {
      active = false;
    };
  }, [user.business_id]);

  async function reloadAccounts() {
    const [result, balances, transferResult] = await Promise.all([
      invoke<CashAccount[]>("cash_account_list", { businessId: user.business_id }),
      invoke<CashAccountPaymentOption[]>("cash_account_payment_options", { businessId: user.business_id }),
      invoke<CashTransfer[]>("cash_transfer_list", { businessId: user.business_id }),
    ]);
    setAccounts(result);
    setPaymentOptions(balances);
    setTransfers(transferResult);
  }

  async function handleCreateTransfer(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const amount = parseMoney(transferAmount);
    if (!fromAccountId || !toAccountId || fromAccountId === toAccountId || amount === null || amount <= 0) {
      setError("Select two different opened accounts and enter a valid transfer amount.");
      return;
    }
    setError(null);
    setNotice(null);
    setIsSavingTransfer(true);
    try {
      await invoke<CashTransfer>("cash_transfer_create", {
        businessId: user.business_id,
        fromAccountId,
        toAccountId,
        amount,
      });
      setTransferAmount("");
      await reloadAccounts();
      setNotice("Transfer prepared. It will affect balances only after Owner/Admin confirmation.");
    } catch (cause) {
      setError(`Could not prepare the transfer. ${errorText(cause)}`);
    } finally {
      setIsSavingTransfer(false);
    }
  }

  async function handleConfirmTransfer(transfer: CashTransfer) {
    setError(null);
    setNotice(null);
    setConfirmingTransferId(transfer.id);
    try {
      await invoke("cash_transfer_confirm", {
        businessId: user.business_id,
        transferId: transfer.id,
        confirmationReference: confirmationReferences[transfer.id]?.trim() || null,
      });
      await reloadAccounts();
      setNotice("Transfer confirmed and both account movements were recorded.");
    } catch (cause) {
      setError(`Could not confirm the transfer. ${errorText(cause)}`);
    } finally {
      setConfirmingTransferId(null);
    }
  }

  async function handleOpeningBalance(event: FormEvent<HTMLFormElement>, account: CashAccount) {
    event.preventDefault();
    const openingAmount = parseMoney(openingAmounts[account.id] ?? "");
    if (openingAmount === null) {
      setError("Enter a valid opening balance in RWF.");
      return;
    }
    setError(null);
    setNotice(null);
    setOpeningAccountId(account.id);
    try {
      await invoke("cash_account_opening_create", {
        businessId: user.business_id,
        accountId: account.id,
        openingAmount,
      });
      setOpeningAmounts((current) => ({ ...current, [account.id]: "" }));
      await reloadAccounts();
      setNotice(`Opening balance recorded for ${account.name}. It is permanent and cannot be silently edited.`);
    } catch (cause) {
      setError(`Could not record an opening balance for ${account.name}. ${errorText(cause)}`);
    } finally {
      setOpeningAccountId(null);
    }
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    setNotice(null);
    setIsSaving(true);
    try {
      await invoke<CashAccount>("cash_account_create", {
        businessId: user.business_id,
        accountType,
        name: name.trim(),
      });
      setName("");
      await reloadAccounts();
      setNotice("Cash account created. Existing sales and payments are not posted to it automatically.");
    } catch (cause) {
      setError(`Could not create the cash account. ${errorText(cause)}`);
    } finally {
      setIsSaving(false);
    }
  }

  async function setAccountActive(account: CashAccount, isActive: boolean) {
    setError(null);
    setNotice(null);
    setSavingAccountId(account.id);
    try {
      await invoke("cash_account_set_active", {
        businessId: user.business_id,
        accountId: account.id,
        isActive,
      });
      await reloadAccounts();
      setNotice(`${account.name} ${isActive ? "activated" : "deactivated"}. Its history is preserved.`);
    } catch (cause) {
      setError(`Could not update ${account.name}. ${errorText(cause)}`);
    } finally {
      setSavingAccountId(null);
    }
  }

  return (
    <div className="mx-auto max-w-5xl space-y-6">
      <PageHeader
        eyebrow="Finance setup"
        title="Cash accounts"
        description="Register where business money is held before connecting sales, payments and transfers."
      />

      <div className="rounded-lg border border-amber-500/30 bg-amber-500/5 p-4 text-sm text-foreground">
        Balances start only from an Owner/Admin-authorized opening amount. New linked payments update the account automatically; legacy payments remain unassigned and are not guessed.
      </div>

      {error ? (
        <div role="alert" className="rounded-lg border border-destructive/30 bg-destructive/5 p-4 text-sm text-destructive">
          <p className="font-medium">Cash account action failed</p>
          <p className="mt-1">{error}</p>
        </div>
      ) : null}
      {notice ? (
        <div role="status" className="rounded-lg border border-primary/20 bg-primary/5 p-4 text-sm text-foreground">
          {notice}
        </div>
      ) : null}

      <Card>
        <CardHeader>
          <CardTitle>Add an account</CardTitle>
          <CardDescription>
            Choose the destination type and give it a recognizable name, such as “Front counter drawer” or “MTN Till”.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <form className="grid gap-5 md:grid-cols-2" onSubmit={handleSubmit}>
            <div className="space-y-2">
              <Label htmlFor="cash-account-type">Account type</Label>
              <SelectField
                id="cash-account-type"
                value={accountType}
                onChange={(event) => {
                  const selectedType = ACCOUNT_TYPES.find((type) => type.value === event.target.value);
                  if (selectedType) {
                    setAccountType(selectedType.value);
                  }
                }}
              >
                {ACCOUNT_TYPES.map((type) => (
                  <option key={type.value} value={type.value}>{type.label}</option>
                ))}
              </SelectField>
            </div>
            <div className="space-y-2">
              <Label htmlFor="cash-account-name">Account name</Label>
              <Input
                id="cash-account-name"
                maxLength={120}
                required
                value={name}
                onChange={(event) => setName(event.target.value)}
                placeholder="e.g. Front counter drawer"
              />
            </div>
            {showResponsibleUser ? (
              <p className="text-sm text-muted-foreground md:col-span-2">
                Responsible owner: {user.full_name}. Employee assignment will be available after staff accounts are implemented.
              </p>
            ) : null}
            <div className="flex justify-end md:col-span-2">
              <Button type="submit" disabled={isSaving || !name.trim()}>
                {isSaving ? <LoaderCircle className="animate-spin" /> : <Plus />}
                {isSaving ? "Saving..." : "Add account"}
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Registered accounts</CardTitle>
          <CardDescription>
            Deactivating an account preserves its existing records and removes it from future selection lists.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="flex items-center justify-center gap-2 py-10 text-sm text-muted-foreground">
              <LoaderCircle className="size-4 animate-spin" />
              Loading accounts...
            </div>
          ) : accounts.length === 0 ? (
            <div className="flex flex-col items-center gap-3 py-10 text-center">
              <Landmark className="size-8 text-muted-foreground" />
              <p className="font-medium">No accounts have been registered</p>
              <p className="text-sm text-muted-foreground">Add the business cash drawer, bank and mobile-money accounts to get started.</p>
            </div>
          ) : (
            <div className="divide-y">
              {accounts.map((account) => {
                const paymentOption = paymentOptions.find((option) => option.id === account.id);
                return (
                <div key={account.id} className="flex flex-col gap-3 py-4 sm:flex-row sm:items-center sm:justify-between">
                  <div className="flex min-w-0 items-start gap-3">
                    <Wallet className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
                    <div className="min-w-0 space-y-1">
                      <p className="truncate font-medium">{account.name}</p>
                      <p className="text-sm text-muted-foreground">
                        {accountTypeLabel(account.account_type)}
                        {account.responsible_name ? ` · Responsible: ${account.responsible_name}` : ""}
                      </p>
                      {paymentOption ? (
                        <p className="text-sm font-medium">
                          Available balance: {formatMoney(paymentOption.availableBalance)}
                        </p>
                      ) : account.is_active ? (
                        <form
                          className="flex flex-col gap-2 sm:flex-row sm:items-end"
                          onSubmit={(event) => void handleOpeningBalance(event, account)}
                        >
                          <div className="space-y-1">
                            <Label htmlFor={`opening-balance-${account.id}`} className="text-xs">
                              Authorized opening balance (RWF)
                            </Label>
                            <Input
                              id={`opening-balance-${account.id}`}
                              type="number"
                              min="0"
                              step="0.01"
                              required
                              value={openingAmounts[account.id] ?? ""}
                              onChange={(event) => setOpeningAmounts((current) => ({
                                ...current,
                                [account.id]: event.target.value,
                              }))}
                            />
                          </div>
                          <Button type="submit" size="sm" disabled={openingAccountId !== null}>
                            {openingAccountId === account.id ? <LoaderCircle className="animate-spin" /> : null}
                            Record opening
                          </Button>
                        </form>
                      ) : (
                        <p className="text-sm text-muted-foreground">
                          {account.is_active ? "No opening balance recorded" : "Inactive; unavailable for new payments"}
                        </p>
                      )}
                    </div>
                    <Badge variant={account.is_active ? "default" : "secondary"}>
                      {account.is_active ? "Active" : "Inactive"}
                    </Badge>
                  </div>
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    disabled={savingAccountId !== null}
                    onClick={() => void setAccountActive(account, !account.is_active)}
                  >
                    {savingAccountId === account.id ? <LoaderCircle className="animate-spin" /> : null}
                    {account.is_active ? "Deactivate" : "Activate"}
                  </Button>
                </div>
                );
              })}
            </div>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Transfer between accounts</CardTitle>
          <CardDescription>
            Transfers are custody movements, not expenses. Only confirmed transfers change account balances.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-5">
          {paymentOptions.length < 2 ? (
            <p className="text-sm text-muted-foreground">
              Record authorized opening balances for at least two active accounts before preparing a transfer.
            </p>
          ) : (
            <form className="grid gap-4 md:grid-cols-2" onSubmit={(event) => void handleCreateTransfer(event)}>
              <div className="space-y-2">
                <Label htmlFor="transfer-from">Transfer from</Label>
                <SelectField
                  id="transfer-from"
                  required
                  value={fromAccountId}
                  onChange={(event) => setFromAccountId(event.target.value)}
                >
                  <option value="">Select source account</option>
                  {paymentOptions.map((account) => (
                    <option key={account.id} value={account.id}>
                      {account.name} · Available {formatMoney(account.availableBalance)}
                    </option>
                  ))}
                </SelectField>
              </div>
              <div className="space-y-2">
                <Label htmlFor="transfer-to">Transfer to</Label>
                <SelectField
                  id="transfer-to"
                  required
                  value={toAccountId}
                  onChange={(event) => setToAccountId(event.target.value)}
                >
                  <option value="">Select destination account</option>
                  {paymentOptions.map((account) => (
                    <option key={account.id} value={account.id} disabled={account.id === fromAccountId}>
                      {account.name}
                    </option>
                  ))}
                </SelectField>
              </div>
              <div className="space-y-2">
                <Label htmlFor="transfer-amount">Amount to transfer (RWF)</Label>
                <Input
                  id="transfer-amount"
                  type="number"
                  min="0.01"
                  step="0.01"
                  required
                  value={transferAmount}
                  onChange={(event) => setTransferAmount(event.target.value)}
                />
              </div>
              <div className="flex items-end justify-end">
                <Button type="submit" disabled={isSavingTransfer}>
                  {isSavingTransfer ? <LoaderCircle className="animate-spin" /> : <Landmark />}
                  {isSavingTransfer ? "Preparing..." : "Prepare transfer"}
                </Button>
              </div>
            </form>
          )}
          {transfers.length > 0 ? (
            <div className="divide-y rounded-lg border px-4">
              {transfers.map((transfer) => {
                const source = accounts.find((account) => account.id === transfer.fromAccountId);
                const destination = accounts.find((account) => account.id === transfer.toAccountId);
                return (
                  <div key={transfer.id} className="space-y-3 py-4">
                    <div className="flex flex-wrap items-center justify-between gap-2">
                      <p className="font-medium">
                        {source?.name ?? "Account"} → {destination?.name ?? "Account"} · {formatMoney(transfer.amount)}
                      </p>
                      <Badge variant={transfer.status === "confirmed" ? "default" : "secondary"}>
                        {transfer.status === "confirmed" ? "Confirmed" : transfer.status}
                      </Badge>
                    </div>
                    <p className="text-sm text-muted-foreground">
                      Initiated {new Date(transfer.createdAt).toLocaleString()}
                      {transfer.confirmationReference ? ` · Ref: ${transfer.confirmationReference}` : ""}
                    </p>
                    {transfer.status === "pending" ? (
                      <div className="flex flex-col gap-2 sm:flex-row sm:items-end">
                        <div className="w-full space-y-1 sm:max-w-sm">
                          <Label htmlFor={`transfer-reference-${transfer.id}`} className="text-xs">
                            Confirmation reference (optional)
                          </Label>
                          <Input
                            id={`transfer-reference-${transfer.id}`}
                            maxLength={200}
                            value={confirmationReferences[transfer.id] ?? ""}
                            onChange={(event) => setConfirmationReferences((current) => ({
                              ...current,
                              [transfer.id]: event.target.value,
                            }))}
                          />
                        </div>
                        <Button
                          type="button"
                          size="sm"
                          disabled={confirmingTransferId !== null}
                          onClick={() => void handleConfirmTransfer(transfer)}
                        >
                          {confirmingTransferId === transfer.id ? <LoaderCircle className="animate-spin" /> : null}
                          Confirm transfer
                        </Button>
                      </div>
                    ) : null}
                  </div>
                );
              })}
            </div>
          ) : (
            <p className="text-sm text-muted-foreground">No account transfers have been prepared.</p>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
