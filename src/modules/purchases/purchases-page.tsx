import { invoke } from "@tauri-apps/api/core";
import { useEffect, useMemo, useState, type FormEvent } from "react";
import { AlertTriangle, CreditCard, LoaderCircle, PackagePlus, ReceiptText, Truck } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { PageHeader } from "@/components/shared/page-header";
import { SelectField } from "@/components/shared/select-field";
import {
  accountsForPaymentMethod,
  loadCashAccountPaymentOptions,
  type CashAccountPaymentOption,
  type PaymentMethod,
} from "@/modules/cash-accounts/cash-account-payment-options";

const QUANTITY_SCALE = 1000n;
const MONEY_SCALE = 100n;
const MAX_SAFE_INTEGER = BigInt(Number.MAX_SAFE_INTEGER);
const PAYMENT_METHODS: { value: PaymentMethod; label: string }[] = [
  { value: "cash", label: "Cash" },
  { value: "mtn_mobile_money", label: "MTN Mobile Money" },
  { value: "airtel_money", label: "Airtel Money" },
  { value: "bank_transfer", label: "Bank transfer" },
];

interface Business {
  id: string;
  name: string;
}

interface Supplier {
  id: string;
  name: string;
  is_active: boolean;
}

interface PurchaseProduct {
  id: string;
  name: string;
  sku: string;
  base_unit_name: string;
  purchase_price: number;
  base_unit_allows_decimals: boolean;
}

interface PurchaseItem {
  id: string;
  product_id: string;
  product_name: string;
  sku: string;
  unit_name: string;
  ordered_quantity: number;
  received_quantity: number;
  unit_cost: number;
  line_total: number;
  base_unit_allows_decimals: boolean;
}

interface Purchase {
  id: string;
  supplier_id: string;
  supplier_name: string;
  supplier_invoice_number: string;
  status: "pending" | "partially_received" | "received";
  subtotal: number;
  amount_paid: number;
  balance_due: number;
  duplicate_authorized: boolean;
  duplicate_of_purchase_id: string | null;
  created_at: string;
  items: PurchaseItem[];
  deliveries: {
    id: string;
    delivery_reference: string | null;
    notes: string | null;
    total_cost: number;
    created_at: string;
    items: {
      product_name: string;
      quantity: number;
      unit_name: string;
      total_cost: number;
    }[];
  }[];
  payments: {
    id: string;
    payment_method: PaymentMethod;
    amount: number;
    created_at: string;
  }[];
}

interface PurchaseReceipt {
  id: string;
  purchase_id: string;
  status: Purchase["status"];
  delivery_reference: string | null;
  total_cost: number;
  created_at: string;
}

interface DraftLine {
  product_id: string;
  quantity: string;
  unit_cost: string;
}

function errorText(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

function parseQuantity(value: string, allowsDecimals: boolean): number | null {
  const pattern = allowsDecimals ? /^\d+(?:\.\d{1,3})?$/ : /^\d+$/;
  if (!pattern.test(value)) {
    return null;
  }
  const [whole, fraction = ""] = value.split(".");
  const scaled = BigInt(whole) * QUANTITY_SCALE + BigInt(fraction.padEnd(3, "0") || "0");
  return scaled > 0n && scaled <= MAX_SAFE_INTEGER ? Number(scaled) : null;
}

function parseUnitCost(value: string): number | null {
  if (!/^\d+$/.test(value)) {
    return null;
  }
  const scaled = BigInt(value) * MONEY_SCALE;
  return scaled <= MAX_SAFE_INTEGER ? Number(scaled) : null;
}

function parsePaymentAmount(value: string): number | null {
  const match = /^(\d+)(?:\.(\d{1,2}))?$/.exec(value);
  if (!match) {
    return null;
  }
  const fraction = (match[2] ?? "").padEnd(2, "0");
  const amount = BigInt(match[1]) * MONEY_SCALE + BigInt(fraction || "0");
  return amount > 0n && amount <= MAX_SAFE_INTEGER ? Number(amount) : null;
}

function moneyInputValue(scaledAmount: number): string {
  const amount = BigInt(scaledAmount);
  const whole = amount / MONEY_SCALE;
  const fraction = amount % MONEY_SCALE;
  return `${whole}.${fraction.toString().padStart(2, "0")}`;
}

function formatQuantity(quantity: number): string {
  if (!Number.isSafeInteger(quantity)) {
    return "Invalid quantity";
  }
  const scaled = BigInt(quantity);
  const whole = scaled / QUANTITY_SCALE;
  const fraction = scaled % QUANTITY_SCALE;
  return fraction === 0n
    ? new Intl.NumberFormat("en-RW").format(whole)
    : `${new Intl.NumberFormat("en-RW").format(whole)}.${fraction.toString().padStart(3, "0").replace(/0+$/, "")}`;
}

function formatMoney(scaledAmount: number): string {
  if (!Number.isSafeInteger(scaledAmount) || scaledAmount < 0) {
    return "Invalid amount";
  }
  const amount = BigInt(scaledAmount);
  const whole = amount / MONEY_SCALE;
  const fraction = amount % MONEY_SCALE;
  return `${new Intl.NumberFormat("en-RW").format(whole)}${
    fraction === 0n ? "" : `.${fraction.toString().padStart(2, "0")}`
  } RWF`;
}

function lineTotal(quantity: string, unitCost: string, allowsDecimals: boolean): bigint | null {
  const scaledQuantity = parseQuantity(quantity, allowsDecimals);
  const scaledUnitCost = parseUnitCost(unitCost);
  if (scaledQuantity === null || scaledUnitCost === null) {
    return null;
  }
  return (BigInt(scaledQuantity) * BigInt(scaledUnitCost) + 500n) / QUANTITY_SCALE;
}

function statusLabel(status: Purchase["status"]): string {
  switch (status) {
    case "pending":
      return "Awaiting delivery";
    case "partially_received":
      return "Partially received";
    case "received":
      return "Fully received";
  }
}

function statusVariant(status: Purchase["status"]): "outline" | "secondary" | "default" {
  switch (status) {
    case "pending":
      return "outline";
    case "partially_received":
      return "secondary";
    case "received":
      return "default";
  }
}

function paymentMethodLabel(method: PaymentMethod): string {
  return PAYMENT_METHODS.find((item) => item.value === method)?.label ?? method;
}

export function PurchasesPage() {
  const [businesses, setBusinesses] = useState<Business[]>([]);
  const [businessId, setBusinessId] = useState("");
  const [suppliers, setSuppliers] = useState<Supplier[]>([]);
  const [products, setProducts] = useState<PurchaseProduct[]>([]);
  const [purchases, setPurchases] = useState<Purchase[]>([]);
  const [cashAccounts, setCashAccounts] = useState<CashAccountPaymentOption[]>([]);
  const [supplierId, setSupplierId] = useState("");
  const [invoiceNumber, setInvoiceNumber] = useState("");
  const [draftLines, setDraftLines] = useState<DraftLine[]>([]);
  const [draftProductId, setDraftProductId] = useState("");
  const [draftQuantity, setDraftQuantity] = useState("");
  const [draftUnitCost, setDraftUnitCost] = useState("");
  const [receiveQuantities, setReceiveQuantities] = useState<Record<string, string>>({});
  const [deliveryReferences, setDeliveryReferences] = useState<Record<string, string>>({});
  const [receiveNotes, setReceiveNotes] = useState<Record<string, string>>({});
  const [paymentAmounts, setPaymentAmounts] = useState<Record<string, string>>({});
  const [paymentMethods, setPaymentMethods] = useState<Record<string, PaymentMethod>>({});
  const [paymentAccountIds, setPaymentAccountIds] = useState<Record<string, string>>({});
  const [duplicateWarning, setDuplicateWarning] = useState<string | null>(null);
  const [duplicateAuthorizationRequested, setDuplicateAuthorizationRequested] = useState(false);
  const [isLoadingBusinesses, setIsLoadingBusinesses] = useState(true);
  const [isLoadingData, setIsLoadingData] = useState(false);
  const [isLoadingCashAccounts, setIsLoadingCashAccounts] = useState(false);
  const [isSavingPurchase, setIsSavingPurchase] = useState(false);
  const [receivingPurchaseId, setReceivingPurchaseId] = useState<string | null>(null);
  const [payingPurchaseId, setPayingPurchaseId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const selectedProduct = products.find((product) => product.id === draftProductId);
  const draftSubtotal = useMemo(
    () => draftLines.reduce((sum, line) => {
      const product = products.find((candidate) => candidate.id === line.product_id);
      if (!product) {
        return sum;
      }
      return sum + (lineTotal(line.quantity, line.unit_cost, product.base_unit_allows_decimals) ?? 0n);
    }, 0n),
    [draftLines, products],
  );
  const validDraft = draftLines.length > 0 && draftLines.every((line) => {
    const product = products.find((candidate) => candidate.id === line.product_id);
    return product !== undefined &&
      parseQuantity(line.quantity, product.base_unit_allows_decimals) !== null &&
      parseUnitCost(line.unit_cost) !== null;
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
      setSuppliers([]);
      setProducts([]);
      setPurchases([]);
      setSupplierId("");
      setCashAccounts([]);
      return;
    }

    let active = true;
    setIsLoadingData(true);
    setError(null);
    Promise.all([
      invoke<Supplier[]>("supplier_list", { businessId }),
      invoke<PurchaseProduct[]>("purchase_product_list", { businessId }),
      invoke<Purchase[]>("purchase_list", { businessId }),
      loadCashAccountPaymentOptions(businessId),
    ])
      .then(([supplierResult, productResult, purchaseResult, accountResult]) => {
        if (active) {
          const activeSuppliers = supplierResult.filter((supplier) => supplier.is_active);
          setSuppliers(activeSuppliers);
          setSupplierId((current) =>
            activeSuppliers.some((supplier) => supplier.id === current)
              ? current
              : (activeSuppliers[0]?.id ?? ""),
          );
          setProducts(productResult);
          setPurchases(purchaseResult);
          setCashAccounts(accountResult);
        }
      })
      .catch((cause: unknown) => {
        if (active) {
          setError(`Could not load purchasing data. ${errorText(cause)}`);
        }
      })
      .finally(() => {
        if (active) {
          setIsLoadingData(false);
          setIsLoadingCashAccounts(false);
        }
      });
    setIsLoadingCashAccounts(true);
    return () => {
      active = false;
    };
  }, [businessId]);

  function resetDraft() {
    setInvoiceNumber("");
    setDraftLines([]);
    setDraftProductId("");
    setDraftQuantity("");
    setDraftUnitCost("");
    setDuplicateWarning(null);
    setDuplicateAuthorizationRequested(false);
  }

  function addDraftLine() {
    if (!selectedProduct) {
      setError("Select a product to add to the purchase invoice.");
      return;
    }
    if (draftLines.some((line) => line.product_id === selectedProduct.id)) {
      setError("Each product can only be added once per invoice.");
      return;
    }
    if (
      parseQuantity(draftQuantity, selectedProduct.base_unit_allows_decimals) === null ||
      parseUnitCost(draftUnitCost) === null
    ) {
      setError("Enter a valid ordered quantity and whole-RWF cost per base unit.");
      return;
    }
    setError(null);
    setDuplicateAuthorizationRequested(false);
    setDraftLines((current) => [
      ...current,
      {
        product_id: selectedProduct.id,
        quantity: draftQuantity,
        unit_cost: draftUnitCost,
      },
    ]);
    setDraftProductId("");
    setDraftQuantity("");
    setDraftUnitCost("");
  }

  async function savePurchase(authorizeDuplicate: boolean) {
    if (!businessId || !supplierId || !invoiceNumber.trim() || !validDraft || isSavingPurchase) {
      return;
    }

    setError(null);
    setIsSavingPurchase(true);
    try {
      const purchase = await invoke<Purchase>("purchase_create", {
        businessId,
        supplierId,
        supplierInvoiceNumber: invoiceNumber.trim(),
        authorizeDuplicate,
        items: draftLines.map((line) => {
          const product = products.find((candidate) => candidate.id === line.product_id);
          const quantity = product ? parseQuantity(line.quantity, product.base_unit_allows_decimals) : null;
          const unitCost = parseUnitCost(line.unit_cost);
          if (quantity === null || unitCost === null) {
            throw new Error("A purchase line has invalid quantity or unit cost.");
          }
          return {
            productId: line.product_id,
            orderedQuantity: quantity,
            unitCost,
          };
        }),
      });
      setPurchases((current) => [purchase, ...current]);
      resetDraft();
    } catch (cause) {
      const message = errorText(cause);
      if (message.includes("DUPLICATE_PURCHASE_INVOICE:")) {
        setDuplicateWarning(
          "This supplier already has a purchase with that invoice number. Confirm only if this is genuinely a separate duplicate invoice.",
        );
      } else {
        setError(`Could not save the purchase invoice. ${message}`);
      }
    } finally {
      setIsSavingPurchase(false);
    }
  }

  async function handleCreatePurchase(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    await savePurchase(duplicateAuthorizationRequested);
  }

  async function handleReceiveDelivery(event: FormEvent<HTMLFormElement>, purchase: Purchase) {
    event.preventDefault();
    if (!businessId || receivingPurchaseId) {
      return;
    }
    const items: { purchaseItemId: string; quantity: number }[] = [];
    for (const item of purchase.items) {
      const value = receiveQuantities[item.id]?.trim();
      if (!value) {
        continue;
      }
      const quantity = parseQuantity(value, item.base_unit_allows_decimals);
      const remaining = item.ordered_quantity - item.received_quantity;
      if (quantity === null || quantity > remaining) {
        setError(`Enter a valid quantity up to ${formatQuantity(remaining)} for ${item.product_name}.`);
        return;
      }
      items.push({ purchaseItemId: item.id, quantity });
    }
    if (items.length === 0) {
      setError("Enter at least one delivered quantity.");
      return;
    }

    setError(null);
    setReceivingPurchaseId(purchase.id);
    try {
      await invoke<PurchaseReceipt>("purchase_receive_delivery", {
        businessId,
        purchaseId: purchase.id,
        items,
        deliveryReference: deliveryReferences[purchase.id]?.trim() || null,
        notes: receiveNotes[purchase.id]?.trim() || null,
      });
      setReceiveQuantities((current) => {
        const next = { ...current };
        for (const item of purchase.items) {
          delete next[item.id];
        }
        return next;
      });
      setDeliveryReferences((current) => ({ ...current, [purchase.id]: "" }));
      setReceiveNotes((current) => ({ ...current, [purchase.id]: "" }));
      try {
        const updated = await invoke<Purchase[]>("purchase_list", { businessId });
        setPurchases(updated);
      } catch (cause) {
        setError(`Delivery was recorded, but purchase history could not be refreshed. ${errorText(cause)}`);
      }
    } catch (cause) {
      setError(`Could not receive this delivery. ${errorText(cause)}`);
    } finally {
      setReceivingPurchaseId(null);
    }
  }

  async function handleRecordPayment(event: FormEvent<HTMLFormElement>, purchase: Purchase) {
    event.preventDefault();
    if (!businessId || payingPurchaseId) {
      return;
    }
    const amount = parsePaymentAmount(paymentAmounts[purchase.id]?.trim() ?? "");
    if (amount === null || amount > purchase.balance_due) {
      setError(`Enter a valid payment up to ${formatMoney(purchase.balance_due)}.`);
      return;
    }
    const paymentMethod = paymentMethods[purchase.id] ?? "cash";
    const accounts = accountsForPaymentMethod(cashAccounts, paymentMethod);
    const accountId = accounts.some((account) => account.id === paymentAccountIds[purchase.id])
      ? paymentAccountIds[purchase.id]
      : accounts[0]?.id;
    if (!accountId) {
      setError("Ask the Owner/Admin to set up an account and authorize its opening balance before paying this supplier.");
      return;
    }

    setError(null);
    setPayingPurchaseId(purchase.id);
    try {
      const updated = await invoke<Purchase>("purchase_record_payment", {
        businessId,
        purchaseId: purchase.id,
        amount,
        paymentMethod,
        accountId,
      });
      setPurchases((current) => current.map((item) => item.id === purchase.id ? updated : item));
      setPaymentAmounts((current) => ({ ...current, [purchase.id]: "" }));
      try {
        setCashAccounts(await loadCashAccountPaymentOptions(businessId));
      } catch (cause) {
        setError(`Supplier payment was recorded, but account balances could not be refreshed. ${errorText(cause)}`);
      }
    } catch (cause) {
      setError(`Could not record the supplier payment. ${errorText(cause)}`);
    } finally {
      setPayingPurchaseId(null);
    }
  }

  const pageHeader = (
    <PageHeader
      eyebrow="Purchasing"
      title="Purchase invoices"
      description="Record supplier invoices, then receive stock as each delivery arrives."
      actions={businesses.length > 0 ? (
        <div className="w-full space-y-1.5 sm:w-56">
          <Label htmlFor="purchase-business">Business</Label>
          <SelectField
            id="purchase-business"
            value={businessId}
            onChange={(event) => {
              setBusinessId(event.target.value);
              resetDraft();
            }}
          >
            {businesses.map((business) => (
              <option key={business.id} value={business.id}>{business.name}</option>
            ))}
          </SelectField>
        </div>
      ) : undefined}
    />
  );

  if (isLoadingBusinesses) {
    return (
      <div className="mx-auto max-w-7xl space-y-6">
        {pageHeader}
        <Card>
          <CardContent className="flex items-center gap-2 py-6 text-sm text-muted-foreground">
            <LoaderCircle className="size-4 animate-spin" />
            Loading businesses...
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-7xl space-y-6">
      {pageHeader}

      {error ? (
        <div role="alert" className="rounded-lg border border-destructive/30 bg-destructive/5 p-4 text-sm text-destructive">
          {error}
        </div>
      ) : null}

      {businesses.length === 0 ? (
        <Card>
          <CardContent className="py-6 text-sm text-muted-foreground">
            Register a business before recording purchases.
          </CardContent>
        </Card>
      ) : (
        <>
          <Card>
            <CardHeader>
              <CardTitle>Record purchase invoice</CardTitle>
              <CardDescription>
                Invoice lines use product base units. Stock is not increased until a delivery is received.
              </CardDescription>
            </CardHeader>
            <CardContent>
              {suppliers.length === 0 ? (
                <p className="text-sm text-muted-foreground">
                  Add an active supplier before recording a purchase.
                </p>
              ) : products.length === 0 ? (
                <p className="text-sm text-muted-foreground">
                  Add active products before recording a purchase.
                </p>
              ) : (
                <form className="space-y-5" onSubmit={handleCreatePurchase}>
                  <div className="grid gap-4 md:grid-cols-2">
                    <div className="space-y-2">
                      <Label htmlFor="purchase-supplier">Supplier</Label>
                      <SelectField
                        id="purchase-supplier"
                        required
                        value={supplierId}
                        onChange={(event) => {
                          setSupplierId(event.target.value);
                          setDuplicateWarning(null);
                          setDuplicateAuthorizationRequested(false);
                        }}
                      >
                        {suppliers.map((supplier) => (
                          <option key={supplier.id} value={supplier.id}>{supplier.name}</option>
                        ))}
                      </SelectField>
                    </div>
                    <div className="space-y-2">
                      <Label htmlFor="purchase-invoice-number">Supplier invoice number</Label>
                      <Input
                        id="purchase-invoice-number"
                        required
                        maxLength={200}
                        value={invoiceNumber}
                        onChange={(event) => {
                          setInvoiceNumber(event.target.value);
                          setDuplicateWarning(null);
                          setDuplicateAuthorizationRequested(false);
                        }}
                      />
                    </div>
                  </div>

                  <div className="grid items-end gap-3 rounded-lg border p-4 md:grid-cols-2 xl:grid-cols-[minmax(12rem,1.5fr)_1fr_1fr_auto]">
                    <div className="space-y-2">
                      <Label htmlFor="purchase-product">Product</Label>
                      <SelectField
                        id="purchase-product"
                        value={draftProductId}
                        onChange={(event) => {
                          const product = products.find((item) => item.id === event.target.value);
                          setDraftProductId(event.target.value);
                          setDraftUnitCost(product ? String(Math.floor(product.purchase_price / 100)) : "");
                        }}
                      >
                        <option value="">Select product</option>
                        {products.map((product) => (
                          <option key={product.id} value={product.id}>
                            {product.name} ({product.sku})
                          </option>
                        ))}
                      </SelectField>
                    </div>
                    <div className="space-y-2">
                      <Label htmlFor="purchase-quantity">
                        Ordered quantity{selectedProduct ? ` (${selectedProduct.base_unit_name})` : ""}
                      </Label>
                      <Input
                        id="purchase-quantity"
                        type="number"
                        min={selectedProduct?.base_unit_allows_decimals ? "0.001" : "1"}
                        step={selectedProduct?.base_unit_allows_decimals ? "0.001" : "1"}
                        value={draftQuantity}
                        onChange={(event) => setDraftQuantity(event.target.value)}
                      />
                    </div>
                    <div className="space-y-2">
                      <Label htmlFor="purchase-unit-cost">Cost per base unit (RWF)</Label>
                      <Input
                        id="purchase-unit-cost"
                        type="number"
                        min="0"
                        step="1"
                        value={draftUnitCost}
                        onChange={(event) => setDraftUnitCost(event.target.value)}
                      />
                    </div>
                    <Button type="button" variant="outline" onClick={addDraftLine}>
                      <PackagePlus />
                      Add line
                    </Button>
                  </div>

                  {draftLines.length > 0 ? (
                    <div className="space-y-2">
                      <h3 className="font-medium">Invoice lines</h3>
                      <ul className="divide-y rounded-lg border px-4">
                        {draftLines.map((line) => {
                          const product = products.find((item) => item.id === line.product_id);
                          if (!product) {
                            return null;
                          }
                          const total = lineTotal(line.quantity, line.unit_cost, product.base_unit_allows_decimals);
                          return (
                            <li key={line.product_id} className="flex items-center justify-between gap-4 py-3">
                              <div className="min-w-0">
                                <p className="truncate font-medium">{product.name}</p>
                                <p className="text-sm text-muted-foreground">
                                  {line.quantity} {product.base_unit_name} × {line.unit_cost} RWF
                                </p>
                              </div>
                              <div className="flex items-center gap-3">
                                <span className="font-medium">{total === null ? "Invalid line" : formatMoney(Number(total))}</span>
                                <Button
                                  type="button"
                                  variant="outline"
                                  size="sm"
                                  onClick={() => {
                                    setDuplicateAuthorizationRequested(false);
                                    setDraftLines((current) => current.filter((entry) => entry.product_id !== line.product_id));
                                  }}
                                >
                                  Remove
                                </Button>
                              </div>
                            </li>
                          );
                        })}
                      </ul>
                      <div className="flex justify-end gap-4 text-lg font-semibold">
                        <span>Invoice subtotal</span>
                        <span>{formatMoney(Number(draftSubtotal))}</span>
                      </div>
                    </div>
                  ) : null}

                  {duplicateWarning ? (
                    <div role="alert" className="space-y-3 rounded-lg border border-amber-500/40 bg-amber-500/5 p-4 text-sm">
                      <p className="flex items-start gap-2">
                        <AlertTriangle className="mt-0.5 size-4 shrink-0 text-amber-600" />
                        {duplicateWarning}
                      </p>
                      <label className="flex items-center gap-2">
                        <input
                          type="checkbox"
                          checked={duplicateAuthorizationRequested}
                          onChange={(event) => setDuplicateAuthorizationRequested(event.target.checked)}
                        />
                        I confirm that this duplicate invoice is intentional.
                      </label>
                    </div>
                  ) : null}

                  <Button
                    type={duplicateAuthorizationRequested ? "button" : "submit"}
                    disabled={isSavingPurchase || !supplierId || !invoiceNumber.trim() || !validDraft ||
                      (duplicateWarning !== null && !duplicateAuthorizationRequested)}
                    onClick={duplicateAuthorizationRequested ? () => void savePurchase(true) : undefined}
                  >
                    {isSavingPurchase ? <LoaderCircle className="animate-spin" /> : <ReceiptText />}
                    {isSavingPurchase
                      ? "Saving invoice..."
                      : duplicateAuthorizationRequested
                        ? "Authorize duplicate and save"
                        : "Save purchase invoice"}
                  </Button>
                </form>
              )}
            </CardContent>
          </Card>

          <section className="space-y-4">
            <div>
              <h2 className="text-xl font-semibold">Purchase history</h2>
              <p className="text-sm text-muted-foreground">
                Receiving a delivery updates stock only for the quantities recorded in that delivery.
              </p>
            </div>
            {isLoadingData ? (
              <Card>
                <CardContent className="flex items-center gap-2 py-6 text-sm text-muted-foreground">
                  <LoaderCircle className="size-4 animate-spin" />
                  Loading purchases...
                </CardContent>
              </Card>
            ) : purchases.length === 0 ? (
              <Card>
                <CardContent className="py-6 text-sm text-muted-foreground">
                  No supplier invoices have been recorded for this business.
                </CardContent>
              </Card>
            ) : (
              purchases.map((purchase) => {
                const outstandingItems = purchase.items.filter(
                  (item) => item.received_quantity < item.ordered_quantity,
                );
                return (
                  <Card key={purchase.id}>
                    <CardHeader>
                      <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
                        <div>
                          <CardTitle className="flex flex-wrap items-center gap-2 text-lg">
                            {purchase.supplier_name}
                            <Badge variant={statusVariant(purchase.status)}>{statusLabel(purchase.status)}</Badge>
                            {purchase.duplicate_authorized ? (
                              <Badge variant="outline">Authorized duplicate</Badge>
                            ) : null}
                          </CardTitle>
                          <CardDescription className="mt-1">
                            Invoice {purchase.supplier_invoice_number} · {new Date(purchase.created_at).toLocaleString()}
                          </CardDescription>
                        </div>
                        <div className="text-sm sm:text-right">
                          <p className="font-semibold">Invoice: {formatMoney(purchase.subtotal)}</p>
                          <p className="text-muted-foreground">Paid: {formatMoney(purchase.amount_paid)}</p>
                          <p className="font-medium">Balance due: {formatMoney(purchase.balance_due)}</p>
                        </div>
                      </div>
                      {purchase.duplicate_of_purchase_id ? (
                        <p className="text-sm text-amber-700">
                          Duplicate was explicitly authorized; original invoice record is retained.
                        </p>
                      ) : null}
                    </CardHeader>
                    <CardContent className="space-y-4">
                      {purchase.balance_due > 0 ? (
                        <form
                          className="grid gap-3 rounded-lg border p-4 sm:grid-cols-2 xl:grid-cols-[1fr_1fr_1fr_auto]"
                          onSubmit={(event) => void handleRecordPayment(event, purchase)}
                        >
                          <div className="space-y-2">
                            <Label htmlFor={`payment-amount-${purchase.id}`}>Payment amount (RWF)</Label>
                            <Input
                              id={`payment-amount-${purchase.id}`}
                              type="number"
                              min="0.01"
                              max={moneyInputValue(purchase.balance_due)}
                              step="0.01"
                              value={paymentAmounts[purchase.id] ?? ""}
                              onChange={(event) => setPaymentAmounts((current) => ({
                                ...current,
                                [purchase.id]: event.target.value,
                              }))}
                            />
                          </div>
                          <div className="space-y-2">
                            <Label htmlFor={`payment-method-${purchase.id}`}>Payment method</Label>
                            <SelectField
                              id={`payment-method-${purchase.id}`}
                              value={paymentMethods[purchase.id] ?? "cash"}
                              onChange={(event) => {
                                const method = PAYMENT_METHODS.find((item) => item.value === event.target.value)?.value;
                                if (method) {
                                  setPaymentMethods((current) => ({ ...current, [purchase.id]: method }));
                                  setPaymentAccountIds((current) => ({
                                    ...current,
                                    [purchase.id]: accountsForPaymentMethod(cashAccounts, method)[0]?.id ?? "",
                                  }));
                                }
                              }}
                            >
                              {PAYMENT_METHODS.map((method) => (
                                <option key={method.value} value={method.value}>{method.label}</option>
                              ))}
                            </SelectField>
                          </div>
                          <div className="space-y-2">
                            <Label htmlFor={`payment-account-${purchase.id}`}>Pay from account</Label>
                            <SelectField
                              id={`payment-account-${purchase.id}`}
                              value={(() => {
                                const options = accountsForPaymentMethod(
                                  cashAccounts,
                                  paymentMethods[purchase.id] ?? "cash",
                                );
                                return options.some((account) => account.id === paymentAccountIds[purchase.id])
                                  ? paymentAccountIds[purchase.id]
                                  : options[0]?.id ?? "";
                              })()}
                              disabled={isLoadingCashAccounts}
                              onChange={(event) => setPaymentAccountIds((current) => ({
                                ...current,
                                [purchase.id]: event.target.value,
                              }))}
                            >
                              <option value="">Select opened account</option>
                              {accountsForPaymentMethod(
                                cashAccounts,
                                paymentMethods[purchase.id] ?? "cash",
                              ).map((account) => (
                                <option key={account.id} value={account.id}>
                                  {account.name} · Available {formatMoney(account.availableBalance)}
                                </option>
                              ))}
                            </SelectField>
                            {accountsForPaymentMethod(
                              cashAccounts,
                              paymentMethods[purchase.id] ?? "cash",
                            ).length === 0 ? (
                              <p className="text-xs text-amber-700">
                                An authorized opening balance is required before payment.
                              </p>
                            ) : null}
                          </div>
                          <Button type="submit" className="self-end" disabled={payingPurchaseId !== null || isLoadingCashAccounts}>
                            {payingPurchaseId === purchase.id
                              ? <LoaderCircle className="animate-spin" />
                              : <CreditCard />}
                            {payingPurchaseId === purchase.id ? "Recording..." : "Record payment"}
                          </Button>
                        </form>
                      ) : (
                        <Badge variant="secondary">Invoice fully paid</Badge>
                      )}
                      {purchase.payments.length > 0 ? (
                        <div className="space-y-2">
                          <h3 className="text-sm font-medium">Payment history</h3>
                          <ul className="divide-y rounded-lg border px-4">
                            {purchase.payments.map((payment) => (
                              <li key={payment.id} className="flex flex-wrap items-center justify-between gap-2 py-3 text-sm">
                                <span>{paymentMethodLabel(payment.payment_method)} · {new Date(payment.created_at).toLocaleString()}</span>
                                <span className="font-medium">{formatMoney(payment.amount)}</span>
                              </li>
                            ))}
                          </ul>
                        </div>
                      ) : null}
                      <ul className="divide-y rounded-lg border px-4">
                        {purchase.items.map((item) => {
                          const remaining = item.ordered_quantity - item.received_quantity;
                          return (
                            <li key={item.id} className="flex flex-col gap-2 py-3 sm:flex-row sm:items-center sm:justify-between">
                              <div className="min-w-0">
                                <p className="truncate font-medium">{item.product_name} <span className="text-sm text-muted-foreground">({item.sku})</span></p>
                                <p className="text-sm text-muted-foreground">
                                  {formatQuantity(item.received_quantity)} of {formatQuantity(item.ordered_quantity)} {item.unit_name} received · {formatMoney(item.unit_cost)} per unit
                                </p>
                              </div>
                              {remaining > 0 ? (
                                <div className="w-full space-y-1 sm:max-w-48">
                                  <Label htmlFor={`receive-${item.id}`} className="text-xs">
                                    Receive now (up to {formatQuantity(remaining)})
                                  </Label>
                                  <Input
                                    id={`receive-${item.id}`}
                                    type="number"
                                    min={item.base_unit_allows_decimals ? "0.001" : "1"}
                                    max={formatQuantity(remaining)}
                                    step={item.base_unit_allows_decimals ? "0.001" : "1"}
                                    value={receiveQuantities[item.id] ?? ""}
                                    onChange={(event) => setReceiveQuantities((current) => ({
                                      ...current,
                                      [item.id]: event.target.value,
                                    }))}
                                  />
                                </div>
                              ) : (
                                <Badge variant="secondary">Complete</Badge>
                              )}
                            </li>
                          );
                        })}
                      </ul>
                      {purchase.deliveries.length > 0 ? (
                        <div className="space-y-2">
                          <h3 className="text-sm font-medium">Delivery history</h3>
                          <ul className="divide-y rounded-lg border px-4">
                            {purchase.deliveries.map((delivery) => (
                              <li key={delivery.id} className="space-y-1 py-3">
                                <div className="flex flex-wrap items-center justify-between gap-2">
                                  <p className="font-medium">
                                    {delivery.delivery_reference
                                      ? `Delivery note ${delivery.delivery_reference}`
                                      : "Delivery received"}
                                  </p>
                                  <span className="text-sm font-medium">{formatMoney(delivery.total_cost)}</span>
                                </div>
                                <p className="text-xs text-muted-foreground">
                                  {new Date(delivery.created_at).toLocaleString()}
                                  {delivery.notes ? ` · ${delivery.notes}` : ""}
                                </p>
                                <ul className="text-sm text-muted-foreground">
                                  {delivery.items.map((item, index) => (
                                    <li key={`${delivery.id}-${index}`}>
                                      {item.product_name}: {formatQuantity(item.quantity)} {item.unit_name}
                                    </li>
                                  ))}
                                </ul>
                              </li>
                            ))}
                          </ul>
                        </div>
                      ) : null}
                      {outstandingItems.length > 0 ? (
                        <form className="grid gap-3 md:grid-cols-2" onSubmit={(event) => void handleReceiveDelivery(event, purchase)}>
                          <div className="space-y-2">
                            <Label htmlFor={`delivery-reference-${purchase.id}`}>Delivery note reference (optional)</Label>
                            <Input
                              id={`delivery-reference-${purchase.id}`}
                              maxLength={200}
                              value={deliveryReferences[purchase.id] ?? ""}
                              onChange={(event) => setDeliveryReferences((current) => ({
                                ...current,
                                [purchase.id]: event.target.value,
                              }))}
                              placeholder="Delivery note number"
                            />
                          </div>
                          <div className="space-y-2">
                            <Label htmlFor={`delivery-notes-${purchase.id}`}>Receiving notes (optional)</Label>
                            <Input
                              id={`delivery-notes-${purchase.id}`}
                              maxLength={500}
                              value={receiveNotes[purchase.id] ?? ""}
                              onChange={(event) => setReceiveNotes((current) => ({
                                ...current,
                                [purchase.id]: event.target.value,
                              }))}
                            />
                          </div>
                          <Button type="submit" className="md:col-span-2" disabled={receivingPurchaseId !== null}>
                            {receivingPurchaseId === purchase.id
                              ? <LoaderCircle className="animate-spin" />
                              : <Truck />}
                            {receivingPurchaseId === purchase.id ? "Recording delivery..." : "Receive entered quantities"}
                          </Button>
                        </form>
                      ) : (
                        <p className="text-sm text-muted-foreground">All invoice quantities have been received.</p>
                      )}
                    </CardContent>
                  </Card>
                );
              })
            )}
          </section>
          <p className="text-xs text-muted-foreground">
            Discounts, taxes, extra transport costs, and attachments are not part of this step.
          </p>
        </>
      )}
    </div>
  );
}
