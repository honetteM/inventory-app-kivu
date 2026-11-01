import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState, type FormEvent } from "react";
import { LoaderCircle, Ruler } from "lucide-react";

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

interface Unit {
  id: string;
  business_id: string | null;
  code: string;
  name: string;
  decimals_ok: boolean;
  is_system: boolean;
  is_active: boolean;
}

export function UnitManagementSection({ businesses }: { businesses: BusinessOption[] }) {
  const [businessId, setBusinessId] = useState("");
  const [units, setUnits] = useState<Unit[]>([]);
  const [code, setCode] = useState("");
  const [name, setName] = useState("");
  const [decimalsOk, setDecimalsOk] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!businessId && businesses.length > 0) {
      setBusinessId(businesses[0].id);
    }
  }, [businessId, businesses]);

  useEffect(() => {
    if (!businessId) {
      setUnits([]);
      return;
    }

    let active = true;
    setIsLoading(true);
    setError(null);

    invoke<Unit[]>("unit_list", { businessId })
      .then((result) => {
        if (active) {
          setUnits(result);
        }
      })
      .catch((cause: unknown) => {
        if (active) {
          setError(`Could not load units. ${cause instanceof Error ? cause.message : String(cause)}`);
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
      const unit = await invoke<Unit>("unit_create", {
        businessId,
        code: code.trim(),
        name: name.trim(),
        decimalsOk,
      });
      setUnits((current) =>
        [...current, unit].sort(
          (left, right) =>
            Number(right.is_system) - Number(left.is_system) ||
            left.name.localeCompare(right.name),
        ),
      );
      setCode("");
      setName("");
      setDecimalsOk(false);
    } catch (cause) {
      setError(`Could not save the unit. ${cause instanceof Error ? cause.message : String(cause)}`);
    } finally {
      setIsSaving(false);
    }
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Units of measurement</CardTitle>
        <CardDescription>
          Use shared hardware units or add units specific to a business.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        {businesses.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            Register a business before managing its units.
          </p>
        ) : (
          <>
            <div className="max-w-sm space-y-2">
              <Label htmlFor="unit-business">Business</Label>
              <SelectField
                id="unit-business"
                value={businessId}
                onChange={(event) => setBusinessId(event.target.value)}
              >
                {businesses.map((business) => (
                  <option key={business.id} value={business.id}>
                    {business.name}
                  </option>
                ))}
              </SelectField>
            </div>

            <form className="grid gap-4 rounded-lg border p-4 md:grid-cols-[1fr_1fr_auto_auto]" onSubmit={handleSubmit}>
              <div className="space-y-2">
                <Label htmlFor="unit-code">Unit code</Label>
                <Input
                  id="unit-code"
                  autoComplete="off"
                  maxLength={20}
                  required
                  value={code}
                  onChange={(event) => setCode(event.target.value)}
                  placeholder="e.g. box"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="unit-name">Unit name</Label>
                <Input
                  id="unit-name"
                  maxLength={80}
                  required
                  value={name}
                  onChange={(event) => setName(event.target.value)}
                  placeholder="e.g. Box"
                />
              </div>
              <label className="flex items-center gap-2 self-end pb-2 text-sm">
                <input
                  type="checkbox"
                  checked={decimalsOk}
                  onChange={(event) => setDecimalsOk(event.target.checked)}
                  className="size-4 rounded border-input accent-primary"
                />
                Allows decimals
              </label>
              <Button type="submit" className="self-end" disabled={isSaving || !code.trim() || !name.trim()}>
                {isSaving ? <LoaderCircle className="animate-spin" /> : <Ruler />}
                {isSaving ? "Saving..." : "Add unit"}
              </Button>
            </form>

            {error ? (
              <div role="alert" className="rounded-lg border border-destructive/30 bg-destructive/5 p-3 text-sm text-destructive">
                {error}
              </div>
            ) : null}

            {isLoading ? (
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <LoaderCircle className="h-4 w-4 animate-spin" />
                Loading units...
              </div>
            ) : units.length === 0 ? (
              <p className="text-sm text-muted-foreground">No units are available for this business.</p>
            ) : (
              <div className="divide-y">
                {units.map((unit) => (
                  <div key={unit.id} className="flex flex-wrap items-center justify-between gap-3 py-3 first:pt-0 last:pb-0">
                    <div className="flex items-center gap-3">
                      <span className="flex size-9 items-center justify-center rounded-md bg-muted text-sm font-medium">
                        {unit.code}
                      </span>
                      <div>
                        <p className="font-medium">{unit.name}</p>
                        <p className="text-xs text-muted-foreground">
                          {unit.decimals_ok ? "Decimal quantities allowed" : "Whole quantities only"}
                        </p>
                      </div>
                    </div>
                    <Badge variant={unit.is_system ? "secondary" : "outline"}>
                      {unit.is_system ? "System" : "Custom"}
                    </Badge>
                  </div>
                ))}
              </div>
            )}
          </>
        )}
      </CardContent>
    </Card>
  );
}
