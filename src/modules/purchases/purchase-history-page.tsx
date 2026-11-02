import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { Eye, LoaderCircle, Search } from "lucide-react";

import { PageHeader } from "@/components/shared/page-header";
import { SelectField } from "@/components/shared/select-field";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";

const PAGE_SIZE = 30;
const MONEY_SCALE = 100n;
const QUANTITY_SCALE = 1000n;

interface Business { id: string; name: string }
interface PurchaseItem {
  id: string; product_name: string; sku: string; unit_name: string;
  ordered_quantity: number; received_quantity: number; unit_cost: number; line_total: number;
}
interface Purchase {
  id: string; supplier_name: string; supplier_invoice_number: string; status: string;
  subtotal: number; amount_paid: number; balance_due: number; created_at: string;
  items: PurchaseItem[];
  deliveries: { id: string; delivery_reference: string | null; notes: string | null; total_cost: number; created_at: string; items: { product_name: string; quantity: number; unit_name: string; total_cost: number }[] }[];
  payments: { id: string; payment_method: string; amount: number; created_at: string }[];
}
interface PurchaseHistoryData {
  items: Purchase[]; total_count: number; purchase_total: number; paid_total: number; balance_due_total: number;
}

function localDate(date = new Date()): string {
  const offset = date.getTimezoneOffset() * 60000;
  return new Date(date.getTime() - offset).toISOString().slice(0, 10);
}
function formatMoney(value: number): string {
  if (!Number.isSafeInteger(value)) return "—";
  const amount = BigInt(value); const whole = amount / MONEY_SCALE; const fraction = amount % MONEY_SCALE;
  return `${new Intl.NumberFormat("en-RW").format(Number(whole))}${fraction ? `.${fraction.toString().padStart(2, "0")}` : ""} RWF`;
}
function formatQuantity(value: number): string {
  const quantity = BigInt(value); const whole = quantity / QUANTITY_SCALE; const fraction = quantity % QUANTITY_SCALE;
  return fraction === 0n ? new Intl.NumberFormat("en-RW").format(Number(whole)) : `${whole}.${fraction.toString().padStart(3, "0").replace(/0+$/, "")}`;
}
function errorText(cause: unknown): string { return cause instanceof Error ? cause.message : String(cause); }
function timestamp(value: string): string {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString("en-RW", { day: "2-digit", month: "short", year: "numeric" });
}

export function PurchaseHistoryPage() {
  const [businesses, setBusinesses] = useState<Business[]>([]);
  const [businessId, setBusinessId] = useState("");
  const [search, setSearch] = useState("");
  const [fromDate, setFromDate] = useState(localDate(new Date(Date.now() - 29 * 86400000)));
  const [toDate, setToDate] = useState(localDate());
  const [page, setPage] = useState(0);
  const [data, setData] = useState<PurchaseHistoryData | null>(null);
  const [selected, setSelected] = useState<Purchase | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<Business[]>("business_list").then((result) => {
      setBusinesses(result);
      setBusinessId((current) => result.some((item) => item.id === current) ? current : (result[0]?.id ?? ""));
    }).catch((cause) => setError(`Could not load businesses. ${errorText(cause)}`));
  }, []);
  useEffect(() => {
    if (!businessId || !fromDate || !toDate || fromDate > toDate) { setData(null); return; }
    let active = true;
    const timer = window.setTimeout(() => {
      setLoading(true); setError(null);
      invoke<PurchaseHistoryData>("purchase_history_list", {
        businessId, search: search.trim() || null, fromDate, toDate, offset: page * PAGE_SIZE, limit: PAGE_SIZE,
      }).then((result) => { if (active) setData(result); })
        .catch((cause) => { if (active) { setData(null); setError(`Could not load purchase history. ${errorText(cause)}`); } })
        .finally(() => { if (active) setLoading(false); });
    }, search.trim() ? 250 : 0);
    return () => { active = false; window.clearTimeout(timer); };
  }, [businessId, fromDate, toDate, search, page]);

  const totalPages = Math.max(1, Math.ceil((data?.total_count ?? 0) / PAGE_SIZE));
  const invalidRange = Boolean(fromDate && toDate && fromDate > toDate);
  return (
    <div className="space-y-6">
      <PageHeader eyebrow="Purchasing" title="Purchase history" description="Review purchases that are fully received and fully paid." actions={
        <div className="w-full space-y-1.5 sm:w-56"><label className="text-xs font-medium text-muted-foreground">Business</label>
          <SelectField value={businessId} onChange={(event) => { setBusinessId(event.target.value); setPage(0); }} disabled={!businesses.length}>
            {businesses.map((business) => <option key={business.id} value={business.id}>{business.name}</option>)}
          </SelectField>
        </div>
      } />
      <Card><CardHeader><CardTitle>Find completed purchases</CardTitle><CardDescription>Search by supplier, invoice, product, SKU, or delivery reference.</CardDescription></CardHeader>
        <CardContent className="grid gap-4 md:grid-cols-[minmax(16rem,1fr)_repeat(2,minmax(9rem,12rem))]">
          <div className="relative space-y-1.5"><label htmlFor="purchase-search" className="text-xs font-medium text-muted-foreground">Search</label><Search className="pointer-events-none absolute top-9 left-3 size-4 text-muted-foreground" /><Input id="purchase-search" className="pl-9" value={search} onChange={(event) => { setSearch(event.target.value); setPage(0); }} placeholder="Supplier, invoice, product, SKU" /></div>
          <div className="space-y-1.5"><label htmlFor="purchase-from" className="text-xs font-medium text-muted-foreground">From date</label><Input id="purchase-from" type="date" value={fromDate} onChange={(event) => { setFromDate(event.target.value); setPage(0); }} /></div>
          <div className="space-y-1.5"><label htmlFor="purchase-to" className="text-xs font-medium text-muted-foreground">To date</label><Input id="purchase-to" type="date" value={toDate} onChange={(event) => { setToDate(event.target.value); setPage(0); }} /></div>
        </CardContent>
      </Card>
      {invalidRange ? <div role="alert" className="rounded-md border border-destructive/30 bg-destructive/5 p-4 text-sm text-destructive">The start date cannot be after the end date.</div> : null}
      {error ? <div role="alert" className="rounded-md border border-destructive/30 bg-destructive/5 p-4 text-sm text-destructive">{error}</div> : null}
      <div className="grid gap-4 sm:grid-cols-4">
        {[
          ["Matching purchases", data?.total_count ?? 0],
          ["Purchase total", formatMoney(data?.purchase_total ?? 0)],
          ["Paid", formatMoney(data?.paid_total ?? 0)],
          ["Balance due", formatMoney(data?.balance_due_total ?? 0)],
        ].map(([label, value]) => <Card key={String(label)}><CardHeader className="pb-2"><CardDescription>{label}</CardDescription><CardTitle className="data-number text-xl">{value}</CardTitle></CardHeader></Card>)}
      </div>
      <Card><CardHeader className="flex flex-row items-center justify-between"><div><CardTitle>Completed purchases</CardTitle><CardDescription>Only fully received purchases appear here.</CardDescription></div>{loading ? <LoaderCircle className="size-5 animate-spin text-muted-foreground" /> : null}</CardHeader>
        <CardContent><Table><TableHeader><TableRow><TableHead>Supplier / invoice</TableHead><TableHead>Date</TableHead><TableHead>Items received</TableHead><TableHead className="text-right">Total</TableHead><TableHead className="text-right">Balance</TableHead><TableHead>Status</TableHead><TableHead /></TableRow></TableHeader><TableBody>
          {data?.items.map((purchase) => <TableRow key={purchase.id}><TableCell><p className="font-medium">{purchase.supplier_name}</p><p className="text-xs text-muted-foreground">{purchase.supplier_invoice_number}</p></TableCell><TableCell>{timestamp(purchase.created_at)}</TableCell><TableCell>{formatQuantity(purchase.items.reduce((sum, item) => sum + item.received_quantity, 0))}</TableCell><TableCell className="data-number text-right">{formatMoney(purchase.subtotal)}</TableCell><TableCell className="data-number text-right">{formatMoney(purchase.balance_due)}</TableCell><TableCell><Badge variant="secondary">Completed</Badge></TableCell><TableCell className="text-right"><Button variant="outline" size="sm" onClick={() => setSelected(purchase)}><Eye /> View</Button></TableCell></TableRow>)}
          {!loading && data?.items.length === 0 ? <TableRow><TableCell colSpan={7} className="h-24 text-center text-muted-foreground">No completed purchases match these filters.</TableCell></TableRow> : null}
        </TableBody></Table>
        <div className="mt-4 flex items-center justify-between border-t pt-4"><p className="text-sm text-muted-foreground">Page {page + 1} of {totalPages}</p><div className="flex gap-2"><Button variant="outline" size="sm" disabled={page === 0 || loading} onClick={() => setPage((value) => value - 1)}>Previous</Button><Button variant="outline" size="sm" disabled={page + 1 >= totalPages || loading} onClick={() => setPage((value) => value + 1)}>Next</Button></div></div>
        </CardContent>
      </Card>
      <Dialog open={selected !== null} onOpenChange={(open) => !open && setSelected(null)}><DialogContent size="wide">{selected ? <><DialogHeader><DialogTitle>{selected.supplier_name} · {selected.supplier_invoice_number}</DialogTitle><DialogDescription>Purchase recorded {timestamp(selected.created_at)} · Fully received and paid</DialogDescription></DialogHeader><div className="grid gap-6 lg:grid-cols-[minmax(0,1.5fr)_minmax(16rem,1fr)]"><div><h3 className="mb-2 text-sm font-semibold">Purchase lines</h3><Table><TableHeader><TableRow><TableHead>Product</TableHead><TableHead>Ordered</TableHead><TableHead>Received</TableHead><TableHead className="text-right">Unit cost</TableHead><TableHead className="text-right">Total</TableHead></TableRow></TableHeader><TableBody>{selected.items.map((item) => <TableRow key={item.id}><TableCell><p className="font-medium">{item.product_name}</p><p className="text-xs text-muted-foreground">{item.sku}</p></TableCell><TableCell>{formatQuantity(item.ordered_quantity)} {item.unit_name}</TableCell><TableCell>{formatQuantity(item.received_quantity)} {item.unit_name}</TableCell><TableCell className="data-number text-right">{formatMoney(item.unit_cost)}</TableCell><TableCell className="data-number text-right">{formatMoney(item.line_total)}</TableCell></TableRow>)}</TableBody></Table><h3 className="mt-6 mb-2 text-sm font-semibold">Deliveries</h3>{selected.deliveries.map((delivery) => <div key={delivery.id} className="border-b py-2 text-sm"><p className="font-medium">{delivery.delivery_reference || "Delivery"} · {timestamp(delivery.created_at)}</p><p className="text-muted-foreground">{formatMoney(delivery.total_cost)}</p></div>)}</div><div><h3 className="mb-2 text-sm font-semibold">Supplier payments</h3>{selected.payments.map((payment) => <div key={payment.id} className="flex justify-between border-b py-2 text-sm"><span>{payment.payment_method} · {timestamp(payment.created_at)}</span><span className="data-number">{formatMoney(payment.amount)}</span></div>)}<dl className="mt-4 space-y-2 border-t pt-3 text-sm"><div className="flex justify-between"><dt>Total</dt><dd>{formatMoney(selected.subtotal)}</dd></div><div className="flex justify-between"><dt>Paid</dt><dd>{formatMoney(selected.amount_paid)}</dd></div><div className="flex justify-between font-semibold"><dt>Balance due</dt><dd>{formatMoney(selected.balance_due)}</dd></div></dl></div></div></> : null}</DialogContent></Dialog>
    </div>
  );
}
