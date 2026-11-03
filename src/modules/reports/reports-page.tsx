import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { BarChart3, CircleDollarSign, LoaderCircle, Package, ReceiptText, ShoppingBag, Wallet } from "lucide-react";

import { PageHeader } from "@/components/shared/page-header";
import { SelectField } from "@/components/shared/select-field";
import { Badge } from "@/components/ui/badge";
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

interface CountBreakdown {
  movement_type: string;
  count: number;
}

interface DailyBusinessReport {
  business_name: string;
  report_date: string;
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

function paymentMethodLabel(method: string): string {
  const labels: Record<string, string> = {
    cash: "Cash",
    mtn_mobile_money: "MTN Mobile Money",
    airtel_money: "Airtel Money",
    bank_transfer: "Bank transfer",
  };
  return labels[method] ?? method;
}

function categoryLabel(category: string): string {
  return category
    .split("_")
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(" ");
}

function errorText(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

function SummaryCard({
  label,
  value,
  detail,
  icon: Icon,
  emphasis = false,
}: {
  label: string;
  value: string;
  detail: string;
  icon: typeof CircleDollarSign;
  emphasis?: boolean;
}) {
  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardDescription>{label}</CardDescription>
        <Icon className="size-4 text-muted-foreground" />
      </CardHeader>
      <CardContent>
        <p className={`data-number text-xl font-semibold tracking-tight sm:text-2xl ${emphasis ? "text-primary" : ""}`}>
          {value}
        </p>
        <p className="mt-1 text-xs text-muted-foreground">{detail}</p>
      </CardContent>
    </Card>
  );
}

function BreakdownList({
  title,
  description,
  items,
  emptyText,
  formatLabel = categoryLabel,
}: {
  title: string;
  description: string;
  items: AmountBreakdown[];
  emptyText: string;
  formatLabel?: (category: string) => string;
}) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>{title}</CardTitle>
        <CardDescription>{description}</CardDescription>
      </CardHeader>
      <CardContent>
        {items.length ? (
          <ul className="divide-y divide-border">
            {items.map((item) => (
              <li key={item.category} className="flex items-center justify-between gap-4 py-3 first:pt-0 last:pb-0">
                <span className="text-sm">{formatLabel(item.category)}</span>
                <span className="data-number text-right text-sm font-medium">{formatMoney(item.amount)}</span>
              </li>
            ))}
          </ul>
        ) : (
          <p className="text-sm text-muted-foreground">{emptyText}</p>
        )}
      </CardContent>
    </Card>
  );
}

export function ReportsPage() {
  const [businesses, setBusinesses] = useState<Business[]>([]);
  const [businessId, setBusinessId] = useState("");
  const [reportDate, setReportDate] = useState(localDate);
  const [report, setReport] = useState<DailyBusinessReport | null>(null);
  const [isLoadingBusinesses, setIsLoadingBusinesses] = useState(true);
  const [isLoadingReport, setIsLoadingReport] = useState(false);
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
    if (!businessId || !reportDate) {
      setReport(null);
      setIsLoadingReport(false);
      return;
    }

    let active = true;
    setReport(null);
    setIsLoadingReport(true);
    setError(null);
    invoke<DailyBusinessReport>("daily_business_report", { businessId, reportDate })
      .then((result) => {
        if (active) {
          setReport(result);
        }
      })
      .catch((cause: unknown) => {
        if (active) {
          setReport(null);
          setError(`Could not load the daily report. ${errorText(cause)}`);
        }
      })
      .finally(() => {
        if (active) {
          setIsLoadingReport(false);
        }
      });
    return () => {
      active = false;
    };
  }, [businessId, reportDate]);

  const selectedBusiness = businesses.find((business) => business.id === businessId);
  const dateLabel = reportDate
    ? new Date(`${reportDate}T00:00:00`).toLocaleDateString("en-RW", {
        day: "numeric",
        month: "long",
        year: "numeric",
      })
    : "Select a date";

  return (
    <div className="space-y-6">
      <PageHeader
        eyebrow="Finance & reporting"
        title="Daily business report"
        description={selectedBusiness ? `Financial activity for ${selectedBusiness.name}.` : "Review sales and financial activity for a selected day."}
        actions={
          <>
            <div className="w-full space-y-1.5 sm:w-56">
              <label htmlFor="report-business" className="text-xs font-medium text-muted-foreground">Business</label>
              <SelectField
                id="report-business"
                value={businessId}
                onChange={(event) => setBusinessId(event.target.value)}
                disabled={isLoadingBusinesses || businesses.length === 0}
              >
                {businesses.map((business) => (
                  <option key={business.id} value={business.id}>{business.name}</option>
                ))}
              </SelectField>
            </div>
            <div className="w-full space-y-1.5 sm:w-44">
              <label htmlFor="report-date" className="text-xs font-medium text-muted-foreground">Report date</label>
              <Input
                id="report-date"
                type="date"
                value={reportDate}
                onChange={(event) => setReportDate(event.target.value)}
                max={localDate()}
              />
            </div>
          </>
        }
      />

      {error ? (
        <div role="alert" className="rounded-md border border-destructive/30 bg-destructive/5 p-4 text-sm text-destructive">
          {error}
        </div>
      ) : null}

      {isLoadingBusinesses ? (
        <Card>
          <CardContent className="flex items-center gap-2 py-8 text-sm text-muted-foreground">
            <LoaderCircle className="size-4 animate-spin" />
            Loading business information...
          </CardContent>
        </Card>
      ) : businesses.length === 0 ? (
        <Card>
          <CardHeader>
            <CardTitle>No business registered</CardTitle>
            <CardDescription>Set up a business before generating a financial report.</CardDescription>
          </CardHeader>
        </Card>
      ) : report ? (
        <>
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div>
              <h2 className="font-heading text-lg font-semibold">{dateLabel}</h2>
              <p className="text-sm text-muted-foreground">Daily totals and current outstanding balances</p>
            </div>
            {isLoadingReport ? <Badge variant="outline">Updating…</Badge> : <Badge variant="outline">Daily summary</Badge>}
          </div>

          <section aria-label="Daily financial summary" className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
            <SummaryCard
              label="Sales"
              value={formatMoney(report.sales_total)}
              detail={`${report.sale_count.toLocaleString("en-RW")} completed sales`}
              icon={ShoppingBag}
            />
            <SummaryCard
              label="Cost of goods sold"
              value={formatMoney(report.cost_of_goods_sold)}
              detail={`${report.sale_line_count.toLocaleString("en-RW")} sold product lines`}
              icon={Package}
            />
            <SummaryCard
              label="Gross profit"
              value={formatMoney(report.gross_profit)}
              detail="Sales less historical FIFO cost"
              icon={CircleDollarSign}
              emphasis
            />
            <SummaryCard
              label="Operating result"
              value={formatMoney(report.operating_result)}
              detail="Gross profit less recorded expenses"
              icon={BarChart3}
              emphasis
            />
          </section>

          <section aria-label="Daily cash and purchasing activity" className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
            <SummaryCard
              label="Credit sales"
              value={formatMoney(report.credit_sales)}
              detail="Unpaid amount from sales on this date"
              icon={ReceiptText}
            />
            <SummaryCard
              label="Customer repayments"
              value={formatMoney(report.customer_repayments)}
              detail="Debt collections received on this date"
              icon={Wallet}
            />
            <SummaryCard
              label="Supplier payments"
              value={formatMoney(report.supplier_payments)}
              detail="Payments made on this date"
              icon={Wallet}
            />
            <SummaryCard
              label="Purchases received"
              value={formatMoney(report.purchases_received)}
              detail="Goods received, not treated as an expense"
              icon={Package}
            />
          </section>

          <section aria-label="Daily breakdowns" className="grid gap-4 xl:grid-cols-2">
            <BreakdownList
              title="Sales collections"
              description="Payments recorded against completed sales by method"
              items={report.sales_by_payment_method}
              emptyText="No sale payments were recorded on this date."
              formatLabel={paymentMethodLabel}
            />
            <BreakdownList
              title="Operating expenses"
              description={`Total recorded: ${formatMoney(report.expenses_total)}`}
              items={report.expenses_by_category}
              emptyText="No operating expenses were recorded on this date."
            />
          </section>

          <section aria-label="Outstanding balances and adjustments" className="grid gap-4 xl:grid-cols-2">
            <Card>
              <CardHeader>
                <CardTitle>Outstanding balances</CardTitle>
                <CardDescription>Current business-wide balances, not limited to this report date</CardDescription>
              </CardHeader>
              <CardContent className="grid gap-4 sm:grid-cols-2">
                <div className="rounded-lg border border-border/80 p-4">
                  <p className="text-xs font-medium text-muted-foreground">Customer receivables</p>
                  <p className="data-number mt-2 text-lg font-semibold">{formatMoney(report.customer_outstanding)}</p>
                </div>
                <div className="rounded-lg border border-border/80 p-4">
                  <p className="text-xs font-medium text-muted-foreground">Supplier balances</p>
                  <p className="data-number mt-2 text-lg font-semibold">{formatMoney(report.supplier_outstanding)}</p>
                </div>
              </CardContent>
            </Card>
            <Card>
              <CardHeader>
                <CardTitle>Stock adjustments</CardTitle>
                <CardDescription>Adjustment events recorded on {dateLabel}</CardDescription>
              </CardHeader>
              <CardContent>
                {report.stock_adjustment_counts.length ? (
                  <ul className="divide-y divide-border">
                    {report.stock_adjustment_counts.map((item) => (
                      <li key={item.movement_type} className="flex items-center justify-between gap-4 py-3 first:pt-0 last:pb-0">
                        <span className="text-sm">{categoryLabel(item.movement_type)}</span>
                        <span className="data-number text-sm font-medium">{item.count.toLocaleString("en-RW")}</span>
                      </li>
                    ))}
                  </ul>
                ) : (
                  <p className="text-sm text-muted-foreground">No stock adjustment events were recorded on this date.</p>
                )}
              </CardContent>
            </Card>
          </section>

          <p className="rounded-lg border border-border/80 bg-muted/30 p-4 text-xs leading-5 text-muted-foreground">
            This report uses recorded sale costs and ledgers. Timestamp-based activity follows the computer&apos;s local timezone; the business timezone setting is not applied yet. Cash opening/closing balances, cash discrepancies, employee attribution, and PDF export are also not available yet.
          </p>
        </>
      ) : isLoadingReport ? (
        <Card>
          <CardContent className="flex items-center gap-2 py-8 text-sm text-muted-foreground">
            <LoaderCircle className="size-4 animate-spin" />
            Loading the daily report...
          </CardContent>
        </Card>
      ) : null}
    </div>
  );
}
