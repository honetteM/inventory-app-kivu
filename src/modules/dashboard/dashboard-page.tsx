import {
  ArrowDownRight,
  ArrowUpRight,
  Banknote,
  Boxes,
  BriefcaseBusiness,
  CircleDollarSign,
  PackageSearch,
  TrendingDown,
  TrendingUp,
  Truck,
} from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";

const summaryCards = [
  {
    label: "Today's sales",
    value: "1,130,000 RWF",
    change: "+18.4%",
    trend: "up",
    description: "vs yesterday",
    icon: CircleDollarSign,
  },
  {
    label: "Today's gross profit",
    value: "482,000 RWF",
    change: "+9.1%",
    trend: "up",
    description: "margin 42.7%",
    icon: TrendingUp,
  },
  {
    label: "Today's expenses",
    value: "218,400 RWF",
    change: "-4.3%",
    trend: "down",
    description: "lower than last week",
    icon: Banknote,
  },
  {
    label: "Net operating result",
    value: "+263,600 RWF",
    change: "+12.8%",
    trend: "up",
    description: "after operating costs",
    icon: BriefcaseBusiness,
  },
] as const;

const salesBars = [
  { day: "Mon", value: 42 },
  { day: "Tue", value: 64 },
  { day: "Wed", value: 36 },
  { day: "Thu", value: 76 },
  { day: "Fri", value: 58 },
  { day: "Sat", value: 88 },
  { day: "Sun", value: 49 },
] as const;

const paymentMix = [
  { label: "Cash", value: 42, amount: "487,000 RWF" },
  { label: "Mobile Money", value: 34, amount: "392,000 RWF" },
  { label: "Bank transfer", value: 18, amount: "209,000 RWF" },
  { label: "Credit", value: 6, amount: "68,000 RWF" },
] as const;

const topProducts = [
  { name: "Cement bags", sold: 186, share: 92 },
  { name: "Steel rods", sold: 142, share: 78 },
  { name: "PVC pipes", sold: 118, share: 69 },
  { name: "Tile adhesive", sold: 96, share: 54 },
] as const;

const monthlyPerformance = [
  { month: "Jan", sales: 52, expenses: 31 },
  { month: "Feb", sales: 64, expenses: 35 },
  { month: "Mar", sales: 58, expenses: 34 },
  { month: "Apr", sales: 80, expenses: 41 },
  { month: "May", sales: 70, expenses: 38 },
  { month: "Jun", sales: 92, expenses: 46 },
] as const;

const activityRows = [
  { type: "Sale", ref: "INV-2014", customer: "Aline M.", amount: "96,000 RWF", time: "08:45" },
  { type: "Purchase", ref: "PO-332", customer: "Kigali Build", amount: "240,000 RWF", time: "09:20" },
  { type: "Expense", ref: "EXP-118", customer: "Electricity", amount: "18,500 RWF", time: "10:05" },
  { type: "Stock", ref: "ADJ-009", customer: "Warehouse A", amount: "-12 bags", time: "11:10" },
  { type: "Payment", ref: "PY-402", customer: "M. Hirwa", amount: "34,000 RWF", time: "12:30" },
] as const;

const inventoryAlerts = [
  { item: "Cement 50kg", qty: "12 remaining" },
  { item: "Nails box", qty: "3 remaining" },
  { item: "Paint white", qty: "0 remaining" },
  { item: "Pipes 110mm", qty: "6 remaining" },
] as const;

function MetricCard({
  label,
  value,
  description,
  change,
  trend,
  icon: Icon,
}: {
  label: string;
  value: string;
  description: string;
  change: string;
  trend: "up" | "down";
  icon: typeof CircleDollarSign;
}) {
  const positive = trend === "up";

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardDescription>{label}</CardDescription>
        <div className="rounded-md bg-muted p-2">
          <Icon className="h-4 w-4 text-muted-foreground" />
        </div>
      </CardHeader>
      <CardContent>
        <div className="text-2xl font-semibold tracking-tight">{value}</div>
        <div className="mt-3 flex items-center gap-2 text-sm">
          <Badge variant={positive ? "default" : "secondary"}>
            {positive ? <ArrowUpRight className="h-3.5 w-3.5" /> : <ArrowDownRight className="h-3.5 w-3.5" />}
            {change}
          </Badge>
          <span className="text-muted-foreground">{description}</span>
        </div>
      </CardContent>
    </Card>
  );
}

export function DashboardPage() {
  return (
    <div className="space-y-6">
      <header className="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
        <div>
          <p className="text-sm font-medium text-muted-foreground">Overview</p>
          <h1 className="text-3xl font-semibold tracking-tight">Business dashboard</h1>
        </div>

        <div className="flex items-center gap-2">
          <Button variant="outline">Download report</Button>
          <Button>New sale</Button>
        </div>
      </header>

      <section className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        {summaryCards.map((card) => (
          <MetricCard key={card.label} {...card} />
        ))}
      </section>

      <section className="grid gap-4 xl:grid-cols-2">
        <Card>
          <CardHeader>
            <div className="flex items-center justify-between gap-2">
              <div>
                <CardTitle>Sales over the last 7 days</CardTitle>
                <CardDescription>Total revenue by day</CardDescription>
              </div>
              <Badge variant="outline">RWF</Badge>
            </div>
          </CardHeader>
          <CardContent>
            <div className="mt-4 flex h-40 items-end gap-3">
              {salesBars.map((bar) => (
                <div key={bar.day} className="flex flex-1 flex-col items-center gap-2">
                  <div
                    className="flex w-full items-end justify-center rounded-t-md bg-gradient-to-t from-primary to-primary/60"
                    style={{ height: `${bar.value}%` }}
                  />
                  <span className="text-xs text-muted-foreground">{bar.day}</span>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Sales by payment method</CardTitle>
            <CardDescription>Revenue distribution</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="flex items-center justify-center">
              <div className="relative flex h-32 w-32 items-center justify-center rounded-full bg-[radial-gradient(circle,_transparent_0_58%,_hsl(var(--muted))_59%_100%)]">
                <div className="flex h-20 w-20 items-center justify-center rounded-full bg-background text-sm font-semibold">
                  1.16M
                </div>
              </div>
            </div>
            <div className="space-y-3">
              {paymentMix.map((method) => (
                <div key={method.label}>
                  <div className="mb-1 flex items-center justify-between text-sm">
                    <span className="text-muted-foreground">{method.label}</span>
                    <span className="font-medium">{method.amount}</span>
                  </div>
                  <div className="h-2 rounded-full bg-muted">
                    <div className="h-2 rounded-full bg-primary" style={{ width: `${method.value}%` }} />
                  </div>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      </section>

      <section className="grid gap-4 xl:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Top product categories</CardTitle>
            <CardDescription>Most active categories this month</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            {topProducts.map((product) => (
              <div key={product.name}>
                <div className="mb-1 flex items-center justify-between text-sm">
                  <span>{product.name}</span>
                  <span className="text-muted-foreground">{product.sold} sold</span>
                </div>
                <div className="h-2 rounded-full bg-muted">
                  <div className="h-2 rounded-full bg-primary/80" style={{ width: `${product.share}%` }} />
                </div>
              </div>
            ))}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Monthly revenue & expenses</CardTitle>
            <CardDescription>Half-year summary</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="mt-6 grid grid-cols-6 items-end gap-3">
              {monthlyPerformance.map((point) => (
                <div key={point.month} className="flex flex-col items-center gap-2">
                  <div className="flex h-28 w-full items-end justify-center gap-1">
                    <div className="w-2 rounded-t-sm bg-primary/80" style={{ height: `${point.sales}%` }} />
                    <div className="w-2 rounded-t-sm bg-muted-foreground/40" style={{ height: `${point.expenses}%` }} />
                  </div>
                  <span className="text-xs text-muted-foreground">{point.month}</span>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      </section>

      <section className="grid gap-4 xl:grid-cols-[1.3fr_0.7fr]">
        <Card>
          <CardHeader>
            <div className="flex items-center justify-between gap-2">
              <div>
                <CardTitle>Recent activity</CardTitle>
                <CardDescription>Latest business events</CardDescription>
              </div>
              <Badge variant="secondary">Live</Badge>
            </div>
          </CardHeader>
          <CardContent className="p-0">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Type</TableHead>
                  <TableHead>Reference</TableHead>
                  <TableHead>Customer</TableHead>
                  <TableHead>Amount</TableHead>
                  <TableHead>Time</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {activityRows.map((row) => (
                  <TableRow key={`${row.type}-${row.ref}`}>
                    <TableCell>{row.type}</TableCell>
                    <TableCell>{row.ref}</TableCell>
                    <TableCell>{row.customer}</TableCell>
                    <TableCell>{row.amount}</TableCell>
                    <TableCell>{row.time}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Inventory snapshot</CardTitle>
            <CardDescription>Priority alerts</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            {inventoryAlerts.map((item) => (
              <div key={item.item} className="flex items-center justify-between rounded-lg border bg-muted/20 p-3">
                <div className="flex items-center gap-2">
                  <div className="rounded-md bg-warning/10 p-2 text-warning">
                    <Boxes className="h-4 w-4" />
                  </div>
                  <span className="font-medium">{item.item}</span>
                </div>
                <span className="text-sm text-muted-foreground">{item.qty}</span>
              </div>
            ))}
          </CardContent>
        </Card>
      </section>

      <section className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Inventory value</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="flex items-center justify-between">
              <span className="text-xl font-semibold">18.6M</span>
              <PackageSearch className="h-5 w-5 text-muted-foreground" />
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Purchase orders</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="flex items-center justify-between">
              <span className="text-xl font-semibold">24</span>
              <Truck className="h-5 w-5 text-muted-foreground" />
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Cash flow</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="flex items-center justify-between">
              <span className="text-xl font-semibold">+1.12M</span>
              <TrendingUp className="h-5 w-5 text-muted-foreground" />
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Outstanding debts</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="flex items-center justify-between">
              <span className="text-xl font-semibold">764K</span>
              <TrendingDown className="h-5 w-5 text-muted-foreground" />
            </div>
          </CardContent>
        </Card>
      </section>
    </div>
  );
}
