import { invoke } from "@tauri-apps/api/core";
import { ArrowLeft, LoaderCircle, Printer } from "lucide-react";
import { useEffect, useState } from "react";
import { Link, useSearchParams } from "react-router-dom";

import { Button } from "@/components/ui/button";

const MONEY_SCALE = 100n;

interface CashOutflowBreakdown {
  category: string;
  description: string;
  amount: number;
}

interface DailyBusinessReport {
  business_name: string;
  report_date: string;
  net_position: number;
  opening_physical_cash: number;
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
  cash_loan_paid: number;
  cash_loan_received: number;
  cash_customer_repayment: number;
  bank_loan_paid: number;
  momo_loan_paid: number;
  cash_supplier_payments: number;
  bank_supplier_payments: number;
  momo_supplier_payments: number;
  calculated_physical_cash: number;
  cash_reconciliation_difference: number;
  customer_outstanding: number;
}

interface LedgerRow {
  description: string;
  amount: number;
  destination: string;
  emphasis?: boolean;
  section?: boolean;
}

function formatMoney(value: number): string {
  if (!Number.isSafeInteger(value)) return "NOT AVAILABLE";
  const amount = BigInt(value);
  const absolute = amount < 0n ? -amount : amount;
  const whole = absolute / MONEY_SCALE;
  const fraction = absolute % MONEY_SCALE;
  return `${amount < 0n ? "−" : ""}${new Intl.NumberFormat("en-RW").format(Number(whole))}${
    fraction === 0n ? "" : `.${fraction.toString().padStart(2, "0")}`
  } RWF`;
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

function LedgerTable({ rows }: { rows: LedgerRow[] }) {
  return (
    <table className="cashbook-table">
      <thead>
        <tr><th>Description</th><th>Amount</th><th>Balance / destination</th></tr>
      </thead>
      <tbody>
        {rows.map((row, index) => (
          <tr className={`${row.emphasis ? "is-emphasis " : ""}${row.section ? "is-section" : ""}`} key={`${row.description}-${index}`}>
            <td>{row.description}</td>
            <td>{formatMoney(row.amount)}</td>
            <td>{row.destination}</td>
          </tr>
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

  useEffect(() => {
    document.title = "KIVU_SUBLIME_Daily_Cashbook";
    return () => {
      document.title = "Quincaillerie Manager";
    };
  }, []);

  if (error) {
    return <main className="print-report-error"><p>{error}</p><Link to="/reports">Return to Reports</Link></main>;
  }
  if (!report) {
    return <main className="print-report-loading"><LoaderCircle className="animate-spin" /> Preparing printable report...</main>;
  }

  const rows: LedgerRow[] = [
    { description: "STARTING BALANCE", amount: report.opening_physical_cash, destination: "Physical cash", emphasis: true },
    { description: "TOTAL INFLOW", amount: report.total_inflow, destination: "Completed sales", emphasis: true },
    { description: "TOTAL OUTFLOW", amount: -report.total_outflow, destination: "Operating expenses", emphasis: true },
    { description: "NET POSITION", amount: report.net_position, destination: "Inflow − outflow", emphasis: true },
    { description: "ENDING CASH", amount: report.ending_cash, destination: "Starting balance + net position", emphasis: true },
    { description: "LOAN RECEIVED — CASH", amount: report.cash_loan_received, destination: "Physical cash" },
    { description: "LOAN PAID — CASH", amount: report.cash_loan_paid, destination: "Physical cash" },
    { description: "LOAN PAID — BANK", amount: report.bank_loan_paid, destination: "Bank" },
    { description: "LOAN PAID — MOMO", amount: report.momo_loan_paid, destination: "MOMO" },
    { description: "CUSTOMER REPAYMENTS — CASH", amount: report.cash_customer_repayment, destination: "Physical cash" },
    { description: "VERSEMENT", amount: 0, destination: "Confirmed transfers from physical cash", section: true },
    { description: "BANK", amount: -report.cash_transferred_to_bank, destination: "Cash → BANK" },
    { description: "MOMO", amount: -report.cash_transferred_to_mobile_money, destination: "Cash → MOMO" },
    { description: "SUPPLIER PAYMENTS", amount: 0, destination: "Actual payments during period", section: true },
    { description: "CASH", amount: -report.cash_supplier_payments, destination: "Physical cash" },
    { description: "BANK", amount: -report.bank_supplier_payments, destination: "Bank" },
    { description: "MOMO", amount: -report.momo_supplier_payments, destination: "MOMO" },
    ...report.other_cash_outflows.map((outflow) => ({
      description: `OTHER CASH OUTFLOW — ${outflow.description}`,
      amount: -outflow.amount,
      destination: outflow.category,
    })),
    { description: "CLOSING BALANCE", amount: report.physical_closing_cash, destination: "Actual physical cash ledger", emphasis: true },
    { description: "RECONCILIATION DIFFERENCE", amount: report.cash_reconciliation_difference, destination: "Expected physical cash − actual ledger", emphasis: true },
  ];

  return (
    <main className="print-report-page">
      <div className="print-report-actions">
        <Button variant="outline" asChild><Link to="/reports"><ArrowLeft />Back to Reports</Link></Button>
        <Button variant="outline" onClick={() => window.print()}><Printer />Print</Button>
      </div>
      <article className="print-report-document">
        <header className="cashbook-header">
          <div>
            <p className="print-report-kicker">Daily cashbook reconciliation</p>
            <h1>{report.business_name}</h1>
            <p>Combined financial statement · RWF</p>
          </div>
          <div className="print-report-meta">
            <span>Reporting date</span>
            <strong>{dateLabel(report.report_date)}</strong>
          </div>
        </header>
        <section className="cashbook-ledger">
          <LedgerTable rows={rows} />
        </section>
        <section className={`cashbook-status ${report.cash_reconciliation_difference === 0 ? "is-balanced" : "is-unbalanced"}`}>
          <strong>{report.cash_reconciliation_difference === 0 ? "RECONCILIATION BALANCED" : "RECONCILIATION DIFFERENCE IDENTIFIED"}</strong>
          <p>
            Expected physical cash: {formatMoney(report.calculated_physical_cash)} · Actual ledger closing balance: {formatMoney(report.physical_closing_cash)}.
            Customer debt closing balance: {formatMoney(report.customer_outstanding)}.
          </p>
          <p>Bank and MOMO customer receipts, repayments, expenses, supplier payments, and balances remain account-specific and are not deducted from physical cash unless paid from the cash drawer.</p>
        </section>
        <footer className="print-report-footer">
          <span>KIVU SUBLIME · {report.report_date}</span>
        </footer>
      </article>
    </main>
  );
}
