import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState, type FormEvent } from "react";
import { LoaderCircle, ReceiptText, Wallet } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { PageHeader } from "@/components/shared/page-header";
import { SelectField } from "@/components/shared/select-field";

interface Business {
  id: string;
  name: string;
}

type PaymentMethod = "cash" | "mtn_mobile_money" | "airtel_money" | "bank_transfer";

const PAYMENT_METHODS: { value: PaymentMethod; label: string }[] = [
  { value: "cash", label: "Cash" },
  { value: "mtn_mobile_money", label: "MTN Mobile Money" },
  { value: "airtel_money", label: "Airtel Money" },
  { value: "bank_transfer", label: "Bank transfer" },
];

interface CustomerDebt {
  id: string;
  customer_id: string;
  customer_name: string;
  sale_id: string;
  transaction_number: string;
  original_amount: number;
  amount_paid: number;
  balance_due: number;
  created_at: string;
}

interface DebtPayment {
  id: string;
  debt_id: string;
  customer_name: string;
  payment_method: PaymentMethod;
  amount: number;
  balance_due: number;
  created_at: string;
}

function errorText(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
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

export function PaymentsPage() {
  const [businesses, setBusinesses] = useState<Business[]>([]);
  const [businessId, setBusinessId] = useState("");
  const [debts, setDebts] = useState<CustomerDebt[]>([]);
  const [selectedDebtId, setSelectedDebtId] = useState("");
  const [amount, setAmount] = useState("");
  const [paymentMethod, setPaymentMethod] = useState<PaymentMethod>("cash");
  const [isLoadingBusinesses, setIsLoadingBusinesses] = useState(true);
  const [isLoadingDebts, setIsLoadingDebts] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const selectedDebt = debts.find((debt) => debt.id === selectedDebtId);
  const outstandingTotal = debts.reduce((sum, debt) => sum + debt.balance_due, 0);

  useEffect(() => {
    let active = true;
    invoke<Business[]>("business_list")
      .then((result) => {
        if (active) {
          setBusinesses(result);
          setBusinessId((current) =>
            result.some((business) => business.id === current) ? current : (result[0]?.id ?? ""),
          );
        }
      })
      .catch((cause: unknown) => {
        if (active) {
          setError(`Could not load businesses. ${errorText(cause)}`);
        }
      })
      .finally(() => {
        if (active) {
          setIsLoadingBusinesses(false);
        }
      });
    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    if (!businessId) {
      setDebts([]);
      setSelectedDebtId("");
      return;
    }

    let active = true;
    setIsLoadingDebts(true);
    setError(null);
    invoke<CustomerDebt[]>("customer_debt_list", { businessId })
      .then((result) => {
        if (active) {
          setDebts(result);
          setSelectedDebtId((current) =>
            result.some((debt) => debt.id === current && debt.balance_due > 0)
              ? current
              : (result.find((debt) => debt.balance_due > 0)?.id ?? ""),
          );
        }
      })
      .catch((cause: unknown) => {
        if (active) {
          setError(`Could not load customer debts. ${errorText(cause)}`);
        }
      })
      .finally(() => {
        if (active) {
          setIsLoadingDebts(false);
        }
      });

    return () => {
      active = false;
    };
  }, [businessId]);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const parsedAmount = parseMoney(amount);
    if (!businessId || !selectedDebtId || parsedAmount === null || parsedAmount <= 0) {
      setError("Select an outstanding customer debt and enter a valid repayment amount.");
      return;
    }
    if (!selectedDebt || parsedAmount > selectedDebt.balance_due) {
      setError("Repayment cannot exceed the selected outstanding balance.");
      return;
    }

    setError(null);
    setNotice(null);
    setIsSaving(true);
    try {
      const payment = await invoke<DebtPayment>("customer_debt_record_payment", {
        businessId,
        debtId: selectedDebtId,
        paymentMethod,
        amount: parsedAmount,
      });
      setDebts((current) =>
        current.map((debt) =>
          debt.id === payment.debt_id
            ? {
                ...debt,
                amount_paid: debt.amount_paid + payment.amount,
                balance_due: payment.balance_due,
              }
            : debt,
        ),
      );
      setAmount("");
      setNotice(
        `Recorded ${formatMoney(payment.amount)} from ${payment.customer_name}. Remaining balance: ${formatMoney(payment.balance_due)}.`,
      );
      if (payment.balance_due === 0) {
        const nextOpenDebt = debts.find((debt) => debt.id !== payment.debt_id && debt.balance_due > 0);
        setSelectedDebtId(nextOpenDebt?.id ?? "");
      }
    } catch (cause) {
      setError(`Could not record the repayment. ${errorText(cause)}`);
    } finally {
      setIsSaving(false);
    }
  }

  const pageHeader = (
    <PageHeader
      eyebrow="Receivables"
      title="Customer payments"
      description="Track customer credit balances and record repayments against the original sale."
      actions={businesses.length > 0 ? (
        <div className="w-full space-y-1.5 sm:w-56">
          <Label htmlFor="payments-business">Business</Label>
          <SelectField
            id="payments-business"
            value={businessId}
            onChange={(event) => {
              setBusinessId(event.target.value);
              setAmount("");
              setNotice(null);
            }}
          >
            {businesses.map((business) => (
              <option key={business.id} value={business.id}>{business.name}</option>
            ))}
          </SelectField>
        </div>
      ) : undefined}
    />
  );

  if (isLoadingBusinesses) {
    return (
      <div className="mx-auto max-w-7xl space-y-6">
        {pageHeader}
        <Card>
          <CardContent className="flex items-center gap-2 py-6 text-sm text-muted-foreground">
            <LoaderCircle className="size-4 animate-spin" />
            Loading businesses...
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-7xl space-y-6">
      {pageHeader}
      {error ? (
        <div role="alert" className="rounded-lg border border-destructive/30 bg-destructive/5 p-4 text-sm text-destructive">
          {error}
        </div>
      ) : null}
      {notice ? (
        <div role="status" className="rounded-lg border border-emerald-700/20 bg-emerald-700/5 p-4 text-sm text-emerald-800">
          {notice}
        </div>
      ) : null}

      {businesses.length === 0 ? (
        <Card>
          <CardContent className="py-6 text-sm text-muted-foreground">
            Register a business before managing customer payments.
          </CardContent>
        </Card>
      ) : (
        <>
          <div className="grid gap-6 xl:grid-cols-[minmax(20rem,0.8fr)_minmax(0,1.2fr)]">
            <Card>
              <CardHeader>
                <CardTitle>Record repayment</CardTitle>
                <CardDescription>
                  A repayment reduces the selected debt; it never creates another sale.
                </CardDescription>
              </CardHeader>
              <CardContent>
                <form className="space-y-4" onSubmit={handleSubmit}>
                  <div className="space-y-2">
                    <Label htmlFor="debt-account">Outstanding customer sale</Label>
                    <SelectField
                      id="debt-account"
                      required
                      value={selectedDebtId}
                      onChange={(event) => setSelectedDebtId(event.target.value)}
                    >
                      <option value="">Select a debt</option>
                      {debts.filter((debt) => debt.balance_due > 0).map((debt) => (
                        <option key={debt.id} value={debt.id}>
                          {debt.customer_name} · {debt.transaction_number} · {formatMoney(debt.balance_due)}
                        </option>
                      ))}
                    </SelectField>
                  </div>
                  {selectedDebt ? (
                    <div className="rounded-lg border bg-muted/30 p-3 text-sm">
                      <div className="flex justify-between gap-3">
                        <span className="text-muted-foreground">Original sale</span>
                        <span>{formatMoney(selectedDebt.original_amount)}</span>
                      </div>
                      <div className="mt-1 flex justify-between gap-3 font-medium">
                        <span>Balance due</span>
                        <span>{formatMoney(selectedDebt.balance_due)}</span>
                      </div>
                    </div>
                  ) : null}
                  <div className="space-y-2">
                    <Label htmlFor="repayment-amount">Amount received (RWF)</Label>
                    <Input
                      id="repayment-amount"
                      type="number"
                      min="0.01"
                      max={selectedDebt ? (selectedDebt.balance_due / 100).toFixed(2) : undefined}
                      step="0.01"
                      required
                      value={amount}
                      onChange={(event) => setAmount(event.target.value)}
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="repayment-method">Payment method</Label>
                    <SelectField
                      id="repayment-method"
                      value={paymentMethod}
                      onChange={(event) => {
                        const selected = PAYMENT_METHODS.find((method) => method.value === event.target.value);
                        if (selected) {
                          setPaymentMethod(selected.value);
                        }
                      }}
                    >
                      {PAYMENT_METHODS.map((method) => (
                        <option key={method.value} value={method.value}>{method.label}</option>
                      ))}
                    </SelectField>
                  </div>
                  <Button
                    type="submit"
                    disabled={isSaving || isLoadingDebts || !selectedDebt || selectedDebt.balance_due <= 0}
                  >
                    {isSaving ? <LoaderCircle className="animate-spin" /> : <Wallet />}
                    {isSaving ? "Recording..." : "Record repayment"}
                  </Button>
                </form>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle>Customer debt ledger</CardTitle>
                <CardDescription>
                  {debts.filter((debt) => debt.balance_due > 0).length} open accounts · {formatMoney(outstandingTotal)} outstanding
                </CardDescription>
              </CardHeader>
              <CardContent>
                {isLoadingDebts ? (
                  <div className="flex items-center gap-2 py-4 text-sm text-muted-foreground">
                    <LoaderCircle className="size-4 animate-spin" />
                    Loading debt ledger...
                  </div>
                ) : debts.length === 0 ? (
                  <div className="flex items-center gap-3 py-4 text-sm text-muted-foreground">
                    <ReceiptText className="size-4 shrink-0" />
                    No credit sales have been recorded for this business.
                  </div>
                ) : (
                  <ul className="divide-y">
                    {debts.map((debt) => (
                      <li key={debt.id} className="flex flex-col gap-3 py-4 first:pt-0 sm:flex-row sm:items-start sm:justify-between">
                        <div className="min-w-0 space-y-1">
                          <div className="flex flex-wrap items-center gap-2">
                            <h2 className="font-medium">{debt.customer_name}</h2>
                            <Badge variant={debt.balance_due > 0 ? "secondary" : "outline"}>
                              {debt.balance_due > 0 ? "Outstanding" : "Settled"}
                            </Badge>
                          </div>
                          <p className="text-sm text-muted-foreground">
                            {debt.transaction_number} · {new Date(debt.created_at).toLocaleString()}
                          </p>
                          <p className="text-sm text-muted-foreground">
                            Original: {formatMoney(debt.original_amount)} · Paid: {formatMoney(debt.amount_paid)}
                          </p>
                        </div>
                        <div className="shrink-0 text-left sm:text-right">
                          <p className="text-xs text-muted-foreground">Balance due</p>
                          <p className="font-semibold">{formatMoney(debt.balance_due)}</p>
                        </div>
                      </li>
                    ))}
                  </ul>
                )}
              </CardContent>
            </Card>
          </div>
        </>
      )}
    </div>
  );
}
