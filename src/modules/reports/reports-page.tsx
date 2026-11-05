import { invoke } from "@tauri-apps/api/core";
import { LoaderCircle, Printer, WalletCards } from "lucide-react";
import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";

import { PageHeader } from "@/components/shared/page-header";
import { SelectField } from "@/components/shared/select-field";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";

const MONEY_SCALE = 100n;

interface Business {
  id: string;
  name: string;
}

interface AmountBreakdown {
  category: string;
  amount: number;
}

interface CashOutflowBreakdown {
  category: string;
  description: string;
  amount: number;
}

interface CountBreakdown {
  movement_type: string;
  count: number;
}

interface DailyBusinessReport {
  business_name: string;
  report_date: string;
  money_received: number;
  net_position: number;
  opening_physical_cash: number;
  cash_received: number;
  cash_expenses: number;
  cash_transferred_to_bank: number;
  cash_transferred_to_mobile_money: number;
  other_cash_outflows: CashOutflowBreakdown[];
  physical_closing_cash: number;
  total_inflow: number;
  total_outflow: number;
  ending_cash: number;
  loan_taken: number;
  loan_paid: number;
  bank: number;
  momo: number;
  opening_customer_outstanding: number;
  closing_customer_outstanding: number;
  calculated_physical_cash: number;
  cash_reconciliation_difference: number;
  loan_received: number;
  loan_repayments: number;
  other_receipts: number;
  sale_count: number;
  sales_total: number;
  sales_by_payment_method: AmountBreakdown[];
  credit_sales: number;
  customer_repayments: number;
  expenses_total: number;
  expenses_by_category: AmountBreakdown[];
  supplier_payments: number;
  purchases_received: number;
  cost_of_goods_sold: number;
  gross_profit: number;
  operating_result: number;
  sale_line_count: number;
  customer_outstanding: number;
  supplier_outstanding: number;
  stock_adjustment_counts: CountBreakdown[];
}

function localDate(): string {
  const date = new Date();
  const offset = date.getTimezoneOffset() * 60_000;
  return new Date(date.getTime() - offset).toISOString().slice(0, 10);
}

function formatMoney(value: number): string {
  if (!Number.isSafeInteger(value)) return "—";
  const amount = BigInt(value);
  const absoluteAmount = amount < 0n ? -amount : amount;
  const whole = absoluteAmount / MONEY_SCALE;
  const fraction = absoluteAmount % MONEY_SCALE;
  return `${amount < 0n ? "−" : ""}${new Intl.NumberFormat("en-RW").format(Number(whole))}${
    fraction === 0n ? "" : `.${fraction.toString().padStart(2, "0")}`
  } RWF`;
}

function label(value: string): string {
  return value.split("_").map((word) => word.charAt(0).toUpperCase() + word.slice(1)).join(" ");
}

function errorText(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

function SummaryCard({ title, value, detail, positive }: {
  title: string;
  value: string;
  detail: string;
  positive?: boolean;
}) {
  return (
    <Card>
      <CardHeader className="pb-2"><CardDescription>{title}</CardDescription></CardHeader>
      <CardContent>
        <p className={`data-number text-xl font-semibold ${positive === true ? "text-emerald-700" : positive === false ? "text-destructive" : ""}`}>{value}</p>
        <p className="mt-1 text-xs text-muted-foreground">{detail}</p>
      </CardContent>
    </Card>
  );
}

function BreakdownTable({ title, description, items }: {
  title: string;
  description: string;
  items: AmountBreakdown[];
}) {
  return (
    <Card>
      <CardHeader><CardTitle>{title}</CardTitle><CardDescription>{description}</CardDescription></CardHeader>
      <CardContent>
        {items.length ? (
          <table className="w-full text-sm">
            <tbody>{items.map((item) => <tr className="border-b last:border-0" key={item.category}><td className="py-2">{label(item.category)}</td><td className="data-number py-2 text-right font-medium">{formatMoney(item.amount)}</td></tr>)}</tbody>
          </table>
        ) : <p className="text-sm text-muted-foreground">No records were posted for this date.</p>}
      </CardContent>
    </Card>
  );
}

export function ReportsPage() {
  const navigate = useNavigate();
  const [businesses, setBusinesses] = useState<Business[]>([]);
  const [businessId, setBusinessId] = useState("");
  const [fromDate, setFromDate] = useState(localDate);
  const [toDate, setToDate] = useState(localDate);
  const [report, setReport] = useState<DailyBusinessReport | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<Business[]>("business_list")
      .then((result) => {
        setBusinesses(result);
        setBusinessId(result[0]?.id ?? "");
      })
      .catch((cause) => setError(`Could not load businesses. ${errorText(cause)}`))
      .finally(() => setIsLoading(false));
  }, []);

  useEffect(() => {
    if (!businessId || !fromDate || !toDate || fromDate > toDate) return;
    setIsLoading(true);
    setError(null);
    invoke<DailyBusinessReport>("daily_business_report_range", { businessId, fromDate, toDate })
      .then(setReport)
      .catch((cause) => {
        setReport(null);
        setError(`Could not calculate the daily report. ${errorText(cause)}`);
      })
      .finally(() => setIsLoading(false));
  }, [businessId, fromDate, toDate]);

  const dateLabel = fromDate && toDate && fromDate <= toDate
    ? `${new Date(`${fromDate}T00:00:00`).toLocaleDateString("en-RW", { day: "numeric", month: "long", year: "numeric" })} — ${new Date(`${toDate}T00:00:00`).toLocaleDateString("en-RW", { day: "numeric", month: "long", year: "numeric" })}`
    : "Select a date";

  return (
    <div className="daily-report space-y-6">
      <PageHeader
        eyebrow="Finance & reporting"
        title="Financial reconciliation report"
        description="Cash, Bank, MOMO, credit, supplier payments, transfers, and physical cash reconciliation."
        actions={
          <div className="flex flex-wrap gap-2">
            <SelectField value={businessId} onChange={(event) => setBusinessId(event.target.value)} aria-label="Business">
              {businesses.map((business) => <option key={business.id} value={business.id}>{business.name}</option>)}
            </SelectField>
            <Input type="date" value={fromDate} max={toDate || localDate()} onChange={(event) => setFromDate(event.target.value)} aria-label="From date" />
            <Input type="date" value={toDate} min={fromDate} max={localDate()} onChange={(event) => setToDate(event.target.value)} aria-label="To date" />
          </div>
        }
      />
      {error ? <div role="alert" className="rounded-md border border-destructive/30 bg-destructive/5 p-4 text-sm text-destructive">{error}</div> : null}
      {fromDate > toDate ? <div role="alert" className="rounded-md border border-destructive/30 bg-destructive/5 p-4 text-sm text-destructive">From date must be before or equal to to date.</div> : null}
      {isLoading ? <Card><CardContent className="flex items-center gap-2 py-10 text-sm text-muted-foreground"><LoaderCircle className="size-4 animate-spin" />Calculating report range...</CardContent></Card> : null}
      {report ? (
        <article className="report-paper space-y-6">
          <header className="flex flex-wrap items-start justify-between gap-4 border-b-2 border-primary pb-5">
            <div>
              <p className="text-sm font-medium uppercase tracking-wide text-muted-foreground">Daily business calculation</p>
              <h2 className="font-heading text-2xl font-semibold">{report.business_name}</h2>
              <p className="text-sm text-muted-foreground">{dateLabel}</p>
            </div>
            <div className="flex gap-2 print:hidden">
              <Button variant="outline" disabled={!fromDate || !toDate || fromDate > toDate} onClick={() => navigate(`/reports/daily/print?businessId=${encodeURIComponent(businessId)}&fromDate=${encodeURIComponent(fromDate)}&toDate=${encodeURIComponent(toDate)}`)}><Printer />Print view</Button>
            </div>
          </header>

          <section className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4" aria-label="Daily calculation summary">
            <SummaryCard title="Total inflow" value={formatMoney(report.total_inflow)} detail="All completed sales, including credit sales" />
            <SummaryCard title="Total outflow" value={formatMoney(report.total_outflow)} detail="All recorded operating expenses" />
            <SummaryCard title="Net position" value={formatMoney(report.net_position)} detail="Money received minus expenses" positive={report.net_position >= 0} />
            <SummaryCard title="Physical cash remaining" value={formatMoney(report.physical_closing_cash)} detail="Verified cash drawer balance" />
          </section>

          <section className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
            <SummaryCard title="Opening physical cash" value={formatMoney(report.opening_physical_cash)} detail="Carried from the previous day" />
          </section>

          <section className="grid gap-6 xl:grid-cols-2">
            <BreakdownTable title="Money received" description="Each incoming source is counted once from the verified cash ledger." items={[
              ...report.sales_by_payment_method,
              ...(report.customer_repayments ? [{ category: "customer repayments", amount: report.customer_repayments }] : []),
              ...(report.loan_received ? [{ category: "loan received", amount: report.loan_received }] : []),
              ...(report.other_receipts ? [{ category: "other receipts", amount: report.other_receipts }] : []),
            ]} />
            <BreakdownTable title="Expenses by category" description={`Total recorded operating expenses: ${formatMoney(report.expenses_total)}`} items={report.expenses_by_category} />
          </section>

          <Card>
            <CardHeader><CardTitle>Physical cash reconciliation</CardTitle><CardDescription>This is cash in the physical drawer, not total business assets and not profit.</CardDescription></CardHeader>
            <CardContent>
              <table className="w-full text-sm"><tbody>
                <tr className="border-b"><td className="py-2">Starting balance</td><td className="data-number py-2 text-right">{formatMoney(report.opening_physical_cash)}</td></tr>
                <tr className="border-b"><td className="py-2">Total inflow</td><td className="data-number py-2 text-right">{formatMoney(report.total_inflow)}</td></tr>
                <tr className="border-b"><td className="py-2">Total outflow</td><td className="data-number py-2 text-right">{formatMoney(report.total_outflow)}</td></tr>
                <tr className="border-b font-semibold"><td className="py-2">Net position</td><td className="data-number py-2 text-right">{formatMoney(report.net_position)}</td></tr>
                <tr className="border-b font-semibold"><td className="py-2">Ending cash</td><td className="data-number py-2 text-right">{formatMoney(report.ending_cash)}</td></tr>
                <tr className="border-b"><td className="py-2">Loan taken</td><td className="data-number py-2 text-right">{formatMoney(report.loan_taken)}</td></tr>
                <tr className="border-b"><td className="py-2">Loan paid</td><td className="data-number py-2 text-right">{formatMoney(report.loan_paid)}</td></tr>
                <tr className="border-b"><td className="py-2">Bank</td><td className="data-number py-2 text-right">{formatMoney(report.bank)}</td></tr>
                <tr><td className="py-2">MOMO</td><td className="data-number py-2 text-right">{formatMoney(report.momo)}</td></tr>
                <tr className="border-t"><td className="py-2">Supplier payments</td><td className="data-number py-2 text-right">{formatMoney(report.supplier_payments)}</td></tr>
                {report.other_cash_outflows.map((outflow, index) => <tr key={`bridge-${outflow.category}-${outflow.description}-${index}`}><td className="py-2">{outflow.category} — {outflow.description}</td><td className="data-number py-2 text-right">{formatMoney(outflow.amount)}</td></tr>)}
                <tr className="border-t"><td className="py-2">Calculated physical cash</td><td className="data-number py-2 text-right">{formatMoney(report.calculated_physical_cash)}</td></tr>
                <tr className="border-t font-semibold"><td className="pt-3">Physical cash remaining</td><td className="data-number pt-3 text-right">{formatMoney(report.physical_closing_cash)}</td></tr>
                <tr><td className="py-2">Reconciliation difference</td><td className="data-number py-2 text-right">{formatMoney(report.cash_reconciliation_difference)}</td></tr>
              </tbody></table>
            </CardContent>
          </Card>

          <section className="grid gap-6 xl:grid-cols-3">
            <SummaryCard title="Sales value" value={formatMoney(report.sales_total)} detail={`${report.sale_count} completed sales; this is not all cash received`} />
            <SummaryCard title="Supplier payments" value={formatMoney(report.supplier_payments)} detail="Separate from operating expenses" />
            <SummaryCard title="Credit sales" value={formatMoney(report.credit_sales)} detail="Customer debt created by completed sales" />
            <SummaryCard title="Customer debt at close" value={formatMoney(report.closing_customer_outstanding)} detail="Outstanding customer balance" />
          </section>

          <p className="flex items-start gap-2 rounded-lg border bg-muted/30 p-4 text-sm leading-6 text-muted-foreground"><WalletCards className="mt-0.5 size-4 shrink-0" />Net position is a cash-based management measure, not accounting profit. Loans and transfers are shown separately and are not treated as operating expenses.</p>
        </article>
      ) : null}
      {!isLoading && !report && !error ? <Card><CardContent className="py-10 text-center text-sm text-muted-foreground">Select a business and date to calculate the daily report.</CardContent></Card> : null}
    </div>
  );
}
