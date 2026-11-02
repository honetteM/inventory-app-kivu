import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState, type FormEvent } from "react";
import { ArrowLeft, ArrowRight, LoaderCircle, Plus, Receipt, Wallet } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { PageHeader } from "@/components/shared/page-header";
import { SelectField } from "@/components/shared/select-field";

const PAGE_SIZE = 20;
const MONEY_SCALE = 100n;
const MAX_SAFE_INTEGER = BigInt(Number.MAX_SAFE_INTEGER);

type ExpenseCategory =
  | "rent"
  | "electricity"
  | "transport"
  | "salaries"
  | "internet"
  | "repairs"
  | "security"
  | "other";

type PaymentMethod = "cash" | "mtn_mobile_money" | "airtel_money" | "bank_transfer";

const CATEGORIES: { value: ExpenseCategory; label: string }[] = [
  { value: "rent", label: "Rent" },
  { value: "electricity", label: "Electricity" },
  { value: "transport", label: "Transport" },
  { value: "salaries", label: "Salaries" },
  { value: "internet", label: "Internet" },
  { value: "repairs", label: "Repairs" },
  { value: "security", label: "Security" },
  { value: "other", label: "Other" },
];

const PAYMENT_METHODS: { value: PaymentMethod; label: string }[] = [
  { value: "cash", label: "Cash" },
  { value: "mtn_mobile_money", label: "MTN Mobile Money" },
  { value: "airtel_money", label: "Airtel Money" },
  { value: "bank_transfer", label: "Bank transfer" },
];

interface Business {
  id: string;
  name: string;
}

interface Expense {
  id: string;
  business_id: string;
  category: ExpenseCategory;
  amount: number;
  expense_date: string;
  payment_method: PaymentMethod;
  description: string;
  responsible_name: string;
  created_at: string;
}

interface ExpensePageData {
  items: Expense[];
  total_count: number;
}

interface ExpenseForm {
  category: ExpenseCategory;
  amount: string;
  expenseDate: string;
  paymentMethod: PaymentMethod;
  description: string;
  responsibleName: string;
}

function localDate(): string {
  const date = new Date();
  const offset = date.getTimezoneOffset() * 60_000;
  return new Date(date.getTime() - offset).toISOString().slice(0, 10);
}

function emptyForm(): ExpenseForm {
  return {
    category: "other",
    amount: "",
    expenseDate: localDate(),
    paymentMethod: "cash",
    description: "",
    responsibleName: "",
  };
}

function parseMoney(value: string): number | null {
  const normalized = value.trim();
  const match = /^(\d+)(?:\.(\d{1,2}))?$/.exec(normalized);
  if (!match || normalized.length > 17) {
    return null;
  }
  const amount = BigInt(match[1]) * MONEY_SCALE + BigInt((match[2] ?? "").padEnd(2, "0"));
  return amount > 0n && amount <= MAX_SAFE_INTEGER ? Number(amount) : null;
}

function formatMoney(scaledAmount: number): string {
  if (!Number.isSafeInteger(scaledAmount) || scaledAmount < 0) {
    return "Invalid amount";
  }
  const amount = BigInt(scaledAmount);
  const whole = amount / MONEY_SCALE;
  const fraction = amount % MONEY_SCALE;
  return `${new Intl.NumberFormat("en-RW").format(Number(whole))}${
    fraction === 0n ? "" : `.${fraction.toString().padStart(2, "0")}`
  } RWF`;
}

function errorText(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

function categoryLabel(category: ExpenseCategory): string {
  return CATEGORIES.find((item) => item.value === category)?.label ?? category;
}

function paymentMethodLabel(method: PaymentMethod): string {
  return PAYMENT_METHODS.find((item) => item.value === method)?.label ?? method;
}

export function ExpensesPage() {
  const [businesses, setBusinesses] = useState<Business[]>([]);
  const [businessId, setBusinessId] = useState("");
  const [expenses, setExpenses] = useState<Expense[]>([]);
  const [totalCount, setTotalCount] = useState(0);
  const [page, setPage] = useState(0);
  const [refreshToken, setRefreshToken] = useState(0);
  const [form, setForm] = useState<ExpenseForm>(emptyForm);
  const [isLoadingBusinesses, setIsLoadingBusinesses] = useState(true);
  const [isLoadingExpenses, setIsLoadingExpenses] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const pageCount = Math.ceil(totalCount / PAGE_SIZE);
  const pageTotal = expenses.reduce((sum, expense) => sum + expense.amount, 0);

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
      setExpenses([]);
      setTotalCount(0);
      return;
    }

    let active = true;
    setIsLoadingExpenses(true);
    setError(null);
    invoke<ExpensePageData>("expense_list", {
      businessId,
      offset: page * PAGE_SIZE,
      limit: PAGE_SIZE,
    })
      .then((result) => {
        if (active) {
          setExpenses(result.items);
          setTotalCount(result.total_count);
        }
      })
      .catch((cause: unknown) => {
        if (active) {
          setError(`Could not load expenses. ${errorText(cause)}`);
        }
      })
      .finally(() => {
        if (active) {
          setIsLoadingExpenses(false);
        }
      });
    return () => {
      active = false;
    };
  }, [businessId, page, refreshToken]);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const amount = parseMoney(form.amount);
    if (!businessId || amount === null) {
      setError("Enter an expense amount greater than zero with no more than 2 decimal places.");
      return;
    }
    if (!form.description.trim() || !form.responsibleName.trim()) {
      setError("Enter a description and the responsible person.");
      return;
    }

    setError(null);
    setNotice(null);
    setIsSaving(true);
    try {
      const expense = await invoke<Expense>("expense_create", {
        businessId,
        category: form.category,
        amount,
        expenseDate: form.expenseDate,
        paymentMethod: form.paymentMethod,
        description: form.description.trim(),
        responsibleName: form.responsibleName.trim(),
      });
      setForm(emptyForm());
      setPage(0);
      setTotalCount((current) => current + 1);
      setRefreshToken((current) => current + 1);
      setNotice(`Recorded ${categoryLabel(expense.category).toLowerCase()} expense: ${formatMoney(expense.amount)}.`);
    } catch (cause) {
      setError(`Could not record the expense. ${errorText(cause)}`);
    } finally {
      setIsSaving(false);
    }
  }

  const pageHeader = (
    <PageHeader
      eyebrow="Finance"
      title="Expenses"
      description="Record operating costs separately from stock purchases and supplier invoices."
      actions={businesses.length > 0 ? (
        <div className="w-full space-y-1.5 sm:w-56">
          <Label htmlFor="expenses-business">Business</Label>
          <SelectField
            id="expenses-business"
            value={businessId}
            onChange={(event) => {
              setBusinessId(event.target.value);
              setPage(0);
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
            Register a business before recording expenses.
          </CardContent>
        </Card>
      ) : (
        <div className="grid items-start gap-6 xl:grid-cols-[minmax(20rem,0.8fr)_minmax(0,1.2fr)]">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Plus className="size-4" />
                Record expense
              </CardTitle>
              <CardDescription>
                Operating costs are logged independently of inventory purchases.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <form className="space-y-4" onSubmit={handleSubmit}>
                <div className="grid gap-4 sm:grid-cols-2">
                  <div className="space-y-2">
                    <Label htmlFor="expense-category">Category</Label>
                    <SelectField
                      id="expense-category"
                      value={form.category}
                      onChange={(event) => {
                        const selected = CATEGORIES.find((item) => item.value === event.target.value);
                        if (selected) {
                          setForm((current) => ({ ...current, category: selected.value }));
                        }
                      }}
                    >
                      {CATEGORIES.map((category) => (
                        <option key={category.value} value={category.value}>{category.label}</option>
                      ))}
                    </SelectField>
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="expense-date">Date</Label>
                    <Input
                      id="expense-date"
                      type="date"
                      required
                      value={form.expenseDate}
                      onChange={(event) => setForm((current) => ({ ...current, expenseDate: event.target.value }))}
                    />
                  </div>
                </div>
                <div className="grid gap-4 sm:grid-cols-2">
                  <div className="space-y-2">
                    <Label htmlFor="expense-amount">Amount (RWF)</Label>
                    <Input
                      id="expense-amount"
                      type="number"
                      min="0.01"
                      step="0.01"
                      inputMode="decimal"
                      required
                      value={form.amount}
                      onChange={(event) => setForm((current) => ({ ...current, amount: event.target.value }))}
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="expense-payment-method">Payment method</Label>
                    <SelectField
                      id="expense-payment-method"
                      value={form.paymentMethod}
                      onChange={(event) => {
                        const selected = PAYMENT_METHODS.find((method) => method.value === event.target.value);
                        if (selected) {
                          setForm((current) => ({ ...current, paymentMethod: selected.value }));
                        }
                      }}
                    >
                      {PAYMENT_METHODS.map((method) => (
                        <option key={method.value} value={method.value}>{method.label}</option>
                      ))}
                    </SelectField>
                  </div>
                </div>
                <div className="space-y-2">
                  <Label htmlFor="expense-description">Description</Label>
                  <Input
                    id="expense-description"
                    maxLength={1000}
                    required
                    value={form.description}
                    onChange={(event) => setForm((current) => ({ ...current, description: event.target.value }))}
                    placeholder="What was this expense for?"
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="expense-responsible">Responsible person</Label>
                  <Input
                    id="expense-responsible"
                    maxLength={160}
                    required
                    value={form.responsibleName}
                    onChange={(event) => setForm((current) => ({ ...current, responsibleName: event.target.value }))}
                    placeholder="Name of the person recording this expense"
                  />
                  <p className="text-xs leading-5 text-muted-foreground">
                    Staff accounts are not connected to the expense workflow yet.
                  </p>
                </div>
                <Button type="submit" disabled={isSaving || !form.amount.trim() || !form.description.trim() || !form.responsibleName.trim()}>
                  {isSaving ? <LoaderCircle className="animate-spin" /> : <Wallet />}
                  {isSaving ? "Recording..." : "Record expense"}
                </Button>
              </form>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>Recent expenses</CardTitle>
              <CardDescription>
                {totalCount.toLocaleString("en-RW")} total records · {formatMoney(pageTotal)} on this page
              </CardDescription>
            </CardHeader>
            <CardContent>
              {isLoadingExpenses ? (
                <div className="flex items-center gap-2 py-6 text-sm text-muted-foreground">
                  <LoaderCircle className="size-4 animate-spin" />
                  Loading expenses...
                </div>
              ) : expenses.length === 0 ? (
                <div className="flex items-center gap-3 py-6 text-sm text-muted-foreground">
                  <Receipt className="size-4 shrink-0" />
                  No operating expenses have been recorded for this business.
                </div>
              ) : (
                <ul className="divide-y">
                  {expenses.map((expense) => (
                    <li key={expense.id} className="flex flex-col gap-3 py-4 first:pt-0 sm:flex-row sm:items-start sm:justify-between">
                      <div className="min-w-0 space-y-1">
                        <div className="flex flex-wrap items-center gap-2">
                          <h2 className="font-medium">{expense.description}</h2>
                          <Badge variant="secondary">{categoryLabel(expense.category)}</Badge>
                        </div>
                        <p className="text-sm text-muted-foreground">
                          {new Date(`${expense.expense_date}T00:00:00`).toLocaleDateString()} · {paymentMethodLabel(expense.payment_method)}
                        </p>
                        <p className="text-sm text-muted-foreground">
                          Recorded by {expense.responsible_name}
                        </p>
                      </div>
                      <p className="shrink-0 font-semibold">{formatMoney(expense.amount)}</p>
                    </li>
                  ))}
                </ul>
              )}
              {totalCount > PAGE_SIZE ? (
                <div className="flex items-center justify-between border-t pt-4">
                  <p className="text-sm text-muted-foreground">
                    Page {page + 1} of {pageCount}
                  </p>
                  <div className="flex gap-2">
                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      disabled={page === 0 || isLoadingExpenses}
                      onClick={() => setPage((current) => Math.max(0, current - 1))}
                    >
                      <ArrowLeft />
                      Previous
                    </Button>
                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      disabled={page + 1 >= pageCount || isLoadingExpenses}
                      onClick={() => setPage((current) => current + 1)}
                    >
                      Next
                      <ArrowRight />
                    </Button>
                  </div>
                </div>
              ) : null}
            </CardContent>
          </Card>
        </div>
      )}
    </div>
  );
}
