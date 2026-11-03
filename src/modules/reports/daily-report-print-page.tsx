import { invoke } from "@tauri-apps/api/core";
import { ArrowLeft, Download, LoaderCircle, Printer } from "lucide-react";
import { useEffect, useState } from "react";
import { Link, useSearchParams } from "react-router-dom";

import { Button } from "@/components/ui/button";

const MONEY_SCALE = 100n;

interface AmountBreakdown {
  category: string;
  amount: number;
}

interface CashOutflowBreakdown {
  category: string;
  description: string;
  amount: number;
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
  loan_received: number;
  loan_repayments: number;
  other_receipts: number;
  sale_count: number;
  sales_total: number;
  sales_by_payment_method: AmountBreakdown[];
  customer_repayments: number;
  expenses_total: number;
  expenses_by_category: AmountBreakdown[];
  supplier_payments: number;
  customer_outstanding: number;
  supplier_outstanding: number;
}

function formatMoney(value: number): string {
  if (!Number.isSafeInteger(value)) return "—";
  const amount = BigInt(value);
  const absolute = amount < 0n ? -amount : amount;
  const whole = absolute / MONEY_SCALE;
  const fraction = absolute % MONEY_SCALE;
  return `${amount < 0n ? "−" : ""}${new Intl.NumberFormat("en-RW").format(Number(whole))}${
    fraction === 0n ? "" : `.${fraction.toString().padStart(2, "0")}`
  } RWF`;
}

function label(value: string): string {
  return value.split("_").map((word) => word.charAt(0).toUpperCase() + word.slice(1)).join(" ");
}

function dateLabel(value: string): string {
  return new Date(`${value}T00:00:00`).toLocaleDateString("en-RW", {
    day: "numeric",
    month: "long",
    year: "numeric",
  });
}

function errorText(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

function PrintMoneyTable({ rows }: { rows: Array<[string, number]> }) {
  return (
    <table className="print-report-table">
      <tbody>
        {rows.map(([name, amount]) => (
          <tr key={name}><td>{name}</td><td>{formatMoney(amount)}</td></tr>
        ))}
      </tbody>
    </table>
  );
}

export function DailyReportPrintPage() {
  const [searchParams] = useSearchParams();
  const businessId = searchParams.get("businessId") ?? "";
  const reportDate = searchParams.get("date") ?? "";
  const [report, setReport] = useState<DailyBusinessReport | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!businessId || !reportDate) {
      setError("A business and report date are required.");
      return;
    }
    invoke<DailyBusinessReport>("daily_business_report", { businessId, reportDate })
      .then(setReport)
      .catch((cause) => setError(`Could not calculate the report. ${errorText(cause)}`));
  }, [businessId, reportDate]);

  if (error) {
    return <main className="print-report-error"><p>{error}</p><Link to="/reports">Return to Reports</Link></main>;
  }

  if (!report) {
    return <main className="print-report-loading"><LoaderCircle className="animate-spin" /> Preparing printable report...</main>;
  }

  const incomingRows: Array<[string, number]> = [
    ...report.sales_by_payment_method.map((item) => [label(item.category), item.amount] as [string, number]),
    ...(report.customer_repayments ? [["Customer repayments", report.customer_repayments] as [string, number]] : []),
    ...(report.loan_received ? [["Loans received", report.loan_received] as [string, number]] : []),
    ...(report.other_receipts ? [["Other receipts", report.other_receipts] as [string, number]] : []),
  ];

  return (
    <main className="print-report-page">
      <div className="print-report-actions">
        <Button variant="outline" asChild><Link to="/reports"><ArrowLeft />Back to Reports</Link></Button>
        <Button variant="outline" onClick={() => window.print()}><Printer />Print</Button>
        <Button onClick={() => window.print()}><Download />Download PDF</Button>
      </div>

      <article className="print-report-document">
        <header className="print-report-header">
          <div>
            <p className="print-report-kicker">Daily business report</p>
            <h1>{report.business_name}</h1>
            <p>Financial calculation and physical cash reconciliation</p>
          </div>
          <div className="print-report-meta">
            <span>Reporting date</span>
            <strong>{dateLabel(report.report_date)}</strong>
          </div>
        </header>

        <section className="print-report-summary">
          <div><span>Money received</span><strong>{formatMoney(report.money_received)}</strong><small>Actual incoming payments</small></div>
          <div><span>Recorded expenses</span><strong>{formatMoney(report.expenses_total)}</strong><small>Operating expenses only</small></div>
          <div className={report.net_position >= 0 ? "is-positive" : "is-negative"}><span>Net position</span><strong>{formatMoney(report.net_position)}</strong><small>Received less expenses</small></div>
          <div><span>Physical cash remaining</span><strong>{formatMoney(report.physical_closing_cash)}</strong><small>Verified cash drawer balance</small></div>
        </section>

        <section className="print-report-section">
          <h2>Executive explanation</h2>
          <p>
            On {dateLabel(report.report_date)}, the business received {formatMoney(report.money_received)} and recorded
            {" "}{formatMoney(report.expenses_total)} in operating expenses. The calculated net position was
            {" "}{formatMoney(report.net_position)}. This net position is a cash-based management measure and is not
            presented as accounting profit.
          </p>
        </section>

        <div className="print-report-columns">
          <section className="print-report-section">
            <h2>Money received</h2>
            <p className="print-report-note">Incoming sources counted once from recorded payments.</p>
            <PrintMoneyTable rows={incomingRows} />
            <p className="print-report-total"><span>Total received</span><strong>{formatMoney(report.money_received)}</strong></p>
          </section>
          <section className="print-report-section">
            <h2>Expenses by category</h2>
            <p className="print-report-note">Transfers and supplier payments are excluded from expenses.</p>
            <PrintMoneyTable rows={report.expenses_by_category.map((item) => [label(item.category), item.amount])} />
            <p className="print-report-total"><span>Total expenses</span><strong>{formatMoney(report.expenses_total)}</strong></p>
          </section>
        </div>

        <section className="print-report-section">
          <h2>Physical cash reconciliation</h2>
          <p className="print-report-note">This calculation covers the physical cash drawer only.</p>
          <PrintMoneyTable rows={[
            ["Opening physical cash", report.opening_physical_cash],
            ["Cash received", report.cash_received],
            ["Cash expenses paid", -report.cash_expenses],
            ["Bank", -report.cash_transferred_to_bank],
            ["MOMO", -report.cash_transferred_to_mobile_money],
            ...report.other_cash_outflows.map((outflow) => [
              `${outflow.category} — ${outflow.description}`,
              -outflow.amount,
            ] as [string, number]),
          ]} />
          <p className="print-report-total print-report-total-emphasis"><span>Physical closing cash</span><strong>{formatMoney(report.physical_closing_cash)}</strong></p>
        </section>

        <section className="print-report-columns">
          <section className="print-report-section">
            <h2>Business activity</h2>
            <PrintMoneyTable rows={[
              ["Sales value", report.sales_total],
              ["Supplier payments", report.supplier_payments],
            ]} />
            <p className="print-report-note">{report.sale_count.toLocaleString("en-RW")} completed sales recorded on this date.</p>
          </section>
          <section className="print-report-section">
            <h2>Balances and obligations</h2>
            <PrintMoneyTable rows={[
              ["Customer outstanding", report.customer_outstanding],
              ["Supplier outstanding", report.supplier_outstanding],
              ["Loans received", report.loan_received],
              ["Loan repayments", report.loan_repayments],
            ]} />
          </section>
        </section>

        <footer className="print-report-footer">
          <span>Generated from verified application records</span>
          <span>Daily report · {report.report_date}</span>
        </footer>
      </article>
    </main>
  );
}
