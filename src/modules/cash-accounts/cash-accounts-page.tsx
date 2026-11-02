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

export function CashAccountsPage() {
  const { user } = useAuthentication();
  const [accounts, setAccounts] = useState<CashAccount[]>([]);
  const [accountType, setAccountType] = useState<AccountType>("cash_drawer");
  const [name, setName] = useState("");
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [savingAccountId, setSavingAccountId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const showResponsibleUser = accountType === "cash_drawer";

  useEffect(() => {
    let active = true;
    invoke<CashAccount[]>("cash_account_list", { businessId: user.business_id })
      .then((result) => {
        if (active) {
          setAccounts(result);
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
    const result = await invoke<CashAccount[]>("cash_account_list", {
      businessId: user.business_id,
    });
    setAccounts(result);
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
        These are account records only. Balances, transaction posting, transfers and daily cash closing are not enabled yet.
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
              {accounts.map((account) => (
                <div key={account.id} className="flex flex-col gap-3 py-4 sm:flex-row sm:items-center sm:justify-between">
                  <div className="flex min-w-0 items-start gap-3">
                    <Wallet className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
                    <div className="min-w-0 space-y-1">
                      <p className="truncate font-medium">{account.name}</p>
                      <p className="text-sm text-muted-foreground">
                        {accountTypeLabel(account.account_type)}
                        {account.responsible_name ? ` · Responsible: ${account.responsible_name}` : ""}
                      </p>
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
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
