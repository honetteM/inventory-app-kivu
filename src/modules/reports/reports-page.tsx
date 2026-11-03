import { invoke } from "@tauri-apps/api/core";
import { BarChart3, Download, LoaderCircle, Printer } from "lucide-react";
import { useEffect, useState } from "react";

import { PageHeader } from "@/components/shared/page-header";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { SelectField } from "@/components/shared/select-field";

const SCALE = 100n;
type ReportType = "daily" | "weekly" | "monthly" | "yearly";
type Business = { id: string; name: string };
type Breakdown = { category: string; amount: number };
type ReportDay = { date: string; money_received: number; expenses: number; net_position: number };
type BusinessReport = {
  business_name: string;
  currency: string;
  report_type: ReportType;
  start_date: string;
  end_date: string;
  money_received: number;
  expenses_total: number;
  net_position: number;
  sales_received: number;
  customer_repayments: number;
  expenses_by_category: Breakdown[];
  received_by_method: Breakdown[];
  cash_received: number;
  cash_expenses: number;
  cash_transferred_out: number;
  cash_other_out: number;
  opening_physical_cash: number | null;
  closing_physical_cash: number | null;
  customer_outstanding: number;
  supplier_outstanding: number;
  daily_breakdown: ReportDay[];
};

function localDate() {
  const date = new Date();
  date.setMinutes(date.getMinutes() - date.getTimezoneOffset());
  return date.toISOString().slice(0, 10);
}

function money(value: number, currency = "RWF") {
  if (!Number.isSafeInteger(value)) return "—";
  const amount = BigInt(value);
  const absolute = amount < 0n ? -amount : amount;
  const whole = absolute / SCALE;
  const fraction = absolute % SCALE;
  return `${amount < 0n ? "−" : ""}${new Intl.NumberFormat("en-RW").format(Number(whole))}${
    fraction ? `.${fraction.toString().padStart(2, "0")}` : ""
  } ${currency}`;
}

function label(value: string) {
  return value.split("_").join(" ").replace(/\b\w/g, (letter: string) => letter.toUpperCase());
}

function errorText(cause: unknown) {
  return cause instanceof Error ? cause.message : String(cause);
}

function SummaryCard({ title, value, detail }: { title: string; value: string; detail: string }) {
  return (
    <Card>
      <CardHeader className="pb-2"><CardDescription>{title}</CardDescription></CardHeader>
      <CardContent><p className="data-number text-xl font-semibold">{value}</p><p className="mt-1 text-xs text-muted-foreground">{detail}</p></CardContent>
    </Card>
  );
}

export function ReportsPage() {
  const [businesses, setBusinesses] = useState<Business[]>([]);
  const [businessId, setBusinessId] = useState("");
  const [reportType, setReportType] = useState<ReportType>("daily");
  const [anchorDate, setAnchorDate] = useState(localDate);
  const [report, setReport] = useState<BusinessReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    invoke<Business[]>("business_list")
      .then((items) => {
        setBusinesses(items);
        setBusinessId(items[0]?.id ?? "");
      })
      .catch((cause) => setError(`Could not load businesses. ${errorText(cause)}`));
  }, []);

  useEffect(() => {
    if (!businessId || !anchorDate) return;
    setLoading(true);
    setError(null);
    invoke<BusinessReport>("business_report", { businessId, reportType, anchorDate })
      .then(setReport)
      .catch((cause) => {
        setReport(null);
        setError(`Could not load the report. ${errorText(cause)}`);
      })
      .finally(() => setLoading(false));
  }, [businessId, reportType, anchorDate]);

  const period = report ? `${report.start_date} to ${report.end_date}` : "Select a period";
  const printReport = () => window.print();

  return (
    <div className="report-page space-y-6">
      <PageHeader
        eyebrow="Finance & reporting"
        title="Business reports"
        description="Understand money received, recorded expenses, and verified cash activity without confusing cash position with profit."
        actions={
          <div className="flex flex-wrap gap-2">
            <SelectField value={businessId} onChange={(event) => setBusinessId(event.target.value)} aria-label="Business">
              {businesses.map((business) => <option key={business.id} value={business.id}>{business.name}</option>)}
            </SelectField>
            <SelectField value={reportType} onChange={(event) => setReportType(event.target.value as ReportType)} aria-label="Report type">
              <option value="daily">Daily</option><option value="weekly">Weekly</option><option value="monthly">Monthly</option><option value="yearly">Yearly</option>
            </SelectField>
            <Input type="date" value={anchorDate} max={localDate()} onChange={(event) => setAnchorDate(event.target.value)} aria-label="Report date" />
          </div>
        }
      />
      {error ? <div role="alert" className="rounded-md border border-destructive/30 bg-destructive/5 p-4 text-sm text-destructive">{error}</div> : null}
      {loading ? <Card><CardContent className="flex items-center gap-2 py-10 text-sm text-muted-foreground"><LoaderCircle className="size-4 animate-spin" />Preparing report...</CardContent></Card> : null}
      {report ? (
        <article className="report-document space-y-6">
          <header className="flex flex-wrap items-start justify-between gap-4 border-b pb-5">
            <div><p className="text-sm font-medium text-muted-foreground">{label(report.report_type)} business report</p><h2 className="font-heading text-2xl font-semibold">{report.business_name}</h2><p className="text-sm text-muted-foreground">{period}</p></div>
            <div className="flex gap-2 print:hidden"><Button variant="outline" onClick={printReport}><Printer />Print</Button><Button onClick={printReport}><Download />Export PDF</Button></div>
          </header>
          <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-5">
            <SummaryCard title="Money received" value={money(report.money_received, report.currency)} detail="Actual recorded incoming payments" />
            <SummaryCard title="Recorded expenses" value={money(report.expenses_total, report.currency)} detail="Operating expenses only" />
            <SummaryCard title="Net position" value={money(report.net_position, report.currency)} detail="Money received less expenses" />
            <SummaryCard title="Sales received" value={money(report.sales_received, report.currency)} detail="Payments against completed sales" />
            <SummaryCard title="Customer debt" value={money(report.customer_outstanding, report.currency)} detail="Outstanding at report generation" />
          </div>
          {report.report_type === "daily" ? (
            <section className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
              <SummaryCard title="Opening physical cash" value={money(report.opening_physical_cash ?? 0, report.currency)} detail="Cash carried from the previous day" />
              <SummaryCard title="Cash received" value={money(report.cash_received, report.currency)} detail="Verified cash drawer inflows" />
              <SummaryCard title="Cash expenses" value={money(report.cash_expenses, report.currency)} detail="Expenses paid from cash" />
              <SummaryCard title="Physical cash remaining" value={money(report.closing_physical_cash ?? 0, report.currency)} detail={`After transfers and other cash outflows (${money(report.cash_other_out, report.currency)})`} />
            </section>
          ) : null}
          <div className="grid gap-6 xl:grid-cols-2">
            <Card><CardHeader><CardTitle>Where money came from</CardTitle><CardDescription>Incoming verified cash movements by recorded source.</CardDescription></CardHeader><CardContent className="space-y-3">{report.received_by_method.length ? report.received_by_method.map((item) => <div className="flex justify-between border-b pb-2 text-sm" key={item.category}><span>{label(item.category)}</span><strong>{money(item.amount, report.currency)}</strong></div>) : <p className="text-sm text-muted-foreground">No money was received in this period.</p>}</CardContent></Card>
            <Card><CardHeader><CardTitle>Where money went</CardTitle><CardDescription>Recorded operating expenses. Transfers and supplier payments are excluded.</CardDescription></CardHeader><CardContent className="space-y-3">{report.expenses_by_category.length ? report.expenses_by_category.map((item) => <div className="flex justify-between border-b pb-2 text-sm" key={item.category}><span>{label(item.category)}</span><strong>{money(item.amount, report.currency)}</strong></div>) : <p className="text-sm text-muted-foreground">No operating expenses were recorded.</p>}</CardContent></Card>
          </div>
          <Card><CardHeader><CardTitle>{report.report_type === "daily" ? "Daily activity" : "Performance breakdown"}</CardTitle><CardDescription>Net position is not accounting profit; it is actual money received less recorded expenses.</CardDescription></CardHeader><CardContent><div className="overflow-x-auto"><table className="w-full text-sm"><thead><tr className="border-b text-left"><th className="p-2">Date</th><th className="p-2 text-right">Money received</th><th className="p-2 text-right">Expenses</th><th className="p-2 text-right">Net position</th></tr></thead><tbody>{report.daily_breakdown.map((row) => <tr className="border-b" key={row.date}><td className="p-2">{row.date}</td><td className="data-number p-2 text-right">{money(row.money_received, report.currency)}</td><td className="data-number p-2 text-right">{money(row.expenses, report.currency)}</td><td className={`data-number p-2 text-right font-medium ${row.net_position < 0 ? "text-destructive" : "text-emerald-700"}`}>{money(row.net_position, report.currency)}</td></tr>)}</tbody></table></div></CardContent></Card>
          <Card><CardHeader><CardTitle>Financial obligations</CardTitle><CardDescription>These balances are separate from available cash and net position.</CardDescription></CardHeader><CardContent className="grid gap-4 sm:grid-cols-2"><div><p className="text-xs text-muted-foreground">Customer outstanding</p><p className="data-number text-lg font-semibold">{money(report.customer_outstanding, report.currency)}</p></div><div><p className="text-xs text-muted-foreground">Supplier outstanding</p><p className="data-number text-lg font-semibold">{money(report.supplier_outstanding, report.currency)}</p></div></CardContent></Card>
          <p className="text-sm text-muted-foreground">Management insight: {report.business_name} received {money(report.money_received, report.currency)} and recorded {money(report.expenses_total, report.currency)} in expenses during this period. The resulting net cash position was {money(report.net_position, report.currency)}. Loan, prepaid-cash, and dedicated deposit records are not available in the current database schema.</p>
        </article>
      ) : null}
      {!loading && !report && !error ? <Card><CardContent className="py-10 text-center text-sm text-muted-foreground"><BarChart3 className="mx-auto mb-2 size-8" />Choose a period to prepare a report.</CardContent></Card> : null}
    </div>
  );
}
