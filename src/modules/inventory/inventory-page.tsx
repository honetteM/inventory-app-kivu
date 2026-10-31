import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState, type FormEvent } from "react";
import { Fragment } from "react";
import { History, LoaderCircle, Search, Warehouse } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";

const PAGE_SIZE = 20;
const MOVEMENT_PAGE_SIZE = 20;
const QUANTITY_SCALE = 1000n;
const MAX_SAFE_INTEGER = BigInt(Number.MAX_SAFE_INTEGER);

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
  min_stock_quantity: number;
  base_unit_allows_decimals: boolean;
  has_movements: boolean;
}

interface InventoryPageData {
  items: InventoryItem[];
  total_count: number;
}

interface StockMovement {
  id: string;
  product_id: string;
  product_name: string;
  movement_type: string;
  quantity_delta: number;
  previous_quantity: number;
  new_quantity: number;
  reference_type: string | null;
  reference_id: string | null;
  reason: string | null;
  user_id: string | null;
  created_at: string;
}

function formatQuantity(quantity: number): string {
  if (!Number.isSafeInteger(quantity)) {
    return "Invalid quantity";
  }
  const scaled = BigInt(quantity);
  const whole = scaled / QUANTITY_SCALE;
  const fraction = scaled % QUANTITY_SCALE;
  const formattedWhole = new Intl.NumberFormat("en-RW").format(whole);
  return fraction === 0n
    ? formattedWhole
    : `${formattedWhole}.${fraction.toString().padStart(3, "0").replace(/0+$/, "")}`;
}

function toScaledQuantity(value: string, allowsDecimals: boolean): number {
  const pattern = allowsDecimals ? /^\d+(?:\.\d{1,3})?$/ : /^\d+$/;
  if (!pattern.test(value)) {
    throw new Error(
      allowsDecimals
        ? "Quantity must be non-negative with up to 3 decimal places."
        : "This product only allows whole base units.",
    );
  }
  const [whole, fraction = ""] = value.split(".");
  const scaled = BigInt(whole) * QUANTITY_SCALE + BigInt(fraction.padEnd(3, "0") || "0");
  if (scaled > MAX_SAFE_INTEGER) {
    throw new Error("Quantity is too large.");
  }
  return Number(scaled);
}

function errorText(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

function movementLabel(value: string): string {
  return value
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

export function InventoryPage() {
  const [businesses, setBusinesses] = useState<Business[]>([]);
  const [businessId, setBusinessId] = useState("");
  const [inventory, setInventory] = useState<InventoryItem[]>([]);
  const [totalCount, setTotalCount] = useState(0);
  const [search, setSearch] = useState("");
  const [page, setPage] = useState(0);
  const [expandedProductId, setExpandedProductId] = useState<string | null>(null);
  const [movements, setMovements] = useState<StockMovement[]>([]);
  const [openingQuantity, setOpeningQuantity] = useState("");
  const [openingReason, setOpeningReason] = useState("");
  const [isLoadingBusinesses, setIsLoadingBusinesses] = useState(true);
  const [isLoadingInventory, setIsLoadingInventory] = useState(false);
  const [isLoadingMovements, setIsLoadingMovements] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const pageCount = Math.ceil(totalCount / PAGE_SIZE);
  const firstItem = totalCount === 0 ? 0 : page * PAGE_SIZE + 1;
  const lastItem = Math.min((page + 1) * PAGE_SIZE, totalCount);
  const selectedProduct = inventory.find((item) => item.product_id === expandedProductId);

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
    if (!businessId) {
      setInventory([]);
      setTotalCount(0);
      setExpandedProductId(null);
      return;
    }

    let active = true;
    const timer = window.setTimeout(() => {
      setIsLoadingInventory(true);
      invoke<InventoryPageData>("inventory_list", {
        businessId,
        search: search.trim() || null,
        offset: page * PAGE_SIZE,
        limit: PAGE_SIZE,
      })
        .then((result) => {
          if (active) {
            setInventory(result.items);
            setTotalCount(result.total_count);
            if (expandedProductId && !result.items.some((item) => item.product_id === expandedProductId)) {
              setExpandedProductId(null);
            }
          }
        })
        .catch((cause: unknown) => {
          if (active) {
            setError(`Could not load inventory. ${errorText(cause)}`);
          }
        })
        .finally(() => {
          if (active) {
            setIsLoadingInventory(false);
          }
        });
    }, 150);
    return () => {
      active = false;
      window.clearTimeout(timer);
    };
  }, [businessId, expandedProductId, page, search]);

  useEffect(() => {
    if (!businessId || !expandedProductId) {
      setMovements([]);
      setOpeningQuantity("");
      setOpeningReason("");
      return;
    }

    let active = true;
    setIsLoadingMovements(true);
    invoke<StockMovement[]>("stock_movement_list", {
      businessId,
      productId: expandedProductId,
      offset: 0,
      limit: MOVEMENT_PAGE_SIZE,
    })
      .then((result) => {
        if (active) {
          setMovements(result);
        }
      })
      .catch((cause: unknown) => {
        if (active) {
          setError(`Could not load stock history. ${errorText(cause)}`);
        }
      })
      .finally(() => {
        if (active) {
          setIsLoadingMovements(false);
        }
      });
    return () => {
      active = false;
    };
  }, [businessId, expandedProductId]);

  async function handleOpeningStock(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!businessId || !selectedProduct || selectedProduct.has_movements) {
      setError("Opening stock is only available before the product has any stock movements.");
      return;
    }

    setError(null);
    setIsSaving(true);
    try {
      await invoke<StockMovement>("inventory_set_opening_stock", {
        businessId,
        productId: selectedProduct.product_id,
        quantity: toScaledQuantity(openingQuantity, selectedProduct.base_unit_allows_decimals),
        reason: openingReason.trim() || null,
      });
      setOpeningQuantity("");
      setOpeningReason("");
      const result = await invoke<InventoryPageData>("inventory_list", {
        businessId,
        search: search.trim() || null,
        offset: page * PAGE_SIZE,
        limit: PAGE_SIZE,
      });
      setInventory(result.items);
      setTotalCount(result.total_count);
      const history = await invoke<StockMovement[]>("stock_movement_list", {
        businessId,
        productId: selectedProduct.product_id,
        offset: 0,
        limit: MOVEMENT_PAGE_SIZE,
      });
      setMovements(history);
    } catch (cause) {
      setError(`Could not record opening stock. ${errorText(cause)}`);
    } finally {
      setIsSaving(false);
    }
  }

  return (
    <div className="mx-auto max-w-7xl space-y-6">
      <header className="flex flex-col gap-4 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="text-sm font-medium text-muted-foreground">Stock control</p>
          <h1 className="text-3xl font-semibold tracking-tight">Inventory</h1>
          <p className="mt-2 text-muted-foreground">
            Record opening quantities and review traceable stock movements.
          </p>
        </div>
        {businesses.length > 0 ? (
          <div className="w-full space-y-2 md:max-w-xs">
            <Label htmlFor="inventory-business">Business</Label>
            <select
              id="inventory-business"
              value={businessId}
              onChange={(event) => {
                setBusinessId(event.target.value);
                setPage(0);
                setExpandedProductId(null);
              }}
              className="h-9 w-full rounded-lg border border-input bg-background px-3 text-sm focus-visible:outline-none focus-visible:ring-3 focus-visible:ring-ring/50"
            >
              {businesses.map((business) => (
                <option key={business.id} value={business.id}>{business.name}</option>
              ))}
            </select>
          </div>
        ) : null}
      </header>

      {error ? (
        <div role="alert" className="rounded-lg border border-destructive/30 bg-destructive/5 p-4 text-sm text-destructive">
          {error}
        </div>
      ) : null}

      {!isLoadingBusinesses && businesses.length === 0 ? (
        <Card>
          <CardContent className="py-6 text-sm text-muted-foreground">
            Register a business and add products before managing stock.
          </CardContent>
        </Card>
      ) : isLoadingBusinesses ? (
        <Card>
          <CardContent className="flex items-center gap-2 py-6 text-sm text-muted-foreground">
            <LoaderCircle className="size-4 animate-spin" />
            Loading businesses...
          </CardContent>
        </Card>
      ) : (
        <Card>
          <CardHeader>
            <CardTitle>Stock balances</CardTitle>
            <CardDescription>
              {totalCount.toLocaleString("en-RW")} active products · opening quantities use base units
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="relative max-w-md">
              <Search className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
              <Input
                aria-label="Search inventory"
                className="pl-9"
                placeholder="Search product, SKU, barcode, category..."
                value={search}
                onChange={(event) => {
                  setSearch(event.target.value);
                  setPage(0);
                }}
              />
            </div>

            {isLoadingInventory ? (
              <div className="flex items-center justify-center gap-2 py-8 text-sm text-muted-foreground">
                <LoaderCircle className="size-4 animate-spin" />
                Loading inventory...
              </div>
            ) : inventory.length === 0 ? (
              <div className="py-8 text-center">
                <Warehouse className="mx-auto mb-2 size-8 text-muted-foreground" />
                <p className="text-sm text-muted-foreground">
                  {search.trim() ? "No inventory items match your search." : "No active products found."}
                </p>
              </div>
            ) : (
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Product</TableHead>
                    <TableHead>SKU</TableHead>
                    <TableHead>Base unit</TableHead>
                    <TableHead className="text-right">On hand</TableHead>
                    <TableHead className="text-right">Minimum</TableHead>
                    <TableHead>Status</TableHead>
                    <TableHead>Actions</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {inventory.map((item) => {
                    const isExpanded = expandedProductId === item.product_id;
                    const isLowStock = item.quantity_on_hand <= item.min_stock_quantity;
                    return (
                      <Fragment key={item.product_id}>
                        <TableRow>
                          <TableCell className="font-medium">{item.name}</TableCell>
                          <TableCell>{item.sku}</TableCell>
                          <TableCell>{item.base_unit_name}</TableCell>
                          <TableCell className="text-right">{formatQuantity(item.quantity_on_hand)}</TableCell>
                          <TableCell className="text-right">{formatQuantity(item.min_stock_quantity)}</TableCell>
                          <TableCell>
                            <Badge variant={isLowStock ? "destructive" : "secondary"}>
                              {item.quantity_on_hand === 0 ? "Out of stock" : isLowStock ? "Low stock" : "In stock"}
                            </Badge>
                          </TableCell>
                          <TableCell>
                            <Button
                              type="button"
                              variant="outline"
                              size="sm"
                              aria-expanded={isExpanded}
                              onClick={() => setExpandedProductId((current) => current === item.product_id ? null : item.product_id)}
                            >
                              <History />
                              {isExpanded ? "Hide details" : "Manage stock"}
                            </Button>
                          </TableCell>
                        </TableRow>
                        {isExpanded ? (
                          <TableRow key={`${item.product_id}-details`}>
                            <TableCell colSpan={7} className="bg-muted/20">
                              <div className="space-y-5 p-2">
                                {!item.has_movements ? (
                                  <form className="grid gap-3 rounded-lg border bg-background p-4 md:grid-cols-[1fr_1fr_auto]" onSubmit={handleOpeningStock}>
                                    <div className="space-y-2">
                                      <Label htmlFor={`opening-quantity-${item.product_id}`}>
                                        Opening quantity ({item.base_unit_name})
                                      </Label>
                                      <Input
                                        id={`opening-quantity-${item.product_id}`}
                                        type="number"
                                        min="0"
                                        step={item.base_unit_allows_decimals ? "0.001" : "1"}
                                        required
                                        value={openingQuantity}
                                        onChange={(event) => setOpeningQuantity(event.target.value)}
                                      />
                                    </div>
                                    <div className="space-y-2">
                                      <Label htmlFor={`opening-reason-${item.product_id}`}>Reason (optional)</Label>
                                      <Input
                                        id={`opening-reason-${item.product_id}`}
                                        maxLength={500}
                                        value={openingReason}
                                        onChange={(event) => setOpeningReason(event.target.value)}
                                        placeholder="Initial physical stock count"
                                      />
                                    </div>
                                    <Button type="submit" className="self-end" disabled={isSaving || !openingQuantity}>
                                      {isSaving ? <LoaderCircle className="animate-spin" /> : <Warehouse />}
                                      Record opening stock
                                    </Button>
                                  </form>
                                ) : (
                                  <p className="text-sm text-muted-foreground">
                                    Opening stock was already recorded. Future quantity changes must use a stock movement.
                                  </p>
                                )}

                                <div className="space-y-3">
                                  <h3 className="font-medium">Movement history</h3>
                                  {isLoadingMovements ? (
                                    <div className="flex items-center gap-2 text-sm text-muted-foreground">
                                      <LoaderCircle className="size-4 animate-spin" />
                                      Loading movement history...
                                    </div>
                                  ) : movements.length === 0 ? (
                                    <p className="text-sm text-muted-foreground">No stock movements recorded.</p>
                                  ) : (
                                    <div className="space-y-2">
                                      {movements.map((movement) => (
                                        <div key={movement.id} className="flex flex-wrap items-center justify-between gap-3 rounded-lg border bg-background p-3 text-sm">
                                          <div>
                                            <p className="font-medium">{movementLabel(movement.movement_type)}</p>
                                            <p className="text-muted-foreground">
                                              {movement.reason ?? movement.reference_id ?? "No reason recorded"}
                                              {" · "}
                                              {new Date(movement.created_at).toLocaleString()}
                                            </p>
                                          </div>
                                          <p className="text-right">
                                            {formatQuantity(movement.previous_quantity)} → {formatQuantity(movement.new_quantity)}
                                            <span className="ml-2 text-muted-foreground">
                                              ({movement.quantity_delta >= 0 ? "+" : ""}
                                              {formatQuantity(movement.quantity_delta)})
                                            </span>
                                          </p>
                                        </div>
                                      ))}
                                    </div>
                                  )}
                                </div>
                              </div>
                            </TableCell>
                          </TableRow>
                        ) : null}
                      </Fragment>
                    );
                  })}
                </TableBody>
              </Table>
            )}

            <div className="flex flex-wrap items-center justify-between gap-3 border-t pt-4">
              <p className="text-sm text-muted-foreground">Showing {firstItem}–{lastItem} of {totalCount}</p>
              <div className="flex gap-2">
                <Button variant="outline" disabled={page === 0 || isLoadingInventory} onClick={() => setPage((current) => Math.max(0, current - 1))}>
                  Previous
                </Button>
                <Button variant="outline" disabled={page + 1 >= pageCount || isLoadingInventory} onClick={() => setPage((current) => current + 1)}>
                  Next
                </Button>
              </div>
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
