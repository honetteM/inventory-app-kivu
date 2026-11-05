import { useEffect, useRef } from "react";
import { Outlet, useLocation } from "react-router-dom";
import { SidebarInset, SidebarProvider, SidebarTrigger } from "@/components/ui/sidebar";
import { Separator } from "@/components/ui/separator";
import { Button } from "@/components/ui/button";
import { AppSidebar } from "./app-sidebar";
import { getAvailableNavGroups, navGroups } from "@/app/routes/navigation";
import { useAuthentication } from "@/modules/auth/auth-gate";

export function AppLayout() {
  const { user, permissions, logout, error } = useAuthentication();
  const { pathname } = useLocation();
  const mainRef = useRef<HTMLElement>(null);
  const currentRoute = navGroups.flatMap((group) => group.items).find(
    (item) => pathname === item.url || (item.url !== "/" && pathname.startsWith(`${item.url}/`)),
  );
  const currentGroup = getAvailableNavGroups(permissions).find((group) =>
    group.items.some(
      (item) => pathname === item.url || (item.url !== "/" && pathname.startsWith(`${item.url}/`)),
    ),
  );
  const routeIsRestricted = currentRoute && !permissions.includes(currentRoute.permission);

  useEffect(() => {
    mainRef.current?.scrollTo({ top: 0 });
  }, [pathname]);

  return (
    <SidebarProvider>
      <AppSidebar />
      <SidebarInset className="min-h-svh min-w-0 bg-background">
        <header className="sticky top-0 z-20 flex h-16 shrink-0 items-center gap-3 border-b bg-card/95 px-4 backdrop-blur-sm lg:px-6 print:hidden">
          <SidebarTrigger />
          <Separator orientation="vertical" className="!h-5" />
          <div className="min-w-0">
            <p className="text-[10px] font-semibold tracking-[0.12em] text-muted-foreground uppercase">
              Workspace
            </p>
            <p className="truncate text-sm font-semibold tracking-tight">
              {currentGroup?.title ?? "Quincaillerie Manager"}
            </p>
          </div>
          <div className="ml-auto flex items-center gap-2 rounded-full border border-border/80 bg-background px-3 py-1.5 text-xs font-medium text-muted-foreground">
            <span aria-hidden="true" className="size-1.5 rounded-full bg-emerald-600" />
            <span className="hidden sm:inline">{user.full_name}</span>
            <span className="sm:hidden">Local</span>
          </div>
          <Button type="button" variant="outline" size="sm" onClick={() => void logout()}>
            Sign out
          </Button>
        </header>
        {error ? (
          <div role="alert" className="border-b border-destructive/30 bg-destructive/5 px-4 py-2 text-sm text-destructive lg:px-6">
            {error}
          </div>
        ) : null}
        <main ref={mainRef} className="min-w-0 flex-1 overflow-x-hidden overflow-y-auto">
          <div className="page-content">
            {routeIsRestricted ? (
              <div role="status" className="mx-auto max-w-2xl rounded-lg border bg-card p-6 text-sm">
                <h1 className="font-semibold">Access restricted</h1>
                <p className="mt-2 text-muted-foreground">
                  Your account does not have permission to view this page. Contact the Owner/Admin if you need access.
                </p>
              </div>
            ) : (
              <Outlet />
            )}
          </div>
        </main>
      </SidebarInset>
    </SidebarProvider>
  );
}
