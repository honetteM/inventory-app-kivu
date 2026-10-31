import { invoke } from "@tauri-apps/api/core";
import { useEffect, useMemo, useState, type FormEvent } from "react";
import { ChevronDown, ChevronUp, LoaderCircle, PackagePlus, Plus, Search } from "lucide-react";

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
const MONEY_SCALE = 100n;
const QUANTITY_SCALE = 1000n;
const MAX_SAFE_INTEGER = BigInt(Number.MAX_SAFE_INTEGER);

interface Business {
  id: string;
  name: string;
}

interface Category {
  id: string;
  name: string;
}

interface Brand {
  id: string;
  name: string;
}

interface Unit {
  id: string;
  code: string;
  name: string;
  is_active: boolean;
}

interface Product {
  id: string;
  name: string;
  sku: string;
  barcode: string | null;
  category_name: string | null;
  brand_name: string | null;
  base_unit_name: string;
  base_unit_code: string;
  purchase_price: number;
  selling_price: number;
  min_stock_quantity: number;
  vat_class: string;
  is_active: boolean;
}

interface ProductPage {
  items: Product[];
  total_count: number;
}

interface ProductUnit {
  id: string;
  product_id: string;
  unit_id: string;
  unit_name: string;
  unit_code: string;
  conversion_factor: number;
  is_base: boolean;
  selling_price: number | null;
}

function toScaledInteger(value: string, scale: bigint, field: string): number {
  if (!/^\d+$/.test(value)) {
    throw new Error(`${field} must be a whole non-negative number.`);
  }

  const scaled = BigInt(value) * scale;
  if (scaled > MAX_SAFE_INTEGER) {
    throw new Error(`${field} is too large.`);
  }

  return Number(scaled);
}

function toScaledDecimal(value: string, scale: bigint, decimalPlaces: number, field: string): number {
  const match = new RegExp(`^(\\d+)(?:\\.(\\d{1,${decimalPlaces}}))?$`).exec(value);
  if (!match) {
    throw new Error(`${field} must be a non-negative number with up to ${decimalPlaces} decimal places.`);
  }

  const fraction = (match[2] ?? "").padEnd(decimalPlaces, "0");
  const scaled = BigInt(match[1]) * scale + BigInt(fraction || "0");
  if (scaled > MAX_SAFE_INTEGER) {
    throw new Error(`${field} is too large.`);
  }

  return Number(scaled);
}

function formatMoney(scaledAmount: number): string {
  if (!Number.isSafeInteger(scaledAmount)) {
    return "Invalid amount";
  }

  const amount = BigInt(scaledAmount);
  const whole = amount / MONEY_SCALE;
  const fraction = amount % MONEY_SCALE;
  const formattedWhole = new Intl.NumberFormat("en-RW").format(whole);
  const formattedFraction = fraction === 0n ? "" : `.${fraction.toString().padStart(2, "0")}`;
  return `${formattedWhole}${formattedFraction} RWF`;
}

function formatQuantity(scaledQuantity: number): string {
  if (!Number.isSafeInteger(scaledQuantity)) {
    return "Invalid quantity";
  }

  const quantity = BigInt(scaledQuantity);
  const whole = quantity / QUANTITY_SCALE;
  const fraction = quantity % QUANTITY_SCALE;
  const formattedWhole = new Intl.NumberFormat("en-RW").format(whole);
  if (fraction === 0n) {
    return formattedWhole;
  }

  return `${formattedWhole}.${fraction.toString().padStart(3, "0").replace(/0+$/, "")}`;
}

function formatConversion(scaledFactor: number): string {
  if (!Number.isSafeInteger(scaledFactor) || scaledFactor <= 0) {
    return "Invalid conversion";
  }

  const factor = BigInt(scaledFactor);
  const whole = factor / QUANTITY_SCALE;
  const fraction = factor % QUANTITY_SCALE;
  const formattedFraction = fraction === 0n ? "" : `.${fraction.toString().padStart(3, "0").replace(/0+$/, "")}`;
  return `${whole}${formattedFraction}`;
}

function errorText(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

export function ProductsPage() {
  const [businesses, setBusinesses] = useState<Business[]>([]);
  const [businessId, setBusinessId] = useState("");
  const [categories, setCategories] = useState<Category[]>([]);
  const [brands, setBrands] = useState<Brand[]>([]);
  const [units, setUnits] = useState<Unit[]>([]);
  const [products, setProducts] = useState<Product[]>([]);
  const [expandedProductId, setExpandedProductId] = useState<string | null>(null);
  const [productUnits, setProductUnits] = useState<ProductUnit[]>([]);
  const [isLoadingProductUnits, setIsLoadingProductUnits] = useState(false);
  const [isSavingProductUnit, setIsSavingProductUnit] = useState(false);
  const [sellingUnitId, setSellingUnitId] = useState("");
  const [conversionFactor, setConversionFactor] = useState("");
  const [unitSellingPrice, setUnitSellingPrice] = useState("");
  const [totalCount, setTotalCount] = useState(0);
  const [search, setSearch] = useState("");
  const [page, setPage] = useState(0);
  const [isLoadingBusinesses, setIsLoadingBusinesses] = useState(true);
  const [isLoadingOptions, setIsLoadingOptions] = useState(false);
  const [isLoadingProducts, setIsLoadingProducts] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [name, setName] = useState("");
  const [sku, setSku] = useState("");
  const [barcode, setBarcode] = useState("");
  const [categoryId, setCategoryId] = useState("");
  const [brandId, setBrandId] = useState("");
  const [baseUnitId, setBaseUnitId] = useState("");
  const [purchasePrice, setPurchasePrice] = useState("");
  const [sellingPrice, setSellingPrice] = useState("");
  const [minimumStock, setMinimumStock] = useState("");
  const [description, setDescription] = useState("");
  const [sizeDimensions, setSizeDimensions] = useState("");
  const [colour, setColour] = useState("");
  const [vatClass, setVatClass] = useState("standard");

  const pageCount = Math.ceil(totalCount / PAGE_SIZE);
  const firstItem = totalCount === 0 ? 0 : page * PAGE_SIZE + 1;
  const lastItem = Math.min((page + 1) * PAGE_SIZE, totalCount);
  const availableUnits = useMemo(() => units.filter((unit) => unit.is_active), [units]);
  const availableSellingUnits = useMemo(
    () => availableUnits.filter((unit) => !productUnits.some((productUnit) => productUnit.unit_id === unit.id)),
    [availableUnits, productUnits],
  );

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
      setCategories([]);
      setBrands([]);
      setUnits([]);
      setProducts([]);
      setTotalCount(0);
      setBaseUnitId("");
      return;
    }

    let active = true;
    setIsLoadingOptions(true);
    setError(null);
    Promise.all([
      invoke<Category[]>("category_list", { businessId }),
      invoke<Brand[]>("brand_list", { businessId }),
      invoke<Unit[]>("unit_list", { businessId }),
    ])
      .then(([categoryResult, brandResult, unitResult]) => {
        if (active) {
          setCategories(categoryResult);
          setBrands(brandResult);
          setUnits(unitResult);
          setBaseUnitId((current) =>
            unitResult.some((unit) => unit.id === current && unit.is_active)
              ? current
              : (unitResult.find((unit) => unit.is_active)?.id ?? ""),
          );
        }
      })
      .catch((cause: unknown) => {
        if (active) {
          setError(`Could not load product options. ${errorText(cause)}`);
        }
      })
      .finally(() => {
        if (active) {
          setIsLoadingOptions(false);
        }
      });

    return () => {
      active = false;
    };
  }, [businessId]);

  useEffect(() => {
    if (!businessId) {
      return;
    }

    let active = true;
    const timer = window.setTimeout(() => {
      setIsLoadingProducts(true);
      invoke<ProductPage>("product_list", {
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
            setError(`Could not load products. ${errorText(cause)}`);
          }
        })
        .finally(() => {
          if (active) {
            setIsLoadingProducts(false);
          }
        });
    }, 200);

    return () => {
      active = false;
      window.clearTimeout(timer);
    };
  }, [businessId, page, search]);

  useEffect(() => {
    if (!businessId || !expandedProductId) {
      setProductUnits([]);
      setSellingUnitId("");
      return;
    }

    let active = true;
    setIsLoadingProductUnits(true);
    setProductUnits([]);
    setError(null);
    invoke<ProductUnit[]>("product_unit_list", {
      businessId,
      productId: expandedProductId,
    })
      .then((result) => {
        if (active) {
          setProductUnits(result);
          setSellingUnitId((current) =>
            result.some((unit) => unit.unit_id === current) ? current : "",
          );
        }
      })
      .catch((cause: unknown) => {
        if (active) {
          setError(`Could not load product units. ${errorText(cause)}`);
        }
      })
      .finally(() => {
        if (active) {
          setIsLoadingProductUnits(false);
        }
      });

    return () => {
      active = false;
    };
  }, [businessId, expandedProductId]);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!businessId || !baseUnitId) {
      setError("Select a business and base unit before creating a product.");
      return;
    }

    setError(null);
    setIsSaving(true);
    try {
      await invoke<Product>("product_create", {
        businessId,
        categoryId: categoryId || null,
        brandId: brandId || null,
        name: name.trim(),
        sku: sku.trim(),
        barcode: barcode.trim() || null,
        description: description.trim() || null,
        sizeDimensions: sizeDimensions.trim() || null,
        colour: colour.trim() || null,
        baseUnitId,
        purchasePrice: toScaledInteger(purchasePrice, MONEY_SCALE, "Purchase price"),
        sellingPrice: toScaledInteger(sellingPrice, MONEY_SCALE, "Selling price"),
        minStockQuantity: toScaledInteger(minimumStock || "0", QUANTITY_SCALE, "Minimum stock"),
        vatClass,
      });

      setName("");
      setSku("");
      setBarcode("");
      setCategoryId("");
      setBrandId("");
      setPurchasePrice("");
      setSellingPrice("");
      setMinimumStock("");
      setDescription("");
      setSizeDimensions("");
      setColour("");
      setVatClass("standard");
      setPage(0);

      const result = await invoke<ProductPage>("product_list", {
        businessId,
        search: search.trim() || null,
        offset: 0,
        limit: PAGE_SIZE,
      });
      setProducts(result.items);
      setTotalCount(result.total_count);
    } catch (cause) {
      setError(`Could not save product. ${errorText(cause)}`);
    } finally {
      setIsSaving(false);
    }
  }

  async function handleProductUnitSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!businessId || !expandedProductId || !sellingUnitId) {
      setError("Select a selling unit before adding its conversion.");
      return;
    }

    setError(null);
    setIsSavingProductUnit(true);
    try {
      await invoke<ProductUnit>("product_unit_create", {
        businessId,
        productId: expandedProductId,
        unitId: sellingUnitId,
        conversionFactor: toScaledDecimal(conversionFactor, QUANTITY_SCALE, 3, "Conversion factor"),
        sellingPrice: unitSellingPrice.trim()
          ? toScaledInteger(unitSellingPrice, MONEY_SCALE, "Unit selling price")
          : null,
      });
      const result = await invoke<ProductUnit[]>("product_unit_list", {
        businessId,
        productId: expandedProductId,
      });
      setProductUnits(result);
      setSellingUnitId("");
      setConversionFactor("");
      setUnitSellingPrice("");
    } catch (cause) {
      setError(`Could not save product unit. ${errorText(cause)}`);
    } finally {
      setIsSavingProductUnit(false);
    }
  }

  return (
    <div className="mx-auto max-w-7xl space-y-6">
      <header className="flex flex-col gap-4 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="text-sm font-medium text-muted-foreground">Catalogue</p>
          <h1 className="text-3xl font-semibold tracking-tight">Products</h1>
          <p className="mt-2 text-muted-foreground">
            Manage product details, pricing, categories, and base units.
          </p>
        </div>
        {businesses.length > 0 ? (
          <div className="w-full space-y-2 md:max-w-xs">
            <Label htmlFor="products-business">Business</Label>
            <select
              id="products-business"
              value={businessId}
              onChange={(event) => {
                setBusinessId(event.target.value);
                setPage(0);
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
            Register a business in Settings before adding products.
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
        <>
          <Card>
            <CardHeader>
              <CardTitle>Add a product</CardTitle>
              <CardDescription>
                Prices are entered in whole RWF. The database stores scaled integer amounts.
              </CardDescription>
            </CardHeader>
            <CardContent>
              {isLoadingOptions ? (
                <div className="flex items-center gap-2 text-sm text-muted-foreground">
                  <LoaderCircle className="size-4 animate-spin" />
                  Loading categories and units...
                </div>
              ) : availableUnits.length === 0 ? (
                <p className="text-sm text-muted-foreground">
                  No active units are available. Configure units in Settings before adding products.
                </p>
              ) : (
                <form className="grid gap-4 md:grid-cols-2 xl:grid-cols-3" onSubmit={handleSubmit}>
                  <div className="space-y-2">
                    <Label htmlFor="product-name">Product name</Label>
                    <Input id="product-name" required maxLength={160} value={name} onChange={(event) => setName(event.target.value)} placeholder="e.g. Cement 50kg" />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="product-sku">SKU</Label>
                    <Input id="product-sku" required maxLength={80} value={sku} onChange={(event) => setSku(event.target.value)} placeholder="e.g. CEM-50" />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="product-barcode">Barcode (optional)</Label>
                    <Input id="product-barcode" maxLength={120} value={barcode} onChange={(event) => setBarcode(event.target.value)} />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="product-category">Category (optional)</Label>
                    <select id="product-category" value={categoryId} onChange={(event) => setCategoryId(event.target.value)} className="h-9 w-full rounded-lg border border-input bg-background px-3 text-sm focus-visible:outline-none focus-visible:ring-3 focus-visible:ring-ring/50">
                      <option value="">No category</option>
                      {categories.map((category) => <option key={category.id} value={category.id}>{category.name}</option>)}
                    </select>
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="product-brand">Brand (optional)</Label>
                    <select id="product-brand" value={brandId} onChange={(event) => setBrandId(event.target.value)} className="h-9 w-full rounded-lg border border-input bg-background px-3 text-sm focus-visible:outline-none focus-visible:ring-3 focus-visible:ring-ring/50">
                      <option value="">No brand</option>
                      {brands.map((brand) => <option key={brand.id} value={brand.id}>{brand.name}</option>)}
                    </select>
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="product-unit">Base unit</Label>
                    <select id="product-unit" required value={baseUnitId} onChange={(event) => setBaseUnitId(event.target.value)} className="h-9 w-full rounded-lg border border-input bg-background px-3 text-sm focus-visible:outline-none focus-visible:ring-3 focus-visible:ring-ring/50">
                      {availableUnits.map((unit) => <option key={unit.id} value={unit.id}>{unit.name} ({unit.code})</option>)}
                    </select>
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="product-vat">VAT classification</Label>
                    <select id="product-vat" value={vatClass} onChange={(event) => setVatClass(event.target.value)} className="h-9 w-full rounded-lg border border-input bg-background px-3 text-sm focus-visible:outline-none focus-visible:ring-3 focus-visible:ring-ring/50">
                      <option value="standard">Standard</option>
                      <option value="zero">Zero-rated</option>
                      <option value="exempt">Exempt</option>
                    </select>
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="product-purchase-price">Purchase price per base unit (RWF)</Label>
                    <Input id="product-purchase-price" type="number" min="0" step="1" maxLength={12} required value={purchasePrice} onChange={(event) => setPurchasePrice(event.target.value)} />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="product-selling-price">Selling price per base unit (RWF)</Label>
                    <Input id="product-selling-price" type="number" min="0" step="1" maxLength={12} required value={sellingPrice} onChange={(event) => setSellingPrice(event.target.value)} />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="product-minimum-stock">Minimum stock (base units)</Label>
                    <Input id="product-minimum-stock" type="number" min="0" step="1" maxLength={9} value={minimumStock} onChange={(event) => setMinimumStock(event.target.value)} />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="product-size">Size / dimensions (optional)</Label>
                    <Input id="product-size" maxLength={120} value={sizeDimensions} onChange={(event) => setSizeDimensions(event.target.value)} />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="product-colour">Colour (optional)</Label>
                    <Input id="product-colour" maxLength={80} value={colour} onChange={(event) => setColour(event.target.value)} />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="product-description">Description (optional)</Label>
                    <Input id="product-description" maxLength={500} value={description} onChange={(event) => setDescription(event.target.value)} />
                  </div>
                  <div className="flex justify-end md:col-span-2 xl:col-span-3">
                    <Button type="submit" disabled={isSaving || isLoadingOptions || !baseUnitId}>
                      {isSaving ? <LoaderCircle className="animate-spin" /> : <PackagePlus />}
                      {isSaving ? "Saving..." : "Add product"}
                    </Button>
                  </div>
                </form>
              )}
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>Product catalogue</CardTitle>
              <CardDescription>{totalCount.toLocaleString("en-RW")} products</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="relative max-w-md">
                <Search className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
                <Input
                  aria-label="Search products"
                  className="pl-9"
                  placeholder="Search name, SKU, barcode, category..."
                  value={search}
                  onChange={(event) => {
                    setSearch(event.target.value);
                    setPage(0);
                  }}
                />
              </div>

              {isLoadingProducts ? (
                <div className="flex items-center justify-center gap-2 py-8 text-sm text-muted-foreground">
                  <LoaderCircle className="size-4 animate-spin" />
                  Loading products...
                </div>
              ) : products.length === 0 ? (
                <p className="py-8 text-center text-sm text-muted-foreground">
                  {search.trim() ? "No products match your search." : "No products have been added yet."}
                </p>
              ) : (
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Product</TableHead>
                      <TableHead>SKU</TableHead>
                      <TableHead>Category</TableHead>
                      <TableHead>Brand</TableHead>
                      <TableHead>Base unit</TableHead>
                      <TableHead className="text-right">Purchase</TableHead>
                      <TableHead className="text-right">Selling</TableHead>
                      <TableHead className="text-right">Min. stock</TableHead>
                      <TableHead>Status</TableHead>
                      <TableHead>Units</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {products.map((product) => (
                      <TableRow key={product.id}>
                        <TableCell>
                          <div className="font-medium">{product.name}</div>
                          {product.barcode ? <div className="text-xs text-muted-foreground">{product.barcode}</div> : null}
                        </TableCell>
                        <TableCell>{product.sku}</TableCell>
                        <TableCell>{product.category_name ?? "—"}</TableCell>
                        <TableCell>{product.brand_name ?? "—"}</TableCell>
                        <TableCell>{product.base_unit_name}</TableCell>
                        <TableCell className="text-right">{formatMoney(product.purchase_price)}</TableCell>
                        <TableCell className="text-right">{formatMoney(product.selling_price)}</TableCell>
                        <TableCell className="text-right">{formatQuantity(product.min_stock_quantity)}</TableCell>
                        <TableCell>
                          <Badge variant={product.is_active ? "secondary" : "outline"}>
                            {product.is_active ? "Active" : "Inactive"}
                          </Badge>
                        </TableCell>
                        <TableCell>
                          <Button
                            type="button"
                            variant="outline"
                            size="sm"
                            aria-expanded={expandedProductId === product.id}
                            aria-label={`${expandedProductId === product.id ? "Hide" : "Manage"} selling units for ${product.name}`}
                            onClick={() => setExpandedProductId((current) => current === product.id ? null : product.id)}
                          >
                            {expandedProductId === product.id ? <ChevronUp /> : <ChevronDown />}
                            Manage
                          </Button>
                        </TableCell>
                      </TableRow>
                    ))}
                    {products.map((product) =>
                      expandedProductId === product.id ? (
                        <TableRow key={`${product.id}-units`}>
                          <TableCell colSpan={10} className="bg-muted/20">
                            <div className="space-y-4 p-2">
                              <div>
                                <h3 className="font-medium">Selling units for {product.name}</h3>
                                <p className="text-sm text-muted-foreground">
                                  Conversion factor means how many base units are in one selling unit. Use up to three decimal places.
                                </p>
                              </div>

                              {isLoadingProductUnits ? (
                                <div className="flex items-center gap-2 text-sm text-muted-foreground">
                                  <LoaderCircle className="size-4 animate-spin" />
                                  Loading product units...
                                </div>
                              ) : (
                                <>
                                  <div className="flex flex-wrap gap-2">
                                    {productUnits.map((unit) => (
                                      <Badge key={unit.id} variant={unit.is_base ? "secondary" : "outline"}>
                                        {unit.unit_name}: 1 {unit.unit_code} = {formatConversion(unit.conversion_factor)} {product.base_unit_code}
                                        {unit.selling_price !== null ? ` · ${formatMoney(unit.selling_price)}` : ""}
                                        {unit.is_base ? " · base" : ""}
                                      </Badge>
                                    ))}
                                  </div>

                                  {availableSellingUnits.length > 0 ? (
                                    <form className="grid gap-3 md:grid-cols-[1fr_1fr_1fr_auto]" onSubmit={handleProductUnitSubmit}>
                                      <div className="space-y-2">
                                        <Label htmlFor={`selling-unit-${product.id}`}>Selling unit</Label>
                                        <select
                                          id={`selling-unit-${product.id}`}
                                          required
                                          value={sellingUnitId}
                                          onChange={(event) => setSellingUnitId(event.target.value)}
                                          className="h-9 w-full rounded-lg border border-input bg-background px-3 text-sm focus-visible:outline-none focus-visible:ring-3 focus-visible:ring-ring/50"
                                        >
                                          <option value="">Select unit</option>
                                          {availableSellingUnits.map((unit) => (
                                            <option key={unit.id} value={unit.id}>{unit.name} ({unit.code})</option>
                                          ))}
                                        </select>
                                      </div>
                                      <div className="space-y-2">
                                        <Label htmlFor={`conversion-${product.id}`}>Base units per selling unit</Label>
                                        <Input
                                          id={`conversion-${product.id}`}
                                          type="number"
                                          min="0.001"
                                          step="0.001"
                                          required
                                          value={conversionFactor}
                                          onChange={(event) => setConversionFactor(event.target.value)}
                                          placeholder="e.g. 100"
                                        />
                                      </div>
                                      <div className="space-y-2">
                                        <Label htmlFor={`unit-price-${product.id}`}>Selling price (RWF, optional)</Label>
                                        <Input
                                          id={`unit-price-${product.id}`}
                                          type="number"
                                          min="0"
                                          step="1"
                                          value={unitSellingPrice}
                                          onChange={(event) => setUnitSellingPrice(event.target.value)}
                                        />
                                      </div>
                                      <Button
                                        type="submit"
                                        className="self-end"
                                        disabled={isSavingProductUnit || isLoadingProductUnits || !sellingUnitId || !conversionFactor}
                                      >
                                        {isSavingProductUnit ? <LoaderCircle className="animate-spin" /> : <Plus />}
                                        Add unit
                                      </Button>
                                    </form>
                                  ) : (
                                    <p className="text-sm text-muted-foreground">
                                      No other active units are available. Add a unit in Settings first.
                                    </p>
                                  )}
                                </>
                              )}
                            </div>
                          </TableCell>
                        </TableRow>
                      ) : null,
                    )}
                  </TableBody>
                </Table>
              )}

              <div className="flex flex-wrap items-center justify-between gap-3 border-t pt-4">
                <p className="text-sm text-muted-foreground">
                  Showing {firstItem}–{lastItem} of {totalCount}
                </p>
                <div className="flex gap-2">
                  <Button variant="outline" disabled={page === 0 || isLoadingProducts} onClick={() => setPage((current) => Math.max(0, current - 1))}>
                    Previous
                  </Button>
                  <Button variant="outline" disabled={page + 1 >= pageCount || isLoadingProducts} onClick={() => setPage((current) => current + 1)}>
                    Next
                  </Button>
                </div>
              </div>
            </CardContent>
          </Card>
        </>
      )}
    </div>
  );
}
