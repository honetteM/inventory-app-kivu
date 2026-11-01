import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { ArrowRight, Boxes, CircleDollarSign, LoaderCircle, Package, ShoppingCart, TriangleAlert } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { SelectField } from "@/components/shared/select-field";
import { PageHeader } from "@/components/shared/page-header";

const QUANTITY_SCALE = 1000n;
const MONEY_SCALE = 100n;

interface Business {
  id: string;
  name: string;
}

interface InventoryItem {
  product_id: string;
  name: string;
  sku: string;
  base_unit_name: string;
  quantity_on_hand: number;
  stock_value: number;
  min_stock_quantity: number;
}

interface InventorySummary {
  items: InventoryItem[];
  total_count: number;
  total_stock_value: number;
  low_stock_count: number;
  out_of_stock_count: number;
}

function formatQuantity(value: number): string {
  if (!Number.isSafeInteger(value)) {
    return "—";
  }
  const scaled = BigInt(value);
  const whole = scaled / QUANTITY_SCALE;
  const fraction = scaled % QUANTITY_SCALE;
  return fraction === 0n
    ? new Intl.NumberFormat("en-RW").format(whole)
    : `${new Intl.NumberFormat("en-RW").format(whole)}.${fraction.toString().padStart(3, "0").replace(/0+$/, "")}`;
}

function formatMoney(value: number): string {
  if (!Number.isSafeInteger(value)) {
    return "—";
  }
  const amount = BigInt(value);
  const whole = amount / MONEY_SCALE;
  const fraction = amount % MONEY_SCALE;
  return `${new Intl.NumberFormat("en-RW").format(whole)}${fraction === 0n ? "" : `.${fraction.toString().padStart(2, "0")}`} RWF`;
}

function errorText(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

export function DashboardPage() {
  const [businesses, setBusinesses] = useState<Business[]>([]);
  const [businessId, setBusinessId] = useState("");
  const [summary, setSummary] = useState<InventorySummary | null>(null);
  const [isLoadingBusinesses, setIsLoadingBusinesses] = useState(true);
  const [isLoadingInventory, setIsLoadingInventory] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    invoke<Business[]>("business_list")
      .then((result) => {
        if (active) {
          setBusinesses(result);
          setBusinessId((current) => current || result[0]?.id || "");
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
    if (!businessId) {
      setSummary(null);
      return;
    }

    let active = true;
    setIsLoadingInventory(true);
    setError(null);
    invoke<InventorySummary>("inventory_list", {
      businessId,
      search: null,
      offset: 0,
      limit: 8,
    })
      .then((result) => {
        if (active) {
          setSummary(result);
        }
      })
      .catch((cause: unknown) => {
        if (active) {
          setSummary(null);
          setError(`Could not load the inventory summary. ${errorText(cause)}`);
        }
      })
      .finally(() => {
        if (active) {
          setIsLoadingInventory(false);
        }
      });

    return () => {
      active = false;
    };
  }, [businessId]);

  const selectedBusiness = businesses.find((business) => business.id === businessId);
  const stockAlerts = summary?.items.filter(
    (item) => item.quantity_on_hand === 0 || item.quantity_on_hand <= item.min_stock_quantity,
  ) ?? [];

  return (
    <div className="space-y-6">
      <PageHeader
        eyebrow="Overview"
        title="Business dashboard"
        description={selectedBusiness ? `A live inventory overview for ${selectedBusiness.name}.` : "A live overview of your shop's inventory."}
        actions={
          <>
            {businesses.length > 1 ? (
              <div className="w-full space-y-1.5 sm:w-56">
                <label htmlFor="dashboard-business" className="text-xs font-medium text-muted-foreground">Business</label>
                <SelectField id="dashboard-business" value={businessId} onChange={(event) => setBusinessId(event.target.value)}>
                  {businesses.map((business) => (
                    <option key={business.id} value={business.id}>{business.name}</option>
                  ))}
                </SelectField>
              </div>
            ) : null}
            <Button asChild>
              <Link to="/pos"><ShoppingCart /> New sale</Link>
            </Button>
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
            <CardTitle>Set up your business</CardTitle>
            <CardDescription>Register a business before using inventory and point of sale.</CardDescription>
          </CardHeader>
          <CardContent>
            <Button asChild variant="outline"><Link to="/settings">Open settings <ArrowRight /></Link></Button>
          </CardContent>
        </Card>
      ) : (
        <>
          <section aria-label="Inventory summary" className="grid gap-4 xl:grid-cols-3">
            <Card>
              <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                <CardDescription>Inventory value</CardDescription>
                <CircleDollarSign className="size-4 text-muted-foreground" />
              </CardHeader>
              <CardContent>
                <p className="data-number text-2xl font-semibold tracking-tight">
                  {isLoadingInventory || !summary ? "—" : formatMoney(summary.total_stock_value)}
                </p>
                <p className="mt-1 text-xs text-muted-foreground">Remaining stock at recorded FIFO purchase costs</p>
              </CardContent>
            </Card>
            <Card>
              <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                <CardDescription>Active products</CardDescription>
                <Package className="size-4 text-muted-foreground" />
              </CardHeader>
              <CardContent>
                <p className="data-number text-2xl font-semibold tracking-tight">
                  {isLoadingInventory || !summary ? "—" : summary.total_count.toLocaleString("en-RW")}
                </p>
                <p className="mt-1 text-xs text-muted-foreground">Products in this business catalogue</p>
              </CardContent>
            </Card>
            <Card>
              <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                <CardDescription>Stock needing attention</CardDescription>
                <TriangleAlert className="size-4 text-muted-foreground" />
              </CardHeader>
              <CardContent>
                <p className="data-number text-2xl font-semibold tracking-tight">
                  {isLoadingInventory || !summary
                    ? "—"
                    : (summary.low_stock_count + summary.out_of_stock_count).toLocaleString("en-RW")}
                </p>
                <p className="mt-1 text-xs text-muted-foreground">
                  {summary ? `${summary.low_stock_count.toLocaleString("en-RW")} low · ${summary.out_of_stock_count.toLocaleString("en-RW")} out of stock` : "Based on saved minimum stock levels"}
                </p>
              </CardContent>
            </Card>
          </section>

          <section className="grid gap-4 xl:grid-cols-[minmax(0,1.5fr)_minmax(18rem,0.8fr)]">
            <Card>
              <CardHeader className="flex flex-row items-start justify-between gap-4">
                <div>
                  <CardTitle>Stock overview</CardTitle>
                  <CardDescription className="mt-1">Current on-hand quantity and FIFO value by product.</CardDescription>
                </div>
                <Button asChild variant="outline" size="sm">
                  <Link to="/inventory">View inventory <ArrowRight /></Link>
                </Button>
              </CardHeader>
              <CardContent>
                {isLoadingInventory ? (
                  <div className="flex items-center gap-2 py-8 text-sm text-muted-foreground">
                    <LoaderCircle className="size-4 animate-spin" />
                    Loading inventory...
                  </div>
                ) : !summary || summary.items.length === 0 ? (
                  <div className="py-8 text-center">
                    <Boxes className="mx-auto mb-2 size-7 text-muted-foreground" />
                    <p className="text-sm text-muted-foreground">No active products to display.</p>
                    <Button asChild variant="link" className="mt-2"><Link to="/products">Add products</Link></Button>
                  </div>
                ) : (
                  <div className="divide-y">
                    {summary.items.map((item) => {
                      const low = item.quantity_on_hand <= item.min_stock_quantity;
                      return (
                        <div key={item.product_id} className="flex items-center justify-between gap-4 py-3 first:pt-0 last:pb-0">
                          <div className="min-w-0">
                            <p className="truncate text-sm font-medium">{item.name}</p>
                            <p className="mt-0.5 text-xs text-muted-foreground">{item.sku}</p>
                          </div>
                          <div className="flex shrink-0 items-center gap-5">
                            <div className="text-right">
                              <p className="data-number text-sm font-medium">{formatQuantity(item.quantity_on_hand)} <span className="text-muted-foreground">{item.base_unit_name}</span></p>
                              <p className="data-number text-xs text-muted-foreground">{formatMoney(item.stock_value)}</p>
                            </div>
                            <Badge
                              variant={item.quantity_on_hand === 0 ? "destructive" : low ? "outline" : "secondary"}
                              className={low && item.quantity_on_hand > 0
                                ? "border-amber-500/40 bg-amber-500/10 text-amber-800 dark:text-amber-300"
                                : undefined}
                            >
                              {item.quantity_on_hand === 0 ? "Out" : low ? "Low" : "In stock"}
                            </Badge>
                          </div>
                        </div>
                      );
                    })}
                  </div>
                )}
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle>Quick actions</CardTitle>
                <CardDescription>Shortcuts to common shop tasks.</CardDescription>
              </CardHeader>
              <CardContent className="grid gap-2">
                <Button asChild variant="outline" className="h-auto justify-between py-3">
                  <Link to="/pos"><span className="flex items-center gap-3"><ShoppingCart className="size-4 text-primary" /> Start a sale</span><ArrowRight /></Link>
                </Button>
                <Button asChild variant="outline" className="h-auto justify-between py-3">
                  <Link to="/inventory"><span className="flex items-center gap-3"><Boxes className="size-4 text-primary" /> Receive or adjust stock</span><ArrowRight /></Link>
                </Button>
                <Button asChild variant="outline" className="h-auto justify-between py-3">
                  <Link to="/products"><span className="flex items-center gap-3"><Package className="size-4 text-primary" /> Manage products</span><ArrowRight /></Link>
                </Button>
              </CardContent>
            </Card>
          </section>

          {stockAlerts.length > 0 ? (
            <section>
              <div className="mb-3 flex items-end justify-between gap-4">
                <div>
                  <h2 className="text-base font-semibold">Stock alerts in this preview</h2>
                  <p className="mt-1 text-sm text-muted-foreground">Low and out-of-stock products among the first items shown.</p>
                </div>
                <Button asChild variant="link" size="sm"><Link to="/inventory">Review all alerts <ArrowRight /></Link></Button>
              </div>
              <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
                {stockAlerts.slice(0, 4).map((item) => (
                  <Card key={item.product_id} size="sm">
                    <CardContent className="flex items-center justify-between gap-3">
                      <p className="truncate text-sm font-medium">{item.name}</p>
                      <Badge variant={item.quantity_on_hand === 0 ? "destructive" : "outline"}>
                        {item.quantity_on_hand === 0 ? "Out" : `${formatQuantity(item.quantity_on_hand)} left`}
                      </Badge>
                    </CardContent>
                  </Card>
                ))}
              </div>
            </section>
          ) : null}
        </>
      )}
    </div>
  );
}
