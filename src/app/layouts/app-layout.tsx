import { useEffect, useRef } from "react";
import { Outlet, useLocation } from "react-router-dom";
import { SidebarInset, SidebarProvider, SidebarTrigger } from "@/components/ui/sidebar";
import { Separator } from "@/components/ui/separator";
import { AppSidebar } from "./app-sidebar";
import { navGroups } from "@/app/routes/navigation";

export function AppLayout() {
  const { pathname } = useLocation();
  const mainRef = useRef<HTMLElement>(null);
  const currentGroup = navGroups.find((group) =>
    group.items.some(
      (item) => pathname === item.url || (item.url !== "/" && pathname.startsWith(`${item.url}/`)),
    ),
  );

  useEffect(() => {
    mainRef.current?.scrollTo({ top: 0 });
  }, [pathname]);

  return (
    <SidebarProvider>
      <AppSidebar />
      <SidebarInset className="min-h-svh min-w-0 bg-background">
        <header className="sticky top-0 z-20 flex h-16 shrink-0 items-center gap-3 border-b bg-card/95 px-4 backdrop-blur-sm lg:px-6">
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
            <span className="hidden sm:inline">Local workspace</span>
            <span className="sm:hidden">Local</span>
          </div>
        </header>
        <main ref={mainRef} className="min-w-0 flex-1 overflow-x-hidden overflow-y-auto">
          <div className="page-content">
          <Outlet />
          </div>
        </main>
      </SidebarInset>
    </SidebarProvider>
  );
}
