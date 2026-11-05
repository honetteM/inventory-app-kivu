import { invoke } from "@tauri-apps/api/core";
import { KeyRound, LoaderCircle, Plus, UserCog } from "lucide-react";
import { Fragment, useEffect, useState, type FormEvent } from "react";

import { PageHeader } from "@/components/shared/page-header";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { useAuthentication } from "@/modules/auth/auth-gate";

type EmployeeRole = "manager" | "cashier";

interface Employee {
  id: string;
  business_id: string;
  branch_id: string | null;
  username: string;
  full_name: string;
  role: EmployeeRole;
  is_active: boolean;
  last_login_at: string | null;
  created_at: string;
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

interface RolePermissionSetting {
  permissionId: string;
  description: string;
  enabled: boolean;
}

const PERMISSION_LABELS: Record<string, string> = {
  "system.read": "View basic application information",
  "business.read": "View business profile",
  "catalog.read": "View products, categories, brands and units",
  "catalog.manage": "Create and update products, categories, brands and units",
  "inventory.read": "View stock and movement history",
  "inventory.manage": "Receive stock and make stock adjustments",
  "suppliers.read": "View supplier records",
  "suppliers.manage": "Add and edit suppliers",
  "customers.read": "View customer records",
  "customers.manage": "Add and edit customers",
  "customer_debts.read": "View customer debts for collection",
  "customer_payments.manage": "Record customer debt repayments",
  "pos.use": "Search products and use the point of sale",
  "sales.read": "View sales history",
  "sales.manage": "Record sales",
  "purchases.read": "View purchases and purchase costs",
  "purchases.manage": "Create purchases, receive deliveries and record supplier payments",
  "expenses.read": "View operating expense records",
  "expenses.manage": "Record operating expenses from verified account balances",
};

function errorText(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

function roleLabel(role: EmployeeRole): string {
  return role === "manager" ? "Manager" : "Cashier";
}

export function EmployeesPage() {
  const { user } = useAuthentication();
  const [employees, setEmployees] = useState<Employee[]>([]);
  const [branches, setBranches] = useState<Branch[]>([]);
  const [selectedBranchId, setSelectedBranchId] = useState("");
  const [managerPermissions, setManagerPermissions] = useState<RolePermissionSetting[]>([]);
  const [username, setUsername] = useState("");
  const [fullName, setFullName] = useState("");
  const [password, setPassword] = useState("");
  const [passwordConfirmation, setPasswordConfirmation] = useState("");
  const [isLoading, setIsLoading] = useState(true);
  const [isLoadingPermissions, setIsLoadingPermissions] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [isSavingPermissions, setIsSavingPermissions] = useState(false);
  const [savingEmployeeId, setSavingEmployeeId] = useState<string | null>(null);
  const [resetEmployeeId, setResetEmployeeId] = useState<string | null>(null);
  const [resetPassword, setResetPassword] = useState("");
  const [resetPasswordConfirmation, setResetPasswordConfirmation] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    Promise.all([
      invoke<Employee[]>("employee_list", { businessId: user.business_id }),
      invoke<RolePermissionSetting[]>("manager_permission_list", { businessId: user.business_id }),
      invoke<Branch[]>("branch_list", { businessId: user.business_id }),
    ])
      .then(([employeeResult, permissionResult, branchResult]) => {
        if (active) {
          setEmployees(employeeResult);
          setManagerPermissions(permissionResult);
          const activeBranches = branchResult.filter((branch) => branch.is_active);
          setBranches(activeBranches);
          setSelectedBranchId((current) => current || activeBranches[0]?.id || "");
          setError(null);
        }
      })
      .catch((cause: unknown) => {
        if (active) {
          setError(`Could not load employee and permission settings. ${errorText(cause)}`);
        }
      })
      .finally(() => {
        if (active) {
          setIsLoading(false);
          setIsLoadingPermissions(false);
        }
      });
    return () => {
      active = false;
    };
  }, [user.business_id]);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    setNotice(null);
    if (!selectedBranchId) {
      setError("Select a branch for the Manager profile.");
      return;
    }
    if (password !== passwordConfirmation) {
      setError("Password confirmation does not match.");
      return;
    }

    setIsSaving(true);
    try {
      const createdEmployee = await invoke<Employee>("employee_create", {
        businessId: user.business_id,
        branchId: selectedBranchId,
        username: username.trim(),
        fullName: fullName.trim(),
        role: "manager",
        password,
      });
      setUsername("");
      setFullName("");
      setPassword("");
      setPasswordConfirmation("");
      setEmployees((current) =>
        [...current, createdEmployee].sort((a, b) =>
          Number(b.is_active) - Number(a.is_active) || a.full_name.localeCompare(b.full_name),
        ),
      );
      setNotice("Manager profile created and audited. The Manager can now sign in with this username and password.");
    } catch (cause) {
      setError(`Could not create employee profile. ${errorText(cause)}`);
    } finally {
      setIsSaving(false);
    }
  }

  async function resetEmployeePassword(employee: Employee) {
    setError(null);
    setNotice(null);
    if (resetPassword !== resetPasswordConfirmation) {
      setError("Password confirmation does not match.");
      return;
    }
    if (resetPassword.length < 12 || resetPassword.length > 128) {
      setError("Password must be 12-128 characters long.");
      return;
    }
    setSavingEmployeeId(employee.id);
    try {
      await invoke("employee_password_reset", {
        businessId: user.business_id,
        employeeId: employee.id,
        password: resetPassword,
      });
      setResetEmployeeId(null);
      setResetPassword("");
      setResetPasswordConfirmation("");
      setNotice(`${employee.full_name}'s password was reset and the action was recorded in the audit log.`);
    } catch (cause) {
      setError(`Could not reset ${employee.full_name}'s password. ${errorText(cause)}`);
    } finally {
      setSavingEmployeeId(null);
    }
  }

  async function setEmployeeActive(employee: Employee, isActive: boolean) {
    setError(null);
    setNotice(null);
    setSavingEmployeeId(employee.id);
    try {
      await invoke("employee_set_active", {
        businessId: user.business_id,
        employeeId: employee.id,
        isActive,
      });
      setEmployees((current) =>
        current
          .map((item) => item.id === employee.id ? { ...item, is_active: isActive } : item)
          .sort((a, b) =>
            Number(b.is_active) - Number(a.is_active) || a.full_name.localeCompare(b.full_name),
          ),
      );
      setNotice(`${employee.full_name} ${isActive ? "activated" : "deactivated"}. The change is recorded in the audit log.`);
    } catch (cause) {
      setError(`Could not update ${employee.full_name}. ${errorText(cause)}`);
    } finally {
      setSavingEmployeeId(null);
    }

  }

  async function saveManagerPermissions() {
    setError(null);
    setNotice(null);
    setIsSavingPermissions(true);
    try {
      const updated = await invoke<RolePermissionSetting[]>("manager_permission_update", {
        businessId: user.business_id,
        grants: managerPermissions.map(({ permissionId, enabled }) => ({
          permissionId,
          enabled,
        })),
      });
      setManagerPermissions(updated);
      setNotice("Manager permissions saved and recorded in the audit log.");
    } catch (cause) {
      setError(`Could not save Manager permissions. ${errorText(cause)}`);
    } finally {
      setIsSavingPermissions(false);
    }
  }

  return (
    <div className="mx-auto max-w-5xl space-y-6">
      <PageHeader
        eyebrow="People and access"
        title="Employees"
        description="Create and maintain staff profiles for this business. Changes are recorded against your owner account."
      />

      <div className="rounded-lg border border-amber-500/30 bg-amber-500/5 p-4 text-sm text-foreground">
        Active Managers can sign in with their profile username and password. Their access is limited to the permissions configured above; Cashier sign-in is unavailable.
      </div>

      {error ? (
        <div role="alert" className="rounded-lg border border-destructive/30 bg-destructive/5 p-4 text-sm text-destructive">
          <p className="font-medium">Employee action failed</p>
          <p className="mt-1">{error}</p>
        </div>
      ) : null}
      {notice ? (
        <div role="status" className="rounded-lg border border-primary/20 bg-primary/5 p-4 text-sm text-foreground">
          {notice}
        </div>
      ) : null}

      <Card>
        <CardHeader>
          <CardTitle>Role access</CardTitle>
          <CardDescription>
            Configure only the Manager role. Owner/Admin always retains full access; Cashier permissions are not configurable or granted.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-5">
          <div className="rounded-lg border bg-muted/30 p-4">
            <p className="font-medium">Owner / Admin</p>
            <p className="mt-1 text-sm text-muted-foreground">Full access to all application capabilities. This role cannot be restricted here.</p>
          </div>
          <div className="space-y-3">
            <div>
              <p className="font-medium">Manager permissions</p>
              <p className="mt-1 text-sm text-muted-foreground">
                Managers can handle products, stock, purchases, sales, customers, suppliers and operating expenses. Expenses require an opened payment account with enough verified funds. Financial reports, cash-account administration and employee access remain Owner/Admin-only. Managers can view purchase costs.
              </p>
            </div>
            {isLoadingPermissions ? (
              <div className="flex items-center gap-2 py-4 text-sm text-muted-foreground">
                <LoaderCircle className="size-4 animate-spin" />
                Loading Manager permissions...
              </div>
            ) : (
              <>
                <div className="grid gap-2 sm:grid-cols-2">
                  {managerPermissions.map((permission) => (
                    <label
                      key={permission.permissionId}
                      className="flex cursor-pointer items-start gap-3 rounded-md border p-3 text-sm transition-colors hover:bg-muted/40"
                    >
                      <input
                        type="checkbox"
                        className="mt-0.5 size-4 accent-primary"
                        checked={permission.enabled}
                        disabled={isSavingPermissions}
                        onChange={(event) => {
                          const enabled = event.target.checked;
                          setManagerPermissions((current) =>
                            current.map((item) =>
                              item.permissionId === permission.permissionId ? { ...item, enabled } : item,
                            ),
                          );
                        }}
                      />
                      <span>
                        <span className="block font-medium">
                          {PERMISSION_LABELS[permission.permissionId] ?? permission.description}
                        </span>
                        <span className="mt-1 block text-xs text-muted-foreground">{permission.description}</span>
                      </span>
                    </label>
                  ))}
                </div>
                <div className="flex justify-end">
                  <Button type="button" onClick={() => void saveManagerPermissions()} disabled={isSavingPermissions}>
                    {isSavingPermissions ? <LoaderCircle className="animate-spin" /> : null}
                    {isSavingPermissions ? "Saving..." : "Save Manager permissions"}
                  </Button>
                </div>
              </>
            )}
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Add an employee</CardTitle>
          <CardDescription>
            Create a Manager profile with a unique sign-in username and initial password. Passwords are hashed on this device and are never displayed after creation.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <form className="grid gap-5 md:grid-cols-2" onSubmit={handleSubmit}>
            <div className="space-y-2">
              <Label htmlFor="employee-full-name">Full name</Label>
              <Input
                id="employee-full-name"
                autoComplete="name"
                maxLength={120}
                required
                value={fullName}
                onChange={(event) => setFullName(event.target.value)}
                placeholder="e.g. Jean Niyonzima"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="employee-branch">Assigned branch</Label>
              <select
                id="employee-branch"
                required
                value={selectedBranchId}
                disabled={isSaving || branches.length === 0}
                onChange={(event) => setSelectedBranchId(event.target.value)}
                className="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
              >
                {branches.length === 0 ? (
                  <option value="">No active branches available</option>
                ) : (
                  branches.map((branch) => (
                    <option key={branch.id} value={branch.id}>
                      {branch.name}
                    </option>
                  ))
                )}
              </select>
              <p className="text-xs text-muted-foreground">The Manager will be restricted to this branch.</p>
            </div>
            <div className="space-y-2">
              <Label htmlFor="employee-username">Username</Label>
              <Input
                id="employee-username"
                autoComplete="off"
                minLength={3}
                maxLength={32}
                required
                value={username}
                onChange={(event) => setUsername(event.target.value)}
                placeholder="e.g. jean.manager"
              />
              <p className="text-xs text-muted-foreground">3–32 letters, numbers, dots, underscores or hyphens.</p>
            </div>
            <div className="space-y-2">
              <Label htmlFor="employee-password">Initial password</Label>
              <Input
                id="employee-password"
                type="password"
                autoComplete="new-password"
                minLength={12}
                maxLength={128}
                required
                value={password}
                onChange={(event) => setPassword(event.target.value)}
              />
              <p className="text-xs text-muted-foreground">Use 12–128 characters.</p>
            </div>
            <div className="space-y-2">
              <Label htmlFor="employee-password-confirmation">Confirm initial password</Label>
              <Input
                id="employee-password-confirmation"
                type="password"
                autoComplete="new-password"
                minLength={12}
                maxLength={128}
                required
                value={passwordConfirmation}
                onChange={(event) => setPasswordConfirmation(event.target.value)}
              />
            </div>
            <div className="flex justify-end md:col-span-2">
              <Button type="submit" disabled={isSaving}>
                {isSaving ? <LoaderCircle className="animate-spin" /> : <Plus />}
                {isSaving ? "Saving..." : "Create Manager profile"}
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Employee profiles</CardTitle>
          <CardDescription>Inactive profiles are retained for audit history and can be reactivated.</CardDescription>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="flex items-center justify-center gap-2 py-10 text-sm text-muted-foreground">
              <LoaderCircle className="size-4 animate-spin" />
              Loading employees...
            </div>
          ) : employees.length === 0 ? (
            <div className="flex flex-col items-center gap-3 py-10 text-center">
              <UserCog className="size-8 text-muted-foreground" />
              <p className="font-medium">No employees added yet</p>
              <p className="text-sm text-muted-foreground">Employee profiles will appear here after you add them.</p>
            </div>
          ) : (
            <div className="divide-y">
              {employees.map((employee) => (
                <Fragment key={employee.id}>
                  <div className="flex flex-col gap-3 py-4 sm:flex-row sm:items-center sm:justify-between">
                    <div className="flex min-w-0 items-start gap-3">
                      <UserCog className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
                      <div className="min-w-0 space-y-1">
                        <p className="truncate font-medium">{employee.full_name}</p>
                        <p className="text-sm text-muted-foreground">
                          @{employee.username} · {roleLabel(employee.role)}
                          {employee.branch_id
                            ? ` · ${branches.find((branch) => branch.id === employee.branch_id)?.name ?? "Assigned branch"}`
                            : ""}
                        </p>
                      </div>
                      <Badge variant={employee.is_active ? "default" : "secondary"}>
                        {employee.is_active ? "Active" : "Inactive"}
                      </Badge>
                    </div>
                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      disabled={savingEmployeeId !== null}
                      onClick={() => void setEmployeeActive(employee, !employee.is_active)}
                    >
                      {savingEmployeeId === employee.id ? <LoaderCircle className="animate-spin" /> : null}
                      {employee.is_active ? "Deactivate" : "Activate"}
                    </Button>
                    {employee.role === "manager" ? (
                      <Button
                        type="button"
                        variant="outline"
                        size="sm"
                        disabled={savingEmployeeId !== null}
                        onClick={() => {
                          setResetEmployeeId((current) => current === employee.id ? null : employee.id);
                          setResetPassword("");
                          setResetPasswordConfirmation("");
                        }}
                      >
                        <KeyRound />
                        Reset password
                      </Button>
                    ) : null}
                  </div>
                  {resetEmployeeId === employee.id ? (
                    <form
                      className="grid gap-3 rounded-lg border bg-muted/20 p-4 sm:grid-cols-[1fr_1fr_auto]"
                      onSubmit={(event) => {
                        event.preventDefault();
                        void resetEmployeePassword(employee);
                      }}
                    >
                      <div className="space-y-2">
                        <Label htmlFor={`reset-password-${employee.id}`}>New password</Label>
                        <Input
                          id={`reset-password-${employee.id}`}
                          type="password"
                          autoComplete="new-password"
                          minLength={12}
                          maxLength={128}
                          required
                          value={resetPassword}
                          onChange={(event) => setResetPassword(event.target.value)}
                        />
                      </div>
                      <div className="space-y-2">
                        <Label htmlFor={`reset-password-confirmation-${employee.id}`}>Confirm new password</Label>
                        <Input
                          id={`reset-password-confirmation-${employee.id}`}
                          type="password"
                          autoComplete="new-password"
                          minLength={12}
                          maxLength={128}
                          required
                          value={resetPasswordConfirmation}
                          onChange={(event) => setResetPasswordConfirmation(event.target.value)}
                        />
                      </div>
                      <div className="flex items-end">
                        <Button type="submit" disabled={savingEmployeeId !== null}>
                          {savingEmployeeId === employee.id ? <LoaderCircle className="animate-spin" /> : <KeyRound />}
                          {savingEmployeeId === employee.id ? "Resetting..." : "Save password"}
                        </Button>
                      </div>
                    </form>
                  ) : null}
                </Fragment>
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
