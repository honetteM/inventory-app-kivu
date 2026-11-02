import { createBrowserRouter } from "react-router-dom";
import { AppLayout } from "../layouts/app-layout";
import { PagePlaceholder } from "@/components/shared/page-placeholder";
import { DashboardPage } from "@/modules/dashboard/dashboard-page";
import { InventoryPage } from "@/modules/inventory/inventory-page";
import { PosPage } from "@/modules/pos/pos-page";
import { PurchasesPage } from "@/modules/purchases/purchases-page";
import { ProductsPage } from "@/modules/products/products-page";
import { BusinessSetupPage } from "@/modules/settings/business-setup-page";
import { SuppliersPage } from "@/modules/suppliers/suppliers-page";
import { CustomersPage } from "@/modules/customers/customers-page";
import { PaymentsPage } from "@/modules/payments/payments-page";
import { ExpensesPage } from "@/modules/expenses/expenses-page";
import { ReportsPage } from "@/modules/reports/reports-page";

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
      { path: "purchases", element: <PurchasesPage /> },
      { path: "sales", element: placeholder("Sales History", "Completed transactions and returns.") },
      { path: "customers", element: <CustomersPage /> },
      { path: "suppliers", element: <SuppliersPage /> },
      { path: "expenses", element: <ExpensesPage /> },
      { path: "payments", element: <PaymentsPage /> },
      { path: "reports", element: <ReportsPage /> },
      { path: "employees", element: placeholder("Employees", "Users, roles and permissions.") },
      { path: "settings", element: <BusinessSetupPage /> },
    ],
  },
]);
