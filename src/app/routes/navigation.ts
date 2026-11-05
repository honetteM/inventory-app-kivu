import {
  BarChart3,
  Boxes,
  Factory,
  HandCoins,
  LayoutDashboard,
  Landmark,
  Package,
  ReceiptText,
  Scale,
  History,
  Settings,
  ShoppingCart,
  Truck,
  UserCog,
  Users,
  Wallet,
  type LucideIcon,
} from "lucide-react";

export interface NavItem {
  title: string;
  url: string;
  icon: LucideIcon;
  permission: string;
}

export interface NavGroup {
  title: string;
  items: NavItem[];
}

export const navGroups: NavGroup[] = [
  {
    title: "Operations",
    items: [
      { title: "Dashboard", url: "/", icon: LayoutDashboard, permission: "inventory.read" },
      { title: "Point of Sale", url: "/pos", icon: ShoppingCart, permission: "pos.use" },
      { title: "Products", url: "/products", icon: Package, permission: "catalog.read" },
      { title: "Inventory", url: "/inventory", icon: Boxes, permission: "inventory.read" },
      { title: "Purchases", url: "/purchases", icon: Truck, permission: "purchases.read" },
      { title: "Purchase History", url: "/purchase-history", icon: History, permission: "purchases.read" },
    ],
  },
  {
    title: "Business",
    items: [
      { title: "Sales History", url: "/sales", icon: ReceiptText, permission: "sales.read" },
      { title: "Customers", url: "/customers", icon: Users, permission: "customers.read" },
      { title: "Suppliers", url: "/suppliers", icon: Factory, permission: "suppliers.read" },
      { title: "Expenses", url: "/expenses", icon: Wallet, permission: "expenses.read" },
      { title: "Payments", url: "/payments", icon: HandCoins, permission: "customer_debts.read" },
      { title: "Cash accounts", url: "/cash-accounts", icon: Landmark, permission: "cash_accounts.manage" },
      { title: "Reports", url: "/reports", icon: BarChart3, permission: "reports.read" },
      { title: "Income statement", url: "/reports/income", icon: BarChart3, permission: "reports.read" },
      { title: "Balance sheet", url: "/reports/balance-sheet", icon: Scale, permission: "reports.read" },
      { title: "Employees", url: "/employees", icon: UserCog, permission: "employees.manage" },
    ],
  },
  {
    title: "System",
    items: [{ title: "Settings", url: "/settings", icon: Settings, permission: "catalog.manage" }],
  },
];

export const mainNav = navGroups.flatMap((group) => group.items);

export function getAvailableNavGroups(permissions: readonly string[]): NavGroup[] {
  const grantedPermissions = new Set(permissions);
  return navGroups
    .map((group) => ({
      ...group,
      items: group.items.filter((item) => grantedPermissions.has(item.permission)),
    }))
    .filter((group) => group.items.length > 0);
}
