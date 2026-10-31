import { createBrowserRouter } from "react-router-dom";
import { AppLayout } from "../layouts/app-layout";
import { PagePlaceholder } from "@/components/shared/page-placeholder";
import { DashboardPage } from "@/modules/dashboard/dashboard-page";
import { InventoryPage } from "@/modules/inventory/inventory-page";
import { PosPage } from "@/modules/pos/pos-page";
import { ProductsPage } from "@/modules/products/products-page";
import { BusinessSetupPage } from "@/modules/settings/business-setup-page";
import { SuppliersPage } from "@/modules/suppliers/suppliers-page";

const placeholder = (title: string, description: string) => (
  <PagePlaceholder title={title} description={description} />
);

export const router = createBrowserRouter([
  {
    path: "/",
    element: <AppLayout />,
    children: [
      { index: true, element: <DashboardPage /> },
      { path: "pos", element: <PosPage /> },
      { path: "products", element: <ProductsPage /> },
      { path: "inventory", element: <InventoryPage /> },
      { path: "purchases", element: placeholder("Purchases", "Supplier invoices and stock receiving.") },
      { path: "sales", element: placeholder("Sales History", "Completed transactions and returns.") },
      { path: "customers", element: placeholder("Customers", "Customer accounts, credit limits and debts.") },
      { path: "suppliers", element: <SuppliersPage /> },
      { path: "expenses", element: placeholder("Expenses", "Operational expense tracking.") },
      { path: "payments", element: placeholder("Payments", "Customer repayments and supplier payments.") },
      { path: "reports", element: placeholder("Reports", "Daily, weekly and monthly business reports.") },
      { path: "employees", element: placeholder("Employees", "Users, roles and permissions.") },
      { path: "settings", element: <BusinessSetupPage /> },
    ],
  },
]);
