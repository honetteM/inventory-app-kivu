import { invoke } from "@tauri-apps/api/core";
import { Building2, KeyRound, LoaderCircle, LogIn, UserRoundPlus } from "lucide-react";
import { createContext, useContext, useEffect, useMemo, useState, type FormEvent, type ReactNode } from "react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { SelectField } from "@/components/shared/select-field";

interface BusinessStatus {
  id: string;
  name: string;
  requires_owner_setup: boolean;
}

interface AuthUser {
  id: string;
  business_id: string;
  username: string;
  full_name: string;
  role: string;
}

interface AuthContextValue {
  user: AuthUser;
  logout: () => Promise<void>;
  error: string | null;
}

const AuthContext = createContext<AuthContextValue | null>(null);

function errorText(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

export function useAuthentication(): AuthContextValue {
  const context = useContext(AuthContext);
  if (!context) {
    throw new Error("Authentication context is only available to signed-in application screens.");
  }
  return context;
}

export function AuthGate({ children }: { children: ReactNode }) {
  const [businesses, setBusinesses] = useState<BusinessStatus[]>([]);
  const [businessId, setBusinessId] = useState("");
  const [businessName, setBusinessName] = useState("");
  const [username, setUsername] = useState("");
  const [fullName, setFullName] = useState("");
  const [password, setPassword] = useState("");
  const [passwordConfirmation, setPasswordConfirmation] = useState("");
  const [user, setUser] = useState<AuthUser | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    invoke<BusinessStatus[]>("auth_business_status")
      .then((result) => {
        if (active) {
          setBusinesses(result);
          setBusinessId(result[0]?.id ?? "");
        }
      })
      .catch((cause: unknown) => {
        if (active) {
          setError(`Could not load sign-in information. ${errorText(cause)}`);
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

  const selectedBusiness = businesses.find((business) => business.id === businessId);
  const isInitialBusinessSetup = businesses.length === 0;
  const requiresOwnerSetup = isInitialBusinessSetup || selectedBusiness?.requires_owner_setup === true;

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    if (requiresOwnerSetup && password !== passwordConfirmation) {
      setError("The password confirmation does not match.");
      return;
    }

    setIsSaving(true);
    try {
      const signedInUser = requiresOwnerSetup
        ? await invoke<AuthUser>("auth_bootstrap_owner", {
            businessId: isInitialBusinessSetup ? null : businessId,
            newBusinessName: isInitialBusinessSetup ? businessName.trim() : null,
            username: username.trim(),
            fullName: fullName.trim(),
            password,
          })
        : await invoke<AuthUser>("auth_login", {
            businessId,
            username: username.trim(),
            password,
          });
      setUser(signedInUser);
      setPassword("");
      setPasswordConfirmation("");
    } catch (cause) {
      setError(
        requiresOwnerSetup
          ? `Could not create the owner account. ${errorText(cause)}`
          : `Sign-in failed. ${errorText(cause)}`,
      );
    } finally {
      setIsSaving(false);
    }
  }

  async function logout() {
    try {
      await invoke("auth_logout");
      setUser(null);
      setPassword("");
      setPasswordConfirmation("");
      setError(null);
    } catch (cause) {
      setError(`Could not sign out. ${errorText(cause)}`);
    }
  }

  const contextValue = useMemo(
    () => (user ? { user, logout, error } : null),
    [user, error],
  );

  if (user && contextValue) {
    return <AuthContext.Provider value={contextValue}>{children}</AuthContext.Provider>;
  }

  return (
    <main className="flex min-h-svh items-center justify-center bg-muted/30 p-4">
      {isLoading ? (
        <Card className="w-full max-w-md">
          <CardContent className="flex items-center justify-center gap-3 py-10 text-sm text-muted-foreground">
            <LoaderCircle className="size-4 animate-spin" />
            Loading secure sign-in...
          </CardContent>
        </Card>
      ) : (
        <Card className="w-full max-w-md">
          <CardHeader className="space-y-3">
            <div className="flex size-11 items-center justify-center rounded-xl bg-primary/10 text-primary">
              {requiresOwnerSetup ? <UserRoundPlus className="size-5" /> : <KeyRound className="size-5" />}
            </div>
            <div className="space-y-1">
              <CardTitle>{requiresOwnerSetup ? "Set up owner access" : "Sign in to your business"}</CardTitle>
              <CardDescription>
                {requiresOwnerSetup
                  ? "Create the first owner account. Passwords are securely hashed on this device."
                  : "Sign in to access your business records on this device."}
              </CardDescription>
            </div>
          </CardHeader>
          <CardContent>
            {error ? (
              <div role="alert" className="mb-5 rounded-md border border-destructive/30 bg-destructive/5 p-3 text-sm text-destructive">
                {error}
              </div>
            ) : null}
            <form className="space-y-4" onSubmit={handleSubmit}>
              {isInitialBusinessSetup ? (
                <div className="space-y-2">
                  <Label htmlFor="auth-business-name">Business name</Label>
                  <Input
                    id="auth-business-name"
                    autoComplete="organization"
                    maxLength={120}
                    required
                    value={businessName}
                    onChange={(event) => setBusinessName(event.target.value)}
                  />
                </div>
              ) : (
                <div className="space-y-2">
                  <Label htmlFor="auth-business">Business</Label>
                  <SelectField
                    id="auth-business"
                    value={businessId}
                    onChange={(event) => setBusinessId(event.target.value)}
                    required
                  >
                    {businesses.map((business) => (
                      <option key={business.id} value={business.id}>{business.name}</option>
                    ))}
                  </SelectField>
                </div>
              )}
              {requiresOwnerSetup ? (
                <div className="space-y-2">
                  <Label htmlFor="auth-full-name">Owner full name</Label>
                  <Input
                    id="auth-full-name"
                    autoComplete="name"
                    maxLength={120}
                    required
                    value={fullName}
                    onChange={(event) => setFullName(event.target.value)}
                  />
                </div>
              ) : null}
              <div className="space-y-2">
                <Label htmlFor="auth-username">Username</Label>
                <Input
                  id="auth-username"
                  autoComplete="username"
                  minLength={3}
                  maxLength={32}
                  required
                  value={username}
                  onChange={(event) => setUsername(event.target.value)}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="auth-password">Password</Label>
                <Input
                  id="auth-password"
                  type="password"
                  autoComplete={requiresOwnerSetup ? "new-password" : "current-password"}
                  minLength={12}
                  maxLength={128}
                  required
                  value={password}
                  onChange={(event) => setPassword(event.target.value)}
                />
                {requiresOwnerSetup ? (
                  <p className="text-xs text-muted-foreground">Use at least 12 characters.</p>
                ) : null}
              </div>
              {requiresOwnerSetup ? (
                <div className="space-y-2">
                  <Label htmlFor="auth-password-confirmation">Confirm password</Label>
                  <Input
                    id="auth-password-confirmation"
                    type="password"
                    autoComplete="new-password"
                    minLength={12}
                    maxLength={128}
                    required
                    value={passwordConfirmation}
                    onChange={(event) => setPasswordConfirmation(event.target.value)}
                  />
                </div>
              ) : null}
              <Button
                className="w-full"
                type="submit"
                disabled={isSaving || (!businessId && !isInitialBusinessSetup)}
              >
                {isSaving ? <LoaderCircle className="animate-spin" /> : requiresOwnerSetup ? <Building2 /> : <LogIn />}
                {isSaving ? "Please wait..." : requiresOwnerSetup ? "Create owner account" : "Sign in"}
              </Button>
            </form>
          </CardContent>
        </Card>
      )}
    </main>
  );
}
