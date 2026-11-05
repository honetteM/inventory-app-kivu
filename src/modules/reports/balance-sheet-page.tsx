import { invoke } from "@tauri-apps/api/core";
import { LoaderCircle, Printer } from "lucide-react";
import { useEffect, useState } from "react";

import { PageHeader } from "@/components/shared/page-header";
import { SelectField } from "@/components/shared/select-field";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";

interface Business { id: string; name: string; }
interface BalanceSheetAccount {
  account_type: string;
  name: string;
  balance: number;
}
interface BalanceSheetReport {
  business_name: string;
  report_date: string;
  cash_accounts: BalanceSheetAccount[];
  cash_and_bank: number;
  inventory: number;
  accounts_receivable: number;
  total_assets: number;
  accounts_payable: number;
  loan_payable: number;
  total_liabilities: number;
  retained_earnings: number;
  owner_equity: number;
  total_equity: number;
  total_liabilities_and_equity: number;
  balance_difference: number;
  is_balanced: boolean;
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
const accountLabel = (value: string) =>
  value.split("_").join(" ").replace(/\b\w/g, (letter: string) => letter.toUpperCase());

export function BalanceSheetPage() {
  const [businesses, setBusinesses] = useState<Business[]>([]);
  const [businessId, setBusinessId] = useState("");
  const [fromDate, setFromDate] = useState(today);
  const [toDate, setToDate] = useState(today);
  const [openingReport, setOpeningReport] = useState<BalanceSheetReport | null>(null);
  const [report, setReport] = useState<BalanceSheetReport | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<Business[]>("business_list").then((items) => {
      setBusinesses(items);
      setBusinessId(items[0]?.id ?? "");
    }).catch((cause) => setError(`Could not load businesses. ${errorText(cause)}`));
  }, []);

  useEffect(() => {
    if (!businessId || !fromDate || !toDate || fromDate > toDate) return;
    setReport(null);
    setOpeningReport(null);
    setError(null);
    Promise.all([
      invoke<BalanceSheetReport>("balance_sheet_report", { businessId, reportDate: fromDate }),
      invoke<BalanceSheetReport>("balance_sheet_report", { businessId, reportDate: toDate }),
    ])
      .then(([opening, closing]) => {
        setOpeningReport(opening);
        setReport(closing);
      })
      .catch((cause) => setError(`Could not calculate the balance sheet. ${errorText(cause)}`));
  }, [businessId, fromDate, toDate]);

  return (
    <div className="daily-report space-y-6">
      <PageHeader
        eyebrow="Finance & reporting"
        title="Business balance sheet"
        description="Financial position as of the selected date."
        actions={<div className="flex flex-wrap gap-2 print:hidden">
          <SelectField value={businessId} onChange={(event) => setBusinessId(event.target.value)} aria-label="Business">
            {businesses.map((business) => <option key={business.id} value={business.id}>{business.name}</option>)}
          </SelectField>
          <Input type="date" value={fromDate} max={toDate || today()} onChange={(event) => setFromDate(event.target.value)} aria-label="From date" />
          <Input type="date" value={toDate} min={fromDate} max={today()} onChange={(event) => setToDate(event.target.value)} aria-label="To date" />
          <Button variant="outline" onClick={() => window.print()}><Printer />Print</Button>
        </div>}
      />
      {error ? <div role="alert" className="rounded-md border border-destructive/30 bg-destructive/5 p-4 text-sm text-destructive">{error}</div> : null}
      {fromDate > toDate ? <div role="alert" className="rounded-md border border-destructive/30 bg-destructive/5 p-4 text-sm text-destructive">From date must be before or equal to to date.</div> : null}
      {!report ? <Card><CardContent className="flex items-center gap-2 py-10 text-sm text-muted-foreground"><LoaderCircle className="size-4 animate-spin" />Calculating balance sheet...</CardContent></Card> : (
        <article className="report-paper space-y-6">
          <header className="border-b-2 border-primary pb-5">
            <p className="text-sm font-medium uppercase tracking-wide text-muted-foreground">Balance sheet</p>
            <h2 className="font-heading text-2xl font-semibold">{report.business_name}</h2>
            <p className="text-sm text-muted-foreground">Period: {fromDate} — {toDate}</p>
          </header>

          {openingReport ? (
            <Card>
              <CardHeader><CardTitle>Opening and closing position</CardTitle><CardDescription>Comparing the financial position at each selected date.</CardDescription></CardHeader>
              <CardContent>
                <table className="w-full text-sm"><tbody>
                  <tr className="border-b"><td className="py-2">Opening total assets ({openingReport.report_date})</td><td className="data-number py-2 text-right">{money(openingReport.total_assets)}</td></tr>
                  <tr className="border-b"><td className="py-2">Closing total assets ({report.report_date})</td><td className="data-number py-2 text-right">{money(report.total_assets)}</td></tr>
                  <tr className="bg-primary/10 font-bold"><td className="py-3">Net asset movement</td><td className="data-number py-3 text-right">{money(report.total_assets - openingReport.total_assets)}</td></tr>
                </tbody></table>
              </CardContent>
            </Card>
          ) : null}

          <div className="grid gap-6 xl:grid-cols-2">
            <Card>
              <CardHeader><CardTitle>Assets</CardTitle><CardDescription>Resources owned or controlled by the business.</CardDescription></CardHeader>
              <CardContent>
                <table className="w-full text-sm"><tbody>
                  {report.cash_accounts.map((account) => (
                    <tr className="border-b" key={`${account.account_type}-${account.name}`}>
                      <td className="py-2">{account.name} <span className="text-muted-foreground">({accountLabel(account.account_type)})</span></td>
                      <td className="data-number py-2 text-right">{money(account.balance)}</td>
                    </tr>
                  ))}
                  <tr className="border-b font-medium"><td className="py-2">Total cash and bank</td><td className="data-number py-2 text-right">{money(report.cash_and_bank)}</td></tr>
                  <tr className="border-b"><td className="py-2">Inventory</td><td className="data-number py-2 text-right">{money(report.inventory)}</td></tr>
                  <tr className="border-b"><td className="py-2">Accounts receivable / customer credit</td><td className="data-number py-2 text-right">{money(report.accounts_receivable)}</td></tr>
                  <tr className="bg-primary/10 text-base font-bold"><td className="py-3">Total assets</td><td className="data-number py-3 text-right">{money(report.total_assets)}</td></tr>
                </tbody></table>
              </CardContent>
            </Card>

            <Card>
              <CardHeader><CardTitle>Liabilities and equity</CardTitle><CardDescription>Obligations and owner claims against business assets.</CardDescription></CardHeader>
              <CardContent>
                <table className="w-full text-sm"><tbody>
                  <tr className="border-b"><td className="py-2">Accounts payable / supplier credit</td><td className="data-number py-2 text-right">{money(report.accounts_payable)}</td></tr>
                  <tr className="border-b"><td className="py-2">Loan payable</td><td className="data-number py-2 text-right">{money(report.loan_payable)}</td></tr>
                  <tr className="border-b font-medium"><td className="py-2">Total liabilities</td><td className="data-number py-2 text-right">{money(report.total_liabilities)}</td></tr>
                  <tr className="border-b"><td className="py-2">Retained earnings</td><td className="data-number py-2 text-right">{money(report.retained_earnings)}</td></tr>
                  <tr className="border-b"><td className="py-2">Owner equity / opening capital</td><td className="data-number py-2 text-right">{money(report.owner_equity)}</td></tr>
                  <tr className="border-b font-medium"><td className="py-2">Total equity</td><td className="data-number py-2 text-right">{money(report.total_equity)}</td></tr>
                  <tr className="bg-primary/10 text-base font-bold"><td className="py-3">Total liabilities and equity</td><td className="data-number py-3 text-right">{money(report.total_liabilities_and_equity)}</td></tr>
                </tbody></table>
              </CardContent>
            </Card>
          </div>

          <Card>
            <CardHeader><CardTitle>Balance check</CardTitle></CardHeader>
            <CardContent>
              <p className={`text-sm font-medium ${report.is_balanced ? "text-emerald-700" : "text-destructive"}`}>
                {report.is_balanced ? "The balance sheet is balanced." : `Balance difference: ${money(report.balance_difference)}`}
              </p>
              <p className="mt-2 text-xs text-muted-foreground">
                Owner equity is the balancing capital amount because the system does not maintain a separate owner-contribution ledger.
              </p>
            </CardContent>
          </Card>
        </article>
      )}
    </div>
  );
}
