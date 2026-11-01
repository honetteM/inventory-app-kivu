import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState, type FormEvent } from "react";
import { LoaderCircle, Tag } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { SelectField } from "@/components/shared/select-field";

interface BusinessOption {
  id: string;
  name: string;
}

interface Brand {
  id: string;
  business_id: string;
  name: string;
  is_active: boolean;
}

export function BrandManagementSection({ businesses }: { businesses: BusinessOption[] }) {
  const [businessId, setBusinessId] = useState("");
  const [brands, setBrands] = useState<Brand[]>([]);
  const [name, setName] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!businesses.some((business) => business.id === businessId)) {
      setBusinessId(businesses[0]?.id ?? "");
    }
  }, [businessId, businesses]);

  useEffect(() => {
    if (!businessId) {
      setBrands([]);
      return;
    }

    let active = true;
    setIsLoading(true);
    setError(null);
    invoke<Brand[]>("brand_list", { businessId })
      .then((result) => {
        if (active) {
          setBrands(result);
        }
      })
      .catch((cause: unknown) => {
        if (active) {
          setError(`Could not load brands. ${cause instanceof Error ? cause.message : String(cause)}`);
        }
      })
      .finally(() => {
        if (active) {
          setIsLoading(false);
        }
      });

    return () => {
      active = false;
    };
  }, [businessId]);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!businessId) {
      return;
    }

    setError(null);
    setIsSaving(true);
    try {
      const brand = await invoke<Brand>("brand_create", {
        businessId,
        name: name.trim(),
      });
      setBrands((current) =>
        [...current, brand].sort((left, right) => left.name.localeCompare(right.name)),
      );
      setName("");
    } catch (cause) {
      setError(`Could not save the brand. ${cause instanceof Error ? cause.message : String(cause)}`);
    } finally {
      setIsSaving(false);
    }
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Brands</CardTitle>
        <CardDescription>
          Manage manufacturers and product brands for each business.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        {businesses.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            Register a business before managing its brands.
          </p>
        ) : (
          <>
            <div className="max-w-sm space-y-2">
              <Label htmlFor="brand-business">Business</Label>
              <SelectField
                id="brand-business"
                value={businessId}
                onChange={(event) => setBusinessId(event.target.value)}
              >
                {businesses.map((business) => (
                  <option key={business.id} value={business.id}>{business.name}</option>
                ))}
              </SelectField>
            </div>

            <form className="flex flex-col gap-4 rounded-lg border p-4 sm:flex-row sm:items-end" onSubmit={handleSubmit}>
              <div className="flex-1 space-y-2">
                <Label htmlFor="brand-name">Brand name</Label>
                <Input
                  id="brand-name"
                  maxLength={120}
                  required
                  value={name}
                  onChange={(event) => setName(event.target.value)}
                  placeholder="e.g. BuildPro"
                />
              </div>
              <Button type="submit" disabled={isSaving || !name.trim()}>
                {isSaving ? <LoaderCircle className="animate-spin" /> : <Tag />}
                {isSaving ? "Saving..." : "Add brand"}
              </Button>
            </form>

            {error ? (
              <div role="alert" className="rounded-lg border border-destructive/30 bg-destructive/5 p-3 text-sm text-destructive">
                {error}
              </div>
            ) : null}

            {isLoading ? (
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <LoaderCircle className="size-4 animate-spin" />
                Loading brands...
              </div>
            ) : brands.length === 0 ? (
              <p className="text-sm text-muted-foreground">No brands have been added for this business.</p>
            ) : (
              <ul className="divide-y">
                {brands.map((brand) => (
                  <li key={brand.id} className="flex items-center justify-between gap-3 py-3 first:pt-0 last:pb-0">
                    <span className="font-medium">{brand.name}</span>
                    <Badge variant={brand.is_active ? "secondary" : "outline"}>
                      {brand.is_active ? "Active" : "Inactive"}
                    </Badge>
                  </li>
                ))}
              </ul>
            )}
          </>
        )}
      </CardContent>
    </Card>
  );
}
