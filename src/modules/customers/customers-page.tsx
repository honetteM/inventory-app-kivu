import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState, type FormEvent } from "react";
import { FileText, LoaderCircle, Pencil, Plus, Search, Save, UserRound, X } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { PageHeader } from "@/components/shared/page-header";
import { SelectField } from "@/components/shared/select-field";

interface Business {
  id: string;
  name: string;
}

type CustomerType = "individual" | "business";

interface Customer {
  id: string;
  business_id: string;
  name: string;
  phone: string | null;
  address: string | null;
  customer_type: CustomerType;
  credit_limit: number;
  outstanding_balance: number;
  is_active: boolean;
}

interface CustomerStatementEntry {
  id: string;
  transaction_number: string;
  entry_type: "sale" | "sale_payment" | "repayment";
  payment_method: string | null;
  amount: number;
  balance_after: number;
  created_at: string;
}

interface CustomerStatement {
  customer: Customer;
  total_sales: number;
  total_payments: number;
  outstanding_balance: number;
  entries: CustomerStatementEntry[];
}

interface CustomerForm {
  name: string;
  phone: string;
  address: string;
  customerType: CustomerType;
  creditLimit: string;
  isActive: boolean;
}

const EMPTY_FORM: CustomerForm = {
  name: "",
  phone: "",
  address: "",
  customerType: "individual",
  creditLimit: "0",
  isActive: true,
};

function errorText(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

function parseCreditLimit(value: string): number | null {
  const normalized = value.trim();
  if (normalized.length > 17 || !/^\d+(?:\.\d{1,2})?$/.test(normalized)) {
    return null;
  }
  const [whole, fraction = ""] = normalized.split(".");
  const scaled = BigInt(whole) * 100n + BigInt(fraction.padEnd(2, "0"));
  if (scaled > BigInt(Number.MAX_SAFE_INTEGER)) {
    return null;
  }
  return Number(scaled);
}

function formatMoney(scaledAmount: number): string {
  const amount = BigInt(scaledAmount);
  const whole = amount / 100n;
  const fraction = amount % 100n;
  const formattedWhole = new Intl.NumberFormat("en-RW").format(Number(whole));
  return `${formattedWhole}${fraction === 0n ? "" : `.${fraction.toString().padStart(2, "0")}`} RWF`;
}

function paymentMethodLabel(method: string): string {
  const labels: Record<string, string> = {
    cash: "Cash",
    mtn_mobile_money: "MTN Mobile Money",
    airtel_money: "Airtel Money",
    bank_transfer: "Bank transfer",
  };
  return labels[method] ?? method;
}

function formFromCustomer(customer: Customer): CustomerForm {
  return {
    name: customer.name,
    phone: customer.phone ?? "",
    address: customer.address ?? "",
    customerType: customer.customer_type,
    creditLimit: (customer.credit_limit / 100).toFixed(2).replace(/\.00$/, ""),
    isActive: customer.is_active,
  };
}

export function CustomersPage() {
  const [businesses, setBusinesses] = useState<Business[]>([]);
  const [businessId, setBusinessId] = useState("");
  const [customers, setCustomers] = useState<Customer[]>([]);
  const [form, setForm] = useState<CustomerForm>(EMPTY_FORM);
  const [search, setSearch] = useState("");
  const [editingCustomerId, setEditingCustomerId] = useState<string | null>(null);
  const [statementCustomerId, setStatementCustomerId] = useState<string | null>(null);
  const [statement, setStatement] = useState<CustomerStatement | null>(null);
  const [statementError, setStatementError] = useState<string | null>(null);
  const [isLoadingBusinesses, setIsLoadingBusinesses] = useState(true);
  const [isLoadingCustomers, setIsLoadingCustomers] = useState(false);
  const [isLoadingStatement, setIsLoadingStatement] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

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
      setCustomers([]);
      return;
    }

    let active = true;
    setIsLoadingCustomers(true);
    setError(null);
    invoke<Customer[]>("customer_list", { businessId })
      .then((result) => {
        if (active) {
          setCustomers(result);
        }
      })
      .catch((cause: unknown) => {
        if (active) {
          setError(`Could not load customers. ${errorText(cause)}`);
        }
      })
      .finally(() => {
        if (active) {
          setIsLoadingCustomers(false);
        }
      });

    return () => {
      active = false;
    };
  }, [businessId]);

  function resetForm() {
    setEditingCustomerId(null);
    setForm(EMPTY_FORM);
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const creditLimit = parseCreditLimit(form.creditLimit);
    if (!businessId || !form.name.trim()) {
      return;
    }
    if (creditLimit === null) {
      setError("Enter a non-negative credit limit with no more than 2 decimal places.");
      return;
    }

    setError(null);
    setIsSaving(true);
    const fields = {
      businessId,
      name: form.name.trim(),
      phone: form.phone.trim() || null,
      address: form.address.trim() || null,
      customerType: form.customerType,
      creditLimit,
    };
    try {
      if (editingCustomerId) {
        const customer = await invoke<Customer>("customer_update", {
          ...fields,
          customerId: editingCustomerId,
          isActive: form.isActive,
        });
        setCustomers((current) =>
          current
            .map((item) => item.id === customer.id ? customer : item)
            .sort((left, right) =>
              Number(right.is_active) - Number(left.is_active) ||
              left.name.localeCompare(right.name),
            ),
        );
      } else {
        const customer = await invoke<Customer>("customer_create", fields);
        setCustomers((current) =>
          [...current, customer].sort((left, right) =>
            Number(right.is_active) - Number(left.is_active) ||
            left.name.localeCompare(right.name),
          ),
        );
      }
      resetForm();
    } catch (cause) {
      setError(`Could not save the customer. ${errorText(cause)}`);
    } finally {
      setIsSaving(false);
    }
  }

  function editCustomer(customer: Customer) {
    setEditingCustomerId(customer.id);
    setForm(formFromCustomer(customer));
    setError(null);
  }

  async function viewStatement(customer: Customer) {
    setStatementCustomerId(customer.id);
    setStatement(null);
    setStatementError(null);
    setError(null);
    setIsLoadingStatement(true);
    try {
      const result = await invoke<CustomerStatement>("customer_statement", {
        businessId,
        customerId: customer.id,
      });
      setStatement(result);
    } catch (cause) {
      setStatementError(`Could not load the customer statement. ${errorText(cause)}`);
    } finally {
      setIsLoadingStatement(false);
    }
  }

  function closeStatement(open: boolean) {
    if (!open) {
      setStatementCustomerId(null);
      setStatement(null);
      setStatementError(null);
    }
  }

  const normalizedSearch = search.trim().toLocaleLowerCase();
  const filteredCustomers = customers.filter((customer) =>
    [customer.name, customer.phone ?? "", customer.address ?? ""]
      .some((value) => value.toLocaleLowerCase().includes(normalizedSearch)),
  );
  const pageHeader = (
    <PageHeader
      eyebrow="Relationships"
      title="Customers"
      description="Keep customer contact details and credit limits organized by business."
      actions={businesses.length > 0 ? (
        <div className="w-full space-y-1.5 sm:w-56">
          <Label htmlFor="customer-business">Business</Label>
          <SelectField
            id="customer-business"
            value={businessId}
            onChange={(event) => {
              setBusinessId(event.target.value);
              resetForm();
              closeStatement(false);
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

      {businesses.length === 0 ? (
        <Card>
          <CardContent className="py-6 text-sm text-muted-foreground">
            Register a business before adding customers.
          </CardContent>
        </Card>
      ) : (
        <>
          <div className="grid gap-6 xl:grid-cols-[minmax(20rem,0.8fr)_minmax(0,1.2fr)]">
            <Card>
              <CardHeader>
                <CardTitle>{editingCustomerId ? "Edit customer" : "Add customer"}</CardTitle>
                <CardDescription>
                  Customer records are kept separate for each business.
                </CardDescription>
              </CardHeader>
              <CardContent>
                <form className="space-y-4" onSubmit={handleSubmit}>
                  <div className="space-y-2">
                    <Label htmlFor="customer-name">Customer name</Label>
                    <Input
                      id="customer-name"
                      maxLength={160}
                      required
                      value={form.name}
                      onChange={(event) => setForm((current) => ({ ...current, name: event.target.value }))}
                      placeholder="e.g. Aline Mukamana"
                    />
                  </div>
                  <div className="grid gap-4 sm:grid-cols-2">
                    <div className="space-y-2">
                      <Label htmlFor="customer-type">Customer type</Label>
                      <SelectField
                        id="customer-type"
                        value={form.customerType}
                        onChange={(event) =>
                          setForm((current) => ({
                            ...current,
                            customerType: event.target.value === "business" ? "business" : "individual",
                          }))
                        }
                      >
                        <option value="individual">Individual</option>
                        <option value="business">Business</option>
                      </SelectField>
                    </div>
                    <div className="space-y-2">
                      <Label htmlFor="customer-phone">Phone (optional)</Label>
                      <Input
                        id="customer-phone"
                        type="tel"
                        maxLength={80}
                        value={form.phone}
                        onChange={(event) => setForm((current) => ({ ...current, phone: event.target.value }))}
                      />
                    </div>
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="customer-address">Address (optional)</Label>
                    <Input
                      id="customer-address"
                      maxLength={500}
                      value={form.address}
                      onChange={(event) => setForm((current) => ({ ...current, address: event.target.value }))}
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="customer-credit-limit">Credit limit (RWF)</Label>
                    <Input
                      id="customer-credit-limit"
                      type="number"
                      min="0"
                      step="0.01"
                      inputMode="decimal"
                      required
                      value={form.creditLimit}
                      onChange={(event) => setForm((current) => ({ ...current, creditLimit: event.target.value }))}
                    />
                    <p className="text-xs leading-5 text-muted-foreground">
                      Credit sales are checked against this limit and outstanding balances.
                    </p>
                  </div>
                  {editingCustomerId ? (
                    <label className="flex items-center gap-2 text-sm">
                      <input
                        type="checkbox"
                        checked={form.isActive}
                        onChange={(event) => setForm((current) => ({ ...current, isActive: event.target.checked }))}
                      />
                      Customer is active
                    </label>
                  ) : null}
                  <div className="flex flex-wrap gap-2">
                    <Button type="submit" disabled={isSaving || !form.name.trim()}>
                      {isSaving ? <LoaderCircle className="animate-spin" /> : editingCustomerId ? <Save /> : <Plus />}
                      {isSaving ? "Saving..." : editingCustomerId ? "Save changes" : "Add customer"}
                    </Button>
                    {editingCustomerId ? (
                      <Button type="button" variant="outline" onClick={resetForm} disabled={isSaving}>
                        <X />
                        Cancel
                      </Button>
                    ) : null}
                  </div>
                </form>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle>Customer directory</CardTitle>
                <CardDescription>
                  {filteredCustomers.length} of {customers.length} {customers.length === 1 ? "customer" : "customers"}
                </CardDescription>
                <div className="relative mt-3">
                  <Search className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground" />
                  <Input
                    aria-label="Search customers"
                    className="pl-9"
                    placeholder="Search name, phone, or address"
                    value={search}
                    onChange={(event) => setSearch(event.target.value)}
                  />
                </div>
              </CardHeader>
              <CardContent>
                {isLoadingCustomers ? (
                  <div className="flex items-center gap-2 py-4 text-sm text-muted-foreground">
                    <LoaderCircle className="size-4 animate-spin" />
                    Loading customers...
                  </div>
                ) : filteredCustomers.length === 0 ? (
                  <div className="flex items-center gap-3 py-4 text-sm text-muted-foreground">
                    <UserRound className="size-4 shrink-0" />
                    {customers.length === 0
                      ? "No customers have been added for this business."
                      : "No customers match your search."}
                  </div>
                ) : (
                  <ul className="divide-y">
                    {filteredCustomers.map((customer) => (
                      <li key={customer.id} className="flex flex-col gap-3 py-4 first:pt-0 sm:flex-row sm:items-start sm:justify-between">
                        <div className="min-w-0 space-y-1">
                          <div className="flex flex-wrap items-center gap-2">
                            <h2 className="font-medium">{customer.name}</h2>
                            <Badge variant={customer.is_active ? "secondary" : "outline"}>
                              {customer.is_active ? "Active" : "Inactive"}
                            </Badge>
                            <Badge variant="outline">
                              {customer.customer_type === "business" ? "Business" : "Individual"}
                            </Badge>
                          </div>
                          {customer.phone ? (
                            <p className="text-sm text-muted-foreground">Phone: {customer.phone}</p>
                          ) : null}
                          {customer.address ? (
                            <p className="text-sm text-muted-foreground">{customer.address}</p>
                          ) : null}
                          <p className="text-sm text-muted-foreground">
                            Credit limit: {formatMoney(customer.credit_limit)}
                          </p>
                          <p className="text-sm text-muted-foreground">
                            Outstanding balance: {formatMoney(customer.outstanding_balance)}
                          </p>
                        </div>
                        <div className="flex shrink-0 gap-2">
                          <Button
                            type="button"
                            variant="outline"
                            size="sm"
                            disabled={isLoadingStatement}
                            onClick={() => void viewStatement(customer)}
                          >
                            <FileText />
                            Statement
                          </Button>
                          <Button
                            type="button"
                            variant="outline"
                            size="sm"
                            onClick={() => editCustomer(customer)}
                          >
                            <Pencil />
                            Edit
                          </Button>
                        </div>
                      </li>
                    ))}
                  </ul>
                )}
              </CardContent>
            </Card>
          </div>
          <Card>
            <CardContent className="flex items-start gap-3 py-4 text-sm text-muted-foreground">
              <UserRound className="mt-0.5 size-4 shrink-0" />
              Review each customer&apos;s sale and payment history from their statement.
            </CardContent>
          </Card>
        </>
      )}
      <Dialog open={statementCustomerId !== null} onOpenChange={closeStatement}>
        <DialogContent className="max-h-[85vh] w-[calc(100vw-2rem)] max-w-4xl overflow-y-auto sm:max-w-4xl">
          <DialogHeader>
            <DialogTitle>
              {statement?.customer.name ?? "Customer statement"}
            </DialogTitle>
            <DialogDescription>
              Sales, payments received, and running outstanding balance.
            </DialogDescription>
          </DialogHeader>
          {isLoadingStatement ? (
            <div className="flex items-center gap-2 py-10 text-sm text-muted-foreground">
              <LoaderCircle className="size-4 animate-spin" />
              Loading customer statement...
            </div>
          ) : statementError ? (
            <div role="alert" className="rounded-lg border border-destructive/30 bg-destructive/5 p-4 text-sm text-destructive">
              {statementError}
            </div>
          ) : statement ? (
            <div className="space-y-5">
              <div className="grid gap-3 sm:grid-cols-3">
                <div className="rounded-lg border p-3">
                  <p className="text-xs text-muted-foreground">Total sales</p>
                  <p className="mt-1 font-semibold">{formatMoney(statement.total_sales)}</p>
                </div>
                <div className="rounded-lg border p-3">
                  <p className="text-xs text-muted-foreground">Total payments</p>
                  <p className="mt-1 font-semibold">{formatMoney(statement.total_payments)}</p>
                </div>
                <div className="rounded-lg border p-3">
                  <p className="text-xs text-muted-foreground">Outstanding balance</p>
                  <p className="mt-1 font-semibold">{formatMoney(statement.outstanding_balance)}</p>
                </div>
              </div>
              {statement.customer.phone || statement.customer.address ? (
                <div className="text-sm text-muted-foreground">
                  {statement.customer.phone ? <p>Phone: {statement.customer.phone}</p> : null}
                  {statement.customer.address ? <p>{statement.customer.address}</p> : null}
                </div>
              ) : null}
              {statement.entries.length === 0 ? (
                <p className="py-6 text-center text-sm text-muted-foreground">
                  No sales or payments have been recorded for this customer.
                </p>
              ) : (
                <div className="overflow-x-auto rounded-lg border">
                  <table className="w-full min-w-[680px] text-sm">
                    <thead className="bg-muted/50 text-left text-xs text-muted-foreground">
                      <tr>
                        <th className="px-3 py-2.5 font-medium">Date</th>
                        <th className="px-3 py-2.5 font-medium">Transaction</th>
                        <th className="px-3 py-2.5 font-medium">Details</th>
                        <th className="px-3 py-2.5 text-right font-medium">Charge</th>
                        <th className="px-3 py-2.5 text-right font-medium">Payment</th>
                        <th className="px-3 py-2.5 text-right font-medium">Balance</th>
                      </tr>
                    </thead>
                    <tbody className="divide-y">
                      {statement.entries.map((entry) => {
                        const isSale = entry.entry_type === "sale";
                        const label = isSale
                          ? "Sale"
                          : entry.entry_type === "sale_payment"
                            ? "Payment at sale"
                            : "Repayment";
                        return (
                          <tr key={`${entry.entry_type}-${entry.id}`}>
                            <td className="whitespace-nowrap px-3 py-3 text-muted-foreground">
                              {new Date(entry.created_at).toLocaleString()}
                            </td>
                            <td className="whitespace-nowrap px-3 py-3 font-medium">
                              {entry.transaction_number}
                            </td>
                            <td className="px-3 py-3">
                              <div className="flex flex-wrap items-center gap-2">
                                <Badge variant={isSale ? "secondary" : "outline"}>{label}</Badge>
                                {entry.payment_method ? (
                                  <span className="text-muted-foreground">
                                    {paymentMethodLabel(entry.payment_method)}
                                  </span>
                                ) : null}
                              </div>
                            </td>
                            <td className="whitespace-nowrap px-3 py-3 text-right">
                              {isSale ? formatMoney(entry.amount) : "—"}
                            </td>
                            <td className="whitespace-nowrap px-3 py-3 text-right">
                              {isSale ? "—" : formatMoney(entry.amount)}
                            </td>
                            <td className="whitespace-nowrap px-3 py-3 text-right font-medium">
                              {formatMoney(entry.balance_after)}
                            </td>
                          </tr>
                        );
                      })}
                    </tbody>
                  </table>
                </div>
              )}
            </div>
          ) : null}
        </DialogContent>
      </Dialog>
    </div>
  );
}
