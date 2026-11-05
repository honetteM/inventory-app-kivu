import { invoke } from "@tauri-apps/api/core";
import { LoaderCircle, Printer } from "lucide-react";
import { useEffect, useState } from "react";

import { PageHeader } from "@/components/shared/page-header";
import { SelectField } from "@/components/shared/select-field";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";

interface Business { id: string; name: string; }
interface Report {
  business_name: string;
  sales_total: number;
  cost_of_goods_sold: number;
  gross_profit: number;
  expenses_total: number;
  operating_result: number;
  sale_count: number;
  expenses_by_category: { category: string; amount: number }[];
}

const money = (value: number) => {
  if (!Number.isSafeInteger(value)) return "NOT AVAILABLE";
  const amount = BigInt(value);
  const absolute = amount < 0n ? -amount : amount;
  return `${amount < 0n ? "−" : ""}${new Intl.NumberFormat("en-RW").format(Number(absolute / 100n))}${absolute % 100n ? `.${(absolute % 100n).toString().padStart(2, "0")}` : ""} RWF`;
};
const today = () => {
  const date = new Date();
  return new Date(date.getTime() - date.getTimezoneOffset() * 60000).toISOString().slice(0, 10);
};
const errorText = (error: unknown) => error instanceof Error ? error.message : String(error);

export function IncomeStatementPage() {
  const [businesses, setBusinesses] = useState<Business[]>([]);
  const [businessId, setBusinessId] = useState("");
  const [date, setDate] = useState(today);
  const [report, setReport] = useState<Report | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<Business[]>("business_list").then((items) => {
      setBusinesses(items);
      setBusinessId(items[0]?.id ?? "");
    }).catch((cause) => setError(`Could not load businesses. ${errorText(cause)}`));
  }, []);

  useEffect(() => {
    if (!businessId || !date) return;
    setReport(null);
    invoke<Report>("daily_business_report", { businessId, reportDate: date })
      .then(setReport)
      .catch((cause) => setError(`Could not calculate the income statement. ${errorText(cause)}`));
  }, [businessId, date]);

  return (
    <div className="daily-report space-y-6">
      <PageHeader eyebrow="Finance & reporting" title="Business income statement"
        description="Revenue and profitability from KIVU SUBLIME product sales only."
        actions={<div className="flex flex-wrap gap-2 print:hidden">
          <SelectField value={businessId} onChange={(event) => setBusinessId(event.target.value)} aria-label="Business">
            {businesses.map((business) => <option key={business.id} value={business.id}>{business.name}</option>)}
          </SelectField>
          <Input type="date" value={date} max={today()} onChange={(event) => setDate(event.target.value)} aria-label="Report date" />
          <Button variant="outline" onClick={() => window.print()}><Printer />Print</Button>
        </div>}
      />
      {error ? <div role="alert" className="rounded-md border border-destructive/30 bg-destructive/5 p-4 text-sm text-destructive">{error}</div> : null}
      {!report ? <Card><CardContent className="flex items-center gap-2 py-10 text-sm text-muted-foreground"><LoaderCircle className="size-4 animate-spin" />Calculating income statement...</CardContent></Card> : (
        <article className="report-paper space-y-6">
          <header className="border-b-2 border-primary pb-5">
            <p className="text-sm font-medium uppercase tracking-wide text-muted-foreground">Business income statement</p>
            <h2 className="font-heading text-2xl font-semibold">{report.business_name}</h2>
            <p className="text-sm text-muted-foreground">{date}</p>
          </header>
          <Card>
            <CardHeader><CardTitle>Income statement</CardTitle><CardDescription>Only completed product sales and operating expenses are included. Loans, transfers, repayments, and supplier payments are excluded.</CardDescription></CardHeader>
            <CardContent><table className="w-full text-sm"><tbody>
              <tr className="border-b"><td className="py-3">Sales revenue ({report.sale_count} completed sales)</td><td className="data-number py-3 text-right">{money(report.sales_total)}</td></tr>
              <tr className="border-b"><td className="py-3">Cost of goods sold</td><td className="data-number py-3 text-right">{money(report.cost_of_goods_sold)}</td></tr>
              <tr className="border-b bg-muted/40 font-semibold"><td className="py-3">Gross profit</td><td className="data-number py-3 text-right">{money(report.gross_profit)}</td></tr>
              <tr className="border-b"><td className="py-3">Operating expenses</td><td className="data-number py-3 text-right">{money(report.expenses_total)}</td></tr>
              <tr className="bg-primary/10 text-base font-bold"><td className="py-3">Net operating profit / loss</td><td className="data-number py-3 text-right">{money(report.operating_result)}</td></tr>
            </tbody></table></CardContent>
          </Card>
          <Card><CardHeader><CardTitle>Operating expenses by category</CardTitle></CardHeader><CardContent>
            {report.expenses_by_category.length ? <table className="w-full text-sm"><tbody>{report.expenses_by_category.map((item) => <tr className="border-b last:border-0" key={item.category}><td className="py-2">{item.category}</td><td className="data-number py-2 text-right">{money(item.amount)}</td></tr>)}</tbody></table> : <p className="text-sm text-muted-foreground">No operating expenses recorded.</p>}
          </CardContent></Card>
        </article>
      )}
    </div>
  );
}
