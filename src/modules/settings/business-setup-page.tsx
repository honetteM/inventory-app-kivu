import { invoke } from "@tauri-apps/api/core";
import { save, open } from "@tauri-apps/plugin-dialog";
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
import { useAuthentication } from "@/modules/auth/auth-gate";

interface Business {
  id: string;
  name: string;
  tax_id: string | null;
  currency: string;
  timezone: string;
  date_format: string;
  language: string;
}

interface Branch {
  id: string;
  business_id: string;
  name: string;
  address: string | null;
  phone: string | null;
  is_active: boolean;
  manager_id: string | null;
  manager_name: string | null;
}

interface Employee {
  id: string;
  business_id: string;
  branch_id: string | null;
  username: string;
  full_name: string;
  role: string;
  is_active: boolean;
  last_login_at: string | null;
  created_at: string;
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function BusinessSetupPage() {
  const { user } = useAuthentication();
  const [businesses, setBusinesses] = useState<Business[]>([]);
  const [branches, setBranches] = useState<Branch[]>([]);
  const [employees, setEmployees] = useState<Employee[]>([]);
  const [name, setName] = useState("");
  const [taxId, setTaxId] = useState("");
  const [branchName, setBranchName] = useState("");
  const [branchAddress, setBranchAddress] = useState("");
  const [branchPhone, setBranchPhone] = useState("");
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

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

    if (user.role === "owner") {
      invoke<Branch[]>("branch_list", { businessId: user.business_id })
        .then((result) => {
          if (active) setBranches(result);
        })
        .catch((cause: unknown) => {
          if (active) setError(`Could not load branches. ${errorMessage(cause)}`);
        });
      invoke<Employee[]>("employee_list", { businessId: user.business_id })
        .then((result) => {
          if (active) setEmployees(result.filter((employee) => employee.role === "manager"));
        })
        .catch((cause: unknown) => {
          if (active) setError(`Could not load managers. ${errorMessage(cause)}`);
        });
    }

    return () => {
      active = false;
    };
  }, [user.business_id, user.role]);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    setNotice(null);
    setIsSaving(true);

    try {
      await invoke<Business>("business_create", {
        name: name.trim(),
        taxId: taxId.trim() || null,
      });
      setName("");
      setTaxId("");
      setNotice("Business created with your owner sign-in. Sign out and choose it on the sign-in screen before managing its records.");
    } catch (cause) {
      setError(`Could not save the business. ${errorMessage(cause)}`);
    } finally {
      setIsSaving(false);
    }

  }

  async function handleBranchSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    setNotice(null);
    setIsSaving(true);
    try {
      const created = await invoke<Branch>("branch_create", {
        businessId: user.business_id,
        name: branchName.trim(),
        address: branchAddress.trim() || null,
        phone: branchPhone.trim() || null,
      });
      setBranches((current) => [...current, created].sort((a, b) => a.name.localeCompare(b.name)));
      setBranchName("");
      setBranchAddress("");
      setBranchPhone("");
      setNotice("Branch created and audited.");
    } catch (cause) {
      setError(`Could not create branch. ${errorMessage(cause)}`);
    } finally {
      setIsSaving(false);
    }
  }

  async function handleManagerAssignment(branch: Branch, managerId: string) {
    if (!managerId) return;
    setError(null);
    setNotice(null);
    try {
      const updated = await invoke<Branch>("branch_manager_assign", {
        businessId: user.business_id,
        branchId: branch.id,
        managerId,
      });
      setBranches((current) => current.map((item) => item.id === updated.id ? updated : item));
      setNotice("Branch manager assigned and audited.");
    } catch (cause) {
      setError(`Could not assign manager. ${errorMessage(cause)}`);
    }
  }

  async function handleBackup() {
    setError(null);
    setNotice(null);
    const destination = await save({
      title: "Save KIVU SUBLIME database backup",
      defaultPath: "kivu-sublime-backup.db",
      filters: [{ name: "SQLite database", extensions: ["db"] }],
    });
    if (!destination) return;
    try {
      await invoke("database_backup", { destination });
      setNotice("Database backup created successfully.");
    } catch (cause) {
      setError(`Could not create the database backup. ${errorMessage(cause)}`);
    }
  }

  async function handleRestore() {
    setError(null);
    setNotice(null);
    const source = await open({
      title: "Select a KIVU SUBLIME database backup",
      multiple: false,
      directory: false,
      filters: [{ name: "SQLite database", extensions: ["db"] }],
    });
    if (!source || Array.isArray(source)) return;
    if (!window.confirm("Restore this backup? Current data will be replaced and the application will close.")) return;
    try {
      await invoke("database_restore", { source });
    } catch (cause) {
      setError(`Could not restore the database. ${errorMessage(cause)}`);
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
      {notice ? (
        <div role="status" className="rounded-lg border border-primary/20 bg-primary/5 p-4 text-sm text-foreground">
          {notice}
        </div>
      ) : null}

      {user.role === "owner" ? (
        <Card>
          <CardHeader>
            <CardTitle>Branches</CardTitle>
            <CardDescription>Create branch locations and view their assigned managers.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-5">
            <form className="grid gap-4 md:grid-cols-3" onSubmit={handleBranchSubmit}>
              <div className="space-y-2">
                <Label htmlFor="branch-name">Branch name</Label>
                <Input id="branch-name" required maxLength={120} value={branchName} onChange={(event) => setBranchName(event.target.value)} placeholder="e.g. Goma branch" />
              </div>
              <div className="space-y-2">
                <Label htmlFor="branch-address">Address</Label>
                <Input id="branch-address" maxLength={200} value={branchAddress} onChange={(event) => setBranchAddress(event.target.value)} />
              </div>
              <div className="space-y-2">
                <Label htmlFor="branch-phone">Phone</Label>
                <Input id="branch-phone" maxLength={80} value={branchPhone} onChange={(event) => setBranchPhone(event.target.value)} />
              </div>
              <div className="md:col-span-3 flex justify-end">
                <Button type="submit" disabled={isSaving || !branchName.trim()}>
                  {isSaving ? <LoaderCircle className="animate-spin" /> : null}
                  Create branch
                </Button>
              </div>
            </form>
            <div className="divide-y rounded-lg border">
              {branches.map((branch) => (
                <div key={branch.id} className="flex flex-wrap items-center justify-between gap-3 px-4 py-3">
                  <div>
                    <p className="font-medium">{branch.name}</p>
                    <p className="text-sm text-muted-foreground">
                      {branch.address || "No address"} · {branch.phone || "No phone"}
                    </p>
                  </div>
                  <select
                    className="rounded-md border bg-background px-3 py-2 text-sm"
                    value={branch.manager_id || ""}
                    onChange={(event) => void handleManagerAssignment(branch, event.target.value)}
                    aria-label={`Manager for ${branch.name}`}
                  >
                    <option value="">No manager assigned</option>
                    {employees.map((employee) => (
                      <option key={employee.id} value={employee.id}>{employee.full_name}</option>
                    ))}
                  </select>
                </div>
              ))}
              {branches.length === 0 ? <p className="px-4 py-3 text-sm text-muted-foreground">No branches registered.</p> : null}
            </div>
          </CardContent>
        </Card>
      ) : null}

      {user.role === "owner" ? (
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
      ) : null}

      {user.role === "owner" ? (
        <Card>
          <CardHeader>
            <CardTitle>Database safety</CardTitle>
            <CardDescription>
              Create a verified SQLite backup before upgrades. Restoring replaces the current data and closes the application.
            </CardDescription>
          </CardHeader>
          <CardContent className="flex flex-wrap gap-3">
            <Button type="button" variant="outline" onClick={handleBackup}>
              Create database backup
            </Button>
            <Button type="button" variant="destructive" onClick={handleRestore}>
              Restore database backup
            </Button>
          </CardContent>
        </Card>
      ) : null}

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
