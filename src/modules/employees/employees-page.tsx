import { invoke } from "@tauri-apps/api/core";
import { LoaderCircle, Plus, UserCog } from "lucide-react";
import { useEffect, useState, type FormEvent } from "react";

import { PageHeader } from "@/components/shared/page-header";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { SelectField } from "@/components/shared/select-field";
import { Badge } from "@/components/ui/badge";
import { useAuthentication } from "@/modules/auth/auth-gate";

type EmployeeRole = "manager" | "cashier";

interface Employee {
  id: string;
  business_id: string;
  username: string;
  full_name: string;
  role: EmployeeRole;
  is_active: boolean;
  last_login_at: string | null;
  created_at: string;
}

const ROLES: { value: EmployeeRole; label: string }[] = [
  { value: "manager", label: "Manager" },
  { value: "cashier", label: "Cashier" },
];

function errorText(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

function roleLabel(role: EmployeeRole): string {
  return ROLES.find((item) => item.value === role)?.label ?? role;
}

export function EmployeesPage() {
  const { user } = useAuthentication();
  const [employees, setEmployees] = useState<Employee[]>([]);
  const [username, setUsername] = useState("");
  const [fullName, setFullName] = useState("");
  const [role, setRole] = useState<EmployeeRole>("cashier");
  const [password, setPassword] = useState("");
  const [passwordConfirmation, setPasswordConfirmation] = useState("");
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [savingEmployeeId, setSavingEmployeeId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    invoke<Employee[]>("employee_list", { businessId: user.business_id })
      .then((result) => {
        if (active) {
          setEmployees(result);
          setError(null);
        }
      })
      .catch((cause: unknown) => {
        if (active) {
          setError(`Could not load employee records. ${errorText(cause)}`);
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
  }, [user.business_id]);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    setNotice(null);
    if (password !== passwordConfirmation) {
      setError("Password confirmation does not match.");
      return;
    }

    setIsSaving(true);
    try {
      const createdEmployee = await invoke<Employee>("employee_create", {
        businessId: user.business_id,
        username: username.trim(),
        fullName: fullName.trim(),
        role,
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
      setNotice("Employee profile created and audited. Employee sign-in is not enabled yet.");
    } catch (cause) {
      setError(`Could not create employee profile. ${errorText(cause)}`);
    } finally {
      setIsSaving(false);
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

  return (
    <div className="mx-auto max-w-5xl space-y-6">
      <PageHeader
        eyebrow="People and access"
        title="Employees"
        description="Create and maintain staff profiles for this business. Changes are recorded against your owner account."
      />

      <div className="rounded-lg border border-amber-500/30 bg-amber-500/5 p-4 text-sm text-foreground">
        Employee sign-in is not enabled yet. Staff roles do not grant application access until role-specific permissions are implemented.
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
          <CardTitle>Add an employee</CardTitle>
          <CardDescription>
            Choose a unique username and role. Passwords are hashed on this device and are never displayed after creation.
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
              <Label htmlFor="employee-username">Username</Label>
              <Input
                id="employee-username"
                autoComplete="off"
                minLength={3}
                maxLength={32}
                required
                value={username}
                onChange={(event) => setUsername(event.target.value)}
                placeholder="e.g. jean.cashier"
              />
              <p className="text-xs text-muted-foreground">3–32 letters, numbers, dots, underscores or hyphens.</p>
            </div>
            <div className="space-y-2">
              <Label htmlFor="employee-role">Role</Label>
              <SelectField
                id="employee-role"
                value={role}
                onChange={(event) => {
                  const selectedRole = ROLES.find((item) => item.value === event.target.value);
                  if (selectedRole) {
                    setRole(selectedRole.value);
                  }
                }}
              >
                {ROLES.map((item) => <option key={item.value} value={item.value}>{item.label}</option>)}
              </SelectField>
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
                {isSaving ? "Saving..." : "Create employee"}
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
                <div key={employee.id} className="flex flex-col gap-3 py-4 sm:flex-row sm:items-center sm:justify-between">
                  <div className="flex min-w-0 items-start gap-3">
                    <UserCog className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
                    <div className="min-w-0 space-y-1">
                      <p className="truncate font-medium">{employee.full_name}</p>
                      <p className="text-sm text-muted-foreground">
                        @{employee.username} · {roleLabel(employee.role)}
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
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
