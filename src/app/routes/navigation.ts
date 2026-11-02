import {
  BarChart3,
  Boxes,
  Factory,
  HandCoins,
  LayoutDashboard,
  Landmark,
  Package,
  ReceiptText,
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
}

export interface NavGroup {
  title: string;
  items: NavItem[];
}

export const navGroups: NavGroup[] = [
  {
    title: "Operations",
    items: [
      { title: "Dashboard", url: "/", icon: LayoutDashboard },
      { title: "Point of Sale", url: "/pos", icon: ShoppingCart },
      { title: "Products", url: "/products", icon: Package },
      { title: "Inventory", url: "/inventory", icon: Boxes },
      { title: "Purchases", url: "/purchases", icon: Truck },
    ],
  },
  {
    title: "Business",
    items: [
      { title: "Sales History", url: "/sales", icon: ReceiptText },
      { title: "Customers", url: "/customers", icon: Users },
      { title: "Suppliers", url: "/suppliers", icon: Factory },
      { title: "Expenses", url: "/expenses", icon: Wallet },
      { title: "Payments", url: "/payments", icon: HandCoins },
      { title: "Cash accounts", url: "/cash-accounts", icon: Landmark },
      { title: "Reports", url: "/reports", icon: BarChart3 },
      { title: "Employees", url: "/employees", icon: UserCog },
    ],
  },
  {
    title: "System",
    items: [{ title: "Settings", url: "/settings", icon: Settings }],
  },
];

export const mainNav = navGroups.flatMap((group) => group.items);
