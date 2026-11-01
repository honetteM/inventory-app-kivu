import { invoke } from "@tauri-apps/api/core";
import { useEffect, useMemo, useState } from "react";
import { LoaderCircle, Plus, Search, ShoppingCart, Trash2 } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

const PAGE_SIZE = 30;
const QUANTITY_SCALE = 1000n;
const MONEY_SCALE = 100n;
const MAX_SAFE_INTEGER = BigInt(Number.MAX_SAFE_INTEGER);

interface Business {
  id: string;
  name: string;
}

type PaymentMethod = "cash" | "mtn_mobile_money" | "airtel_money" | "bank_transfer";

const PAYMENT_METHODS: { value: PaymentMethod; label: string }[] = [
  { value: "cash", label: "Cash" },
  { value: "mtn_mobile_money", label: "MTN Mobile Money" },
  { value: "airtel_money", label: "Airtel Money" },
  { value: "bank_transfer", label: "Bank transfer" },
];

function formatScaledQuantity(scaled: bigint): string {
  const whole = scaled / QUANTITY_SCALE;
  const fraction = scaled % QUANTITY_SCALE;
  const formattedWhole = new Intl.NumberFormat("en-RW").format(whole);
  return fraction === 0n
    ? formattedWhole
    : `${formattedWhole}.${fraction.toString().padStart(3, "0").replace(/0+$/, "")}`;
}

interface PosProduct {
  id: string;
  name: string;
  sku: string;
  barcode: string | null;
  base_unit_name: string;
  selling_price: number;
  minimum_selling_price: number;
  quantity_on_hand: number;
  base_unit_allows_decimals: boolean;
}

interface PosProductPageData {
  items: PosProduct[];
  total_count: number;
}

interface CartItem {
  product: PosProduct;
  quantity: string;
  sellingPrice: string;
  priceChangeReason: string;
  belowCostReasonRequired: boolean;
}

interface SaleReceipt {
  id: string;
  transaction_number: string;
  payment_method: PaymentMethod;
  total: number;
  created_at: string;
  items: {
    product_name: string;
    sku: string;
    quantity: number;
    unit_name: string;
    listed_unit_price: number;
    unit_price: number;
    line_total: number;
    price_change_reason: string | null;
  }[];
}

function formatQuantity(quantity: number): string {
  if (!Number.isSafeInteger(quantity)) {
    return "Invalid quantity";
  }
  return formatScaledQuantity(BigInt(quantity));
}

function parseQuantity(value: string, allowsDecimals: boolean): bigint | null {
  const pattern = allowsDecimals ? /^\d+(?:\.\d{1,3})?$/ : /^\d+$/;
  if (!pattern.test(value)) {
    return null;
  }
  const [whole, fraction = ""] = value.split(".");
  const scaled = BigInt(whole) * QUANTITY_SCALE + BigInt(fraction.padEnd(3, "0") || "0");
  return scaled <= MAX_SAFE_INTEGER ? scaled : null;
}

function parseMoney(value: string): bigint | null {
  const match = /^(\d+)(?:\.(\d{1,2}))?$/.exec(value);
  if (!match) {
    return null;
  }
  const fraction = (match[2] ?? "").padEnd(2, "0");
  const scaled = BigInt(match[1]) * MONEY_SCALE + BigInt(fraction || "0");
  return scaled <= MAX_SAFE_INTEGER ? scaled : null;
}

function moneyInputValue(scaledAmount: number): string {
  if (!Number.isSafeInteger(scaledAmount) || scaledAmount < 0) {
    return "";
  }
  const amount = BigInt(scaledAmount);
  const whole = amount / MONEY_SCALE;
  const fraction = amount % MONEY_SCALE;
  return fraction === 0n ? whole.toString() : `${whole}.${fraction.toString().padStart(2, "0")}`;
}

function formatMoney(scaledAmount: bigint): string {
  const whole = scaledAmount / MONEY_SCALE;
  const fraction = scaledAmount % MONEY_SCALE;
  return `${new Intl.NumberFormat("en-RW").format(whole)}${
    fraction === 0n ? "" : `.${fraction.toString().padStart(2, "0")}`
  } RWF`;
}

function formatProductPrice(scaledAmount: number): string {
  return Number.isSafeInteger(scaledAmount)
    ? formatMoney(BigInt(scaledAmount))
    : "Invalid price";
}

function lineTotal(product: PosProduct, quantity: string, sellingPrice: string): bigint | null {
  const scaledQuantity = parseQuantity(quantity, product.base_unit_allows_decimals);
  const scaledPrice = parseMoney(sellingPrice);
  if (
    scaledQuantity === null ||
    scaledQuantity === 0n ||
    !Number.isSafeInteger(product.quantity_on_hand) ||
    product.quantity_on_hand < 0 ||
    scaledQuantity > BigInt(product.quantity_on_hand) ||
    scaledPrice === null
  ) {
    return null;
  }
  return (scaledQuantity * scaledPrice + 500n) / QUANTITY_SCALE;
}

function errorText(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

export function PosPage() {
  const [businesses, setBusinesses] = useState<Business[]>([]);
  const [businessId, setBusinessId] = useState("");
  const [products, setProducts] = useState<PosProduct[]>([]);
  const [totalCount, setTotalCount] = useState(0);
  const [search, setSearch] = useState("");
  const [page, setPage] = useState(0);
  const [cart, setCart] = useState<CartItem[]>([]);
  const [paymentMethod, setPaymentMethod] = useState<PaymentMethod>("cash");
  const [completedSale, setCompletedSale] = useState<SaleReceipt | null>(null);
  const [isLoadingBusinesses, setIsLoadingBusinesses] = useState(true);
  const [isLoadingProducts, setIsLoadingProducts] = useState(false);
  const [isCheckingOut, setIsCheckingOut] = useState(false);
  const [refreshToken, setRefreshToken] = useState(0);
  const [error, setError] = useState<string | null>(null);

  const pageCount = Math.ceil(totalCount / PAGE_SIZE);
  const cartTotal = useMemo(
    () => cart.reduce(
      (total, item) => total + (lineTotal(item.product, item.quantity, item.sellingPrice) ?? 0n),
      0n,
    ),
    [cart],
  );
  const hasInvalidCartItem = cart.some((item) => {
    const price = parseMoney(item.sellingPrice);
    const listedPrice = Number.isSafeInteger(item.product.selling_price) && item.product.selling_price >= 0
      ? BigInt(item.product.selling_price)
      : null;
    const hasPriceChange = price !== null && price !== listedPrice;
    return lineTotal(item.product, item.quantity, item.sellingPrice) === null ||
      (hasPriceChange && !item.priceChangeReason.trim()) ||
      (item.belowCostReasonRequired && !item.priceChangeReason.trim());
  });

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
      setProducts([]);
      setTotalCount(0);
      return;
    }

    let active = true;
    const timer = window.setTimeout(() => {
      setIsLoadingProducts(true);
      invoke<PosProductPageData>("pos_product_search", {
        businessId,
        search: search.trim() || null,
        offset: page * PAGE_SIZE,
        limit: PAGE_SIZE,
      })
        .then((result) => {
          if (active) {
            setProducts(result.items);
            setTotalCount(result.total_count);
          }
        })
        .catch((cause: unknown) => {
          if (active) {
            setError(`Could not search products. ${errorText(cause)}`);
          }
        })
        .finally(() => {
          if (active) {
            setIsLoadingProducts(false);
          }
        });
    }, 150);
    return () => {
      active = false;
      window.clearTimeout(timer);
    };
  }, [businessId, page, search, refreshToken]);

  function addToCart(product: PosProduct) {
    setError(null);
    if (!Number.isSafeInteger(product.selling_price) || product.selling_price < 0) {
      setError(`Selling price is outside the supported range for ${product.name}.`);
      return;
    }
    if (!Number.isSafeInteger(product.quantity_on_hand) || product.quantity_on_hand < 0) {
      setError("Stock quantity is outside the supported range.");
      return;
    }
    const oneUnit = QUANTITY_SCALE;
    const existing = cart.find((item) => item.product.id === product.id);
    const currentQuantity = existing
      ? parseQuantity(existing.quantity, product.base_unit_allows_decimals)
      : 0n;
    if (currentQuantity === null || currentQuantity + oneUnit > BigInt(product.quantity_on_hand)) {
      setError(`Not enough ${product.base_unit_name} in stock for ${product.name}.`);
      return;
    }
    if (existing) {
      setCart(cart.map((item) =>
        item.product.id === product.id
          ? { ...item, product, quantity: formatScaledQuantity(currentQuantity + oneUnit) }
          : item,
      ));
      return;
    }
    setCart([...cart, {
      product,
      quantity: "1",
      sellingPrice: moneyInputValue(product.selling_price),
      priceChangeReason: "",
      belowCostReasonRequired: false,
    }]);
  }

  function updateCartItem(productId: string, patch: Partial<Pick<CartItem, "quantity" | "sellingPrice" | "priceChangeReason">>) {
    setCart((current) =>
      current.map((item) =>
        item.product.id === productId
          ? {
              ...item,
              ...patch,
              belowCostReasonRequired:
                patch.quantity !== undefined || patch.sellingPrice !== undefined
                  ? false
                  : item.belowCostReasonRequired,
            }
          : item,
      ),
    );
  }

  function removeFromCart(productId: string) {
    setCart((current) => current.filter((item) => item.product.id !== productId));
  }

  async function handleCheckout() {
    if (!businessId || cart.length === 0 || hasInvalidCartItem || isCheckingOut) {
      return;
    }

    const items: {
      productId: string;
      quantity: number;
      sellingPrice: number | null;
      priceChangeReason: string | null;
    }[] = [];
    for (const item of cart) {
      const quantity = parseQuantity(item.quantity, item.product.base_unit_allows_decimals);
      const sellingPrice = parseMoney(item.sellingPrice);
      if (quantity === null || sellingPrice === null) {
        setError(`Enter a valid quantity and selling price for ${item.product.name}.`);
        return;
      }
      if (!Number.isSafeInteger(item.product.selling_price) || item.product.selling_price < 0) {
        setError(`The listed price is outside the supported range for ${item.product.name}.`);
        return;
      }
      const priceChanged = sellingPrice !== BigInt(item.product.selling_price);
      if ((priceChanged || item.belowCostReasonRequired) && !item.priceChangeReason.trim()) {
        setError(`Enter a reason for the price change or below-cost sale of ${item.product.name}.`);
        return;
      }
      items.push({
        productId: item.product.id,
        quantity: Number(quantity),
        sellingPrice: priceChanged ? Number(sellingPrice) : null,
        priceChangeReason: item.priceChangeReason.trim() || null,
      });
    }

    setError(null);
    setIsCheckingOut(true);
    try {
      const receipt = await invoke<SaleReceipt>("pos_checkout", {
        businessId,
        items,
        paymentMethod,
      });
      setCompletedSale(receipt);
      setCart([]);
      setRefreshToken((current) => current + 1);
    } catch (cause) {
      const message = errorText(cause);
      const belowCostMatch = /BELOW_FIFO_COST:([^:]+):/.exec(message);
      if (belowCostMatch) {
        const productId = belowCostMatch[1];
        setCart((current) => current.map((item) =>
          item.product.id === productId ? { ...item, belowCostReasonRequired: true } : item,
        ));
        setError("This sale is below the actual FIFO purchase cost. Enter a reason to authorize and record the loss, then retry.");
      } else {
        setError(`Could not complete the sale. ${message}`);
      }
    } finally {
      setIsCheckingOut(false);
    }
  }

  if (isLoadingBusinesses) {
    return (
      <Card>
        <CardContent className="flex items-center gap-2 py-6 text-sm text-muted-foreground">
          <LoaderCircle className="size-4 animate-spin" />
          Loading businesses...
        </CardContent>
      </Card>
    );
  }

  if (businesses.length === 0) {
    return (
      <Card>
        <CardContent className="py-6 text-sm text-muted-foreground">
          Register a business and add products before using the point of sale.
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="mx-auto max-w-7xl space-y-6">
      <header className="flex flex-col gap-4 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="text-sm font-medium text-muted-foreground">Sales</p>
          <h1 className="text-3xl font-semibold tracking-tight">Point of Sale</h1>
          <p className="mt-2 text-muted-foreground">
            Find products, take payment, and save sales with their stock movements.
          </p>
        </div>
        <div className="w-full space-y-2 md:max-w-xs">
          <Label htmlFor="pos-business">Business</Label>
          <select
            id="pos-business"
            value={businessId}
            onChange={(event) => {
              setBusinessId(event.target.value);
              setPage(0);
              setCart([]);
              setCompletedSale(null);
            }}
            className="h-9 w-full rounded-lg border border-input bg-background px-3 text-sm focus-visible:outline-none focus-visible:ring-3 focus-visible:ring-ring/50"
          >
            {businesses.map((business) => (
              <option key={business.id} value={business.id}>{business.name}</option>
            ))}
          </select>
        </div>
      </header>

      {error ? (
        <div role="alert" className="rounded-lg border border-destructive/30 bg-destructive/5 p-4 text-sm text-destructive">
          {error}
        </div>
      ) : null}

      {completedSale ? (
        <Card>
          <CardHeader>
            <CardTitle>Sale completed — receipt</CardTitle>
            <CardDescription>
              {completedSale.transaction_number} · {new Date(completedSale.created_at).toLocaleString()}
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            <div className="divide-y rounded-lg border">
              {completedSale.items.map((item, index) => (
                <div key={`${item.sku}-${index}`} className="flex items-center justify-between gap-4 p-3">
                  <div className="min-w-0">
                    <p className="truncate font-medium">{item.product_name}</p>
                    <p className="text-sm text-muted-foreground">
                      {formatQuantity(item.quantity)} {item.unit_name} × {formatProductPrice(item.unit_price)}
                    </p>
                    {item.unit_price !== item.listed_unit_price ? (
                      <p className="text-xs text-muted-foreground">
                        Price adjusted from {formatProductPrice(item.listed_unit_price)} · Reason: {item.price_change_reason}
                      </p>
                    ) : null}
                  </div>
                  <p className="shrink-0 font-medium">{formatProductPrice(item.line_total)}</p>
                </div>
              ))}
            </div>
            <div className="flex justify-between gap-4">
              <span className="text-muted-foreground">
                Paid by {PAYMENT_METHODS.find((method) => method.value === completedSale.payment_method)?.label}
              </span>
              <span className="font-semibold">{formatProductPrice(completedSale.total)}</span>
            </div>
            <p className="text-xs text-muted-foreground">
              Ordinary sales receipt; this is not a certified EBM tax invoice.
            </p>
            <Button type="button" variant="outline" onClick={() => setCompletedSale(null)}>
              Close receipt
            </Button>
          </CardContent>
        </Card>
      ) : null}

      <div className="grid gap-6 xl:grid-cols-[minmax(0,1.5fr)_minmax(22rem,1fr)]">
        <Card>
          <CardHeader>
            <CardTitle>Find products</CardTitle>
            <CardDescription>
              Selling prices are shown per base unit. Product costs are not shown here.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="relative max-w-md">
              <Search className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
              <Input
                aria-label="Search POS products"
                className="pl-9"
                placeholder="Search name, SKU, or barcode..."
                value={search}
                onChange={(event) => {
                  setSearch(event.target.value);
                  setPage(0);
                }}
              />
            </div>
            {isLoadingProducts ? (
              <div className="flex items-center justify-center gap-2 py-10 text-sm text-muted-foreground">
                <LoaderCircle className="size-4 animate-spin" />
                Searching products...
              </div>
            ) : products.length === 0 ? (
              <p className="py-10 text-center text-sm text-muted-foreground">
                {search.trim() ? "No products match your search." : "No active products found."}
              </p>
            ) : (
              <div className="grid gap-3 sm:grid-cols-2">
                {products.map((product) => {
                  const outOfStock = product.quantity_on_hand <= 0;
                  return (
                    <div key={product.id} className="flex flex-col gap-3 rounded-lg border p-4">
                      <div className="min-w-0">
                        <h2 className="truncate font-medium">{product.name}</h2>
                        <p className="text-sm text-muted-foreground">{product.sku}</p>
                      </div>
                      <div className="flex items-end justify-between gap-3">
                        <div>
                          <p className="font-semibold">{formatProductPrice(product.selling_price)}</p>
                          <p className="text-xs text-muted-foreground">
                            per {product.base_unit_name} · {formatQuantity(product.quantity_on_hand)} available
                          </p>
                        </div>
                        <Button type="button" size="sm" disabled={outOfStock} onClick={() => addToCart(product)}>
                          <Plus />
                          Add
                        </Button>
                      </div>
                      {outOfStock ? <Badge variant="destructive" className="w-fit">Out of stock</Badge> : null}
                    </div>
                  );
                })}
              </div>
            )}
            <div className="flex items-center justify-between border-t pt-4">
              <p className="text-sm text-muted-foreground">
                {totalCount === 0 ? "No products" : `${page * PAGE_SIZE + 1}–${Math.min((page + 1) * PAGE_SIZE, totalCount)} of ${totalCount}`}
              </p>
              <div className="flex gap-2">
                <Button variant="outline" disabled={page === 0 || isLoadingProducts} onClick={() => setPage((value) => Math.max(0, value - 1))}>
                  Previous
                </Button>
                <Button variant="outline" disabled={page + 1 >= pageCount || isLoadingProducts} onClick={() => setPage((value) => value + 1)}>
                  Next
                </Button>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <ShoppingCart className="size-5" />
              Cart
            </CardTitle>
            <CardDescription>{cart.length} {cart.length === 1 ? "product" : "products"}</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            {cart.length === 0 ? (
              <p className="py-8 text-center text-sm text-muted-foreground">
                Add products to prepare a cart.
              </p>
            ) : (
              <div className="space-y-3">
                {cart.map((item) => {
                  const total = lineTotal(item.product, item.quantity, item.sellingPrice);
                  const parsedPrice = parseMoney(item.sellingPrice);
                  const listedPrice = Number.isSafeInteger(item.product.selling_price) && item.product.selling_price >= 0
                    ? BigInt(item.product.selling_price)
                    : null;
                  const priceChanged = parsedPrice !== null && parsedPrice !== listedPrice;
                  return (
                    <div key={item.product.id} className="space-y-2 rounded-lg border p-3">
                      <div className="flex items-start justify-between gap-2">
                        <div className="min-w-0">
                          <p className="truncate font-medium">{item.product.name}</p>
                          <p className="text-xs text-muted-foreground">
                            Default price: {formatProductPrice(item.product.selling_price)} per {item.product.base_unit_name}
                          </p>
                          {item.product.selling_price < item.product.minimum_selling_price ? (
                            <p className="text-xs text-amber-700">
                              Below some on-hand purchase costs ({formatProductPrice(item.product.minimum_selling_price)}); a sale that makes a FIFO loss needs a reason.
                            </p>
                          ) : null}
                        </div>
                        <Button
                          type="button"
                          variant="ghost"
                          size="icon"
                          aria-label={`Remove ${item.product.name} from cart`}
                          onClick={() => removeFromCart(item.product.id)}
                        >
                          <Trash2 />
                        </Button>
                      </div>
                      <div className="flex items-end justify-between gap-3">
                        <div className="grid flex-1 gap-3 sm:grid-cols-2">
                          <div className="space-y-1">
                            <Label htmlFor={`pos-quantity-${item.product.id}`} className="text-xs">
                              Quantity ({item.product.base_unit_name})
                            </Label>
                            <Input
                              id={`pos-quantity-${item.product.id}`}
                              type="number"
                              min={item.product.base_unit_allows_decimals ? "0.001" : "1"}
                              max={formatQuantity(item.product.quantity_on_hand)}
                              step={item.product.base_unit_allows_decimals ? "0.001" : "1"}
                              value={item.quantity}
                              onChange={(event) => updateCartItem(item.product.id, { quantity: event.target.value })}
                            />
                          </div>
                          <div className="space-y-1">
                            <Label htmlFor={`pos-price-${item.product.id}`} className="text-xs">
                              Selling price per {item.product.base_unit_name} (RWF)
                            </Label>
                            <Input
                              id={`pos-price-${item.product.id}`}
                              type="number"
                              min="0"
                              step="0.01"
                              value={item.sellingPrice}
                              onChange={(event) => updateCartItem(item.product.id, { sellingPrice: event.target.value })}
                            />
                          </div>
                        </div>
                        {priceChanged || item.belowCostReasonRequired ? (
                          <div className="space-y-1">
                            <Label htmlFor={`pos-price-reason-${item.product.id}`} className="text-xs">
                              Reason for price change or below-cost sale (required)
                            </Label>
                            <Input
                              id={`pos-price-reason-${item.product.id}`}
                              maxLength={500}
                              required
                              value={item.priceChangeReason}
                              onChange={(event) => updateCartItem(item.product.id, { priceChangeReason: event.target.value })}
                              placeholder="e.g. Customer negotiated"
                            />
                          </div>
                        ) : null}
                        <p className="text-right font-semibold">
                          {total === null ? "Check quantity / price / reason" : formatMoney(total)}
                        </p>
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
            <div className="space-y-3 border-t pt-4">
              <div className="flex items-center justify-between text-lg font-semibold">
                <span>Total</span>
                <span>{hasInvalidCartItem ? "—" : formatMoney(cartTotal)}</span>
              </div>
              <div className="space-y-2">
                <Label htmlFor="pos-payment-method">Payment method</Label>
                <select
                  id="pos-payment-method"
                  value={paymentMethod}
                  onChange={(event) => {
                    const selected = PAYMENT_METHODS.find((method) => method.value === event.target.value);
                    if (selected) {
                      setPaymentMethod(selected.value);
                    }
                  }}
                  className="h-9 w-full rounded-lg border border-input bg-background px-3 text-sm focus-visible:outline-none focus-visible:ring-3 focus-visible:ring-ring/50"
                >
                  {PAYMENT_METHODS.map((method) => (
                    <option key={method.value} value={method.value}>{method.label}</option>
                  ))}
                </select>
              </div>
              <Button
                className="w-full"
                disabled={cart.length === 0 || hasInvalidCartItem || isCheckingOut}
                onClick={handleCheckout}
              >
                {isCheckingOut ? <LoaderCircle className="animate-spin" /> : <ShoppingCart />}
                {isCheckingOut ? "Completing sale..." : "Complete sale"}
              </Button>
              <p className="text-xs text-muted-foreground">
                Sale, payment, FIFO purchase cost, receipt, and stock deduction are saved together. A sale below actual FIFO cost requires a recorded reason.
              </p>
              {hasInvalidCartItem ? (
                <p role="alert" className="text-xs text-destructive">
                  Check each quantity and price; any changed price needs a reason before checkout.
                </p>
              ) : null}
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
