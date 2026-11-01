import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState, type FormEvent } from "react";
import { Building2, LoaderCircle } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { PageHeader } from "@/components/shared/page-header";
import { BrandManagementSection } from "./brand-management-section";
import { CategoryManagementSection } from "./category-management-section";
import { UnitManagementSection } from "./unit-management-section";

interface Business {
  id: string;
  name: string;
  tax_id: string | null;
  currency: string;
  timezone: string;
  date_format: string;
  language: string;
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function BusinessSetupPage() {
  const [businesses, setBusinesses] = useState<Business[]>([]);
  const [name, setName] = useState("");
  const [taxId, setTaxId] = useState("");
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;

    invoke<Business[]>("business_list")
      .then((result) => {
        if (active) {
          setBusinesses(result);
          setError(null);
        }
      })
      .catch((cause: unknown) => {
        if (active) {
          setError(`Could not load businesses. ${errorMessage(cause)}`);
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
  }, []);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    setIsSaving(true);

    try {
      const business = await invoke<Business>("business_create", {
        name: name.trim(),
        taxId: taxId.trim() || null,
      });
      setBusinesses((current) => [...current, business].sort((a, b) => a.name.localeCompare(b.name)));
      setName("");
      setTaxId("");
    } catch (cause) {
      setError(`Could not save the business. ${errorMessage(cause)}`);
    } finally {
      setIsSaving(false);
    }
  }

  return (
    <div className="mx-auto max-w-4xl space-y-6">
      <PageHeader
        eyebrow="Workspace settings"
        title="Business configuration"
        description="Register your business details. Currency and regional defaults are configured for Rwanda."
      />

      {error ? (
        <div
          role="alert"
          className="rounded-lg border border-destructive/30 bg-destructive/5 p-4 text-sm text-destructive"
        >
          <p className="font-medium">Business configuration failed</p>
          <p className="mt-1">{error}</p>
        </div>
      ) : null}

      <Card>
        <CardHeader>
          <CardTitle>Business details</CardTitle>
          <CardDescription>
            Add the business name and, if applicable, its tax identification number.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <form className="grid gap-5 md:grid-cols-2" onSubmit={handleSubmit}>
            <div className="space-y-2">
              <Label htmlFor="business-name">Business name</Label>
              <Input
                id="business-name"
                autoComplete="organization"
                maxLength={120}
                required
                value={name}
                onChange={(event) => setName(event.target.value)}
                placeholder="e.g. Kigali Hardware"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="business-tax-id">Tax identification number (optional)</Label>
              <Input
                id="business-tax-id"
                maxLength={80}
                value={taxId}
                onChange={(event) => setTaxId(event.target.value)}
                placeholder="TIN"
              />
            </div>
            <div className="flex items-center justify-between gap-4 md:col-span-2">
              <p className="text-sm text-muted-foreground">
                Defaults: RWF · Africa/Kigali · DD/MM/YYYY · English
              </p>
              <Button type="submit" disabled={isSaving || !name.trim()}>
                {isSaving ? <LoaderCircle className="animate-spin" /> : <Building2 />}
                {isSaving ? "Saving..." : "Save business"}
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Registered businesses</CardTitle>
          <CardDescription>Business records stored in this installation.</CardDescription>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="flex items-center gap-2 py-4 text-sm text-muted-foreground">
              <LoaderCircle className="h-4 w-4 animate-spin" />
              Loading business records...
            </div>
          ) : businesses.length === 0 ? (
            <p className="py-4 text-sm text-muted-foreground">
              No business has been configured yet.
            </p>
          ) : (
            <ul className="divide-y">
              {businesses.map((business) => (
                <li key={business.id} className="flex flex-wrap items-center justify-between gap-3 py-4 first:pt-0 last:pb-0">
                  <div>
                    <p className="font-medium">{business.name}</p>
                    {business.tax_id ? (
                      <p className="text-sm text-muted-foreground">TIN: {business.tax_id}</p>
                    ) : null}
                  </div>
                  <p className="text-sm text-muted-foreground">
                    {business.currency} · {business.timezone}
                  </p>
                </li>
              ))}
            </ul>
          )}
        </CardContent>
      </Card>

      <UnitManagementSection businesses={businesses} />
      <CategoryManagementSection businesses={businesses} />
      <BrandManagementSection businesses={businesses} />
    </div>
  );
}
