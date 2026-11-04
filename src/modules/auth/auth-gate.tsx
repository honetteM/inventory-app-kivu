import { invoke } from "@tauri-apps/api/core";
import { ArrowLeft, Building2, KeyRound, LoaderCircle, LogIn, UserRoundPlus } from "lucide-react";
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

interface RecoveryQuestion {
  id: string;
  label: string;
}

interface RecoveryPrompts {
  question_one: string;
  question_two: string;
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
  permissions: string[];
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
  const [recoveryCode, setRecoveryCode] = useState("");
  const [recoveryQuestionOne, setRecoveryQuestionOne] = useState("");
  const [recoveryAnswerOne, setRecoveryAnswerOne] = useState("");
  const [recoveryQuestionTwo, setRecoveryQuestionTwo] = useState("");
  const [recoveryAnswerTwo, setRecoveryAnswerTwo] = useState("");
  const [recoveryQuestions, setRecoveryQuestions] = useState<RecoveryQuestion[]>([]);
  const [recoveryPrompts, setRecoveryPrompts] = useState<RecoveryPrompts | null>(null);
  const [isRecovery, setIsRecovery] = useState(false);
  const [user, setUser] = useState<AuthUser | null>(null);
  const [permissions, setPermissions] = useState<string[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    async function restoreAuthentication() {
      try {
        const currentUser = await invoke<AuthUser | null>("auth_current_user");
        if (currentUser) {
          const currentPermissions = await invoke<string[]>("auth_current_permissions");
          if (active) {
            setUser(currentUser);
            setPermissions(currentPermissions);
          }
          return;
        }

        const result = await invoke<BusinessStatus[]>("auth_business_status");
        const questions = await invoke<RecoveryQuestion[]>("auth_recovery_questions");
        if (active) {
          setBusinesses(result);
          setRecoveryQuestions(questions);
          setRecoveryQuestionOne(questions[0]?.id ?? "");
          setRecoveryQuestionTwo(questions[1]?.id ?? "");
          setBusinessId(result[0]?.id ?? "");
          if (result.length === 0 || result[0]?.requires_owner_setup) {
            setUsername("admin");
            setFullName("Administrator");
          }
        }
      } catch (cause) {
        if (active) {
          setError(`Could not restore sign-in information. ${errorText(cause)}`);
        }
      } finally {
        if (active) {
          setIsLoading(false);
        }
      }
    }

    void restoreAuthentication();
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
    if ((requiresOwnerSetup || isRecovery) && password !== passwordConfirmation) {
      setError("The password confirmation does not match.");
      return;
    }

    setIsSaving(true);
    try {
      if (isRecovery) {
        await invoke("auth_recover_owner", {
          businessId,
          username: username.trim(),
          recoveryCode,
          questionOneAnswer: recoveryAnswerOne,
          questionTwoAnswer: recoveryAnswerTwo,
          newPassword: password,
        });
        setIsRecovery(false);
        setPassword("");
        setPasswordConfirmation("");
        setRecoveryCode("");
        setRecoveryAnswerOne("");
        setRecoveryAnswerTwo("");
        setRecoveryPrompts(null);
        setError("Password recovered. Sign in with your new password.");
        return;
      }
      const signedInUser = requiresOwnerSetup
        ? await invoke<AuthUser>("auth_bootstrap_owner", {
            businessId: isInitialBusinessSetup ? null : businessId,
            newBusinessName: isInitialBusinessSetup ? businessName.trim() : null,
            username: username.trim(),
            fullName: fullName.trim(),
            password,
            recoveryCode,
            recoveryQuestionOne,
            recoveryAnswerOne,
            recoveryQuestionTwo,
            recoveryAnswerTwo,
          })
        : await invoke<AuthUser>("auth_login", {
            businessId,
            username: username.trim(),
            password,
          });
      const signedInPermissions = await invoke<string[]>("auth_current_permissions");
      setUser(signedInUser);
      setPermissions(signedInPermissions);
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

  async function beginRecovery() {
    setError(null);
    setIsRecovery(true);
    setPassword("");
    setPasswordConfirmation("");
    setRecoveryCode("");
    setRecoveryAnswerOne("");
    setRecoveryAnswerTwo("");
    if (businessId && username.trim()) {
      try {
        setRecoveryPrompts(await invoke<RecoveryPrompts | null>("auth_recovery_prompts", {
          businessId,
          username: username.trim(),
        }));
      } catch (cause) {
        setError(`Could not load recovery questions. ${errorText(cause)}`);
      }
    }
  }

  async function logout() {
    try {
      await invoke("auth_logout");
      setUser(null);
      setPermissions([]);
      setPassword("");
      setPasswordConfirmation("");
      setError(null);
    } catch (cause) {
      setError(`Could not sign out. ${errorText(cause)}`);
    }
  }

  const contextValue = useMemo(
    () => (user ? { user, permissions, logout, error } : null),
    [user, permissions, error],
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
              <CardTitle>{isRecovery ? "Recover Owner/Admin access" : requiresOwnerSetup ? "Set up Owner/Admin access" : "Sign in to your business"}</CardTitle>
              <CardDescription>
                {isRecovery
                  ? "Verify your offline recovery code and security answers, then choose a new password."
                  : requiresOwnerSetup
                  ? "Create the first Owner/Admin account. Choose a password; it is securely hashed on this device."
                  : "Owners and active Managers can sign in with their business username and password."}
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
                    onChange={(event) => {
                      const selectedId = event.target.value;
                      setBusinessId(selectedId);
                      const selected = businesses.find((business) => business.id === selectedId);
                      if (selected?.requires_owner_setup) {
                        setUsername("admin");
                        setFullName("Administrator");
                      } else {
                        setUsername("");
                        setFullName("");
                      }
                    }}
                    required
                  >
                    {businesses.map((business) => (
                      <option key={business.id} value={business.id}>{business.name}</option>
                    ))}
                  </SelectField>
                </div>
              )}
              {!isRecovery && requiresOwnerSetup ? (
                <div className="space-y-2">
                  <Label htmlFor="auth-full-name">Owner/Admin full name</Label>
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
                {!isRecovery && requiresOwnerSetup ? (
                  <p className="text-xs text-muted-foreground">The default username is admin; you can change it before creating the account.</p>
                ) : null}
              </div>
              {!isRecovery && !requiresOwnerSetup ? <div className="space-y-2">
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
              </div> : null}
              {isRecovery ? (
                <>
                  <div className="space-y-2">
                    <Label htmlFor="auth-recovery-code">Recovery code</Label>
                    <Input id="auth-recovery-code" type="password" required value={recoveryCode} onChange={(event) => setRecoveryCode(event.target.value)} />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="auth-recovery-answer-one">{recoveryPrompts?.question_one ?? "First security-question answer"}</Label>
                    <Input id="auth-recovery-answer-one" required value={recoveryAnswerOne} onChange={(event) => setRecoveryAnswerOne(event.target.value)} />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="auth-recovery-answer-two">{recoveryPrompts?.question_two ?? "Second security-question answer"}</Label>
                    <Input id="auth-recovery-answer-two" required value={recoveryAnswerTwo} onChange={(event) => setRecoveryAnswerTwo(event.target.value)} />
                  </div>
                </>
              ) : null}
              {(requiresOwnerSetup || isRecovery) ? (
                <div className="space-y-2">
                  <Label htmlFor="auth-password">{isRecovery ? "New password" : "Password"}</Label>
                  <Input
                    id="auth-password"
                    type="password"
                    minLength={12}
                    maxLength={128}
                    required
                    value={password}
                    onChange={(event) => setPassword(event.target.value)}
                  />
                </div>
              ) : null}
              {(requiresOwnerSetup || isRecovery) ? (
                <div className="space-y-2">
                  <Label htmlFor="auth-password-confirmation">Confirm password</Label>
                  <Input id="auth-password-confirmation" type="password" minLength={12} maxLength={128} required value={passwordConfirmation} onChange={(event) => setPasswordConfirmation(event.target.value)} />
                </div>
              ) : null}
              {requiresOwnerSetup && !isRecovery ? (
                <div className="grid gap-4 rounded-md border p-3">
                  <p className="text-sm font-medium">Offline recovery setup</p>
                  <div className="space-y-2">
                    <Label htmlFor="auth-recovery-code">Recovery code</Label>
                    <Input id="auth-recovery-code" type="password" required value={recoveryCode} onChange={(event) => setRecoveryCode(event.target.value)} />
                  </div>
                  <div className="grid gap-3 sm:grid-cols-2">
                    {[{ number: "one", value: recoveryQuestionOne, setValue: setRecoveryQuestionOne }, { number: "two", value: recoveryQuestionTwo, setValue: setRecoveryQuestionTwo }].map((question) => (
                      <div className="space-y-2" key={question.number}>
                        <Label>Security question {question.number}</Label>
                        <SelectField value={question.value} onChange={(event) => question.setValue(event.target.value)}>
                          {recoveryQuestions.filter((item) => item.id === question.value || item.id !== (question.number === "one" ? recoveryQuestionTwo : recoveryQuestionOne)).map((item) => <option key={item.id} value={item.id}>{item.label}</option>)}
                        </SelectField>
                        <Input required placeholder="Answer" value={question.number === "one" ? recoveryAnswerOne : recoveryAnswerTwo} onChange={(event) => question.number === "one" ? setRecoveryAnswerOne(event.target.value) : setRecoveryAnswerTwo(event.target.value)} />
                      </div>
                    ))}
                  </div>
                  <p className="text-xs text-muted-foreground">Keep the recovery code and answers safe. They are required to recover the Owner password without deleting data.</p>
                </div>
              ) : null}
              <Button
                className="w-full"
                type="submit"
                disabled={isSaving || (!businessId && !isInitialBusinessSetup)}
              >
                {isSaving ? <LoaderCircle className="animate-spin" /> : requiresOwnerSetup ? <Building2 /> : <LogIn />}
                {isSaving ? "Please wait..." : isRecovery ? "Reset Owner password" : requiresOwnerSetup ? "Create Owner/Admin account" : "Sign in"}
              </Button>
            </form>
            {!requiresOwnerSetup && !isRecovery ? (
              <Button variant="link" className="mt-3 w-full" type="button" onClick={() => void beginRecovery()}>Forgot Owner password?</Button>
            ) : null}
            {isRecovery ? (
              <Button variant="ghost" className="mt-2 w-full" type="button" onClick={() => { setIsRecovery(false); setRecoveryPrompts(null); setError(null); }}><ArrowLeft />Back to sign in</Button>
            ) : null}
          </CardContent>
        </Card>
      )}
    </main>
  );
}
