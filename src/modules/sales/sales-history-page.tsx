import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { Eye, LoaderCircle, Search } from "lucide-react";

import { PageHeader } from "@/components/shared/page-header";
import { SelectField } from "@/components/shared/select-field";
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
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";

const PAGE_SIZE = 30;
const MONEY_SCALE = 100n;
const QUANTITY_SCALE = 1000n;

interface Business {
  id: string;
  name: string;
}

interface SaleHistoryItem {
  product_name: string;
  sku: string;
  quantity: number;
  unit_name: string;
  unit_price: number;
  line_total: number;
}

interface SaleHistoryPayment {
  payment_method: string;
  amount: number;
}

interface SaleHistoryEntry {
  id: string;
  transaction_number: string;
  status: "completed" | "cancelled";
  customer_name: string | null;
  sold_by: string | null;
  subtotal: number;
  discount: number;
  total: number;
  amount_paid: number;
  balance_due: number;
  item_quantity: number;
  created_at: string;
  items: SaleHistoryItem[];
  payments: SaleHistoryPayment[];
}

interface SaleHistoryPageData {
  items: SaleHistoryEntry[];
  total_count: number;
  completed_sales_total: number;
  collected_at_sale_total: number;
}

function localDate(date = new Date()): string {
  const offset = date.getTimezoneOffset() * 60_000;
  return new Date(date.getTime() - offset).toISOString().slice(0, 10);
}

function dateThirtyDaysAgo(): string {
  const date = new Date();
  date.setDate(date.getDate() - 29);
  return localDate(date);
}

function formatMoney(value: number): string {
  if (!Number.isSafeInteger(value)) {
    return "—";
  }
  const amount = BigInt(value);
  const absoluteAmount = amount < 0n ? -amount : amount;
  const whole = absoluteAmount / MONEY_SCALE;
  const fraction = absoluteAmount % MONEY_SCALE;
  return `${amount < 0n ? "−" : ""}${new Intl.NumberFormat("en-RW").format(Number(whole))}${
    fraction === 0n ? "" : `.${fraction.toString().padStart(2, "0")}`
  } RWF`;
}

function formatQuantity(value: number): string {
  if (!Number.isSafeInteger(value)) {
    return "—";
  }
  const quantity = BigInt(value);
  const whole = quantity / QUANTITY_SCALE;
  const fraction = quantity % QUANTITY_SCALE;
  return fraction === 0n
    ? new Intl.NumberFormat("en-RW").format(Number(whole))
    : `${new Intl.NumberFormat("en-RW").format(Number(whole))}.${fraction.toString().padStart(3, "0").replace(/0+$/, "")}`;
}

function formatTimestamp(value: string): string {
  const date = new Date(value);
  return Number.isNaN(date.getTime())
    ? value
    : date.toLocaleString("en-RW", {
        day: "2-digit",
        month: "short",
        year: "numeric",
        hour: "2-digit",
        minute: "2-digit",
      });
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

function errorText(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

export function SalesHistoryPage() {
  const [businesses, setBusinesses] = useState<Business[]>([]);
  const [businessId, setBusinessId] = useState("");
  const [search, setSearch] = useState("");
  const [fromDate, setFromDate] = useState(dateThirtyDaysAgo);
  const [toDate, setToDate] = useState(localDate);
  const [page, setPage] = useState(0);
  const [history, setHistory] = useState<SaleHistoryPageData | null>(null);
  const [selectedSale, setSelectedSale] = useState<SaleHistoryEntry | null>(null);
  const [isLoadingBusinesses, setIsLoadingBusinesses] = useState(true);
  const [isLoadingHistory, setIsLoadingHistory] = useState(false);
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
    if (!businessId || !fromDate || !toDate || fromDate > toDate) {
      setHistory(null);
      setIsLoadingHistory(false);
      return;
    }

    let active = true;
    const timer = window.setTimeout(() => {
      setIsLoadingHistory(true);
      setError(null);
      invoke<SaleHistoryPageData>("sales_history_list", {
        businessId,
        search: search.trim() || null,
        fromDate,
        toDate,
        offset: page * PAGE_SIZE,
        limit: PAGE_SIZE,
      })
        .then((result) => {
          if (active) {
            setHistory(result);
          }
        })
        .catch((cause: unknown) => {
          if (active) {
            setHistory(null);
            setError(`Could not load sales history. ${errorText(cause)}`);
          }
        })
        .finally(() => {
          if (active) {
            setIsLoadingHistory(false);
          }
        });
    }, search.trim() ? 250 : 0);
    return () => {
      active = false;
      window.clearTimeout(timer);
    };
  }, [businessId, fromDate, toDate, search, page]);

  const selectedBusiness = businesses.find((business) => business.id === businessId);
  const totalPages = Math.max(1, Math.ceil((history?.total_count ?? 0) / PAGE_SIZE));
  const invalidDateRange = Boolean(fromDate && toDate && fromDate > toDate);

  return (
    <div className="space-y-6">
      <PageHeader
        eyebrow="Business"
        title="Sales history"
        description={
          selectedBusiness
            ? `Review completed and cancelled transactions for ${selectedBusiness.name}.`
            : "Search sales, review payments, and open transaction details."
        }
        actions={
          <div className="w-full space-y-1.5 sm:w-56">
            <label htmlFor="sales-business" className="text-xs font-medium text-muted-foreground">
              Business
            </label>
            <SelectField
              id="sales-business"
              value={businessId}
              onChange={(event) => {
                setBusinessId(event.target.value);
                setPage(0);
              }}
              disabled={isLoadingBusinesses || businesses.length === 0}
            >
              {businesses.map((business) => (
                <option key={business.id} value={business.id}>
                  {business.name}
                </option>
              ))}
            </SelectField>
          </div>
        }
      />

      <Card>
        <CardHeader>
          <CardTitle>Find transactions</CardTitle>
          <CardDescription>Search by receipt number, customer, product name, or SKU.</CardDescription>
        </CardHeader>
        <CardContent className="grid gap-4 md:grid-cols-[minmax(16rem,1fr)_repeat(2,minmax(9rem,12rem))]">
          <div className="relative space-y-1.5">
            <label htmlFor="sales-search" className="text-xs font-medium text-muted-foreground">
              Search
            </label>
            <Search className="pointer-events-none absolute top-9 left-3 size-4 text-muted-foreground" />
            <Input
              id="sales-search"
              value={search}
              onChange={(event) => {
                setSearch(event.target.value);
                setPage(0);
              }}
              placeholder="Receipt, customer, product, or SKU"
              className="pl-9"
            />
          </div>
          <div className="space-y-1.5">
            <label htmlFor="sales-from-date" className="text-xs font-medium text-muted-foreground">
              From date
            </label>
            <Input
              id="sales-from-date"
              type="date"
              value={fromDate}
              max={toDate || undefined}
              onChange={(event) => {
                setFromDate(event.target.value);
                setPage(0);
              }}
            />
          </div>
          <div className="space-y-1.5">
            <label htmlFor="sales-to-date" className="text-xs font-medium text-muted-foreground">
              To date
            </label>
            <Input
              id="sales-to-date"
              type="date"
              value={toDate}
              min={fromDate || undefined}
              onChange={(event) => {
                setToDate(event.target.value);
                setPage(0);
              }}
            />
          </div>
        </CardContent>
      </Card>

      {invalidDateRange ? (
        <div role="alert" className="rounded-md border border-destructive/30 bg-destructive/5 p-4 text-sm text-destructive">
          The start date cannot be after the end date.
        </div>
      ) : null}
      {error ? (
        <div role="alert" className="rounded-md border border-destructive/30 bg-destructive/5 p-4 text-sm text-destructive">
          {error}
        </div>
      ) : null}

      <div className="grid gap-4 sm:grid-cols-3">
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Matching transactions</CardDescription>
            <CardTitle className="data-number text-2xl">{history?.total_count ?? "—"}</CardTitle>
          </CardHeader>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Completed sales total</CardDescription>
            <CardTitle className="data-number text-xl">{formatMoney(history?.completed_sales_total ?? 0)}</CardTitle>
          </CardHeader>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Collected at sale</CardDescription>
            <CardTitle className="data-number text-xl">{formatMoney(history?.collected_at_sale_total ?? 0)}</CardTitle>
          </CardHeader>
        </Card>
      </div>

      <Card>
        <CardHeader className="flex flex-row items-center justify-between gap-4">
          <div>
            <CardTitle>Transactions</CardTitle>
            <CardDescription className="mt-1">
              {history ? `${history.total_count} matching transaction${history.total_count === 1 ? "" : "s"}` : "Sales recorded in the selected date range."}
            </CardDescription>
          </div>
          {isLoadingHistory ? <LoaderCircle className="size-5 animate-spin text-muted-foreground" aria-label="Loading sales" /> : null}
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Receipt</TableHead>
                <TableHead>Date</TableHead>
                <TableHead>Customer</TableHead>
                <TableHead>Items</TableHead>
                <TableHead className="text-right">Total</TableHead>
                <TableHead className="text-right">Paid at sale</TableHead>
                <TableHead>Status</TableHead>
                <TableHead className="text-right">Details</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {history?.items.map((sale) => (
                <TableRow key={sale.id}>
                  <TableCell className="font-medium">{sale.transaction_number}</TableCell>
                  <TableCell>{formatTimestamp(sale.created_at)}</TableCell>
                  <TableCell>{sale.customer_name ?? "Walk-in customer"}</TableCell>
                  <TableCell>{formatQuantity(sale.item_quantity)} items</TableCell>
                  <TableCell className="data-number text-right font-medium">{formatMoney(sale.total)}</TableCell>
                  <TableCell className="data-number text-right">{formatMoney(sale.amount_paid)}</TableCell>
                  <TableCell>
                    <Badge variant={sale.status === "completed" ? "secondary" : "destructive"}>
                      {sale.status === "completed" ? "Completed" : "Cancelled"}
                    </Badge>
                  </TableCell>
                  <TableCell className="text-right">
                    <Button variant="outline" size="sm" onClick={() => setSelectedSale(sale)}>
                      <Eye />
                      View
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
              {!isLoadingHistory && history?.items.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={8} className="h-24 text-center text-muted-foreground">
                    No sales match these filters.
                  </TableCell>
                </TableRow>
              ) : null}
              {!history && !isLoadingHistory && !error && !invalidDateRange ? (
                <TableRow>
                  <TableCell colSpan={8} className="h-24 text-center text-muted-foreground">
                    {isLoadingBusinesses ? "Loading sales history…" : "Select a business to view sales."}
                  </TableCell>
                </TableRow>
              ) : null}
            </TableBody>
          </Table>
          <div className="mt-4 flex flex-col gap-3 border-t pt-4 sm:flex-row sm:items-center sm:justify-between">
            <p className="text-sm text-muted-foreground">
              Page {page + 1} of {totalPages}
            </p>
            <div className="flex gap-2">
              <Button
                type="button"
                variant="outline"
                size="sm"
                disabled={page === 0 || isLoadingHistory}
                onClick={() => setPage((current) => Math.max(0, current - 1))}
              >
                Previous
              </Button>
              <Button
                type="button"
                variant="outline"
                size="sm"
                disabled={page + 1 >= totalPages || isLoadingHistory}
                onClick={() => setPage((current) => current + 1)}
              >
                Next
              </Button>
            </div>
          </div>
        </CardContent>
      </Card>

      <Dialog open={selectedSale !== null} onOpenChange={(open) => !open && setSelectedSale(null)}>
        <DialogContent size="wide">
          {selectedSale ? (
            <>
              <DialogHeader>
                <DialogTitle>Sale {selectedSale.transaction_number}</DialogTitle>
                <DialogDescription>
                  {formatTimestamp(selectedSale.created_at)}
                  {selectedSale.sold_by ? ` · Sold by ${selectedSale.sold_by}` : ""}
                  {selectedSale.customer_name ? ` · ${selectedSale.customer_name}` : " · Walk-in customer"}
                </DialogDescription>
              </DialogHeader>
              <div className="grid gap-6 lg:grid-cols-[minmax(0,1.5fr)_minmax(16rem,1fr)]">
                <div className="min-w-0">
                  <h3 className="mb-2 text-sm font-semibold">Items sold</h3>
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>Product</TableHead>
                        <TableHead>Quantity</TableHead>
                        <TableHead className="text-right">Unit price</TableHead>
                        <TableHead className="text-right">Line total</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {selectedSale.items.map((item, index) => (
                        <TableRow key={`${item.sku}-${index}`}>
                          <TableCell>
                            <p className="font-medium">{item.product_name}</p>
                            <p className="text-xs text-muted-foreground">{item.sku}</p>
                          </TableCell>
                          <TableCell>{formatQuantity(item.quantity)} {item.unit_name}</TableCell>
                          <TableCell className="data-number text-right">{formatMoney(item.unit_price)}</TableCell>
                          <TableCell className="data-number text-right font-medium">{formatMoney(item.line_total)}</TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                </div>
                <div className="space-y-5">
                  <div>
                    <h3 className="mb-2 text-sm font-semibold">Payments recorded at checkout</h3>
                    {selectedSale.payments.length ? (
                      <ul className="divide-y divide-border">
                        {selectedSale.payments.map((payment, index) => (
                          <li key={`${payment.payment_method}-${index}`} className="flex justify-between gap-3 py-2 text-sm">
                            <span>{paymentMethodLabel(payment.payment_method)}</span>
                            <span className="data-number font-medium">{formatMoney(payment.amount)}</span>
                          </li>
                        ))}
                      </ul>
                    ) : (
                      <p className="text-sm text-muted-foreground">No payment was collected at checkout.</p>
                    )}
                  </div>
                  <dl className="space-y-2 border-t pt-3 text-sm">
                    <div className="flex justify-between gap-3">
                      <dt className="text-muted-foreground">Subtotal</dt>
                      <dd className="data-number">{formatMoney(selectedSale.subtotal)}</dd>
                    </div>
                    <div className="flex justify-between gap-3">
                      <dt className="text-muted-foreground">Discount</dt>
                      <dd className="data-number">{formatMoney(selectedSale.discount)}</dd>
                    </div>
                    <div className="flex justify-between gap-3 border-t pt-2 font-semibold">
                      <dt>Total</dt>
                      <dd className="data-number">{formatMoney(selectedSale.total)}</dd>
                    </div>
                    <div className="flex justify-between gap-3">
                      <dt className="text-muted-foreground">Paid at sale</dt>
                      <dd className="data-number">{formatMoney(selectedSale.amount_paid)}</dd>
                    </div>
                    <div className="flex justify-between gap-3">
                      <dt className="text-muted-foreground">Credit at sale</dt>
                      <dd className="data-number">{formatMoney(selectedSale.balance_due)}</dd>
                    </div>
                  </dl>
                </div>
              </div>
            </>
          ) : null}
        </DialogContent>
      </Dialog>
    </div>
  );
}
